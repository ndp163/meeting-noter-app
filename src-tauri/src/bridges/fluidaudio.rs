use super::{SpeechRecognizer, VoiceDetector};
use async_trait::async_trait;
use std::ffi::{c_char, c_void, CStr, CString};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

type FluidAudioCallback = unsafe extern "C" fn(*const c_char, *const c_char, *mut c_void);

/// Thread-safe counter for tracking callback context allocations (for debugging)
static CALLBACK_CONTEXT_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Safety wrapper for FFI callback context
///
/// This struct ensures safe handling of oneshot channels across FFI boundaries.
/// It uses atomic operations to prevent double-consumption and tracks allocation
/// count for leak detection in debug builds.
struct CallbackContext<T> {
    inner: Option<T>,
    consumed: AtomicBool,
    /// Unique ID for debugging
    #[cfg(debug_assertions)]
    id: usize,
}

impl<T> CallbackContext<T> {
    fn new(inner: T) -> Self {
        let _id = CALLBACK_CONTEXT_COUNT.fetch_add(1, Ordering::Relaxed);
        #[cfg(debug_assertions)]
        {
            Self {
                inner: Some(inner),
                consumed: AtomicBool::new(false),
                id: _id,
            }
        }
        #[cfg(not(debug_assertions))]
        {
            Self {
                inner: Some(inner),
                consumed: AtomicBool::new(false),
            }
        }
    }
}

impl<T> Drop for CallbackContext<T> {
    fn drop(&mut self) {
        CALLBACK_CONTEXT_COUNT.fetch_sub(1, Ordering::Relaxed);
        #[cfg(debug_assertions)]
        {
            let remaining = CALLBACK_CONTEXT_COUNT.load(Ordering::Relaxed);
            if remaining > 10 {
                tracing::warn!(
                    remaining,
                    "Callback contexts still allocated (potential leak)"
                );
            }
        }
    }
}

// Serializes the *stateful* VAD streaming calls (create/process/destroy), which
// mutate per-stream state in the Swift bridge and are not safe to interleave
// across the mic and speaker threads. `transcribe` is a stateless per-call
// request and may run concurrently (one in-flight call per stream's worker).
static FFI_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn get_ffi_lock() -> &'static Mutex<()> {
    FFI_LOCK.get_or_init(|| Mutex::new(()))
}

#[link(name = "FluidAudioBridge")]
extern "C" {
    fn fluid_audio_init(
        model_path: *const c_char,
        language: *const c_char,
        callback: FluidAudioCallback,
        context: *mut c_void,
    );
    fn fluid_audio_transcribe(
        audio_data: *const u8,
        data_len: usize,
        callback: FluidAudioCallback,
        context: *mut c_void,
    );
    fn fluid_audio_shutdown();

    fn fluid_audio_diarize_file(
        path: *const c_char,
        diarize: bool,
        language: *const c_char,
        callback: FluidAudioCallback,
        context: *mut c_void,
    );

    fn fluid_audio_prefetch_diarizer(callback: FluidAudioCallback, context: *mut c_void);
    fn fluid_audio_models_present() -> bool;

    // VAD functions
    fn fluid_audio_vad_create_state(stream_id: *const c_char) -> bool;
    fn fluid_audio_vad_process(
        stream_id: *const c_char,
        audio_data: *const u8,
        data_len: usize,
        out_probability: *mut f32,
    ) -> bool;
    fn fluid_audio_vad_destroy_state(stream_id: *const c_char);
}

/// Thread-safe FluidAudio FFI wrapper
pub struct FluidAudio {
    initialized: Arc<AtomicBool>,
}

impl Default for FluidAudio {
    fn default() -> Self {
        Self::new()
    }
}

