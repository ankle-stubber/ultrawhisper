//! FileSystem destination adapter
//!
//! This destination saves transcribed text to files on disk, with support for:
//! - Template application with variable substitution
//! - Custom output paths
//! - Filename patterns with variables
//! - Automatic directory creation

use crate::workflow::destinations::{Destination, DestinationContext, Metadata};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tauri::Manager;

/// FileSystem destination - saves transcriptions to files
pub struct FileSystemDestination {
    /// Template string with {variable} placeholders
    template: String,

    /// Base output path (supports ~ expansion)
    path: String,

    /// File extension (e.g., "md", "txt")
    extension: String,

    /// Filename pattern with variables (e.g., "transcription_{timestamp}.md")
    filename_pattern: String,
}

impl FileSystemDestination {
    /// Create a new FileSystem destination
    pub fn new(template: String, path: String, extension: String, filename_pattern: String) -> Self {
        Self {
            template,
            path,
            extension,
            filename_pattern,
        }
    }

    /// Apply template with variable substitution
    fn apply_template(&self, text: &str, metadata: &Metadata) -> String {
        let mut variables = HashMap::new();

        // Canonical variables from Bundle 2
        variables.insert("timestamp", self.format_timestamp(metadata.timestamp));
        variables.insert("workflow_name", metadata.workflow_id.clone());
        variables.insert("model_name", metadata.model_used.clone());
        variables.insert("duration", self.format_duration(metadata.duration_ms));
        variables.insert("transcription_text", text.to_string());

        let mut content = self.template.clone();

        // Replace each variable in the template
        for (key, value) in &variables {
            let placeholder = format!("{{{}}}", key);
            content = content.replace(&placeholder, value);
        }

        content
    }

    /// Apply filename pattern with variable substitution
    fn apply_filename_pattern(&self, metadata: &Metadata) -> String {
        let mut variables = HashMap::new();

        // Canonical variables
        variables.insert("timestamp", self.format_timestamp_filename(metadata.timestamp));
        variables.insert("workflow_name", metadata.workflow_id.clone());
        variables.insert("model_name", metadata.model_used.clone());
        variables.insert("duration", self.format_duration(metadata.duration_ms));

        let mut filename = self.filename_pattern.clone();

        // Replace each variable in the filename pattern
        for (key, value) in &variables {
            let placeholder = format!("{{{}}}", key);
            filename = filename.replace(&placeholder, &value);
        }

        filename
    }

    /// Format timestamp in ISO-like format (YYYY-MM-DD HH:MM:SS)
    fn format_timestamp(&self, timestamp: u64) -> String {
        use chrono::{DateTime, Utc};

        let datetime = DateTime::<Utc>::from_timestamp(timestamp as i64, 0)
            .unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap());
        datetime.format("%Y-%m-%d %H:%M:%S").to_string()
    }

    /// Format timestamp for use in filenames (YYYYMMDD_HHMMSS)
    fn format_timestamp_filename(&self, timestamp: u64) -> String {
        use chrono::{DateTime, Utc};

        let datetime = DateTime::<Utc>::from_timestamp(timestamp as i64, 0)
            .unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap());
        datetime.format("%Y%m%d_%H%M%S").to_string()
    }

    /// Format duration in milliseconds as HH:MM:SS or MM:SS
    fn format_duration(&self, duration_ms: u64) -> String {
        let total_seconds = duration_ms / 1000;
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;

        if hours > 0 {
            format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
        } else {
            format!("{:02}:{:02}", minutes, seconds)
        }
    }

    /// Expand ~ to home directory
    fn expand_tilde(&self, path: &str) -> String {
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
}

