//! Small persisted key/value settings (`settings.json` in the app data dir).
//! Currently just the summary provider; kept generic so future toggles can
//! share the file.

use crate::paths::get_app_data_dir;
use serde_json::{Map, Value};
use std::path::PathBuf;
use tauri::command;

fn settings_path() -> PathBuf {
    get_app_data_dir().join("settings.json")
}

fn load() -> Map<String, Value> {
    std::fs::read_to_string(settings_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save(map: &Map<String, Value>) -> Result<(), String> {
    let json = serde_json::to_string_pretty(map).map_err(|e| e.to_string())?;
    std::fs::write(settings_path(), json).map_err(|e| format!("write settings: {e}"))
}

/// The configured summary provider, defaulting to `"claude"`.
/// `"claude"` → Claude Code CLI · `"local"` → on-device MLX model.
pub fn summary_provider() -> String {
    load()
        .get("summary_provider")
        .and_then(|v| v.as_str())
        .unwrap_or("claude")
        .to_string()
}

#[command]
pub fn get_summary_provider() -> String {
    summary_provider()
}

#[command]
pub fn set_summary_provider(provider: String) -> Result<(), String> {
    if provider != "claude" && provider != "local" {
        return Err(format!("unknown summary provider: {provider}"));
    }
    let mut map = load();
    map.insert("summary_provider".into(), Value::String(provider));
    save(&map)
}
