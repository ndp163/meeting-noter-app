//! Local MLX summary provider — optional, downloaded on demand.
//!
//! mlx is **not** bundled in the `.app` (keeps the base install tiny). When the
//! user picks the local summary provider we download three things from the
//! model CDN into Application Support:
//!   - `libMlxBridge.dylib` — the mlx runtime + LLM stack (`dlopen`ed at runtime)
//!   - `mlx.metallib` — the GPU kernels, kept next to the dylib so mlx's
//!     colocated lookup (`dladdr` → dylib dir) finds them with no bundle plumbing
//!   - the Qwen3-4B-Instruct-2507-4bit weights, under `model/`
//!
//! Downloads go through the same CloudFront CDN as the ASR models. We shell out
//! to `curl`/`shasum` rather than linking an HTTP+TLS stack into this otherwise
//! network-free binary; progress is polled from the partial file size and
//! streamed to the UI via `mlx://progress`.

mod bridge;
pub use bridge::generate;

use crate::config::LLM_BASE_URL;
use crate::paths::get_app_data_dir;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::process::Command;

/// Manifest listing every file to fetch (CDN-relative `url` + local `dest` +
/// `size` + `sha256`). Version-prefixed so a future mlx bump can ship without
/// breaking installed clients.
const MANIFEST_URL: &str = "https://d17sbkyjhl5fws.cloudfront.net/llm/v1/manifest.json";

pub fn mlx_dir() -> PathBuf {
    get_app_data_dir().join("mlx")
}
pub fn dylib_path() -> PathBuf {
    mlx_dir().join("libMlxBridge.dylib")
}
fn metallib_path() -> PathBuf {
    mlx_dir().join("mlx.metallib")
}
pub fn model_dir() -> PathBuf {
    mlx_dir().join("model")
}
/// Written only after a full, checksum-verified download so a partial install
/// never reports ready.
fn marker_path() -> PathBuf {
    mlx_dir().join(".installed")
}

/// True when the dylib, metallib, model config and the success marker all exist.
pub fn installed() -> bool {
    marker_path().exists()
        && dylib_path().exists()
        && metallib_path().exists()
        && model_dir().join("config.json").exists()
}

#[derive(Deserialize)]
struct Manifest {
    files: Vec<FileEntry>,
}

#[derive(Deserialize)]
struct FileEntry {
    /// Path relative to `LLM_BASE_URL` (e.g. `v1/model/…`).
    url: String,
    /// Local path relative to `mlx_dir()`.
    dest: String,
    size: u64,
    sha256: String,
}

#[derive(Clone, Serialize)]
struct ProgressEvent {
    fraction: f64,
}

fn emit(app: &AppHandle, fraction: f64) {
    let _ = app.emit("mlx://progress", ProgressEvent { fraction });
}

/// Download (or repair) the full local model set, streaming `mlx://progress`.
/// Idempotent: files already present with a matching checksum are skipped.
pub async fn download(app: AppHandle) -> Result<(), String> {
    let dir = mlx_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("create mlx dir: {e}"))?;

    let manifest = fetch_manifest().await?;
    let total: u64 = manifest.files.iter().map(|f| f.size).sum();
    if total == 0 {
        return Err("mlx manifest is empty".into());
    }

    let mut done: u64 = 0;
    for f in &manifest.files {
        let dest = dir.join(&f.dest);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("create {parent:?}: {e}"))?;
        }

        // Resume: skip files that are already present and valid.
        if dest.exists() && sha256_file(&dest)? == f.sha256 {
            done += f.size;
            emit(&app, (done as f64 / total as f64).min(0.999));
            continue;
        }

        download_file(&app, &f.url, &dest, done, total).await?;
        let got = sha256_file(&dest)?;
        if got != f.sha256 {
            let _ = std::fs::remove_file(&dest);
            return Err(format!(
                "checksum mismatch for {}: expected {}, got {}",
                f.dest, f.sha256, got
            ));
        }
        done += f.size;
        emit(&app, (done as f64 / total as f64).min(0.999));
    }

    std::fs::write(marker_path(), "1").map_err(|e| format!("write marker: {e}"))?;
    emit(&app, 1.0);
    Ok(())
}

/// Remove the entire local model directory (dylib, metallib, weights).
pub fn delete() -> Result<(), String> {
    let dir = mlx_dir();
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| format!("delete mlx dir: {e}"))?;
    }
    Ok(())
}

async fn fetch_manifest() -> Result<Manifest, String> {
    let out = Command::new("curl")
        .args(["-fsSL", MANIFEST_URL])
        .output()
        .await
        .map_err(|e| format!("run curl for manifest: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "manifest download failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    serde_json::from_slice(&out.stdout).map_err(|e| format!("parse manifest: {e}"))
}

/// Download one file to `<dest>.part`, polling its size for progress, then move
/// it into place. `base_done`/`total` let the fraction span the whole set.
async fn download_file(
    app: &AppHandle,
    url_suffix: &str,
    dest: &PathBuf,
    base_done: u64,
    total: u64,
) -> Result<(), String> {
    let url = format!("{LLM_BASE_URL}/{url_suffix}");
    let tmp = dest.with_extension("part");

    let mut child = Command::new("curl")
        .args(["-fL", "--retry", "3", "-o"])
        .arg(&tmp)
        .arg(&url)
        .spawn()
        .map_err(|e| format!("spawn curl: {e}"))?;

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    let _ = std::fs::remove_file(&tmp);
                    return Err(format!("download failed for {url_suffix}"));
                }
                break;
            }
            Ok(None) => {
                let cur = std::fs::metadata(&tmp).map(|m| m.len()).unwrap_or(0);
                emit(app, ((base_done + cur) as f64 / total as f64).min(0.999));
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
            Err(e) => return Err(format!("wait curl: {e}")),
        }
    }

    std::fs::rename(&tmp, dest).map_err(|e| format!("move {url_suffix} into place: {e}"))?;
    Ok(())
}

/// Hex SHA-256 of a file via the system `shasum` (keeps a crypto crate out of
/// the dep tree).
fn sha256_file(path: &PathBuf) -> Result<String, String> {
    let out = std::process::Command::new("shasum")
        .args(["-a", "256"])
        .arg(path)
        .output()
        .map_err(|e| format!("run shasum: {e}"))?;
    if !out.status.success() {
        return Err(format!("shasum failed for {path:?}"));
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    stdout
        .split_whitespace()
        .next()
        .map(|s| s.to_string())
        .ok_or_else(|| "empty shasum output".to_string())
}
