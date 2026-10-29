//! Centralized path utilities
//!
//! Provides consistent path resolution across the application,
//! handling edge cases like:
//! - Running from different locations (dev vs production)
//! - Missing directories
//! - Platform-specific path handling

use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

/// Cached application data directory
static APP_DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

/// User-chosen recordings folder (Settings › Storage). `None` = the default
/// under the app data dir.
static RECORDINGS_DIR_OVERRIDE: RwLock<Option<PathBuf>> = RwLock::new(None);

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

/// Default recordings directory, used when no custom folder is set
pub fn default_recordings_dir() -> PathBuf {
    get_app_data_dir().join("recordings")
}

/// Point the recordings directory at a custom folder (`None` = default)
pub fn set_recordings_dir_override(dir: Option<PathBuf>) {
    if let Ok(mut guard) = RECORDINGS_DIR_OVERRIDE.write() {
        *guard = dir;
    }
}

fn recordings_dir_override() -> Option<PathBuf> {
    RECORDINGS_DIR_OVERRIDE.read().ok().and_then(|g| g.clone())
}

/// Get the recordings directory
pub fn get_recordings_dir() -> PathBuf {
    recordings_dir_override().unwrap_or_else(default_recordings_dir)
}

/// A custom recordings folder must already exist: never recreate it, or an
/// unmounted external drive would silently get a stub `/Volumes/<name>/…` on
/// the system disk.
fn check_recordings_available() -> Result<(), std::io::Error> {
    match recordings_dir_override() {
        Some(dir) if !dir.is_dir() => Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "Recordings folder {} is unavailable — is its drive connected?",
                dir.display()
            ),
        )),
        _ => Ok(()),
    }
}

/// Move every entry of `from` into `to` (renaming, or copy + delete across
/// volumes). Fails up front, moving nothing, if a name already exists in `to`.
pub fn move_dir_contents(from: &Path, to: &Path) -> std::io::Result<()> {
    let entries = std::fs::read_dir(from)?
        .filter(|e| !matches!(e, Ok(e) if e.file_name() == ".DS_Store"))
        .collect::<Result<Vec<_>, _>>()?;
    for entry in &entries {
        let dest = to.join(entry.file_name());
        if dest.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("{} already exists", dest.display()),
            ));
        }
    }
    for entry in &entries {
        let src = entry.path();
        let dest = to.join(entry.file_name());
        if std::fs::rename(&src, &dest).is_ok() {
            continue;
        }
        if let Err(e) = copy_recursive(&src, &dest) {
            let _ = if dest.is_dir() {
                std::fs::remove_dir_all(&dest)
            } else {
                std::fs::remove_file(&dest)
            };
            return Err(e);
        }
        if src.is_dir() {
            std::fs::remove_dir_all(&src)?;
        } else {
            std::fs::remove_file(&src)?;
        }
    }
    Ok(())
}

fn copy_recursive(src: &Path, dest: &Path) -> std::io::Result<()> {
    if src.is_dir() {
        std::fs::create_dir(dest)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            copy_recursive(&entry.path(), &dest.join(entry.file_name()))?;
        }
    } else {
        std::fs::copy(src, dest)?;
    }
    Ok(())
}

/// Get the meeting directory for a specific meeting ID
///
/// Ids arrive from the webview, so anything outside a UUID's alphabet is
/// replaced: `../..` must never resolve outside the recordings folder (callers
/// `remove_dir_all` this path).
pub fn get_meeting_dir(meeting_id: &str) -> PathBuf {
    let safe_id: String = meeting_id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' })
        .collect();
    get_recordings_dir().join(format!("meeting_{}", safe_id))
}

/// Write `contents` via a temp file + fsync + rename, so a crash mid-write
/// leaves the previous file intact instead of truncated JSON.
pub fn write_atomic(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    use std::io::Write;

    let mut tmp_name = path.file_name().unwrap_or_default().to_os_string();
    tmp_name.push(".tmp");
    let tmp = path.with_file_name(tmp_name);
    let mut file = std::fs::File::create(&tmp)?;
    file.write_all(contents)?;
    file.sync_all()?;
    std::fs::rename(&tmp, path)
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
    check_recordings_available()?;
    let dir = get_recordings_dir();
    ensure_dir(&dir)?;
    Ok(dir)
}

/// Ensure a meeting directory exists
pub fn ensure_meeting_dir(meeting_id: &str) -> Result<PathBuf, std::io::Error> {
    check_recordings_available()?;
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
    fn test_move_dir_contents() {
        let root = std::env::temp_dir().join(format!("noter-move-{}", std::process::id()));
        let (from, to) = (root.join("from"), root.join("to"));
        std::fs::create_dir_all(from.join("meeting_a")).unwrap();
        std::fs::create_dir_all(&to).unwrap();
        std::fs::write(from.join("index.json"), "{}").unwrap();
        std::fs::write(from.join("meeting_a/audio.wav"), "x").unwrap();

        move_dir_contents(&from, &to).unwrap();
        assert!(to.join("index.json").is_file());
        assert!(to.join("meeting_a/audio.wav").is_file());
        assert_eq!(std::fs::read_dir(&from).unwrap().count(), 0);

        // A name clash aborts before anything moves.
        std::fs::write(from.join("index.json"), "{}").unwrap();
        std::fs::write(from.join("other.txt"), "y").unwrap();
        assert!(move_dir_contents(&from, &to).is_err());
        assert!(from.join("other.txt").is_file());

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn test_move_dir_contents_ignores_ds_store() {
        let root = std::env::temp_dir().join(format!("noter-move-ds-{}", std::process::id()));
        let (from, to) = (root.join("from"), root.join("to"));
        std::fs::create_dir_all(from.join("meeting_a")).unwrap();
        std::fs::create_dir_all(&to).unwrap();
        // Finder drops .DS_Store in both folders; it must not count as a clash.
        std::fs::write(from.join(".DS_Store"), "f").unwrap();
        std::fs::write(to.join(".DS_Store"), "t").unwrap();

        move_dir_contents(&from, &to).unwrap();
        assert!(to.join("meeting_a").is_dir());
        assert_eq!(std::fs::read_to_string(to.join(".DS_Store")).unwrap(), "t");

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn test_meeting_dir_cannot_escape_recordings() {
        let recordings = get_recordings_dir();
        for id in ["../..", "a/../../b", "/etc", "..", "x\\..\\y"] {
            let dir = get_meeting_dir(id);
            assert_eq!(dir.parent(), Some(recordings.as_path()), "id {id:?} -> {}", dir.display());
        }
        let uuid = "3f2b8c1e-9a4d-4e2b-8f1a-0c9d2e7b6a51";
        assert!(get_meeting_dir(uuid).ends_with(format!("meeting_{uuid}")));
    }

    #[test]
    fn test_write_atomic_replaces_and_leaves_no_temp() {
        let root = std::env::temp_dir().join(format!("noter-atomic-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("index.json");
        std::fs::write(&path, "old").unwrap();

        write_atomic(&path, b"new").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn test_meeting_data_path() {
        let data_path = get_meeting_data_path("abc");
        assert!(data_path.ends_with("data.json"));
        assert!(data_path.to_string_lossy().contains("meeting_abc"));
    }
}
