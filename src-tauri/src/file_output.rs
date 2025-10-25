use chrono::Local;
use log::debug;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub fn save_transcription_to_file(
    text: &str,
    app_handle: &AppHandle,
    custom_path: Option<String>,
) -> Result<(), String> {
    // Determine output directory based on custom path or default
    let output_dir = if let Some(custom) = custom_path {
        // Expand tilde to home directory if present
        let expanded = expand_tilde(&custom);
        PathBuf::from(expanded)
    } else {
        // Default to Documents/UltraWhisper
        app_handle
            .path()
            .document_dir()
            .map_err(|e| format!("Failed to get documents dir: {}", e))?
            .join("UltraWhisper")
    };

    fs::create_dir_all(&output_dir)
        .map_err(|e| format!("Failed to create output dir: {}", e))?;

    // Timestamped markdown filename
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("transcription_{}.md", timestamp);
    let filepath = output_dir.join(filename);

    fs::write(&filepath, text).map_err(|e| format!("Failed to write file: {}", e))?;

    debug!("Saved transcription to: {:?}", filepath);

    Ok(())
}

// Helper function to expand ~ to home directory
fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") || path == "~" {
        if let Ok(home) = std::env::var("HOME") {
            return path.replacen("~", &home, 1);
        }
        // On Windows, try USERPROFILE
        if let Ok(home) = std::env::var("USERPROFILE") {
            return path.replacen("~", &home, 1);
        }
    }
    path.to_string()
}

