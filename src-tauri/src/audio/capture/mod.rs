//! Audio capture devices
//! 
//! This module provides abstractions for capturing audio from different sources:
//! - Microphone (via cpal)
//! - System audio/Speaker (via ScreenCaptureKit on macOS)

pub mod mic;
pub mod speaker;

pub use mic::Mic;
pub use speaker::Speaker;
