//! Workflow execution engine - orchestrates the entire pipeline

use super::destinations::{DestinationContext, DestinationResult, DestinationRouter, Metadata};
use super::mapper::binding_to_workflow;
use super::types::{DestinationConfig, Workflow};
use crate::managers::history::HistoryManager;
use crate::model_pool::ModelPool;
use crate::router::clipboard::ClipboardDestination;
use crate::router::file::FileDestination;
use crate::settings::get_settings;
use anyhow::{anyhow, Result};
use log::{debug, error, info};
use std::env;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

/// Result summary for a workflow execution
pub struct ExecutionResult {
    pub workflow_id: String,
    pub text: String,
}

/// Workflow engine - orchestrates transcription through the workflow pipeline
pub struct WorkflowEngine {
    model_pool: Arc<ModelPool>,
}

impl WorkflowEngine {
    /// Create a new workflow engine
    pub fn new(model_pool: Arc<ModelPool>) -> Self {
        Self { model_pool }
    }

    /// Get the workflow configuration for a given binding ID
    ///
    /// This method loads the current settings and binding configuration,
    /// then maps them to a Workflow struct using the compatibility mapper.
    pub fn get_workflow_for_binding(
        &self,
        app: &AppHandle,
        binding_id: &str,
    ) -> Result<Workflow> {
        debug!("Loading workflow for binding: {}", binding_id);

        let settings = get_settings(app);

        // Find the binding configuration
        let binding = settings
            .bindings
            .get(binding_id)
            .ok_or_else(|| anyhow!("Binding '{}' not found in settings", binding_id))?;

        // Map the binding to a workflow
        let workflow = binding_to_workflow(binding, &settings);

        debug!(
            "Workflow loaded: {} with {} destination(s)",
            workflow.name,
            workflow.destinations.len()
        );

        Ok(workflow)
    }

    /// Build a destination router from a workflow's destination configs
    ///
    /// This method instantiates the appropriate destination implementations
    /// (Clipboard, File, etc.) based on the workflow configuration.
    pub fn build_router(&self, workflow: &Workflow) -> Result<DestinationRouter> {
        let mut router = DestinationRouter::new();

        for dest_config in &workflow.destinations {
            match dest_config {
                DestinationConfig::Clipboard { paste_immediately } => {
                    debug!("Adding clipboard destination (paste: {})", paste_immediately);
                    router.add_destination(Box::new(ClipboardDestination::new(*paste_immediately)));
                }
                DestinationConfig::File { path, .. } => {
                    debug!("Adding file destination (path: {:?})", path);
                    // Convert PathBuf to String for FileDestination
                    // Expand tilde if present (matching batch semantics; support HOME and USERPROFILE)
                    let path_str = path.to_string_lossy();
                    let expanded_path = expand_tilde_str(&path_str);
                    router.add_destination(Box::new(FileDestination::new(Some(expanded_path))));
                }
                DestinationConfig::Telegram { .. } | DestinationConfig::Webhook { .. } => {
                    // Phase 1: External destinations not yet implemented
                    debug!("Skipping external destination (Phase 4 feature)");
                }
            }
        }

        Ok(router)
    }

    /// Expand a leading '~' in a path string using HOME or USERPROFILE
    /// This is a simple, cross-platform expansion consistent with batch semantics.
    fn _expand_tilde_for_tests(path: &str) -> String {
        expand_tilde_str(path)
    }

    /// Build metadata for a transcription
    ///
    /// This creates the metadata struct that will be passed to destinations,
    /// containing information about the workflow and transcription.
    pub fn build_metadata(&self, workflow: &Workflow) -> Metadata {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Metadata {
            workflow_id: workflow.id.clone(),
            timestamp,
            duration_ms: 0, // Phase 1: Not tracking duration yet
            model_used: workflow.model_config.model_id.clone(),
        }
    }

