use anyhow::Result;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use crate::audio::{
    Mic,
    Speaker,
    processing::mixer,
    streams::{MicStreamHandler, SpeakerStreamHandler},
    transcription::{vad_batch_transcription_task, TranscriptionResult},
};
use crate::bridges::FluidAudio;
use crate::config::AudioConfig;
use crate::types::AudioSource;
use crossbeam_channel::Sender;

/// Wrapper for transcription engine
type Engine = Arc<FluidAudio>;

/// Manages the audio recording and transcription system
/// 
/// Orchestrates:
/// - FluidAudio transcription engine initialization
/// - Separate mic and speaker audio streams
/// - Mixer for WAV recording
/// - Dual transcription pipelines (mic + speaker)
pub struct AudioRecorder {
    config: AudioConfig,
    engine: Option<Engine>,
    cancel_token: CancellationToken,
}

#[derive(Debug, Clone, Copy)]
pub enum TranscriptionSource {
    Mic,
    Speaker,
}

#[derive(Debug, Clone)]
pub struct TranscriptionEvent {
    pub source: TranscriptionSource,
    pub result: TranscriptionResult,
}

impl TranscriptionEvent {
    pub fn new(source: TranscriptionSource, result: TranscriptionResult) -> Self {
        Self { source, result }
    }
}

impl TranscriptionSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Mic => "mic",
            Self::Speaker => "speaker",
        }
    }
}

impl AudioRecorder {
    /// Create a new AudioRecorder with custom configuration
    pub fn new(config: AudioConfig) -> Self {
        Self {
            config,
            engine: None,
            cancel_token: CancellationToken::new(),
        }
    }

    /// Create a new AudioRecorder with default configuration
    pub fn with_default() -> Self {
        Self::new(AudioConfig::default())
    }

    /// Check if transcription is available
    pub fn has_transcription(&self) -> bool {
        self.engine.is_some()
    }

    /// Get a clone of the cancellation token
    pub fn get_cancel_token(&self) -> CancellationToken {
        self.cancel_token.clone()
    }

    /// Reset recorder for reuse (creates new cancel token)
    pub fn reset(&mut self) {
        self.cancel_token = CancellationToken::new();
    }

    /// Start recording and transcription
    /// 
    /// This will:
    /// 1. Initialize transcription engine (if not already done)
    /// 2. Start mic and speaker capture
    /// 3. Start mixer for WAV recording
    /// 4. Start separate transcription tasks
    #[tracing::instrument(skip(self, events_tx))]
    pub async fn start(
        &mut self,
        events_tx: Option<Sender<TranscriptionEvent>>,
        meeting_id: Option<String>,
    ) -> Result<()> {
        // Check if mic is available (but don't create it yet)
        let has_mic = Mic::new().is_ok();
        
        if has_mic {
            tracing::info!("Microphone available");
        } else {
            tracing::warn!("Microphone not available - continuing with speaker only");
        }
        
        // Use standard macOS audio sample rate (48kHz) for WAV file
        // We cannot create Speaker twice (once to detect, once to stream) as it causes TAP device conflicts
        // macOS system audio typically runs at 48kHz, which is perfect for high-quality recording
        let output_sample_rate = 48000u32;
        tracing::info!(sample_rate = output_sample_rate, "Using macOS standard sample rate for WAV");
        tracing::debug!("Audio will be resampled to 16kHz for transcription");

        // Now we can safely await
        // Initialize transcription engine (non-fatal if it fails)
        if self.engine.is_none() {
            tracing::info!("Transcription engine not initialized yet, initializing FluidAudio...");
            if let Err(e) = self.initialize_engine().await {
                tracing::warn!(error = %e, "Continuing without transcription");
            }
        } else {
            tracing::info!("Transcription engine already initialized, reusing...");
        }

        // Setup channels
        let (audio_tx, audio_rx, mic_tx, mic_rx, speaker_tx, speaker_rx) = 
            self.create_channels(output_sample_rate);

        // Start background tasks
        self.spawn_mixer_task(audio_rx, output_sample_rate, meeting_id);
        // For transcription, we'll use the actual speaker sample rate (since mic may not exist)
        // If mic exists, it typically has the same or similar sample rate
        self.spawn_transcription_tasks(mic_rx, speaker_rx, events_tx, output_sample_rate, output_sample_rate);

        // Run audio streams
        self.run_streams(
            has_mic,
            self.config.transcription_chunk_size,
            mic_tx,
            speaker_tx,
            audio_tx,
        ).await?;

        Ok(())
    }

    /// Stop recording gracefully
    pub fn stop(&self) {
        tracing::info!("Stopping audio recorder...");
        self.cancel_token.cancel();
    }

