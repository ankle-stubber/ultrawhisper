//! Active Window destination adapter
//!
//! This destination sends transcribed text to the currently active window
//! by simulating a paste operation (Ctrl+V or direct text input).
//!
//! It supports:
//! - Template application with variable substitution
//! - Multiple paste methods (ctrl_v or direct)
//! - Optional clipboard preservation

use crate::workflow::destinations::{Destination, DestinationContext, Metadata};
use anyhow::Result;
use async_trait::async_trait;
use enigo::{Enigo, Key, Keyboard, Settings};
use std::collections::HashMap;
use tauri::AppHandle;
use tauri_plugin_clipboard_manager::ClipboardExt;

/// Active Window destination - pastes text to the currently focused application
pub struct ActiveWindowDestination {
    /// Template string with {variable} placeholders
    template: String,

    /// Paste method: "ctrl_v" or "direct"
    paste_method: String,

    /// Whether to preserve the original clipboard content
    preserve_clipboard: bool,
}

impl ActiveWindowDestination {
    /// Create a new Active Window destination
    pub fn new(template: String, paste_method: String, preserve_clipboard: bool) -> Self {
        Self {
            template,
            paste_method,
            preserve_clipboard,
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

    /// Format timestamp in ISO-like format (YYYY-MM-DD HH:MM:SS)
    fn format_timestamp(&self, timestamp: u64) -> String {
        use chrono::{DateTime, Utc};

        let datetime = DateTime::<Utc>::from_timestamp(timestamp as i64, 0)
            .unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap());
        datetime.format("%Y-%m-%d %H:%M:%S").to_string()
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

    /// Send a paste command (Cmd+V or Ctrl+V) using platform-specific virtual key codes
    fn send_paste() -> Result<(), String> {
        #[cfg(target_os = "macos")]
        let (modifier_key, v_key_code) = (Key::Meta, Key::Other(9));
        #[cfg(target_os = "windows")]
        let (modifier_key, v_key_code) = (Key::Control, Key::Other(0x56));
        #[cfg(target_os = "linux")]
        let (modifier_key, v_key_code) = (Key::Control, Key::Unicode('v'));

        let mut enigo = Enigo::new(&Settings::default())
            .map_err(|e| format!("Failed to initialize Enigo: {}", e))?;

        enigo
            .key(modifier_key, enigo::Direction::Press)
            .map_err(|e| format!("Failed to press modifier key: {}", e))?;
        enigo
            .key(v_key_code, enigo::Direction::Click)
            .map_err(|e| format!("Failed to click V key: {}", e))?;

        std::thread::sleep(std::time::Duration::from_millis(100));

        enigo
            .key(modifier_key, enigo::Direction::Release)
            .map_err(|e| format!("Failed to release modifier key: {}", e))?;

        Ok(())
    }

    /// Paste text directly using enigo text method
    fn paste_via_direct_input(text: &str) -> Result<(), String> {
        let mut enigo = Enigo::new(&Settings::default())
            .map_err(|e| format!("Failed to initialize Enigo: {}", e))?;

        enigo
            .text(text)
            .map_err(|e| format!("Failed to send text directly: {}", e))?;

        Ok(())
    }

    /// Paste text via clipboard with preservation
    fn paste_via_clipboard(text: &str, app_handle: &AppHandle) -> Result<(), String> {
        let clipboard = app_handle.clipboard();

        // Get current clipboard content
        let clipboard_content = clipboard.read_text().unwrap_or_default();

        // Write new text to clipboard
        clipboard
            .write_text(text)
            .map_err(|e| format!("Failed to write to clipboard: {}", e))?;

        std::thread::sleep(std::time::Duration::from_millis(50));

        // Send paste command
        Self::send_paste()?;

        std::thread::sleep(std::time::Duration::from_millis(50));

        // Restore original clipboard
        clipboard
            .write_text(&clipboard_content)
            .map_err(|e| format!("Failed to restore clipboard: {}", e))?;

        Ok(())
    }
}

#[async_trait]
impl Destination for ActiveWindowDestination {
    async fn send(&self, ctx: &DestinationContext<'_>, text: &str, metadata: &Metadata) -> Result<()> {
        // Apply template to the transcription text
        let formatted_text = self.apply_template(text, metadata);

        // Use the existing clipboard/paste utility, respecting main-thread constraints
        let app = ctx.app.clone();
        let text_owned = formatted_text;
        let app_for_closure = app.clone();
        let paste_method = self.paste_method.clone();
        let preserve_clipboard = self.preserve_clipboard;

        app.run_on_main_thread(move || {
            let result = match paste_method.as_str() {
                "direct" => {
                    // Use direct input method
                    Self::paste_via_direct_input(&text_owned)
                },
                _ => {
                    // Use clipboard method (ctrl_v)
                    if preserve_clipboard {
                        Self::paste_via_clipboard(&text_owned, &app_for_closure)
                    } else {
                        // Just use clipboard without preservation
                        let clipboard = app_for_closure.clipboard();
                        match clipboard.write_text(&text_owned) {
                            Ok(_) => {
                                std::thread::sleep(std::time::Duration::from_millis(50));
                                Self::send_paste()
                            }
                            Err(e) => Err(format!("Failed to write to clipboard: {}", e))
                        }
                    }
                }
            };

            if let Err(e) = result {
                log::error!("Failed to paste to active window: {}", e);
            }
        })
        .map_err(|e| anyhow::anyhow!("Failed to run paste on main thread: {:?}", e))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_active_window_creation() {
        let dest = ActiveWindowDestination::new(
            "{transcription_text}".to_string(),
            "ctrl_v".to_string(),
            true,
        );

        assert_eq!(dest.template, "{transcription_text}");
        assert_eq!(dest.paste_method, "ctrl_v");
        assert!(dest.preserve_clipboard);
    }

    #[test]
    fn test_template_application() {
        let dest = ActiveWindowDestination::new(
            "[{workflow_name}] {transcription_text}".to_string(),
            "ctrl_v".to_string(),
            false,
        );

        let metadata = Metadata {
            workflow_id: "quick_capture".to_string(),
            timestamp: 1698768000, // 2023-10-31 12:00:00 UTC
            duration_ms: 65000, // 1 minute 5 seconds
            model_used: "whisper-small".to_string(),
        };

        let result = dest.apply_template("Hello world", &metadata);

        assert!(result.contains("[quick_capture]"));
        assert!(result.contains("Hello world"));
    }

    #[test]
    fn test_plain_text_template() {
        let dest = ActiveWindowDestination::new(
            "{transcription_text}".to_string(),
            "ctrl_v".to_string(),
            true,
        );

        let metadata = Metadata {
            workflow_id: "test".to_string(),
            timestamp: 0,
            duration_ms: 0,
            model_used: "whisper-small".to_string(),
        };

        let result = dest.apply_template("Test text", &metadata);
        assert_eq!(result, "Test text");
    }

    #[test]
    fn test_duration_formatting() {
        let dest = ActiveWindowDestination::new(
            "{transcription_text}".to_string(),
            "ctrl_v".to_string(),
            true,
        );

        // Test short duration (under 1 minute)
        assert_eq!(dest.format_duration(30_000), "00:30");

        // Test medium duration (minutes and seconds)
        assert_eq!(dest.format_duration(125_000), "02:05");

        // Test long duration (hours, minutes, seconds)
        assert_eq!(dest.format_duration(3_665_000), "01:01:05");
    }
}
