//! Audio stream handlers
//!
//! Stream handlers manage the dual-output pattern:
//! 1. Send raw audio to mixer for WAV recording
//! 2. Buffer audio chunks for transcription pipeline
//!
//! Each handler (mic/speaker) operates independently with:
//! - Cancellation support
//! - Pre-allocated buffers
//! - Zero-cost transfers (std::mem::swap)

mod mic_handler;
mod speaker_handler;

pub use mic_handler::MicStreamHandler;
pub use speaker_handler::SpeakerStreamHandler;
