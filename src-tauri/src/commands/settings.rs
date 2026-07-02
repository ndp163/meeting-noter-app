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

/// Realtime translation config: whether it's on, and the target BCP-47 language
/// (empty = unset). Source language is the meeting's ASR language.
pub fn translate_config() -> (bool, String) {
    let map = load();
    let enabled = map
        .get("translate_enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let target = map
        .get("translate_target")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    (enabled, target)
}

/// Transcript display mode for translations: `"original"`, `"translated"` or
/// `"both"`. Purely a frontend view pref, persisted here for durability.
fn translate_view() -> String {
    load()
        .get("translate_view")
        .and_then(|v| v.as_str())
        .unwrap_or("both")
        .to_string()
}

#[derive(serde::Serialize)]
pub struct TranslateConfig {
    pub enabled: bool,
    pub target: String,
    pub view: String,
}

#[command]
pub fn get_translate_config() -> TranslateConfig {
    let (enabled, target) = translate_config();
    TranslateConfig {
        enabled,
        target,
        view: translate_view(),
    }
}

#[command]
pub fn set_translate_config(enabled: bool, target: String, view: String) -> Result<(), String> {
    let mut map = load();
    map.insert("translate_enabled".into(), Value::Bool(enabled));
    map.insert("translate_target".into(), Value::String(target));
    map.insert("translate_view".into(), Value::String(view));
    save(&map)
}