impl FluidAudio {
    pub fn new() -> Self {
        Self {
            initialized: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Offline pass: transcribe a whole WAV file and attribute each part to a
    /// speaker. `diarize` clusters multiple speakers (remote track); when false
    /// the file is treated as a single known speaker (mic track). Returns the
    /// segments as a JSON string. Models load lazily inside the Swift bridge.
    pub async fn diarize_file(
        &self,
        path: &str,
        diarize: bool,
        language: &str,
    ) -> Result<String, String> {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<String, String>>();

        let ctx = CallbackContext::new(tx);
        let context = Box::into_raw(Box::new(ctx)) as *mut c_void;

        let c_path = CString::new(path).map_err(|e| format!("Invalid path: {}", e))?;
        let c_lang = CString::new(language).map_err(|e| format!("Invalid language: {}", e))?;

        unsafe {
            fluid_audio_diarize_file(
                c_path.as_ptr(),
                diarize,
                c_lang.as_ptr(),
                string_result_callback,
                context,
            );
        }

        rx.await
            .map_err(|_| "Diarization callback not received".to_string())?
    }

    /// Eagerly download/load the diarizer models (used by onboarding) so the
    /// first real diarization doesn't pay the download cost.
    pub async fn prefetch_diarizer(&self) -> Result<(), String> {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<String, String>>();

        let ctx = CallbackContext::new(tx);
        let context = Box::into_raw(Box::new(ctx)) as *mut c_void;

        unsafe {
            fluid_audio_prefetch_diarizer(string_result_callback, context);
        }

        rx.await
            .map_err(|_| "Prefetch callback not received".to_string())??;
        Ok(())
    }

    /// True only when every required model is already cached on disk, so
    /// onboarding can skip the download step.
    pub fn models_present() -> bool {
        unsafe { fluid_audio_models_present() }
    }
}

/// Shared FFI callback that resolves a oneshot channel with a string result.
unsafe extern "C" fn string_result_callback(
    text_ptr: *const c_char,
    error_ptr: *const c_char,
    context: *mut c_void,
) {
    if context.is_null() {
        tracing::error!("String result callback: context is null");
        return;
    }

    let mut ctx = Box::from_raw(
        context as *mut CallbackContext<tokio::sync::oneshot::Sender<Result<String, String>>>,
    );

    if ctx.consumed.swap(true, Ordering::SeqCst) {
        tracing::warn!("String result callback called multiple times, ignoring");
        return;
    }

    let tx = match ctx.inner.take() {
        Some(t) => t,
        None => {
            tracing::error!("String result callback: sender already taken");
            return;
        }
    };

    let result = if !error_ptr.is_null() {
        let error_str = CStr::from_ptr(error_ptr).to_string_lossy().to_string();
        tracing::error!(error = %error_str, "FluidAudio diarization error");
        Err(error_str)
    } else if !text_ptr.is_null() {
        let text = CStr::from_ptr(text_ptr).to_string_lossy().to_string();
        Ok(text)
    } else {
        Err("Invalid callback: both pointers null".to_string())
    };

    if tx.send(result).is_err() {
        tracing::warn!("String result callback: receiver dropped");
    }
}

#[async_trait]
impl SpeechRecognizer for FluidAudio {
    async fn initialize(&self, language: &str) -> Result<(), String> {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), String>>();

        let ctx = CallbackContext::new(tx);
        let context = Box::into_raw(Box::new(ctx)) as *mut c_void;

        unsafe extern "C" fn init_callback(
            success_ptr: *const c_char,
            error_ptr: *const c_char,
            context: *mut c_void,
        ) {
            if context.is_null() {
                tracing::error!("Init callback: context is null");
                return;
            }

            let mut ctx = Box::from_raw(
                context as *mut CallbackContext<tokio::sync::oneshot::Sender<Result<(), String>>>,
            );

            // Check if already consumed (double-call protection)
            // Use SeqCst for maximum safety in FFI context
            if ctx.consumed.swap(true, Ordering::SeqCst) {
                tracing::warn!("Init callback called multiple times, ignoring");
                // Don't leak - ctx will be properly dropped here since we own it.
                // The first call already took ownership of inner, so drop is safe.
                return;
            }

            let tx = match ctx.inner.take() {
                Some(t) => t,
                None => {
                    tracing::error!("Init callback: sender already taken");
                    return;
                }
            };

            let result = if !error_ptr.is_null() {
                let error_str = CStr::from_ptr(error_ptr).to_string_lossy().to_string();
                tracing::error!(error = %error_str, "FluidAudio init error");
                Err(error_str)
            } else if !success_ptr.is_null() {
                let success_str = CStr::from_ptr(success_ptr).to_string_lossy();
                tracing::info!(message = %success_str, "FluidAudio init success");
                Ok(())
            } else {
                Err("Invalid callback: both pointers null".to_string())
            };

            if tx.send(result).is_err() {
                tracing::warn!("Init callback: receiver dropped");
            }
            // ctx will be dropped here, cleaning up safely
        }

        // FluidAudio bundles its own model; no path is passed. The language
        // selects the ASR model (Japanese -> tdtJa, otherwise English v2).
        let c_lang = CString::new(language).map_err(|e| format!("Invalid language: {}", e))?;
        unsafe {
            fluid_audio_init(std::ptr::null(), c_lang.as_ptr(), init_callback, context);
        }

        let result = rx
            .await
            .map_err(|_| "Initialization callback not received".to_string())?;

        if result.is_ok() {
            self.initialized.store(true, Ordering::Release);
        }

        result
    }

    async fn transcribe(&self, audio: &[f32]) -> Result<String, String> {
        if !self.initialized.load(Ordering::Acquire) {
            return Err("FluidAudio not initialized".to_string());
        }

        let (tx, rx) = tokio::sync::oneshot::channel::<Result<String, String>>();

        let ctx = CallbackContext::new(tx);
        let context = Box::into_raw(Box::new(ctx)) as *mut c_void;

        unsafe extern "C" fn transcribe_callback(
            text_ptr: *const c_char,
            error_ptr: *const c_char,
            context: *mut c_void,
        ) {
            if context.is_null() {
                tracing::error!("Transcribe callback: context is null");
                return;
            }

            let mut ctx = Box::from_raw(
                context
                    as *mut CallbackContext<tokio::sync::oneshot::Sender<Result<String, String>>>,
            );

            // Check if already consumed (double-call protection)
            if ctx.consumed.swap(true, Ordering::SeqCst) {
                tracing::warn!("Transcribe callback called multiple times, ignoring");
                // Don't leak - ctx will be properly dropped here since we own it.
                return;
            }

            let tx = match ctx.inner.take() {
                Some(t) => t,
                None => {
                    tracing::error!("Transcribe callback: sender already taken");
                    return;
                }
            };

            let result = if !error_ptr.is_null() {
                let error_str = CStr::from_ptr(error_ptr).to_string_lossy().to_string();
                tracing::error!(error = %error_str, "FluidAudio transcription error");
                Err(error_str)
            } else if !text_ptr.is_null() {
                let text = CStr::from_ptr(text_ptr).to_string_lossy().to_string();
                Ok(text)
            } else {
                Err("Invalid callback: both pointers null".to_string())
            };

            if tx.send(result).is_err() {
                tracing::warn!("Transcribe callback: receiver dropped");
            }
        }

        // Convert f32 samples to little-endian i16 PCM bytes.
        let pcm_bytes: Vec<u8> = audio
            .iter()
            .flat_map(|&sample| {
                let s = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                s.to_le_bytes()
            })
            .collect();

        // Pin to ensure the buffer lives until the FFI call completes.
        let pcm_bytes = Box::pin(pcm_bytes);

        unsafe {
            fluid_audio_transcribe(
                pcm_bytes.as_ptr(),
                pcm_bytes.len(),
                transcribe_callback,
                context,
            );
        }

        rx.await
            .map_err(|_| "Transcription callback not received".to_string())?
    }

    fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::Acquire)
    }
}