    /// Execute a binding/workflow with the given audio samples
    ///
    /// This is the main entry point for workflow execution. It:
    /// 1. Loads the workflow configuration
    /// 2. Ensures the model is loaded
    /// 3. Transcribes the audio
    /// 4. Saves to history
    /// 5. Routes outputs to destinations
    pub async fn execute_binding(
        &self,
        app: &AppHandle,
        binding_id: &str,
        samples: Vec<f32>,
    ) -> Result<ExecutionResult> {
        info!("Executing workflow for binding: {}", binding_id);

        // 1. Load workflow configuration
        let workflow = self.get_workflow_for_binding(app, binding_id)?;
        debug!("Workflow loaded: {}", workflow.name);

        // 2. Ensure model is loaded
        debug!("Loading model: {}", workflow.model_config.model_id);
        let model_handle = self
            .model_pool
            .get_or_load(&workflow.model_config.model_id)
            .await
            .map_err(|e| anyhow!("Failed to load model: {}", e))?;

        // 3. Transcribe audio
        debug!("Transcribing {} audio samples", samples.len());
        let text = model_handle
            .transcribe(samples.clone())
            .await
            .map_err(|e| anyhow!("Transcription failed: {}", e))?;

        info!("Transcription complete: {} characters", text.len());

        // 4. Match legacy parity: if transcription is empty, do not save history or route outputs
        if text.trim().is_empty() {
            debug!("Empty transcription; skipping history save and destination routing");
        } else {
            // Save to history with workflow tracking
            let hm = app.state::<Arc<HistoryManager>>();
            hm.save_transcription(
                samples,
                text.clone(),
                Some(&workflow.id),
                Some(&workflow.name),
            )
            .await
            .map_err(|e| {
                error!("Failed to save to history: {}", e);
                anyhow!("History save failed: {}", e)
            })?;

            debug!("Saved to history with workflow_id: {}", workflow.id);

            // 5. Route to destinations
            let router = self.build_router(&workflow)?;
            let ctx = DestinationContext {
                app,
                output_base: workflow.audio_processing.save_path.clone(),
                audio_path: None, // Phase 1: Not providing audio path to destinations
            };
            let metadata = self.build_metadata(&workflow);

            let results = router.route(&ctx, &text, &metadata).await?;

            // Log destination results (but don't fail the overall operation)
            for (idx, result) in results.iter().enumerate() {
                match result {
                    DestinationResult::Success => {
                        debug!("Destination {} succeeded", idx + 1);
                    }
                    DestinationResult::Failed(err) => {
                        error!("Destination {} failed: {}", idx + 1, err);
                    }
                }
            }
        }

        info!("Workflow execution complete for: {}", workflow.id);

        Ok(ExecutionResult {
            workflow_id: workflow.id,
            text,
        })
    }
}

/// Expand a leading '~' using HOME or USERPROFILE.
fn expand_tilde_str(path: &str) -> String {
    if let Some(stripped) = path.strip_prefix('~') {
        // Try HOME, then USERPROFILE
        if let Ok(home) = env::var("HOME").or_else(|_| env::var("USERPROFILE")) {
            // Preserve the remainder (which may start with '/' or '\\')
            let mut result = home;
            result.push_str(stripped);
            result
        } else {
            path.to_string()
        }
    } else {
        path.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_result() {
        let result = ExecutionResult {
            workflow_id: "test".to_string(),
            text: "test transcription".to_string(),
        };

        assert_eq!(result.workflow_id, "test");
        assert_eq!(result.text, "test transcription");
    }

    #[test]
    fn test_expand_tilde_uses_home() {
        // Save and set env
        let original_home = env::var("HOME").ok();
        let _ = env::remove_var("USERPROFILE");
        env::set_var("HOME", "/home/testuser");

        let expanded = WorkflowEngine::_expand_tilde_for_tests("~/Documents/UltraWhisper");
        assert_eq!(expanded, "/home/testuser/Documents/UltraWhisper");

        // Restore
        if let Some(h) = original_home { env::set_var("HOME", h); } else { env::remove_var("HOME"); }
    }

    #[test]
    fn test_expand_tilde_uses_userprofile_when_home_missing() {
        // Save and adjust env
        let original_home = env::var("HOME").ok();
        let original_up = env::var("USERPROFILE").ok();
        env::remove_var("HOME");
        env::set_var("USERPROFILE", "C:/Users/TestUser");

        let expanded = WorkflowEngine::_expand_tilde_for_tests("~\\Documents\\UltraWhisper");
        assert_eq!(expanded, "C:/Users/TestUser\\Documents\\UltraWhisper");

        // Restore
        if let Some(h) = original_home { env::set_var("HOME", h); } else { env::remove_var("HOME"); }
        if let Some(u) = original_up { env::set_var("USERPROFILE", u); } else { env::remove_var("USERPROFILE"); }
    }
}
