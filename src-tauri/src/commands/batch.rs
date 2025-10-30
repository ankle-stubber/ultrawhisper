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

/// Normalize a path for consistent comparison
fn normalize_folder_path(path: &str) -> Result<String, String> {
    use std::path::Path;

    // Expand tilde
    let expanded = if path.starts_with("~/") || path == "~" {
        if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
            path.replacen("~", &home, 1)
        } else {
            path.to_string()
        }
    } else {
        path.to_string()
    };

    // Canonicalize if possible (resolves symlinks, removes .., etc.)
    let path_buf = Path::new(&expanded);
    match path_buf.canonicalize() {
        Ok(canonical) => Ok(canonical.to_string_lossy().to_string()),
        Err(_) => {
            // If canonicalize fails (e.g., path doesn't exist yet), use expanded path
            Ok(expanded)
        }
    }
}

/// Add a watch folder
#[tauri::command]
pub fn add_watch_folder(app: AppHandle, folder_path: String) -> Result<(), String> {
    let mut settings = get_settings(&app);

    // Normalize the input path
    let normalized_path = normalize_folder_path(&folder_path)?;

    // Normalize existing folders and check for duplicates
    let existing_normalized: Vec<String> = settings
        .batch_transcription
        .watch_folders
        .iter()
        .filter_map(|p| normalize_folder_path(p).ok())
        .collect();

    if existing_normalized.contains(&normalized_path) {
        return Err("Folder already in watch list (duplicate path detected)".to_string());
    }

    // Add the normalized path
    settings.batch_transcription.watch_folders.push(normalized_path);
    write_settings(&app, settings);
    Ok(())
}

/// Remove a watch folder
#[tauri::command]
pub fn remove_watch_folder(app: AppHandle, folder_path: String) -> Result<(), String> {
    let mut settings = get_settings(&app);

    // Normalize the input path for comparison
    let normalized_path = normalize_folder_path(&folder_path).unwrap_or(folder_path.clone());

    // Remove folders that match when normalized
    let original_count = settings.batch_transcription.watch_folders.len();
    settings.batch_transcription.watch_folders.retain(|f| {
        let f_normalized = normalize_folder_path(f).unwrap_or(f.clone());
        f_normalized != normalized_path
    });

    // Check if anything was actually removed
    if settings.batch_transcription.watch_folders.len() == original_count {
        return Err("Folder not found in watch list".to_string());
    }

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
