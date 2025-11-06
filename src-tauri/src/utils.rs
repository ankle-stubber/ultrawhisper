use crate::managers::audio::AudioRecordingManager;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

// Re-export all utility modules for easy access
// pub use crate::audio_feedback::*;
pub use crate::clipboard::*;
pub use crate::overlay::*;
pub use crate::tray::*;

/// Centralized cancellation function that can be called from anywhere in the app.
/// Handles cancelling both recording and transcription operations and updates UI state.
pub fn cancel_current_operation(app: &AppHandle) {
    println!("Initiating operation cancellation...");

    // Cancel any ongoing recording
    if let Some(audio_manager) = app.try_state::<Arc<AudioRecordingManager>>() {
        audio_manager.cancel_recording();
    }

    // Emit cancellation event for workflows
    let _ = app.emit("operation-cancelled", ());

    // Update tray icon and menu to idle state
    change_tray_icon(app, crate::tray::TrayIconState::Idle);

    println!("Operation cancellation completed - returned to idle state");
}
