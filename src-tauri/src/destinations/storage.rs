//! Destination storage - CRUD operations for destination entities
//!
//! Destinations are stored using Tauri's plugin-store in JSON format.
//! This module provides thread-safe CRUD operations.

use super::types::{Destination, DestinationConfig};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

const DESTINATIONS_STORE_KEY: &str = "destinations";

/// Destination storage manager
#[derive(Clone)]
pub struct DestinationStorage {
    app: AppHandle,
    /// In-memory cache for fast access
    cache: Arc<RwLock<HashMap<String, Destination>>>,
}

impl DestinationStorage {
    /// Create a new destination storage instance
    pub fn new(app: AppHandle) -> Result<Self> {
        let storage = Self {
            app: app.clone(),
            cache: Arc::new(RwLock::new(HashMap::new())),
        };

        // Load destinations from disk into cache
        storage.reload()?;

        Ok(storage)
    }

    /// Reload destinations from disk into cache
    pub fn reload(&self) -> Result<()> {
        let destinations = self.load_from_disk()?;

        let mut cache = self.cache.write()
            .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;

        cache.clear();
        for dest in destinations {
            cache.insert(dest.id.clone(), dest);
        }

        Ok(())
    }

    /// Load destinations from Tauri store
    fn load_from_disk(&self) -> Result<Vec<Destination>> {
        let store = self.app.store("store.json")
            .context("Failed to get store")?;

        let value = store.get(DESTINATIONS_STORE_KEY);

        if let Some(v) = value {
            let destinations: Vec<Destination> = serde_json::from_value(v)
                .context("Failed to deserialize destinations")?;
            Ok(destinations)
        } else {
            Ok(Vec::new())
        }
    }

    /// Save destinations to Tauri store
    fn save_to_disk(&self, destinations: &[Destination]) -> Result<()> {
        let store = self.app.store("store.json")
            .context("Failed to get store")?;

        store.set(DESTINATIONS_STORE_KEY, serde_json::to_value(destinations)?);
        store.save().context("Failed to save store")?;

        Ok(())
    }

    /// Get all destinations
    pub fn list(&self) -> Result<Vec<Destination>> {
        let cache = self.cache.read()
            .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {}", e))?;

        Ok(cache.values().cloned().collect())
    }

    /// Get a destination by ID
    pub fn get(&self, id: &str) -> Result<Option<Destination>> {
        let cache = self.cache.read()
            .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {}", e))?;

        Ok(cache.get(id).cloned())
    }

    /// Create a new destination
    pub fn create(&self, destination: Destination) -> Result<()> {
        // Validate the destination
        destination.validate()
            .context("Destination validation failed")?;

        // Check if ID already exists
        {
            let cache = self.cache.read()
                .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {}", e))?;

            if cache.contains_key(&destination.id) {
                anyhow::bail!("Destination with ID '{}' already exists", destination.id);
            }
        }

        // Add to cache
        {
            let mut cache = self.cache.write()
                .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;

            cache.insert(destination.id.clone(), destination);
        }

        // Persist to disk
        let all_destinations = self.list()?;
        self.save_to_disk(&all_destinations)?;

        Ok(())
    }

    /// Update an existing destination
    pub fn update(&self, destination: Destination) -> Result<()> {
        // Validate the destination
        destination.validate()
            .context("Destination validation failed")?;

        // Check if destination exists
        {
            let cache = self.cache.read()
                .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {}", e))?;

            if !cache.contains_key(&destination.id) {
                anyhow::bail!("Destination with ID '{}' not found", destination.id);
            }
        }

        // Update in cache
        {
            let mut cache = self.cache.write()
                .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;

            cache.insert(destination.id.clone(), destination);
        }

        // Persist to disk
        let all_destinations = self.list()?;
        self.save_to_disk(&all_destinations)?;

        Ok(())
    }

