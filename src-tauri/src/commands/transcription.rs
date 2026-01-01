use crate::recorder::{AudioRecorder, TranscriptionEvent};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::{Mutex, Notify};
use tokio_util::sync::CancellationToken;

const TRANSCRIPTION_EVENT: &str = "transcription://chunk";

pub struct RecorderSession {
    id: u64,
    cancel_token: CancellationToken,
    notify: Arc<Notify>,
    result: Arc<Mutex<Option<Result<(), String>>>>,
    recorder_handle: tauri::async_runtime::JoinHandle<()>,
    events_handle: tauri::async_runtime::JoinHandle<()>,
}

pub struct RecorderState {
    inner: Mutex<Option<RecorderSession>>,
    recorder: Arc<Mutex<AudioRecorder>>,
    next_id: AtomicU64,
}

impl Default for RecorderState {
    fn default() -> Self {
        Self {
            inner: Mutex::new(None),
            recorder: Arc::new(Mutex::new(AudioRecorder::with_default())),
            next_id: AtomicU64::new(0),
        }
    }
}

impl RecorderState {
    fn allocate_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed) + 1
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RecorderStatusPayload {
    active: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EventStatsPayload {
    received: usize,
    with_speech: usize,
    transcribed: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TranscriptionEventPayload {
    source: String,
    text: String,
    raw_text: String,
    confidence: f32,
    duration_sec: f32,
    processing_time_ms: u128,
    audio_level_db: f32,
    stats: EventStatsPayload,
    received_at_ms: u128,
}

impl From<&TranscriptionEvent> for TranscriptionEventPayload {
    fn from(event: &TranscriptionEvent) -> Self {
        Self {
            source: event.source.as_str().to_string(),
            text: event.result.text.clone(),
            raw_text: event.result.raw_text.clone(),
            confidence: event.result.confidence,
            duration_sec: event.result.duration_sec,
            processing_time_ms: event.result.processing_time_ms,
            audio_level_db: event.result.audio_level_db,
            stats: EventStatsPayload {
                received: event.result.chunks_stats.received,
                with_speech: event.result.chunks_stats.with_speech,
                transcribed: event.result.chunks_stats.transcribed,
            },
            received_at_ms: current_timestamp_ms(),
        }
    }
}

#[tauri::command]
pub async fn start_transcription(
    app_handle: AppHandle,
    state: State<'_, RecorderState>,
) -> Result<(), String> {
    let mut guard = state.inner.lock().await;
    if guard.is_some() {
        return Err("A recording session is already running".into());
    }

    let (event_tx, event_rx) = crossbeam_channel::unbounded::<TranscriptionEvent>();
    let notify = Arc::new(Notify::new());
    let result_state = Arc::new(Mutex::new(None));

    // Get cancel token from persistent recorder
    let cancel_token = {
        let recorder_guard = state.recorder.lock().await;
        recorder_guard.get_cancel_token()
    };

    let recorder_handle = spawn_recorder_task(
        state.recorder.clone(),
        notify.clone(),
        result_state.clone(),
        event_tx,
    );

    let events_handle = spawn_events_handler(app_handle.clone(), event_rx);

    let session_id = state.allocate_id();
    let session = RecorderSession {
        id: session_id,
        cancel_token,
        notify: notify.clone(),
        result: result_state.clone(),
        recorder_handle,
        events_handle,
    };

    *guard = Some(session);
    drop(guard);

    spawn_cleanup_watcher(app_handle.clone(), session_id, notify);

    Ok(())
}

#[tauri::command]
pub async fn stop_transcription(state: State<'_, RecorderState>) -> Result<(), String> {
    let session = {
        let mut guard = state.inner.lock().await;
        guard.take()
    };

    if let Some(session) = session {
        session.cancel_token.cancel();
        let result = finalize_session(session).await;

        // Reset recorder for next use
        let mut recorder = state.recorder.lock().await;
        recorder.reset();
        eprintln!("Recorder reset for next session");

        result
    } else {
        Err("No active recording session".into())
    }
}

#[tauri::command]
pub async fn transcription_status(
    state: State<'_, RecorderState>,
) -> Result<RecorderStatusPayload, String> {
    let guard = state.inner.lock().await;
    Ok(RecorderStatusPayload {
        active: guard.is_some(),
    })
}

// Helper functions

fn spawn_recorder_task(
    recorder_arc: Arc<Mutex<AudioRecorder>>,
    notify: Arc<Notify>,
    result_state: Arc<Mutex<Option<Result<(), String>>>>,
    event_tx: crossbeam_channel::Sender<TranscriptionEvent>,
) -> tauri::async_runtime::JoinHandle<()> {
    tauri::async_runtime::spawn_blocking(move || {
        tokio::runtime::Handle::current().block_on(async move {
            let mut recorder = recorder_arc.lock().await;
            let outcome = recorder
                .start(Some(event_tx))
                .await
                .map_err(|err| format!("{}", err));

            *result_state.lock().await = Some(outcome);
            notify.notify_waiters();
        })
    })
}

fn spawn_events_handler(
    app_handle: AppHandle,
    event_rx: crossbeam_channel::Receiver<TranscriptionEvent>,
) -> tauri::async_runtime::JoinHandle<()> {
    tauri::async_runtime::spawn_blocking(move || {
        while let Ok(event) = event_rx.recv() {
            if let Err(err) = emit_transcription_event(&app_handle, &event) {
                eprintln!("Failed to emit transcription event: {err}");
            }
        }
    })
}

fn spawn_cleanup_watcher(app_handle: AppHandle, session_id: u64, notify: Arc<Notify>) {
    tauri::async_runtime::spawn(async move {
        notify.notified().await;

        let state_ref = app_handle.state::<RecorderState>();
        let mut guard = state_ref.inner.lock().await;
        
        if let Some(session) = guard.as_ref() {
            if session.id == session_id {
                guard.take();
            }
        }
    });
}

async fn finalize_session(session: RecorderSession) -> Result<(), String> {
    session.notify.notified().await;

    let outcome = session.result.lock().await.take();

    let join_handles = async {
        let _ = session.recorder_handle.await;
        let _ = session.events_handle.await;
    };

    if let Some(Err(err)) = outcome {
        join_handles.await;
        return Err(err);
    }

    join_handles.await;
    Ok(())
}

fn emit_transcription_event(
    app_handle: &AppHandle,
    event: &TranscriptionEvent,
) -> tauri::Result<()> {
    let payload = TranscriptionEventPayload::from(event);
    app_handle.emit(TRANSCRIPTION_EVENT, payload)
}

fn current_timestamp_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| std::time::Duration::from_millis(0))
        .as_millis()
}
