use anyhow::{Context, Result};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use crate::audio::{
    Mic,
    Speaker,
    processing::mixer,
    streams::{MicStreamHandler, SpeakerStreamHandler},
    transcription::transcription_task,
};
use crate::bridges::whisperkit::WhisperKit;
use crate::config::AudioConfig;
use crate::types::AudioSource;

/// Manages the audio recording and transcription system
/// 
/// Orchestrates:
/// - WhisperKit initialization for AI transcription
/// - Separate mic and speaker audio streams
/// - Mixer for WAV recording
/// - Dual transcription pipelines (mic + speaker)
pub struct AudioRecorder {
    config: AudioConfig,
    whisper: Option<Arc<WhisperKit>>,
    cancel_token: CancellationToken,
}

impl AudioRecorder {
    /// Create a new AudioRecorder with custom configuration
    pub fn new(config: AudioConfig) -> Self {
        Self {
            config,
            whisper: None,
            cancel_token: CancellationToken::new(),
        }
    }

    /// Create a new AudioRecorder with default configuration
    pub fn with_default() -> Self {
        Self::new(AudioConfig::default())
    }

    /// Check if transcription is available
    pub fn has_transcription(&self) -> bool {
        self.whisper.is_some()
    }

    /// Start recording and transcription
    /// 
    /// This will:
    /// 1. Initialize WhisperKit (if not already done)
    /// 2. Start mic and speaker capture
    /// 3. Start mixer for WAV recording
    /// 4. Start separate transcription tasks
    pub async fn start(&mut self) -> Result<()> {
        // Initialize WhisperKit (non-fatal if it fails)
        if let Err(e) = self.initialize_whisper().await {
            eprintln!("Continuing without transcription: {}", e);
        }

        // Initialize audio devices
        let mic = Mic::new().context("Failed to initialize microphone")?;
        let speaker = Speaker::new().context("Failed to initialize speaker")?;

        // Get sample rates for logging
        let mic_sample_rate = mic.sample_rate();
        let speaker_sample_rate = speaker.sample_rate();
        let output_sample_rate = mic_sample_rate.max(speaker_sample_rate);

        eprintln!(
            "Recording mic ({}Hz) + speaker ({}Hz)",
            mic_sample_rate, speaker_sample_rate
        );

        // Create stream handlers
        let mic_handler = MicStreamHandler::new(mic, self.config.transcription_chunk_size);
        let speaker_handler = SpeakerStreamHandler::new(speaker, self.config.transcription_chunk_size);

        // Setup channels
        let (audio_tx, audio_rx, mic_tx, mic_rx, speaker_tx, speaker_rx) = 
            self.create_channels(output_sample_rate);

        // Start background tasks
        self.spawn_mixer_task(audio_rx, output_sample_rate);
        self.spawn_transcription_tasks(mic_rx, speaker_rx);

        // Run audio streams
        self.run_streams(
            mic_handler,
            speaker_handler,
            mic_tx,
            speaker_tx,
            audio_tx,
        ).await?;

        Ok(())
    }

    /// Stop recording gracefully
    pub fn stop(&self) {
        eprintln!("Stopping audio recorder...");
        self.cancel_token.cancel();
    }

    /// Initialize WhisperKit with timeout
    async fn initialize_whisper(&mut self) -> Result<()> {
        let whisper = Arc::new(WhisperKit::new());
        eprintln!(
            "Initializing WhisperKit (model: {}) with {}s timeout...",
            self.config.whisper_model,
            self.config.init_timeout_secs
        );

        match tokio::time::timeout(
            std::time::Duration::from_secs(self.config.init_timeout_secs),
            whisper.initialize(Some(&self.config.whisper_model))
        ).await {
            Ok(Ok(_)) => {
                eprintln!("✓ WhisperKit initialized successfully");
                self.whisper = Some(whisper);
                Ok(())
            }
            Ok(Err(e)) => {
                eprintln!("✗ Failed to initialize WhisperKit: {}", e);
                Err(anyhow::anyhow!("{}", e))
            }
            Err(_) => {
                eprintln!("✗ WhisperKit initialization timed out");
                Err(anyhow::anyhow!("Initialization timeout"))
            }
        }
    }

