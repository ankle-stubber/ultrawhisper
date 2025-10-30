//! Binding-to-workflow mapper - converts legacy bindings to workflow configs

use super::types::*;
use crate::destinations::DestinationStorage;
use crate::settings::{AppSettings, ModelUnloadTimeout, ShortcutBinding};
use std::path::PathBuf;

/// Maps a ShortcutBinding and AppSettings to a Workflow
///
/// NOTE (Bundle 2): This function is being refactored to use destination entities.
/// For now, it uses default destination IDs. Bundle 3 will implement full migration
/// from legacy binding configuration to destination references.
///
/// This function provides compatibility between the legacy binding system
/// and the new workflow architecture by converting settings on-the-fly.
pub fn binding_to_workflow(binding: &ShortcutBinding, settings: &AppSettings) -> Workflow {
    // Bundle 2: Simplified destination mapping using default destination IDs
    // Map legacy binding flags to destination IDs
    let mut destination_ids = Vec::new();
    if binding.paste_to_window {
        destination_ids.push("active_window_default".to_string());
    }
    if binding.save_to_file {
        destination_ids.push("file_default".to_string());
    }
    // Fallback: if neither set, default to active window to preserve UX
    if destination_ids.is_empty() {
        destination_ids.push("active_window_default".to_string());
    }

    // Map model unload timeout to unload strategy
    let unload_strategy = match settings.model_unload_timeout {
        ModelUnloadTimeout::Never => UnloadStrategy::Never,
        ModelUnloadTimeout::Immediately => UnloadStrategy::Immediately,
        timeout => {
            // Convert to seconds, defaulting to 300 (5 minutes) if None
            let seconds = timeout.to_seconds().unwrap_or(300);
            UnloadStrategy::AfterDelaySecs(seconds)
        }
    };

    Workflow {
        id: binding.id.clone(),
        name: binding.name.clone(),
        description: binding.description.clone(),
        enabled: true,
        trigger: TriggerConfig::Hotkey {
            binding: binding.current_binding.clone(),
            push_to_talk: settings.push_to_talk,
        },
        audio_input: AudioInputConfig {
            device_id: settings.selected_microphone.clone(),
            sample_rate: None,  // Use default
            channels: None,     // Use default
            vad_enabled: true,  // VAD is always enabled in current impl
            vad_threshold: 0.5, // Default threshold
        },
        model_config: ModelConfig {
            model_id: settings.selected_model.clone(),
            language: Some(settings.selected_language.clone()),
            translate_to_english: settings.translate_to_english,
        },
        model_management: ModelManagement {
            preload_on_startup: false, // Not supported in Phase 1
            unload_strategy,
        },
        streaming_enabled: false, // Phase 2 feature
        audio_processing: AudioProcessingConfig {
            save_original: true,  // History manager saves audio
            save_path: Some(PathBuf::from("~/UltraWhisper/recordings")),
            compress: None,       // Phase 3 feature
            delete_after_processing: false,
        },
        // Destination references (Bundle 2)
        destination_ids,
    }
}

