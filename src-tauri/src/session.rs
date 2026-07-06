//! Recording session state and lifecycle, independent of the IPC layer.
//!
//! Owns the persistent [`AudioRecorder`], tracks the active session, and
//! forwards transcription events to the frontend.

use crate::recorder::{AudioRecorder, TranscriptionEvent};
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::{mpsc, Mutex, Notify};
use tokio_util::sync::CancellationToken;

const TRANSCRIPTION_EVENT: &str = "transcription://chunk";
const TRANSLATION_EVENT: &str = "translation://chunk";

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
            recorder: Arc::new(Mutex::new(AudioRecorder::default())),
            next_id: AtomicU64::new(0),
        }
    }
}

impl RecorderState {
    fn allocate_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// Non-blocking session check; `None` if the state lock is contended.
    pub fn is_active_now(&self) -> Option<bool> {
        self.inner.try_lock().ok().map(|guard| guard.is_some())
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
    is_result_final: bool, // True for final results, false for partial/streaming
    is_sentence_final: bool, // True when sentence is complete (start new message)
    start_sec: f32,        // Offset (seconds) of this result within the recording
    /// Per-word time spans (absolute recording seconds); only on committed
    /// results, empty for partials.
    words: Vec<crate::types::DiarizedWord>,
}

impl From<&TranscriptionEvent> for TranscriptionEventPayload {
    fn from(event: &TranscriptionEvent) -> Self {
        let (is_result_final, is_sentence_final) = event.result.finality.as_flags();
        Self {
            source: event.source.as_str().to_string(),
            text: event.result.text.clone(),
            raw_text: event.result.text.clone(),
            confidence: 1.0,
            duration_sec: event.result.duration_sec,
            processing_time_ms: 0,
            audio_level_db: 0.0,
            stats: EventStatsPayload {
                received: 0,
                with_speech: 0,
                transcribed: 0,
            },
            received_at_ms: current_timestamp_ms(),
            is_result_final,
            is_sentence_final,
            start_sec: event.result.start_sec,
            words: event.result.words.clone(),
        }
    }
}

/// A translated segment, matched to its transcript chunk on the frontend by
/// `(source, start_sec)`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TranslationEventPayload {
    source: String,
    text: String,
    start_sec: f32,
    is_result_final: bool,
    is_sentence_final: bool,
}

/// One unit of work for the translate worker.
struct TranslateJob {
    source: String,
    text: String,
    start_sec: f32,
    is_result_final: bool,
    is_sentence_final: bool,
}

impl TranslateJob {
    /// Dedup key: same transcript segment (per source stream + start offset).
    fn key(&self) -> (String, u32) {
        (self.source.clone(), self.start_sec.to_bits())
    }
}

/// Start a new recording session. Fails if one is already running.
pub async fn start(
    app_handle: AppHandle,
    state: &RecorderState,
    meeting_id: String,
    language: Option<String>,
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
        recorder_guard.cancel_token()
    };

    // Realtime translation (optional): source = the meeting's ASR language.
    let source_lang = language.clone().unwrap_or_else(|| "en".to_string());
    let translate_tx = maybe_spawn_translate(app_handle.clone(), source_lang);

    let recorder_handle = spawn_recorder_task(
        state.recorder.clone(),
        notify.clone(),
        result_state.clone(),
        event_tx,
        meeting_id,
        language,
        app_handle.clone(),
    );

    let events_handle = spawn_events_handler(app_handle.clone(), event_rx, translate_tx);

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

/// Stop the active recording session and reset the recorder for reuse.
pub async fn stop(state: &RecorderState) -> Result<(), String> {
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
        tracing::debug!("Recorder reset for next session");

        result
    } else {
        Err("No active recording session".into())
    }
}

pub async fn status(state: &RecorderState) -> RecorderStatusPayload {
    let guard = state.inner.lock().await;
    RecorderStatusPayload {
        active: guard.is_some(),
    }
}

// Helper functions

