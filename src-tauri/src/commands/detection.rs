//! Thin IPC layer over meeting detection state.

use crate::meeting_detector::DetectorState;
use tauri::{AppHandle, Emitter, Manager, State};

/// Returns the display name of the meeting app currently using the mic, if any.
#[tauri::command]
pub fn meeting_detection_status(state: State<'_, DetectorState>) -> Option<String> {
    state.detected_app()
}

/// Invoked by the always-on-top alert overlay when the user clicks
/// "Start recording": brings the main window forward, tells it to start the
/// recording flow, and closes the overlay.
#[tauri::command]
pub fn start_recording_from_alert(app: AppHandle) {
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.unminimize();
        let _ = main.show();
        let _ = main.set_focus();
        let _ = main.emit("start-recording-from-alert", ());
    }
    if let Some(alert) = app.get_webview_window("alert") {
        let _ = alert.close();
    }
}

/// Invoked by the alert overlay's dismiss button: just closes the overlay.
#[tauri::command]
pub fn dismiss_alert(app: AppHandle) {
    if let Some(alert) = app.get_webview_window("alert") {
        let _ = alert.close();
    }
}
