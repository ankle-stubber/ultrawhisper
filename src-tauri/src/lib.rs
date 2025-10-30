mod actions;
mod audio_feedback;
pub mod audio_toolkit;
mod clipboard;
mod commands;
mod destinations;
mod file_output;
mod logger;
mod managers;
mod overlay;
mod settings;
mod shortcut;
mod templates;
mod tray;
mod utils;

// New modules for workflow architecture
mod workflow;
mod model_pool;
mod router;
mod streaming;

use managers::audio::AudioRecordingManager;
use managers::batch::BatchTranscriptionManager;
use managers::history::HistoryManager;
use managers::logs::LogManager;
use managers::model::ModelManager;
use managers::transcription::TranscriptionManager;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::image::Image;

use tauri::tray::TrayIconBuilder;
use tauri::Emitter;
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

#[derive(Default)]
struct ShortcutToggleStates {
    // Map: shortcut_binding_id -> is_active
    active_toggles: HashMap<String, bool>,
}

type ManagedToggleState = Mutex<ShortcutToggleStates>;

/// Clean up stale temporary files from the recordings directory.
///
/// This function removes `.tmp` files older than 3 days from the recordings directory.
/// These files may have been left behind if the app crashed during recording.
///
/// # Arguments
/// * `app` - The Tauri app handle for resolving the recordings directory
///
/// # Behavior
/// - Logs warnings on failures but does not crash the application
/// - Runs at startup to clean up leftover temporary files
fn cleanup_stale_temp_files(app: &AppHandle) {
    use log::{debug, warn};
    use std::fs;

    // Get the recordings directory
    let recordings_dir = match app.path().app_data_dir() {
        Ok(dir) => dir.join("recordings"),
        Err(e) => {
            warn!("Failed to get app data directory for temp cleanup: {}", e);
            return;
        }
    };

    // If the recordings directory doesn't exist yet, nothing to clean
    if !recordings_dir.exists() {
        debug!("Recordings directory does not exist, skipping temp cleanup");
        return;
    }

    // Read directory entries
    let entries = match fs::read_dir(&recordings_dir) {
        Ok(entries) => entries,
        Err(e) => {
            warn!("Failed to read recordings directory for temp cleanup: {}", e);
            return;
        }
    };

    // Calculate cutoff time (3 days ago)
    let now = SystemTime::now();
    let three_days_ago = now
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .saturating_sub(3 * 24 * 60 * 60); // 3 days in seconds

    let mut cleaned_count = 0;
    let mut failed_count = 0;

    // Iterate through entries and clean up stale .tmp files
    for entry in entries.flatten() {
        let path = entry.path();

        // Only process .tmp files
        if path.extension().and_then(|s| s.to_str()) != Some("tmp") {
            continue;
        }

        // Check file age
        match fs::metadata(&path) {
            Ok(metadata) => {
                match metadata.modified() {
                    Ok(modified) => {
                        match modified.duration_since(UNIX_EPOCH) {
                            Ok(duration) => {
                                let file_age_secs = duration.as_secs();

                                // If file is older than 3 days, delete it
                                if file_age_secs < three_days_ago {
                                    debug!(
                                        "Removing stale temp file: {} (age: {} days)",
                                        path.display(),
                                        (now.duration_since(modified).unwrap().as_secs() / 86400)
                                    );

                                    match fs::remove_file(&path) {
                                        Ok(_) => {
                                            cleaned_count += 1;
                                        }
                                        Err(e) => {
                                            warn!(
                                                "Failed to remove stale temp file {}: {}",
                                                path.display(),
                                                e
                                            );
                                            failed_count += 1;
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("Failed to get duration for {}: {}", path.display(), e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to get modified time for {}: {}", path.display(), e);
                    }
                }
            }
            Err(e) => {
                warn!("Failed to get metadata for {}: {}", path.display(), e);
            }
        }
    }

    if cleaned_count > 0 {
        debug!(
            "Temp file cleanup complete: {} removed, {} failed",
            cleaned_count, failed_count
        );
    } else {
        debug!("No stale temp files found");
    }
}

fn show_main_window(app: &AppHandle) {
    if let Some(main_window) = app.get_webview_window("main") {
        // First, ensure the window is visible
        if let Err(e) = main_window.show() {
            eprintln!("Failed to show window: {}", e);
        }
        // Then, bring it to the front and give it focus
        if let Err(e) = main_window.set_focus() {
            eprintln!("Failed to focus window: {}", e);
        }
        // Optional: On macOS, ensure the app becomes active if it was an accessory
        #[cfg(target_os = "macos")]
        {
            if let Err(e) = app.set_activation_policy(tauri::ActivationPolicy::Regular) {
                eprintln!("Failed to set activation policy to Regular: {}", e);
            }
        }
    } else {
        eprintln!("Main window not found.");
    }
}

fn initialize_core_logic(app_handle: &AppHandle) {
    // Initialize log manager FIRST (before any logging happens)
    let log_manager = Arc::new(LogManager::new(app_handle.clone()));
    app_handle.manage(log_manager.clone());

    // Initialize the combined logger (forwards to env_logger + captures to LogManager)
    if let Err(e) = crate::logger::CombinedLogger::init(log_manager.clone()) {
        eprintln!("Failed to initialize logger: {}", e);
    }

    // Log startup message to test logger
    log::info!("UltraWhisper starting up - logging system initialized");

    // Clean up stale temporary files from previous sessions
    cleanup_stale_temp_files(app_handle);

    // First, initialize the managers
    let recording_manager = Arc::new(
        AudioRecordingManager::new(app_handle).expect("Failed to initialize recording manager"),
    );
    let model_manager =
        Arc::new(ModelManager::new(app_handle).expect("Failed to initialize model manager"));
    let transcription_manager = Arc::new(
        TranscriptionManager::new(app_handle, model_manager.clone())
            .expect("Failed to initialize transcription manager"),
    );
    let history_manager =
        Arc::new(HistoryManager::new(app_handle).expect("Failed to initialize history manager"));
    let batch_manager = Arc::new(
        BatchTranscriptionManager::new(app_handle, transcription_manager.clone())
            .expect("Failed to initialize batch transcription manager"),
    );

    // Add managers to Tauri's managed state
    app_handle.manage(recording_manager.clone());
    app_handle.manage(model_manager.clone());
    app_handle.manage(transcription_manager.clone());
    app_handle.manage(history_manager.clone());
    app_handle.manage(batch_manager.clone());

    // Initialize destination storage and seed defaults (Bundle 2)
    let dest_storage = crate::destinations::DestinationStorage::new(app_handle.clone())
        .expect("Failed to initialize destination storage");
    // Best-effort seeding; don't crash on error
    let _ = crate::destinations::seed_defaults_if_empty(&dest_storage);
    // Migrate legacy binding configuration to destinations (Bundle 3)
    let _ = crate::destinations::migrate_legacy_bindings_if_needed(app_handle, &dest_storage);
    app_handle.manage(dest_storage.clone());

    // Initialize workflow architecture components (Phase 0)
    let model_pool = Arc::new(model_pool::ModelPool::new(Arc::clone(&transcription_manager)));
    let workflow_engine = Arc::new(workflow::WorkflowEngine::new(Arc::clone(&model_pool)));
    app_handle.manage(model_pool);
    app_handle.manage(workflow_engine);

    // Initialize the shortcuts
    shortcut::init_shortcuts(app_handle);

    // Apply macOS Accessory policy if starting hidden
    #[cfg(target_os = "macos")]
    {
        let settings = settings::get_settings(app_handle);
        if settings.start_hidden {
            let _ = app_handle.set_activation_policy(tauri::ActivationPolicy::Accessory);
        }
    }
    // Get the current theme to set the appropriate initial icon
    let initial_theme = tray::get_current_theme(app_handle);

    // Choose the appropriate initial icon based on theme
    let initial_icon_path = tray::get_icon_path(initial_theme, tray::TrayIconState::Idle);

    let tray = TrayIconBuilder::new()
        .icon(
            Image::from_path(
                app_handle
                    .path()
                    .resolve(initial_icon_path, tauri::path::BaseDirectory::Resource)
                    .unwrap(),
            )
            .unwrap(),
        )
        .show_menu_on_left_click(true)
        .icon_as_template(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "settings" => {
                show_main_window(app);
            }
            "check_updates" => {
                show_main_window(app);
                let _ = app.emit("check-for-updates", ());
            }
            "cancel" => {
                use crate::utils::cancel_current_operation;

                // Use centralized cancellation that handles all operations
                cancel_current_operation(app);
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .build(app_handle)
        .unwrap();
    app_handle.manage(tray);

    // Initialize tray menu with idle state
    utils::update_tray_menu(app_handle, &utils::TrayIconState::Idle);

    // Get the autostart manager and configure based on user setting
    let autostart_manager = app_handle.autolaunch();
    let settings = settings::get_settings(&app_handle);

    if settings.autostart_enabled {
        // Enable autostart if user has opted in
        let _ = autostart_manager.enable();
    } else {
        // Disable autostart if user has opted out
        let _ = autostart_manager.disable();
    }

    // Create the recording overlay window (hidden by default)
    utils::create_recording_overlay(app_handle);
}

#[tauri::command]
fn trigger_update_check(app: AppHandle) -> Result<(), String> {
    app.emit("check-for-updates", ())
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Note: Logger will be initialized in setup() after LogManager is created

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app);
        }))
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_macos_permissions::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(
            tauri_plugin_sql::Builder::default()
                .add_migrations(
                    "sqlite:history.db",
                    managers::history::HistoryManager::get_migrations(),
                )
                .build(),
        )
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .manage(Mutex::new(ShortcutToggleStates::default()))
        .setup(move |app| {
            let settings = settings::get_settings(&app.handle());
            let app_handle = app.handle().clone();

            initialize_core_logic(&app_handle);

            // Show main window only if not starting hidden
            if !settings.start_hidden {
                if let Some(main_window) = app_handle.get_webview_window("main") {
                    main_window.show().unwrap();
                    main_window.set_focus().unwrap();
                }
            }

            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _res = window.hide();
                #[cfg(target_os = "macos")]
                {
                    let res = window
                        .app_handle()
                        .set_activation_policy(tauri::ActivationPolicy::Accessory);
                    if let Err(e) = res {
                        println!("Failed to set activation policy: {}", e);
                    }
                }
            }
            tauri::WindowEvent::ThemeChanged(theme) => {
                println!("Theme changed to: {:?}", theme);
                // Update tray icon to match new theme, maintaining idle state
                utils::change_tray_icon(&window.app_handle(), utils::TrayIconState::Idle);
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            shortcut::change_binding,
            shortcut::reset_binding,
            shortcut::update_binding_output_config,
            shortcut::change_ptt_setting,
            shortcut::change_audio_feedback_setting,
            shortcut::change_audio_feedback_volume_setting,
            shortcut::change_sound_theme_setting,
            shortcut::change_start_hidden_setting,
            shortcut::change_autostart_setting,
            shortcut::change_translate_to_english_setting,
            shortcut::change_selected_language_setting,
            shortcut::change_overlay_position_setting,
            shortcut::change_debug_mode_setting,
            shortcut::change_word_correction_threshold_setting,
            shortcut::change_paste_method_setting,
            shortcut::change_clipboard_handling_setting,
            shortcut::update_custom_words,
            shortcut::suspend_binding,
            shortcut::resume_binding,
            trigger_update_check,
            commands::cancel_operation,
            commands::get_app_dir_path,
            commands::models::get_available_models,
            commands::models::get_model_info,
            commands::models::download_model,
            commands::models::delete_model,
            commands::models::cancel_download,
            commands::models::set_active_model,
            commands::models::get_current_model,
            commands::models::get_transcription_model_status,
            commands::models::is_model_loading,
            commands::models::has_any_models_available,
            commands::models::has_any_models_or_downloads,
            commands::models::get_recommended_first_model,
            commands::audio::update_microphone_mode,
            commands::audio::get_microphone_mode,
            commands::audio::get_available_microphones,
            commands::audio::set_selected_microphone,
            commands::audio::get_selected_microphone,
            commands::audio::get_available_output_devices,
            commands::audio::set_selected_output_device,
            commands::audio::get_selected_output_device,
            commands::audio::play_test_sound,
            commands::audio::check_custom_sounds,
            commands::transcription::set_model_unload_timeout,
            commands::transcription::get_model_load_status,
            commands::transcription::unload_model_manually,
            commands::history::get_history_entries,
            commands::history::toggle_history_entry_saved,
            commands::history::get_audio_file_path,
            commands::history::delete_history_entry,
            commands::history::update_history_limit,
            commands::settings::pick_directory,
            commands::settings::change_use_workflow_engine_setting,
            commands::settings::change_streaming_settings,
            commands::telegram::store_telegram_credentials,
            commands::telegram::get_telegram_credentials,
            commands::telegram::delete_telegram_credentials,
            commands::telegram::test_telegram_connection,
            commands::telegram::telegram_credentials_exist,
            commands::batch::process_batch_now,
            commands::batch::get_batch_settings,
            commands::batch::update_batch_settings,
            commands::batch::set_batch_enabled,
            commands::batch::add_watch_folder,
            commands::batch::remove_watch_folder,
            commands::batch::set_check_interval,
            commands::batch::set_stability_timeout,
            commands::batch::set_delete_after_transcription,
            commands::batch::set_save_to_history,
            commands::batch::set_file_patterns,
            commands::batch::validate_watch_folder,
            commands::logs::get_logs,
            commands::logs::clear_logs,
            commands::logs::export_logs,
            commands::destinations::list_destinations,
            commands::destinations::get_destination,
            commands::destinations::update_destination,
            commands::destinations::create_destination,
            commands::destinations::delete_destination
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
