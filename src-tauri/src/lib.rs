mod audio;
mod bridges;
mod commands;
mod config;
pub mod logging;
mod meeting_detector;
pub mod paths;
mod recorder;
mod session;
mod tray;
mod types;

use anyhow::Result;
use commands::{
    start_transcription, stop_transcription, transcription_status, meeting_detection_status,
    start_recording_from_alert, dismiss_alert,
    get_meetings, get_meeting_detail, save_meeting, delete_meeting, get_meeting_audio_path,
    diarize_meeting,
    models_ready, models_manifest_url, installed_languages, download_language, delete_language,
    claude_available, generate_title, summarize_meeting, translate_summary,
};
use session::RecorderState;

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
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(RecorderState::default())
        .manage(meeting_detector::DetectorState::default())
        .setup(|app| {
            meeting_detector::spawn(app.handle().clone());
            tray::setup(app)?;
            Ok(())
        })
        .on_window_event(tray::on_window_event)
        .invoke_handler(tauri::generate_handler![
            start_transcription,
            stop_transcription,
            transcription_status,
            meeting_detection_status,
            start_recording_from_alert,
            dismiss_alert,
            get_meetings,
            get_meeting_detail,
            save_meeting,
            delete_meeting,
            get_meeting_audio_path,
            diarize_meeting,
            models_ready,
            models_manifest_url,
            installed_languages,
            download_language,
            delete_language,
            claude_available,
            generate_title,
            summarize_meeting,
            translate_summary,
        ])
        .run(tauri::generate_context!())
        .map_err(|e| anyhow::anyhow!(e))
}
