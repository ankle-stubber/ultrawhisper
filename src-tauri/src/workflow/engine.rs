//! Workflow execution engine - orchestrates the entire pipeline

use crate::model_pool::ModelPool;
use anyhow::Result;
use std::sync::Arc;
use tauri::AppHandle;

// Phase 0: Allow dead code as these types are scaffolding for future phases
#[allow(dead_code)]
/// Result summary for a workflow execution
pub struct ExecutionResult {
    pub workflow_id: String,
    pub text: String,
}

/// Workflow engine - Phase 0: stub only, delegates to legacy
pub struct WorkflowEngine {
    #[allow(dead_code)]
    model_pool: Arc<ModelPool>,
}

impl WorkflowEngine {
    /// Create a new workflow engine
    pub fn new(model_pool: Arc<ModelPool>) -> Self {
        Self { model_pool }
    }

    /// Execute a binding/workflow (Phase 0: stub that returns empty result)
    /// In Phase 0, the actual transcription and output still happens via legacy paths
    pub async fn execute_binding(
        &self,
        _app: &AppHandle,
        binding_id: &str,
        _samples: Vec<f32>,
    ) -> Result<ExecutionResult> {
        // Phase 0: Just return an empty result
        // The actual transcription happens in the legacy path
        Ok(ExecutionResult {
            workflow_id: binding_id.to_string(),
            text: String::new(),
        })
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
}
