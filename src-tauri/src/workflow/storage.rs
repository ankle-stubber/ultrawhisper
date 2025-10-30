//! Workflow storage - CRUD operations for workflow entities
//!
//! Workflows are stored using Tauri's plugin-store in JSON format.
//! This module provides thread-safe CRUD operations and validation.

use super::types::{TriggerConfig, Workflow, AudioInputConfig, ModelConfig, ModelManagement,
                    UnloadStrategy, AudioProcessingConfig};
use crate::destinations::DestinationStorage;
use crate::settings::get_settings;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_store::StoreExt;

const WORKFLOWS_STORE_KEY: &str = "workflows";

/// Simplified workflow for storage and UI (MVP subset)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredWorkflow {
    /// Unique identifier (UUID)
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Whether this workflow is enabled
    pub enabled: bool,

    /// Trigger configuration (reuse from workflow::types)
    pub trigger: TriggerConfig,

    /// Model configuration (simplified)
    pub model: ModelConfigDto,

    /// Destination IDs to route output to
    pub destination_ids: Vec<String>,

    /// Optional notes/description
    pub notes: Option<String>,
}

/// Simplified model configuration for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfigDto {
    pub model_id: String,
    pub language: String,
    pub translate_to_english: bool,
}

impl StoredWorkflow {
    /// Convert StoredWorkflow to full Workflow for execution
    pub fn to_full_workflow(&self) -> Workflow {
        Workflow {
            id: self.id.clone(),
            name: self.name.clone(),
            description: self.notes.clone().unwrap_or_default(),
            enabled: self.enabled,
            trigger: self.trigger.clone(),
            audio_input: AudioInputConfig::default(),
            model_config: ModelConfig {
                model_id: self.model.model_id.clone(),
                language: Some(self.model.language.clone()),
                translate_to_english: self.model.translate_to_english,
            },
            model_management: ModelManagement {
                preload_on_startup: false,
                unload_strategy: UnloadStrategy::Never,
            },
            streaming_enabled: false, // MVP: use non-streaming path
            audio_processing: AudioProcessingConfig {
                save_original: false,
                save_path: None,
                compress: None,
                delete_after_processing: false,
            },
            destination_ids: self.destination_ids.clone(),
        }
    }
}

/// Workflow storage manager
#[derive(Clone)]
pub struct WorkflowStorage {
    app: AppHandle,
    /// In-memory cache for fast access
    cache: Arc<RwLock<HashMap<String, StoredWorkflow>>>,
}

impl WorkflowStorage {
    /// Create a new workflow storage instance
    pub fn new(app: AppHandle) -> Result<Self> {
        let storage = Self {
            app: app.clone(),
            cache: Arc::new(RwLock::new(HashMap::new())),
        };

        // Load workflows from disk into cache
        storage.reload()?;

        Ok(storage)
    }

    /// Reload workflows from disk into cache
    pub fn reload(&self) -> Result<()> {
        let workflows = self.load_from_disk()?;

        let mut cache = self.cache.write()
            .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;

        cache.clear();
        for workflow in workflows {
            cache.insert(workflow.id.clone(), workflow);
        }

        Ok(())
    }

    /// Load workflows from Tauri store
    fn load_from_disk(&self) -> Result<Vec<StoredWorkflow>> {
        let store = self.app.store("store.json")
            .context("Failed to get store")?;

        let value = store.get(WORKFLOWS_STORE_KEY);

        if let Some(v) = value {
            let workflows: Vec<StoredWorkflow> = serde_json::from_value(v)
                .context("Failed to deserialize workflows")?;
            Ok(workflows)
        } else {
            Ok(Vec::new())
        }
    }

    /// Save workflows to Tauri store
    fn save_to_disk(&self, workflows: &[StoredWorkflow]) -> Result<()> {
        let store = self.app.store("store.json")
            .context("Failed to get store")?;

        store.set(WORKFLOWS_STORE_KEY, serde_json::to_value(workflows)?);
        store.save().context("Failed to save store")?;

        Ok(())
    }

    /// Emit workflows-changed event
    fn emit_changed_event(&self) -> Result<()> {
        self.app.emit("workflows-changed", ())
            .context("Failed to emit workflows-changed event")?;
        Ok(())
    }

    /// Get all workflows
    pub fn list(&self) -> Result<Vec<StoredWorkflow>> {
        let cache = self.cache.read()
            .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {}", e))?;

        Ok(cache.values().cloned().collect())
    }

    /// Get a workflow by ID
    pub fn get(&self, id: &str) -> Result<Option<StoredWorkflow>> {
        let cache = self.cache.read()
            .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {}", e))?;

        Ok(cache.get(id).cloned())
    }

