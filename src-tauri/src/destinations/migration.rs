//! Migration from legacy binding configuration to destination entities
//!
//! This module provides automatic migration from the old per-binding output configuration
//! (paste_to_window, save_to_file, output_path) to the new destination entity system.
//!
//! Migration happens transparently when settings are loaded.

use super::storage::DestinationStorage;
use super::types::{Destination, DestinationConfig};
use crate::settings::{get_settings, AppSettings, PasteMethod, ClipboardHandling};
use anyhow::Result;
use log::{debug, info, warn};
use std::collections::HashMap;
use tauri::AppHandle;

/// Check if any bindings need migration and perform it
pub fn migrate_legacy_bindings_if_needed(app: &AppHandle, storage: &DestinationStorage) -> Result<()> {
    let settings = get_settings(app);

    // Check if migration is needed
    let needs_migration = check_if_migration_needed(&settings);

    if !needs_migration {
        debug!("No legacy binding configuration detected, skipping migration");
        return Ok(());
    }

    info!("Detected legacy binding configuration, starting migration to destinations");

    // Perform migration
    migrate_bindings_to_destinations(app, storage, &settings)?;

    info!("Successfully migrated legacy bindings to destination entities");

    Ok(())
}

/// Check if any bindings have legacy configuration that needs migration
fn check_if_migration_needed(settings: &AppSettings) -> bool {
    for (_id, binding) in &settings.bindings {
        // Check if binding has legacy configuration
        if binding.save_to_file || binding.output_path.is_some() {
            return true;
        }
    }
    false
}

/// Migrate all bindings to destination entities
fn migrate_bindings_to_destinations(
    app: &AppHandle,
    storage: &DestinationStorage,
    settings: &AppSettings,
) -> Result<()> {
    // Track which destination IDs we've created
    let mut created_destinations = HashMap::new();

    for (binding_id, binding) in &settings.bindings {
        debug!("Checking binding '{}' for migration", binding_id);

        // Create destinations based on binding configuration
        let dest_ids = create_destinations_for_binding(
            app,
            storage,
            binding_id,
            binding,
            settings,
            &mut created_destinations,
        )?;

        if !dest_ids.is_empty() {
            info!("Migrated binding '{}' -> {} destination(s)", binding_id, dest_ids.len());
        }
    }

    Ok(())
}

/// Create destination entities for a single binding
fn create_destinations_for_binding(
    app: &AppHandle,
    storage: &DestinationStorage,
    binding_id: &str,
    binding: &crate::settings::ShortcutBinding,
    settings: &AppSettings,
    created_destinations: &mut HashMap<String, String>,
) -> Result<Vec<String>> {
    let mut dest_ids = Vec::new();

    // 1. Handle paste_to_window -> ActiveWindow destination
    if binding.paste_to_window {
        let dest_id = format!("migrated_active_window_{}", binding_id);

        if !storage.exists(&dest_id)? {
            let paste_method = match settings.paste_method {
                PasteMethod::CtrlV => "ctrl_v",
                PasteMethod::Direct => "direct",
            };

            let preserve_clipboard = settings.clipboard_handling == ClipboardHandling::DontModify;

            let destination = Destination::new(
                dest_id.clone(),
                format!("Active Window ({})", binding.name),
                DestinationConfig::ActiveWindow {
                    paste_method: paste_method.to_string(),
                    preserve_clipboard,
                },
            );

            storage.create(destination)?;
            created_destinations.insert(dest_id.clone(), "active_window".to_string());
            debug!("Created ActiveWindow destination: {}", dest_id);
        }

        dest_ids.push(dest_id);
    }

    // 2. Handle save_to_file -> FileSystem destination
    if binding.save_to_file {
        let dest_id = format!("migrated_file_{}", binding_id);

        if !storage.exists(&dest_id)? {
            let output_path = binding.output_path.clone().unwrap_or_else(|| {
                // Default to Documents/UltraWhisper
                "~/Documents/UltraWhisper".to_string()
            });

            let destination = Destination::with_template(
                dest_id.clone(),
                format!("File Output ({})", binding.name),
                DestinationConfig::FileSystem {
                    path: output_path,
                    extension: "md".to_string(),
                    filename_pattern: "transcription_{timestamp}.md".to_string(),
                },
                // Use a simple template that preserves existing behavior
                "{transcription_text}".to_string(),
            );

            storage.create(destination)?;
            created_destinations.insert(dest_id.clone(), "file".to_string());
            debug!("Created FileSystem destination: {}", dest_id);
        }

        dest_ids.push(dest_id);
    }

    // If no destinations were created, create a default active_window destination
    // to preserve the existing behavior of pasting by default
    if dest_ids.is_empty() && binding.paste_to_window {
        let dest_id = format!("migrated_active_window_{}", binding_id);

        if !storage.exists(&dest_id)? {
            let paste_method = match settings.paste_method {
                PasteMethod::CtrlV => "ctrl_v",
                PasteMethod::Direct => "direct",
            };

            let preserve_clipboard = settings.clipboard_handling == ClipboardHandling::DontModify;

            let destination = Destination::new(
                dest_id.clone(),
                format!("Active Window ({})", binding.name),
                DestinationConfig::ActiveWindow {
                    paste_method: paste_method.to_string(),
                    preserve_clipboard,
                },
            );

            storage.create(destination)?;
            debug!("Created default ActiveWindow destination: {}", dest_id);
        }

        dest_ids.push(dest_id);
    }

    Ok(dest_ids)
}