    /// Clone the internal cancellation token for external callers
    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel_token.clone()
    }

    /// Initialize transcription engine with timeout
    async fn initialize_engine(&mut self) -> Result<()> {
        let fluid = Arc::new(FluidAudio::new());
        tracing::info!(
            timeout_secs = self.config.init_timeout_secs,
            "Initializing FluidAudio..."
        );

        match tokio::time::timeout(
            std::time::Duration::from_secs(self.config.init_timeout_secs),
            fluid.initialize(None) // FluidAudio doesn't use model path
        ).await {
            Ok(Ok(_)) => {
                tracing::info!("FluidAudio initialized successfully");
                self.engine = Some(fluid);
                Ok(())
            }
            Ok(Err(e)) => {
                tracing::error!(error = %e, "Failed to initialize FluidAudio");
                Err(anyhow::anyhow!("{}", e))
            }
            Err(_) => {
                tracing::error!("FluidAudio initialization timed out");
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
        meeting_id: Option<String>,
    ) {
        tokio::task::spawn_blocking(move || {
            mixer(audio_rx, sample_rate, meeting_id);
            tracing::debug!("Mixer task completed");
        });
    }

    /// Spawn separate transcription tasks for mic and speaker
    fn spawn_transcription_tasks(
        &self,
        mic_rx: crossbeam_channel::Receiver<Vec<f32>>,
        speaker_rx: crossbeam_channel::Receiver<Vec<f32>>,
        events_tx: Option<Sender<TranscriptionEvent>>,
        mic_sample_rate: u32,
        speaker_sample_rate: u32,
    ) {
        if let Some(ref engine) = self.engine {
            tracing::info!("Starting transcription tasks with FluidAudio...");
            tracing::debug!("   Mic sample rate: {}Hz, Speaker sample rate: {}Hz", mic_sample_rate, speaker_sample_rate);
            
            // Use VAD-batch transcription like RealTimeMicTest
            self.spawn_vad_batch_tasks(engine.clone(), mic_rx, speaker_rx, events_tx, mic_sample_rate, speaker_sample_rate);
        } else {
            tracing::warn!("Transcription engine not initialized - transcription tasks will NOT run!");
            tracing::debug!("    Audio will still be recorded, but no transcription will occur.");
        }
    }



    /// Spawn VAD-guided batch transcription tasks for FluidAudio (like RealTimeMicTest)
    fn spawn_vad_batch_tasks(
        &self,
        engine: Arc<FluidAudio>,
        mic_rx: crossbeam_channel::Receiver<Vec<f32>>,
        speaker_rx: crossbeam_channel::Receiver<Vec<f32>>,
        events_tx: Option<Sender<TranscriptionEvent>>,
        mic_sample_rate: u32,
        speaker_sample_rate: u32,
    ) {
        // Mic VAD-batch transcription task
        let engine_mic = engine.clone();
        let cancel_mic = self.cancel_token.clone();
        let mic_events = events_tx.clone();
        tokio::task::spawn(async move {
            tracing::info!("Mic transcription task started");
            tokio::select! {
                _ = vad_batch_transcription_task(mic_rx, mic_sample_rate, engine_mic, move |result| {
                    if let Some(tx) = mic_events.as_ref() {
                        let _ = tx.send(TranscriptionEvent::new(TranscriptionSource::Mic, result.clone()));
                    }

                    if result.is_result_final {
                        println!("✅ 🎤 Microphone (FINAL): {}", result.text);
                    } else {
                        println!("📝 🎤 Microphone (partial): {}", result.text);
                    }
                }) => {
                    tracing::debug!("Mic transcription task completed");
                }
                _ = cancel_mic.cancelled() => {
                    tracing::debug!("Mic transcription task cancelled");
                }
            }
        });

        // Speaker VAD-batch transcription task
        let engine_speaker = engine;
        let cancel_speaker = self.cancel_token.clone();
        let speaker_events = events_tx;
        tokio::task::spawn(async move {
            tracing::info!("Speaker transcription task started");
            tokio::select! {
                _ = vad_batch_transcription_task(speaker_rx, speaker_sample_rate, engine_speaker, move |result| {
                        if let Some(tx) = speaker_events.as_ref() {
                            let _ = tx.send(TranscriptionEvent::new(TranscriptionSource::Speaker, result.clone()));
                        }

                        if result.is_result_final {
                            println!("✅ 🔊 Speaker (FINAL): {}", result.text);
                        } else {
                            println!("📝 🔊 Speaker (partial): {}", result.text);
                        }
                    }) => {
                        tracing::debug!("Speaker transcription task completed");
                    }
                    _ = cancel_speaker.cancelled() => {
                        tracing::debug!("Speaker transcription task cancelled");
                    }
                }
        });
    }

    /// Run audio streams with cancellation support
    async fn run_streams(
        &self,
        has_mic: bool,
        chunk_size: usize,
        mic_tx: crossbeam_channel::Sender<Vec<f32>>,
        speaker_tx: crossbeam_channel::Sender<Vec<f32>>,
        audio_tx: crossbeam_channel::Sender<AudioSource>,
    ) -> Result<()> {
        tracing::debug!("Starting separate mic and speaker streams...");

        let speaker_cancel = self.cancel_token.clone();
        let audio_tx_speaker = audio_tx.clone();

        // Solution: Create and run streams in separate OS threads
        // This prevents CoreAudio conflicts by isolating their thread-local state
        
        if has_mic {
            tracing::info!("Starting speaker and mic in separate OS threads...");
            
            let mic_cancel = self.cancel_token.clone();
            let audio_tx_mic = audio_tx.clone();
            
            // Spawn speaker in dedicated OS thread
            let speaker_thread = std::thread::spawn(move || {
                tracing::info!("Speaker thread started");
                
                // Create speaker inside thread
                let speaker = match Speaker::new() {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::error!("Failed to create speaker: {:?}", e);
                        return Err(e);
                    }
                };
                let speaker_handler = SpeakerStreamHandler::new(speaker, chunk_size);
                
                let rt = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build() {
                    Ok(rt) => rt,
                    Err(e) => {
                        tracing::error!("Failed to create speaker runtime: {}", e);
                        return Err(anyhow::anyhow!("Failed to create speaker runtime: {}", e));
                    }
                };
                
                let result = rt.block_on(async {
                    speaker_handler.run(speaker_tx, audio_tx_speaker, speaker_cancel).await
                });
                tracing::info!("Speaker thread finished: {:?}", result);
                result
            });
            
            // Spawn mic in dedicated OS thread
            let mic_thread = std::thread::spawn(move || {
                tracing::info!("Mic thread started");
                
                // Create mic inside thread
                let mic = match Mic::new() {
                    Ok(m) => m,
                    Err(e) => {
                        tracing::error!("Failed to create mic: {:?}", e);
                        return Err(e);
                    }
                };
                let mic_handler = MicStreamHandler::new(mic, chunk_size);
                
                let rt = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build() {
                    Ok(rt) => rt,
                    Err(e) => {
                        tracing::error!("Failed to create mic runtime: {}", e);
                        return Err(anyhow::anyhow!("Failed to create mic runtime: {}", e));
                    }
                };
                
                let result = rt.block_on(async {
                    mic_handler.run(mic_tx, audio_tx_mic, mic_cancel).await
                });
                tracing::info!("Mic thread finished: {:?}", result);
                result
            });
            
            tracing::info!("Both threads spawned, waiting for completion...");
            
            // Wait for both threads to complete with timeout protection
            tokio::task::spawn_blocking(move || {
                match speaker_thread.join() {
                    Ok(Ok(_)) => tracing::info!("Speaker thread completed successfully"),
                    Ok(Err(e)) => tracing::error!("Speaker thread error: {:?}", e),
                    Err(e) => tracing::error!("Speaker thread panicked: {:?}", e),
                }
                
                match mic_thread.join() {
                    Ok(Ok(_)) => tracing::info!("Mic thread completed successfully"),
                    Ok(Err(e)) => tracing::error!("Mic thread error: {:?}", e),
                    Err(e) => tracing::error!("Mic thread panicked: {:?}", e),
                }
            }).await?;
            
        } else {
            // No mic, just run speaker in separate thread
            tracing::info!("Running speaker stream only...");
            drop(mic_tx);
            drop(audio_tx);
            
            tokio::task::spawn_blocking(move || {
                let speaker_thread = std::thread::spawn(move || {
                    tracing::info!("Speaker thread started");
                    
                    let speaker = match Speaker::new() {
                        Ok(s) => s,
                        Err(e) => {
                            tracing::error!("Failed to create speaker: {:?}", e);
                            return Err(e);
                        }
                    };
                    let speaker_handler = SpeakerStreamHandler::new(speaker, chunk_size);
                    
                    let rt = match tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build() {
                        Ok(rt) => rt,
                        Err(e) => {
                            tracing::error!("Failed to create speaker runtime: {}", e);
                            return Err(anyhow::anyhow!("Failed to create speaker runtime: {}", e));
                        }
                    };
                    
                    let result = rt.block_on(async {
                        speaker_handler.run(speaker_tx, audio_tx_speaker, speaker_cancel).await
                    });
                    tracing::info!("Speaker thread finished: {:?}", result);
                    result
                });
                
                let _ = speaker_thread.join().map_err(|e| {
                    tracing::error!("Speaker-only thread panicked: {:?}", e);
                });
            }).await?;
        }

        // Explicitly drop senders to close channels and signal downstream tasks to stop
        tracing::debug!("Closing audio channels...");
        
        Ok(())
    }
}
