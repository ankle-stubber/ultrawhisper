use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
pub fn pick_directory(app: AppHandle) -> Result<Option<String>, String> {
    let (tx, rx) = std::sync::mpsc::channel();

    app.dialog()
        .file()
        .set_title("Select output directory for transcriptions")
        .pick_folder(move |path| {
            let result = path.map(|p| {
                // Ensure we get a valid string path
                p.to_string()
            });
            // Send the result through the channel
            let _ = tx.send(result);
        });

    // Block and wait for the result
    match rx.recv() {
        Ok(result) => Ok(result),
        Err(e) => {
            // If we fail to receive, return None instead of an error
            // This can happen if the user cancels the dialog
            log::warn!("Directory picker channel error: {}", e);
            Ok(None)
        }
    }
}