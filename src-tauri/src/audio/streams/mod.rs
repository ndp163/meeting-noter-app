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
//!
//! The handlers use a generic trait-based approach to eliminate code duplication
//! while maintaining type safety.

mod handler;
mod mic_handler;
mod speaker_handler;

// Export handler internals for potential extension
pub(crate) use handler::{AudioStreamSource, GenericStreamHandler};
pub use mic_handler::MicStreamHandler;
pub use speaker_handler::SpeakerStreamHandler;
