//! Realtime translation commands (Apple Translation framework, on-device).
//!
//! Config (enabled + target language) lives in [`settings`](super::settings).
//! Translation during a session happens in [`crate::session`]; these commands
//! just cover availability checks and the one-time pack download.

use crate::bridges::translate;
use tauri::command;

/// Availability of a translation pair: `"installed"` | `"supported"` |
/// `"unsupported"`. Errors on macOS < 26.
#[command]
pub async fn translate_status(source: String, target: String) -> Result<String, String> {
    translate::status(&source, &target).await
}

/// Trigger the one-time pack-download consent sheet, resolving when done.
#[command]
pub async fn translate_download(source: String, target: String) -> Result<(), String> {
    translate::download(&source, &target).await
}
