//! Model pool implementation - Phase 0 delegates to TranscriptionManager

use crate::managers::transcription::TranscriptionManager;
use anyhow::Result;
use std::sync::Arc;

// Phase 0: Allow dead code as these types are scaffolding for future phases
#[allow(dead_code)]
/// Manages a pool of transcription models
/// Phase 0: Simply wraps the existing TranscriptionManager
pub struct ModelPool {
    transcription_manager: Arc<TranscriptionManager>,
}

impl ModelPool {
    /// Create a new model pool
    pub fn new(transcription_manager: Arc<TranscriptionManager>) -> Self {
        Self {
            transcription_manager,
        }
    }

    /// Get or load a model for transcription (delegates to TranscriptionManager)
    pub async fn get_or_load(&self, model_id: &str) -> Result<ModelHandle> {
        // Check if we need to load a different model
        if self
            .transcription_manager
            .get_current_model()
            .as_deref()
            != Some(model_id)
        {
            self.transcription_manager
                .load_model(model_id)
                .map_err(|e| anyhow::anyhow!("Failed to load model: {}", e))?;
        }

        Ok(ModelHandle {
            model_id: model_id.to_string(),
            manager: Arc::clone(&self.transcription_manager),
        })
    }
}

/// Handle to a loaded model
pub struct ModelHandle {
    #[allow(dead_code)]
    model_id: String,
    manager: Arc<TranscriptionManager>,
}

impl ModelHandle {
    /// Transcribe audio using this model
    pub async fn transcribe(&self, audio: Vec<f32>) -> Result<String> {
        // Phase 0: Delegate to existing manager
        self.manager.transcribe(audio)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_model_pool_types_compile() {
        // Test that the types compile correctly
        // In a real test we'd need a TranscriptionManager instance,
        // but for Phase 0 we just verify the structure is correct
    }
}
