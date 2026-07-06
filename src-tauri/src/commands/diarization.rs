//! Offline speaker diarization for a finished meeting.
//!
//! Runs on demand (when the user opens the Diarization tab) via FluidAudio's
//! offline VBx pipeline. Prefers the per-source tracks written during
//! recording: `speaker.wav` (remote audio) is diarized into distinct speakers,
//! and `mic.wav` is diarized with a pinned single cluster as the known "You".
//! Older meetings without side tracks fall back to diarizing the mixed
//! `audio.wav`, where your own voice becomes one of the clustered speakers.
//! Emits `diarization://progress` ({ meetingId, fraction }) while running.

use crate::bridges::FluidAudio;
use crate::paths;
use crate::types::{DiarizedSegment, DiarizedWord, Meeting};
use serde::{Deserialize, Serialize};
use tauri::{command, AppHandle, Emitter};

/// Raw segment shape returned by the Swift bridge (0-based speaker index).
#[derive(Debug, Deserialize)]
struct RawSegment {
    speaker: i32,
    start: f32,
    end: f32,
    text: String,
    #[serde(default)]
    words: Vec<DiarizedWord>,
}

/// A speaker turn without text, from the diarize-only pass (no ASR).
#[derive(Debug, Deserialize)]
struct RawSpan {
    speaker: i32,
    start: f32,
    end: f32,
}

/// Words further than this (seconds) from every diarized span are dropped
/// instead of glued onto the nearest one — the Rust twin of the Swift
/// `maxTokenAttachDistance` used on the full (re-ASR) path.
const MAX_WORD_ATTACH_DISTANCE: f32 = 1.5;

/// Progress event payload for `diarization://progress`, fraction 0.0–1.0 across
/// all tracks of one meeting.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DiarizationProgress {
    meeting_id: String,
    fraction: f64,
}

/// Read a meeting's stored data.json, if parsable (absent/corrupt → `None`).
fn load_meeting(meeting_id: &str) -> Option<Meeting> {
    let path = paths::get_meeting_data_path(meeting_id);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|c| serde_json::from_str::<Meeting>(&c).ok())
}

/// All stored word timings for one source stream, in chronological order.
/// Empty for meetings recorded before word timings existed — the caller then
/// falls back to the full (re-ASR) diarization path for that track.
fn transcript_words(meeting: Option<&Meeting>, source: &str) -> Vec<DiarizedWord> {
    meeting
        .map(|m| {
            m.transcript
                .iter()
                .filter(|msg| msg.source.as_deref() == Some(source))
                .flat_map(|msg| msg.words.iter().cloned())
                .collect()
        })
        .unwrap_or_default()
}

