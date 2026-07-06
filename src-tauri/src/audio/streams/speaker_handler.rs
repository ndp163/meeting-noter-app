//! Speaker/System audio stream handler
//!
//! Handles system audio stream with dual output:
//! - Sends raw audio to mixer for WAV recording
//! - Buffers and sends chunks to transcription pipeline

use anyhow::Result;

use crate::audio::alignment::StreamKind;
use crate::audio::capture::speaker::SpeakerStream;
use crate::audio::Speaker;
use crate::types::AudioSource;

use super::handler::{AudioStreamSource, GenericStreamHandler};

// Implement AudioStreamSource for Speaker
impl AudioStreamSource for Speaker {
    type Stream = SpeakerStream;
    
    fn sample_rate(&self) -> u32 {
        Speaker::sample_rate(self)
    }
    
    fn into_stream(self) -> Result<Self::Stream> {
        self.stream()
    }
    
    fn display_name() -> &'static str {
        "speaker"
    }
    
    fn log_emoji() -> &'static str {
        "🔊"
    }
    
    fn to_audio_source(data: Vec<f32>) -> AudioSource {
        AudioSource::System(data)
    }

    fn stream_kind() -> StreamKind {
        StreamKind::Speaker
    }
}

/// Type alias for speaker stream handler
pub type SpeakerStreamHandler = GenericStreamHandler<Speaker>;
