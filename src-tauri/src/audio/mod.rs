//! Audio subsystem
//!
//! This module contains all audio-related functionality:
//! - **capture**: Microphone and system audio capture
//! - **processing**: Audio processing (mixing, resampling, filtering)
//! - **streams**: Stream handlers with dual output (mixer + transcription)
//! - **transcription**: VAD + FluidAudio integration

pub mod capture;
pub mod processing;
pub mod streams;
pub mod transcription;

// Re-export common types for convenience
pub use capture::{Mic, Speaker};