#[command]
#[tracing::instrument(skip(app))]
pub async fn diarize_meeting(
    app: AppHandle,
    meeting_id: String,
    num_speakers: Option<u32>,
) -> Result<Vec<DiarizedSegment>, String> {
    let dir = paths::get_meeting_dir(&meeting_id);
    let speaker_wav = dir.join("speaker.wav");
    let mic_wav = dir.join("mic.wav");
    let audio_wav = dir.join("audio.wav");

    // A recording cut off mid-write (app killed / crashed) leaves WAV headers
    // unfinalized, which CoreAudio rejects outright — salvage them first.
    for wav in [&speaker_wav, &mic_wav, &audio_wav] {
        if wav.exists() {
            if let Err(e) = paths::repair_wav_header(wav) {
                tracing::warn!(path = %wav.display(), "WAV header check failed: {e}");
            }
        }
    }

    // Offline ASR must use the same language model the meeting was recorded
    // with (only needed on the fallback path that re-transcribes the file).
    let meeting = load_meeting(&meeting_id);
    let language = meeting
        .as_ref()
        .and_then(|m| m.language.clone())
        .unwrap_or_else(|| "en".to_string());
    // Expected number of remote speakers; <= 0 lets VBx cluster automatically.
    let remote_speakers = num_speakers.map(|n| n as i32).unwrap_or(-1);

    let fluid = FluidAudio::new();
    let mut segments: Vec<DiarizedSegment> = Vec::new();

    // A track file with no samples beyond the 44-byte header (e.g. nothing
    // ever played on the speakers, or the mic was disabled) must be treated
    // as absent — CoreAudio errors out on it and there is nothing to diarize.
    let has_remote = has_samples(&speaker_wav);
    let has_mic = has_samples(&mic_wav);

    if has_remote || has_mic {
        tracing::info!(has_remote, has_mic, "Diarizing per-source tracks for meeting {}", meeting_id);
        // Split the progress window across however many tracks will run.
        let scale = if has_remote && has_mic { 0.5 } else { 1.0 };

        if has_remote {
            let words = transcript_words(meeting.as_ref(), "speaker");
            let remote = diarize_track(
                &fluid,
                &speaker_wav,
                remote_speakers,
                &language,
                words,
                progress_emitter(&app, &meeting_id, 0.0, scale),
            )
            .await?;
            segments.extend(remote.into_iter().map(|r| DiarizedSegment {
                speaker_id: format!("remote-{}", r.speaker),
                label: format!("Speaker {}", r.speaker + 1),
                start: r.start,
                end: r.end,
                text: r.text,
                words: r.words,
            }));
        }

        if has_mic {
            // The mic track is a single known speaker: pin the cluster count to
            // 1 so the diarizer only does segmentation-quality boundary work
            // (accurate speech bounds) without wasted multi-speaker clustering.
            let words = transcript_words(meeting.as_ref(), "mic");
            let mic = diarize_track(
                &fluid,
                &mic_wav,
                1,
                &language,
                words,
                progress_emitter(&app, &meeting_id, 1.0 - scale, scale),
            )
            .await?;
            segments.extend(mic.into_iter().map(|r| DiarizedSegment {
                speaker_id: "you".to_string(),
                label: "You".to_string(),
                start: r.start,
                end: r.end,
                text: r.text,
                words: r.words,
            }));
        }
    } else if has_samples(&audio_wav) {
        tracing::info!("No side tracks; diarizing mixed audio for meeting {}", meeting_id);
        let mixed = run(
            &fluid,
            &audio_wav,
            remote_speakers,
            &language,
            progress_emitter(&app, &meeting_id, 0.0, 1.0),
        )
        .await?;
        segments.extend(mixed.into_iter().map(|r| DiarizedSegment {
            speaker_id: format!("remote-{}", r.speaker),
            label: format!("Speaker {}", r.speaker + 1),
            start: r.start,
            end: r.end,
            text: r.text,
            words: r.words,
        }));
    } else {
        return Err("This meeting has no captured audio to identify speakers in.".to_string());
    }

    segments.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap_or(std::cmp::Ordering::Equal));
    let segments = coalesce_turns(segments);
    tracing::info!("Diarization produced {} segments", segments.len());
    Ok(segments)
}

/// Longest silent gap (seconds) bridged when coalescing consecutive
/// same-speaker segments into one turn. Within-track merging only bridges 1s
/// (a breath); this display-level pass joins whole sentences of the same
/// speaker as long as nobody else spoke in between — without it the view
/// fragments into many consecutive rows with the same label. A long silence
/// (screen sharing, a break) still starts a new row.
const MAX_TURN_GAP_SEC: f32 = 30.0;

/// Merge consecutive segments of the SAME speaker on the final merged
/// timeline into one conversational turn. Only neighbours in the sorted list
/// merge, so any interjection by another speaker keeps the turns apart.
fn coalesce_turns(segments: Vec<DiarizedSegment>) -> Vec<DiarizedSegment> {
    let mut out: Vec<DiarizedSegment> = Vec::new();
    for seg in segments {
        if let Some(last) = out.last_mut() {
            if last.speaker_id == seg.speaker_id && seg.start - last.end <= MAX_TURN_GAP_SEC {
                last.end = last.end.max(seg.end);
                if !seg.text.is_empty() {
                    if !last.text.is_empty() {
                        last.text.push(' ');
                    }
                    last.text.push_str(&seg.text);
                }
                last.words.extend(seg.words);
                continue;
            }
        }
        out.push(seg);
    }
    out
}

/// Whether a WAV file exists and holds any samples beyond the 44-byte header.
/// A header-only track (source never produced audio) is useless to CoreAudio
/// and must be treated as missing.
fn has_samples(path: &std::path::Path) -> bool {
    std::fs::metadata(path).map(|m| m.len() > 44).unwrap_or(false)
}

