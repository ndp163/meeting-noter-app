//! One-time model prefetch for the onboarding screen.
//!
//! Downloads the ASR/VAD and diarizer models up front so the first real
//! transcription or diarization doesn't pay the download cost. Emits coarse
//! `setup://stage` events ("preparing-transcription", "preparing-speaker",
//! "ready") for the onboarding UI. There is no per-byte progress because the
//! FluidAudio library does not expose download progress to callers.

use crate::bridges::{create_transcriber, FluidAudio};
use crate::config::AudioConfig;
use serde::Serialize;
use tauri::{command, AppHandle, Emitter};

#[derive(Clone, Serialize)]
struct StageEvent {
    stage: String,
}

fn emit_stage(app: &AppHandle, stage: &str) {
    let _ = app.emit(
        "setup://stage",
        StageEvent {
            stage: stage.to_string(),
        },
    );
}

/// Whether all models are already cached, so onboarding can be skipped.
#[command]
pub fn models_ready() -> bool {
    FluidAudio::models_present()
}

/// Download both model sets, emitting a coarse stage per set. Errors propagate
/// so the onboarding screen can show a retry instead of failing silently.
#[command]
#[tracing::instrument(skip(app))]
pub async fn prefetch_models(app: AppHandle) -> Result<(), String> {
    emit_stage(&app, "preparing-transcription");
    let transcriber = create_transcriber(AudioConfig::default().engine);
    transcriber.asr.initialize().await?;

    emit_stage(&app, "preparing-speaker");
    FluidAudio::new().prefetch_diarizer().await?;

    emit_stage(&app, "ready");
    Ok(())
}
