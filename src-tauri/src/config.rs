// Configuration for audio recording and transcription
//
// Provides AudioConfig with builder pattern for configuring:
// - Transcription engine selection (WhisperKit or FluidAudio)
// - Model selection
// - Transcription chunk size
// - Initialization timeout
// - Channel buffer sizes

/// Transcription engine type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineType {
    /// WhisperKit by Argmax (CoreML-based)
    WhisperKit,
    /// FluidAudio with Parakeet models (faster, more accurate)
    FluidAudio,
}

impl Default for EngineType {
    fn default() -> Self {
        Self::FluidAudio // Default to FluidAudio as it's faster
    }
}

/// Configuration for the audio recording system
#[derive(Debug, Clone)]
///
/// # Example
///
/// ```rust
/// use noter_lib::{AudioConfig, EngineType};
///
/// let config = AudioConfig::default()
///     .with_engine(EngineType::FluidAudio)
///     .with_model("small.en")
///     .with_chunk_size(160000)
///     .with_timeout(60);
/// ```
pub struct AudioConfig {
    /// Transcription engine to use
    pub engine: EngineType,
    
    /// Model to use for transcription (path or name)
    pub model: String,
    
    /// Maximum time to wait for engine initialization (seconds)
    pub init_timeout_secs: u64,
    
    /// Buffer size for audio processing
    pub buffer_size: usize,
    
    /// Transcription chunk size (~5 seconds of audio at 16kHz)
    pub transcription_chunk_size: usize,
    
    /// Channel buffer size for bounded channels
    pub channel_buffer_size: usize,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            engine: EngineType::FluidAudio,
            model: "medium.en".to_string(),
            init_timeout_secs: 120,
            buffer_size: 1024,
            transcription_chunk_size: 8000, // 0.5s at 16kHz - matches RealTimeMicTest
            channel_buffer_size: 30, // Large buffer for VAD-batch mode
        }
    }
}

impl AudioConfig {
    /// Create a new config with custom engine
    pub fn with_engine(mut self, engine: EngineType) -> Self {
        self.engine = engine;
        self
    }
    
    /// Create a new config with custom model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
    
    /// Set transcription chunk size
    pub fn with_chunk_size(mut self, size: usize) -> Self {
        self.transcription_chunk_size = size;
        self
    }
    
    /// Set initialization timeout
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.init_timeout_secs = secs;
        self
    }
}

