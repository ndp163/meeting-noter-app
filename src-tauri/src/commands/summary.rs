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

/// Extra guidance when the input is the diarized transcript: speaker names are
/// trustworthy, so the model should attribute content to them by name.
const SPEAKER_NOTE: &str = " Each transcript line is prefixed with the \
speaker's name. The names are accurate — use them when attributing key \
points, decisions, and action-item owners.";

/// Extra guidance for the raw transcript: the remote side is one unlabelled
/// "Speaker" stream, so the model must not pretend to know who said what.
const RAW_NOTE: &str = " Lines prefixed \"You\" are the user; lines prefixed \
\"Speaker\" are remote participants and may be several different people — do \
not attribute them to specific individuals.";

const TITLE_INSTRUCTIONS: &str = "Generate a short, descriptive title for this \
meeting transcript. Maximum 6 words. Output the title text only — no quotes, \
no markdown, no trailing punctuation, no commentary. The transcript follows on stdin.";

/// Build the translation prompt for a target language name.
fn translate_instructions(language: &str) -> String {
    format!(
        "Translate the following meeting summary into {language}. Preserve the \
markdown structure and heading levels exactly; translate the heading text too. \
Keep checkbox syntax `- [ ]` intact. Keep people's names unchanged — do not \
translate or transliterate them. Output only the translated markdown with \
no extra commentary. The summary follows on stdin."
    )
}

/// Map a BCP-47 target code (from the Translation settings) to the language
/// name used in the prompt. Mirrors the frontend `TRANSLATE_TARGETS`.
fn target_language_name(code: &str) -> &'static str {
    match code {
        "vi" => "Vietnamese",
        "ja" => "Japanese",
        "zh" => "Chinese",
        "ko" => "Korean",
        "es" => "Spanish",
        "fr" => "French",
        _ => "English",
    }
}

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
pub async fn summarize_meeting(
    meeting_id: String,
    source: Option<String>,
) -> Result<String, String> {
    let data_path = paths::get_meeting_data_path(&meeting_id);
    if !data_path.exists() {
        return Err(format!("Meeting {} not found", meeting_id));
    }

    let content = fs::read_to_string(&data_path)
        .map_err(|e| format!("Failed to read meeting data: {}", e))?;
    let meeting: Meeting = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse meeting data: {}", e))?;

    // `speakers` summarizes the diarized, speaker-labelled transcript (with any
    // user renames); `transcript` uses the raw live transcript. Unspecified
    // prefers speakers when a diarization result exists.
    let diarized = build_diarized_transcript(&meeting);
    let (transcript, use_speakers) = match source.as_deref() {
        Some("speakers") => (
            diarized.ok_or("Run Identify speakers first to summarize by speaker.")?,
            true,
        ),
        Some(_) => (build_transcript(&meeting), false),
        None => match diarized {
            Some(d) => (d, true),
            None => (build_transcript(&meeting), false),
        },
    };
    if transcript.trim().is_empty() {
        return Err("This meeting has no transcript to summarize.".to_string());
    }

    let instructions = if use_speakers {
        format!("{INSTRUCTIONS}{SPEAKER_NOTE}")
    } else {
        format!("{INSTRUCTIONS}{RAW_NOTE}")
    };

    tracing::info!(use_speakers, "Summarizing meeting {}", meeting_id);
    let summary = run_inference(&meeting_id, &instructions, &transcript, 2048).await?;
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
pub async fn translate_summary(meeting_id: String, target: String) -> Result<String, String> {
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

    let instructions = translate_instructions(target_language_name(&target));
    tracing::info!("Translating summary for meeting {} into {}", meeting_id, target);
    let translated = run_inference(&meeting_id, &instructions, &summary, 2048).await?;
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

/// Flatten the diarization result into name-labelled lines, in time order,
/// merging consecutive segments of the same speaker into one line. Labels
/// carry any user renames ("You", "Speaker 2", "Alice"…). None when the
/// meeting has no (non-empty) diarization result.
fn build_diarized_transcript(meeting: &Meeting) -> Option<String> {
    let segments = meeting.diarization.as_ref()?;
    let mut sorted: Vec<_> = segments
        .iter()
        .filter(|s| !s.text.trim().is_empty())
        .collect();
    if sorted.is_empty() {
        return None;
    }
    sorted.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap_or(std::cmp::Ordering::Equal));

    let mut lines: Vec<String> = Vec::new();
    let mut last_label: Option<&str> = None;
    for seg in sorted {
        let text = seg.text.trim();
        if last_label == Some(seg.label.as_str()) {
            if let Some(line) = lines.last_mut() {
                line.push(' ');
                line.push_str(text);
            }
        } else {
            lines.push(format!("{}: {}", seg.label, text));
            last_label = Some(seg.label.as_str());
        }
    }
    Some(lines.join("\n"))
}
