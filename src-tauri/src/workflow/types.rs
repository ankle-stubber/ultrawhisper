//! Core workflow types - defines the data model only, no implementation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// Phase 0: Allow dead code as these types are scaffolding for future phases
#[allow(dead_code)]
/// A workflow represents a complete transcription pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub trigger: TriggerConfig,
    pub audio_input: AudioInputConfig,
    pub model_config: ModelConfig,
    pub model_management: ModelManagement,
    pub streaming_enabled: bool,
    pub audio_processing: AudioProcessingConfig,
    pub destinations: Vec<DestinationConfig>,
}

/// Trigger configuration - what initiates this workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TriggerConfig {
    Hotkey {
        binding: String,
        push_to_talk: bool,
    },
    FolderWatch {
        paths: Vec<PathBuf>,
        interval_seconds: u32,
        file_patterns: Vec<String>,
        stability_timeout_seconds: u32,
    },
    Schedule {
        cron: String,
        timezone: String,
    },
    Api {
        endpoint: String,
    },
}

/// Audio input device configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioInputConfig {
    pub device_id: Option<String>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u16>,
    pub vad_enabled: bool,
    pub vad_threshold: f32,
}

impl Default for AudioInputConfig {
    fn default() -> Self {
        Self {
            device_id: None,
            sample_rate: None,
            channels: None,
            vad_enabled: true,
            vad_threshold: 0.5,
        }
    }
}

/// Model configuration for transcription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub model_id: String,
    pub language: Option<String>,
    pub translate_to_english: bool,
}

/// Model lifecycle management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelManagement {
    pub preload_on_startup: bool,
    pub unload_strategy: UnloadStrategy,
}

/// Unload strategy using serialization-friendly types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnloadStrategy {
    Never,
    Immediately,
    /// Delay in seconds before unloading
    AfterDelaySecs(u64),
}

/// Audio processing and storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioProcessingConfig {
    pub save_original: bool,
    pub save_path: Option<PathBuf>,
    pub compress: Option<CompressionConfig>,
    pub delete_after_processing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    pub format: AudioFormat,
    pub quality: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AudioFormat {
    Wav,
    Opus,
    Flac,
}

