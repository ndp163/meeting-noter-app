//! FFI bridges to external libraries
//!
//! Provides transcription engines: WhisperKit (argmax) and FluidAudio

pub mod whisperkit;
pub mod fluidaudio;

pub use whisperkit::WhisperKit;
pub use fluidaudio::FluidAudio;

use async_trait::async_trait;

/// Common trait for all transcription engines
#[async_trait]
pub trait TranscriptionEngine: Send + Sync {
    /// Initialize the transcription engine with optional model path
    async fn initialize(&self, model_path: Option<&str>) -> Result<(), String>;
    
    /// Transcribe audio data (batch mode)
    async fn transcribe(&self, audio_data: &[f32]) -> Result<String, String>;
    
    /// Transcribe audio data (streaming mode)
    async fn transcribe_stream(&self, audio_data: &[f32]) -> Result<String, String>;
    
    /// Check if engine is initialized
    fn is_initialized(&self) -> bool;
}

#[async_trait]
impl TranscriptionEngine for WhisperKit {
    async fn initialize(&self, model_path: Option<&str>) -> Result<(), String> {
        WhisperKit::initialize(self, model_path).await
    }
    
    async fn transcribe(&self, audio_data: &[f32]) -> Result<String, String> {
        WhisperKit::transcribe(self, audio_data).await
    }
    
    async fn transcribe_stream(&self, audio_data: &[f32]) -> Result<String, String> {
        WhisperKit::transcribe_stream(self, audio_data).await
    }
    
    fn is_initialized(&self) -> bool {
        WhisperKit::is_initialized(self)
    }
}

#[async_trait]
impl TranscriptionEngine for FluidAudio {
    async fn initialize(&self, model_path: Option<&str>) -> Result<(), String> {
        FluidAudio::initialize(self, model_path).await
    }
    
    async fn transcribe(&self, audio_data: &[f32]) -> Result<String, String> {
        FluidAudio::transcribe(self, audio_data).await
    }
    
    async fn transcribe_stream(&self, audio_data: &[f32]) -> Result<String, String> {
        FluidAudio::transcribe_stream(self, audio_data).await
    }
    
    fn is_initialized(&self) -> bool {
        FluidAudio::is_initialized(self)
    }
}
