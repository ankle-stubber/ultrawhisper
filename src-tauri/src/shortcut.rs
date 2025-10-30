use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use std::sync::Arc;
use log::{debug, error, info, warn};

use crate::actions::ACTION_MAP;
use crate::audio_feedback::{SoundType, play_feedback_sound};
use crate::managers::audio::AudioRecordingManager;
use crate::overlay::{show_recording_overlay, show_transcribing_overlay};
use crate::settings::ShortcutBinding;
use crate::settings::{self, get_settings, ClipboardHandling, OverlayPosition, PasteMethod, SoundTheme};
use crate::tray::{change_tray_icon, TrayIconState};
use crate::utils;
use crate::workflow::{WorkflowStorage, types::TriggerConfig};
use crate::ManagedToggleState;

pub fn init_shortcuts(app: &AppHandle) {
    let settings = settings::load_or_create_app_settings(app);

    // Register legacy bindings first
    for (_id, binding) in settings.bindings {
        if let Err(e) = _register_shortcut(app, binding) {
            eprintln!("Failed to register shortcut {} during init: {}", _id, e);
        }
    }

    // Then register workflow hotkeys (if flag enabled)
    let registered = register_workflow_shortcuts(app);
    if !registered.is_empty() {
        info!("Registered {} workflow hotkey(s)", registered.len());
    }
}

#[derive(Serialize)]
pub struct BindingResponse {
    success: bool,
    binding: Option<ShortcutBinding>,
    error: Option<String>,
}

#[tauri::command]
pub fn change_binding(
    app: AppHandle,
    id: String,
    binding: String,
) -> Result<BindingResponse, String> {
    let mut settings = settings::get_settings(&app);

    // Get the binding to modify
    let binding_to_modify = match settings.bindings.get(&id) {
        Some(binding) => binding.clone(),
        None => {
            let error_msg = format!("Binding with id '{}' not found", id);
            eprintln!("change_binding error: {}", error_msg);
            return Ok(BindingResponse {
                success: false,
                binding: None,
                error: Some(error_msg),
            });
        }
    };

    // Unregister the existing binding
    if let Err(e) = _unregister_shortcut(&app, binding_to_modify.clone()) {
        let error_msg = format!("Failed to unregister shortcut: {}", e);
        eprintln!("change_binding error: {}", error_msg);
    }

    // Validate the new shortcut before we touch the current registration
    if let Err(e) = validate_shortcut_string(&binding) {
        eprintln!("change_binding validation error: {}", e);
        return Err(e);
    }

    // Create an updated binding
    let mut updated_binding = binding_to_modify;
    updated_binding.current_binding = binding;

    // Register the new binding
    if let Err(e) = _register_shortcut(&app, updated_binding.clone()) {
        let error_msg = format!("Failed to register shortcut: {}", e);
        eprintln!("change_binding error: {}", error_msg);
        return Ok(BindingResponse {
            success: false,
            binding: None,
            error: Some(error_msg),
        });
    }

    // Update the binding in the settings
    settings.bindings.insert(id, updated_binding.clone());

    // Save the settings
    settings::write_settings(&app, settings);

    // Return the updated binding
    Ok(BindingResponse {
        success: true,
        binding: Some(updated_binding),
        error: None,
    })
}

