use crate::paths;
use crate::types::Meeting;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::command;

#[derive(Debug, Serialize, Deserialize)]
pub struct MeetingsIndex {
    pub meetings: Vec<Meeting>,
}

fn get_recordings_dir() -> Result<PathBuf, String> {
    let dir = paths::get_recordings_dir();
    paths::ensure_dir(&dir)
        .map_err(|e| {
            tracing::error!("Failed to ensure recordings directory: {}", e);
            format!("Failed to access recordings directory: {}", e)
        })?;
    Ok(dir)
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
    
    fs::write(&index_path, &content)
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
    
    fs::write(&data_path, content)
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
