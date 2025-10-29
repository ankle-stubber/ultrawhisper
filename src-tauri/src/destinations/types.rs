//! Destination types - reusable output configurations that workflows reference
//!
//! Destinations are defined once and can be referenced by multiple workflows.
//! Each destination has:
//! - A unique ID
//! - A human-readable name
//! - A type (ActiveWindow, FileSystem, Telegram, etc.)
//! - Type-specific configuration
//! - An optional template with variables

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A destination entity - reusable across multiple workflows
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Destination {
    /// Unique identifier for this destination
    pub id: String,

    /// Human-readable name (e.g., "My Obsidian Vault", "Team Updates")
    pub name: String,

    /// Destination type and configuration
    pub config: DestinationConfig,

    /// Template string with {variable} placeholders
    /// If None, uses a sensible default based on destination type
    pub template: Option<String>,
}

/// Destination configuration - the type and its specific settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DestinationConfig {
    /// Paste to the currently active window
    ActiveWindow {
        /// Method to use for pasting (ctrl_v or direct)
        #[serde(default = "default_paste_method")]
        paste_method: String,

        /// Whether to preserve the original clipboard
        #[serde(default)]
        preserve_clipboard: bool,
    },

    /// Save to filesystem
    FileSystem {
        /// Base path for saving files
        path: String,

        /// File extension (e.g., "md", "txt")
        #[serde(default = "default_file_extension")]
        extension: String,

        /// Filename pattern with variables (e.g., "transcription_{timestamp}.md")
        #[serde(default = "default_filename_pattern")]
        filename_pattern: String,
    },

    /// Send to Telegram
    Telegram {
        /// Reference to credential stored in OS keychain
        credential_id: String,

        /// Telegram chat ID
        chat_id: String,

        /// Whether to include audio file
        #[serde(default)]
        include_audio: bool,
    },
}

// Default functions for serde defaults
fn default_paste_method() -> String {
    "ctrl_v".to_string()
}

fn default_file_extension() -> String {
    "md".to_string()
}

fn default_filename_pattern() -> String {
    "transcription_{timestamp}.md".to_string()
}

impl Destination {
    /// Create a new destination with the given configuration
    pub fn new(id: String, name: String, config: DestinationConfig) -> Self {
        Self {
            id,
            name,
            config,
            template: None,
        }
    }

    /// Create a new destination with a custom template
    pub fn with_template(id: String, name: String, config: DestinationConfig, template: String) -> Self {
        Self {
            id,
            name,
            config,
            template: Some(template),
        }
    }

    /// Get the default template for this destination type
    pub fn default_template(&self) -> &str {
        match &self.config {
            DestinationConfig::ActiveWindow { .. } => "{transcription_text}",
            DestinationConfig::FileSystem { .. } => {
                // Default markdown template for files
                "# Transcription\n\n**Date:** {timestamp}\n**Duration:** {duration}\n**Model:** {model_name}\n\n---\n\n{transcription_text}"
            }
            DestinationConfig::Telegram { .. } => {
                "[{timestamp}] {workflow_name}\n\n{transcription_text}"
            }
        }
    }

    /// Get the template to use (custom or default)
    pub fn get_template(&self) -> &str {
        self.template.as_deref().unwrap_or_else(|| self.default_template())
    }

    /// Apply the template with variable substitution
    pub fn apply_template(&self, variables: &HashMap<&str, String>) -> String {
        let mut content = self.get_template().to_string();

        // Replace each variable in the template
        for (key, value) in variables {
            let placeholder = format!("{{{}}}", key);
            content = content.replace(&placeholder, value);
        }

        content
    }
}

/// Validation errors for destination configuration
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    EmptyId,
    EmptyName,
    InvalidPath(String),
    InvalidChatId(String),
    InvalidCredentialId(String),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::EmptyId => write!(f, "Destination ID cannot be empty"),
            ValidationError::EmptyName => write!(f, "Destination name cannot be empty"),
            ValidationError::InvalidPath(msg) => write!(f, "Invalid path: {}", msg),
            ValidationError::InvalidChatId(msg) => write!(f, "Invalid chat ID: {}", msg),
            ValidationError::InvalidCredentialId(msg) => write!(f, "Invalid credential ID: {}", msg),
        }
    }
}

impl std::error::Error for ValidationError {}

