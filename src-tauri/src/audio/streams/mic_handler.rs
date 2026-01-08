//! Microphone stream handler
//!
//! Handles microphone audio stream with dual output:
//! - Sends raw audio to mixer for WAV recording
//! - Buffers and sends chunks to transcription pipeline

use anyhow::Result;

use crate::audio::capture::mic::MicStream;
use crate::audio::Mic;
use crate::types::AudioSource;

use super::handler::{AudioStreamSource, GenericStreamHandler};

// Implement AudioStreamSource for Mic
impl AudioStreamSource for Mic {
    type Stream = MicStream;
    
    fn sample_rate(&self) -> u32 {
        Mic::sample_rate(self)
    }
    
    fn into_stream(self) -> Result<Self::Stream> {
        self.stream()
    }
    
    fn display_name() -> &'static str {
        "mic"
    }
    
    fn log_emoji() -> &'static str {
        "🎤"
    }
    
    fn to_audio_source(data: Vec<f32>) -> AudioSource {
        AudioSource::Mic(data)
    }
}

/// Type alias for microphone stream handler
pub type MicStreamHandler = GenericStreamHandler<Mic>;
