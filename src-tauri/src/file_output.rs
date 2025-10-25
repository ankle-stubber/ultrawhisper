use chrono::Local;
use log::debug;
use std::fs;
use tauri::{AppHandle, Manager};

pub fn save_transcription_to_file(text: &str, app_handle: &AppHandle) -> Result<(), String> {
    // Resolve Documents/UltraWhisper directory
    let output_dir = app_handle
        .path()
        .document_dir()
        .map_err(|e| format!("Failed to get documents dir: {}", e))?
        .join("UltraWhisper");

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