/// Storage-aware mapper (Bundle 3+): prefers migrated destination IDs when present in storage
///
/// This function mirrors `binding_to_workflow` but, for each legacy flag, it first checks
/// for a migrated per-binding destination (e.g., `migrated_active_window_{binding_id}` or
/// `migrated_file_{binding_id}`) in `DestinationStorage`. If found, it uses that ID; otherwise
/// it falls back to the default shared destination IDs.
pub fn binding_to_workflow_with_storage(
    binding: &ShortcutBinding,
    settings: &AppSettings,
    storage: &DestinationStorage,
) -> Workflow {
    // Resolve destination IDs with a preference for migrated per-binding IDs when available
    let mut destination_ids = Vec::new();

    if binding.paste_to_window {
        let migrated_id = format!("migrated_active_window_{}", binding.id);
        let use_migrated = storage.exists(&migrated_id).unwrap_or(false);
        destination_ids.push(if use_migrated {
            migrated_id
        } else {
            "active_window_default".to_string()
        });
    }

    if binding.save_to_file {
        let migrated_id = format!("migrated_file_{}", binding.id);
        let use_migrated = storage.exists(&migrated_id).unwrap_or(false);
        if use_migrated {
            destination_ids.push(migrated_id);
        } else {
            // Try to find a shared per-path FileSystem destination matching this binding's output_path
            let desired = binding
                .output_path
                .as_ref()
                .map(|p| normalize_path_portable(p));

            if let Some(desired_norm) = desired {
                if let Ok(list) = storage.list() {
                    if let Some(found) = list.into_iter().find(|d| match &d.config {
                        crate::destinations::DestinationConfig::FileSystem { path, .. } => {
                            normalize_path_portable(path) == desired_norm
                        }
                        _ => false,
                    }) {
                        destination_ids.push(found.id);
                    } else {
                        // Fall back to the default shared file destination
                        destination_ids.push("file_default".to_string());
                    }
                } else {
                    destination_ids.push("file_default".to_string());
                }
            } else {
                destination_ids.push("file_default".to_string());
            }
        }
    }

    // Fallback: if neither flag set, default to active window to preserve UX
    if destination_ids.is_empty() {
        destination_ids.push("active_window_default".to_string());
    }

    // Map model unload timeout to unload strategy
    let unload_strategy = match settings.model_unload_timeout {
        ModelUnloadTimeout::Never => UnloadStrategy::Never,
        ModelUnloadTimeout::Immediately => UnloadStrategy::Immediately,
        timeout => {
            // Convert to seconds, defaulting to 300 (5 minutes) if None
            let seconds = timeout.to_seconds().unwrap_or(300);
            UnloadStrategy::AfterDelaySecs(seconds)
        }
    };

    Workflow {
        id: binding.id.clone(),
        name: binding.name.clone(),
        description: binding.description.clone(),
        enabled: true,
        trigger: TriggerConfig::Hotkey {
            binding: binding.current_binding.clone(),
            push_to_talk: settings.push_to_talk,
        },
        audio_input: AudioInputConfig {
            device_id: settings.selected_microphone.clone(),
            sample_rate: None,  // Use default
            channels: None,     // Use default
            vad_enabled: true,  // VAD is always enabled in current impl
            vad_threshold: 0.5, // Default threshold
        },
        model_config: ModelConfig {
            model_id: settings.selected_model.clone(),
            language: Some(settings.selected_language.clone()),
            translate_to_english: settings.translate_to_english,
        },
        model_management: ModelManagement {
            preload_on_startup: false, // Not supported in Phase 1
            unload_strategy,
        },
        streaming_enabled: false, // Phase 2 feature
        audio_processing: AudioProcessingConfig {
            save_original: true,  // History manager saves audio
            save_path: Some(PathBuf::from("~/UltraWhisper/recordings")),
            compress: None,       // Phase 3 feature
            delete_after_processing: false,
        },
        destination_ids,
    }
}

