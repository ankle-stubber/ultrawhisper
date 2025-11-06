//! Legacy binding to destination migration utilities (deprecated)
//!
//! This module is deprecated as bindings have been removed from the system.
//! It remains as a stub for compatibility but performs no operations.

use anyhow::Result;
use crate::destinations::DestinationStorage;
use crate::settings::AppSettings;
use tauri::AppHandle;

/// Check if any bindings need migration and perform it (deprecated - no-op)
pub fn migrate_legacy_bindings_if_needed(_app: &AppHandle, _storage: &DestinationStorage) -> Result<()> {
    // Legacy bindings have been removed, nothing to migrate
    Ok(())
}

/// Check if any bindings have legacy configuration that needs migration (deprecated - always false)
fn check_if_migration_needed(_settings: &AppSettings) -> bool {
    // Legacy bindings have been removed, no migration needed
    false
}

/// Migrate legacy binding configurations to destination entities (deprecated - no-op)
fn migrate_bindings_to_destinations(
    _app: &AppHandle,
    _storage: &DestinationStorage,
    _settings: &AppSettings,
) -> Result<()> {
    // Legacy bindings have been removed, nothing to migrate
    Ok(())
}

/// Clean up legacy binding configuration after successful migration (deprecated - no-op)
pub fn cleanup_legacy_binding_config(_app: &AppHandle) -> Result<()> {
    // Legacy bindings have been removed, nothing to clean up
    Ok(())
}
