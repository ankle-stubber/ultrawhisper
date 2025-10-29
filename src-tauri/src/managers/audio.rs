use crate::audio_toolkit::{list_input_devices, vad::SmoothedVad, AudioRecorder, SileroVad};
use crate::settings::get_settings;
use crate::streaming::chunker::{AudioChunk, AudioChunker};
use crate::streaming::queue::{try_send_with_policy, BackpressurePolicy};
use crate::utils;
use log::{debug, info, warn};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::Manager;

const WHISPER_SAMPLE_RATE: usize = 16000;

/* ──────────────────────────────────────────────────────────────── */

#[derive(Clone, Debug)]
pub enum RecordingState {
    Idle,
    Recording {
        binding_id: String,
        is_streaming: bool,
    },
}

#[derive(Clone, Debug)]
pub enum MicrophoneMode {
    AlwaysOn,
    OnDemand,
}

/* ──────────────────────────────────────────────────────────────── */

fn create_audio_recorder(
    vad_path: &str,
    app_handle: &tauri::AppHandle,
) -> Result<AudioRecorder, anyhow::Error> {
    let silero = SileroVad::new(vad_path, 0.3)
        .map_err(|e| anyhow::anyhow!("Failed to create SileroVad: {}", e))?;
    let smoothed_vad = SmoothedVad::new(Box::new(silero), 15, 15, 2);

    // Recorder with VAD plus a spectrum-level callback that forwards updates to
    // the frontend.
    let recorder = AudioRecorder::new()
        .map_err(|e| anyhow::anyhow!("Failed to create AudioRecorder: {}", e))?
        .with_vad(Box::new(smoothed_vad))
        .with_level_callback({
            let app_handle = app_handle.clone();
            move |levels| {
                utils::emit_levels(&app_handle, &levels);
            }
        });

    Ok(recorder)
}

/* ──────────────────────────────────────────────────────────────── */

#[derive(Clone)]
pub struct AudioRecordingManager {
    state: Arc<Mutex<RecordingState>>,
    mode: Arc<Mutex<MicrophoneMode>>,
    app_handle: tauri::AppHandle,

    recorder: Arc<Mutex<Option<AudioRecorder>>>,
    is_open: Arc<Mutex<bool>>,
    is_recording: Arc<Mutex<bool>>,
}

impl AudioRecordingManager {
    /* ---------- construction ------------------------------------------------ */

    pub fn new(app: &tauri::AppHandle) -> Result<Self, anyhow::Error> {
        let settings = get_settings(app);
        let mode = if settings.always_on_microphone {
            MicrophoneMode::AlwaysOn
        } else {
            MicrophoneMode::OnDemand
        };

        let manager = Self {
            state: Arc::new(Mutex::new(RecordingState::Idle)),
            mode: Arc::new(Mutex::new(mode.clone())),
            app_handle: app.clone(),

            recorder: Arc::new(Mutex::new(None)),
            is_open: Arc::new(Mutex::new(false)),
            is_recording: Arc::new(Mutex::new(false)),
        };

        // Always-on?  Open immediately.
        if matches!(mode, MicrophoneMode::AlwaysOn) {
            manager.start_microphone_stream()?;
        }

        Ok(manager)
    }

    /* ---------- microphone life-cycle -------------------------------------- */

    pub fn start_microphone_stream(&self) -> Result<(), anyhow::Error> {
        let mut open_flag = self.is_open.lock().unwrap();
        if *open_flag {
            debug!("Microphone stream already active");
            return Ok(());
        }

        let start_time = Instant::now();

        let vad_path = self
            .app_handle
            .path()
            .resolve(
                "resources/models/silero_vad_v4.onnx",
                tauri::path::BaseDirectory::Resource,
            )
            .map_err(|e| anyhow::anyhow!("Failed to resolve VAD path: {}", e))?;
        let mut recorder_opt = self.recorder.lock().unwrap();

        if recorder_opt.is_none() {
            *recorder_opt = Some(create_audio_recorder(
                vad_path.to_str().unwrap(),
                &self.app_handle,
            )?);
        }

        // Get the selected device from settings
        let settings = get_settings(&self.app_handle);
        let selected_device = if let Some(device_name) = settings.selected_microphone {
            // Find the device by name
            match list_input_devices() {
                Ok(devices) => devices
                    .into_iter()
                    .find(|d| d.name == device_name)
                    .map(|d| d.device),
                Err(e) => {
                    debug!("Failed to list devices, using default: {}", e);
                    None
                }
            }
        } else {
            None
        };

        if let Some(rec) = recorder_opt.as_mut() {
            rec.open(selected_device)
                .map_err(|e| anyhow::anyhow!("Failed to open recorder: {}", e))?;
        }

        *open_flag = true;
        info!(
            "Microphone stream initialized in {:?}",
            start_time.elapsed()
        );
        Ok(())
    }

