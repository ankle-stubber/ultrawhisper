//! Workflow execution engine - orchestrates the entire pipeline

use super::destinations::{DestinationContext, DestinationResult, DestinationRouter, Metadata};
use super::mapper::binding_to_workflow;
use super::types::Workflow;
use crate::managers::history::HistoryManager;
use crate::model_pool::ModelPool;
use crate::router::clipboard::ClipboardDestination;
use crate::router::file::FileDestination;
use crate::destinations::{DestinationConfig, DestinationStorage};
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
            workflow.destination_ids.len()
        );

        Ok(workflow)
    }

    /// Build a destination router from a workflow's destination configs
    ///
    /// NOTE (Bundle 2): This is temporarily simplified during the transition
    /// to destination entities. Bundle 3 will implement full destination lookup
    /// and instantiation from the destination storage.
    ///
    /// This method instantiates the appropriate destination implementations
    /// (Clipboard, File, etc.) based on the workflow configuration.
    #[allow(unused_variables)]
    pub fn build_router(&self, app: &AppHandle, workflow: &Workflow) -> Result<DestinationRouter> {
        let mut router = DestinationRouter::new();

        // Look up destinations by ID from DestinationStorage and instantiate implementations
        let storage_state = app.state::<DestinationStorage>();
        for dest_id in &workflow.destination_ids {
            match storage_state.get(dest_id) {
                Ok(Some(dest)) => {
                    match dest.config {
                        DestinationConfig::ActiveWindow { .. } => {
                            // Minimal: always paste immediately
                            router.add_destination(Box::new(ClipboardDestination::new(true)));
                            debug!("Added ActiveWindow destination: {}", dest_id);
                        }
                        DestinationConfig::FileSystem { ref path, .. } => {
                            let expanded = expand_tilde_str(path);
                            router.add_destination(Box::new(FileDestination::new(Some(expanded))));
                            debug!("Added FileSystem destination: {} -> {}", dest_id, path);
                        }
                        DestinationConfig::Telegram { .. } => {
                            // Telegram not implemented in Bundle 2
                            debug!("Skipping Telegram destination '{}' (not implemented yet)", dest_id);
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
            let router = self.build_router(app, &workflow)?;
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

    /// Execute a binding/workflow with streaming audio chunks
    ///
    /// This is the Phase 2 streaming execution path. It:
    /// 1. Loads the workflow configuration
    /// 2. Ensures the model is loaded
    /// 3. Processes audio chunks as they arrive
    /// 4. Merges chunks using StreamingSession
    /// 5. Saves final transcript to history (if non-empty)
    /// 6. Routes output to destinations
    ///
    /// # Arguments
    /// * `app` - Tauri app handle
    /// * `binding_id` - The binding ID to execute
    /// * `chunk_receiver` - Channel receiving audio chunks during recording
    pub async fn execute_binding_streaming(
        &self,
        app: &AppHandle,
        binding_id: &str,
        mut chunk_receiver: tokio::sync::mpsc::Receiver<AudioChunk>,
    ) -> Result<ExecutionResult> {
        info!("Executing streaming workflow for binding: {}", binding_id);

        // 1. Load workflow configuration
        let workflow = self.get_workflow_for_binding(app, binding_id)?;
        debug!("Streaming workflow loaded: {}", workflow.name);

        // 2. Initialize streaming session
        let mut session = StreamingSession::new();
        debug!(
            "Streaming session initialized: {}",
            session.session_id()
        );

        // 3. Process chunks as they arrive
        let mut chunks_processed = 0;
        while let Some(chunk) = chunk_receiver.recv().await {
            debug!(
                "Processing chunk {} ({} samples, final: {})",
                chunks_processed + 1,
                chunk.audio.len(),
                chunk.is_final
            );

            // Ensure model is loaded (handles mid-stream unloads)
            debug!("Loading model for chunk {}: {}", chunks_processed + 1, workflow.model_config.model_id);
            let model_handle = match self
                .model_pool
                .get_or_load(&workflow.model_config.model_id)
                .await
            {
                Ok(handle) => handle,
                Err(e) => {
                    error!("Failed to load model for chunk {}: {}", chunks_processed + 1, e);
                    continue; // Skip this chunk and try the next one
                }
            };

            // Transcribe chunk
            match model_handle.transcribe(chunk.audio.clone()).await {
                Ok(chunk_text) => {
                    debug!(
                        "Chunk {} transcribed: {} chars",
                        chunks_processed + 1,
                        chunk_text.len()
                    );

                    // Calculate audio duration. Net-new duration subtracts the overlap from subsequent chunks.
                    let total_chunk_secs = chunk.audio.len() as f32 / 16000.0;
                    let overlap_secs = chunk.overlap_samples as f32 / 16000.0;
                    let audio_duration_secs = if chunks_processed == 0 {
                        total_chunk_secs
                    } else {
                        (total_chunk_secs - overlap_secs).max(0.0)
                    };

                    // Merge with accumulated transcript
                    let _current_transcript = session.merge_chunk(&chunk_text, audio_duration_secs);

                    chunks_processed += 1;

                    // TODO Phase 3: Emit streaming-progress event
                    // app.emit_all("streaming-progress", &session.progress())?;
                }
                Err(e) => {
                    error!("Failed to transcribe chunk {}: {}", chunks_processed + 1, e);
                    // Continue processing remaining chunks despite error
                    warn!("Continuing with remaining chunks despite transcription error");
                }
            }
        }

        debug!(
            "All chunks processed ({} total), finalizing session",
            chunks_processed
        );

        // 5. Get audio duration before finalizing (finalize consumes session)
        let audio_duration_secs = session.total_audio_duration();

        // Finalize session and get complete transcript
        let mut final_transcript = session.finalize();

        info!(
            "Streaming transcription complete: {} characters, {} chunks",
            final_transcript.len(),
            chunks_processed
        );

        // 6. Capture settings once at start for finalize/backfill
        let settings = crate::settings::get_settings(app);
        let enable_backfill = settings.streaming.enable_backfill;

        // Get audio manager and check for saved file
        let audio_manager = app.state::<Arc<crate::managers::audio::AudioRecordingManager>>();
        let saved_file_name = audio_manager.get_last_streaming_file_name();

        // Track backfill status for end-of-session logging
        let mut backfill_status = if enable_backfill {
            "none" // Will be updated below
        } else {
            "disabled"
        };

        if enable_backfill {
            if let Some(file_name) = &saved_file_name {
                debug!(
                    "Attempting whole-file backfill from saved audio: {}",
                    file_name
                );

                // Get transcription manager from model pool
                let tm_state = app.state::<Arc<crate::managers::transcription::TranscriptionManager>>();

                match crate::streaming::backfill::backfill_whole_file(
                    app,
                    file_name,
                    tm_state.inner().clone(),
                )
                .await
                {
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
                        warn!(
                            "Backfill failed for {}: {}. Using live transcript.",
                            file_name, e
                        );
                        backfill_status = "failed";
                        // Keep the live merged transcript (final_transcript remains unchanged)
                    }
                }
            } else {
                debug!("No saved audio file available for backfill, using live transcript");
                backfill_status = "none";
            }
        }

        // 7. Phase 1 parity: if transcription is empty, skip history and destinations
        if final_transcript.trim().is_empty() {
            debug!("Empty transcription; skipping history save and destination routing");
        } else {
            // Save to history with workflow tracking
            let hm = app.state::<Arc<HistoryManager>>();

            // If we have a saved WAV file from the writer, use save_transcription_with_path
            // which doesn't write the WAV again (it already exists). Otherwise, fall back
            // to the legacy path with empty samples.
            if let Some(file_name) = &saved_file_name {
                debug!("Saving to history using existing file: {}", file_name);
                hm.save_transcription_with_path(
                    file_name.clone(),
                    final_transcript.clone(),
                    Some(&workflow.id),
                    Some(&workflow.name),
                )
                .await
                .map_err(|e| {
                    error!("Failed to save to history with path: {}", e);
                    anyhow!("History save failed: {}", e)
                })?;
            } else {
                debug!("No saved file available; saving with empty samples");
                hm.save_transcription(
                    vec![], // No audio samples in streaming mode
                    final_transcript.clone(),
                    Some(&workflow.id),
                    Some(&workflow.name),
                )
                .await
                .map_err(|e| {
                    error!("Failed to save to history: {}", e);
                    anyhow!("History save failed: {}", e)
                })?;
            }

            debug!("Saved to history with workflow_id: {}", workflow.id);

            // 8. Route to destinations
            let router = self.build_router(app, &workflow)?;
            let ctx = DestinationContext {
                app,
                output_base: workflow.audio_processing.save_path.clone(),
                audio_path: None, // No audio file in streaming mode
            };
            let metadata = self.build_metadata(&workflow);

            let results = router.route(&ctx, &final_transcript, &metadata).await?;

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

        // Calculate writer stats
        let writer_ok = saved_file_name.is_some();

        // End-of-session summary log
        info!(
            "session={} chunks={} blocked=0 writer_secs={:.1} writer_ok={} backfill={}",
            workflow.id, // Use workflow id as session id
            chunks_processed,
            audio_duration_secs,
            writer_ok,
            backfill_status
        );

        info!("Streaming workflow execution complete for: {}", workflow.id);

        Ok(ExecutionResult {
            workflow_id: workflow.id,
            text: final_transcript,
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