impl VoiceDetector for FluidAudio {
    fn process(&self, stream_id: &str, audio: &[f32]) -> Result<f32, String> {
        if !self.is_initialized() {
            return Err("FluidAudio not initialized".to_string());
        }

        if audio.is_empty() {
            return Ok(0.0); // No data, no voice
        }

        let stream_id_c =
            CString::new(stream_id).map_err(|e| format!("Invalid stream ID: {}", e))?;

        // Convert f32 samples to bytes and pin until the FFI call returns.
        let f32_bytes: Vec<u8> = audio.iter().flat_map(|&f| f.to_le_bytes()).collect();

        let f32_bytes = Box::pin(f32_bytes);
        let mut probability: f32 = 0.0;

        // Safety: ensure pointers are valid before FFI call
        let stream_ptr = stream_id_c.as_ptr();
        let data_ptr = f32_bytes.as_ptr();
        let data_len = f32_bytes.len();
        let prob_ptr = &mut probability as *mut f32;

        if stream_ptr.is_null() || data_ptr.is_null() {
            return Err("Invalid pointers for VAD process".to_string());
        }

        // Lock to serialize FFI calls
        let _ffi_guard = get_ffi_lock()
            .lock()
            .map_err(|e| format!("FFI lock poisoned: {}", e))?;

        let success = unsafe { fluid_audio_vad_process(stream_ptr, data_ptr, data_len, prob_ptr) };

        if success {
            Ok(probability)
        } else {
            Err("VAD processing failed".to_string())
        }
    }

    fn create_stream(&self, stream_id: &str) -> Result<(), String> {
        let stream_id_c =
            CString::new(stream_id).map_err(|e| format!("Invalid stream ID: {}", e))?;

        // Lock to serialize FFI calls
        let _ffi_guard = get_ffi_lock()
            .lock()
            .map_err(|e| format!("FFI lock poisoned: {}", e))?;

        let success = unsafe { fluid_audio_vad_create_state(stream_id_c.as_ptr()) };

        if success {
            Ok(())
        } else {
            Err("Failed to create VAD state".to_string())
        }
    }

    fn destroy_stream(&self, stream_id: &str) {
        if let Ok(stream_id_c) = CString::new(stream_id) {
            // Lock to serialize FFI calls
            if let Ok(_ffi_guard) = get_ffi_lock().lock() {
                unsafe {
                    fluid_audio_vad_destroy_state(stream_id_c.as_ptr());
                }
            }
        }
    }
}

impl Drop for FluidAudio {
    fn drop(&mut self) {
        if self.initialized.load(Ordering::Acquire) {
            unsafe {
                fluid_audio_shutdown();
            }
        }
    }
}
