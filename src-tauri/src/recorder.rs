use anyhow::Result;
use serde::Serialize;
use std::future::Future;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio_util::sync::CancellationToken;

use crate::audio::constants::SAMPLE_RATE_48KHZ;
use crate::audio::{
    processing::mixer,
    streams::{MicStreamHandler, SpeakerStreamHandler},
    transcription::{self, TranscriptionResult},
    Mic, Speaker,
};
use crate::bridges::{create_transcriber, Transcriber};
use crate::config::AudioConfig;
use crate::types::AudioSource;
use crossbeam_channel::{Receiver, Sender};

/// Model-readiness signal emitted to the frontend so the UI can show a
/// "preparing model" state instead of jumping straight to "Listening" while
/// the ASR engine loads (a cold first-run load that has no other feedback).
const STATUS_EVENT: &str = "transcription://status";

#[derive(Clone, Serialize)]
struct StatusEvent {
    status: String,
}

fn emit_status(app: &Option<AppHandle>, status: &str) {
    if let Some(app) = app {
        let _ = app.emit(
            STATUS_EVENT,
            StatusEvent {
                status: status.to_string(),
            },
        );
    }
}

/// Where a transcription came from.
#[derive(Debug, Clone, Copy)]
pub enum Source {
    Mic,
    Speaker,
}

impl Source {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Mic => "mic",
            Self::Speaker => "speaker",
        }
    }
}

#[derive(Debug, Clone)]
pub struct TranscriptionEvent {
    pub source: Source,
    pub result: TranscriptionResult,
}

impl TranscriptionEvent {
    pub fn new(source: Source, result: TranscriptionResult) -> Self {
        Self { source, result }
    }
}

/// Records mic + speaker audio to a WAV file while transcribing both streams.
pub struct AudioRecorder {
    config: AudioConfig,
    transcriber: Option<Transcriber>,
    /// Language the cached `transcriber` was initialized for. When a new session
    /// requests a different language, the engine is reloaded with the right model.
    current_language: Option<String>,
    cancel_token: CancellationToken,
}

impl Default for AudioRecorder {
    fn default() -> Self {
        Self::new(AudioConfig::default())
    }
}

impl AudioRecorder {
    pub fn new(config: AudioConfig) -> Self {
        Self {
            config,
            transcriber: None,
            current_language: None,
            cancel_token: CancellationToken::new(),
        }
    }

    pub fn has_transcription(&self) -> bool {
        self.transcriber.is_some()
    }