impl Destination {
    /// Validate the destination configuration
    pub fn validate(&self) -> Result<(), ValidationError> {
        // Check ID
        if self.id.trim().is_empty() {
            return Err(ValidationError::EmptyId);
        }

        // Check name
        if self.name.trim().is_empty() {
            return Err(ValidationError::EmptyName);
        }

        // Validate config-specific fields
        match &self.config {
            DestinationConfig::ActiveWindow { .. } => {
                // No additional validation needed
                Ok(())
            }
            DestinationConfig::FileSystem { path, .. } => {
                if path.trim().is_empty() {
                    return Err(ValidationError::InvalidPath("Path cannot be empty".to_string()));
                }
                Ok(())
            }
            DestinationConfig::Telegram { credential_id, chat_id, .. } => {
                if credential_id.trim().is_empty() {
                    return Err(ValidationError::InvalidCredentialId("Credential ID cannot be empty".to_string()));
                }
                if chat_id.trim().is_empty() {
                    return Err(ValidationError::InvalidChatId("Chat ID cannot be empty".to_string()));
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_destination_creation() {
        let dest = Destination::new(
            "active_window_1".to_string(),
            "Active Window".to_string(),
            DestinationConfig::ActiveWindow {
                paste_method: "ctrl_v".to_string(),
                preserve_clipboard: true,
            },
        );

        assert_eq!(dest.id, "active_window_1");
        assert_eq!(dest.name, "Active Window");
        assert!(dest.template.is_none());
    }

    #[test]
    fn test_destination_with_template() {
        let template = "Custom: {transcription_text}";
        let dest = Destination::with_template(
            "file_1".to_string(),
            "My Files".to_string(),
            DestinationConfig::FileSystem {
                path: "~/Documents".to_string(),
                extension: "txt".to_string(),
                filename_pattern: "{timestamp}.txt".to_string(),
            },
            template.to_string(),
        );

        assert_eq!(dest.template, Some(template.to_string()));
        assert_eq!(dest.get_template(), template);
    }

    #[test]
    fn test_default_templates() {
        let active_window = Destination::new(
            "aw".to_string(),
            "AW".to_string(),
            DestinationConfig::ActiveWindow {
                paste_method: "ctrl_v".to_string(),
                preserve_clipboard: false,
            },
        );
        assert_eq!(active_window.get_template(), "{transcription_text}");

        let file = Destination::new(
            "f".to_string(),
            "F".to_string(),
            DestinationConfig::FileSystem {
                path: "/tmp".to_string(),
                extension: "md".to_string(),
                filename_pattern: "test.md".to_string(),
            },
        );
        assert!(file.get_template().contains("{transcription_text}"));
        assert!(file.get_template().contains("{timestamp}"));

        let telegram = Destination::new(
            "t".to_string(),
            "T".to_string(),
            DestinationConfig::Telegram {
                credential_id: "cred".to_string(),
                chat_id: "123".to_string(),
                include_audio: false,
            },
        );
        assert!(telegram.get_template().contains("{workflow_name}"));
    }

    #[test]
    fn test_template_application() {
        let dest = Destination::with_template(
            "test".to_string(),
            "Test".to_string(),
            DestinationConfig::ActiveWindow {
                paste_method: "ctrl_v".to_string(),
                preserve_clipboard: false,
            },
            "Time: {timestamp}, Text: {transcription_text}".to_string(),
        );

        let mut vars = HashMap::new();
        vars.insert("timestamp", "2025-10-29 10:00:00".to_string());
        vars.insert("transcription_text", "Hello world".to_string());

        let result = dest.apply_template(&vars);
        assert_eq!(result, "Time: 2025-10-29 10:00:00, Text: Hello world");
    }

    #[test]
    fn test_validation_valid() {
        let dest = Destination::new(
            "valid".to_string(),
            "Valid Destination".to_string(),
            DestinationConfig::ActiveWindow {
                paste_method: "ctrl_v".to_string(),
                preserve_clipboard: false,
            },
        );

        assert!(dest.validate().is_ok());
    }

    #[test]
    fn test_validation_empty_id() {
        let dest = Destination::new(
            "".to_string(),
            "Valid Name".to_string(),
            DestinationConfig::ActiveWindow {
                paste_method: "ctrl_v".to_string(),
                preserve_clipboard: false,
            },
        );

        assert_eq!(dest.validate(), Err(ValidationError::EmptyId));
    }

    #[test]
    fn test_validation_empty_name() {
        let dest = Destination::new(
            "valid_id".to_string(),
            "".to_string(),
            DestinationConfig::ActiveWindow {
                paste_method: "ctrl_v".to_string(),
                preserve_clipboard: false,
            },
        );

        assert_eq!(dest.validate(), Err(ValidationError::EmptyName));
    }

    #[test]
    fn test_validation_filesystem_empty_path() {
        let dest = Destination::new(
            "valid_id".to_string(),
            "Valid Name".to_string(),
            DestinationConfig::FileSystem {
                path: "".to_string(),
                extension: "md".to_string(),
                filename_pattern: "test.md".to_string(),
            },
        );

        assert!(matches!(dest.validate(), Err(ValidationError::InvalidPath(_))));
    }

    #[test]
    fn test_validation_telegram_empty_credential() {
        let dest = Destination::new(
            "valid_id".to_string(),
            "Valid Name".to_string(),
            DestinationConfig::Telegram {
                credential_id: "".to_string(),
                chat_id: "123".to_string(),
                include_audio: false,
            },
        );

        assert!(matches!(dest.validate(), Err(ValidationError::InvalidCredentialId(_))));
    }

    #[test]
    fn test_serialization() {
        let dest = Destination::with_template(
            "test".to_string(),
            "Test Destination".to_string(),
            DestinationConfig::FileSystem {
                path: "~/Documents".to_string(),
                extension: "md".to_string(),
                filename_pattern: "{timestamp}.md".to_string(),
            },
            "Custom template".to_string(),
        );

        let serialized = serde_json::to_string(&dest).unwrap();
        let deserialized: Destination = serde_json::from_str(&serialized).unwrap();

        assert_eq!(dest, deserialized);
    }
}
