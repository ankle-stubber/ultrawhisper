use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use std::sync::Arc;
use log::{debug, error, info, warn};

use crate::audio_feedback::{SoundType, play_feedback_sound};
use crate::managers::audio::AudioRecordingManager;
use crate::overlay::{show_recording_overlay, show_transcribing_overlay};
use crate::settings::{self, get_settings, ClipboardHandling, OverlayPosition, PasteMethod, SoundTheme};
use crate::tray::{change_tray_icon, TrayIconState};
use crate::streaming::queue::create_bounded_queue;
use crate::utils;
use crate::workflow::{WorkflowStorage, types::TriggerConfig};

/// Registry to track which hotkeys are currently registered for workflows
/// Maps workflow_id -> binding string
pub struct WorkflowShortcutRegistry(pub std::sync::Mutex<std::collections::HashMap<String, String>>);

pub fn init_shortcuts(app: &AppHandle) {
    // Register workflow hotkeys only. Legacy settings-driven bindings are deprecated.
    let _ = settings::load_or_create_app_settings(app);

    // Register workflow hotkeys (if any are enabled)
    let registered = register_workflow_shortcuts(app);
    if !registered.is_empty() {
        info!("Registered {} workflow hotkey(s)", registered.len());
    }
}

#[tauri::command]
pub fn change_ptt_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);

    // TODO if the setting is currently false, we probably want to
    // cancel any ongoing recordings or actions
    settings.push_to_talk = enabled;

    settings::write_settings(&app, settings);

    Ok(())
}

#[tauri::command]
pub fn change_audio_feedback_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.audio_feedback = enabled;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
pub fn change_audio_feedback_volume_setting(app: AppHandle, volume: f32) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.audio_feedback_volume = volume;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
pub fn change_sound_theme_setting(app: AppHandle, theme: String) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    let parsed = match theme.as_str() {
        "marimba" => SoundTheme::Marimba,
        "pop" => SoundTheme::Pop,
        "custom" => SoundTheme::Custom,
        other => {
            eprintln!("Invalid sound theme '{}', defaulting to marimba", other);
            SoundTheme::Marimba
        }
    };
    settings.sound_theme = parsed;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
pub fn change_translate_to_english_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.translate_to_english = enabled;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
pub fn change_selected_language_setting(app: AppHandle, language: String) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.selected_language = language;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
pub fn change_overlay_position_setting(app: AppHandle, position: String) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    let parsed = match position.as_str() {
        "none" => OverlayPosition::None,
        "top" => OverlayPosition::Top,
        "bottom" => OverlayPosition::Bottom,
        other => {
            eprintln!("Invalid overlay position '{}', defaulting to bottom", other);
            OverlayPosition::Bottom
        }
    };
    settings.overlay_position = parsed;
    settings::write_settings(&app, settings);

    // Update overlay position without recreating window
    crate::utils::update_overlay_position(&app);

    Ok(())
}

#[tauri::command]
pub fn change_debug_mode_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.debug_mode = enabled;
    settings::write_settings(&app, settings);

    // Emit event to notify frontend of debug mode change
    let _ = app.emit(
        "settings-changed",
        serde_json::json!({
            "setting": "debug_mode",
            "value": enabled
        }),
    );

    Ok(())
}

#[tauri::command]
pub fn change_start_hidden_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.start_hidden = enabled;
    settings::write_settings(&app, settings);

    // Notify frontend
    let _ = app.emit(
        "settings-changed",
        serde_json::json!({
            "setting": "start_hidden",
            "value": enabled
        }),
    );

    Ok(())
}

#[tauri::command]
pub fn change_autostart_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.autostart_enabled = enabled;
    settings::write_settings(&app, settings);

    // Apply the autostart setting immediately
    let autostart_manager = app.autolaunch();
    if enabled {
        let _ = autostart_manager.enable();
    } else {
        let _ = autostart_manager.disable();
    }

    // Notify frontend
    let _ = app.emit(
        "settings-changed",
        serde_json::json!({
            "setting": "autostart_enabled",
            "value": enabled
        }),
    );

    Ok(())
}

