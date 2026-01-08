//! Permission-related Tauri commands

use crate::permissions::{
    check_screen_recording_permission, 
    request_screen_recording_permission,
    open_screen_recording_settings,
    get_permission_status,
    PermissionStatus,
};

/// Check all permission statuses
#[tauri::command]
pub fn check_permissions() -> PermissionStatus {
    get_permission_status()
}

/// Check if screen recording permission is granted
#[tauri::command]
pub fn check_screen_recording() -> bool {
    check_screen_recording_permission()
}

/// Request screen recording permission
/// 
/// This will:
/// 1. Check if permission is already granted
/// 2. If not, open System Settings to the Screen Recording pane
/// 
/// Returns true if permission is granted, false otherwise.
#[tauri::command]
pub fn request_screen_recording() -> bool {
    if check_screen_recording_permission() {
        tracing::info!("Screen recording permission already granted");
        return true;
    }
    
    // Try the CGRequestScreenCaptureAccess first
    let result = request_screen_recording_permission();
    
    if !result {
        // Open System Settings as a fallback
        let _ = open_screen_recording_settings();
    }
    
    result
}

/// Open System Settings to Screen Recording permission page
#[tauri::command]
pub fn open_permission_settings() -> Result<(), String> {
    open_screen_recording_settings()
}
