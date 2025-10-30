use crate::managers::logs::{LogEntry, LogManager};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct GetLogsResponse {
    pub entries: Vec<LogEntry>,
    pub buffer_size: usize,
}

/// Get logs from the in-memory buffer
#[tauri::command]
pub fn get_logs(
    log_manager: State<Arc<LogManager>>,
    limit: Option<usize>,
) -> Result<GetLogsResponse, String> {
    let entries = log_manager.get_logs(limit);
    let buffer_size = log_manager.buffer_size();

    Ok(GetLogsResponse {
        entries,
        buffer_size,
    })
}

/// Clear all logs from the buffer
#[tauri::command]
pub fn clear_logs(log_manager: State<Arc<LogManager>>) -> Result<(), String> {
    log_manager.clear();
    Ok(())
}

/// Mask secrets in log messages before export
fn mask_secrets(message: &str) -> String {
    let mut masked = message.to_string();

    // Patterns to mask (bot tokens, API keys, etc.)
    // Using simple word boundary matching to avoid complex escaping
    let patterns = vec![
        (r"bot_token\s*[=:]\s*\S+", "bot_token=***"),
        (r"api_key\s*[=:]\s*\S+", "api_key=***"),
        (r"token\s*[=:]\s*\S+", "token=***"),
        (r"password\s*[=:]\s*\S+", "password=***"),
        (r"secret\s*[=:]\s*\S+", "secret=***"),
    ];

    for (pattern, replacement) in patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            masked = re.replace_all(&masked, replacement).to_string();
        }
    }

    masked
}

/// Export logs to a file (txt or json format)
#[tauri::command]
pub async fn export_logs(
    log_manager: State<'_, Arc<LogManager>>,
    format: String,
    path: String,
) -> Result<(), String> {
    use std::fs;

    let entries = log_manager.get_logs(None);

    // Mask secrets in all entries
    let masked_entries: Vec<LogEntry> = entries
        .into_iter()
        .map(|mut entry| {
            entry.message = mask_secrets(&entry.message);
            entry
        })
        .collect();

    let content = match format.as_str() {
        "txt" => {
            // Plain text format
            masked_entries
                .iter()
                .map(|entry| {
                    let timestamp = chrono::DateTime::from_timestamp_millis(entry.timestamp)
                        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S%.3f").to_string())
                        .unwrap_or_else(|| "unknown".to_string());
                    format!(
                        "[{}] {:5} {} - {}",
                        timestamp, entry.level, entry.target, entry.message
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
        "json" => {
            // JSON format
            serde_json::to_string_pretty(&masked_entries).map_err(|e| e.to_string())?
        }
        _ => return Err(format!("Unsupported format: {}", format)),
    };

    // Write to file
    fs::write(&path, content).map_err(|e| format!("Failed to write log file: {}", e))?;

    Ok(())
}
