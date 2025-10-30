use crate::destinations::{Destination, DestinationStorage};
use tauri::State;

/// List all destinations
#[tauri::command]
pub fn list_destinations(
    storage: State<DestinationStorage>,
) -> Result<Vec<Destination>, String> {
    storage.list().map_err(|e| e.to_string())
}

/// Get a specific destination by ID
#[tauri::command]
pub fn get_destination(
    storage: State<DestinationStorage>,
    id: String,
) -> Result<Option<Destination>, String> {
    storage.get(&id).map_err(|e| e.to_string())
}

/// Update an existing destination
#[tauri::command]
pub fn update_destination(
    storage: State<DestinationStorage>,
    destination: Destination,
) -> Result<(), String> {
    storage.update(destination).map_err(|e| e.to_string())
}

/// Create a new destination
#[tauri::command]
pub fn create_destination(
    storage: State<DestinationStorage>,
    destination: Destination,
) -> Result<(), String> {
    storage.create(destination).map_err(|e| e.to_string())
}

/// Delete a destination by ID
#[tauri::command]
pub fn delete_destination(
    storage: State<DestinationStorage>,
    id: String,
) -> Result<(), String> {
    storage.delete(&id).map_err(|e| e.to_string())
}