/// Output destination configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DestinationConfig {
    Clipboard {
        paste_immediately: bool,
    },
    File {
        path: PathBuf,
        template: String,
        naming_pattern: String,
    },
    Telegram {
        credential_id: String,
        chat_id: String,
        include_audio: bool,
    },
    Webhook {
        url: String,
        credential_id: Option<String>,
        headers: HashMap<String, String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_serialization() {
        let workflow = Workflow {
            id: "test".to_string(),
            name: "Test Workflow".to_string(),
            description: "Test description".to_string(),
            enabled: true,
            trigger: TriggerConfig::Hotkey {
                binding: "ctrl+space".to_string(),
                push_to_talk: true,
            },
            audio_input: AudioInputConfig::default(),
            model_config: ModelConfig {
                model_id: "whisper-small".to_string(),
                language: Some("en".to_string()),
                translate_to_english: false,
            },
            model_management: ModelManagement {
                preload_on_startup: false,
                unload_strategy: UnloadStrategy::AfterDelaySecs(300),
            },
            streaming_enabled: false,
            audio_processing: AudioProcessingConfig {
                save_original: false,
                save_path: None,
                compress: None,
                delete_after_processing: false,
            },
            destinations: vec![DestinationConfig::Clipboard {
                paste_immediately: true,
            }],
        };

        // Test serialization round-trip
        let serialized = serde_json::to_string(&workflow).unwrap();
        let deserialized: Workflow = serde_json::from_str(&serialized).unwrap();

        assert_eq!(workflow.id, deserialized.id);
        assert_eq!(workflow.name, deserialized.name);
        assert_eq!(workflow.enabled, deserialized.enabled);
    }

    #[test]
    fn test_default_audio_config() {
        let config = AudioInputConfig::default();
        assert_eq!(config.vad_enabled, true);
        assert_eq!(config.vad_threshold, 0.5);
        assert_eq!(config.device_id, None);
    }

    #[test]
    fn test_all_trigger_types() {
        // Test Hotkey
        let hotkey = TriggerConfig::Hotkey {
            binding: "ctrl+alt+t".to_string(),
            push_to_talk: false,
        };
        let serialized = serde_json::to_string(&hotkey).unwrap();
        let _deserialized: TriggerConfig = serde_json::from_str(&serialized).unwrap();

        // Test FolderWatch
        let folder_watch = TriggerConfig::FolderWatch {
            paths: vec![PathBuf::from("/test/path")],
            interval_seconds: 60,
            file_patterns: vec!["*.wav".to_string()],
            stability_timeout_seconds: 30,
        };
        let serialized = serde_json::to_string(&folder_watch).unwrap();
        let _deserialized: TriggerConfig = serde_json::from_str(&serialized).unwrap();

        // Test Schedule
        let schedule = TriggerConfig::Schedule {
            cron: "0 9 * * 1-5".to_string(),
            timezone: "America/Los_Angeles".to_string(),
        };
        let serialized = serde_json::to_string(&schedule).unwrap();
        let _deserialized: TriggerConfig = serde_json::from_str(&serialized).unwrap();

        // Test Api
        let api = TriggerConfig::Api {
            endpoint: "/transcribe".to_string(),
        };
        let serialized = serde_json::to_string(&api).unwrap();
        let _deserialized: TriggerConfig = serde_json::from_str(&serialized).unwrap();
    }

    #[test]
    fn test_all_destination_types() {
        // Test Clipboard
        let clipboard = DestinationConfig::Clipboard {
            paste_immediately: true,
        };
        let serialized = serde_json::to_string(&clipboard).unwrap();
        let _deserialized: DestinationConfig = serde_json::from_str(&serialized).unwrap();

        // Test File
        let file = DestinationConfig::File {
            path: PathBuf::from("/output/path"),
            template: "## Transcription\n{text}".to_string(),
            naming_pattern: "transcription_{timestamp}.md".to_string(),
        };
        let serialized = serde_json::to_string(&file).unwrap();
        let _deserialized: DestinationConfig = serde_json::from_str(&serialized).unwrap();

        // Test Telegram
        let telegram = DestinationConfig::Telegram {
            credential_id: "telegram_bot_1".to_string(),
            chat_id: "123456789".to_string(),
            include_audio: false,
        };
        let serialized = serde_json::to_string(&telegram).unwrap();
        let _deserialized: DestinationConfig = serde_json::from_str(&serialized).unwrap();

        // Test Webhook
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer token".to_string());
        let webhook = DestinationConfig::Webhook {
            url: "https://example.com/webhook".to_string(),
            credential_id: Some("webhook_cred".to_string()),
            headers,
        };
        let serialized = serde_json::to_string(&webhook).unwrap();
        let _deserialized: DestinationConfig = serde_json::from_str(&serialized).unwrap();
    }

    #[test]
    fn test_unload_strategy_serialization() {
        // Test Never
        let never = UnloadStrategy::Never;
        let serialized = serde_json::to_string(&never).unwrap();
        let _deserialized: UnloadStrategy = serde_json::from_str(&serialized).unwrap();

        // Test Immediately
        let immediately = UnloadStrategy::Immediately;
        let serialized = serde_json::to_string(&immediately).unwrap();
        let _deserialized: UnloadStrategy = serde_json::from_str(&serialized).unwrap();

        // Test AfterDelaySecs
        let delay = UnloadStrategy::AfterDelaySecs(300);
        let serialized = serde_json::to_string(&delay).unwrap();
        let deserialized: UnloadStrategy = serde_json::from_str(&serialized).unwrap();
        match deserialized {
            UnloadStrategy::AfterDelaySecs(secs) => assert_eq!(secs, 300),
            _ => panic!("Expected AfterDelaySecs variant"),
        }
    }

    #[test]
    fn test_audio_format_variants() {
        let formats = vec![AudioFormat::Wav, AudioFormat::Opus, AudioFormat::Flac];
        for format in formats {
            let serialized = serde_json::to_string(&format).unwrap();
            let _deserialized: AudioFormat = serde_json::from_str(&serialized).unwrap();
        }
    }
}
