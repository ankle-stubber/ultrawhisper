use crate::audio_feedback::{SoundType, play_feedback_sound};
use crate::file_output;
use crate::managers::audio::AudioRecordingManager;
use crate::managers::history::HistoryManager;
use crate::managers::transcription::TranscriptionManager;
use crate::overlay::{show_recording_overlay, show_transcribing_overlay};
use crate::settings::get_settings;
use crate::streaming::queue::create_bounded_queue;
use crate::tray::{change_tray_icon, TrayIconState};
use crate::utils;
use log::{debug, error};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tauri::AppHandle;
use tauri::Manager;

// Shortcut Action Trait
pub trait ShortcutAction: Send + Sync {
    fn start(&self, app: &AppHandle, binding_id: &str, shortcut_str: &str);
    fn stop(&self, app: &AppHandle, binding_id: &str, shortcut_str: &str);
}

// Test Action
struct TestAction;

impl ShortcutAction for TestAction {
    fn start(&self, app: &AppHandle, binding_id: &str, shortcut_str: &str) {
        println!(
            "Shortcut ID '{}': Started - {} (App: {})", // Changed "Pressed" to "Started" for consistency
            binding_id,
            shortcut_str,
            app.package_info().name
        );
    }

    fn stop(&self, app: &AppHandle, binding_id: &str, shortcut_str: &str) {
        println!(
            "Shortcut ID '{}': Stopped - {} (App: {})", // Changed "Released" to "Stopped" for consistency
            binding_id,
            shortcut_str,
            app.package_info().name
        );
    }
}

// Unified Transcribe Action - handles both paste and file save based on binding config
struct UnifiedTranscribeAction;

