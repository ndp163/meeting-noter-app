use std::ffi::{c_char, c_void, CStr, CString};
use std::sync::{Arc, Mutex};

type WhisperCallback = unsafe extern "C" fn(*const c_char, *const c_char, *mut c_void);

#[repr(C)]
struct CallbackContext {
    callback: WhisperCallback,
    user_data: *mut c_void,
}

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

pub struct WhisperKit {
    initialized: Arc<Mutex<bool>>,
}

impl WhisperKit {
    pub fn new() -> Self {
        Self {
            initialized: Arc::new(Mutex::new(false)),
        }
    }

    pub async fn initialize(&self, model_path: Option<&str>) -> Result<(), String> {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), String>>();
        let tx = Arc::new(Mutex::new(Some(tx)));

        unsafe extern "C" fn init_callback(
            success_ptr: *const c_char,
            error_ptr: *const c_char,
            context: *mut c_void,
        ) {
            let tx = context as *mut Arc<Mutex<Option<tokio::sync::oneshot::Sender<Result<(), String>>>>>;
            let tx = &*tx;

            if !error_ptr.is_null() {
                let error_str = CStr::from_ptr(error_ptr).to_string_lossy().to_string();
                println!("Rust: Received error from Swift: {}", error_str);
                if let Some(sender) = tx.lock().unwrap().take() {
                    let _ = sender.send(Err(error_str));
                }
            } else if !success_ptr.is_null() {
                let success_str = CStr::from_ptr(success_ptr).to_string_lossy();
                println!("Rust: Received success from Swift: {}", success_str);
                if let Some(sender) = tx.lock().unwrap().take() {
                    let _ = sender.send(Ok(()));
                }
            } else {
                println!("Rust: Received callback with both pointers null");
                if let Some(sender) = tx.lock().unwrap().take() {
                    let _ = sender.send(Err("Invalid callback: both pointers null".to_string()));
                }
            }
        }

        let model_c_str = model_path.map(|p| CString::new(p).unwrap());
        let model_ptr = model_c_str
            .as_ref()
            .map(|s| s.as_ptr())
            .unwrap_or(std::ptr::null());

        let context = Box::into_raw(Box::new(tx)) as *mut c_void;

        unsafe {
            whisper_kit_init(model_ptr, init_callback, context);
        }

        let result = rx.await.map_err(|_| "Initialization callback not received".to_string())?;
        
        unsafe {
            let _ = Box::from_raw(context as *mut Arc<Mutex<Option<tokio::sync::oneshot::Sender<Result<(), String>>>>>);
        }

        if result.is_ok() {
            *self.initialized.lock().unwrap() = true;
        }

        result
    }

    pub async fn transcribe(&self, audio_data: &[f32]) -> Result<String, String> {
        if !*self.initialized.lock().unwrap() {
            return Err("WhisperKit not initialized".to_string());
        }

        let (tx, rx) = tokio::sync::oneshot::channel();
        let tx = Arc::new(Mutex::new(Some(tx)));

        unsafe extern "C" fn transcribe_callback(
            text_ptr: *const c_char,
            error_ptr: *const c_char,
            context: *mut c_void,
        ) {
            let tx = context as *mut Arc<Mutex<Option<tokio::sync::oneshot::Sender<Result<String, String>>>>>;
            let tx = &*tx;

            if !text_ptr.is_null() {
                let text = CStr::from_ptr(text_ptr).to_string_lossy().to_string();
                if let Some(sender) = tx.lock().unwrap().take() {
                    let _ = sender.send(Ok(text));
                }
            } else if !error_ptr.is_null() {
                let error = CStr::from_ptr(error_ptr).to_string_lossy().to_string();
                if let Some(sender) = tx.lock().unwrap().take() {
                    let _ = sender.send(Err(error));
                }
            }
        }

        // Convert f32 audio to i16 bytes for transfer
        let audio_bytes: Vec<u8> = audio_data
            .iter()
            .flat_map(|&sample| {
                let s = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                s.to_le_bytes()
            })
            .collect();

        let context = Box::into_raw(Box::new(tx)) as *mut c_void;

        unsafe {
            whisper_kit_transcribe(
                audio_bytes.as_ptr(),
                audio_bytes.len(),
                transcribe_callback,
                context,
            );
        }

        let result = rx.await.map_err(|_| "Transcription callback not received".to_string())?;
        
        unsafe {
            let _ = Box::from_raw(context as *mut Arc<Mutex<Option<tokio::sync::oneshot::Sender<Result<String, String>>>>>);
        }

        result
    }

    pub async fn transcribe_stream(&self, audio_data: &[f32]) -> Result<String, String> {
        if !*self.initialized.lock().unwrap() {
            return Err("WhisperKit not initialized".to_string());
        }

        let (tx, rx) = tokio::sync::oneshot::channel();
        let tx = Arc::new(Mutex::new(Some(tx)));

        unsafe extern "C" fn stream_callback(
            text_ptr: *const c_char,
            error_ptr: *const c_char,
            context: *mut c_void,
        ) {
            let tx = context as *mut Arc<Mutex<Option<tokio::sync::oneshot::Sender<Result<String, String>>>>>;
            let tx = &*tx;

            if !text_ptr.is_null() {
                let text = CStr::from_ptr(text_ptr).to_string_lossy().to_string();
                if let Some(sender) = tx.lock().unwrap().take() {
                    let _ = sender.send(Ok(text));
                }
            } else if !error_ptr.is_null() {
                let error = CStr::from_ptr(error_ptr).to_string_lossy().to_string();
                if let Some(sender) = tx.lock().unwrap().take() {
                    let _ = sender.send(Err(error));
                }
            }
        }

        // Convert f32 audio to i16 bytes
        let audio_bytes: Vec<u8> = audio_data
            .iter()
            .flat_map(|&sample| {
                let s = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                s.to_le_bytes()
            })
            .collect();

        let context = Box::into_raw(Box::new(tx)) as *mut c_void;

        unsafe {
            whisper_kit_transcribe_stream(
                audio_bytes.as_ptr(),
                audio_bytes.len(),
                stream_callback,
                context,
            );
        }

        let result = rx.await.map_err(|_| "Stream transcription callback not received".to_string())?;
        
        unsafe {
            let _ = Box::from_raw(context as *mut Arc<Mutex<Option<tokio::sync::oneshot::Sender<Result<String, String>>>>>);
        }

        result
    }
}

impl Drop for WhisperKit {
    fn drop(&mut self) {
        unsafe {
            whisper_kit_shutdown();
        }
    }
}
