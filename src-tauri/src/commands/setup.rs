//! Per-language model management for onboarding and the in-app Models panel.
//!
//! Each transcription language needs its own ASR model (English → Parakeet v2,
//! Japanese → tdtJa) plus a shared diarizer. Models download on demand: the
//! onboarding screen installs the language(s) the user picks, and the Models
//! panel lets them add or remove languages later.
//!
//! `download_language` streams real byte-weighted progress via `setup://progress`
//! ({ language, fraction }), sourced from FluidAudio's own `progressHandler` —
//! files download to a temp path and move into the cache dir only on completion,
//! so polling dir size stalls then jumps.

use crate::bridges::FluidAudio;
use crate::types::MeetingLanguage;
use serde::Serialize;
use tauri::{command, AppHandle, Emitter};

/// Languages the app can transcribe. Mirrors the frontend `MeetingLanguage`.
const ALL_LANGUAGES: [MeetingLanguage; 2] = [MeetingLanguage::En, MeetingLanguage::Ja];

#[derive(Clone, Serialize)]
struct ProgressEvent {
    language: MeetingLanguage,
    fraction: f64,
}

fn emit_progress(app: &AppHandle, language: MeetingLanguage, fraction: f64) {
    let _ = app.emit("setup://progress", ProgressEvent { language, fraction });
}

/// Manifest URL for the self-hosted model bucket. Single source of truth so the
/// frontend never hardcodes (and drifts from) the version.
#[command]
pub fn models_manifest_url() -> &'static str {
    crate::config::MODELS_MANIFEST_URL
}

/// Whether at least one language is installed, so onboarding can be skipped.
#[command]
pub fn models_ready() -> bool {
    ALL_LANGUAGES
        .iter()
        .any(|lang| FluidAudio::model_installed(lang.as_code()))
}

/// Which languages are currently installed (ASR + diarizer cached on disk).
#[command]
pub fn installed_languages() -> Vec<MeetingLanguage> {
    ALL_LANGUAGES
        .iter()
        .copied()
        .filter(|lang| FluidAudio::model_installed(lang.as_code()))
        .collect()
}

/// Download the model for one language, streaming `setup://progress`. Errors
/// propagate so the UI can show a retry.
#[command]
#[tracing::instrument(skip(app))]
pub async fn download_language(app: AppHandle, language: MeetingLanguage) -> Result<(), String> {
    let app_progress = app.clone();
    FluidAudio::download_language_with_progress(language.as_code(), move |fraction| {
        emit_progress(&app_progress, language, fraction);
    })
    .await
}

/// Remove a language's ASR model to reclaim disk (shared diarizer is kept).
#[command]
#[tracing::instrument]
pub fn delete_language(language: MeetingLanguage) -> Result<(), String> {
    if FluidAudio::delete_language(language.as_code()) {
        Ok(())
    } else {
        Err(format!("Failed to delete {} model", language.as_code()))
    }
}