impl ShortcutAction for UnifiedTranscribeAction {
    fn start(&self, app: &AppHandle, binding_id: &str, _shortcut_str: &str) {
        let start_time = Instant::now();
        debug!("UnifiedTranscribeAction::start called for binding: {}", binding_id);

        // Load model in the background
        let tm = app.state::<Arc<TranscriptionManager>>();
        tm.initiate_model_load();

        let binding_id = binding_id.to_string();
        change_tray_icon(app, TrayIconState::Recording);
        show_recording_overlay(app);

        let rm = app.state::<Arc<AudioRecordingManager>>();

        // Get settings to determine streaming mode
        let settings = get_settings(app);
        let is_always_on = settings.always_on_microphone;
        let streaming_enabled = settings.streaming.enabled;

        debug!(
            "Recording mode - always_on: {}, streaming: {}",
            is_always_on, streaming_enabled
        );

        // Phase 2: Decide between streaming and batch mode
        if streaming_enabled {
            debug!("Starting streaming recording for binding: {}", binding_id);

            // Create chunk queue
            let (chunk_tx, chunk_rx) = create_bounded_queue(settings.streaming.max_queue_size);

            // Start streaming recording
            if let Err(e) = rm.start_streaming_recording(
                &binding_id,
                settings.streaming.chunk_duration_seconds * 1000,
                settings.streaming.overlap_seconds * 1000,
                chunk_tx,
                settings.streaming.backpressure_policy,
            ) {
                error!("Failed to start streaming recording: {}", e);
                return;
            }

            // Spawn workflow engine streaming execution
            let workflow_engine = Arc::clone(&app.state::<Arc<crate::workflow::WorkflowEngine>>());
            let app_clone = app.clone();
            let binding_id_clone = binding_id.clone();

            tauri::async_runtime::spawn(async move {
                debug!("Streaming workflow task started for binding: {}", binding_id_clone);

                match workflow_engine
                    .execute_binding_streaming(&app_clone, &binding_id_clone, chunk_rx)
                    .await
                {
                    Ok(_result) => {
                        debug!("Streaming workflow completed successfully");
                        // Cleanup UI on success
                        utils::hide_recording_overlay(&app_clone);
                        change_tray_icon(&app_clone, TrayIconState::Idle);
                    }
                    Err(e) => {
                        error!("Streaming workflow failed: {}", e);
                        // Cleanup UI even on error
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
            // Batch mode (existing behavior)
            debug!("Starting batch recording for binding: {}", binding_id);

            if is_always_on {
                // Always-on mode: Play audio feedback immediately
                debug!("Always-on mode: Playing audio feedback immediately");
                play_feedback_sound(app, SoundType::Start);
                let recording_started = rm.try_start_recording(&binding_id);
                debug!("Recording started: {}", recording_started);
            } else {
                // On-demand mode: Start recording first, then play audio feedback
                debug!("On-demand mode: Starting recording first, then audio feedback");
                let recording_start_time = Instant::now();
                if rm.try_start_recording(&binding_id) {
                    debug!("Recording started in {:?}", recording_start_time.elapsed());
                    // Small delay to ensure microphone stream is active
                    let app_clone = app.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(100));
                        debug!("Playing delayed audio feedback");
                        play_feedback_sound(&app_clone, SoundType::Start);
                    });
                } else {
                    debug!("Failed to start recording");
                }
            }
        }

        debug!(
            "UnifiedTranscribeAction::start completed in {:?}",
            start_time.elapsed()
        );
    }

    fn stop(&self, app: &AppHandle, binding_id: &str, _shortcut_str: &str) {
        let stop_time = Instant::now();
        debug!("UnifiedTranscribeAction::stop called for binding: {}", binding_id);

        let ah = app.clone();
        let rm = Arc::clone(&app.state::<Arc<AudioRecordingManager>>());
        let tm = Arc::clone(&app.state::<Arc<TranscriptionManager>>());
        let hm = Arc::clone(&app.state::<Arc<HistoryManager>>());

        change_tray_icon(app, TrayIconState::Transcribing);
        show_transcribing_overlay(app);

        // Play audio feedback for recording stop
        play_feedback_sound(app, SoundType::Stop);

        // Get settings to determine mode
        let settings = get_settings(app);
        let binding_config = settings.bindings.get(binding_id).cloned();
        let streaming_enabled = settings.streaming.enabled;

        let binding_id = binding_id.to_string();

        // Phase 2: Check if this was a streaming recording
        if streaming_enabled {
            debug!("Stopping streaming recording for binding: {}", binding_id);

            // Stop streaming recording (this flushes final chunk and closes queue)
            if let Err(e) = rm.stop_streaming_recording(&binding_id) {
                error!("Failed to stop streaming recording: {}", e);
                utils::hide_recording_overlay(&ah);
                change_tray_icon(&ah, TrayIconState::Idle);
                return;
            }

            // The streaming workflow task spawned in start() will complete asynchronously
            // and handle history save, destination routing, and UI cleanup.
            debug!("Streaming recording stopped, workflow will complete asynchronously");
            return;
        }

        // Batch mode (existing behavior)
        tauri::async_runtime::spawn(async move {
            let binding_id = binding_id.clone();
            debug!(
                "Starting async transcription task for binding: {}",
                binding_id
            );

            let stop_recording_time = Instant::now();
            if let Some(samples) = rm.stop_recording(&binding_id) {
                debug!(
                    "Recording stopped and samples retrieved in {:?}, sample count: {}",
                    stop_recording_time.elapsed(),
                    samples.len()
                );

                // PHASE 1: Workflow engine path with short-circuit on success, fallback on error
                if settings.use_workflow_engine {
                    let workflow_engine = Arc::clone(&ah.state::<Arc<crate::workflow::WorkflowEngine>>());
                    let binding_id_clone = binding_id.clone();
                    let samples_for_engine = samples.clone();
                    let ah_clone = ah.clone();

                    debug!("Attempting workflow engine path for binding: {}", binding_id_clone);

                    match workflow_engine.execute_binding(&ah_clone, &binding_id_clone, samples_for_engine).await {
                        Ok(_result) => {
                            debug!("Workflow engine succeeded, short-circuiting legacy path");
                            // Success: short-circuit the legacy path
                            // Still need to do UI cleanup (overlay/tray)
                            utils::hide_recording_overlay(&ah_clone);
                            change_tray_icon(&ah_clone, TrayIconState::Idle);
                            return; // Exit the async task - legacy path will not run
                        }
                        Err(e) => {
                            error!("Workflow engine failed: {}. Falling back to legacy path.", e);
                            // Fall through to legacy path below
                        }
                    }
                }
                // If workflow engine is disabled or failed, continue with legacy transcription

                let transcription_time = Instant::now();
                let samples_clone = samples.clone();
                match tm.transcribe(samples) {
                    Ok(mut transcription) => {
                        debug!(
                            "Transcription completed in {:?}: '{}'",
                            transcription_time.elapsed(),
                            transcription
                        );

                        // Apply text cleaning to legacy transcription
                        let settings = get_settings(&ah);
                        transcription = crate::text_cleaning::clean_text(&transcription, &settings.cleaning);

                        if !transcription.is_empty() {
                            // Save to history with workflow information
                            let hm_clone = Arc::clone(&hm);
                            let transcription_for_history = transcription.clone();

                            // Determine workflow based on binding configuration
                            let (workflow_id, workflow_name) = if let Some(ref binding) = binding_config {
                                if binding.save_to_file {
                                    (Some("save_to_file"), Some("Save To File"))
                                } else {
                                    (Some("quick_transcribe"), Some("Quick Transcribe"))
                                }
                            } else {
                                (Some("quick_transcribe"), Some("Quick Transcribe"))
                            };

                            tauri::async_runtime::spawn(async move {
                                if let Err(e) = hm_clone
                                    .save_transcription(
                                        samples_clone,
                                        transcription_for_history,
                                        workflow_id,
                                        workflow_name,
                                    )
                                    .await
                                {
                                    error!("Failed to save transcription to history: {}", e);
                                }
                            });

                            // Check binding configuration
                            if let Some(binding) = binding_config {
                                let paste_to_window = binding.paste_to_window;
                                let save_to_file = binding.save_to_file;
                                let output_path = binding.output_path.clone();

                                // Save to file if configured
                                if save_to_file {
                                    if let Err(e) = file_output::save_transcription_to_file(
                                        &transcription,
                                        &ah,
                                        output_path
                                    ) {
                                        error!("Failed to save transcription to file: {}", e);
                                    }
                                }

                                // Paste to window if configured
                                if paste_to_window {
                                    let transcription_clone = transcription.clone();
                                    let ah_clone = ah.clone();
                                    let paste_time = Instant::now();
                                    ah.run_on_main_thread(move || {
                                        match utils::paste(transcription_clone, ah_clone.clone()) {
                                            Ok(()) => debug!(
                                                "Text pasted successfully in {:?}",
                                                paste_time.elapsed()
                                            ),
                                            Err(e) => eprintln!("Failed to paste transcription: {}", e),
                                        }
                                        utils::hide_recording_overlay(&ah_clone);
                                        change_tray_icon(&ah_clone, TrayIconState::Idle);
                                    })
                                    .unwrap_or_else(|e| {
                                        eprintln!("Failed to run paste on main thread: {:?}", e);
                                        utils::hide_recording_overlay(&ah);
                                        change_tray_icon(&ah, TrayIconState::Idle);
                                    });
                                } else {
                                    // Just hide overlay if not pasting
                                    utils::hide_recording_overlay(&ah);
                                    change_tray_icon(&ah, TrayIconState::Idle);
                                }
                            } else {
                                // Default behavior if binding not found: paste to window
                                let transcription_clone = transcription.clone();
                                let ah_clone = ah.clone();
                                let paste_time = Instant::now();
                                ah.run_on_main_thread(move || {
                                    match utils::paste(transcription_clone, ah_clone.clone()) {
                                        Ok(()) => debug!(
                                            "Text pasted successfully in {:?}",
                                            paste_time.elapsed()
                                        ),
                                        Err(e) => eprintln!("Failed to paste transcription: {}", e),
                                    }
                                    utils::hide_recording_overlay(&ah_clone);
                                    change_tray_icon(&ah_clone, TrayIconState::Idle);
                                })
                                .unwrap_or_else(|e| {
                                    eprintln!("Failed to run paste on main thread: {:?}", e);
                                    utils::hide_recording_overlay(&ah);
                                    change_tray_icon(&ah, TrayIconState::Idle);
                                });
                            }
                        } else {
                            utils::hide_recording_overlay(&ah);
                            change_tray_icon(&ah, TrayIconState::Idle);
                        }
                    }
                    Err(err) => {
                        debug!("Global Shortcut Transcription error: {}", err);
                        utils::hide_recording_overlay(&ah);
                        change_tray_icon(&ah, TrayIconState::Idle);
                    }
                }
            } else {
                debug!("No samples retrieved from recording stop");
                utils::hide_recording_overlay(&ah);
                change_tray_icon(&ah, TrayIconState::Idle);
            }
        });

        debug!(
            "UnifiedTranscribeAction::stop completed in {:?}",
            stop_time.elapsed()
        );
    }
}

// Static Action Map
pub static ACTION_MAP: Lazy<HashMap<String, Arc<dyn ShortcutAction>>> = Lazy::new(|| {
    let mut map = HashMap::new();
    // Both transcribe and transcribe_to_file use the same unified action
    // The behavior is determined by the binding configuration
    map.insert(
        "transcribe".to_string(),
        Arc::new(UnifiedTranscribeAction) as Arc<dyn ShortcutAction>,
    );
    map.insert(
        "transcribe_to_file".to_string(),
        Arc::new(UnifiedTranscribeAction) as Arc<dyn ShortcutAction>,
    );
    map.insert(
        "test".to_string(),
        Arc::new(TestAction) as Arc<dyn ShortcutAction>,
    );
    map
});
