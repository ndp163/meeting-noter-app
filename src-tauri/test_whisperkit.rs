// Simple test to check if WhisperKit library can be loaded

use std::ffi::{c_char, c_void};

type WhisperCallback = extern "C" fn(*const c_char, *const c_char, *mut c_void);

#[link(name = "WhisperKitBridge")]
extern "C" {
    fn whisper_kit_init(model_path: *const c_char, callback: WhisperCallback, context: *mut c_void);
}

extern "C" fn test_callback(
    success_ptr: *const c_char,
    error_ptr: *const c_char,
    _context: *mut c_void,
) {
    println!("Callback invoked!");
    
    if !success_ptr.is_null() {
        unsafe {
            let success = std::ffi::CStr::from_ptr(success_ptr).to_string_lossy();
            println!("Success: {}", success);
        }
    }
    
    if !error_ptr.is_null() {
        unsafe {
            let error = std::ffi::CStr::from_ptr(error_ptr).to_string_lossy();
            println!("Error: {}", error);
        }
    }
}

fn main() {
    println!("Testing WhisperKit library loading...");
    
    unsafe {
        println!("Calling whisper_kit_init...");
        whisper_kit_init(std::ptr::null(), test_callback, std::ptr::null_mut());
        println!("whisper_kit_init called successfully");
    }
    
    // Wait for callback
    println!("Waiting for callback...");
    std::thread::sleep(std::time::Duration::from_secs(60));
    println!("Test complete");
}
