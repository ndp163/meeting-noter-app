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

/// Progress event payload for `diarization://progress`, fraction 0.0–1.0 across
/// all tracks of one meeting.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DiarizationProgress {
    meeting_id: String,
    fraction: f64,
}

/// Read the language the meeting was recorded with from its data.json, falling
/// back to English so older meetings (no `language` field) keep working.
fn meeting_language(meeting_id: &str) -> String {
    let path = paths::get_meeting_data_path(meeting_id);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|c| serde_json::from_str::<Meeting>(&c).ok())
        .and_then(|m| m.language)
        .unwrap_or_else(|| "en".to_string())
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

    // Offline ASR must use the same language model the meeting was recorded with.
    let language = meeting_language(&meeting_id);
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
            let remote = run(
                &fluid,
                &speaker_wav,
                remote_speakers,
                &language,
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
            let mic = run(
                &fluid,
                &mic_wav,
                1,
                &language,
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
    tracing::info!("Diarization produced {} segments", segments.len());
    Ok(segments)
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