    pub fn stop_microphone_stream(&self) {
        let mut open_flag = self.is_open.lock().unwrap();
        if !*open_flag {
            return;
        }

        if let Some(rec) = self.recorder.lock().unwrap().as_mut() {
            // If still recording, stop first.
            if *self.is_recording.lock().unwrap() {
                let _ = rec.stop();
                *self.is_recording.lock().unwrap() = false;
            }
            let _ = rec.close();
        }

        *open_flag = false;
        debug!("Microphone stream stopped");
    }

    /* ---------- mode switching --------------------------------------------- */

    pub fn update_mode(&self, new_mode: MicrophoneMode) -> Result<(), anyhow::Error> {
        let mode_guard = self.mode.lock().unwrap();
        let cur_mode = mode_guard.clone();

        match (cur_mode, &new_mode) {
            (MicrophoneMode::AlwaysOn, MicrophoneMode::OnDemand) => {
                if matches!(*self.state.lock().unwrap(), RecordingState::Idle) {
                    drop(mode_guard);
                    self.stop_microphone_stream();
                }
            }
            (MicrophoneMode::OnDemand, MicrophoneMode::AlwaysOn) => {
                drop(mode_guard);
                self.start_microphone_stream()?;
            }
            _ => {}
        }

        *self.mode.lock().unwrap() = new_mode;
        Ok(())
    }

    /* ---------- recording --------------------------------------------------- */

    pub fn try_start_recording(&self, binding_id: &str) -> bool {
        let mut state = self.state.lock().unwrap();

        if let RecordingState::Idle = *state {
            // Ensure microphone is open in on-demand mode
            if matches!(*self.mode.lock().unwrap(), MicrophoneMode::OnDemand) {
                if let Err(e) = self.start_microphone_stream() {
                    eprintln!("Failed to open microphone stream: {e}");
                    return false;
                }
            }

            if let Some(rec) = self.recorder.lock().unwrap().as_ref() {
                if rec.start().is_ok() {
                    *self.is_recording.lock().unwrap() = true;
                    *state = RecordingState::Recording {
                        binding_id: binding_id.to_string(),
                        is_streaming: false,
                    };
                    debug!("Recording started for binding {binding_id}");
                    return true;
                }
            }
            eprintln!("Recorder not available");
            false
        } else {
            false
        }
    }

    pub fn update_selected_device(&self) -> Result<(), anyhow::Error> {
        // If currently open, restart the microphone stream to use the new device
        if *self.is_open.lock().unwrap() {
            self.stop_microphone_stream();
            self.start_microphone_stream()?;
        }
        Ok(())
    }

    pub fn stop_recording(&self, binding_id: &str) -> Option<Vec<f32>> {
        let mut state = self.state.lock().unwrap();

        match &*state {
            RecordingState::Recording {
                binding_id: ref active,
                is_streaming,
            } if active == binding_id && !is_streaming => {
                *state = RecordingState::Idle;
                drop(state);

                let samples = if let Some(rec) = self.recorder.lock().unwrap().as_ref() {
                    match rec.stop() {
                        Ok(buf) => buf,
                        Err(e) => {
                            eprintln!("stop() failed: {e}");
                            Vec::new()
                        }
                    }
                } else {
                    eprintln!("Recorder not available");
                    Vec::new()
                };

                *self.is_recording.lock().unwrap() = false;

                // In on-demand mode turn the mic off again
                if matches!(*self.mode.lock().unwrap(), MicrophoneMode::OnDemand) {
                    self.stop_microphone_stream();
                }

                // Pad if very short
                let s_len = samples.len();
                // println!("Got {} samples", { s_len });
                if s_len < WHISPER_SAMPLE_RATE && s_len > 0 {
                    let mut padded = samples;
                    padded.resize(WHISPER_SAMPLE_RATE * 5 / 4, 0.0);
                    Some(padded)
                } else {
                    Some(samples)
                }
            }
            _ => None,
        }
    }

    /// Cancel any ongoing recording without returning audio samples
    pub fn cancel_recording(&self) {
        let mut state = self.state.lock().unwrap();

        if matches!(*state, RecordingState::Recording { .. }) {
            *state = RecordingState::Idle;
            drop(state);

            if let Some(rec) = self.recorder.lock().unwrap().as_ref() {
                let _ = rec.stop(); // Discard the result
            }

            *self.is_recording.lock().unwrap() = false;

            // In on-demand mode turn the mic off again
            if matches!(*self.mode.lock().unwrap(), MicrophoneMode::OnDemand) {
                self.stop_microphone_stream();
            }
        }
    }

