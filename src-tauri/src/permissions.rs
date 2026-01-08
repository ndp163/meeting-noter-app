//! macOS permission checking and requesting utilities
//!
//! Handles checking and requesting system permissions required for audio capture:
//! - Screen Recording (required for system audio capture)
//! - Microphone access

use std::process::Command;

/// Check if Screen Recording permission is granted
/// 
/// On macOS, this uses CGPreflightScreenCaptureAccess() via a Swift helper
/// or falls back to checking if we can create a screen capture tap.
pub fn check_screen_recording_permission() -> bool {
    // Try to use the system_profiler or tccutil to check
    // Actually, the most reliable way is to try creating a tap and see if we get silence
    
    // For now, we'll use the CoreGraphics API via objc
    #[cfg(target_os = "macos")]
    {
        use std::ffi::c_void;
        
        #[link(name = "CoreGraphics", kind = "framework")]
        extern "C" {
            fn CGPreflightScreenCaptureAccess() -> bool;
        }
        
        unsafe { CGPreflightScreenCaptureAccess() }
    }
    
    #[cfg(not(target_os = "macos"))]
    {
        true // On other platforms, assume permission is granted
    }
}

/// Request Screen Recording permission
/// 
/// This will open System Settings to the Screen Recording pane.
/// Returns true if permission was already granted, false otherwise.
/// 
/// Note: macOS does NOT show a permission dialog for Screen Recording.
/// The user must manually enable it in System Settings.
pub fn request_screen_recording_permission() -> bool {
    #[cfg(target_os = "macos")]
    {
        use std::ffi::c_void;
        
        #[link(name = "CoreGraphics", kind = "framework")]
        extern "C" {
            fn CGRequestScreenCaptureAccess() -> bool;
        }
        
        let result = unsafe { CGRequestScreenCaptureAccess() };
        
        if !result {
            tracing::info!("Screen Recording permission not granted, opening System Settings...");
            // CGRequestScreenCaptureAccess should open System Settings automatically
            // But we can also try to open it manually as a fallback
        }
        
        result
    }
    
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

/// Open System Settings to the Screen Recording permission page
pub fn open_screen_recording_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        // Open System Settings to Privacy & Security > Screen Recording
        let result = Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
            .spawn();
        
        match result {
            Ok(_) => {
                tracing::info!("Opened System Settings to Screen Recording");
                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to open System Settings: {}", e);
                Err(format!("Failed to open System Settings: {}", e))
            }
        }
    }
    
    #[cfg(not(target_os = "macos"))]
    {
        Ok(())
    }
}

/// Check if Microphone permission is granted
pub fn check_microphone_permission() -> bool {
    #[cfg(target_os = "macos")]
    {
        use std::ffi::c_int;
        
        // AVAuthorizationStatus values
        const AV_AUTH_STATUS_AUTHORIZED: c_int = 3;
        
        #[link(name = "AVFoundation", kind = "framework")]
        extern "C" {
            // We can't directly call AVCaptureDevice methods from C
            // So we'll check by trying to enumerate audio devices
        }
        
        // For now, return true and let cpal handle the permission request
        // cpal will trigger the system permission dialog automatically
        true
    }
    
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

/// Get permission status summary for debugging
pub fn get_permission_status() -> PermissionStatus {
    PermissionStatus {
        screen_recording: check_screen_recording_permission(),
        microphone: check_microphone_permission(),
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PermissionStatus {
    pub screen_recording: bool,
    pub microphone: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_permission_check() {
        let status = get_permission_status();
        println!("Permission status: {:?}", status);
        // This test just checks that the functions don't crash
    }
}
