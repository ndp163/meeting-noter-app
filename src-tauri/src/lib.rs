mod audio;
mod bridges;
mod commands;
mod config;
mod recorder;
mod types;

use anyhow::Result;
use commands::{start_transcription, stop_transcription, transcription_status, RecorderState};

pub use config::AudioConfig;
pub use recorder::AudioRecorder as Recorder;

// Legacy entry point for backward compatibility
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() -> Result<()> {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(RecorderState::default())
        .invoke_handler(tauri::generate_handler![
            start_transcription,
            stop_transcription,
            transcription_status,
        ])
        .run(tauri::generate_context!())
        .map_err(|e| anyhow::anyhow!(e))
}
