//! Telegram destination adapter
//!
//! This destination sends transcribed text to a Telegram chat via the Bot API.
//!
//! It supports:
//! - Template application with variable substitution
//! - Automatic message truncation (~4000 chars, Telegram limit is 4096)
//! - Basic retry logic for failed sends
//! - Secure credential storage via OS keychain

use crate::workflow::destinations::{Destination, DestinationContext, Metadata};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Maximum message length for Telegram (using 4000 to leave room for template overhead)
const MAX_MESSAGE_LENGTH: usize = 4000;

/// Maximum retry attempts for failed sends
const MAX_RETRIES: u32 = 3;

/// Delay between retries in milliseconds
const RETRY_DELAY_MS: u64 = 1000;

/// Telegram destination - sends transcriptions to Telegram chat
pub struct TelegramDestination {
    /// Template string with {variable} placeholders
    template: String,

    /// Telegram bot token (retrieved from keychain)
    bot_token: String,

    /// Telegram chat ID
    chat_id: String,
}

/// Telegram API response
#[derive(Debug, Deserialize, Serialize)]
struct TelegramResponse {
    ok: bool,
    #[serde(default)]
    description: Option<String>,
}

impl TelegramDestination {
    /// Create a new Telegram destination
    pub fn new(template: String, bot_token: String, chat_id: String) -> Self {
        Self {
            template,
            bot_token,
            chat_id,
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

    /// Truncate message to maximum length with ellipsis
    fn truncate_message(&self, text: &str) -> String {
        if text.len() <= MAX_MESSAGE_LENGTH {
            text.to_string()
        } else {
            let truncated = &text[..MAX_MESSAGE_LENGTH - 3];
            format!("{}...", truncated)
        }
    }

    /// Send message to Telegram with retry logic
    async fn send_message_with_retry(&self, message: &str) -> Result<()> {
        let mut last_error = None;

        for attempt in 1..=MAX_RETRIES {
            match self.send_message_once(message).await {
                Ok(()) => {
                    if attempt > 1 {
                        log::info!("Telegram send succeeded on attempt {}/{}", attempt, MAX_RETRIES);
                    }
                    return Ok(());
                }
                Err(e) => {
                    log::warn!(
                        "Telegram send attempt {}/{} failed: {}",
                        attempt,
                        MAX_RETRIES,
                        e
                    );
                    last_error = Some(e);

                    if attempt < MAX_RETRIES {
                        tokio::time::sleep(tokio::time::Duration::from_millis(RETRY_DELAY_MS)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Failed to send to Telegram after {} attempts", MAX_RETRIES)))
    }

    /// Send message to Telegram once (no retry)
    async fn send_message_once(&self, message: &str) -> Result<()> {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.bot_token);

        let mut payload = HashMap::new();
        payload.insert("chat_id", self.chat_id.as_str());
        payload.insert("text", message);

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .context("Failed to send HTTP request to Telegram API")?;

        let status = response.status();
        let response_text = response.text().await.context("Failed to read response body")?;

        // Try to parse as JSON
        if let Ok(telegram_response) = serde_json::from_str::<TelegramResponse>(&response_text) {
            if telegram_response.ok {
                log::debug!("Telegram message sent successfully");
                return Ok(());
            } else {
                let error_msg = telegram_response
                    .description
                    .unwrap_or_else(|| "Unknown error".to_string());
                return Err(anyhow::anyhow!("Telegram API error: {}", error_msg));
            }
        }

        // If we can't parse as JSON, check HTTP status
        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "Telegram API returned error status {}: {}",
                status,
                response_text
            ));
        }

        Ok(())
    }
}

#[async_trait]
impl Destination for TelegramDestination {
    async fn send(&self, _ctx: &DestinationContext<'_>, text: &str, metadata: &Metadata) -> Result<()> {
        // Apply template to the transcription text
        let formatted_text = self.apply_template(text, metadata);

        // Truncate if necessary
        let message = self.truncate_message(&formatted_text);

        // Log truncation if it occurred
        if message.len() < formatted_text.len() {
            log::warn!(
                "Telegram message truncated from {} to {} characters",
                formatted_text.len(),
                message.len()
            );
        }

        // Send with retry logic
        self.send_message_with_retry(&message).await?;

        log::info!("Successfully sent transcription to Telegram chat {}", self.chat_id);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telegram_creation() {
        let dest = TelegramDestination::new(
            "[{workflow_name}] {transcription_text}".to_string(),
            "test_token".to_string(),
            "123456".to_string(),
        );

        assert_eq!(dest.template, "[{workflow_name}] {transcription_text}");
        assert_eq!(dest.bot_token, "test_token");
        assert_eq!(dest.chat_id, "123456");
    }

    #[test]
    fn test_template_application() {
        let dest = TelegramDestination::new(
            "[{timestamp}] {workflow_name}\n\n{transcription_text}".to_string(),
            "test_token".to_string(),
            "123456".to_string(),
        );

        let metadata = Metadata {
            workflow_id: "quick_capture".to_string(),
            timestamp: 1698768000,
            duration_ms: 65000,
            model_used: "whisper-small".to_string(),
        };

        let result = dest.apply_template("Hello world", &metadata);

        assert!(result.contains("quick_capture"));
        assert!(result.contains("Hello world"));
        assert!(result.contains("2023-10-31"));
    }

    #[test]
    fn test_message_truncation() {
        let dest = TelegramDestination::new(
            "{transcription_text}".to_string(),
            "test_token".to_string(),
            "123456".to_string(),
        );

        // Test short message (no truncation)
        let short = "Short message";
        assert_eq!(dest.truncate_message(short), short);

        // Test long message (truncation)
        let long = "a".repeat(5000);
        let truncated = dest.truncate_message(&long);
        assert_eq!(truncated.len(), MAX_MESSAGE_LENGTH);
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn test_duration_formatting() {
        let dest = TelegramDestination::new(
            "{transcription_text}".to_string(),
            "test_token".to_string(),
            "123456".to_string(),
        );

        // Test various durations
        assert_eq!(dest.format_duration(30_000), "00:30");
        assert_eq!(dest.format_duration(125_000), "02:05");
        assert_eq!(dest.format_duration(3_665_000), "01:01:05");
    }

    #[test]
    fn test_timestamp_formatting() {
        let dest = TelegramDestination::new(
            "{transcription_text}".to_string(),
            "test_token".to_string(),
            "123456".to_string(),
        );

        let timestamp = 1698768000; // 2023-10-31 12:00:00 UTC
        let formatted = dest.format_timestamp(timestamp);

        assert!(formatted.contains("2023-10-31"));
        assert!(formatted.contains("12:00:00"));
    }
}
