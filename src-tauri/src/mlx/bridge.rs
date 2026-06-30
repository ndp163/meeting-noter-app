//! `dlopen` wrapper over the downloaded `libMlxBridge.dylib`.
//!
//! The library is loaded lazily on first use and kept alive for the process
//! lifetime. The C entry point (`mlx_llm_generate`) blocks until generation
//! finishes, so callers must invoke [`generate`] from a blocking thread
//! (`spawn_blocking`). Calls are serialized — mlx runs one GPU inference at a
//! time and the app only ever summarizes one meeting at once.

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_void, CStr, CString};
use std::sync::{Mutex, OnceLock};

type ProgressCb = extern "C" fn(f64, *mut c_void);
type ResultCb = extern "C" fn(bool, *const c_char, *mut c_void);
type GenerateFn = unsafe extern "C" fn(
    *const c_char,        // model dir
    *const c_char,        // system / instructions
    *const c_char,        // user / input
    i32,                  // max tokens
    Option<ProgressCb>,   // load progress (unused here)
    *mut c_void,          // progress ctx
    ResultCb,             // result callback
    *mut c_void,          // result ctx
);

/// Receives the generation result from the C callback.
struct Sink {
    value: Option<Result<String, String>>,
}

extern "C" fn on_result(ok: bool, text: *const c_char, ctx: *mut c_void) {
    // SAFETY: `ctx` is the `&mut Sink` we passed into `mlx_llm_generate`, valid
    // for the duration of the (synchronous) call.
    let sink = unsafe { &mut *(ctx as *mut Sink) };
    let s = if text.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(text) }.to_string_lossy().into_owned()
    };
    sink.value = Some(if ok { Ok(s) } else { Err(s) });
}

static GEN_LOCK: Mutex<()> = Mutex::new(());

fn library() -> Result<&'static Library, String> {
    static LIB: OnceLock<Library> = OnceLock::new();
    if LIB.get().is_none() {
        let path = super::dylib_path();
        // SAFETY: loading our own signed, downloaded dylib. The colocated
        // `mlx.metallib` is found via the library's own directory at init.
        let loaded = unsafe { Library::new(&path) }
            .map_err(|e| format!("Failed to load the local model runtime: {e}"))?;
        let _ = LIB.set(loaded); // a racing thread may have set it first
    }
    LIB.get()
        .ok_or_else(|| "Local model runtime not loaded".to_string())
}

/// Run one summarization-style generation. Blocks until complete.
pub fn generate(model_dir: &str, system: &str, user: &str, max_tokens: i32) -> Result<String, String> {
    let _guard = GEN_LOCK.lock().map_err(|_| "mlx lock poisoned".to_string())?;
    let lib = library()?;
    let func: Symbol<GenerateFn> = unsafe { lib.get(b"mlx_llm_generate\0") }
        .map_err(|e| format!("missing mlx_llm_generate symbol: {e}"))?;

    let c_dir = CString::new(model_dir).map_err(|_| "model dir has NUL byte".to_string())?;
    let c_sys = CString::new(system).map_err(|_| "instructions have NUL byte".to_string())?;
    let c_user = CString::new(user).map_err(|_| "input has NUL byte".to_string())?;

    let mut sink = Sink { value: None };
    unsafe {
        func(
            c_dir.as_ptr(),
            c_sys.as_ptr(),
            c_user.as_ptr(),
            max_tokens,
            None,
            std::ptr::null_mut(),
            on_result,
            &mut sink as *mut Sink as *mut c_void,
        );
    }

    sink.value
        .unwrap_or_else(|| Err("local model returned no result".to_string()))
}
