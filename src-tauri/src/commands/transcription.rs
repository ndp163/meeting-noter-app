//! Thin IPC layer over the recording session lifecycle in [`crate::session`].

use crate::session::{self, RecorderState, RecorderStatusPayload};
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn start_transcription(
    app_handle: AppHandle,
    state: State<'_, RecorderState>,
    meeting_id: String,
    language: Option<String>,
) -> Result<(), String> {
    session::start(app_handle, &state, meeting_id, language).await
}

#[tauri::command]
pub async fn stop_transcription(state: State<'_, RecorderState>) -> Result<(), String> {
    session::stop(&state).await
}

#[tauri::command]
pub async fn transcription_status(
    state: State<'_, RecorderState>,
) -> Result<RecorderStatusPayload, String> {
    Ok(session::status(&state).await)
}
