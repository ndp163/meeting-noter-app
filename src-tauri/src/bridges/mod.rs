//! FFI bridges to external transcription libraries.
//!
//! The pipeline talks to two traits, never a concrete engine:
//! - [`SpeechRecognizer`] turns audio into text (ASR).
//! - [`VoiceDetector`] reports speech probability per chunk (VAD).
//!
//! To add a new model: implement these two traits and add an arm to
//! [`crate::config::EngineType`] + [`create_transcriber`].

pub mod fluidaudio;

pub use fluidaudio::FluidAudio;

use crate::config::EngineType;
use async_trait::async_trait;
use std::sync::Arc;

/// Turns audio samples into text.
#[async_trait]
pub trait SpeechRecognizer: Send + Sync {
    /// Load models for `language` (e.g. `"en"`, `"ja"`). Must be called once
    /// before [`transcribe`](Self::transcribe).
    async fn initialize(&self, language: &str) -> Result<(), String>;

    /// Transcribe a chunk of 16kHz mono f32 samples.
    async fn transcribe(&self, audio: &[f32]) -> Result<String, String>;

    fn is_initialized(&self) -> bool;
}

/// Detects speech activity in a continuous audio stream.
///
/// State is keyed by `stream_id` so mic and speaker can run independently.
pub trait VoiceDetector: Send + Sync {
    fn create_stream(&self, stream_id: &str) -> Result<(), String>;

    /// Returns voice probability `[0.0, 1.0]` for this chunk.
    fn process(&self, stream_id: &str, audio: &[f32]) -> Result<f32, String>;

    fn destroy_stream(&self, stream_id: &str);
}

/// A speech recognizer paired with its voice detector.
///
/// For engines that bundle both (like FluidAudio) the two `Arc`s point at the
/// same instance; for ASR-only models they can be wired to a separate detector.
#[derive(Clone)]
pub struct Transcriber {
    pub asr: Arc<dyn SpeechRecognizer>,
    pub vad: Arc<dyn VoiceDetector>,
}

/// The single place that maps a model choice to a concrete engine.
pub fn create_transcriber(engine: EngineType) -> Transcriber {
    match engine {
        EngineType::FluidAudio => {
            let fluid = Arc::new(FluidAudio::new());
            Transcriber {
                asr: fluid.clone(),
                vad: fluid,
            }
        }
    }
}