#[async_trait]
impl Destination for FileSystemDestination {
    async fn send(&self, ctx: &DestinationContext<'_>, text: &str, metadata: &Metadata) -> Result<()> {
        // Apply template to the transcription text
        let formatted_text = self.apply_template(text, metadata);

        // Determine output directory
        let output_dir = if !self.path.is_empty() {
            PathBuf::from(self.expand_tilde(&self.path))
        } else {
            // Fallback to default Documents/UltraWhisper
            ctx.app
                .path()
                .document_dir()
                .context("Failed to get documents directory")?
                .join("UltraWhisper")
        };

        // Create directory if it doesn't exist
        fs::create_dir_all(&output_dir)
            .context(format!("Failed to create output directory: {:?}", output_dir))?;

        // Generate filename from pattern
        let filename = self.apply_filename_pattern(metadata);

        // Build full file path
        let filepath = output_dir.join(&filename);

        // Write the file
        fs::write(&filepath, formatted_text)
            .context(format!("Failed to write file: {:?}", filepath))?;

        log::info!("Saved transcription to: {:?}", filepath);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filesystem_creation() {
        let dest = FileSystemDestination::new(
            "{transcription_text}".to_string(),
            "~/Documents".to_string(),
            "md".to_string(),
            "test_{timestamp}.md".to_string(),
        );

        assert_eq!(dest.template, "{transcription_text}");
        assert_eq!(dest.path, "~/Documents");
        assert_eq!(dest.extension, "md");
        assert_eq!(dest.filename_pattern, "test_{timestamp}.md");
    }

    #[test]
    fn test_template_application() {
        let dest = FileSystemDestination::new(
            "# {workflow_name}\n\n{transcription_text}".to_string(),
            "~/Documents".to_string(),
            "md".to_string(),
            "{timestamp}.md".to_string(),
        );

        let metadata = Metadata {
            workflow_id: "quick_capture".to_string(),
            timestamp: 1698768000,
            duration_ms: 65000,
            model_used: "whisper-small".to_string(),
        };

        let result = dest.apply_template("Hello world", &metadata);

        assert!(result.contains("# quick_capture"));
        assert!(result.contains("Hello world"));
    }

    #[test]
    fn test_filename_pattern_application() {
        let dest = FileSystemDestination::new(
            "{transcription_text}".to_string(),
            "~/Documents".to_string(),
            "md".to_string(),
            "{workflow_name}_{timestamp}.md".to_string(),
        );

        let metadata = Metadata {
            workflow_id: "quick_capture".to_string(),
            timestamp: 1698768000,
            duration_ms: 65000,
            model_used: "whisper-small".to_string(),
        };

        let filename = dest.apply_filename_pattern(&metadata);

        assert!(filename.contains("quick_capture"));
        assert!(filename.ends_with(".md"));
    }

    #[test]
    fn test_tilde_expansion() {
        let dest = FileSystemDestination::new(
            "{transcription_text}".to_string(),
            "~/Documents".to_string(),
            "md".to_string(),
            "test.md".to_string(),
        );

        let expanded = dest.expand_tilde("~/Documents/test");

        // Should expand on Unix-like systems
        #[cfg(not(windows))]
        {
            if std::env::var("HOME").is_ok() {
                assert!(!expanded.starts_with("~"));
            }
        }
    }

    #[test]
    fn test_duration_formatting() {
        let dest = FileSystemDestination::new(
            "{transcription_text}".to_string(),
            "~/Documents".to_string(),
            "md".to_string(),
            "test.md".to_string(),
        );

        // Test various durations
        assert_eq!(dest.format_duration(30_000), "00:30");
        assert_eq!(dest.format_duration(125_000), "02:05");
        assert_eq!(dest.format_duration(3_665_000), "01:01:05");
    }

    #[test]
    fn test_timestamp_filename_format() {
        let dest = FileSystemDestination::new(
            "{transcription_text}".to_string(),
            "~/Documents".to_string(),
            "md".to_string(),
            "test.md".to_string(),
        );

        let timestamp = 1698768000; // 2023-10-31 12:00:00 UTC
        let formatted = dest.format_timestamp_filename(timestamp);

        // Should be in YYYYMMDD_HHMMSS format
        assert!(formatted.contains("_"));
        assert_eq!(formatted.len(), 15); // YYYYMMDD_HHMMSS = 15 chars
    }
}