    /// Create a new workflow
    pub fn create(&self, workflow: StoredWorkflow) -> Result<()> {
        // Get destination storage for validation
        let dest_storage = self.app.state::<DestinationStorage>();

        // Validate the workflow
        validate_workflow(&workflow, &*dest_storage)
            .context("Workflow validation failed")?;

        // Check if ID already exists
        {
            let cache = self.cache.read()
                .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {}", e))?;

            if cache.contains_key(&workflow.id) {
                anyhow::bail!("Workflow with ID '{}' already exists", workflow.id);
            }
        }

        // Add to cache
        {
            let mut cache = self.cache.write()
                .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;

            cache.insert(workflow.id.clone(), workflow);
        }

        // Persist to disk
        let all_workflows = self.list()?;
        self.save_to_disk(&all_workflows)?;

        // Emit event
        self.emit_changed_event()?;

        Ok(())
    }

    /// Update an existing workflow
    pub fn update(&self, workflow: StoredWorkflow) -> Result<()> {
        // Get destination storage for validation
        let dest_storage = self.app.state::<DestinationStorage>();

        // Validate the workflow
        validate_workflow(&workflow, &*dest_storage)
            .context("Workflow validation failed")?;

        // Check if workflow exists
        {
            let cache = self.cache.read()
                .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {}", e))?;

            if !cache.contains_key(&workflow.id) {
                anyhow::bail!("Workflow with ID '{}' not found", workflow.id);
            }
        }

        // Update in cache
        {
            let mut cache = self.cache.write()
                .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;

            cache.insert(workflow.id.clone(), workflow);
        }

        // Persist to disk
        let all_workflows = self.list()?;
        self.save_to_disk(&all_workflows)?;

        // Emit event
        self.emit_changed_event()?;

        Ok(())
    }

    /// Delete a workflow by ID
    pub fn delete(&self, id: &str) -> Result<()> {
        // Remove from cache
        {
            let mut cache = self.cache.write()
                .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;

            if cache.remove(id).is_none() {
                anyhow::bail!("Workflow with ID '{}' not found", id);
            }
        }

        // Persist to disk
        let all_workflows = self.list()?;
        self.save_to_disk(&all_workflows)?;

        // Emit event
        self.emit_changed_event()?;

        Ok(())
    }

    /// Check if a workflow exists
    pub fn exists(&self, id: &str) -> Result<bool> {
        let cache = self.cache.read()
            .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {}", e))?;

        Ok(cache.contains_key(id))
    }
}

/// Validate a workflow before storage
fn validate_workflow(workflow: &StoredWorkflow, dest_storage: &DestinationStorage) -> Result<()> {
    // Check name
    if workflow.name.trim().is_empty() {
        anyhow::bail!("Workflow name cannot be empty");
    }

    // Validate trigger-specific fields
    match &workflow.trigger {
        TriggerConfig::Hotkey { binding, .. } => {
            let normalized = normalize_binding(binding);
            if normalized.is_empty() {
                anyhow::bail!("Hotkey binding cannot be empty");
            }
        }
        TriggerConfig::FolderWatch {
            paths,
            file_patterns,
            interval_seconds,
            stability_timeout_seconds
        } => {
            if paths.is_empty() {
                anyhow::bail!("Folder Watch requires at least one path");
            }

            // Validate each path
            for path in paths {
                let path_str = path.to_string_lossy();
                let normalized = normalize_path(&path_str)?;
                if normalized.is_empty() {
                    anyhow::bail!("Folder Watch paths cannot be empty");
                }
            }

            if file_patterns.is_empty() {
                anyhow::bail!("Folder Watch requires at least one file pattern");
            }

            // Validate patterns start with *.
            for pattern in file_patterns {
                if !pattern.starts_with("*.") {
                    anyhow::bail!("File patterns must start with '*.' (e.g., '*.wav')");
                }
            }

            if *interval_seconds < 10 {
                anyhow::bail!("Check interval must be at least 10 seconds");
            }

            if *stability_timeout_seconds < 1 {
                anyhow::bail!("Stability timeout must be at least 1 second");
            }
        }
        TriggerConfig::Schedule { .. } | TriggerConfig::Api { .. } => {
            anyhow::bail!("Schedule and API triggers are not supported in MVP");
        }
    }

    // Validate model
    if workflow.model.model_id.trim().is_empty() {
        anyhow::bail!("Model ID cannot be empty");
    }

    // Validate destinations
    if workflow.destination_ids.is_empty() {
        anyhow::bail!("At least one destination is required");
    }

    for dest_id in &workflow.destination_ids {
        if !dest_storage.exists(dest_id)? {
            anyhow::bail!("Destination '{}' does not exist", dest_id);
        }
    }

    Ok(())
}