    /// Clone the cancellation token so callers can stop the recorder.
    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel_token.clone()
    }

    pub fn stop(&self) {
        tracing::info!("Stopping audio recorder...");
        self.cancel_token.cancel();
    }

    /// Reset for reuse with a fresh cancellation token.
    pub fn reset(&mut self) {
        self.cancel_token = CancellationToken::new();
    }

    /// Start recording and transcription, returning when the streams stop.
    #[tracing::instrument(skip(self, events_tx))]
    pub async fn start(
        &mut self,
        events_tx: Option<Sender<TranscriptionEvent>>,
        meeting_id: Option<String>,
        language: Option<String>,
        app_handle: Option<AppHandle>,
    ) -> Result<()> {
        // Probe the mic to learn its real capture rate. A Bluetooth headset
        // used as input gets forced into the 16kHz HFP call profile, so this
        // is often not 48kHz; using the actual rate keeps the WAV and the
        // transcription resampler correct (otherwise the mic plays back ~3x
        // too fast and the ASR is fed garbled audio).
        let mic_rate = Mic::new().ok().map(|m| m.sample_rate());
        let has_mic = mic_rate.is_some();
        if has_mic {
            tracing::info!(?mic_rate, "Microphone available");
        } else {
            tracing::warn!("Microphone not available - continuing with speaker only");
        }
        let mic_rate = mic_rate.unwrap_or(SAMPLE_RATE_48KHZ);

        // macOS system audio runs at 48kHz; we record the WAV at that rate and
        // resample to 16kHz inside the transcription pipeline. We cannot probe
        // the speaker rate separately without causing a TAP device conflict.
        let sample_rate = SAMPLE_RATE_48KHZ;

        // Default to English when unspecified. Reload the engine when the
        // requested language differs from the cached one (e.g. EN -> JA).
        let language = language.unwrap_or_else(|| "en".to_string());
        let needs_init = self.transcriber.is_none()
            || self.current_language.as_deref() != Some(language.as_str());
        if needs_init {
            emit_status(&app_handle, "preparing");
            if let Err(e) = self.initialize_transcriber(&language).await {
                tracing::warn!(error = %e, "Continuing without transcription");
            }
        }
        // Signal readiness whether warm or freshly loaded, so the UI never
        // hangs on the preparing state (audio still records even if init fails).
        emit_status(&app_handle, "ready");

        let (audio_tx, audio_rx, mic_tx, mic_rx, speaker_tx, speaker_rx) =
            self.create_channels(sample_rate);

        self.spawn_mixer(audio_rx, sample_rate, mic_rate, meeting_id, has_mic);
        self.spawn_pipelines(mic_rx, speaker_rx, events_tx, mic_rate, sample_rate);
        self.run_streams(has_mic, self.config.chunk_size, mic_tx, speaker_tx, audio_tx)
            .await?;

        Ok(())
    }

    /// Build the transcriber from config and initialize it for `language`, with
    /// a timeout.
    async fn initialize_transcriber(&mut self, language: &str) -> Result<()> {
        // Drop the stale engine first so a failed reload doesn't leave a
        // wrong-language transcriber cached.
        self.transcriber = None;
        self.current_language = None;

        let transcriber = create_transcriber(self.config.engine);
        tracing::info!(
            timeout_secs = self.config.init_timeout_secs,
            language,
            "Initializing transcription engine..."
        );

        let timeout = Duration::from_secs(self.config.init_timeout_secs);
        match tokio::time::timeout(timeout, transcriber.asr.initialize(language)).await {
            Ok(Ok(())) => {
                tracing::info!("Transcription engine initialized");
                self.transcriber = Some(transcriber);
                self.current_language = Some(language.to_string());
                Ok(())
            }
            Ok(Err(e)) => Err(anyhow::anyhow!(e)),
            Err(_) => Err(anyhow::anyhow!("Initialization timeout")),
        }
    }

    /// Create the mixer and per-source transcription channels.
    #[allow(clippy::type_complexity)]
    fn create_channels(
        &self,
        sample_rate: u32,
    ) -> (
        Sender<AudioSource>,
        Receiver<AudioSource>,
        Sender<Vec<f32>>,
        Receiver<Vec<f32>>,
        Sender<Vec<f32>>,
        Receiver<Vec<f32>>,
    ) {
        // Mixer channel holds ~100ms of audio.
        let (audio_tx, audio_rx) = crossbeam_channel::bounded((sample_rate / 10) as usize);
        let (mic_tx, mic_rx) = crossbeam_channel::bounded(self.config.channel_buffer_size);
        let (speaker_tx, speaker_rx) = crossbeam_channel::bounded(self.config.channel_buffer_size);
        (audio_tx, audio_rx, mic_tx, mic_rx, speaker_tx, speaker_rx)
    }

    /// Spawn the mixer that writes captured audio to a WAV file.
    fn spawn_mixer(
        &self,
        audio_rx: Receiver<AudioSource>,
        target_rate: u32,
        mic_rate: u32,
        meeting_id: Option<String>,
        has_mic: bool,
    ) {
        tokio::task::spawn_blocking(move || {
            mixer(audio_rx, target_rate, mic_rate, meeting_id, has_mic);
            tracing::debug!("Mixer task completed");
        });
    }

    /// Spawn one transcription pipeline per audio source.
    fn spawn_pipelines(
        &self,
        mic_rx: Receiver<Vec<f32>>,
        speaker_rx: Receiver<Vec<f32>>,
        events_tx: Option<Sender<TranscriptionEvent>>,
        mic_rate: u32,
        speaker_rate: u32,
    ) {
        let Some(transcriber) = self.transcriber.clone() else {
            tracing::warn!("No transcription engine - audio will be recorded but not transcribed");
            return;
        };

        self.spawn_pipeline(Source::Mic, transcriber.clone(), mic_rx, events_tx.clone(), mic_rate);
        self.spawn_pipeline(Source::Speaker, transcriber, speaker_rx, events_tx, speaker_rate);
    }

    /// The pipeline loop is blocking (sync VAD FFI + channel reads), so it runs
    /// on a blocking thread. No cancellation hook is needed: cancelling stops
    /// the stream handlers, which drop their senders, which disconnects `rx`
    /// and lets the pipeline exit through its cleanup path (destroying its VAD
    /// stream — a `select!` here would skip that and leak it).
    fn spawn_pipeline(
        &self,
        source: Source,
        transcriber: Transcriber,
        rx: Receiver<Vec<f32>>,
        events_tx: Option<Sender<TranscriptionEvent>>,
        sample_rate: u32,
    ) {
        let language = self.current_language.clone().unwrap_or_else(|| "en".to_string());
        tokio::task::spawn_blocking(move || {
            tracing::info!(?source, "Transcription pipeline starting");
            transcription::run(rx, sample_rate, transcriber, &language, move |result| {
                if let Some(tx) = events_tx.as_ref() {
                    let _ = tx.send(TranscriptionEvent::new(source, result));
                }
            });
            tracing::debug!(?source, "Transcription pipeline finished");
        });
    }

    /// Capture mic and speaker in dedicated OS threads (to isolate CoreAudio
    /// thread-local state) and wait for both to finish.
    async fn run_streams(
        &self,
        has_mic: bool,
        chunk_size: usize,
        mic_tx: Sender<Vec<f32>>,
        speaker_tx: Sender<Vec<f32>>,
        audio_tx: Sender<AudioSource>,
    ) -> Result<()> {
        let speaker_cancel = self.cancel_token.clone();
        let speaker_mixer_tx = audio_tx.clone();
        let speaker = run_on_thread("Speaker", move || async move {
            let speaker = Speaker::new()?;
            SpeakerStreamHandler::new(speaker, chunk_size)
                .run(speaker_tx, speaker_mixer_tx, speaker_cancel)
                .await
        });

        let mic = if has_mic {
            let mic_cancel = self.cancel_token.clone();
            Some(run_on_thread("Mic", move || async move {
                let mic = Mic::new()?;
                MicStreamHandler::new(mic, chunk_size)
                    .run(mic_tx, audio_tx, mic_cancel)
                    .await
            }))
        } else {
            // Drop unused senders so the channels close.
            drop(mic_tx);
            drop(audio_tx);
            None
        };

        tokio::task::spawn_blocking(move || {
            join_stream_thread("Speaker", speaker);
            if let Some(mic) = mic {
                join_stream_thread("Mic", mic);
            }
        })
        .await?;

        Ok(())
    }
}

/// Run an async stream handler to completion on its own OS thread + runtime.
fn run_on_thread<F, Fut>(name: &'static str, make_future: F) -> thread::JoinHandle<Result<()>>
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = Result<()>>,
{
    thread::spawn(move || {
        tracing::info!("{name} thread started");
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build {name} runtime: {e}"))?;
        let result = runtime.block_on(make_future());
        tracing::info!("{name} thread finished: {result:?}");
        result
    })
}

fn join_stream_thread(name: &str, handle: thread::JoinHandle<Result<()>>) {
    match handle.join() {
        Ok(Ok(())) => tracing::info!("{name} thread completed"),
        Ok(Err(e)) => tracing::error!("{name} thread error: {e:?}"),
        Err(e) => tracing::error!("{name} thread panicked: {e:?}"),
    }
}