#[tauri::command]
pub fn update_custom_words(app: AppHandle, words: Vec<String>) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.custom_words = words;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
pub fn change_word_correction_threshold_setting(
    app: AppHandle,
    threshold: f64,
) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    settings.word_correction_threshold = threshold;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
pub fn change_paste_method_setting(app: AppHandle, method: String) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    let parsed = match method.as_str() {
        "ctrl_v" => PasteMethod::CtrlV,
        "direct" => PasteMethod::Direct,
        other => {
            eprintln!("Invalid paste method '{}', defaulting to ctrl_v", other);
            PasteMethod::CtrlV
        }
    };
    settings.paste_method = parsed;
    settings::write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
pub fn change_clipboard_handling_setting(app: AppHandle, handling: String) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);
    let parsed = match handling.as_str() {
        "dont_modify" => ClipboardHandling::DontModify,
        "copy_to_clipboard" => ClipboardHandling::CopyToClipboard,
        other => {
            eprintln!("Invalid clipboard handling '{}', defaulting to dont_modify", other);
            ClipboardHandling::DontModify
        }
    };
    settings.clipboard_handling = parsed;
    settings::write_settings(&app, settings);
    Ok(())
}

/// Determine whether a shortcut string contains at least one non-modifier key.

// ========== Workflow Shortcut Registration ==========

/// Register workflow hotkeys (separate from legacy bindings)
///
/// Notes on behavior:
/// - Collisions are handled in `register_workflow_shortcut` by checking the
///   global shortcut registry. When a collision is detected, we skip
///   registration and log a neutral warning. This results in a "first-win"
///   behavior across legacy bindings and other workflows.
/// - Prior bindings for the same workflow are unregistered atomically via
///   `WorkflowShortcutRegistry` before attempting to register a new one.
///
/// Returns list of successfully registered workflow IDs
pub fn register_workflow_shortcuts(app: &AppHandle) -> Vec<String> {
    let workflow_storage = match app.try_state::<WorkflowStorage>() {
        Some(storage) => storage,
        None => {
            warn!("WorkflowStorage not initialized, skipping workflow shortcut registration");
            return vec![];
        }
    };

    let workflows = match workflow_storage.list() {
        Ok(wfs) => wfs,
        Err(e) => {
            error!("Failed to list workflows: {}", e);
            return vec![];
        }
    };

    let mut registered = Vec::new();

    for workflow in workflows {
        if !workflow.enabled {
            continue;
        }

        if let TriggerConfig::Hotkey { binding, push_to_talk } = &workflow.trigger {
            // Register with closure that calls workflow execution
            match register_workflow_shortcut(app, &workflow.id, binding, *push_to_talk) {
                Ok(_) => {
                    registered.push(workflow.id.clone());
                }
                Err(e) => {
                    // Collision or other error - already logged in register_workflow_shortcut
                    debug!("Skipped workflow {} registration: {}", workflow.id, e);
                }
            }
        }
    }

    registered
}

/// Register a single workflow shortcut
fn register_workflow_shortcut(
    app: &AppHandle,
    workflow_id: &str,
    binding: &str,
    push_to_talk: bool,
) -> Result<(), String> {
    let registry = app.state::<WorkflowShortcutRegistry>();

    // STEP 1: Atomically remove old binding from registry
    let old_binding = {
        let mut reg = registry.0.lock().unwrap();
        reg.remove(workflow_id)
    };

    // STEP 2: Unregister old binding if it exists
    if let Some(old_binding_str) = old_binding {
        match old_binding_str.parse::<Shortcut>() {
            Ok(old_shortcut) => {
                match app.global_shortcut().unregister(old_shortcut) {
                    Ok(_) => debug!("Unregistered workflow shortcut {} -> {}", workflow_id, old_binding_str),
                    Err(e) => error!("Failed to unregister old workflow shortcut {} -> {}: {}", workflow_id, old_binding_str, e),
                }
            }
            Err(e) => {
                error!("Failed to parse old workflow binding {} -> {}: {}", workflow_id, old_binding_str, e);
            }
        }
    }

    // STEP 3: Parse new shortcut
    let shortcut = binding
        .parse::<Shortcut>()
        .map_err(|e| format!("Invalid shortcut: {}", e))?;

    // STEP 4: Check for collision with any existing registration (legacy or other workflows)
    if app.global_shortcut().is_registered(shortcut) {
        // Neutral wording; we don't assert which subsystem owns the registration
        warn!(
            "Skipping workflow {} — shortcut {} already registered",
            workflow_id, binding
        );
        return Err(format!("Shortcut '{}' is already registered", binding));
    }

    // STEP 5: Register new binding
    let workflow_id_clone = workflow_id.to_string();
    app.global_shortcut()
        .on_shortcut(shortcut, move |ah, scut, event| {
            if scut == &shortcut {
                if push_to_talk {
                    if event.state == ShortcutState::Pressed {
                        start_workflow_recording(ah, &workflow_id_clone);
                    } else if event.state == ShortcutState::Released {
                        stop_and_execute_workflow(ah, &workflow_id_clone);
                    }
                } else {
                    // Toggle mode - use AudioRecordingManager state
                    if event.state == ShortcutState::Pressed {
                        if let Some(audio_manager) = ah.try_state::<Arc<AudioRecordingManager>>() {
                            if audio_manager.is_recording() {
                                debug!("Toggle: stopping workflow recording");
                                stop_and_execute_workflow(ah, &workflow_id_clone);
                            } else {
                                debug!("Toggle: starting workflow recording");
                                start_workflow_recording(ah, &workflow_id_clone);
                            }
                        } else {
                            warn!("Audio manager not available for toggle workflow");
                            start_workflow_recording(ah, &workflow_id_clone);
                        }
                    }
                }
            }
        })
        .map_err(|e| format!("Registration failed: {}", e))?;

    // STEP 6: Insert into registry only on success
    {
        let mut reg = registry.0.lock().unwrap();
        reg.insert(workflow_id.to_string(), binding.to_string());
    }
    debug!("Registered workflow shortcut {} -> {}", workflow_id, binding);

    Ok(())
}

