//! Apple Translation framework FFI wrapper (on-device, offline; macOS 26+).
//!
//! Mirrors the callback style of [`fluidaudio`]: each Swift `@_cdecl` function
//! takes a `(text, error, context)` callback; we bridge it to a oneshot channel.
//! Translation itself is headless; only pack download shows a system sheet.

use std::ffi::{c_char, c_void, CStr, CString};
use tokio::sync::oneshot;

type Cb = unsafe extern "C" fn(*const c_char, *const c_char, *mut c_void);

#[link(name = "FluidAudioBridge")]
extern "C" {
    fn apple_translate_status(
        source: *const c_char,
        target: *const c_char,
        callback: Cb,
        context: *mut c_void,
    );
    fn apple_translate_text(
        source: *const c_char,
        target: *const c_char,
        text: *const c_char,
        callback: Cb,
        context: *mut c_void,
    );
    fn apple_translate_download(
        source: *const c_char,
        target: *const c_char,
        callback: Cb,
        context: *mut c_void,
    );
}

/// Resolves the oneshot with the string (or error) the Swift side produced.
/// Swift guarantees exactly one call; the context is a boxed sender.
unsafe extern "C" fn string_cb(
    text: *const c_char,
    error: *const c_char,
    context: *mut c_void,
) {
    if context.is_null() {
        tracing::error!("translate callback: null context");
        return;
    }
    let tx = Box::from_raw(context as *mut oneshot::Sender<Result<String, String>>);
    let result = if !error.is_null() {
        Err(CStr::from_ptr(error).to_string_lossy().into_owned())
    } else if !text.is_null() {
        Ok(CStr::from_ptr(text).to_string_lossy().into_owned())
    } else {
        Err("translate callback: both pointers null".to_string())
    };
    let _ = tx.send(result);
}

/// Invoke an FFI function that takes our `(callback, context)` pair and await
/// the single string result. `invoke` runs synchronously; the Swift side reads
/// its C-string args before returning, so borrowed `CString`s stay valid.
async fn call<F>(invoke: F) -> Result<String, String>
where
    F: FnOnce(Cb, *mut c_void),
{
    let (tx, rx) = oneshot::channel::<Result<String, String>>();
    let ctx = Box::into_raw(Box::new(tx)) as *mut c_void;
    invoke(string_cb, ctx);
    rx.await
        .map_err(|_| "translate callback dropped".to_string())?
}

/// Availability of a language pair: `"installed"`, `"supported"` (needs
/// download) or `"unsupported"`. Errors on macOS < 26.
pub async fn status(source: &str, target: &str) -> Result<String, String> {
    let s = CString::new(source).map_err(|e| e.to_string())?;
    let t = CString::new(target).map_err(|e| e.to_string())?;
    call(|cb, ctx| unsafe { apple_translate_status(s.as_ptr(), t.as_ptr(), cb, ctx) }).await
}

/// Translate one string (headless; requires the pack installed). ~20ms.
pub async fn translate(source: &str, target: &str, text: &str) -> Result<String, String> {
    let s = CString::new(source).map_err(|e| e.to_string())?;
    let t = CString::new(target).map_err(|e| e.to_string())?;
    let x = CString::new(text).map_err(|e| e.to_string())?;
    call(|cb, ctx| unsafe { apple_translate_text(s.as_ptr(), t.as_ptr(), x.as_ptr(), cb, ctx) }).await
}

/// Trigger the one-time pack-download sheet. Resolves when the user finishes
/// (or an error occurs). Must be driven from the main thread — the Swift side
/// dispatches to the main actor itself.
pub async fn download(source: &str, target: &str) -> Result<(), String> {
    let s = CString::new(source).map_err(|e| e.to_string())?;
    let t = CString::new(target).map_err(|e| e.to_string())?;
    call(|cb, ctx| unsafe { apple_translate_download(s.as_ptr(), t.as_ptr(), cb, ctx) })
        .await
        .map(|_| ())
}
