use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{App, Emitter, Manager, Window, WindowEvent};

/// Bring the main window to the front (un-hides it after a close-to-tray).
fn show_main(app: &tauri::AppHandle) -> Option<tauri::WebviewWindow> {
    let w = app.get_webview_window("main")?;
    let _ = w.show();
    let _ = w.set_focus();
    Some(w)
}

/// Build the menu bar tray icon. Menu items show the window, check for updates,
/// or quit.
pub fn setup(app: &App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Show Noter", true, None::<&str>)?;
    let check = MenuItem::with_id(
        app,
        "check_update",
        "Check for Updates…",
        true,
        None::<&str>,
    )?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &check, &sep, &quit])?;
    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                show_main(app);
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
        .build(app)?;
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
