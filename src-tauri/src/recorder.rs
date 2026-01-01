use anyhow::{Context, Result};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use crate::audio::{
    Mic,
    Speaker,
    processing::mixer,
    streams::{MicStreamHandler, SpeakerStreamHandler},
    transcription::{transcription_task, vad_batch_transcription_task, TranscriptionResult},
};
use crate::bridges::{WhisperKit, FluidAudio, TranscriptionEngine};
use crate::config::{AudioConfig, EngineType};
use crate::types::AudioSource;
use crossbeam_channel::Sender;

/// Wrapper for different transcription engines
enum Engine {
    WhisperKit(Arc<WhisperKit>),
    FluidAudio(Arc<FluidAudio>),
}

/// Manages the audio recording and transcription system
/// 
/// Orchestrates:
/// - Transcription engine initialization (WhisperKit or FluidAudio)
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
    pub async fn start(
        &mut self,
        events_tx: Option<Sender<TranscriptionEvent>>,
    ) -> Result<()> {
        // Check if mic is available (but don't create it yet)
        let has_mic = Mic::new().is_ok();
        
        if has_mic {
            eprintln!("✓ Microphone available");
        } else {
            eprintln!("⚠️  Microphone not available - continuing with speaker only");
        }
        
        // We'll create actual streams inside threads later
        let output_sample_rate = 16000; // Standard sample rate

        // Now we can safely await - mic/speaker have been moved and dropped
        // Initialize transcription engine (non-fatal if it fails)
        if self.engine.is_none() {
            eprintln!("Transcription engine not initialized yet, initializing {}...", 
                match self.config.engine {
                    EngineType::WhisperKit => "WhisperKit",
                    EngineType::FluidAudio => "FluidAudio",
                });
            if let Err(e) = self.initialize_engine().await {
                eprintln!("⚠️  Continuing without transcription: {}", e);
            }
        } else {
            eprintln!("✓ Transcription engine already initialized, reusing...");
        }

        // Setup channels
        let (audio_tx, audio_rx, mic_tx, mic_rx, speaker_tx, speaker_rx) = 
            self.create_channels(output_sample_rate);

        // Start background tasks
        self.spawn_mixer_task(audio_rx, output_sample_rate);
        self.spawn_transcription_tasks(mic_rx, speaker_rx, events_tx);

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
        eprintln!("Stopping audio recorder...");
        self.cancel_token.cancel();
    }

    /// Clone the internal cancellation token for external callers
    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel_token.clone()
    }

    /// Initialize transcription engine with timeout
    async fn initialize_engine(&mut self) -> Result<()> {
        match self.config.engine {
            EngineType::WhisperKit => {
                let whisper = Arc::new(WhisperKit::new());
                eprintln!(
                    "Initializing WhisperKit (model: {}) with {}s timeout...",
                    self.config.model,
                    self.config.init_timeout_secs
                );

                match tokio::time::timeout(
                    std::time::Duration::from_secs(self.config.init_timeout_secs),
                    whisper.initialize(Some(&self.config.model))
                ).await {
                    Ok(Ok(_)) => {
                        eprintln!("✓ WhisperKit initialized successfully");
                        self.engine = Some(Engine::WhisperKit(whisper));
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
            EngineType::FluidAudio => {
                let fluid = Arc::new(FluidAudio::new());
                eprintln!(
                    "Initializing FluidAudio with {}s timeout...",
                    self.config.init_timeout_secs
                );

                match tokio::time::timeout(
                    std::time::Duration::from_secs(self.config.init_timeout_secs),
                    fluid.initialize(None) // FluidAudio doesn't use model path
                ).await {
                    Ok(Ok(_)) => {
                        eprintln!("✓ FluidAudio initialized successfully");
                        self.engine = Some(Engine::FluidAudio(fluid));
                        Ok(())
                    }
                    Ok(Err(e)) => {
                        eprintln!("✗ Failed to initialize FluidAudio: {}", e);
                        Err(anyhow::anyhow!("{}", e))
                    }
                    Err(_) => {
                        eprintln!("✗ FluidAudio initialization timed out");
                        Err(anyhow::anyhow!("Initialization timeout"))
                    }
                }
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
        events_tx: Option<Sender<TranscriptionEvent>>,
    ) {
        if let Some(ref engine) = self.engine {
            eprintln!("🚀 Starting transcription tasks with {:?}...", 
                match engine {
                    Engine::WhisperKit(_) => "WhisperKit",
                    Engine::FluidAudio(_) => "FluidAudio",
                });
            
            // Spawn transcription tasks based on engine type
            match engine {
                Engine::WhisperKit(whisper) => {
                    self.spawn_engine_tasks(whisper.clone(), mic_rx, speaker_rx, events_tx);
                }
                Engine::FluidAudio(fluid) => {
                    // Use VAD-batch transcription like RealTimeMicTest
                    self.spawn_vad_batch_tasks(fluid.clone(), mic_rx, speaker_rx, events_tx);
                }
            }
        } else {
            eprintln!("⚠️  Transcription engine not initialized - transcription tasks will NOT run!");
            eprintln!("    Audio will still be recorded, but no transcription will occur.");
        }
    }

    /// Generic function to spawn transcription tasks for any engine
    fn spawn_engine_tasks<E: TranscriptionEngine + 'static>(
        &self,
        engine: Arc<E>,
        mic_rx: crossbeam_channel::Receiver<Vec<f32>>,
        speaker_rx: crossbeam_channel::Receiver<Vec<f32>>,
        events_tx: Option<Sender<TranscriptionEvent>>,
    ) {
        // Mic transcription task
        let engine_mic = engine.clone();
        let cancel_mic = self.cancel_token.clone();
        let mic_events = events_tx.clone();
        tokio::task::spawn_blocking(move || {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(async move {
                tokio::select! {
                    _ = transcription_task(mic_rx, engine_mic, move |result| {
                        if let Some(tx) = mic_events.as_ref() {
                            let _ = tx.send(TranscriptionEvent::new(TranscriptionSource::Mic, result.clone()));
                        }

                        println!("📝 🎤 Microphone [{}ms] {}", result.processing_time_ms, result.text);
                        if result.chunks_stats.received % 20 == 0 {
                            println!("📊 Mic Stats: received={}, speech={}, transcribed={}, level={:.1}dB", 
                                result.chunks_stats.received, result.chunks_stats.with_speech, 
                                result.chunks_stats.transcribed, result.audio_level_db);
                        }
                    }) => {
                        eprintln!("Mic transcription task completed");
                    }
                    _ = cancel_mic.cancelled() => {
                        eprintln!("Mic transcription task cancelled");
                    }
                }
            });
        });

        // Speaker transcription task
        let engine_speaker = engine;
        let cancel_speaker = self.cancel_token.clone();
        let speaker_events = events_tx;
        tokio::task::spawn_blocking(move || {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(async move {
                tokio::select! {
                    _ = transcription_task(speaker_rx, engine_speaker, move |result| {
                        if let Some(tx) = speaker_events.as_ref() {
                            let _ = tx.send(TranscriptionEvent::new(TranscriptionSource::Speaker, result.clone()));
                        }

                        println!("📝 🔊 Speaker [{}ms] {}", result.processing_time_ms, result.text);
                        if result.chunks_stats.received % 20 == 0 {
                            println!("📊 Speaker Stats: received={}, speech={}, transcribed={}, level={:.1}dB", 
                                result.chunks_stats.received, result.chunks_stats.with_speech, 
                                result.chunks_stats.transcribed, result.audio_level_db);
                        }
                    }) => {
                        eprintln!("Speaker transcription task completed");
                    }
                    _ = cancel_speaker.cancelled() => {
                        eprintln!("Speaker transcription task cancelled");
                    }
                }
            });
        });
    }

    /// Spawn VAD-guided batch transcription tasks for FluidAudio (like RealTimeMicTest)
    fn spawn_vad_batch_tasks(
        &self,
        engine: Arc<FluidAudio>,
        mic_rx: crossbeam_channel::Receiver<Vec<f32>>,
        speaker_rx: crossbeam_channel::Receiver<Vec<f32>>,
        events_tx: Option<Sender<TranscriptionEvent>>,
    ) {
        // Mic VAD-batch transcription task
        let engine_mic = engine.clone();
        let cancel_mic = self.cancel_token.clone();
        let mic_events = events_tx.clone();
        tokio::task::spawn(async move {
            tokio::select! {
                _ = vad_batch_transcription_task(mic_rx, engine_mic, move |result| {
                    if let Some(tx) = mic_events.as_ref() {
                        let _ = tx.send(TranscriptionEvent::new(TranscriptionSource::Mic, result.clone()));
                    }

                    if result.is_final {
                        println!("✅ 🎤 Microphone (FINAL): {}", result.text);
                    } else {
                        println!("📝 🎤 Microphone (partial): {}", result.text);
                    }
                }) => {
                    eprintln!("Mic transcription task completed");
                }
                _ = cancel_mic.cancelled() => {
                    eprintln!("Mic transcription task cancelled");
                }
            }
        });

        // Speaker VAD-batch transcription task
        let engine_speaker = engine;
        let cancel_speaker = self.cancel_token.clone();
        let speaker_events = events_tx;
        tokio::task::spawn(async move {
            tokio::select! {
                _ = vad_batch_transcription_task(speaker_rx, engine_speaker, move |result| {
                        if let Some(tx) = speaker_events.as_ref() {
                            let _ = tx.send(TranscriptionEvent::new(TranscriptionSource::Speaker, result.clone()));
                        }

                        if result.is_final {
                            println!("✅ 🔊 Speaker (FINAL): {}", result.text);
                        } else {
                            println!("📝 🔊 Speaker (partial): {}", result.text);
                        }
                    }) => {
                        eprintln!("Speaker transcription task completed");
                    }
                    _ = cancel_speaker.cancelled() => {
                        eprintln!("Speaker transcription task cancelled");
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
        eprintln!("Starting separate mic and speaker streams...");

        let speaker_cancel = self.cancel_token.clone();
        let audio_tx_speaker = audio_tx.clone();

        // Solution: Create and run streams in separate OS threads
        // This prevents CoreAudio conflicts by isolating their thread-local state
        
        if has_mic {
            eprintln!("🔄 Starting speaker and mic in separate OS threads...");
            
            let mic_cancel = self.cancel_token.clone();
            let audio_tx_mic = audio_tx.clone();
            
            // Spawn speaker in dedicated OS thread
            let speaker_thread = std::thread::spawn(move || {
                eprintln!("🔊 Speaker thread started");
                
                // Create speaker inside thread
                let speaker = match Speaker::new() {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("❌ Failed to create speaker: {:?}", e);
                        return Err(e);
                    }
                };
                let speaker_handler = SpeakerStreamHandler::new(speaker, chunk_size);
                
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to create speaker runtime");
                
                let result = rt.block_on(async {
                    speaker_handler.run(speaker_tx, audio_tx_speaker, speaker_cancel).await
                });
                eprintln!("🔊 Speaker thread finished: {:?}", result);
                result
            });
            
            // Spawn mic in dedicated OS thread
            let mic_thread = std::thread::spawn(move || {
                eprintln!("🎤 Mic thread started");
                
                // Create mic inside thread
                let mic = match Mic::new() {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("❌ Failed to create mic: {:?}", e);
                        return Err(e);
                    }
                };
                let mic_handler = MicStreamHandler::new(mic, chunk_size);
                
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to create mic runtime");
                
                let result = rt.block_on(async {
                    mic_handler.run(mic_tx, audio_tx_mic, mic_cancel).await
                });
                eprintln!("🎤 Mic thread finished: {:?}", result);
                result
            });
            
            eprintln!("🎵 Both threads spawned, waiting for completion...");
            
            // Wait for both threads to complete
            tokio::task::spawn_blocking(move || {
                let speaker_result = speaker_thread.join();
                let mic_result = mic_thread.join();
                
                if let Err(e) = speaker_result {
                    eprintln!("❌ Speaker thread panicked: {:?}", e);
                }
                if let Err(e) = mic_result {
                    eprintln!("❌ Mic thread panicked: {:?}", e);
                }
            }).await?;
            
        } else {
            // No mic, just run speaker in separate thread
            eprintln!("🔄 Running speaker stream only...");
            drop(mic_tx);
            drop(audio_tx);
            
            tokio::task::spawn_blocking(move || {
                let speaker_thread = std::thread::spawn(move || {
                    eprintln!("🔊 Speaker thread started");
                    
                    let speaker = match Speaker::new() {
                        Ok(s) => s,
                        Err(e) => {
                            eprintln!("❌ Failed to create speaker: {:?}", e);
                            return Err(e);
                        }
                    };
                    let speaker_handler = SpeakerStreamHandler::new(speaker, chunk_size);
                    
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("Failed to create speaker runtime");
                    
                    let result = rt.block_on(async {
                        speaker_handler.run(speaker_tx, audio_tx_speaker, speaker_cancel).await
                    });
                    eprintln!("🔊 Speaker thread finished: {:?}", result);
                    result
                });
                
                let _ = speaker_thread.join();
            }).await?;
        }

        // Explicitly drop senders to close channels and signal downstream tasks to stop
        eprintln!("Closing audio channels...");
        
        Ok(())
    }
}