#[tauri::command]
pub fn reset_binding(app: AppHandle, id: String) -> Result<BindingResponse, String> {
    let binding = settings::get_stored_binding(&app, &id);

    return change_binding(app, id, binding.default_binding);
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
/// We allow single non-modifier keys (e.g. "f5" or "space") but disallow
/// modifier-only combos (e.g. "ctrl" or "ctrl+shift").
fn validate_shortcut_string(raw: &str) -> Result<(), String> {
    let modifiers = [
        "ctrl", "control", "shift", "alt", "option", "meta", "command", "cmd", "super", "win",
        "windows",
    ];
    let has_non_modifier = raw
        .split('+')
        .any(|part| !modifiers.contains(&part.trim().to_lowercase().as_str()));
    if has_non_modifier {
        Ok(())
    } else {
        Err("Shortcut must contain at least one non-modifier key".into())
    }
}

/// Update the output configuration for a specific binding
#[tauri::command]
pub fn update_binding_output_config(
    app: AppHandle,
    id: String,
    paste_to_window: bool,
    save_to_file: bool,
    output_path: Option<String>
) -> Result<(), String> {
    let mut settings = settings::get_settings(&app);

    if let Some(binding) = settings.bindings.get_mut(&id) {
        binding.paste_to_window = paste_to_window;
        binding.save_to_file = save_to_file;
        binding.output_path = output_path;

        settings::write_settings(&app, settings);
        Ok(())
    } else {
        Err(format!("Binding with id '{}' not found", id))
    }
}

/// Temporarily unregister a binding while the user is editing it in the UI.
/// This avoids firing the action while keys are being recorded.
#[tauri::command]
pub fn suspend_binding(app: AppHandle, id: String) -> Result<(), String> {
    if let Some(b) = settings::get_bindings(&app).get(&id).cloned() {
        if let Err(e) = _unregister_shortcut(&app, b) {
            eprintln!("suspend_binding error for id '{}': {}", id, e);
            return Err(e);
        }
    }
    Ok(())
}

/// Re-register the binding after the user has finished editing.
#[tauri::command]
pub fn resume_binding(app: AppHandle, id: String) -> Result<(), String> {
    if let Some(b) = settings::get_bindings(&app).get(&id).cloned() {
        if let Err(e) = _register_shortcut(&app, b) {
            eprintln!("resume_binding error for id '{}': {}", id, e);
            return Err(e);
        }
    }
    Ok(())
}

fn _register_shortcut(app: &AppHandle, binding: ShortcutBinding) -> Result<(), String> {
    // Validate human-level rules first
    if let Err(e) = validate_shortcut_string(&binding.current_binding) {
        eprintln!(
            "_register_shortcut validation error for binding '{}': {}",
            binding.current_binding, e
        );
        return Err(e);
    }

    // Parse shortcut and return error if it fails
    let shortcut = match binding.current_binding.parse::<Shortcut>() {
        Ok(s) => s,
        Err(e) => {
            let error_msg = format!(
                "Failed to parse shortcut '{}': {}",
                binding.current_binding, e
            );
            eprintln!("_register_shortcut parse error: {}", error_msg);
            return Err(error_msg);
        }
    };

    // Prevent duplicate registrations that would silently shadow one another
    if app.global_shortcut().is_registered(shortcut) {
        let error_msg = format!("Shortcut '{}' is already in use", binding.current_binding);
        eprintln!("_register_shortcut duplicate error: {}", error_msg);
        return Err(error_msg);
    }

    // Clone binding.id for use in the closure
    let binding_id_for_closure = binding.id.clone();

    app.global_shortcut()
        .on_shortcut(shortcut, move |ah, scut, event| {
            if scut == &shortcut {
                let shortcut_string = scut.into_string();
                let settings = get_settings(ah);

                if let Some(action) = ACTION_MAP.get(&binding_id_for_closure) {
                    if settings.push_to_talk {
                        if event.state == ShortcutState::Pressed {
                            action.start(ah, &binding_id_for_closure, &shortcut_string);
                        } else if event.state == ShortcutState::Released {
                            action.stop(ah, &binding_id_for_closure, &shortcut_string);
                        }
                    } else {
                        if event.state == ShortcutState::Pressed {
                            let toggle_state_manager = ah.state::<ManagedToggleState>();

                            let mut states = toggle_state_manager.lock().expect("Failed to lock toggle state manager");

                            let is_currently_active = states.active_toggles
                                .entry(binding_id_for_closure.clone())
                                .or_insert(false);

                            if *is_currently_active {
                                action.stop(
                                    ah,
                                    &binding_id_for_closure,
                                    &shortcut_string,
                                );
                                *is_currently_active = false; // Update state to inactive
                            } else {
                                action.start(ah, &binding_id_for_closure, &shortcut_string);
                                *is_currently_active = true; // Update state to active
                            }
                        }
                    }
                } else {
                    println!(
                        "Warning: No action defined in ACTION_MAP for shortcut ID '{}'. Shortcut: '{}', State: {:?}",
                        binding_id_for_closure, shortcut_string, event.state
                    );
                }
            }
        })
        .map_err(|e| {
            let error_msg = format!("Couldn't register shortcut '{}': {}", binding.current_binding, e);
            eprintln!("_register_shortcut registration error: {}", error_msg);
            error_msg
        })?;

    Ok(())
}

fn _unregister_shortcut(app: &AppHandle, binding: ShortcutBinding) -> Result<(), String> {
    let shortcut = match binding.current_binding.parse::<Shortcut>() {
        Ok(s) => s,
        Err(e) => {
            let error_msg = format!(
                "Failed to parse shortcut '{}' for unregistration: {}",
                binding.current_binding, e
            );
            eprintln!("_unregister_shortcut parse error: {}", error_msg);
            return Err(error_msg);
        }
    };

    app.global_shortcut().unregister(shortcut).map_err(|e| {
        let error_msg = format!(
            "Failed to unregister shortcut '{}': {}",
            binding.current_binding, e
        );
        eprintln!("_unregister_shortcut error: {}", error_msg);
        error_msg
    })?;

    Ok(())
}

// ========== Workflow Shortcut Registration ==========

/// Register workflow hotkeys (separate from legacy bindings)
/// Returns list of successfully registered workflow IDs
pub fn register_workflow_shortcuts(app: &AppHandle) -> Vec<String> {
    let settings = get_settings(app);

    // Gate behind flag
    if !settings.use_workflow_engine {
        debug!("Workflow engine disabled, skipping workflow shortcut registration");
        return vec![];
    }

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
    let mut collisions = Vec::new();

    for workflow in workflows {
        if !workflow.enabled {
            continue;
        }

        if let TriggerConfig::Hotkey { binding, push_to_talk } = &workflow.trigger {
            // Check if binding already registered (collision with legacy)
            if is_shortcut_registered(app, binding) {
                collisions.push((workflow.id.clone(), binding.clone()));
                continue; // Skip, legacy wins for now
            }

            // Register with closure that calls workflow execution
            match register_workflow_shortcut(app, &workflow.id, binding, *push_to_talk) {
                Ok(_) => {
                    registered.push(workflow.id.clone());
                    debug!("Registered workflow hotkey: {} -> {}", workflow.name, binding);
                }
                Err(e) => {
                    error!("Failed to register workflow {}: {}", workflow.id, e);
                }
            }
        }
    }

    if !collisions.is_empty() {
        warn!(
            "Workflow hotkey collisions detected (legacy bindings take precedence): {:?}",
            collisions
        );
    }

    registered
}

/// Check if a shortcut is already registered
fn is_shortcut_registered(app: &AppHandle, binding: &str) -> bool {
    match binding.parse::<Shortcut>() {
        Ok(shortcut) => app.global_shortcut().is_registered(shortcut),
        Err(_) => false,
    }
}

/// Register a single workflow shortcut
fn register_workflow_shortcut(
    app: &AppHandle,
    workflow_id: &str,
    binding: &str,
    push_to_talk: bool,
) -> Result<(), String> {
    let shortcut = binding
        .parse::<Shortcut>()
        .map_err(|e| format!("Invalid shortcut: {}", e))?;

    let workflow_id = workflow_id.to_string();
    let app_clone = app.clone();

    app.global_shortcut()
        .on_shortcut(shortcut, move |ah, scut, event| {
            if scut == &shortcut {
                if push_to_talk {
                    if event.state == ShortcutState::Pressed {
                        start_workflow_recording(ah, &workflow_id);
                    } else if event.state == ShortcutState::Released {
                        stop_and_execute_workflow(ah, &workflow_id);
                    }
                } else {
                    if event.state == ShortcutState::Pressed {
                        toggle_workflow_recording(ah, &workflow_id);
                    }
                }
            }
        })
        .map_err(|e| format!("Registration failed: {}", e))?;

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
    let workflow_storage = match app.try_state::<WorkflowStorage>() {
        Some(storage) => storage,
        None => return,
    };

    let workflows = match workflow_storage.list() {
        Ok(wfs) => wfs,
        Err(_) => return,
    };

    for workflow in workflows {
        if let TriggerConfig::Hotkey { binding, .. } = &workflow.trigger {
            if let Ok(shortcut) = binding.parse::<Shortcut>() {
                let _ = app.global_shortcut().unregister(shortcut);
            }
        }
    }
}

/// Start workflow recording (for push-to-talk or toggle mode)
fn start_workflow_recording(app: &AppHandle, workflow_id: &str) {
    debug!("Starting workflow recording: {}", workflow_id);

    let settings = get_settings(app);
    let is_always_on = settings.always_on_microphone;

    change_tray_icon(app, TrayIconState::Recording);
    show_recording_overlay(app);

    let rm = app.state::<Arc<AudioRecordingManager>>();

    // For MVP, use batch (non-streaming) mode
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

/// Stop recording and execute workflow
fn stop_and_execute_workflow(app: &AppHandle, workflow_id: &str) {
    debug!("Stopping workflow recording: {}", workflow_id);

    let ah = app.clone();
    let rm = Arc::clone(&app.state::<Arc<AudioRecordingManager>>());
    let workflow_id = workflow_id.to_string();

    change_tray_icon(app, TrayIconState::Transcribing);
    show_transcribing_overlay(app);
    play_feedback_sound(app, SoundType::Stop);

    // Stop recording and get samples
    tauri::async_runtime::spawn(async move {
        match rm.stop_recording(&workflow_id) {
            Some(samples) => {
                debug!("Got {} samples from recording", samples.len());

                // Execute workflow via WorkflowEngine
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

/// Toggle workflow recording (for non-push-to-talk mode)
fn toggle_workflow_recording(app: &AppHandle, workflow_id: &str) {
    let toggle_state_manager = app.state::<ManagedToggleState>();
    let mut states = toggle_state_manager
        .lock()
        .expect("Failed to lock toggle state manager");

    let is_currently_active = states
        .active_toggles
        .entry(workflow_id.to_string())
        .or_insert(false);

    if *is_currently_active {
        debug!("Toggle: stopping workflow recording");
        drop(states); // Release lock before calling stop
        stop_and_execute_workflow(app, workflow_id);

        // Re-acquire lock to update state
        let mut states = toggle_state_manager
            .lock()
            .expect("Failed to lock toggle state manager");
        states.active_toggles.insert(workflow_id.to_string(), false);
    } else {
        debug!("Toggle: starting workflow recording");
        *is_currently_active = true;
        drop(states); // Release lock before calling start
        start_workflow_recording(app, workflow_id);
    }
}