    /// Create all communication channels with proper bounds
    fn create_channels(
        &self,
        output_sample_rate: u32,
    ) -> (
        crossbeam_channel::Sender<AudioSource>,
        crossbeam_channel::Receiver<AudioSource>,
        crossbeam_channel::Sender<Vec<f32>>,
        crossbeam_channel::Receiver<Vec<f32>>,
        crossbeam_channel::Sender<Vec<f32>>,
        crossbeam_channel::Receiver<Vec<f32>>,
    ) {
        // Mixer channel - bounded by time (100ms of audio)
        let mixer_buffer_size = (output_sample_rate / 10) as usize;
        let (audio_tx, audio_rx) = crossbeam_channel::bounded(mixer_buffer_size);

        // Transcription channels - bounded by chunk count
        let (mic_tx, mic_rx) = crossbeam_channel::bounded(self.config.channel_buffer_size);
        let (speaker_tx, speaker_rx) = crossbeam_channel::bounded(self.config.channel_buffer_size);

        (audio_tx, audio_rx, mic_tx, mic_rx, speaker_tx, speaker_rx)
    }

    /// Spawn mixer task for WAV recording
    fn spawn_mixer_task(
        &self,
        audio_rx: crossbeam_channel::Receiver<AudioSource>,
        sample_rate: u32,
    ) {
        tokio::task::spawn_blocking(move || {
            mixer(audio_rx, sample_rate);
            eprintln!("Mixer task completed");
        });
    }

    /// Spawn separate transcription tasks for mic and speaker
    fn spawn_transcription_tasks(
        &self,
        mic_rx: crossbeam_channel::Receiver<Vec<f32>>,
        speaker_rx: crossbeam_channel::Receiver<Vec<f32>>,
    ) {
        if let Some(ref whisper) = self.whisper {
            // Mic transcription task
            let whisper_mic = whisper.clone();
            let cancel_mic = self.cancel_token.clone();
            tokio::task::spawn_blocking(move || {
                let rt = tokio::runtime::Handle::current();
                rt.block_on(async move {
                    tokio::select! {
                        _ = transcription_task(mic_rx, whisper_mic, "🎤 Microphone") => {
                            eprintln!("Mic transcription task completed");
                        }
                        _ = cancel_mic.cancelled() => {
                            eprintln!("Mic transcription task cancelled");
                        }
                    }
                });
            });

            // Speaker transcription task
            let whisper_speaker = whisper.clone();
            let cancel_speaker = self.cancel_token.clone();
            tokio::task::spawn_blocking(move || {
                let rt = tokio::runtime::Handle::current();
                rt.block_on(async move {
                    tokio::select! {
                        _ = transcription_task(speaker_rx, whisper_speaker, "🔊 Speaker") => {
                            eprintln!("Speaker transcription task completed");
                        }
                        _ = cancel_speaker.cancelled() => {
                            eprintln!("Speaker transcription task cancelled");
                        }
                    }
                });
            });
        }
    }

    /// Run audio streams with cancellation support
    async fn run_streams(
        &self,
        mic_handler: MicStreamHandler,
        speaker_handler: SpeakerStreamHandler,
        mic_tx: crossbeam_channel::Sender<Vec<f32>>,
        speaker_tx: crossbeam_channel::Sender<Vec<f32>>,
        audio_tx: crossbeam_channel::Sender<AudioSource>,
    ) -> Result<()> {
        eprintln!("Starting separate mic and speaker streams...");

        let mic_cancel = self.cancel_token.clone();
        let speaker_cancel = self.cancel_token.clone();
        let audio_tx_mic = audio_tx.clone();
        let audio_tx_speaker = audio_tx;

        // Run streams concurrently until cancellation or error
        tokio::select! {
            result = mic_handler.run(mic_tx, audio_tx_mic, mic_cancel) => {
                if let Err(e) = result {
                    eprintln!("Mic stream error: {}", e);
                } else {
                    eprintln!("Mic stream completed");
                }
            }
            result = speaker_handler.run(speaker_tx, audio_tx_speaker, speaker_cancel) => {
                if let Err(e) = result {
                    eprintln!("Speaker stream error: {}", e);
                } else {
                    eprintln!("Speaker stream completed");
                }
            }
            _ = self.cancel_token.cancelled() => {
                eprintln!("Audio recording cancelled");
            }
        }

        Ok(())
    }
}
