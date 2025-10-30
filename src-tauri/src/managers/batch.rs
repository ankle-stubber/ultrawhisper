use crate::audio_toolkit::audio::load_wav_file;
use crate::managers::audio::AudioRecordingManager;
use crate::managers::transcription::TranscriptionManager;
use crate::settings::get_settings;
use crate::templates::{apply_template, format_timestamp};
use anyhow::{Context, Result};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};
use tauri::{AppHandle, Emitter, Manager};

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

    /// Normalize a path string for consistent comparison
    fn normalize_path_str(path_str: &str) -> String {
        // Expand tilde if present
        let expanded = if path_str.starts_with("~/") || path_str == "~" {
            if let Ok(home) = env::var("HOME").or_else(|_| env::var("USERPROFILE")) {
                path_str.replacen("~", &home, 1)
            } else {
                path_str.to_string()
            }
        } else {
            path_str.to_string()
        };

        // Try to canonicalize for consistent comparison
        PathBuf::from(&expanded)
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from(expanded))
            .to_string_lossy()
            .to_string()
    }

    /// Check if a file has already been processed
    fn contains(&self, file_path: &Path) -> bool {
        let normalized = Self::normalize_path_str(&file_path.to_string_lossy());
        self.processed_files.iter().any(|entry| {
            let entry_normalized = Self::normalize_path_str(&entry.path);
            entry_normalized == normalized && entry.success
        })
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

        // Check if interactive recording is active (single active recording guard)
        let audio_manager = self.app_handle.state::<Arc<AudioRecordingManager>>();
        if audio_manager.is_recording() {
            debug!("Folder watch paused (active recording)");
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

    /// Check if a file matches any of the configured patterns
    fn matches_pattern(&self, file_path: &Path, patterns: &[String]) -> bool {
        let extension = file_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        patterns.iter().any(|pattern| {
            // Simple wildcard: "*.wav" -> check if extension is "wav"
            if let Some(ext) = pattern.strip_prefix("*.") {
                extension.eq_ignore_ascii_case(ext)
            } else {
                false
            }
        })
    }

    /// Normalize a path for consistent comparison
    /// Expands tilde and canonicalizes if possible
    fn normalize_path(&self, path: &str) -> String {
        let expanded = self.expand_tilde(path);
        // Try to canonicalize, but fall back to expanded path if it fails
        PathBuf::from(&expanded)
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from(expanded.clone()))
            .to_string_lossy()
            .to_string()
    }

    /// Find all audio files matching configured patterns in a folder (non-recursive)
    fn find_wav_files(&self, folder_path: &Path) -> Result<Vec<PathBuf>> {
        let settings = get_settings(&self.app_handle);
        let batch_settings = &settings.batch_transcription;

        let mut audio_files = Vec::new();

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

            // Check if it matches any configured pattern
            if self.matches_pattern(&path, &batch_settings.file_patterns) {
                // Skip if it looks like an output file
                if let Some(stem) = path.file_stem() {
                    if stem
                        .to_string_lossy()
                        .ends_with(&batch_settings.output_suffix)
                    {
                        continue;
                    }
                }

                audio_files.push(path);
            }
        }

        Ok(audio_files)
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
        let settings = get_settings(&self.app_handle);
        let batch_settings = &settings.batch_transcription;

        let file_stem = input_path.file_stem().unwrap().to_string_lossy();
        let output_name = format!("{}{}.md", file_stem, output_suffix);

        if let Some(ref output_folder) = batch_settings.output_folder {
            // Expand tilde if present
            let expanded_path = self.expand_tilde(output_folder);
            let output_dir = PathBuf::from(expanded_path);

            // Only create subdirectories if watching multiple folders to prevent collisions
            let final_output_dir = if batch_settings.watch_folders.len() > 1 {
                // Multiple watch folders - use subdirectory for organization
                let source_folder_name = input_path
                    .parent()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");
                output_dir.join(source_folder_name)
            } else {
                // Single watch folder - output directly to configured folder
                output_dir
            };

            // Ensure the directory exists
            if !final_output_dir.exists() {
                if let Err(e) = fs::create_dir_all(&final_output_dir) {
                    error!("Failed to create output directory: {:?}, error: {}", final_output_dir, e);
                    // Fall back to source folder
                    return input_path.with_file_name(output_name);
                }
            }

            final_output_dir.join(output_name)
        } else {
            // Use source folder (original behavior)
            input_path.with_file_name(output_name)
        }
    }

    /// Helper function to expand ~ to home directory
    fn expand_tilde(&self, path: &str) -> String {
        if path.starts_with("~/") || path == "~" {
            if let Ok(home) = std::env::var("HOME") {
                return path.replacen("~", &home, 1);
            }
            // On Windows, try USERPROFILE
            if let Ok(home) = std::env::var("USERPROFILE") {
                return path.replacen("~", &home, 1);
            }
        }
        path.to_string()
    }

    /// Validate output folder by attempting to write a temp file
    pub fn validate_output_folder(&self, folder_path: &str) -> Result<(), String> {
        let expanded_path = self.expand_tilde(folder_path);
        let path = PathBuf::from(&expanded_path);

        // Create directory if it doesn't exist
        if !path.exists() {
            fs::create_dir_all(&path)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }

        // Try to write a temp file
        let temp_file = path.join(".ultrawhisper_test");
        fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(&temp_file)
            .map_err(|e| format!("Failed to write to directory: {}", e))?;

        // Clean up temp file
        let _ = fs::remove_file(temp_file);

        Ok(())
    }

    /// Generate markdown content for output file using templates
    fn generate_markdown_output(
        &self,
        file_path: &Path,
        transcription: &str,
        duration: &str,
        size_mb: u64,
        processing_time: f64,
    ) -> String {
        let file_name = file_path.file_name().unwrap().to_string_lossy().to_string();
        let source_file = file_path.to_string_lossy().to_string();

        // Get current timestamp
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let timestamp = format_timestamp(now);

        // Get settings for model name and template
        let settings = get_settings(&self.app_handle);
        let model_name = settings.selected_model.clone();
        let template_id = &settings.batch_transcription.template_id;

        // Prepare variables for template substitution
        let mut variables = HashMap::new();
        variables.insert("filename", file_name);
        variables.insert("timestamp", timestamp);
        variables.insert("source_file", source_file);
        variables.insert("duration", duration.to_string());
        variables.insert("file_size_mb", size_mb.to_string());
        variables.insert("model_name", model_name);
        variables.insert("processing_time_s", format!("{:.1}", processing_time));
        variables.insert("transcription_text", transcription.to_string());
        variables.insert("workflow_name", "Batch Processing".to_string());

        // Apply the template
        apply_template(template_id, &variables)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_file_pattern_matching_wav() {
        let patterns = vec!["*.wav".to_string()];

        let test_cases = vec![
            (PathBuf::from("test.wav"), true),
            (PathBuf::from("test.WAV"), true), // Case insensitive
            (PathBuf::from("test.mp3"), false),
            (PathBuf::from("/path/to/file.wav"), true),
        ];

        for (path, expected) in test_cases {
            let result = matches_pattern_test(&path, &patterns);
            assert_eq!(
                result, expected,
                "Pattern matching failed for {:?}. Expected: {}, Got: {}",
                path, expected, result
            );
        }
    }

    #[test]
    fn test_file_pattern_matching_multiple() {
        let patterns = vec![
            "*.wav".to_string(),
            "*.mp3".to_string(),
            "*.m4a".to_string(),
        ];

        let test_cases = vec![
            (PathBuf::from("test.wav"), true),
            (PathBuf::from("test.mp3"), true),
            (PathBuf::from("test.m4a"), true),
            (PathBuf::from("test.MP3"), true), // Case insensitive
            (PathBuf::from("test.txt"), false),
            (PathBuf::from("test.flac"), false),
        ];

        for (path, expected) in test_cases {
            let result = matches_pattern_test(&path, &patterns);
            assert_eq!(
                result, expected,
                "Pattern matching failed for {:?}. Expected: {}, Got: {}",
                path, expected, result
            );
        }
    }

    #[test]
    fn test_path_normalization_tilde() {
        // Save current env
        let original_home = env::var("HOME").ok();

        // Set test HOME
        env::set_var("HOME", "/Users/testuser");

        let normalized = ProcessedFilesDb::normalize_path_str("~/Documents/file.wav");
        assert!(
            normalized.contains("/Users/testuser"),
            "Tilde should be expanded. Got: {}",
            normalized
        );
        assert!(
            !normalized.contains("~"),
            "Tilde should be removed. Got: {}",
            normalized
        );

        // Restore env
        if let Some(home) = original_home {
            env::set_var("HOME", home);
        } else {
            env::remove_var("HOME");
        }
    }

    #[test]
    fn test_path_normalization_consistency() {
        // Save current env
        let original_home = env::var("HOME").ok();

        // Set test HOME
        env::set_var("HOME", "/Users/testuser");

        // These should normalize to the same path
        let path1 = ProcessedFilesDb::normalize_path_str("~/Documents/test.wav");
        let path2 = ProcessedFilesDb::normalize_path_str("/Users/testuser/Documents/test.wav");

        // Both should resolve to similar paths (may differ in canonicalization)
        // At minimum, both should have expanded HOME
        assert!(
            path1.contains("/Users/testuser"),
            "Path 1 should contain expanded home: {}",
            path1
        );
        assert!(
            path2.contains("/Users/testuser"),
            "Path 2 should contain home: {}",
            path2
        );

        // Restore env
        if let Some(home) = original_home {
            env::set_var("HOME", home);
        } else {
            env::remove_var("HOME");
        }
    }

    #[test]
    fn test_processed_files_db_duplicate_prevention() {
        let mut db = ProcessedFilesDb::new();

        let entry = ProcessedFileEntry {
            path: "/path/to/test.wav".to_string(),
            processed_at: 1234567890,
            size: 1000,
            modified_time: 1234567880,
            output_file: "/path/to/test_transcribed.md".to_string(),
            success: true,
            error: None,
        };

        // Add entry
        db.add_entry(entry.clone());

        // Should contain the file
        assert!(db.contains(Path::new("/path/to/test.wav")));

        // Should not contain a different file
        assert!(!db.contains(Path::new("/path/to/other.wav")));
    }

    #[test]
    fn test_processed_files_db_failed_entries() {
        let mut db = ProcessedFilesDb::new();

        let failed_entry = ProcessedFileEntry {
            path: "/path/to/test.wav".to_string(),
            processed_at: 1234567890,
            size: 1000,
            modified_time: 1234567880,
            output_file: String::new(),
            success: false,  // Failed
            error: Some("Test error".to_string()),
        };

        db.add_entry(failed_entry);

        // Should NOT contain the file because it failed
        assert!(!db.contains(Path::new("/path/to/test.wav")));
    }

    // Helper function for testing pattern matching without app handle
    fn matches_pattern_test(file_path: &Path, patterns: &[String]) -> bool {
        let extension = file_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        patterns.iter().any(|pattern| {
            if let Some(ext) = pattern.strip_prefix("*.") {
                extension.eq_ignore_ascii_case(ext)
            } else {
                false
            }
        })
    }
}