/// Build a progress closure that maps one track's local fraction (0–1) into the
/// meeting-wide window `[offset, offset + scale]` and emits it to the frontend.
fn progress_emitter(
    app: &AppHandle,
    meeting_id: &str,
    offset: f64,
    scale: f64,
) -> impl Fn(f64) + Send + 'static {
    let app = app.clone();
    let meeting_id = meeting_id.to_string();
    move |fraction| {
        let _ = app.emit(
            "diarization://progress",
            DiarizationProgress {
                meeting_id: meeting_id.clone(),
                fraction: (offset + fraction * scale).clamp(0.0, 1.0),
            },
        );
    }
}

async fn run<F>(
    fluid: &FluidAudio,
    path: &std::path::Path,
    num_speakers: i32,
    language: &str,
    on_progress: F,
) -> Result<Vec<RawSegment>, String>
where
    F: Fn(f64) + Send + 'static,
{
    let path_str = path
        .to_str()
        .ok_or_else(|| "Invalid audio path".to_string())?;
    let json = fluid
        .diarize_file(path_str, num_speakers, language, on_progress)
        .await?;
    serde_json::from_str(&json).map_err(|e| format!("Failed to parse diarization result: {}", e))
}

/// Diarize one track, reusing the live transcript's word timings when they
/// exist: only the diarizer runs (no second ASR pass — roughly half the
/// cost) and the stored words are attributed to the returned speaker turns.
/// Tracks without stored words (older meetings) take the full path.
async fn diarize_track<F>(
    fluid: &FluidAudio,
    path: &std::path::Path,
    num_speakers: i32,
    language: &str,
    words: Vec<DiarizedWord>,
    on_progress: F,
) -> Result<Vec<RawSegment>, String>
where
    F: Fn(f64) + Send + 'static,
{
    if words.is_empty() {
        return run(fluid, path, num_speakers, language, on_progress).await;
    }

    tracing::info!(path = %path.display(), words = words.len(), "Diarizing with stored transcript words (no re-ASR)");
    let path_str = path
        .to_str()
        .ok_or_else(|| "Invalid audio path".to_string())?;
    let json = fluid
        .diarize_file_only(path_str, num_speakers, on_progress)
        .await?;
    let spans: Vec<RawSpan> = serde_json::from_str(&json)
        .map_err(|e| format!("Failed to parse diarization result: {}", e))?;
    Ok(bucket_words(spans, words))
}

/// Attribute stored transcript words to diarized speaker spans — the Rust
/// twin of the Swift token bucketing, but operating on already-grouped words
/// from the live transcription (both live on the recording's shared
/// timeline). Each word goes to the span containing its midpoint, or the
/// nearest span within [`MAX_WORD_ATTACH_DISTANCE`]; words in regions the
/// diarizer heard as silence are dropped rather than misattributed. Spans
/// that end up with no words are omitted (same as the Swift path).
fn bucket_words(spans: Vec<RawSpan>, words: Vec<DiarizedWord>) -> Vec<RawSegment> {
    let mut texts: Vec<String> = vec![String::new(); spans.len()];
    let mut span_words: Vec<Vec<DiarizedWord>> = vec![Vec::new(); spans.len()];

    for word in words {
        let mid = (word.start + word.end) / 2.0;
        let mut best: Option<(usize, f32)> = None;
        for (i, span) in spans.iter().enumerate() {
            let distance = if mid >= span.start && mid <= span.end {
                0.0
            } else if mid < span.start {
                span.start - mid
            } else {
                mid - span.end
            };
            if best.is_none_or(|(_, d)| distance < d) {
                best = Some((i, distance));
            }
            if distance == 0.0 {
                break;
            }
        }
        let Some((idx, distance)) = best else { continue };
        if distance > MAX_WORD_ATTACH_DISTANCE {
            continue;
        }
        if !texts[idx].is_empty() {
            texts[idx].push(' ');
        }
        texts[idx].push_str(&word.text);
        span_words[idx].push(word);
    }

    spans
        .into_iter()
        .zip(texts.into_iter().zip(span_words))
        .filter(|(_, (text, _))| !text.is_empty())
        .map(|(span, (text, words))| RawSegment {
            speaker: span.speaker,
            start: span.start,
            end: span.end,
            text,
            words,
        })
        .collect()
}
