//! AI meeting summary with a pluggable provider.
//!
//! Two backends, selected by the persisted `summary_provider` setting:
//!   - `claude` (default) — pipes the transcript to the local Claude Code CLI
//!     (`claude -p`), using the user's existing login. Needs the network.
//!   - `local` — runs the on-device MLX model (`crate::mlx`), fully offline.
//!
//! Each task (summarize / title / translate) builds its prompt the same way for
//! both providers; only the inference call differs. Errors surface clearly so
//! the UI can show an install/login/download hint.

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

const TITLE_INSTRUCTIONS: &str = "Generate a short, descriptive title for this \
meeting transcript. Maximum 6 words. Output the title text only — no quotes, \
no markdown, no trailing punctuation, no commentary. The transcript follows on stdin.";

const TRANSLATE_INSTRUCTIONS: &str = "Translate the following meeting summary \
into Vietnamese. Preserve the markdown structure and heading levels exactly; \
translate the heading text too. Keep checkbox syntax `- [ ]` intact. \
Output only the translated markdown with no extra commentary. \
The summary follows on stdin.";

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

    tracing::info!("Summarizing meeting {}", meeting_id);
    let summary = run_inference(&meeting_id, INSTRUCTIONS, &transcript, 2048).await?;
    if summary.is_empty() {
        return Err("The summary came back empty.".to_string());
    }
    Ok(summary)
}

#[command]
#[tracing::instrument]
pub async fn generate_title(meeting_id: String) -> Result<String, String> {
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
        return Err("This meeting has no transcript to title.".to_string());
    }

    tracing::info!("Generating title for meeting {}", meeting_id);
    let title = run_inference(&meeting_id, TITLE_INSTRUCTIONS, &transcript, 64).await?;
    if title.is_empty() {
        return Err("The title came back empty.".to_string());
    }
    Ok(title)
}

#[command]
#[tracing::instrument]
pub async fn translate_summary(meeting_id: String) -> Result<String, String> {
    let data_path = paths::get_meeting_data_path(&meeting_id);
    if !data_path.exists() {
        return Err(format!("Meeting {} not found", meeting_id));
    }

    let content = fs::read_to_string(&data_path)
        .map_err(|e| format!("Failed to read meeting data: {}", e))?;
    let meeting: Meeting = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse meeting data: {}", e))?;

    let summary = meeting
        .summary
        .filter(|s| !s.trim().is_empty())
        .ok_or("Generate a summary before translating.")?;

    tracing::info!("Translating summary for meeting {}", meeting_id);
    let translated = run_inference(&meeting_id, TRANSLATE_INSTRUCTIONS, &summary, 2048).await?;
    if translated.is_empty() {
        return Err("The translation came back empty.".to_string());
    }
    Ok(translated)
}

/// Route one inference to the configured provider. `max_tokens` caps the local
/// model's output (ignored by the Claude CLI, which manages its own length).
async fn run_inference(
    meeting_id: &str,
    instructions: &str,
    input: &str,
    max_tokens: i32,
) -> Result<String, String> {
    match crate::commands::settings::summary_provider().as_str() {
        "local" => run_local(instructions, input, max_tokens).await,
        _ => run_claude(meeting_id, instructions, input).await,
    }
}

/// Generate with the on-device MLX model. The FFI call blocks, so it runs on a
/// blocking thread.
///
/// Small local models don't reliably follow system-prompt-only instructions when
/// the user turn is a long transcript. Embedding the instruction at the end of
/// the user message gives the model a clear, near-context directive to follow.
async fn run_local(instructions: &str, input: &str, max_tokens: i32) -> Result<String, String> {
    if !crate::mlx::installed() {
        return Err("The local model isn't installed yet. Download it in Settings → Models.".to_string());
    }
    let model_dir = crate::mlx::model_dir().to_string_lossy().into_owned();
    let user_message = format!("{input}\n\n---\n{instructions}");
    tokio::task::spawn_blocking(move || {
        crate::mlx::generate(&model_dir, "", &user_message, max_tokens)
    })
    .await
    .map_err(|e| format!("local generation task failed: {e}"))?
}

/// Pipe `input` to `claude -p <instructions>` and return its trimmed stdout.
///
/// Runs inside the meeting's own data dir (under Application Support, not a
/// TCC-protected location). Claude Code scans its working directory for
/// context on startup; left at the app's inherited cwd it can walk into
/// ~/Pictures, ~/Documents, etc. and trip a macOS permission prompt.
async fn run_claude(meeting_id: &str, instructions: &str, input: &str) -> Result<String, String> {
    let bin = resolve_claude()
        .await
        .ok_or("Could not find `claude`. Install Claude Code and log in to enable summaries.")?;

    let work_dir = paths::get_meeting_dir(meeting_id);

    let mut child = Command::new(&bin)
        .arg("-p")
        .arg(instructions)
        .current_dir(&work_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Could not run `claude`: {}", e))?;

    let mut stdin = child.stdin.take().ok_or("Failed to open claude stdin")?;
    stdin
        .write_all(input.as_bytes())
        .await
        .map_err(|e| format!("Failed to send input to claude: {}", e))?;
    drop(stdin);

    let output = child
        .wait_with_output()
        .await
        .map_err(|e| format!("claude did not complete: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("claude failed: {}", stderr.trim()));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
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
