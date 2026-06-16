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
use crate::audio::constants::{MIN_CHUNK_SAMPLES, SAMPLE_RATE_16KHZ};
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
    // Total 16kHz samples consumed so far, i.e. the current playback position
    // in the recording. Used to stamp each segment's start offset.
    let mut cumulative_samples: usize = 0;

    'outer: while let Ok(chunk) = rx.recv() {
        let samples = resampler.process(&chunk);

        let voice_prob = match transcriber.vad.process(&stream_id, &samples) {
            Ok(prob) => prob,
            Err(e) => {
                tracing::error!(stream = %stream_id, error = %e, "VAD processing failed");
                continue;
            }
        };

        cumulative_samples += samples.len();

        for mut segment in segmenter.push(voice_prob, &samples) {
            // The segment ends at the current position; subtract its length
            // (which includes pre-roll) to get where its speech began.
            let start_sample = cumulative_samples.saturating_sub(segment.audio.len());
            segment.start_sec = start_sample as f32 / SAMPLE_RATE_16KHZ as f32;
            if segment_tx.send(segment).is_err() {
                tracing::error!(stream = %stream_id, "Transcription worker died");
                break 'outer;
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
            Ok(text) if !text.trim().is_empty() => {
                on_result(TranscriptionResult {
                    text: text.trim().to_string(),
                    finality: segment.finality,
                    duration_sec,
                    start_sec: segment.start_sec,
                });
            }
            Ok(_) => {}
            Err(e) => {
                tracing::error!(error = %e, finality = ?segment.finality, "Transcription failed")
            }
        }
    }
}

/// A unique id per stream so mic and speaker keep independent VAD state.
fn new_stream_id() -> String {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("stream_{}_{}", millis, rand::random::<u32>())
}