fn spawn_recorder_task(
    recorder_arc: Arc<Mutex<AudioRecorder>>,
    notify: Arc<Notify>,
    result_state: Arc<Mutex<Option<Result<(), String>>>>,
    event_tx: crossbeam_channel::Sender<TranscriptionEvent>,
    meeting_id: String,
    language: Option<String>,
    app_handle: AppHandle,
) -> tauri::async_runtime::JoinHandle<()> {
    tauri::async_runtime::spawn_blocking(move || {
        tokio::runtime::Handle::current().block_on(async move {
            let mut recorder = recorder_arc.lock().await;
            let outcome = recorder
                .start(Some(event_tx), Some(meeting_id), language, Some(app_handle))
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
    translate_tx: Option<mpsc::UnboundedSender<TranslateJob>>,
) -> tauri::async_runtime::JoinHandle<()> {
    tauri::async_runtime::spawn_blocking(move || {
        while let Ok(event) = event_rx.recv() {
            if let Err(err) = emit_transcription_event(&app_handle, &event) {
                tracing::debug!("Failed to emit transcription event: {err}");
            }
            // Hand the committed/partial text to the translate worker, which
            // drops superseded partials and emits `translation://chunk`.
            if let Some(tx) = &translate_tx {
                let text = event.result.text.trim();
                if !text.is_empty() {
                    let (is_result_final, is_sentence_final) = event.result.finality.as_flags();
                    let _ = tx.send(TranslateJob {
                        source: event.source.as_str().to_string(),
                        text: text.to_string(),
                        start_sec: event.result.start_sec,
                        is_result_final,
                        is_sentence_final,
                    });
                }
            }
        }
    })
}

/// Spawn the translation worker if translation is enabled and the target
/// differs from the source. Returns the job sender, or `None` when translation
/// is off (so the events handler skips it entirely).
fn maybe_spawn_translate(
    app_handle: AppHandle,
    source: String,
) -> Option<mpsc::UnboundedSender<TranslateJob>> {
    let (enabled, target) = crate::commands::settings::translate_config();
    if !enabled || target.is_empty() || target == source {
        return None;
    }

    let (tx, rx) = mpsc::unbounded_channel::<TranslateJob>();
    tauri::async_runtime::spawn(translate_worker(app_handle, source, target, rx));
    Some(tx)
}

/// Accumulated state of the message currently being built for one source
/// stream, mirroring the frontend's transcript-merge so the translated text
/// matches what's displayed.
#[derive(Default)]
struct MsgState {
    start_sec: f32,
    committed: String,
    done: bool,
    /// Creation order of the current (or last) message, mirroring the
    /// message's position in the frontend transcript.
    seq: u64,
    /// `seq` of this source's most recently *finished* message (0 = none).
    last_closed_seq: u64,
}

/// Translate jobs in order, collapsing superseded partials, and translate the
/// **full accumulated message** (not each chunk) so the result matches the
/// merged transcript line the frontend shows. Emits `translation://chunk` keyed
/// by the message's `start_sec` for the frontend to match.
///
/// Superseded partials are dropped when jobs pile up (partials arriving faster
/// than translation); committed/sentence-final results have distinct
/// `start_sec` so they're never dropped, keeping accumulation correct.
async fn translate_worker(
    app_handle: AppHandle,
    source_lang: String,
    target: String,
    mut rx: mpsc::UnboundedReceiver<TranslateJob>,
) {
    let mut states: HashMap<String, MsgState> = HashMap::new();
    let mut next_seq: u64 = 0;

    while let Some(first) = rx.recv().await {
        // Collapse everything currently queued, keeping the latest per segment
        // while preserving arrival order (accumulation depends on order).
        let mut order: Vec<(String, u32)> = Vec::new();
        let mut latest: HashMap<(String, u32), TranslateJob> = HashMap::new();
        let k0 = first.key();
        order.push(k0.clone());
        latest.insert(k0, first);
        while let Ok(job) = rx.try_recv() {
            let k = job.key();
            if !latest.contains_key(&k) {
                order.push(k.clone());
            }
            latest.insert(k, job);
        }

        for k in order {
            let Some(job) = latest.remove(&k) else { continue };

            // Mirror of the frontend's interjection split: another stream
            // finished a message *after* this source's open message started,
            // so the continuation begins a fresh line (and fresh translation)
            // instead of merging into the cut-off message.
            let interjected = states
                .get(&job.source)
                .filter(|st| !st.done)
                .map(|st| st.seq)
                .is_some_and(|seq| {
                    states
                        .iter()
                        .any(|(src, other)| *src != job.source && other.last_closed_seq > seq)
                });

            let st = states.entry(job.source.clone()).or_insert_with(|| MsgState {
                done: true,
                ..Default::default()
            });
            // Sentence boundary (or an interjection) → the next result starts
            // a fresh message.
            if st.done || interjected {
                if interjected && !st.committed.is_empty() {
                    // The frontend froze the cut-off line at its committed
                    // text — that counts as a finished message.
                    st.last_closed_seq = st.seq;
                }
                next_seq += 1;
                st.seq = next_seq;
                st.start_sec = job.start_sec;
                st.committed = String::new();
                st.done = false;
            }

            // Reproduce the frontend content merge.
            let content = if job.is_result_final && !job.is_sentence_final {
                st.committed = if st.committed.is_empty() {
                    job.text.clone()
                } else {
                    format!("{} {}", st.committed, job.text)
                };
                st.committed.clone()
            } else if !st.committed.is_empty() {
                format!("{} {}", st.committed, job.text)
            } else {
                job.text.clone()
            };

            let msg_start = st.start_sec;
            if job.is_sentence_final {
                st.done = true;
                st.last_closed_seq = st.seq;
            }

            match crate::bridges::translate::translate(&source_lang, &target, &content).await {
                Ok(translated) => {
                    let payload = TranslationEventPayload {
                        source: job.source,
                        text: translated,
                        start_sec: msg_start,
                        is_result_final: job.is_result_final,
                        is_sentence_final: job.is_sentence_final,
                    };
                    if let Err(err) = app_handle.emit(TRANSLATION_EVENT, payload) {
                        tracing::debug!("Failed to emit translation event: {err}");
                    }
                }
                Err(err) => tracing::debug!("Translation failed: {err}"),
            }
        }
    }
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