/// Normalize a hotkey binding (lowercase modifiers, trim whitespace)
fn normalize_binding(binding: &str) -> String {
    binding
        .split('+')
        .map(|part| part.trim().to_lowercase())
        .collect::<Vec<_>>()
        .join("+")
}

/// Normalize a file path (expand ~, canonicalize best-effort)
fn normalize_path(path: &str) -> Result<String> {
    use std::env;

    // Expand tilde
    let expanded = if path.starts_with('~') {
        if let Ok(home) = env::var("HOME").or_else(|_| env::var("USERPROFILE")) {
            path.replacen('~', &home, 1)
        } else {
            path.to_string()
        }
    } else {
        path.to_string()
    };

    // Try to canonicalize (resolves symlinks, removes .., etc.)
    match std::path::Path::new(&expanded).canonicalize() {
        Ok(canonical) => Ok(canonical.to_string_lossy().to_string()),
        Err(_) => {
            // If canonicalize fails (path doesn't exist yet), use expanded path
            Ok(expanded)
        }
    }
}

/// Seed a legacy batch workflow if needed (non-destructive migration)
pub fn seed_legacy_batch_workflow_if_needed(
    app: &AppHandle,
    workflow_storage: &WorkflowStorage,
) -> Result<()> {
    // Check if workflows already exist
    let existing_workflows = workflow_storage.list()?;
    if !existing_workflows.is_empty() {
        log::debug!("Workflows already exist, skipping legacy batch seeding");
        return Ok(());
    }

    let settings = get_settings(app);
    let batch = &settings.batch_transcription;

    // Check if batch settings are enabled or non-empty
    if !batch.enabled && batch.watch_folders.is_empty() {
        log::debug!("Batch transcription not configured, skipping legacy workflow seeding");
        return Ok(());
    }

    log::info!("Seeding legacy batch workflow from existing batch settings");

    // Create legacy workflow
    let legacy_workflow = StoredWorkflow {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Batch Processing (legacy)".to_string(),
        enabled: batch.enabled,
        trigger: TriggerConfig::FolderWatch {
            paths: batch.watch_folders.iter().map(PathBuf::from).collect(),
            interval_seconds: batch.check_interval_seconds as u32,
            file_patterns: batch.file_patterns.clone(),
            stability_timeout_seconds: batch.stability_timeout_seconds as u32,
        },
        model: ModelConfigDto {
            model_id: settings.selected_model.clone(),
            language: settings.selected_language.clone(),
            translate_to_english: settings.translate_to_english,
        },
        destination_ids: vec!["file_default".to_string()],
        notes: Some("Auto-migrated from legacy batch transcription settings".to_string()),
    };

    workflow_storage.create(legacy_workflow)?;
    log::info!("Legacy batch workflow seeded successfully");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_binding() {
        assert_eq!(normalize_binding("Ctrl+Shift+A"), "ctrl+shift+a");
        assert_eq!(normalize_binding("  cmd + space  "), "cmd+space");
        assert_eq!(normalize_binding("ALT+F4"), "alt+f4");
    }

    #[test]
    fn test_normalize_path() {
        // Basic path (no tilde)
        let result = normalize_path("/tmp/test").unwrap();
        assert!(!result.is_empty());

        // Path with tilde (may or may not expand depending on environment)
        let result = normalize_path("~/Documents").unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_stored_workflow_to_full_workflow() {
        let stored = StoredWorkflow {
            id: "test-id".to_string(),
            name: "Test Workflow".to_string(),
            enabled: true,
            trigger: TriggerConfig::Hotkey {
                binding: "ctrl+space".to_string(),
                push_to_talk: false,
            },
            model: ModelConfigDto {
                model_id: "whisper-small".to_string(),
                language: "auto".to_string(),
                translate_to_english: false,
            },
            destination_ids: vec!["active_window_default".to_string()],
            notes: Some("Test notes".to_string()),
        };

        let full = stored.to_full_workflow();

        assert_eq!(full.id, "test-id");
        assert_eq!(full.name, "Test Workflow");
        assert_eq!(full.enabled, true);
        assert_eq!(full.model_config.model_id, "whisper-small");
        assert_eq!(full.destination_ids, vec!["active_window_default".to_string()]);
    }

    // Note: Tests for WorkflowStorage require a Tauri AppHandle
    // which is difficult to create in unit tests. These should be
    // tested in integration tests with a real Tauri application.
}
