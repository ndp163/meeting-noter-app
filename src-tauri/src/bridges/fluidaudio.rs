use std::ffi::{c_char, c_void, CStr, CString};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

type FluidAudioCallback = unsafe extern "C" fn(*const c_char, *const c_char, *mut c_void);

#[link(name = "WhisperKitBridge")]
extern "C" {
    fn fluid_audio_init(model_path: *const c_char, callback: FluidAudioCallback, context: *mut c_void);
    fn fluid_audio_transcribe(
        audio_data: *const u8,
        data_len: usize,
        callback: FluidAudioCallback,
        context: *mut c_void,
    );
    fn fluid_audio_transcribe_stream(
        audio_data: *const u8,
        data_len: usize,
        callback: FluidAudioCallback,
        context: *mut c_void,
    );
    fn fluid_audio_shutdown();
    
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

    pub async fn initialize(&self, model_path: Option<&str>) -> Result<(), String> {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), String>>();
        
        let context = Box::into_raw(Box::new(tx)) as *mut c_void;

        unsafe extern "C" fn init_callback(
            success_ptr: *const c_char,
            error_ptr: *const c_char,
            context: *mut c_void,
        ) {
            let tx = Box::from_raw(context as *mut tokio::sync::oneshot::Sender<Result<(), String>>);
            
            let result = if !error_ptr.is_null() {
                let error_str = CStr::from_ptr(error_ptr).to_string_lossy().to_string();
                eprintln!("FluidAudio init error: {}", error_str);
                Err(error_str)
            } else if !success_ptr.is_null() {
                let success_str = CStr::from_ptr(success_ptr).to_string_lossy();
                eprintln!("FluidAudio init success: {}", success_str);
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
            fluid_audio_init(model_ptr, init_callback, context);
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

    async fn transcribe_internal(&self, audio_data: &[f32], is_stream: bool) -> Result<String, String> {
        if !self.initialized.load(Ordering::Acquire) {
            return Err("FluidAudio not initialized".to_string());
        }

        let (tx, rx) = tokio::sync::oneshot::channel::<Result<String, String>>();
        let context = Box::into_raw(Box::new(tx)) as *mut c_void;

        unsafe extern "C" fn transcribe_callback(
            text_ptr: *const c_char,
            error_ptr: *const c_char,
            context: *mut c_void,
        ) {
            let tx = Box::from_raw(context as *mut tokio::sync::oneshot::Sender<Result<String, String>>);
            
            let result = if !error_ptr.is_null() {
                let error_str = CStr::from_ptr(error_ptr).to_string_lossy().to_string();
                eprintln!("FluidAudio transcription error: {}", error_str);
                Err(error_str)
            } else if !text_ptr.is_null() {
                let text = CStr::from_ptr(text_ptr).to_string_lossy().to_string();
                Ok(text)
            } else {
                Err("Invalid callback: both pointers null".to_string())
            };
            
            let _ = tx.send(result);
        }

        // Convert f32 to i16 PCM format
        let pcm_data: Vec<i16> = audio_data
            .iter()
            .map(|&sample| (sample * i16::MAX as f32) as i16)
            .collect();

        let byte_data: &[u8] = unsafe {
            std::slice::from_raw_parts(
                pcm_data.as_ptr() as *const u8,
                pcm_data.len() * std::mem::size_of::<i16>(),
            )
        };

        unsafe {
            if is_stream {
                fluid_audio_transcribe_stream(
                    byte_data.as_ptr(),
                    byte_data.len(),
                    transcribe_callback,
                    context,
                );
            } else {
                fluid_audio_transcribe(
                    byte_data.as_ptr(),
                    byte_data.len(),
                    transcribe_callback,
                    context,
                );
            }
        }

        rx.await.map_err(|_| "Transcription callback not received".to_string())?
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::Acquire)
    }
    
    /// Process audio chunk with VAD and return voice probability
    pub fn vad_process(&self, stream_id: &str, audio_data: &[f32]) -> Result<f32, String> {
        if !self.is_initialized() {
            return Err("FluidAudio not initialized".to_string());
        }
        
        let stream_id_c = CString::new(stream_id)
            .map_err(|e| format!("Invalid stream ID: {}", e))?;
        
        // Convert f32 to bytes
        let byte_data: Vec<u8> = audio_data
            .iter()
            .flat_map(|&f| f.to_le_bytes())
            .collect();
        
        let mut probability: f32 = 0.0;
        
        let success = unsafe {
            fluid_audio_vad_process(
                stream_id_c.as_ptr(),
                byte_data.as_ptr(),
                byte_data.len(),
                &mut probability as *mut f32,
            )
        };
        
        if success {
            Ok(probability)
        } else {
            Err("VAD processing failed".to_string())
        }
    }
    
    /// Create VAD state for a stream
    pub fn vad_create_state(&self, stream_id: &str) -> Result<(), String> {
        let stream_id_c = CString::new(stream_id)
            .map_err(|e| format!("Invalid stream ID: {}", e))?;
        
        let success = unsafe {
            fluid_audio_vad_create_state(stream_id_c.as_ptr())
        };
        
        if success {
            Ok(())
        } else {
            Err("Failed to create VAD state".to_string())
        }
    }
    
    /// Destroy VAD state for a stream
    pub fn vad_destroy_state(&self, stream_id: &str) {
        if let Ok(stream_id_c) = CString::new(stream_id) {
            unsafe {
                fluid_audio_vad_destroy_state(stream_id_c.as_ptr());
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
