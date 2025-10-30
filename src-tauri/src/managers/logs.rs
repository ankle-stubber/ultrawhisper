use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

const MAX_LOG_ENTRIES: usize = 1000;
const EVENT_COALESCE_MS: u64 = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: i64,
    pub level: String,
    pub target: String,
    pub message: String,
}

/// Manages in-memory log buffer with circular queue behavior
pub struct LogManager {
    buffer: Arc<Mutex<VecDeque<LogEntry>>>,
    app_handle: AppHandle,
    last_emit: Arc<Mutex<std::time::Instant>>,
}

impl LogManager {
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            buffer: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_LOG_ENTRIES))),
            app_handle,
            last_emit: Arc::new(Mutex::new(std::time::Instant::now())),
        }
    }

    /// Push a log entry to the buffer and emit event (rate-limited)
    pub fn push(&self, level: String, target: String, message: String) {
        let entry = LogEntry {
            timestamp: Utc::now().timestamp_millis(),
            level: level.clone(),
            target,
            message,
        };

        // Add to buffer (drop oldest if at capacity)
        {
            let mut buffer = self.buffer.lock().unwrap();
            if buffer.len() >= MAX_LOG_ENTRIES {
                buffer.pop_front();
            }
            buffer.push_back(entry.clone());
        }

        // Rate-limited event emission
        let should_emit = {
            let mut last_emit = self.last_emit.lock().unwrap();
            let now = std::time::Instant::now();
            if now.duration_since(*last_emit) >= Duration::from_millis(EVENT_COALESCE_MS) {
                *last_emit = now;
                true
            } else {
                false
            }
        };

        if should_emit {
            let _ = self.app_handle.emit("log-entry", entry);
        }
    }

    /// Get all logs in buffer (or last N entries)
    pub fn get_logs(&self, limit: Option<usize>) -> Vec<LogEntry> {
        let buffer = self.buffer.lock().unwrap();
        let entries: Vec<LogEntry> = buffer.iter().cloned().collect();

        match limit {
            Some(n) => {
                let start = entries.len().saturating_sub(n);
                entries[start..].to_vec()
            }
            None => entries,
        }
    }

    /// Get current buffer size
    pub fn buffer_size(&self) -> usize {
        self.buffer.lock().unwrap().len()
    }

    /// Clear all logs
    pub fn clear(&self) {
        self.buffer.lock().unwrap().clear();
        let _ = self.app_handle.emit("log-cleared", ());
    }
}