    /// Start streaming recording - samples are chunked and sent to the provided channel
    ///
    /// This method is for Phase 2 streaming transcription. It:
    /// 1. Starts recording with AudioRecorder in streaming mode
    /// 2. Creates a consumer task that chunks samples
    /// 3. Sends chunks to the provided tokio channel
    ///
    /// # Arguments
    /// * `binding_id` - The binding ID for this recording
    /// * `chunk_duration_ms` - Duration of each chunk in milliseconds
    /// * `overlap_duration_ms` - Overlap duration in milliseconds
    /// * `chunk_sender` - Tokio channel to send audio chunks to
    /// * `backpressure_policy` - Policy for handling full queue
    pub fn start_streaming_recording(
        &self,
        binding_id: &str,
        chunk_duration_ms: u32,
        overlap_duration_ms: u32,
        chunk_sender: tokio::sync::mpsc::Sender<AudioChunk>,
        backpressure_policy: BackpressurePolicy,
    ) -> Result<(), anyhow::Error> {
        let mut state = self.state.lock().unwrap();

        if !matches!(*state, RecordingState::Idle) {
            return Err(anyhow::anyhow!("Already recording"));
        }

        // Ensure microphone is open in on-demand mode
        if matches!(*self.mode.lock().unwrap(), MicrophoneMode::OnDemand) {
            drop(state);
            self.start_microphone_stream()?;
            state = self.state.lock().unwrap();
        }

        // Create std::sync::mpsc channel for AudioRecorder to send samples
        let (sample_tx, sample_rx) = std::sync::mpsc::channel::<Vec<f32>>();

        // Start AudioRecorder in streaming mode
        if let Some(rec) = self.recorder.lock().unwrap().as_ref() {
            rec.start_streaming(sample_tx)
                .map_err(|e| anyhow::anyhow!("Failed to start streaming: {}", e))?;

            *self.is_recording.lock().unwrap() = true;
            *state = RecordingState::Recording {
                binding_id: binding_id.to_string(),
                is_streaming: true,
            };

            debug!(
                "Streaming recording started for binding {} (chunk: {}ms, overlap: {}ms)",
                binding_id, chunk_duration_ms, overlap_duration_ms
            );

            // Spawn consumer task for chunking (runs in background, not on CPAL thread)
            tauri::async_runtime::spawn(async move {
                let mut chunker = AudioChunker::new(
                    chunk_duration_ms,
                    overlap_duration_ms,
                    WHISPER_SAMPLE_RATE as u32,
                );

                debug!("Chunker initialized, starting to process samples");

                // Process samples as they arrive
                while let Ok(samples) = sample_rx.recv() {
                    if let Some(chunk) = chunker.add_samples(&samples) {
                        // Try to send chunk with backpressure policy
                        let result = try_send_with_policy(&chunk_sender, chunk.clone(), backpressure_policy);

                        match result {
                            crate::streaming::queue::SendResult::Sent => {
                                // Success - chunk queued
                            }
                            crate::streaming::queue::SendResult::DroppedNewest => {
                                warn!(
                                    "Chunk queue full, dropped newest chunk (policy: {:?})",
                                    backpressure_policy
                                );
                            }
                            crate::streaming::queue::SendResult::WouldBlock => {
                                // Block policy - try blocking send
                                if chunk_sender.send(chunk).await.is_err() {
                                    warn!("Failed to send chunk - receiver closed");
                                    break;
                                }
                            }
                        }
                    }
                }

                debug!("Sample stream closed, flushing remaining audio");

                // Flush remaining samples when channel closes
                if let Some(final_chunk) = chunker.flush_remaining() {
                    let _ = chunk_sender.send(final_chunk).await;
                    debug!("Final chunk flushed");
                }

                debug!("Streaming consumer task completed");
            });

            Ok(())
        } else {
            Err(anyhow::anyhow!("Recorder not available"))
        }
    }

    /// Stop streaming recording - no samples are returned
    pub fn stop_streaming_recording(&self, binding_id: &str) -> Result<(), anyhow::Error> {
        let mut state = self.state.lock().unwrap();

        match &*state {
            RecordingState::Recording {
                binding_id: ref active,
                is_streaming,
            } if active == binding_id && *is_streaming => {
                *state = RecordingState::Idle;
                drop(state);

                if let Some(rec) = self.recorder.lock().unwrap().as_ref() {
                    rec.stop_streaming()
                        .map_err(|e| anyhow::anyhow!("Failed to stop streaming: {}", e))?;
                }

                *self.is_recording.lock().unwrap() = false;

                // In on-demand mode turn the mic off again
                if matches!(*self.mode.lock().unwrap(), MicrophoneMode::OnDemand) {
                    self.stop_microphone_stream();
                }

                debug!("Streaming recording stopped for binding {}", binding_id);
                Ok(())
            }
            _ => Err(anyhow::anyhow!("Not recording or binding mismatch")),
        }
    }
}
