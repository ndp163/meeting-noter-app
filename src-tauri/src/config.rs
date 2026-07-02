//! Configuration for audio recording and transcription.

// Model assets are served from one CloudFront distribution
// (`https://d17sbkyjhl5fws.cloudfront.net`) in front of the private S3 bucket
// (`meeting-noter`, ap-southeast-1); origin access is locked to CloudFront
// (OAC), so raw S3 URLs 403. Assets are grouped by domain: `asr/…` (speech
// models) and `llm/…` (the on-device summary runtime + weights) — the domain
// prefix lives in each base URL below, not in the manifest keys.

/// Base URL for ASR model files. A file URL is `{BASE}/{key}` where `key` is a
/// manifest entry of the form `<version>/<repo>/...` (e.g. `v1/parakeet-ja/…`).
/// FluidAudio derives its on-disk cache layout by stripping the leading version
/// component, so the `asr/` domain prefix lives here in the base — NOT in the
/// keys — to keep that stripping (and the shipped Swift bridge) unchanged.
pub const MODELS_BASE_URL: &str = "https://d17sbkyjhl5fws.cloudfront.net/asr";

/// URL of the ASR manifest listing every model file, grouped by language. Bump
/// the `v1` here and in the bucket to ship new weights without breaking
/// already-installed clients.
pub const MODELS_MANIFEST_URL: &str =
    "https://d17sbkyjhl5fws.cloudfront.net/asr/v1/manifest.json";

/// Base URL for the on-device LLM assets (runtime dylib + metallib + weights).
/// Manifest `url`s are `<version>/...` (e.g. `v1/model/…`); the `llm/` domain
/// prefix lives here in the base, mirroring [`MODELS_BASE_URL`] for ASR.
pub const LLM_BASE_URL: &str = "https://d17sbkyjhl5fws.cloudfront.net/llm";

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
