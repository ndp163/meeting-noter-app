//! Detects when a meeting app starts using the microphone and reminds the
//! user to record.
//!
//! Polls the CoreAudio process object list (macOS 14.4+, same requirement as
//! the process tap used for speaker capture) every few seconds and checks
//! whether a whitelisted app has an active input stream. The current result
//! is kept in [`DetectorState`] for the frontend to poll; on the rising edge
//! (no meeting -> meeting) it bounces the dock and, when the main window is
//! not focused, floats an always-on-top overlay window carrying a "Start
//! recording" button above the meeting app. Reminders are suppressed while a
//! recording session is active.

use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use cidre::core_audio as ca;
use tauri::{AppHandle, Emitter, Manager, UserAttentionType};
use tauri_plugin_notification::NotificationExt;

use crate::session::RecorderState;

const POLL_INTERVAL: Duration = Duration::from_secs(2);

/// Bundle ID prefixes that indicate a meeting (or browser tab) is using the
/// mic. Prefixes, not exact IDs: browsers and Electron apps often capture
/// audio in helper processes (e.g. `com.google.Chrome.helper`).
const MEETING_APPS: &[(&str, &str)] = &[
    ("us.zoom.xos", "Zoom"),
    ("com.microsoft.teams", "Microsoft Teams"),
    ("Cisco-Systems.Spark", "Webex"),
    ("com.tinyspeck.slackmacgap", "Slack"),
    ("com.hnc.Discord", "Discord"),
    ("com.google.Chrome", "Chrome"),
    ("com.apple.Safari", "Safari"),
    ("com.apple.WebKit.GPU", "Safari"), // Safari captures audio in the WebKit GPU process
    ("org.mozilla.firefox", "Firefox"),
    ("org.mozilla.plugincontainer", "Firefox"),
    ("com.microsoft.edgemac", "Edge"),
    ("company.thebrowser.Browser", "Arc"),
    ("com.brave.Browser", "Brave"),
];

/// Last detection result, polled by the frontend.
#[derive(Default)]
pub struct DetectorState {
    detected: Mutex<Option<String>>,
}

impl DetectorState {
    pub fn detected_app(&self) -> Option<String> {
        self.detected.lock().unwrap().clone()
    }

    fn set(&self, app_name: Option<String>) {
        *self.detected.lock().unwrap() = app_name;
    }
}

/// Spawn the detection loop on its own thread.
pub fn spawn(app: AppHandle) {
    if let Err(e) = app.notification().request_permission() {
        tracing::warn!(error = %e, "Notification permission request failed");
    }

    thread::spawn(move || {
        tracing::info!("Meeting detector started");
        let mut was_in_meeting = false;

        loop {
            thread::sleep(POLL_INTERVAL);

            let app_name = match detect_meeting_app() {
                Ok(name) => name,
                Err(e) => {
                    tracing::warn!(error = %e, "Meeting detection unavailable, stopping");
                    return;
                }
            };

            let in_meeting = app_name.is_some();
            if in_meeting != was_in_meeting {
                tracing::info!(?app_name, "Mic-use detection changed");
            }
            app.state::<DetectorState>().set(app_name.clone());
            match (&app_name, was_in_meeting) {
                // Rising edge: a meeting just started using the mic.
                (Some(name), false) if !is_recording(&app) => show_reminder(&app, name),
                // Falling edge: mic released, tear down the overlay if it's up.
                (None, true) => close_alert_window(&app),
                _ => {}
            }
            was_in_meeting = in_meeting;
        }
    });
}

/// Returns the display name of the first whitelisted app with an active
/// input stream, if any.
fn detect_meeting_app() -> anyhow::Result<Option<String>> {
    let processes = ca::System::processes()?;

    for process in processes {
        if !process.is_running_input().unwrap_or(false) {
            continue;
        }
        let Ok(bundle_id) = process.bundle_id() else {
            continue;
        };
        let bundle_id = bundle_id.to_string();
        if let Some((_, name)) = MEETING_APPS
            .iter()
            .find(|(prefix, _)| bundle_id.starts_with(prefix))
        {
            return Ok(Some((*name).to_string()));
        }
        tracing::debug!(bundle_id, "Unrecognized app using the mic");
    }

    Ok(None)
}

fn is_recording(app: &AppHandle) -> bool {
    // If the lock is contended a session is being started/stopped right now;
    // treat that as recording to avoid a spurious reminder.
    app.state::<RecorderState>().is_active_now().unwrap_or(true)
}

fn show_reminder(app: &AppHandle, app_name: &str) {
    tracing::info!(app_name, "Meeting detected");

    // Bounce the dock icon to draw attention.
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.request_user_attention(Some(UserAttentionType::Informational));
    }

    // Float the always-on-top overlay with the Start button. It's the single
    // reminder surface (no in-app banner), so it shows regardless of focus.
    show_alert_window(app, app_name);
}

/// Create (or re-show) the always-on-top overlay window that carries the
/// "Start recording" button. Window creation must run on the main thread.
fn show_alert_window(app: &AppHandle, app_name: &str) {
    let app = app.clone();
    let name = app_name.to_string();
    let _ = app.clone().run_on_main_thread(move || {
        if let Some(win) = app.get_webview_window("alert") {
            let _ = win.emit("meeting-detected", &name);
            let _ = win.show();
            return;
        }

        let built = tauri::WebviewWindowBuilder::new(
            &app,
            "alert",
            tauri::WebviewUrl::App("index.html".into()),
        )
        .title("Meeting detected")
        .inner_size(380.0, 60.0)
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible_on_all_workspaces(true)
        .build();

        match built {
            Ok(win) => {
                // Park it in the top-right corner of the primary display.
                if let Ok(Some(monitor)) = win.primary_monitor() {
                    let size = monitor.size();
                    let scale = monitor.scale_factor();
                    let x = size.width as f64 - (380.0 + 20.0) * scale;
                    let y = 40.0 * scale;
                    let _ = win.set_position(tauri::PhysicalPosition::new(x, y));
                }
                // The JS also fetches the name via `meeting_detection_status`
                // on mount, so a missed event (listener not yet attached) is fine.
                let _ = win.emit("meeting-detected", &name);
            }
            Err(e) => tracing::warn!(error = %e, "Failed to create alert window"),
        }
    });
}

/// Close the overlay window if it's open. Runs on the main thread.
fn close_alert_window(app: &AppHandle) {
    let app = app.clone();
    let _ = app.clone().run_on_main_thread(move || {
        if let Some(win) = app.get_webview_window("alert") {
            let _ = win.close();
        }
    });
}
