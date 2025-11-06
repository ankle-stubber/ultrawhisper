use crate::workflow::{StoredWorkflow, WorkflowStorage};
use log::{debug, info, warn};
use tauri::State;

/// List all workflows
#[tauri::command]
pub fn list_workflows(
    storage: State<WorkflowStorage>,
) -> Result<Vec<StoredWorkflow>, String> {
    debug!("tauri command list_workflows invoked");
    storage.list().map_err(|e| e.to_string())
}

/// Get a specific workflow by ID
#[tauri::command]
pub fn get_workflow(
    storage: State<WorkflowStorage>,
    id: String,
) -> Result<Option<StoredWorkflow>, String> {
    debug!("tauri command get_workflow invoked for id {}", id);
    storage.get(&id).map_err(|e| e.to_string())
}

/// Create or update a workflow (upsert)
#[tauri::command]
pub fn upsert_workflow(
    storage: State<WorkflowStorage>,
    workflow: StoredWorkflow,
) -> Result<(), String> {
    info!("tauri command upsert_workflow invoked for {}", workflow.id);
    // Check if workflow exists to determine create vs update
    match storage.get(&workflow.id) {
        Ok(Some(_)) => {
            // Update existing workflow
            storage.update(workflow).map_err(|e| e.to_string())
        }
        Ok(None) => {
            // Create new workflow
            storage.create(workflow).map_err(|e| e.to_string())
        }
        Err(e) => Err(e.to_string()),
    }
}

/// Delete a workflow by ID
#[tauri::command]
pub fn delete_workflow(
    storage: State<WorkflowStorage>,
    id: String,
) -> Result<(), String> {
    warn!("tauri command delete_workflow invoked for {}", id);
    storage.delete(&id).map_err(|e| e.to_string())
}
