//! Workflow execution engine - orchestrates the entire pipeline

use super::destinations::{DestinationContext, DestinationResult, DestinationRouter, Metadata};
use super::storage::WorkflowStorage;
use super::types::Workflow;
use crate::managers::history::HistoryManager;
use crate::model_pool::ModelPool;
use crate::destinations::{ActiveWindowDestination, FileSystemDestination, TelegramDestination, DestinationConfig, DestinationStorage};
use crate::settings::get_settings;
use crate::streaming::chunker::AudioChunk;
use crate::streaming::session::StreamingSession;
use anyhow::{anyhow, Result};
use log::{debug, error, info, warn};
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
    pub fn build_router(&self, app: &AppHandle, workflow: &Workflow) -> Result<DestinationRouter> {
        let mut router = DestinationRouter::new();

        // Look up destinations by ID from DestinationStorage and instantiate implementations
        let storage_state = app.state::<DestinationStorage>();
        for dest_id in &workflow.destination_ids {
            match storage_state.get(dest_id) {
                Ok(Some(dest)) => {
                    let template = dest.get_template().to_string();

                    match dest.config {
                        DestinationConfig::ActiveWindow { ref paste_method, preserve_clipboard } => {
                            // Bundle 3: Use ActiveWindowDestination with template support
                            let adapter = ActiveWindowDestination::new(
                                template,
                                paste_method.clone(),
                                preserve_clipboard,
                            );
                            router.add_destination(Box::new(adapter));
                            debug!("Added ActiveWindow destination: {} (paste_method: {}, preserve_clipboard: {})",
                                   dest_id, paste_method, preserve_clipboard);
                        }
                        DestinationConfig::FileSystem { ref path, ref extension, ref filename_pattern } => {
                            // Bundle 3: Use FileSystemDestination with template support
                            let adapter = FileSystemDestination::new(
                                template,
                                path.clone(),
                                extension.clone(),
                                filename_pattern.clone(),
                            );
                            router.add_destination(Box::new(adapter));
                            debug!("Added FileSystem destination: {} -> {} (pattern: {})",
                                   dest_id, path, filename_pattern);
                        }
                        DestinationConfig::Telegram { ref credential_id, ref chat_id, .. } => {
                            // Bundle 4: Retrieve bot token from keychain and instantiate TelegramDestination
                            match crate::commands::telegram::get_telegram_credentials(credential_id.clone()) {
                                Ok(credentials) => {
                                    let adapter = TelegramDestination::new(
                                        template,
                                        credentials.bot_token,
                                        chat_id.clone(),
                                    );
                                    router.add_destination(Box::new(adapter));
                                    info!("Added Telegram destination: {} (chat_id: {})", dest_id, chat_id);
                                }
                                Err(e) => {
                                    warn!(
                                        "Skipping Telegram destination '{}': failed to retrieve credentials for '{}': {}",
                                        dest_id, credential_id, e
                                    );
                                }
                            }
                        }
                    }
                }
                Ok(None) => {
                    warn!("Destination ID '{}' not found in storage", dest_id);
                }
                Err(e) => {
                    warn!("Failed to load destination '{}': {}", dest_id, e);
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

    // Legacy execute_binding functions have been removed.
    // Use execute_workflow_by_id() or execute_workflow_streaming_by_id() instead.

    pub async fn execute_workflow_streaming_by_id(
        &self,
        app: &AppHandle,
        workflow_id: &str,
        mut chunk_receiver: tokio::sync::mpsc::Receiver<AudioChunk>,
    ) -> Result<ExecutionResult> {
        info!("Executing streaming workflow by ID: {}", workflow_id);

        // Load StoredWorkflow from storage and convert to full Workflow
        let workflow_storage = app.state::<WorkflowStorage>();
        let stored_workflow = workflow_storage
            .get(workflow_id)?
            .ok_or_else(|| anyhow!("Workflow '{}' not found", workflow_id))?;
        let workflow = stored_workflow.to_full_workflow();
        debug!("Streaming workflow loaded: {}", workflow.name);

        // Initialize streaming session
        let mut session = StreamingSession::new();
        debug!("Streaming session initialized: {}", session.session_id());

        // Process chunks
        let mut chunks_processed = 0;
        while let Some(chunk) = chunk_receiver.recv().await {
            debug!(
                "Processing chunk {} ({} samples, final: {})",
                chunks_processed + 1,
                chunk.audio.len(),
                chunk.is_final
            );

            // Ensure model is loaded
            let model_handle = match self
                .model_pool
                .get_or_load(&workflow.model_config.model_id)
                .await
            {
                Ok(handle) => handle,
                Err(e) => {
                    error!("Failed to load model for chunk {}: {}", chunks_processed + 1, e);
                    continue;
                }
            };

            match model_handle.transcribe(chunk.audio.clone()).await {
                Ok(chunk_text) => {
                    let total_chunk_secs = chunk.audio.len() as f32 / 16000.0;
                    let overlap_secs = chunk.overlap_samples as f32 / 16000.0;
                    let audio_duration_secs = if chunks_processed == 0 {
                        total_chunk_secs
                    } else {
                        (total_chunk_secs - overlap_secs).max(0.0)
                    };
                    let _ = session.merge_chunk(&chunk_text, audio_duration_secs);
                    chunks_processed += 1;
                }
                Err(e) => {
                    error!("Failed to transcribe chunk {}: {}", chunks_processed + 1, e);
                    warn!("Continuing with remaining chunks despite transcription error");
                }
            }
        }

        debug!("All chunks processed ({} total), finalizing session", chunks_processed);
        let audio_duration_secs = session.total_audio_duration();
        let mut final_transcript = session.finalize();
        info!(
            "Streaming transcription complete: {} characters, {} chunks",
            final_transcript.len(),
            chunks_processed
        );

        // Cleaning and optional backfill
        let settings = crate::settings::get_settings(app);
        let enable_backfill = settings.streaming.enable_backfill;

        // Try whole-file backfill if available
        let audio_manager = app.state::<Arc<crate::managers::audio::AudioRecordingManager>>();
        let saved_file_name = audio_manager.get_last_streaming_file_name();
        let mut backfill_status = if enable_backfill { "none" } else { "disabled" };
        if enable_backfill {
            if let Some(file_name) = &saved_file_name {
                let tm_state = app.state::<Arc<crate::managers::transcription::TranscriptionManager>>();
                match crate::streaming::backfill::backfill_whole_file(app, file_name, tm_state.inner().clone()).await {
                    Ok(backfilled_text) => {
                        info!(
                            "Backfill successful: {} characters (was {} from live chunks)",
                            backfilled_text.len(),
                            final_transcript.len()
                        );
                        final_transcript = backfilled_text;
                        backfill_status = "whole-file";
                    }
                    Err(e) => {
                        warn!("Backfill failed for {}: {}. Using live transcript.", file_name, e);
                        backfill_status = "failed";
                    }
                }
            } else {
                debug!("No saved audio file available for backfill, using live transcript");
                backfill_status = "none";
            }
        }

        // Apply cleaning after backfill
        final_transcript = crate::text_cleaning::clean_text(&final_transcript, &settings.cleaning);

        // Save + route
        if final_transcript.trim().is_empty() {
            debug!("Empty transcription; skipping history save and destination routing");
        } else {
            let hm = app.state::<Arc<HistoryManager>>();
            if let Some(file_name) = &saved_file_name {
                hm.save_transcription_with_path(file_name.clone(), final_transcript.clone(), Some(&workflow.id), Some(&workflow.name))
                    .await
                    .map_err(|e| {
                        error!("Failed to save to history with path: {}", e);
                        anyhow!("History save failed: {}", e)
                    })?;
            } else {
                hm.save_transcription(vec![], final_transcript.clone(), Some(&workflow.id), Some(&workflow.name))
                    .await
                    .map_err(|e| {
                        error!("Failed to save to history: {}", e);
                        anyhow!("History save failed: {}", e)
                    })?;
            }

            let router = self.build_router(app, &workflow)?;
            let ctx = DestinationContext { app, output_base: workflow.audio_processing.save_path.clone(), audio_path: None };
            let metadata = self.build_metadata(&workflow);
            let _ = router.route(&ctx, &final_transcript, &metadata).await?;
        }

        let writer_ok = saved_file_name.is_some();
        info!(
            "session={} chunks={} blocked=0 writer_secs={:.1} writer_ok={} backfill={}",
            workflow.id,
            chunks_processed,
            audio_duration_secs,
            writer_ok,
            backfill_status
        );

        info!("Streaming workflow execution complete for: {}", workflow.id);
        Ok(ExecutionResult { workflow_id: workflow.id, text: final_transcript })
    }

    /// Execute a workflow by its ID (for workflow-based shortcuts)
    ///
    /// This method loads a workflow from storage, converts it to a full Workflow,
    /// and executes it through the standard transcription pipeline.
    ///
    /// # Arguments
    /// * `app` - Tauri app handle
    /// * `workflow_id` - The workflow ID to execute
    /// * `samples` - Audio samples to transcribe
    pub async fn execute_workflow_by_id(
        &self,
        app: &AppHandle,
        workflow_id: &str,
        samples: Vec<f32>,
    ) -> Result<ExecutionResult> {
        info!("Executing workflow by ID: {}", workflow_id);

        // 1. Load StoredWorkflow from storage
        let workflow_storage = app.state::<WorkflowStorage>();
        let stored_workflow = workflow_storage
            .get(workflow_id)?
            .ok_or_else(|| anyhow!("Workflow '{}' not found", workflow_id))?;

        debug!("Loaded stored workflow: {}", stored_workflow.name);

        // 2. Convert to full Workflow
        let workflow = stored_workflow.to_full_workflow();
        debug!("Converted to full workflow: {}", workflow.name);

        // 3. Ensure model is loaded
        debug!("Loading model: {}", workflow.model_config.model_id);
        let model_handle = self
            .model_pool
            .get_or_load(&workflow.model_config.model_id)
            .await
            .map_err(|e| anyhow!("Failed to load model: {}", e))?;

        // 4. Transcribe audio
        debug!("Transcribing {} audio samples", samples.len());
        // Log an approximate duration and warn if very large
        let approx_secs = samples.len() as f32 / 16000.0;
        info!(
            "batch inference: {} samples (~{:.1} min)",
            samples.len(),
            approx_secs / 60.0
        );
        if approx_secs > 600.0 {
            warn!(
                "batch duration {:.1} min exceeds recommended threshold; enable streaming to avoid large single-pass inference",
                approx_secs / 60.0
            );
        }
        let text = model_handle
            .transcribe(samples.clone())
            .await
            .map_err(|e| anyhow!("Transcription failed: {}", e))?;

        // Apply text cleaning to workflow-by-id transcript
        let settings = get_settings(app);
        let text = crate::text_cleaning::clean_text(&text, &settings.cleaning);

        info!("Transcription complete: {} characters", text.len());

        // 5. Match legacy parity: if transcription is empty, skip history and destinations
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

            // 6. Route to destinations
            let router = self.build_router(app, &workflow)?;
            let ctx = DestinationContext {
                app,
                output_base: workflow.audio_processing.save_path.clone(),
                audio_path: None,
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
    use once_cell::sync::Lazy;
    use std::sync::Mutex;

    // Prevent concurrent env var mutation across tests
    static ENV_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

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
        let _g = ENV_LOCK.lock().unwrap();
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
        let _g = ENV_LOCK.lock().unwrap();
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
