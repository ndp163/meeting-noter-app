//! Configuration for audio recording and transcription.

/// Base URL of the self-hosted model bucket. A file URL is `{BASE}/{key}` where
/// `key` is a manifest entry (already version-prefixed, e.g.
/// `v1/parakeet-ja/...`). No trailing slash.
pub const MODELS_BASE_URL: &str = "https://meeting-noter.s3.ap-southeast-1.amazonaws.com";

/// URL of the manifest listing every model file (path + size + sha256), grouped
/// by language. Bump the `v1` prefix here and in the bucket to ship new weights
/// without breaking already-installed clients.
pub const MODELS_MANIFEST_URL: &str =
    "https://meeting-noter.s3.ap-southeast-1.amazonaws.com/v1/manifest.json";

/// Which transcription engine to use.
///
/// Add an arm here and a case in [`crate::bridges::create_transcriber`] to
/// support a new model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EngineType {
    #[default]
    FluidAudio,
}

/// Configuration for the audio recording system.
#[derive(Debug, Clone)]
pub struct AudioConfig {
    /// Transcription engine to use.
    pub engine: EngineType,

    /// Maximum time to wait for engine initialization (seconds). Generous
    /// because the first use of a language downloads its model (~600MB for the
    /// Japanese model), which can take several minutes on a cold cache.
    pub init_timeout_secs: u64,

    /// Capture chunk size in samples handed to each audio stream.
    pub chunk_size: usize,

    /// Capacity of the bounded mic/speaker channels.
    pub channel_buffer_size: usize,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            engine: EngineType::default(),
            init_timeout_secs: 600,
            chunk_size: 8000, // 0.5s at 16kHz
            channel_buffer_size: 30,
        }
    }
}

impl AudioConfig {
    /// Select the transcription engine.
    pub fn with_engine(mut self, engine: EngineType) -> Self {
        self.engine = engine;
        self
    }
}
