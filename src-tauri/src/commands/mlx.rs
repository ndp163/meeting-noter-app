//! Tauri commands for managing the optional local MLX model (download / remove
//! / availability). Generation itself is invoked from `summary.rs`.

use tauri::{command, AppHandle};

/// Whether the local model is fully installed (dylib + metallib + weights).
#[command]
pub fn local_model_available() -> bool {
    crate::mlx::installed()
}

/// Download the local model set, streaming progress via `mlx://progress`.
#[command]
#[tracing::instrument(skip(app))]
pub async fn download_local_model(app: AppHandle) -> Result<(), String> {
    tracing::info!("Downloading local MLX model");
    crate::mlx::download(app).await
}

/// Remove the local model to free disk space.
#[command]
#[tracing::instrument]
pub fn delete_local_model() -> Result<(), String> {
    tracing::info!("Deleting local MLX model");
    crate::mlx::delete()
}
