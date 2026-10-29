use crate::paths;
use crate::types::Meeting;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{command, AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;
use crate::session::RecorderState;

#[derive(Debug, Serialize, Deserialize)]
pub struct MeetingsIndex {
    pub meetings: Vec<Meeting>,
}

fn get_recordings_dir() -> Result<PathBuf, String> {
    paths::ensure_recordings_dir()
        .map_err(|e| {
            tracing::error!("Failed to ensure recordings directory: {}", e);
            format!("Failed to access recordings directory: {}", e)
        })
}

fn get_index_path() -> Result<PathBuf, String> {
    Ok(paths::get_meetings_index_path())
}

#[tracing::instrument]
fn load_meetings_index() -> Result<MeetingsIndex, String> {
    let index_path = get_index_path()?;
    
    if !index_path.exists() {
        return Ok(MeetingsIndex {
            meetings: Vec::new(),
        });
    }

    let content = fs::read_to_string(&index_path)
        .map_err(|e| format!("Failed to read index.json: {}", e))?;
    
    let index: MeetingsIndex = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse index.json: {}", e))?;
    
    Ok(index)
}

#[tracing::instrument]
fn save_meetings_index(index: &MeetingsIndex) -> Result<(), String> {
    let index_path = get_index_path()?;
    let recordings_dir = get_recordings_dir()?;
    
    paths::ensure_dir(&recordings_dir)
        .map_err(|e| format!("Failed to create recordings directory: {}", e))?;

    let content = serde_json::to_string_pretty(index)
        .map_err(|e| format!("Failed to serialize index: {}", e))?;
    
    paths::write_atomic(&index_path, content.as_bytes())
        .map_err(|e| format!("Failed to write index.json: {}", e))?;
    
    tracing::debug!("Saved meetings index with {} entries", index.meetings.len());
    Ok(())
}

#[command]
#[tracing::instrument]
pub async fn get_meetings() -> Result<Vec<Meeting>, String> {
    let index = load_meetings_index()?;
    tracing::debug!("Loaded {} meetings", index.meetings.len());
    Ok(index.meetings)
}

#[command]
#[tracing::instrument]
pub async fn get_meeting_detail(meeting_id: String) -> Result<Meeting, String> {
    let data_path = paths::get_meeting_data_path(&meeting_id);

    if !data_path.exists() {
        tracing::warn!("Meeting {} not found", meeting_id);
        return Err(format!("Meeting {} not found", meeting_id));
    }

    let content = fs::read_to_string(&data_path)
        .map_err(|e| format!("Failed to read meeting data: {}", e))?;
    
    let meeting: Meeting = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse meeting data: {}", e))?;
    
    tracing::debug!("Loaded meeting detail: {}", meeting_id);
    Ok(meeting)
}

#[command]
#[tracing::instrument(skip(meeting))]
pub async fn save_meeting(meeting: Meeting) -> Result<(), String> {
    tracing::info!("Saving meeting: {}", meeting.id);
    
    let meeting_dir = paths::ensure_meeting_dir(&meeting.id)
        .map_err(|e| format!("Failed to create meeting directory: {}", e))?;

    // Save meeting data
    let data_path = meeting_dir.join("data.json");
    let content = serde_json::to_string_pretty(&meeting)
        .map_err(|e| format!("Failed to serialize meeting: {}", e))?;
    
    paths::write_atomic(&data_path, content.as_bytes())
        .map_err(|e| format!("Failed to write meeting data: {}", e))?;

    // Update index
    let mut index = load_meetings_index()?;
    
    if let Some(existing) = index.meetings.iter_mut().find(|m| m.id == meeting.id) {
        *existing = meeting.clone();
    } else {
        index.meetings.insert(0, meeting.clone());
    }
    
    save_meetings_index(&index)?;
    
    tracing::info!("Meeting {} saved successfully", meeting.id);
    Ok(())
}

/// Remove each meeting's directory, then drop them from the index in a single
/// write.
fn remove_meetings(ids: &[String]) -> Result<(), String> {
    if ids.is_empty() {
        return Ok(());
    }
    for id in ids {
        let meeting_dir = paths::get_meeting_dir(id);
        if meeting_dir.exists() {
            fs::remove_dir_all(&meeting_dir)
                .map_err(|e| format!("Failed to delete meeting directory: {}", e))?;
            tracing::info!("Deleted meeting directory: {}", meeting_dir.display());
        }
    }

    let mut index = load_meetings_index()?;
    index.meetings.retain(|m| !ids.contains(&m.id));
    save_meetings_index(&index)?;

    tracing::info!("Deleted {} meeting(s)", ids.len());
    Ok(())
}

#[command]
#[tracing::instrument]
pub async fn delete_meeting(meeting_id: String) -> Result<(), String> {
    remove_meetings(&[meeting_id])
}

#[command]
#[tracing::instrument]
pub async fn delete_meetings(meeting_ids: Vec<String>) -> Result<(), String> {
    remove_meetings(&meeting_ids)
}

/// Delete completed meetings older than the retention setting. Returns the ids
/// removed (empty when retention is "forever"). In-progress recordings are
/// never touched.
#[command]
#[tracing::instrument]
pub async fn purge_expired_meetings() -> Result<Vec<String>, String> {
    let days = crate::commands::settings::retention_days();
    if days == 0 {
        return Ok(Vec::new());
    }
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_millis() as i64;
    let cutoff = now_ms - (days as i64) * 86_400_000;

    let expired: Vec<String> = load_meetings_index()?
        .meetings
        .into_iter()
        .filter(|m| m.status != "recording" && m.created_at < cutoff)
        .map(|m| m.id)
        .collect();
    remove_meetings(&expired)?;
    Ok(expired)
}

#[command]
#[tracing::instrument]
pub async fn get_meeting_audio_path(meeting_id: String) -> Result<String, String> {
    let audio_path = paths::get_meeting_audio_path(&meeting_id);

    if !audio_path.exists() {
        tracing::warn!("Audio file not found for meeting {}", meeting_id);
        return Err(format!("Audio file not found for meeting {}", meeting_id));
    }

    // Salvage recordings cut off mid-write (unfinalized header) so playback
    // still works after a crash.
    if let Err(e) = paths::repair_wav_header(&audio_path) {
        tracing::warn!(path = %audio_path.display(), "WAV header check failed: {e}");
    }

    audio_path
        .to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Failed to convert path to string".to_string())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingsDirInfo {
    pub path: String,
    pub is_default: bool,
}

#[command]
pub fn get_recordings_dir_info() -> RecordingsDirInfo {
    let dir = paths::get_recordings_dir();
    RecordingsDirInfo {
        is_default: dir == paths::default_recordings_dir(),
        path: dir.to_string_lossy().into_owned(),
    }
}

/// Let the webview's asset protocol serve audio from `dir` — its static scope
/// only covers `$HOME`, so a folder on another volume would fail playback.
pub fn allow_recordings_dir(app: &AppHandle, dir: &std::path::Path) {
    if let Err(e) = app.asset_protocol_scope().allow_directory(dir, true) {
        tracing::warn!(dir = %dir.display(), "Failed to allow recordings dir in asset scope: {e}");
    }
}

/// Native folder picker; `None` when cancelled.
#[command]
pub async fn pick_folder(app: AppHandle) -> Option<String> {
    app.dialog()
        .file()
        .blocking_pick_folder()
        .and_then(|p| p.into_path().ok())
        .map(|p| p.to_string_lossy().into_owned())
}

/// Switch the recordings folder (`None` = back to the default), moving every
/// existing meeting into it. Refused while recording.
#[command]
#[tracing::instrument(skip(app, state))]
pub async fn set_recordings_dir(
    app: AppHandle,
    state: State<'_, RecorderState>,
    path: Option<String>,
) -> Result<(), String> {
    if state.is_active_now() != Some(false) {
        return Err("Stop the recording before changing the folder.".into());
    }
    let current = paths::get_recordings_dir();
    let target = match &path {
        Some(p) => PathBuf::from(p),
        None => paths::default_recordings_dir(),
    };
    if target == current {
        return Ok(());
    }
    if target.starts_with(&current) {
        return Err("Choose a folder outside the current recordings folder.".into());
    }
    if path.is_none() {
        paths::ensure_dir(&target).map_err(|e| format!("Failed to create folder: {e}"))?;
    }
    if !target.is_dir() {
        return Err(format!("{} is not a folder.", target.display()));
    }
    let probe = target.join(".noter-write-test");
    fs::write(&probe, b"")
        .and_then(|_| fs::remove_file(&probe))
        .map_err(|e| format!("Can't write to {}: {e}", target.display()))?;

    // Adopt a folder that already holds recordings only when there is nothing
    // to move; merging two indexes is not supported.
    let has_meetings = !load_meetings_index()?.meetings.is_empty();
    if has_meetings && target.join("index.json").exists() {
        return Err("That folder already contains Meeting Noter recordings.".into());
    }
    if current.is_dir() {
        paths::move_dir_contents(&current, &target)
            .map_err(|e| format!("Failed to move recordings: {e}"))?;
        let _ = fs::remove_dir(&current); // leave it if anything else is inside
    }

    crate::commands::settings::set_recordings_dir_setting(path.as_deref())?;
    paths::set_recordings_dir_override(path.map(PathBuf::from));
    allow_recordings_dir(&app, &target);
    tracing::info!(dir = %target.display(), "Recordings folder changed");
    Ok(())
}

#[command]
pub fn open_recordings_dir(app: AppHandle) -> Result<(), String> {
    let dir = paths::ensure_recordings_dir().map_err(|e| e.to_string())?;
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(|e| e.to_string())
}

/// Show the meeting's `audio.wav` selected in Finder.
#[command]
pub fn reveal_meeting_audio(app: AppHandle, meeting_id: String) -> Result<(), String> {
    let audio_path = paths::get_meeting_audio_path(&meeting_id);
    if !audio_path.exists() {
        return Err(format!("Audio file not found for meeting {}", meeting_id));
    }
    app.opener()
        .reveal_item_in_dir(audio_path)
        .map_err(|e| e.to_string())
}
