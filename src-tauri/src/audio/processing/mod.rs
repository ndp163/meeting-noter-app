//! Audio processing utilities
//!
//! - **mixer**: Mix mic + speaker audio and write to WAV
//! - **resample**: Convert sample rates (48kHz → 16kHz for AI)
//! - **filter**: Remove non-speech segments via VAD

mod mixer;
mod resample;
mod filter;

pub use mixer::mixer;
pub use resample::{resample_to_16khz, resample_to_16khz_fast};
pub use filter::filter_non_speech;
