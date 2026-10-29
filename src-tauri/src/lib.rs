mod audio;
mod bridges;
mod commands;
mod config;
pub mod logging;
mod meeting_detector;
mod mlx;
pub mod paths;
mod recorder;
mod session;
mod tray;
mod types;

use anyhow::Result;
use commands::{
    start_transcription, stop_transcription, transcription_status, meeting_detection_status,
    start_recording_from_alert, dismiss_alert,
    show_caption_window, hide_caption_window, set_caption_click_through,
    get_meetings, get_meeting_detail, save_meeting, delete_meeting, delete_meetings,
    purge_expired_meetings, get_meeting_audio_path,
    get_recordings_dir_info, pick_folder, set_recordings_dir, open_recordings_dir, reveal_meeting_audio,
    diarize_meeting,
    models_ready, models_manifest_url, installed_languages, download_language, delete_language,
    cancel_download_language,
    claude_available, generate_title, summarize_meeting, translate_summary,
    local_model_available, download_local_model, delete_local_model,
    get_summary_provider, set_summary_provider, get_retention_days, set_retention_days,
    get_translate_config, set_translate_config, translate_status, translate_download,
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
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(RecorderState::default())
        .manage(meeting_detector::DetectorState::default())
        .setup(|app| {
            // Apply the custom recordings folder before anything touches disk.
            if let Some(dir) = commands::settings::recordings_dir_setting() {
                let dir = std::path::PathBuf::from(dir);
                commands::meetings::allow_recordings_dir(app.handle(), &dir);
                paths::set_recordings_dir_override(Some(dir));
            }
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
            show_caption_window,
            hide_caption_window,
            set_caption_click_through,
            get_meetings,
            get_meeting_detail,
            save_meeting,
            delete_meeting,
            delete_meetings,
            purge_expired_meetings,
            get_meeting_audio_path,
            get_recordings_dir_info,
            pick_folder,
            set_recordings_dir,
            open_recordings_dir,
            reveal_meeting_audio,
            diarize_meeting,
            models_ready,
            models_manifest_url,
            installed_languages,
            download_language,
            delete_language,
            cancel_download_language,
            claude_available,
            generate_title,
            summarize_meeting,
            translate_summary,
            local_model_available,
            download_local_model,
            delete_local_model,
            get_summary_provider,
            set_summary_provider,
            get_translate_config,
            set_translate_config,
            get_retention_days,
            set_retention_days,
            translate_status,
            translate_download,
        ])
        .run(tauri::generate_context!())
        .map_err(|e| anyhow::anyhow!(e))
}
