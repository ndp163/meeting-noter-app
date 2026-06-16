//! AI meeting summary via the local Claude Code CLI.
//!
//! Runs on demand (when the user clicks "Generate summary"). Reads the saved
//! transcript, pipes it to `claude -p` on stdin with summarization
//! instructions, and returns the markdown result. Uses the user's existing
//! Claude Code login — no API key is managed here. If the `claude` binary is
//! absent or not authenticated, the command surfaces a clear error so the UI
//! can fall back to a hint.

use crate::paths;
use crate::types::Meeting;
use std::fs;
use std::path::PathBuf;
use std::process::Stdio;
use tauri::command;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

const INSTRUCTIONS: &str = "Summarize this meeting transcript. \
Output GitHub-flavored markdown with these sections: \
## TL;DR (2-3 sentences), \
## Key Points (bullet list), \
## Decisions (bullet list, omit the section if there were none), \
## Action Items (bullet list as `- [ ] owner — task`, omit if none). \
Be concise and do not invent details. The transcript follows on stdin.";

/// Common install locations to probe before falling back to a login shell.
/// A bundled macOS `.app` launches with a minimal PATH that omits these, so
/// the bare binary name often isn't resolvable at runtime.
const CANDIDATE_PATHS: &[&str] = &[
    ".local/bin/claude",
    ".claude/local/claude",
    ".npm-global/bin/claude",
    ".bun/bin/claude",
];

/// Locate the `claude` binary. Checks `$HOME`-relative install locations and
/// well-known system bins directly, then asks a login shell to resolve it
/// (picks up nvm/fnm/asdf shims and custom PATH entries).
async fn resolve_claude() -> Option<PathBuf> {
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        for rel in CANDIDATE_PATHS {
            let p = home.join(rel);
            if p.is_file() {
                return Some(p);
            }
        }
    }

    for abs in ["/opt/homebrew/bin/claude", "/usr/local/bin/claude"] {
        let p = PathBuf::from(abs);
        if p.is_file() {
            return Some(p);
        }
    }

    // Last resort: a login shell knows the user's full PATH.
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    let output = Command::new(shell)
        .args(["-lc", "command -v claude"])
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() {
        None
    } else {
        Some(PathBuf::from(path))
    }
}

/// True when the `claude` CLI can be located and runs.
#[command]
#[tracing::instrument]
pub async fn claude_available() -> Result<bool, String> {
    let Some(bin) = resolve_claude().await else {
        return Ok(false);
    };
    let status = Command::new(bin)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await;
    Ok(matches!(status, Ok(s) if s.success()))
}

#[command]
#[tracing::instrument]
pub async fn summarize_meeting(meeting_id: String) -> Result<String, String> {
    let data_path = paths::get_meeting_data_path(&meeting_id);
    if !data_path.exists() {
        return Err(format!("Meeting {} not found", meeting_id));
    }

    let content = fs::read_to_string(&data_path)
        .map_err(|e| format!("Failed to read meeting data: {}", e))?;
    let meeting: Meeting = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse meeting data: {}", e))?;

    let transcript = build_transcript(&meeting);
    if transcript.trim().is_empty() {
        return Err("This meeting has no transcript to summarize.".to_string());
    }

    let bin = resolve_claude()
        .await
        .ok_or("Could not find `claude`. Install Claude Code and log in to enable summaries.")?;

    tracing::info!("Summarizing meeting {} via {}", meeting_id, bin.display());

    // Run inside the meeting's own data dir (under Application Support, not a
    // TCC-protected location). Claude Code scans its working directory for
    // context on startup; left at the app's inherited cwd it can walk into
    // ~/Pictures, ~/Documents, etc. and trip a macOS permission prompt.
    let work_dir = paths::get_meeting_dir(&meeting_id);

    let mut child = Command::new(&bin)
        .arg("-p")
        .arg(INSTRUCTIONS)
        .current_dir(&work_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Could not run `claude`: {}", e))?;

    let mut stdin = child.stdin.take().ok_or("Failed to open claude stdin")?;
    stdin
        .write_all(transcript.as_bytes())
        .await
        .map_err(|e| format!("Failed to send transcript to claude: {}", e))?;
    drop(stdin);

    let output = child
        .wait_with_output()
        .await
        .map_err(|e| format!("claude did not complete: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("claude failed: {}", stderr.trim()));
    }

    let summary = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if summary.is_empty() {
        return Err("claude returned an empty summary.".to_string());
    }

    tracing::info!("Summary generated for meeting {}", meeting_id);
    Ok(summary)
}

/// Flatten the transcript into speaker-labelled lines for the model.
fn build_transcript(meeting: &Meeting) -> String {
    meeting
        .transcript
        .iter()
        .map(|msg| {
            let speaker = match msg.source.as_deref() {
                Some("mic") => "You",
                _ => "Speaker",
            };
            format!("{}: {}", speaker, msg.content.trim())
        })
        .collect::<Vec<_>>()
        .join("\n")
}