/// Normalize a user-provided path into an absolute, comparable string using environment only
fn normalize_path_portable(path: &str) -> String {
    use std::path::{Component, Path, PathBuf};

    fn expand_tilde(p: &str) -> String {
        if p.starts_with("~/") || p == "~" {
            if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
                return p.replacen("~", &home, 1);
            }
        }
        p.to_string()
    }

    fn docs_dir_fallback() -> Option<String> {
        if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
            let mut pb = PathBuf::from(home);
            pb.push("Documents");
            return Some(pb.to_string_lossy().to_string());
        }
        None
    }

    let mut p = expand_tilde(path);
    let lower = p.to_lowercase();
    if lower == "documents" || lower.starts_with("documents/") || lower.starts_with("documents\\") {
        if let Some(mut docs) = docs_dir_fallback() {
            let remainder = p.trim_start_matches("Documents/")
                             .trim_start_matches("Documents\\");
            if !remainder.is_empty() {
                if !(docs.ends_with('/') || docs.ends_with('\\')) { docs.push(std::path::MAIN_SEPARATOR); }
                docs.push_str(remainder);
            }
            p = docs;
        }
    }

    let mut buf = PathBuf::new();
    for comp in Path::new(&p).components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => { buf.pop(); }
            other => buf.push(other.as_os_str()),
        }
    }

    let mut s = buf.to_string_lossy().to_string();
    while s.ends_with('/') || s.ends_with('\\') { s.pop(); }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_binding(
        id: &str,
        paste: bool,
        save: bool,
        output_path: Option<String>,
    ) -> ShortcutBinding {
        ShortcutBinding {
            id: id.to_string(),
            name: format!("Test {}", id),
            description: "Test binding".to_string(),
            default_binding: "ctrl+alt+t".to_string(),
            current_binding: "ctrl+alt+t".to_string(),
            paste_to_window: paste,
            save_to_file: save,
            output_path,
        }
    }

    fn create_test_settings() -> AppSettings {
        AppSettings {
            bindings: HashMap::new(),
            push_to_talk: true,
            audio_feedback: true,
            audio_feedback_volume: 0.5,
            sound_theme: crate::settings::SoundTheme::Marimba,
            start_hidden: false,
            autostart_enabled: false,
            selected_model: "whisper-small".to_string(),
            always_on_microphone: false,
            selected_microphone: Some("default".to_string()),
            selected_output_device: None,
            translate_to_english: false,
            selected_language: "auto".to_string(),
            overlay_position: crate::settings::OverlayPosition::Bottom,
            debug_mode: false,
            custom_words: Vec::new(),
            model_unload_timeout: ModelUnloadTimeout::Min5,
            word_correction_threshold: 0.18,
            history_limit: 5,
            paste_method: crate::settings::PasteMethod::CtrlV,
            clipboard_handling: crate::settings::ClipboardHandling::DontModify,
            batch_transcription: crate::settings::BatchTranscriptionSettings::default(),
            use_workflow_engine: false,
            streaming: crate::settings::StreamingSettings::default(),
        }
    }

    #[test]
    fn test_binding_to_workflow_paste_only() {
        let binding = create_test_binding("transcribe", true, false, None);
        let settings = create_test_settings();

        let workflow = binding_to_workflow(&binding, &settings);

        assert_eq!(workflow.id, "transcribe");
        assert_eq!(workflow.name, "Test transcribe");
        assert_eq!(workflow.destination_ids, vec!["active_window_default".to_string()]);
    }

    #[test]
    fn test_binding_to_workflow_file_only() {
        let binding = create_test_binding("save", false, true, None);
        let settings = create_test_settings();

        let workflow = binding_to_workflow(&binding, &settings);

        assert!(workflow.destination_ids.contains(&"file_default".to_string()));
        assert_eq!(workflow.destination_ids.len(), 1);
    }

    #[test]
    fn test_binding_to_workflow_both_destinations() {
        let binding = create_test_binding("both", true, true, Some("/custom/path".to_string()));
        let settings = create_test_settings();

        let workflow = binding_to_workflow(&binding, &settings);

        assert!(workflow.destination_ids.contains(&"active_window_default".to_string()));
        assert!(workflow.destination_ids.contains(&"file_default".to_string()));
        assert_eq!(workflow.destination_ids.len(), 2);
    }

    #[test]
    fn test_model_config_mapping() {
        let binding = create_test_binding("test", true, false, None);
        let mut settings = create_test_settings();
        settings.selected_model = "whisper-large".to_string();
        settings.selected_language = "en".to_string();
        settings.translate_to_english = true;

        let workflow = binding_to_workflow(&binding, &settings);

        assert_eq!(workflow.model_config.model_id, "whisper-large");
        assert_eq!(workflow.model_config.language, Some("en".to_string()));
        assert!(workflow.model_config.translate_to_english);
    }

    #[test]
    fn test_unload_strategy_mapping() {
        let binding = create_test_binding("test", true, false, None);

        // Test Never
        let mut settings = create_test_settings();
        settings.model_unload_timeout = ModelUnloadTimeout::Never;
        let workflow = binding_to_workflow(&binding, &settings);
        assert!(matches!(workflow.model_management.unload_strategy, UnloadStrategy::Never));

        // Test Immediately
        settings.model_unload_timeout = ModelUnloadTimeout::Immediately;
        let workflow = binding_to_workflow(&binding, &settings);
        assert!(matches!(workflow.model_management.unload_strategy, UnloadStrategy::Immediately));

        // Test Min5 (300 seconds)
        settings.model_unload_timeout = ModelUnloadTimeout::Min5;
        let workflow = binding_to_workflow(&binding, &settings);
        assert!(matches!(workflow.model_management.unload_strategy, UnloadStrategy::AfterDelaySecs(300)));

        // Test Hour1 (3600 seconds)
        settings.model_unload_timeout = ModelUnloadTimeout::Hour1;
        let workflow = binding_to_workflow(&binding, &settings);
        assert!(matches!(workflow.model_management.unload_strategy, UnloadStrategy::AfterDelaySecs(3600)));
    }

    #[test]
    fn test_trigger_config_mapping() {
        let binding = create_test_binding("test", true, false, None);
        let mut settings = create_test_settings();
        settings.push_to_talk = false;

        let workflow = binding_to_workflow(&binding, &settings);

        match workflow.trigger {
            TriggerConfig::Hotkey { binding, push_to_talk } => {
                assert_eq!(binding, "ctrl+alt+t");
                assert!(!push_to_talk);
            }
            _ => panic!("Expected Hotkey trigger"),
        }
    }

    #[test]
    fn test_audio_input_config() {
        let binding = create_test_binding("test", true, false, None);
        let mut settings = create_test_settings();
        settings.selected_microphone = Some("custom-mic".to_string());

        let workflow = binding_to_workflow(&binding, &settings);

        assert_eq!(workflow.audio_input.device_id, Some("custom-mic".to_string()));
        assert!(workflow.audio_input.vad_enabled);
        assert_eq!(workflow.audio_input.vad_threshold, 0.5);
    }
}
