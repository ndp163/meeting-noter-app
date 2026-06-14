//! Offline speaker diarization for a finished meeting.
//!
//! Runs on demand (when the user opens the Diarization tab). Prefers the
//! per-source tracks written during recording: `speaker.wav` (remote audio) is
//! diarized into distinct speakers, and `mic.wav` is transcribed as the known
//! "You". Older meetings without side tracks fall back to diarizing the mixed
//! `audio.wav`, where your own voice becomes one of the clustered speakers.

use crate::bridges::FluidAudio;
use crate::paths;
use crate::types::DiarizedSegment;
use serde::Deserialize;
use tauri::command;

/// Raw segment shape returned by the Swift bridge (0-based speaker index).
#[derive(Debug, Deserialize)]
struct RawSegment {
    speaker: i32,
    start: f32,
    end: f32,
    text: String,
}

#[command]
#[tracing::instrument]
pub async fn diarize_meeting(meeting_id: String) -> Result<Vec<DiarizedSegment>, String> {
    let dir = paths::get_meeting_dir(&meeting_id);
    let speaker_wav = dir.join("speaker.wav");
    let mic_wav = dir.join("mic.wav");
    let audio_wav = dir.join("audio.wav");

    let fluid = FluidAudio::new();
    let mut segments: Vec<DiarizedSegment> = Vec::new();

    if speaker_wav.exists() {
        tracing::info!("Diarizing per-source tracks for meeting {}", meeting_id);
        let remote = run(&fluid, &speaker_wav, true).await?;
        segments.extend(remote.into_iter().map(|r| DiarizedSegment {
            speaker_id: format!("remote-{}", r.speaker),
            label: format!("Speaker {}", r.speaker + 1),
            start: r.start,
            end: r.end,
            text: r.text,
        }));

        if mic_wav.exists() {
            // Diarize the mic track too: the diarizer's VAD-based segmentation
            // gives accurate speech boundaries, whereas token-duration splitting
            // inflates segments across trailing silence. The mic is a single
            // known speaker, so collapse every cluster to "You".
            let mic = run(&fluid, &mic_wav, true).await?;
            segments.extend(mic.into_iter().map(|r| DiarizedSegment {
                speaker_id: "you".to_string(),
                label: "You".to_string(),
                start: r.start,
                end: r.end,
                text: r.text,
            }));
        }
    } else if audio_wav.exists() {
        tracing::info!("No side tracks; diarizing mixed audio for meeting {}", meeting_id);
        let mixed = run(&fluid, &audio_wav, true).await?;
        segments.extend(mixed.into_iter().map(|r| DiarizedSegment {
            speaker_id: format!("remote-{}", r.speaker),
            label: format!("Speaker {}", r.speaker + 1),
            start: r.start,
            end: r.end,
            text: r.text,
        }));
    } else {
        return Err(format!("No audio found for meeting {}", meeting_id));
    }

    segments.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap_or(std::cmp::Ordering::Equal));
    tracing::info!("Diarization produced {} segments", segments.len());
    Ok(segments)
}

async fn run(fluid: &FluidAudio, path: &std::path::Path, diarize: bool) -> Result<Vec<RawSegment>, String> {
    let path_str = path
        .to_str()
        .ok_or_else(|| "Invalid audio path".to_string())?;
    let json = fluid.diarize_file(path_str, diarize).await?;
    serde_json::from_str(&json).map_err(|e| format!("Failed to parse diarization result: {}", e))
}
