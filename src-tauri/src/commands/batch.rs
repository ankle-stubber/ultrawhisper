use crate::managers::batch::{BatchCompleteEvent, BatchTranscriptionManager};
use crate::settings::{get_settings, write_settings, BatchTranscriptionSettings};
use std::sync::Arc;
use tauri::{AppHandle, State};

/// Manually trigger batch processing now (for "Process Now" button)
#[tauri::command]
pub async fn process_batch_now(
    batch_manager: State<'_, Arc<BatchTranscriptionManager>>,
) -> Result<BatchCompleteEvent, String> {
    batch_manager
        .process_now()
        .map_err(|e| format!("Failed to process batch: {}", e))
}

/// Get current batch transcription settings
#[tauri::command]
pub fn get_batch_settings(app: AppHandle) -> Result<BatchTranscriptionSettings, String> {
    let settings = get_settings(&app);
    Ok(settings.batch_transcription)
}

/// Update batch transcription settings
#[tauri::command]
pub fn update_batch_settings(
    app: AppHandle,
    batch_settings: BatchTranscriptionSettings,
) -> Result<(), String> {
    let mut settings = get_settings(&app);
    settings.batch_transcription = batch_settings;
    write_settings(&app, settings);
    Ok(())
}

/// Enable or disable batch transcription
#[tauri::command]
pub fn set_batch_enabled(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = get_settings(&app);
    settings.batch_transcription.enabled = enabled;
    write_settings(&app, settings);
    Ok(())
}

/// Add a watch folder
#[tauri::command]
pub fn add_watch_folder(app: AppHandle, folder_path: String) -> Result<(), String> {
    let mut settings = get_settings(&app);

    // Check if folder already exists
    if settings.batch_transcription.watch_folders.contains(&folder_path) {
        return Err("Folder already in watch list".to_string());
    }

    settings.batch_transcription.watch_folders.push(folder_path);
    write_settings(&app, settings);
    Ok(())
}

/// Remove a watch folder
#[tauri::command]
pub fn remove_watch_folder(app: AppHandle, folder_path: String) -> Result<(), String> {
    let mut settings = get_settings(&app);
    settings.batch_transcription.watch_folders.retain(|f| f != &folder_path);
    write_settings(&app, settings);
    Ok(())
}

/// Update check interval (in seconds)
#[tauri::command]
pub fn set_check_interval(app: AppHandle, interval_seconds: u64) -> Result<(), String> {
    if interval_seconds < 10 {
        return Err("Check interval must be at least 10 seconds".to_string());
    }

    let mut settings = get_settings(&app);
    settings.batch_transcription.check_interval_seconds = interval_seconds;
    write_settings(&app, settings);
    Ok(())
}

/// Update stability timeout (in seconds)
#[tauri::command]
pub fn set_stability_timeout(app: AppHandle, timeout_seconds: u64) -> Result<(), String> {
    if timeout_seconds < 1 {
        return Err("Stability timeout must be at least 1 second".to_string());
    }

    let mut settings = get_settings(&app);
    settings.batch_transcription.stability_timeout_seconds = timeout_seconds;
    write_settings(&app, settings);
    Ok(())
}

/// Set whether to delete files after transcription
#[tauri::command]
pub fn set_delete_after_transcription(app: AppHandle, delete: bool) -> Result<(), String> {
    let mut settings = get_settings(&app);
    settings.batch_transcription.delete_after_transcription = delete;
    write_settings(&app, settings);
    Ok(())
}

/// Set whether to save transcriptions to history
#[tauri::command]
pub fn set_save_to_history(app: AppHandle, save: bool) -> Result<(), String> {
    let mut settings = get_settings(&app);
    settings.batch_transcription.save_to_history = save;
    write_settings(&app, settings);
    Ok(())
}

/// Update file patterns for batch processing
#[tauri::command]
pub fn set_file_patterns(app: AppHandle, patterns: Vec<String>) -> Result<(), String> {
    // Validate patterns
    for pattern in &patterns {
        if !pattern.starts_with("*.") {
            return Err(format!("Invalid pattern '{}'. Patterns must start with '*.' (e.g., '*.wav')", pattern));
        }
    }

    if patterns.is_empty() {
        return Err("At least one file pattern is required".to_string());
    }

    let mut settings = get_settings(&app);
    settings.batch_transcription.file_patterns = patterns;
    write_settings(&app, settings);
    Ok(())
}

/// Validate that a folder path exists and is writable
#[tauri::command]
pub fn validate_watch_folder(folder_path: String) -> Result<bool, String> {
    use std::path::Path;

    let path = Path::new(&folder_path);

    if !path.exists() {
        return Err(format!("Folder does not exist: {}", folder_path));
    }

    if !path.is_dir() {
        return Err(format!("Path is not a directory: {}", folder_path));
    }

    // Try to read the directory to check permissions
    match std::fs::read_dir(path) {
        Ok(_) => Ok(true),
        Err(e) => Err(format!("Cannot read folder: {}", e)),
    }
}
