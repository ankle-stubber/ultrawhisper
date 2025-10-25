use crate::audio_toolkit::audio::load_wav_file;
use crate::managers::transcription::TranscriptionManager;
use crate::settings::get_settings;
use anyhow::{Context, Result};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};
use tauri::{AppHandle, Emitter};

/// Represents a single processed file entry in the tracking database
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProcessedFileEntry {
    pub path: String,
    pub processed_at: u64, // Unix timestamp in seconds
    pub size: u64,
    pub modified_time: u64, // Unix timestamp in seconds
    pub output_file: String,
    pub success: bool,
    pub error: Option<String>,
}

/// Tracks which files have been processed in a watch folder
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProcessedFilesDb {
    pub version: u32,
    pub processed_files: Vec<ProcessedFileEntry>,
}

impl ProcessedFilesDb {
    fn new() -> Self {
        Self {
            version: 1,
            processed_files: Vec::new(),
        }
    }

    /// Check if a file has already been processed
    fn contains(&self, file_path: &Path) -> bool {
        let path_str = file_path.to_string_lossy().to_string();
        self.processed_files
            .iter()
            .any(|entry| entry.path == path_str && entry.success)
    }

    /// Add a processed file entry
    fn add_entry(&mut self, entry: ProcessedFileEntry) {
        self.processed_files.push(entry);
    }

    /// Load from JSON file
    fn load_from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let db: ProcessedFilesDb = serde_json::from_str(&content)?;
        Ok(db)
    }

    /// Save to JSON file
    fn save_to_file(&self, path: &Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
}

/// Event emitted when a batch processing cycle completes
#[derive(Serialize, Clone, Debug)]
pub struct BatchCompleteEvent {
    pub processed: usize,
    pub failed: usize,
    pub timestamp: u64,
}

/// Result of processing a single file
#[derive(Debug)]
struct FileProcessingResult {
    success: bool,
    output_file: Option<PathBuf>,
    error: Option<String>,
}

/// Manager for batch transcription of audio files in watched folders
pub struct BatchTranscriptionManager {
    app_handle: AppHandle,
    transcription_manager: Arc<TranscriptionManager>,
    shutdown_signal: Arc<AtomicBool>,
    watcher_handle: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
}

impl BatchTranscriptionManager {
    pub fn new(
        app_handle: &AppHandle,
        transcription_manager: Arc<TranscriptionManager>,
    ) -> Result<Self> {
        let manager = Self {
            app_handle: app_handle.clone(),
            transcription_manager,
            shutdown_signal: Arc::new(AtomicBool::new(false)),
            watcher_handle: Arc::new(Mutex::new(None)),
        };

        // Start the scheduled watcher thread
        {
            let app_handle_cloned = app_handle.clone();
            let manager_cloned = manager.clone_for_thread();
            let shutdown_signal = manager.shutdown_signal.clone();

            let handle = thread::spawn(move || {
                debug!("Batch transcription watcher thread started");

                while !shutdown_signal.load(Ordering::Relaxed) {
                    // Get current interval from settings
                    let settings = get_settings(&app_handle_cloned);
                    let interval_secs = settings.batch_transcription.check_interval_seconds;

                    thread::sleep(Duration::from_secs(interval_secs));

                    // Check shutdown signal again after sleep
                    if shutdown_signal.load(Ordering::Relaxed) {
                        break;
                    }

                    // Check if batch processing is enabled
                    let settings = get_settings(&app_handle_cloned);
                    if !settings.batch_transcription.enabled {
                        continue;
                    }

                    // Scan and process folders
                    if let Err(e) = manager_cloned.scan_and_process_folders() {
                        error!("Error during batch processing: {}", e);
                    }
                }

                debug!("Batch transcription watcher thread shutting down gracefully");
            });

            *manager.watcher_handle.lock().unwrap() = Some(handle);
        }

        Ok(manager)
    }

