use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::image::Image;
use tauri::menu::{AboutMetadata, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Emitter, Listener, Manager, Window, WindowEvent, Wry};

/// Live recording state, mirrored from the frontend (the source of truth for
/// start/stop) via the `tray://recording-state` event.
#[derive(Default)]
struct Recording {
    active: bool,
    /// Set when recording starts; drives the live `MM:SS` header timer.
    started: Option<Instant>,
}

/// Payload of `tray://recording-state`, emitted by the frontend whenever the
/// capture toggle flips.
#[derive(serde::Deserialize)]
struct RecordingStatePayload {
    active: bool,
}

/// Mutable menu handles + icons the tray re-renders when recording state
/// changes. Held in Tauri-managed state so the event listener and the 1 s
/// timer can both reach them.
struct TrayState {
    status: MenuItem<Wry>,
    toggle: MenuItem<Wry>,
    icon: TrayIcon<Wry>,
    idle_icon: Image<'static>,
    recording_icon: Image<'static>,
    recording: Mutex<Recording>,
}

impl TrayState {
    /// Toggle label + tray icon — only changes on a start/stop transition, so
    /// the per-second timer never re-uploads the (large) icon bitmap.
    fn apply_transition(&self) {
        let active = self.recording.lock().unwrap().active;
        if active {
            let _ = self.toggle.set_text("Stop Recording");
            let _ = self.icon.set_icon(Some(self.recording_icon.clone()));
            // Red must show as-is; template mode would flatten it to the bar tint.
            let _ = self.icon.set_icon_as_template(false);
        } else {
            let _ = self.toggle.set_text("Start Recording");
            let _ = self.icon.set_icon(Some(self.idle_icon.clone()));
            let _ = self.icon.set_icon_as_template(true);
        }
    }

    /// Status header. Cheap; called every second while recording so the
    /// `MM:SS` clock ticks.
    fn tick(&self) {
        let rec = self.recording.lock().unwrap();
        if rec.active {
            let secs = rec.started.map(|s| s.elapsed().as_secs()).unwrap_or(0);
            let _ = self
                .status
                .set_text(format!("● Recording · {:02}:{:02}", secs / 60, secs % 60));
        } else {
            let _ = self.status.set_text("Idle");
        }
    }
}

/// Bring the main window to the front (un-hides it after a close-to-tray).
fn show_main(app: &AppHandle) -> Option<tauri::WebviewWindow> {
    let w = app.get_webview_window("main")?;
    let _ = w.unminimize();
    let _ = w.show();
    let _ = w.set_focus();
    Some(w)
}

/// Apple system red (#FF3B30) — the recording-state glyph tint.
const REC_RGB: [u8; 3] = [255, 59, 48];

/// Recolour a glyph mask's opaque pixels (makes the red recording variant).
fn tint(mask: &Image, rgb: [u8; 3]) -> Image<'static> {
    let (w, h) = (mask.width(), mask.height());
    let mut px = mask.rgba().to_vec();
    for i in (0..px.len()).step_by(4) {
        if px[i + 3] > 0 {
            px[i] = rgb[0];
            px[i + 1] = rgb[1];
            px[i + 2] = rgb[2];
        }
    }
    Image::new_owned(px, w, h)
}

/// Build the menu-bar tray icon. The menu surfaces recording state and the
/// app's primary actions; left-clicking the icon opens the window, right-click
/// (or any click with no window) shows the menu.
pub fn setup(app: &App) -> tauri::Result<()> {
    // Status header — disabled, acts as a live state label.
    let status = MenuItem::with_id(app, "status", "Idle", false, None::<&str>)?;
    let toggle = MenuItem::with_id(app, "toggle_recording", "Start Recording", true, None::<&str>)?;
    let open = MenuItem::with_id(app, "show", "Open Noter", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, Some("Cmd+,"))?;
    let check = MenuItem::with_id(app, "check_update", "Check for Updates…", true, None::<&str>)?;
    let about = PredefinedMenuItem::about(
        app,
        Some("About Meeting Noter"),
        Some(AboutMetadata {
            name: Some("Meeting Noter".into()),
            ..Default::default()
        }),
    )?;
    let quit = MenuItem::with_id(app, "quit", "Quit Meeting Noter", true, Some("Cmd+Q"))?;

    let menu = Menu::with_items(
        app,
        &[
            &status,
            &PredefinedMenuItem::separator(app)?,
            &toggle,
            &open,
            &PredefinedMenuItem::separator(app)?,
            &settings,
            &check,
            &PredefinedMenuItem::separator(app)?,
            &about,
            &quit,
        ],
    )?;

    // A bold monochrome cassette glyph — the macOS-standard menu-bar look.
    // Borderless, baked at exactly 36×36 (tray-icon renders at a fixed 18pt →
    // 36px on Retina, so this maps 1:1 with zero runtime resampling); the red
    // variant marks an active recording. Template mode auto-inverts it for
    // light/dark bars.
    let glyph = tauri::include_image!("icons/tray-glyph.png");
    let idle_icon = Image::new_owned(glyph.rgba().to_vec(), glyph.width(), glyph.height());
    let recording_icon = tint(&idle_icon, REC_RGB);

    let icon = TrayIconBuilder::new()
        .icon(idle_icon.clone())
        .icon_as_template(true)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            // Recording is frontend-orchestrated (meeting creation, language);
            // mirror the alert-overlay pattern and let the webview toggle it.
            "toggle_recording" => {
                let _ = app.emit("tray://toggle-recording", ());
            }
            "show" => {
                show_main(app);
            }
            "settings" => {
                if let Some(w) = show_main(app) {
                    let _ = w.emit("tray://settings", ());
                }
            }
            // The updater runs in the webview — surface the window and let the
            // frontend open Settings → Updates and run the check.
            "check_update" => {
                if let Some(w) = show_main(app) {
                    let _ = w.emit("tray://check-update", ());
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // Left-click the icon → open the window. Menu is right-click only.
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        })
        .build(app)?;

    app.manage(TrayState {
        status,
        toggle,
        icon,
        idle_icon,
        recording_icon,
        recording: Mutex::new(Recording::default()),
    });

    // Mirror frontend recording state into the menu (header, toggle, icon).
    let handle = app.handle().clone();
    app.listen("tray://recording-state", move |event| {
        let Ok(payload) = serde_json::from_str::<RecordingStatePayload>(event.payload()) else {
            return;
        };
        let state = handle.state::<TrayState>();
        {
            let mut rec = state.recording.lock().unwrap();
            if payload.active && !rec.active {
                rec.started = Some(Instant::now());
            } else if !payload.active {
                rec.started = None;
            }
            rec.active = payload.active;
        }
        state.apply_transition();
        state.tick();
    });

    // Tick the live `MM:SS` header once a second while recording.
    let handle = app.handle().clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(1));
        if let Some(state) = handle.try_state::<TrayState>() {
            if state.recording.lock().unwrap().active {
                state.tick();
            }
        }
    });

    Ok(())
}

/// Closing the main window hides it instead of quitting; the app keeps running
/// in the background (recording, meeting detection).
pub fn on_window_event(window: &Window, event: &WindowEvent) {
    if let WindowEvent::CloseRequested { api, .. } = event {
        if window.label() == "main" {
            let _ = window.hide();
            api.prevent_close();
        }
    }
}
