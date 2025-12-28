//! Meeting Noter App - Audio Recording & Transcription
//!
//! This library provides a complete audio recording and transcription system with:
//! - Dual audio capture (microphone + system audio)
//! - Real-time VAD (Voice Activity Detection)
//! - AI transcription via WhisperKit
//! - High-quality WAV recording
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use noter_lib::{AudioConfig, AudioRecorder};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let mut recorder = AudioRecorder::with_default();
//!     recorder.start().await?;
//!     Ok(())
//! }
//! ```
//!
//! # Architecture
//!
//! - **AudioRecorder**: Main orchestrator
//! - **AudioConfig**: Configuration with builder pattern
//! - **Stream Handlers**: Manage dual output (mixer + transcription)
//! - **WhisperKit**: Swift FFI bridge for AI transcription

use anyhow::Result;

mod audio;
mod bridges;
mod types;
mod config;
mod recorder;

// Re-exports for public API
pub use config::AudioConfig;
pub use recorder::AudioRecorder;

// Legacy entry point for backward compatibility
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() -> Result<()> {
    let mut recorder = AudioRecorder::with_default();
    recorder.start().await
}
