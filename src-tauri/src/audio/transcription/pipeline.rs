//! Real-time transcription pipeline.
//!
//! Reads audio chunks from a channel, scores each with VAD, feeds them to a
//! [`Segmenter`], and hands the segments it emits to a per-stream worker that
//! transcribes them **sequentially**. In-order transcription means results can
//! never arrive out of order (e.g. a stale preview overwriting committed text),
//! and each stream issues at most one ASR call at a time, so neither source can
//! starve the other.
//!
//! Streaming previews ([`Finality::Partial`]) are best-effort: the worker drops
//! a preview when newer audio is already queued behind it, since the newer
//! segment supersedes it. Committed segments are never dropped.
//!
//! [`run`] blocks until the audio channel disconnects; call it from a blocking
//! task (the VAD calls and channel reads are synchronous).

use super::segmenter::{Segment, Segmenter};
use super::types::{Finality, TranscriptionResult};
use crate::audio::alignment::{StreamAlignment, StreamKind};
use crate::audio::constants::{MIN_CHUNK_SAMPLES, SAMPLE_RATE_16KHZ, VAD_FRAME_SAMPLES};
use crate::audio::processing::Resampler;
use crate::bridges::{SpeechRecognizer, Transcriber};
use std::sync::Arc;
use tokio::sync::mpsc;

type ResultCallback = Arc<dyn Fn(TranscriptionResult) + Send + Sync>;

/// Run the pipeline until the audio channel disconnects.
///
/// Blocking: must be called via `spawn_blocking` (or a dedicated thread) from
/// within a tokio runtime, which the transcription worker is spawned onto.
#[tracing::instrument(skip_all)]
pub fn run<F>(
    rx: crossbeam_channel::Receiver<Vec<f32>>,
    input_sample_rate: u32,
    transcriber: Transcriber,
    alignment: Arc<StreamAlignment>,
    kind: StreamKind,
    on_result: F,
) where
    F: Fn(TranscriptionResult) + Send + Sync + 'static,
{
    let stream_id = new_stream_id();

    if let Err(e) = transcriber.vad.create_stream(&stream_id) {
        tracing::error!(error = %e, "Failed to create VAD stream");
        return;
    }
    tracing::info!(stream = %stream_id, "Transcription pipeline started");

    let (segment_tx, segment_rx) = mpsc::unbounded_channel::<Segment>();
    let worker = tokio::runtime::Handle::current().spawn(transcribe_worker(
        segment_rx,
        transcriber.asr.clone(),
        Arc::new(on_result),
    ));

    let mut segmenter = Segmenter::new();
    let mut resampler = Resampler::new(input_sample_rate, SAMPLE_RATE_16KHZ);
    // Resampled 16kHz audio accumulates here and is scored in fixed
    // VAD_FRAME_SAMPLES windows, since the VAD truncates anything larger.
    let mut frame_buf: Vec<f32> = Vec::new();
    // Total 16kHz samples consumed so far, i.e. the current playback position
    // in the recording. Used to stamp each segment's start offset.
    let mut cumulative_samples: usize = 0;

    'outer: while let Ok(chunk) = rx.recv() {
        frame_buf.extend(resampler.process(&chunk));

        while frame_buf.len() >= VAD_FRAME_SAMPLES {
            let frame: Vec<f32> = frame_buf.drain(..VAD_FRAME_SAMPLES).collect();

            let voice_prob = match transcriber.vad.process(&stream_id, &frame) {
                Ok(prob) => prob,
                Err(e) => {
                    tracing::error!(stream = %stream_id, error = %e, "VAD processing failed");
                    continue;
                }
            };

            cumulative_samples += frame.len();

            for mut segment in segmenter.push(voice_prob, &frame) {
                // The segment's speech ended `trailing_trimmed` samples before
                // the current position (that tail of silence was dropped), and
                // spans `audio.len()` back from there including pre-roll.
                // Adding the stream's alignment offset puts the timestamp on
                // the recording's shared timeline (matching the padded WAVs).
                let end_sample = cumulative_samples.saturating_sub(segment.trailing_trimmed);
                let start_sample = end_sample.saturating_sub(segment.audio.len());
                segment.start_sec = start_sample as f32 / SAMPLE_RATE_16KHZ as f32
                    + alignment.offset_sec(kind);
                if segment_tx.send(segment).is_err() {
                    tracing::error!(stream = %stream_id, "Transcription worker died");
                    break 'outer;
                }
            }
        }
    }

    // Closing the channel lets the worker drain queued segments and exit.
    drop(segment_tx);
    drop(worker);

    transcriber.vad.destroy_stream(&stream_id);
    tracing::info!(stream = %stream_id, "Transcription pipeline stopped");
}

/// Transcribe queued segments one at a time, preserving order.
async fn transcribe_worker(
    mut rx: mpsc::UnboundedReceiver<Segment>,
    asr: Arc<dyn SpeechRecognizer>,
    on_result: ResultCallback,
) {
    while let Some(segment) = rx.recv().await {
        // A preview is only useful while it is the freshest audio; if more
        // segments queued up behind it, skip straight to them.
        if segment.finality == Finality::Partial && !rx.is_empty() {
            continue;
        }

        let mut audio = segment.audio;
        let duration_sec = audio.len() as f32 / SAMPLE_RATE_16KHZ as f32;

        // The ASR requires at least 1 second of audio. A short final segment
        // (e.g. a brief utterance before silence) is padded with trailing
        // silence so it can still be transcribed instead of rejected.
        if audio.len() < MIN_CHUNK_SAMPLES {
            audio.resize(MIN_CHUNK_SAMPLES, 0.0);
        }

        match asr.transcribe(&audio).await {
            Ok(out) if !out.text.trim().is_empty() => {
                let text = out.text.trim().to_string();
                // A mid-utterance flush (buffer/length cap, not a real pause)
                // that happens to land on terminal punctuation is a genuine
                // sentence end: promote it so a long unbroken monologue is
                // split into sentences instead of arbitrary length-based chunks.
                let finality = if segment.finality == Finality::Segment
                    && ends_sentence(&text)
                {
                    Finality::Sentence
                } else {
                    segment.finality
                };
                // Word timings arrive relative to this segment's audio; shift
                // them onto the recording timeline. Partials are skipped —
                // they're superseded within moments and never seekable.
                let words = if finality == Finality::Partial {
                    Vec::new()
                } else {
                    out.words
                        .into_iter()
                        .map(|mut w| {
                            w.start += segment.start_sec;
                            w.end += segment.start_sec;
                            w
                        })
                        .collect()
                };
                on_result(TranscriptionResult {
                    text,
                    finality,
                    duration_sec,
                    start_sec: segment.start_sec,
                    words,
                });
            }
            Ok(_) => {}
            Err(e) => {
                tracing::error!(error = %e, finality = ?segment.finality, "Transcription failed")
            }
        }
    }
}

/// Whether `text` reads as a finished sentence, i.e. its last non-quote,
/// non-bracket character is terminal punctuation. Handles ASCII and the
/// common full-width CJK marks so non-Latin transcripts work too.
fn ends_sentence(text: &str) -> bool {
    text.trim_end()
        .trim_end_matches(['"', '\'', ')', ']', '}', '»', '”', '’'])
        .chars()
        .next_back()
        .is_some_and(|c| matches!(c, '.' | '!' | '?' | '…' | '。' | '！' | '？'))
}

/// A unique id per stream so mic and speaker keep independent VAD state.
fn new_stream_id() -> String {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("stream_{}_{}", millis, rand::random::<u32>())
}
