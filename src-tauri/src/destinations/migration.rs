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
use tauri::{AppHandle, Manager};

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
        // Determine the desired output path and normalize it to an absolute, comparable form
        let raw_path = binding.output_path.clone().unwrap_or_else(|| {
            // Default to Documents/UltraWhisper
            "~/Documents/UltraWhisper".to_string()
        });

        let normalized = normalize_path_for_compare(app, &raw_path);

        // First, attempt to find an existing FileSystem destination with the same normalized path
        if let Some(existing_id) = find_filesystem_destination_by_normalized_path(app, storage, &normalized) {
            debug!("Reusing existing FileSystem destination '{}' for path '{}'", existing_id, normalized);
            dest_ids.push(existing_id);
        } else {
            // Create a shared, per-path destination with a stable ID
            let stable_id = format!("file_path_{}", stable_hash8(&normalized));

            if !storage.exists(&stable_id)? {
                let destination = Destination::new(
                    stable_id.clone(),
                    format!("File Output ({})", binding.name),
                    DestinationConfig::FileSystem {
                        path: normalized.clone(),
                        extension: "md".to_string(),
                        filename_pattern: "transcription_{timestamp}.md".to_string(),
                    },
                );

                storage.create(destination)?;
                created_destinations.insert(stable_id.clone(), "file".to_string());
                debug!("Created shared FileSystem destination '{}' for path '{}'", stable_id, normalized);
            }

            dest_ids.push(stable_id);
        }
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

/// Normalize a user-provided path into an absolute, comparable string.
/// Rules:
/// - Expand '~' to HOME/USERPROFILE
/// - If path starts with 'Documents' (or 'Documents/...'), prefix with system Documents directory
/// - Remove any trailing separators
fn normalize_path_for_compare(app: &AppHandle, path: &str) -> String {
    use std::path::{Component, Path, PathBuf};

    // Helper to expand '~'
    fn expand_tilde(p: &str) -> String {
        if p.starts_with("~/") || p == "~" {
            if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
                return p.replacen("~", &home, 1);
            }
        }
        p.to_string()
    }

    // Expand leading '~'
    let mut p = expand_tilde(path);

    // If path begins with 'Documents' or 'Documents/...', resolve to the system documents dir
    let lower = p.to_lowercase();
    if lower == "documents" || lower.starts_with("documents/") || lower.starts_with("documents\\") {
        if let Ok(mut docs) = app.path().document_dir() {
            // Append the remainder after 'Documents'
            let remainder = p.trim_start_matches("Documents/")
                             .trim_start_matches("Documents\\");
            if !remainder.is_empty() {
                docs = docs.join(remainder);
            }
            p = docs.to_string_lossy().to_string();
        }
    }

    // Convert to Path and normalize simple components
    let mut buf = PathBuf::new();
    for comp in Path::new(&p).components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => { buf.pop(); }
            other => buf.push(other.as_os_str()),
        }
    }

    // Remove trailing separators by converting back to string
    let mut s = buf.to_string_lossy().to_string();
    while s.ends_with('/') || s.ends_with('\\') { s.pop(); }
    s
}

/// Find a FileSystem destination whose path matches the given normalized absolute path
fn find_filesystem_destination_by_normalized_path(
    app: &AppHandle,
    storage: &DestinationStorage,
    normalized_path: &str,
) -> Option<String> {
    if let Ok(list) = storage.list() {
        for dest in list {
            if let DestinationConfig::FileSystem { ref path, .. } = dest.config {
                let existing_norm = normalize_path_for_compare(app, path);
                if existing_norm == normalized_path {
                    return Some(dest.id);
                }
            }
        }
    }
    None
}

/// Simple stable 64-bit FNV-1a hash, returned as the lower 8 hex chars
fn stable_hash8(s: &str) -> String {
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x00000100000001B3;
    let mut hash = FNV_OFFSET_BASIS;
    for b in s.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    format!("{:08x}", (hash & 0xFFFF_FFFF) as u32)
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
        let mut settings = crate::settings::get_default_settings();
        settings.bindings = {
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
        };

        assert!(!check_if_migration_needed(&settings));
    }

    #[test]
    fn test_check_if_migration_needed_with_save_to_file() {
        let mut settings = crate::settings::get_default_settings();
        settings.bindings = {
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
        };

        assert!(check_if_migration_needed(&settings));
    }

    #[test]
    fn test_check_if_migration_needed_with_output_path() {
        let mut settings = crate::settings::get_default_settings();
        settings.bindings = {
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
        };

        assert!(check_if_migration_needed(&settings));
    }
}
