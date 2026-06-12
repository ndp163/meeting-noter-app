//! Audio processing utilities
//!
//! - **mixer**: Mix mic + speaker audio and write to WAV
//! - **resample**: Downsample to the transcription rate (48kHz -> 16kHz)

mod mixer;
mod resample;

pub use mixer::mixer;
pub use resample::Resampler;