/// Clean up legacy binding configuration after successful migration
///
/// This removes the legacy fields from bindings to mark the migration as complete.
/// Note: This is optional and can be called after verifying the migration worked.
pub fn cleanup_legacy_binding_config(app: &AppHandle) -> Result<()> {
    use tauri_plugin_store::StoreExt;

    let store = app.store("store.json")?;
    let mut settings: AppSettings = get_settings(app);

    let mut cleaned_count = 0;

    for (_id, binding) in settings.bindings.iter_mut() {
        if binding.save_to_file || binding.output_path.is_some() {
            // Reset legacy fields to defaults
            binding.save_to_file = false;
            binding.output_path = None;
            cleaned_count += 1;
        }
    }

    if cleaned_count > 0 {
        // Save updated settings
        store.set("settings", serde_json::to_value(&settings)?);
        store.save()?;
        info!("Cleaned up legacy configuration from {} binding(s)", cleaned_count);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::ShortcutBinding;

    #[test]
    fn test_check_if_migration_needed_no_legacy() {
        let settings = AppSettings {
            bindings: {
                let mut map = HashMap::new();
                map.insert(
                    "test".to_string(),
                    ShortcutBinding {
                        id: "test".to_string(),
                        name: "Test".to_string(),
                        description: "Test".to_string(),
                        default_binding: "Ctrl+T".to_string(),
                        current_binding: "Ctrl+T".to_string(),
                        paste_to_window: true,
                        save_to_file: false,
                        output_path: None,
                    },
                );
                map
            },
            ..Default::default()
        };

        assert!(!check_if_migration_needed(&settings));
    }

    #[test]
    fn test_check_if_migration_needed_with_save_to_file() {
        let settings = AppSettings {
            bindings: {
                let mut map = HashMap::new();
                map.insert(
                    "test".to_string(),
                    ShortcutBinding {
                        id: "test".to_string(),
                        name: "Test".to_string(),
                        description: "Test".to_string(),
                        default_binding: "Ctrl+T".to_string(),
                        current_binding: "Ctrl+T".to_string(),
                        paste_to_window: true,
                        save_to_file: true,
                        output_path: None,
                    },
                );
                map
            },
            ..Default::default()
        };

        assert!(check_if_migration_needed(&settings));
    }

    #[test]
    fn test_check_if_migration_needed_with_output_path() {
        let settings = AppSettings {
            bindings: {
                let mut map = HashMap::new();
                map.insert(
                    "test".to_string(),
                    ShortcutBinding {
                        id: "test".to_string(),
                        name: "Test".to_string(),
                        description: "Test".to_string(),
                        default_binding: "Ctrl+T".to_string(),
                        current_binding: "Ctrl+T".to_string(),
                        paste_to_window: true,
                        save_to_file: false,
                        output_path: Some("/custom/path".to_string()),
                    },
                );
                map
            },
            ..Default::default()
        };

        assert!(check_if_migration_needed(&settings));
    }
}
