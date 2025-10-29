use anyhow::Result;
use chrono::{DateTime, Local, Utc};
use log::{debug, error};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_sql::{Migration, MigrationKind};

use crate::audio_toolkit::save_wav_file;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: i64,
    pub file_name: String,
    pub timestamp: i64,
    pub saved: bool,
    pub title: String,
    pub transcription_text: String,
    pub workflow_id: Option<String>,
    pub workflow_name: Option<String>,
}

pub struct HistoryManager {
    app_handle: AppHandle,
    recordings_dir: PathBuf,
    db_path: PathBuf,
}

impl HistoryManager {
    pub fn new(app_handle: &AppHandle) -> Result<Self> {
        // Create recordings directory in app data dir
        let app_data_dir = app_handle.path().app_data_dir()?;
        let recordings_dir = app_data_dir.join("recordings");
        let db_path = app_data_dir.join("history.db");

        // Ensure recordings directory exists
        if !recordings_dir.exists() {
            fs::create_dir_all(&recordings_dir)?;
            debug!("Created recordings directory: {:?}", recordings_dir);
        }

        let manager = Self {
            app_handle: app_handle.clone(),
            recordings_dir,
            db_path,
        };

        // Initialize database
        manager.init_database()?;

        Ok(manager)
    }

    pub fn get_migrations() -> Vec<Migration> {
        vec![
            Migration {
                version: 1,
                description: "create_transcription_history_table",
                sql: "CREATE TABLE IF NOT EXISTS transcription_history (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    file_name TEXT NOT NULL,
                    timestamp INTEGER NOT NULL,
                    saved BOOLEAN NOT NULL DEFAULT 0,
                    title TEXT NOT NULL,
                    transcription_text TEXT NOT NULL
                );",
                kind: MigrationKind::Up,
            },
            Migration {
                version: 2,
                description: "add_workflow_tracking",
                sql: "ALTER TABLE transcription_history ADD COLUMN workflow_id TEXT;
                     ALTER TABLE transcription_history ADD COLUMN workflow_name TEXT;",
                kind: MigrationKind::Up,
            },
        ]
    }

    fn init_database(&self) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;

        // Create table with all columns including workflow fields
        conn.execute(
            "CREATE TABLE IF NOT EXISTS transcription_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_name TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                saved BOOLEAN NOT NULL DEFAULT 0,
                title TEXT NOT NULL,
                transcription_text TEXT NOT NULL,
                workflow_id TEXT,
                workflow_name TEXT
            )",
            [],
        )?;

        // Check if we need to add workflow columns to existing table
        // This handles the case where the table exists but without the new columns
        let _ = conn.execute(
            "ALTER TABLE transcription_history ADD COLUMN workflow_id TEXT",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE transcription_history ADD COLUMN workflow_name TEXT",
            [],
        );

        debug!("Database initialized at: {:?}", self.db_path);
        Ok(())
    }

    fn get_connection(&self) -> Result<Connection> {
        Ok(Connection::open(&self.db_path)?)
    }

    /// Save a transcription to history (both database and WAV file)
    pub async fn save_transcription(
        &self,
        audio_samples: Vec<f32>,
        transcription_text: String,
        workflow_id: Option<&str>,
        workflow_name: Option<&str>,
    ) -> Result<()> {
        // If history limit is 0, do not save at all.
        if crate::settings::get_history_limit(&self.app_handle) == 0 {
            return Ok(());
        }

        let timestamp = Utc::now().timestamp();
        let file_name = format!("handy-{}.wav", timestamp);
        let title = self.format_timestamp_title(timestamp);

        // Save WAV file
        let file_path = self.recordings_dir.join(&file_name);
        save_wav_file(file_path, &audio_samples).await?;

        // Save to database
        self.save_to_database(
            file_name,
            timestamp,
            title,
            transcription_text,
            workflow_id.map(|s| s.to_string()),
            workflow_name.map(|s| s.to_string()),
        )?;

        // Clean up old entries
        self.cleanup_old_entries()?;

        // Emit history updated event
        if let Err(e) = self.app_handle.emit("history-updated", ()) {
            error!("Failed to emit history-updated event: {}", e);
        }

        Ok(())
    }

    /// Save a transcription to history using an existing audio file
    ///
    /// This method is used when the audio file has already been written (e.g., by the streaming WAV writer).
    /// It only inserts a database row, runs cleanup, and emits the history-updated event.
    /// The WAV file must already exist in the recordings directory.
    ///
    /// # Arguments
    /// * `file_name` - The filename of the existing WAV file (e.g., "handy-1234567890.wav")
    /// * `transcription_text` - The transcribed text
    /// * `workflow_id` - Optional workflow ID for tracking
    /// * `workflow_name` - Optional workflow name for display
    pub async fn save_transcription_with_path(
        &self,
        file_name: String,
        transcription_text: String,
        workflow_id: Option<&str>,
        workflow_name: Option<&str>,
    ) -> Result<()> {
        // If history limit is 0, do not save at all.
        if crate::settings::get_history_limit(&self.app_handle) == 0 {
            return Ok(());
        }

        // Extract timestamp from filename (format: "handy-<timestamp>.wav")
        let timestamp = file_name
            .strip_prefix("handy-")
            .and_then(|s| s.strip_suffix(".wav"))
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or_else(|| Utc::now().timestamp());

        let title = self.format_timestamp_title(timestamp);

        // Save to database (WAV file already exists)
        self.save_to_database(
            file_name,
            timestamp,
            title,
            transcription_text,
            workflow_id.map(|s| s.to_string()),
            workflow_name.map(|s| s.to_string()),
        )?;

        // Clean up old entries
        self.cleanup_old_entries()?;

        // Emit history updated event
        if let Err(e) = self.app_handle.emit("history-updated", ()) {
            error!("Failed to emit history-updated event: {}", e);
        }

        Ok(())
    }

    fn save_to_database(
        &self,
        file_name: String,
        timestamp: i64,
        title: String,
        transcription_text: String,
        workflow_id: Option<String>,
        workflow_name: Option<String>,
    ) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute(
            "INSERT INTO transcription_history (file_name, timestamp, saved, title, transcription_text, workflow_id, workflow_name)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![file_name, timestamp, false, title, transcription_text, workflow_id, workflow_name],
        )?;

        debug!("Saved transcription to database with workflow: {:?}", workflow_name);
        Ok(())
    }

    fn cleanup_old_entries(&self) -> Result<()> {
        let conn = self.get_connection()?;

        // Get all entries that are not saved, ordered by timestamp desc
        let mut stmt = conn.prepare(
            "SELECT id, file_name FROM transcription_history WHERE saved = 0 ORDER BY timestamp DESC"
        )?;

        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, i64>("id")?, row.get::<_, String>("file_name")?))
        })?;

        let mut entries: Vec<(i64, String)> = Vec::new();
        for row in rows {
            entries.push(row?);
        }

        let limit = crate::settings::get_history_limit(&self.app_handle);
        if entries.len() > limit {
            let entries_to_delete = &entries[limit..];

            for (id, file_name) in entries_to_delete {
                // Delete database entry
                conn.execute(
                    "DELETE FROM transcription_history WHERE id = ?1",
                    params![id],
                )?;

                // Delete WAV file
                let file_path = self.recordings_dir.join(file_name);
                if file_path.exists() {
                    if let Err(e) = fs::remove_file(&file_path) {
                        error!("Failed to delete WAV file {}: {}", file_name, e);
                    } else {
                        debug!("Deleted old WAV file: {}", file_name);
                    }
                }
            }

            debug!("Cleaned up {} old history entries", entries_to_delete.len());
        }

        Ok(())
    }

    pub async fn get_history_entries(&self) -> Result<Vec<HistoryEntry>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(
            "SELECT id, file_name, timestamp, saved, title, transcription_text, workflow_id, workflow_name
             FROM transcription_history ORDER BY timestamp DESC"
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(HistoryEntry {
                id: row.get("id")?,
                file_name: row.get("file_name")?,
                timestamp: row.get("timestamp")?,
                saved: row.get("saved")?,
                title: row.get("title")?,
                transcription_text: row.get("transcription_text")?,
                workflow_id: row.get("workflow_id")?,
                workflow_name: row.get("workflow_name")?,
            })
        })?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(row?);
        }

        Ok(entries)
    }

    pub async fn toggle_saved_status(&self, id: i64) -> Result<()> {
        let conn = self.get_connection()?;

        // Get current saved status
        let current_saved: bool = conn.query_row(
            "SELECT saved FROM transcription_history WHERE id = ?1",
            params![id],
            |row| row.get("saved"),
        )?;

        let new_saved = !current_saved;

        conn.execute(
            "UPDATE transcription_history SET saved = ?1 WHERE id = ?2",
            params![new_saved, id],
        )?;

        debug!("Toggled saved status for entry {}: {}", id, new_saved);

        // Emit history updated event
        if let Err(e) = self.app_handle.emit("history-updated", ()) {
            error!("Failed to emit history-updated event: {}", e);
        }

        Ok(())
    }

    pub fn get_audio_file_path(&self, file_name: &str) -> PathBuf {
        self.recordings_dir.join(file_name)
    }

    pub async fn get_entry_by_id(&self, id: i64) -> Result<Option<HistoryEntry>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(
            "SELECT id, file_name, timestamp, saved, title, transcription_text, workflow_id, workflow_name
             FROM transcription_history WHERE id = ?1",
        )?;

        let entry = stmt
            .query_row([id], |row| {
                Ok(HistoryEntry {
                    id: row.get("id")?,
                    file_name: row.get("file_name")?,
                    timestamp: row.get("timestamp")?,
                    saved: row.get("saved")?,
                    title: row.get("title")?,
                    transcription_text: row.get("transcription_text")?,
                    workflow_id: row.get("workflow_id")?,
                    workflow_name: row.get("workflow_name")?,
                })
            })
            .optional()?;

        Ok(entry)
    }

    pub async fn delete_entry(&self, id: i64) -> Result<()> {
        let conn = self.get_connection()?;

        // Get the entry to find the file name
        if let Some(entry) = self.get_entry_by_id(id).await? {
            // Delete the audio file first
            let file_path = self.get_audio_file_path(&entry.file_name);
            if file_path.exists() {
                if let Err(e) = fs::remove_file(&file_path) {
                    error!("Failed to delete audio file {}: {}", entry.file_name, e);
                    // Continue with database deletion even if file deletion fails
                }
            }
        }

        // Delete from database
        conn.execute(
            "DELETE FROM transcription_history WHERE id = ?1",
            params![id],
        )?;

        debug!("Deleted history entry with id: {}", id);

        // Emit history updated event
        if let Err(e) = self.app_handle.emit("history-updated", ()) {
            error!("Failed to emit history-updated event: {}", e);
        }

        Ok(())
    }

    fn format_timestamp_title(&self, timestamp: i64) -> String {
        if let Some(utc_datetime) = DateTime::from_timestamp(timestamp, 0) {
            // Convert UTC to local timezone
            let local_datetime = utc_datetime.with_timezone(&Local);
            local_datetime.format("%B %e, %Y - %l:%M%p").to_string()
        } else {
            format!("Recording {}", timestamp)
        }
    }

    pub fn update_history_limit(&self) -> Result<()> {
        self.cleanup_old_entries()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_timestamp_extraction_from_filename() {
        // Test valid filename format
        let file_name = "handy-1234567890.wav";
        let timestamp = file_name
            .strip_prefix("handy-")
            .and_then(|s| s.strip_suffix(".wav"))
            .and_then(|s| s.parse::<i64>().ok());

        assert_eq!(timestamp, Some(1234567890));
    }

    #[test]
    fn test_timestamp_extraction_invalid_filename() {
        // Test invalid filename formats
        let invalid_names = vec![
            "invalid.wav",
            "handy-.wav",
            "handy-abc.wav",
            "handy-123.mp3",
            "recording-123.wav",
        ];

        for file_name in invalid_names {
            let timestamp = file_name
                .strip_prefix("handy-")
                .and_then(|s| s.strip_suffix(".wav"))
                .and_then(|s| s.parse::<i64>().ok());

            assert_eq!(
                timestamp, None,
                "Should return None for invalid filename: {}",
                file_name
            );
        }
    }

    #[test]
    fn test_filename_format_matches_writer() {
        // Verify that the filename format matches what the writer produces
        let unix_ts = 1234567890i64;
        let expected_filename = format!("handy-{}.wav", unix_ts);

        // Extract timestamp back from filename
        let extracted_timestamp = expected_filename
            .strip_prefix("handy-")
            .and_then(|s| s.strip_suffix(".wav"))
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap();

        assert_eq!(extracted_timestamp, unix_ts);
    }
}

