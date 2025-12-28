use std::ffi::{c_char, c_void, CStr, CString};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

type WhisperCallback = unsafe extern "C" fn(*const c_char, *const c_char, *mut c_void);

#[link(name = "WhisperKitBridge")]
extern "C" {
    fn whisper_kit_init(model_path: *const c_char, callback: WhisperCallback, context: *mut c_void);
    fn whisper_kit_transcribe(
        audio_data: *const u8,
        data_len: usize,
        callback: WhisperCallback,
        context: *mut c_void,
    );
    fn whisper_kit_transcribe_stream(
        audio_data: *const u8,
        data_len: usize,
        callback: WhisperCallback,
        context: *mut c_void,
    );
    fn whisper_kit_shutdown();
}

/// Thread-safe WhisperKit FFI wrapper
pub struct WhisperKit {
    initialized: Arc<AtomicBool>,
}

impl Default for WhisperKit {
    fn default() -> Self {
        Self::new()
    }
}

impl WhisperKit {
    pub fn new() -> Self {
        Self {
            initialized: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn initialize(&self, model_path: Option<&str>) -> Result<(), String> {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), String>>();
        
        // Use Box for simpler context management
        let context = Box::into_raw(Box::new(tx)) as *mut c_void;

        unsafe extern "C" fn init_callback(
            success_ptr: *const c_char,
            error_ptr: *const c_char,
            context: *mut c_void,
        ) {
            let tx = Box::from_raw(context as *mut tokio::sync::oneshot::Sender<Result<(), String>>);
            
            let result = if !error_ptr.is_null() {
                let error_str = CStr::from_ptr(error_ptr).to_string_lossy().to_string();
                eprintln!("WhisperKit init error: {}", error_str);
                // Note: Swift manages string memory lifecycle
                Err(error_str)
            } else if !success_ptr.is_null() {
                let success_str = CStr::from_ptr(success_ptr).to_string_lossy();
                eprintln!("WhisperKit init success: {}", success_str);
                // Note: Swift manages string memory lifecycle
                Ok(())
            } else {
                Err("Invalid callback: both pointers null".to_string())
            };
            
            let _ = tx.send(result);
        }

        let model_c_str = model_path
            .map(|p| CString::new(p).map_err(|e| format!("Invalid model path: {}", e)))
            .transpose()?;
        
        let model_ptr = model_c_str
            .as_ref()
            .map(|s| s.as_ptr())
            .unwrap_or(std::ptr::null());

        unsafe {
            whisper_kit_init(model_ptr, init_callback, context);
        }

        let result = rx.await.map_err(|_| "Initialization callback not received".to_string())?;
        
        if result.is_ok() {
            self.initialized.store(true, Ordering::Release);
        }

        result
    }

    pub async fn transcribe(&self, audio_data: &[f32]) -> Result<String, String> {
        self.transcribe_internal(audio_data, false).await
    }

    pub async fn transcribe_stream(&self, audio_data: &[f32]) -> Result<String, String> {
        self.transcribe_internal(audio_data, true).await
    }

    /// Internal transcription method to avoid code duplication
    async fn transcribe_internal(&self, audio_data: &[f32], is_stream: bool) -> Result<String, String> {
        if !self.initialized.load(Ordering::Acquire) {
            return Err("WhisperKit not initialized".to_string());
        }

        let (tx, rx) = tokio::sync::oneshot::channel();
        let context = Box::into_raw(Box::new(tx)) as *mut c_void;

        unsafe extern "C" fn transcribe_callback(
            text_ptr: *const c_char,
            error_ptr: *const c_char,
            context: *mut c_void,
        ) {
            let tx = Box::from_raw(context as *mut tokio::sync::oneshot::Sender<Result<String, String>>);
            
            let result = if !text_ptr.is_null() {
                let text = CStr::from_ptr(text_ptr).to_string_lossy().to_string();
                // Note: Swift manages string memory lifecycle
                Ok(text)
            } else if !error_ptr.is_null() {
                let error = CStr::from_ptr(error_ptr).to_string_lossy().to_string();
                // Note: Swift manages string memory lifecycle
                Err(error)
            } else {
                Err("Invalid callback: both pointers null".to_string())
            };
            
            let _ = tx.send(result);
        }

        // Convert f32 audio to i16 bytes for transfer
        let audio_bytes: Vec<u8> = audio_data
            .iter()
            .flat_map(|&sample| {
                let s = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                s.to_le_bytes()
            })
            .collect();

        // Pin audio_bytes to ensure it lives until FFI completes
        let audio_bytes = Box::pin(audio_bytes);

        unsafe {
            if is_stream {
                whisper_kit_transcribe_stream(
                    audio_bytes.as_ptr(),
                    audio_bytes.len(),
                    transcribe_callback,
                    context,
                );
            } else {
                whisper_kit_transcribe(
                    audio_bytes.as_ptr(),
                    audio_bytes.len(),
                    transcribe_callback,
                    context,
                );
            }
        }

        rx.await.map_err(|_| "Transcription callback not received".to_string())?
    }

    /// Check if WhisperKit is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::Acquire)
    }
}

impl Drop for WhisperKit {
    fn drop(&mut self) {
        if self.initialized.load(Ordering::Acquire) {
            unsafe {
                whisper_kit_shutdown();
            }
        }
    }
}

// Safety: WhisperKit uses atomic operations and is safe to share across threads
unsafe impl Send for WhisperKit {}
unsafe impl Sync for WhisperKit {}