/// Re-register workflows on change
pub fn refresh_workflow_shortcuts(app: &AppHandle) {
    debug!("Refreshing workflow shortcuts");
    unregister_all_workflow_shortcuts(app);
    register_workflow_shortcuts(app);
}

/// Unregister all workflow shortcuts
fn unregister_all_workflow_shortcuts(app: &AppHandle) {
    let registry = match app.try_state::<WorkflowShortcutRegistry>() {
        Some(reg) => reg,
        None => return,
    };

    // Clone snapshot to avoid holding lock during unregister calls
    let snapshot = {
        let reg = registry.0.lock().unwrap();
        reg.clone()
    };

    // Unregister all from snapshot
    for (workflow_id, binding) in snapshot {
        match binding.parse::<Shortcut>() {
            Ok(shortcut) => {
                match app.global_shortcut().unregister(shortcut) {
                    Ok(_) => debug!("Unregistered workflow shortcut {} -> {}", workflow_id, binding),
                    Err(e) => error!("Failed to unregister workflow shortcut {} -> {}: {}", workflow_id, binding, e),
                }
            }
            Err(e) => {
                error!("Failed to parse workflow binding {} -> {}: {}", workflow_id, binding, e);
            }
        }
    }

    // Clear the registry
    {
        let mut reg = registry.0.lock().unwrap();
        reg.clear();
    }
}

/// Start workflow recording (for push-to-talk or toggle mode)
fn start_workflow_recording(app: &AppHandle, workflow_id: &str) {
    debug!("Starting workflow recording: {}", workflow_id);

    let settings = get_settings(app);
    let is_always_on = settings.always_on_microphone;
    let streaming_enabled = settings.streaming.enabled;

    change_tray_icon(app, TrayIconState::Recording);
    show_recording_overlay(app);

    // Emit app-wide event so UI can reflect active workflow
    let _ = app.emit(
        "workflow-recording-started",
        serde_json::json!({ "workflow_id": workflow_id }),
    );

    let rm = app.state::<Arc<AudioRecordingManager>>();

    if streaming_enabled {
        // Streaming mode: create queue, start streaming recording, and spawn engine task
        let (chunk_tx, chunk_rx) = create_bounded_queue(settings.streaming.max_queue_size);
        if let Err(e) = rm.start_streaming_recording(
            workflow_id,
            settings.streaming.chunk_duration_seconds * 1000,
            settings.streaming.overlap_seconds * 1000,
            chunk_tx,
            settings.streaming.backpressure_policy,
        ) {
            error!("Failed to start streaming recording: {}", e);
            return;
        }

        let workflow_engine = Arc::clone(&app.state::<Arc<crate::workflow::WorkflowEngine>>());
        let app_clone = app.clone();
        let wf_id = workflow_id.to_string();
        tauri::async_runtime::spawn(async move {
            match workflow_engine
                .execute_workflow_streaming_by_id(&app_clone, &wf_id, chunk_rx)
                .await
            {
                Ok(_) => {
                    utils::hide_recording_overlay(&app_clone);
                    change_tray_icon(&app_clone, TrayIconState::Idle);
                }
                Err(e) => {
                    error!("Streaming workflow failed: {}", e);
                    utils::hide_recording_overlay(&app_clone);
                    change_tray_icon(&app_clone, TrayIconState::Idle);
                }
            }
        });

        // Play audio feedback
        if is_always_on {
            play_feedback_sound(app, SoundType::Start);
        } else {
            let app_clone = app.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(100));
                play_feedback_sound(&app_clone, SoundType::Start);
            });
        }
    } else {
        // Batch (non-streaming) mode
        if is_always_on {
            play_feedback_sound(app, SoundType::Start);
            let recording_started = rm.try_start_recording(workflow_id);
            debug!("Recording started: {}", recording_started);
        } else {
            if rm.try_start_recording(workflow_id) {
                // Small delay to ensure microphone stream is active
                let app_clone = app.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    play_feedback_sound(&app_clone, SoundType::Start);
                });
            } else {
                debug!("Failed to start recording");
            }
        }
    }
}

