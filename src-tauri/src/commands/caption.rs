//! Lifecycle for the always-on-top live-caption overlay window.
//!
//! The caption window is a transparent, borderless webview (label `caption`)
//! that floats along the bottom of the screen over the meeting app. It listens
//! to `transcription://chunk` directly — the backend broadcasts that event to
//! all windows — so no audio state crosses this boundary; these commands only
//! create, show/hide, and toggle click-through on the window.

use tauri::{AppHandle, Manager};

const CAPTION_LABEL: &str = "caption";
/// Initial size only — the webview resizes itself to hug its content once
/// mounted (see `caption-window.tsx`), so this is just a small first frame.
const CAPTION_WIDTH: f64 = 600.0;
const CAPTION_HEIGHT: f64 = 64.0;
/// Gap (logical px) between the caption card and the bottom screen edge.
const CAPTION_BOTTOM_MARGIN: f64 = 80.0;

/// Create (or re-show) the caption overlay. Window creation must run on the
/// main thread. Mirrors `meeting_detector::show_alert_window`.
#[tauri::command]
pub fn show_caption_window(app: AppHandle) {
    let app = app.clone();
    let _ = app.clone().run_on_main_thread(move || {
        if let Some(win) = app.get_webview_window(CAPTION_LABEL) {
            let _ = win.show();
            return;
        }

        let built = tauri::WebviewWindowBuilder::new(
            &app,
            CAPTION_LABEL,
            tauri::WebviewUrl::App("index.html".into()),
        )
        .title("Live captions")
        .inner_size(CAPTION_WIDTH, CAPTION_HEIGHT)
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .focused(false)
        .visible_on_all_workspaces(true)
        .build();

        match built {
            Ok(win) => {
                // Park it centered along the bottom of the primary display.
                if let Ok(Some(monitor)) = win.primary_monitor() {
                    let size = monitor.size();
                    let scale = monitor.scale_factor();
                    let x = (size.width as f64 - CAPTION_WIDTH * scale) / 2.0;
                    let y = size.height as f64
                        - (CAPTION_HEIGHT + CAPTION_BOTTOM_MARGIN) * scale;
                    let _ = win.set_position(tauri::PhysicalPosition::new(x, y));
                }
                // The window stays interactive so the user can drag it anywhere
                // (see the `data-tauri-drag-region` root). It's a thin bottom
                // strip, so capturing clicks there is an acceptable trade-off.
            }
            Err(e) => tracing::warn!(error = %e, "Failed to create caption window"),
        }
    });
}

/// Hide the caption overlay without destroying it (keeps it warm for re-show).
#[tauri::command]
pub fn hide_caption_window(app: AppHandle) {
    if let Some(win) = app.get_webview_window(CAPTION_LABEL) {
        let _ = win.hide();
    }
}

/// Toggle input pass-through. When `ignore` is true the overlay is transparent
/// to clicks (they reach the meeting app); the JS sets it false on hover so its
/// own controls stay usable.
#[tauri::command]
pub fn set_caption_click_through(app: AppHandle, ignore: bool) {
    if let Some(win) = app.get_webview_window(CAPTION_LABEL) {
        let _ = win.set_ignore_cursor_events(ignore);
    }
}