    /// Clone data needed for the background thread
    fn clone_for_thread(&self) -> BatchTranscriptionManagerThread {
        BatchTranscriptionManagerThread {
            app_handle: self.app_handle.clone(),
            transcription_manager: self.transcription_manager.clone(),
        }
    }

    /// Manually trigger batch processing (for "Process Now" button)
    pub fn process_now(&self) -> Result<BatchCompleteEvent> {
        let thread_manager = self.clone_for_thread();
        thread_manager.scan_and_process_folders()
    }
}

/// Thread-safe subset of BatchTranscriptionManager
struct BatchTranscriptionManagerThread {
    app_handle: AppHandle,
    transcription_manager: Arc<TranscriptionManager>,
}

impl BatchTranscriptionManagerThread {
    /// Main processing loop - scans all configured folders and processes new files
    fn scan_and_process_folders(&self) -> Result<BatchCompleteEvent> {
        let settings = get_settings(&self.app_handle);
        let batch_settings = &settings.batch_transcription;

        if !batch_settings.enabled {
            return Ok(BatchCompleteEvent {
                processed: 0,
                failed: 0,
                timestamp: SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            });
        }

        let mut total_processed = 0;
        let mut total_failed = 0;

        // Process each configured watch folder
        for folder_path_str in &batch_settings.watch_folders {
            let folder_path = PathBuf::from(folder_path_str);

            if !folder_path.exists() {
                warn!("Watch folder does not exist: {:?}", folder_path);
                continue;
            }

            if !folder_path.is_dir() {
                warn!("Watch folder is not a directory: {:?}", folder_path);
                continue;
            }

            match self.process_folder(&folder_path) {
                Ok((processed, failed)) => {
                    total_processed += processed;
                    total_failed += failed;
                }
                Err(e) => {
                    error!("Error processing folder {:?}: {}", folder_path, e);
                }
            }
        }

        let event = BatchCompleteEvent {
            processed: total_processed,
            failed: total_failed,
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        // Emit batch complete event
        let _ = self.app_handle.emit("batch-complete", event.clone());

        info!(
            "Batch processing complete: {} processed, {} failed",
            total_processed, total_failed
        );

        Ok(event)
    }

    /// Process a single watch folder
    fn process_folder(&self, folder_path: &Path) -> Result<(usize, usize)> {
        debug!("Processing folder: {:?}", folder_path);

        let settings = get_settings(&self.app_handle);
        let batch_settings = &settings.batch_transcription;

        // Load or create processed files database
        let db_path = folder_path.join("processed_files.json");
        let mut processed_db = if db_path.exists() {
            ProcessedFilesDb::load_from_file(&db_path).unwrap_or_else(|e| {
                warn!("Failed to load processed files database: {}. Creating new one.", e);
                ProcessedFilesDb::new()
            })
        } else {
            ProcessedFilesDb::new()
        };

        // Find all WAV files in the folder
        let wav_files = self.find_wav_files(folder_path)?;

        // Filter files that need processing
        let files_to_process: Vec<PathBuf> = wav_files
            .into_iter()
            .filter(|path| {
                // Skip if already processed
                if processed_db.contains(path) {
                    return false;
                }

                // Skip if output file already exists
                let output_path = self.get_output_path(path, &batch_settings.output_suffix);
                if output_path.exists() {
                    return false;
                }

                // Check file stability
                match self.is_file_ready(path, batch_settings.stability_timeout_seconds) {
                    Ok(ready) => ready,
                    Err(e) => {
                        warn!("Error checking file readiness for {:?}: {}", path, e);
                        false
                    }
                }
            })
            .collect();

        if files_to_process.is_empty() {
            debug!("No new files to process in {:?}", folder_path);
            return Ok((0, 0));
        }

        info!(
            "Found {} files to process in {:?}",
            files_to_process.len(),
            folder_path
        );

        // Initiate model loading before processing batch
        self.transcription_manager.initiate_model_load();

        let mut processed = 0;
        let mut failed = 0;

        // Process each file sequentially
        for file_path in files_to_process {
            match self.process_file(&file_path) {
                Ok(result) => {
                    // Create processed file entry
                    let metadata = fs::metadata(&file_path)?;
                    let entry = ProcessedFileEntry {
                        path: file_path.to_string_lossy().to_string(),
                        processed_at: SystemTime::now()
                            .duration_since(SystemTime::UNIX_EPOCH)?
                            .as_secs(),
                        size: metadata.len(),
                        modified_time: metadata
                            .modified()?
                            .duration_since(SystemTime::UNIX_EPOCH)?
                            .as_secs(),
                        output_file: result
                            .output_file
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_default(),
                        success: result.success,
                        error: result.error,
                    };

                    processed_db.add_entry(entry);

                    if result.success {
                        processed += 1;
                        info!("Successfully processed: {:?}", file_path);
                    } else {
                        failed += 1;
                        error!("Failed to process: {:?}", file_path);
                    }
                }
                Err(e) => {
                    error!("Error processing file {:?}: {}", file_path, e);
                    failed += 1;

                    // Still add to database to avoid retrying on every scan
                    let metadata = fs::metadata(&file_path).ok();
                    let now = SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    let entry = ProcessedFileEntry {
                        path: file_path.to_string_lossy().to_string(),
                        processed_at: now,
                        size: metadata.map(|m| m.len()).unwrap_or(0),
                        modified_time: now,
                        output_file: String::new(),
                        success: false,
                        error: Some(e.to_string()),
                    };
                    processed_db.add_entry(entry);
                }
            }
        }

        // Save updated processed files database
        processed_db.save_to_file(&db_path)?;

        Ok((processed, failed))
    }

    /// Find all WAV files in a folder (non-recursive)
    fn find_wav_files(&self, folder_path: &Path) -> Result<Vec<PathBuf>> {
        let settings = get_settings(&self.app_handle);
        let batch_settings = &settings.batch_transcription;

        let mut wav_files = Vec::new();

        for entry in fs::read_dir(folder_path)? {
            let entry = entry?;
            let path = entry.path();

            // Skip directories
            if path.is_dir() {
                continue;
            }

            // Skip hidden files
            if let Some(file_name) = path.file_name() {
                if file_name.to_string_lossy().starts_with('.') {
                    continue;
                }
            }

            // Check if it's a WAV file
            if let Some(ext) = path.extension() {
                if ext.eq_ignore_ascii_case("wav") {
                    // Skip if it looks like an output file
                    if let Some(stem) = path.file_stem() {
                        if stem
                            .to_string_lossy()
                            .ends_with(&batch_settings.output_suffix)
                        {
                            continue;
                        }
                    }

                    wav_files.push(path);
                }
            }
        }

        Ok(wav_files)
    }

    /// Check if a file is ready for processing (stable and not being written)
    fn is_file_ready(&self, path: &Path, stability_timeout_secs: u64) -> Result<bool> {
        let metadata = fs::metadata(path)?;
        let mtime = metadata.modified()?;
        let age = SystemTime::now().duration_since(mtime)?;

        // File must be older than stability timeout
        if age.as_secs() < stability_timeout_secs {
            return Ok(false);
        }

        // Quick size validation - check if size changes
        let size_before = metadata.len();
        thread::sleep(Duration::from_secs(1));
        let size_after = fs::metadata(path)?.len();

        Ok(size_before == size_after)
    }

    /// Process a single audio file
    fn process_file(&self, file_path: &Path) -> Result<FileProcessingResult> {
        info!("Processing file: {:?}", file_path);

        let settings = get_settings(&self.app_handle);
        let batch_settings = &settings.batch_transcription;

        // Validate file size
        let metadata = fs::metadata(file_path)?;
        let size_kb = metadata.len() / 1024;
        let size_mb = size_kb / 1024;

        if size_kb < batch_settings.min_file_size_kb {
            return Ok(FileProcessingResult {
                success: false,
                output_file: None,
                error: Some(format!(
                    "File too small: {} KB (minimum: {} KB)",
                    size_kb, batch_settings.min_file_size_kb
                )),
            });
        }

        if size_mb > batch_settings.max_file_size_mb {
            return Ok(FileProcessingResult {
                success: false,
                output_file: None,
                error: Some(format!(
                    "File too large: {} MB (maximum: {} MB)",
                    size_mb, batch_settings.max_file_size_mb
                )),
            });
        }

        // Load audio from WAV file
        let audio_samples = load_wav_file(file_path).context("Failed to load WAV file")?;

        if audio_samples.is_empty() {
            return Ok(FileProcessingResult {
                success: false,
                output_file: None,
                error: Some("Audio file is empty".to_string()),
            });
        }

        // Calculate duration
        let duration_secs = audio_samples.len() as f64 / 16000.0;
        let duration_str = format!("{}:{:02}", (duration_secs / 60.0) as u32, (duration_secs % 60.0) as u32);

        // Transcribe audio
        let transcription_start = std::time::Instant::now();
        let transcription_text = self
            .transcription_manager
            .transcribe(audio_samples)
            .context("Failed to transcribe audio")?;
        let transcription_duration = transcription_start.elapsed();

        // Get output path
        let output_path = self.get_output_path(file_path, &batch_settings.output_suffix);

        // Generate markdown content
        let markdown_content = self.generate_markdown_output(
            file_path,
            &transcription_text,
            &duration_str,
            size_mb,
            transcription_duration.as_secs_f64(),
        );

        // Write output file
        fs::write(&output_path, markdown_content)
            .context("Failed to write output file")?;

        // Optionally delete the original file
        if batch_settings.delete_after_transcription {
            fs::remove_file(file_path).context("Failed to delete original file")?;
            info!("Deleted original file: {:?}", file_path);
        }

        Ok(FileProcessingResult {
            success: true,
            output_file: Some(output_path),
            error: None,
        })
    }

    /// Get output file path for a given input file
    fn get_output_path(&self, input_path: &Path, output_suffix: &str) -> PathBuf {
        let file_stem = input_path.file_stem().unwrap().to_string_lossy();
        let output_name = format!("{}{}.md", file_stem, output_suffix);
        input_path.with_file_name(output_name)
    }

    /// Generate markdown content for output file
    fn generate_markdown_output(
        &self,
        file_path: &Path,
        transcription: &str,
        duration: &str,
        size_mb: u64,
        processing_time: f64,
    ) -> String {
        let file_name = file_path.file_name().unwrap().to_string_lossy();

        // Format current time as YYYY-MM-DD HH:MM:SS
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let datetime = std::time::UNIX_EPOCH + std::time::Duration::from_secs(now);

        // Simple timestamp formatting without chrono
        let timestamp = format!("{:?}", datetime); // Basic formatting for v1

        let settings = get_settings(&self.app_handle);
        let model_name = settings.selected_model.clone();

        format!(
            r#"# Transcription: {}

**Date Processed:** {}
**Original File:** {}
**Duration:** {}
**File Size:** {} MB
**Model:** {}
**Processing Time:** {:.1} seconds

---

{}

---

*Generated by UltraWhisper Batch Processor*
"#,
            file_name,
            timestamp,
            file_name,
            duration,
            size_mb,
            model_name,
            processing_time,
            transcription
        )
    }
}

impl Drop for BatchTranscriptionManager {
    fn drop(&mut self) {
        debug!("Shutting down BatchTranscriptionManager");

        // Signal the watcher thread to shutdown
        self.shutdown_signal.store(true, Ordering::Relaxed);

        // Wait for the thread to finish gracefully
        if let Some(handle) = self.watcher_handle.lock().unwrap().take() {
            if let Err(e) = handle.join() {
                error!("Failed to join batch watcher thread: {:?}", e);
            } else {
                debug!("Batch watcher thread joined successfully");
            }
        }
    }
}