    /// Delete a destination by ID
    pub fn delete(&self, id: &str) -> Result<()> {
        // Remove from cache
        {
            let mut cache = self.cache.write()
                .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;

            if cache.remove(id).is_none() {
                anyhow::bail!("Destination with ID '{}' not found", id);
            }
        }

        // Persist to disk
        let all_destinations = self.list()?;
        self.save_to_disk(&all_destinations)?;

        Ok(())
    }

    /// Check if a destination exists
    pub fn exists(&self, id: &str) -> Result<bool> {
        let cache = self.cache.read()
            .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {}", e))?;

        Ok(cache.contains_key(id))
    }
}

/// Create default seed destinations
pub fn create_default_destinations() -> Vec<Destination> {
    vec![
        // Default active window destination
        Destination::new(
            "active_window_default".to_string(),
            "Active Window".to_string(),
            DestinationConfig::ActiveWindow {
                paste_method: "ctrl_v".to_string(),
                preserve_clipboard: true,
            },
        ),

        // Default file destination
        Destination::with_template(
            "file_default".to_string(),
            "Documents Folder".to_string(),
            DestinationConfig::FileSystem {
                path: "~/Documents/UltraWhisper".to_string(),
                extension: "md".to_string(),
                filename_pattern: "transcription_{timestamp}.md".to_string(),
            },
            "# Transcription\n\n**Date:** {timestamp}\n**Duration:** {duration}\n**Model:** {model_name}\n**Workflow:** {workflow_name}\n\n---\n\n{transcription_text}".to_string(),
        ),

        // Obsidian-style file destination
        Destination::with_template(
            "obsidian_vault".to_string(),
            "Obsidian Vault".to_string(),
            DestinationConfig::FileSystem {
                path: "~/Documents/Obsidian/Inbox".to_string(),
                extension: "md".to_string(),
                filename_pattern: "{timestamp}_transcription.md".to_string(),
            },
            "---\ntags: [transcription, audio]\ncreated: {timestamp}\nworkflow: {workflow_name}\nmodel: {model_name}\nduration: {duration}\n---\n\n{transcription_text}".to_string(),
        ),
    ]
}

/// Initialize default destinations if none exist
pub fn seed_defaults_if_empty(storage: &DestinationStorage) -> Result<()> {
    let existing = storage.list()?;

    if existing.is_empty() {
        log::info!("No destinations found, seeding defaults");

        for dest in create_default_destinations() {
            storage.create(dest)?;
        }

        log::info!("Default destinations created successfully");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_default_destinations() {
        let defaults = create_default_destinations();

        assert!(!defaults.is_empty());
        assert!(defaults.len() >= 2);

        // Verify all defaults are valid
        for dest in defaults {
            assert!(dest.validate().is_ok());
        }
    }

    #[test]
    fn test_default_active_window() {
        let defaults = create_default_destinations();
        let active_window = defaults.iter()
            .find(|d| d.id == "active_window_default")
            .expect("Should have active window default");

        assert_eq!(active_window.name, "Active Window");
        assert!(matches!(active_window.config, DestinationConfig::ActiveWindow { .. }));
    }

    #[test]
    fn test_default_file() {
        let defaults = create_default_destinations();
        let file = defaults.iter()
            .find(|d| d.id == "file_default")
            .expect("Should have file default");

        assert_eq!(file.name, "Documents Folder");
        assert!(matches!(file.config, DestinationConfig::FileSystem { .. }));
        assert!(file.template.is_some());
    }

    #[test]
    fn test_default_obsidian() {
        let defaults = create_default_destinations();
        let obsidian = defaults.iter()
            .find(|d| d.id == "obsidian_vault")
            .expect("Should have Obsidian default");

        assert_eq!(obsidian.name, "Obsidian Vault");
        assert!(matches!(obsidian.config, DestinationConfig::FileSystem { .. }));

        // Check template has frontmatter
        let template = obsidian.get_template();
        assert!(template.contains("---"));
        assert!(template.contains("tags:"));
    }

    // Note: Tests for DestinationStorage require a Tauri AppHandle
    // which is difficult to create in unit tests. These should be
    // tested in integration tests with a real Tauri application.
}
