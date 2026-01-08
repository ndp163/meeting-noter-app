mod audio;
mod bridges;
mod commands;
mod config;
pub mod logging;
pub mod paths;
pub mod permissions;
mod recorder;
mod types;

use anyhow::Result;
use commands::{
    start_transcription, stop_transcription, transcription_status, RecorderState,
    get_meetings, get_meeting_detail, save_meeting, delete_meeting, get_meeting_audio_path,
    check_permissions, check_screen_recording, request_screen_recording, open_permission_settings,
};

pub use config::AudioConfig;
pub use recorder::AudioRecorder as Recorder;
pub use logging::{init_logging, LogLevel};

// Legacy entry point for backward compatibility
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() -> Result<()> {
    // Initialize logging
    init_logging(LogLevel::default());
    tracing::info!("Noter application starting...");
    
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(RecorderState::default())
        .invoke_handler(tauri::generate_handler![
            start_transcription,
            stop_transcription,
            transcription_status,
            get_meetings,
            get_meeting_detail,
            save_meeting,
            delete_meeting,
            get_meeting_audio_path,
            check_permissions,
            check_screen_recording,
            request_screen_recording,
            open_permission_settings,
        ])
        .run(tauri::generate_context!())
        .map_err(|e| anyhow::anyhow!(e))
}