/// Stop recording and execute workflow
fn stop_and_execute_workflow(app: &AppHandle, workflow_id: &str) {
    debug!("Stopping workflow recording: {}", workflow_id);

    let ah = app.clone();
    let rm = Arc::clone(&app.state::<Arc<AudioRecordingManager>>());
    let workflow_id = workflow_id.to_string();

    change_tray_icon(app, TrayIconState::Transcribing);
    show_transcribing_overlay(app);
    play_feedback_sound(app, SoundType::Stop);

    // Emit stopped event immediately so the UI clears promptly
    let _ = app.emit(
        "workflow-recording-stopped",
        serde_json::json!({ "workflow_id": &workflow_id }),
    );

    // Streaming or Batch stop
    let settings = get_settings(app);
    if settings.streaming.enabled {
        // Stop streaming recording; streaming task will finalize and clean up UI
        if let Err(e) = rm.stop_streaming_recording(&workflow_id) {
            error!("Failed to stop streaming recording: {}", e);
            utils::hide_recording_overlay(&ah);
            change_tray_icon(&ah, TrayIconState::Idle);
        }
    } else {
        // Stop recording and get samples (batch)
        tauri::async_runtime::spawn(async move {
            match rm.stop_recording(&workflow_id) {
                Some(samples) => {
                    debug!("Got {} samples from recording", samples.len());
                    let workflow_engine = ah.state::<Arc<crate::workflow::WorkflowEngine>>();
                    match workflow_engine
                        .execute_workflow_by_id(&ah, &workflow_id, samples)
                        .await
                    {
                        Ok(result) => {
                            info!("Workflow execution complete: {} characters", result.text.len());
                        }
                        Err(e) => {
                            error!("Workflow execution failed: {}", e);
                        }
                    }
                }
                None => {
                    error!("Failed to stop recording: no samples returned");
                }
            }
            // Cleanup UI
            utils::hide_recording_overlay(&ah);
            change_tray_icon(&ah, TrayIconState::Idle);
        });
    }
}

// Toggle workflow recording (for non-push-to-talk mode) - Temporarily disabled, needs state management refactor
// fn toggle_workflow_recording(app: &AppHandle, workflow_id: &str) {
//     let toggle_state_manager = app.state::<ManagedToggleState>();
//     let mut states = toggle_state_manager
//         .lock()
//         .expect("Failed to lock toggle state manager");
// 
//     let is_currently_active = states
//         .active_toggles
//         .entry(workflow_id.to_string())
//         .or_insert(false);
// 
//     if *is_currently_active {
//         debug!("Toggle: stopping workflow recording");
//         drop(states); // Release lock before calling stop
//         stop_and_execute_workflow(app, workflow_id);
// 
//         // Re-acquire lock to update state
//         let mut states = toggle_state_manager
//             .lock()
//             .expect("Failed to lock toggle state manager");
//         states.active_toggles.insert(workflow_id.to_string(), false);
//     } else {
//         debug!("Toggle: starting workflow recording");
//         *is_currently_active = true;
//         drop(states); // Release lock before calling start
//         start_workflow_recording(app, workflow_id);
//     }
// }
