//! Centralized path utilities
//!
//! Provides consistent path resolution across the application,
//! handling edge cases like:
//! - Running from different locations (dev vs production)
//! - Missing directories
//! - Platform-specific path handling

use std::path::PathBuf;
use std::sync::OnceLock;

/// Cached application data directory
static APP_DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Check if we're running from a macOS app bundle
fn is_bundled_app() -> bool {
    if let Ok(exe) = std::env::current_exe() {
        // macOS bundles have structure: Something.app/Contents/MacOS/binary
        let path_str = exe.to_string_lossy();
        path_str.contains(".app/Contents/MacOS")
    } else {
        false
    }
}

/// Get the application data directory
/// 
/// Resolution order:
/// 1. Environment variable `NOTER_DATA_DIR` (for testing/override)
/// 2. For bundled macOS app: ~/Library/Application Support/noter
/// 3. For dev builds: Parent of executable's parent directory (target/debug/../)
/// 4. User's home directory + ".noter" (fallback)
/// 5. Current directory (last resort)
pub fn get_app_data_dir() -> &'static PathBuf {
    APP_DATA_DIR.get_or_init(|| {
        // 1. Check environment variable first
        if let Ok(dir) = std::env::var("NOTER_DATA_DIR") {
            let path = PathBuf::from(dir);
            if path.exists() || create_dir_if_needed(&path).is_ok() {
                tracing::info!("Using NOTER_DATA_DIR: {}", path.display());
                return path;
            }
        }
        
        // 2. For bundled macOS app, use Application Support directory
        if is_bundled_app() {
            if let Some(app_support) = dirs::data_dir() {
                // ~/Library/Application Support/noter
                let path = app_support.join("noter");
                if create_dir_if_needed(&path).is_ok() {
                    tracing::info!("Using Application Support: {}", path.display());
                    return path;
                }
            }
            // Fallback for bundled app: home directory
            if let Some(home) = dirs::home_dir() {
                let path = home.join(".noter");
                if create_dir_if_needed(&path).is_ok() {
                    tracing::info!("Using home directory for bundled app: {}", path.display());
                    return path;
                }
            }
        }
        
        // 3. For dev builds: executable-relative path (target/debug/../)
        if let Ok(exe) = std::env::current_exe() {
            // For dev: /path/to/target/debug/noter -> /path/to/target
            if let Some(target_dir) = exe.parent().and_then(|p| p.parent()) {
                let path = target_dir.to_path_buf();
                tracing::debug!("Using dev data dir from exe: {}", path.display());
                return path;
            }
        }
        
        // 4. Try user's home directory
        if let Some(home) = dirs::home_dir() {
            let path = home.join(".noter");
            if create_dir_if_needed(&path).is_ok() {
                tracing::info!("Using home directory: {}", path.display());
                return path;
            }
        }
        
        // 5. Last resort: current directory
        tracing::warn!("Using current directory as fallback");
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    })
}

/// Get the recordings directory
pub fn get_recordings_dir() -> PathBuf {
    get_app_data_dir().join("recordings")
}

/// Get the meeting directory for a specific meeting ID
pub fn get_meeting_dir(meeting_id: &str) -> PathBuf {
    get_recordings_dir().join(format!("meeting_{}", meeting_id))
}

/// Get the audio file path for a meeting
pub fn get_meeting_audio_path(meeting_id: &str) -> PathBuf {
    get_meeting_dir(meeting_id).join("audio.wav")
}

/// Get the data file path for a meeting
pub fn get_meeting_data_path(meeting_id: &str) -> PathBuf {
    get_meeting_dir(meeting_id).join("data.json")
}

/// Repair a WAV whose RIFF/data chunk sizes were never finalized (the
/// recorder was killed mid-write — hound only rewrites the header sizes on
/// finalize, so a crash leaves them at 0). CoreAudio refuses such files with
/// `kAudioFileInvalidFileError` ('dta?'), permanently breaking playback and
/// diarization of that meeting. Sizes are recomputed from the real file
/// length; well-formed files are left untouched.
pub fn repair_wav_header(path: &std::path::Path) -> std::io::Result<()> {
    use std::io::{Read, Seek, SeekFrom, Write};

    let mut file = std::fs::OpenOptions::new().read(true).write(true).open(path)?;
    let file_len = file.metadata()?.len();
    if file_len < 44 {
        return Ok(()); // too short to hold any samples; nothing to salvage
    }

    let mut header = [0u8; 12];
    file.read_exact(&mut header)?;
    if &header[0..4] != b"RIFF" || &header[8..12] != b"WAVE" {
        return Ok(());
    }

    // Walk the chunk list to find `data` (its payload runs to end-of-file in
    // hound-written recordings).
    let mut pos: u64 = 12;
    while pos + 8 <= file_len {
        file.seek(SeekFrom::Start(pos))?;
        let mut chunk = [0u8; 8];
        file.read_exact(&mut chunk)?;
        let size = u64::from(u32::from_le_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]));
        if &chunk[0..4] == b"data" {
            let actual = file_len - (pos + 8);
            if size == actual {
                return Ok(()); // header is consistent; leave it alone
            }
            file.seek(SeekFrom::Start(pos + 4))?;
            file.write_all(&(actual as u32).to_le_bytes())?;
            file.seek(SeekFrom::Start(4))?;
            file.write_all(&((file_len - 8) as u32).to_le_bytes())?;
            tracing::info!(path = %path.display(), "Repaired unfinalized WAV header");
            return Ok(());
        }
        if size == 0 {
            return Ok(()); // corrupt chunk list; don't loop forever
        }
        pos += 8 + size + (size & 1);
    }
    Ok(())
}

/// Get the meetings index file path
pub fn get_meetings_index_path() -> PathBuf {
    get_recordings_dir().join("index.json")
}

/// Ensure a directory exists, creating it if necessary
pub fn ensure_dir(path: &PathBuf) -> Result<(), std::io::Error> {
    if !path.exists() {
        std::fs::create_dir_all(path)?;
        tracing::debug!("Created directory: {}", path.display());
    }
    Ok(())
}

/// Ensure the recordings directory exists
pub fn ensure_recordings_dir() -> Result<PathBuf, std::io::Error> {
    let dir = get_recordings_dir();
    ensure_dir(&dir)?;
    Ok(dir)
}

/// Ensure a meeting directory exists
pub fn ensure_meeting_dir(meeting_id: &str) -> Result<PathBuf, std::io::Error> {
    let dir = get_meeting_dir(meeting_id);
    ensure_dir(&dir)?;
    Ok(dir)
}

// Helper to create directory if needed
fn create_dir_if_needed(path: &PathBuf) -> Result<(), std::io::Error> {
    if !path.exists() {
        std::fs::create_dir_all(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paths_are_consistent() {
        let recordings = get_recordings_dir();
        let meeting_dir = get_meeting_dir("test123");
        let audio_path = get_meeting_audio_path("test123");
        
        assert!(meeting_dir.starts_with(&recordings));
        assert!(audio_path.starts_with(&meeting_dir));
        assert!(audio_path.ends_with("audio.wav"));
    }
    
    #[test]
    fn test_meeting_data_path() {
        let data_path = get_meeting_data_path("abc");
        assert!(data_path.ends_with("data.json"));
        assert!(data_path.to_string_lossy().contains("meeting_abc"));
    }
}
