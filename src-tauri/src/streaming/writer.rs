//! Streaming WAV writer for recording audio during streaming transcription
//!
//! This module provides a robust, single-owner writer that writes 16kHz mono WAV
//! files during streaming. The writer uses a temporary file during recording
//! and atomically renames it to the final filename when finalized.

use anyhow::{Context, Result};
use hound::{WavSpec, WavWriter};
use std::fs;
use std::io::BufWriter;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// Audio format for streaming recordings.
///
/// This enum defines the supported audio formats for streaming transcription.
/// Currently, only WAV is implemented. OPUS and FLAC support will be added in Phase 3b.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioFormat {
    /// Uncompressed WAV format (16kHz, mono, 16-bit PCM)
    Wav,
    // TODO: Phase 3b - Implement OPUS compression
    // Opus,
    // TODO: Phase 3b - Implement FLAC lossless compression
    // Flac,
}

impl AudioFormat {
    /// Validates and parses an audio format string.
    ///
    /// # Arguments
    /// * `format_str` - The format string to validate (e.g., "wav", "opus", "flac")
    ///
    /// # Returns
    /// `Ok(AudioFormat)` if the format is valid and supported
    /// `Err` if the format is unknown or not yet implemented
    ///
    /// # Example
    /// ```rust,ignore
    /// // Example usage within the crate:
    /// // let format = AudioFormat::from_str("wav").unwrap();
    /// // assert_eq!(format, AudioFormat::Wav);
    /// ```
    pub fn from_str(format_str: &str) -> Result<Self> {
        match format_str.to_lowercase().as_str() {
            "wav" => Ok(AudioFormat::Wav),
            // Phase 3b: Uncomment when implementing OPUS support
            // "opus" => Ok(AudioFormat::Opus),
            // Phase 3b: Uncomment when implementing FLAC support
            // "flac" => Ok(AudioFormat::Flac),
            _ => Err(anyhow::anyhow!(
                "Unsupported audio format '{}'. Currently only 'wav' is supported.",
                format_str
            )),
        }
    }

    /// Returns the file extension for this audio format.
    ///
    /// # Returns
    /// The file extension without the dot (e.g., "wav", "opus", "flac")
    pub fn file_extension(&self) -> &'static str {
        match self {
            AudioFormat::Wav => "wav",
            // AudioFormat::Opus => "opus",
            // AudioFormat::Flac => "flac",
        }
    }

    /// Returns a human-readable name for this audio format.
    pub fn display_name(&self) -> &'static str {
        match self {
            AudioFormat::Wav => "WAV (Uncompressed)",
            // AudioFormat::Opus => "OPUS (Compressed)",
            // AudioFormat::Flac => "FLAC (Lossless)",
        }
    }
}

/// A streaming WAV writer that records audio to disk during streaming transcription.
///
/// This writer is designed to be owned by a single task (the consumer task) and should
/// never be shared across threads. It writes 16kHz mono 16-bit PCM WAV files.
///
/// # Usage Flow
/// 1. Call `open()` to create a new writer with a temporary filename
/// 2. Call `append()` repeatedly to write audio samples
/// 3. Optionally call `flush()` periodically to ensure data is written to disk
/// 4. Call `finalize()` to close the file and atomically rename it to the final filename
pub struct StreamingWavWriter {
    /// Temporary file path (e.g., "handy-<timestamp>.tmp")
    tmp_path: PathBuf,
    /// Final file path after finalization (e.g., "handy-<timestamp>.wav")
    final_path: PathBuf,
    /// The underlying WAV writer
    writer: WavWriter<BufWriter<fs::File>>,
}

impl StreamingWavWriter {
    /// Opens a new streaming WAV writer.
    ///
    /// # Arguments
    /// * `app` - The Tauri app handle for resolving the recordings directory
    /// * `unix_ts_secs` - Unix timestamp in seconds, used for the filename
    ///
    /// # Returns
    /// A new `StreamingWavWriter` instance ready to accept audio samples
    ///
    /// # Errors
    /// Returns an error if:
    /// - The recordings directory cannot be created
    /// - The temporary file cannot be opened
    pub fn open(app: &AppHandle, unix_ts_secs: i64) -> Result<Self> {
        // Get the recordings directory: app_data_dir/recordings
        let recordings_dir = app
            .path()
            .app_data_dir()
            .context("Failed to get app data directory")?
            .join("recordings");

        Self::open_with_dir(recordings_dir, unix_ts_secs)
    }

    /// Internal constructor that takes a recordings directory directly.
    /// This is used by the public `open` method and by tests.
    fn open_with_dir(recordings_dir: PathBuf, unix_ts_secs: i64) -> Result<Self> {
        // Create the recordings directory if it doesn't exist
        fs::create_dir_all(&recordings_dir)
            .context("Failed to create recordings directory")?;

        // Generate filenames
        let filename_base = format!("handy-{}", unix_ts_secs);
        let tmp_path = recordings_dir.join(format!("{}.tmp", filename_base));
        let final_path = recordings_dir.join(format!("{}.wav", filename_base));

        // Configure WAV format: mono, 16kHz, 16-bit PCM
        let spec = WavSpec {
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        // Create the WAV writer with buffering
        let file = fs::File::create(&tmp_path)
            .context("Failed to create temporary WAV file")?;
        let buf_writer = BufWriter::new(file);
        let writer = WavWriter::new(buf_writer, spec)
            .context("Failed to create WAV writer")?;

        Ok(Self {
            tmp_path,
            final_path,
            writer,
        })
    }

    /// Appends audio samples to the WAV file.
    ///
    /// # Arguments
    /// * `samples` - A slice of f32 audio samples in the range [-1.0, 1.0]
    ///
    /// # Returns
    /// Ok(()) on success
    ///
    /// # Errors
    /// Returns an error if writing to the file fails
    ///
    /// # Note
    /// The input samples are converted from f32 to i16 PCM format by multiplying
    /// by i16::MAX and clamping to the valid range.
    pub fn append(&mut self, samples: &[f32]) -> Result<()> {
        for &sample in samples {
            // Convert f32 [-1.0, 1.0] to i16 PCM
            // Clamp to prevent overflow
            let sample_clamped = sample.clamp(-1.0, 1.0);
            let sample_i16 = (sample_clamped * i16::MAX as f32) as i16;

            self.writer
                .write_sample(sample_i16)
                .context("Failed to write audio sample")?;
        }
        Ok(())
    }

    /// Flushes buffered data to disk.
    ///
    /// This is a best-effort operation that ensures buffered audio data is written
    /// to the operating system. Should be called periodically (e.g., every 5 seconds)
    /// to prevent data loss in case of crashes.
    ///
    /// # Errors
    /// Returns an error if flushing fails, but this is non-fatal and the writer
    /// can continue to be used.
    pub fn flush(&mut self) -> Result<()> {
        self.writer
            .flush()
            .context("Failed to flush WAV writer")?;
        Ok(())
    }

    /// Finalizes the WAV file and atomically renames it to the final filename.
    ///
    /// This method consumes the writer, closes the file, and attempts to rename
    /// the temporary file to its final name. On Windows, if atomic rename fails,
    /// it will fall back to copy-then-delete.
    ///
    /// # Returns
    /// The final filename (e.g., "handy-1234567890.wav") without the full path
    ///
    /// # Errors
    /// Returns an error if:
    /// - Finalizing the WAV file fails
    /// - The rename/copy operation fails
    pub fn finalize(self) -> Result<String> {
        // Finalize the WAV file (writes the header with correct size)
        self.writer
            .finalize()
            .context("Failed to finalize WAV file")?;

        // Attempt atomic rename
        match fs::rename(&self.tmp_path, &self.final_path) {
            Ok(_) => {
                // Success - extract filename and return
                let filename = self
                    .final_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .context("Invalid final filename")?
                    .to_string();
                Ok(filename)
            }
            Err(e) => {
                // Check if the rename actually succeeded despite returning an error
                // or if the final file already exists
                if self.final_path.exists() {
                    log::info!("Rename reported error but final file exists, considering success");
                    let filename = self
                        .final_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .context("Invalid final filename")?
                        .to_string();

                    // Try to clean up temp file if it still exists
                    if self.tmp_path.exists() {
                        fs::remove_file(&self.tmp_path).ok();
                    }

                    return Ok(filename);
                }

                // On Windows, rename can fail with CrossDeviceLink or PermissionDenied
                // In these cases, fall back to copy + delete
                log::warn!(
                    "Atomic rename failed ({}), attempting copy fallback",
                    e
                );

                // Check if temp file still exists before trying to copy
                if !self.tmp_path.exists() {
                    return Err(anyhow::anyhow!(
                        "Temporary file no longer exists after failed rename: {:?}",
                        self.tmp_path
                    ));
                }

                fs::copy(&self.tmp_path, &self.final_path)
                    .context("Failed to copy temporary file to final location")?;

                // Try to remove the temp file, but don't fail if it doesn't work
                if let Err(e) = fs::remove_file(&self.tmp_path) {
                    log::warn!(
                        "Failed to remove temporary file after copy: {}",
                        e
                    );
                }

                let filename = self
                    .final_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .context("Invalid final filename")?
                    .to_string();
                Ok(filename)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hound::WavReader;
    use std::env;

    // ============================================================================
    // AudioFormat Tests
    // ============================================================================

    #[test]
    fn test_audio_format_from_str_wav() {
        let format = AudioFormat::from_str("wav").unwrap();
        assert_eq!(format, AudioFormat::Wav);
    }

    #[test]
    fn test_audio_format_from_str_case_insensitive() {
        assert_eq!(AudioFormat::from_str("WAV").unwrap(), AudioFormat::Wav);
        assert_eq!(AudioFormat::from_str("Wav").unwrap(), AudioFormat::Wav);
        assert_eq!(AudioFormat::from_str("wav").unwrap(), AudioFormat::Wav);
    }

    #[test]
    fn test_audio_format_from_str_unsupported() {
        // OPUS and FLAC are not yet supported in Phase 3
        assert!(AudioFormat::from_str("opus").is_err());
        assert!(AudioFormat::from_str("flac").is_err());
        assert!(AudioFormat::from_str("mp3").is_err());
        assert!(AudioFormat::from_str("unknown").is_err());
    }

    #[test]
    fn test_audio_format_file_extension() {
        assert_eq!(AudioFormat::Wav.file_extension(), "wav");
    }

    #[test]
    fn test_audio_format_display_name() {
        assert_eq!(AudioFormat::Wav.display_name(), "WAV (Uncompressed)");
    }

    #[test]
    fn test_audio_format_debug() {
        // Ensure the enum derives Debug correctly
        let format = AudioFormat::Wav;
        let debug_str = format!("{:?}", format);
        assert!(debug_str.contains("Wav"));
    }

    #[test]
    fn test_audio_format_clone() {
        // Ensure the enum derives Clone correctly
        let format = AudioFormat::Wav;
        let cloned = format.clone();
        assert_eq!(format, cloned);
    }

    #[test]
    fn test_audio_format_copy() {
        // Ensure the enum derives Copy correctly
        let format = AudioFormat::Wav;
        let copied = format; // Should copy, not move
        assert_eq!(format, copied);
    }

    #[test]
    fn test_audio_format_equality() {
        // Ensure the enum derives PartialEq and Eq correctly
        let format1 = AudioFormat::Wav;
        let format2 = AudioFormat::Wav;
        assert_eq!(format1, format2);
    }

    // ============================================================================
    // StreamingWavWriter Tests
    // ============================================================================

    /// Helper to create a temporary test directory
    fn create_test_dir() -> PathBuf {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_dir = env::temp_dir().join(format!("ultrawhisper_test_{}_{}",
            std::process::id(), nanos));
        fs::create_dir_all(&temp_dir).expect("Failed to create test directory");
        temp_dir
    }

    /// Helper to clean up test directory
    fn cleanup_test_dir(dir: &PathBuf) {
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn test_open_append_flush_finalize() {
        let test_dir = create_test_dir();
        let unix_ts = 1234567890;

        let mut writer = StreamingWavWriter::open_with_dir(test_dir.clone(), unix_ts)
            .expect("Failed to open writer");

        // Create some test samples (1 second of 16kHz audio)
        let sample_count = 16000;
        let samples: Vec<f32> = (0..sample_count)
            .map(|i| (i as f32 / sample_count as f32 * 2.0 - 1.0) * 0.5)
            .collect();

        // Append samples
        writer.append(&samples).expect("Failed to append samples");

        // Flush
        writer.flush().expect("Failed to flush");

        // Finalize and get filename
        let filename = writer.finalize().expect("Failed to finalize");
        assert_eq!(filename, format!("handy-{}.wav", unix_ts));

        // Verify the WAV file is valid
        let wav_path = test_dir.join(&filename);
        assert!(wav_path.exists(), "WAV file should exist");

        // Read and validate the WAV file
        let reader = WavReader::open(&wav_path).expect("Failed to open WAV file");
        let spec = reader.spec();
        assert_eq!(spec.channels, 1, "Should be mono");
        assert_eq!(spec.sample_rate, 16000, "Should be 16kHz");
        assert_eq!(spec.bits_per_sample, 16, "Should be 16-bit");
        assert_eq!(
            spec.sample_format,
            hound::SampleFormat::Int,
            "Should be PCM integer format"
        );

        // Verify sample count
        let samples_read: Vec<i16> = reader
            .into_samples::<i16>()
            .collect::<Result<Vec<_>, _>>()
            .expect("Failed to read samples");
        assert_eq!(
            samples_read.len(),
            sample_count,
            "Sample count should match"
        );

        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_multiple_append_calls() {
        let test_dir = create_test_dir();
        let unix_ts = 1234567891;

        let mut writer = StreamingWavWriter::open_with_dir(test_dir.clone(), unix_ts)
            .expect("Failed to open writer");

        // Append samples in multiple calls (simulating streaming)
        let chunk_size = 1600; // 100ms chunks at 16kHz
        for _ in 0..10 {
            let samples: Vec<f32> = (0..chunk_size)
                .map(|i| (i as f32 / chunk_size as f32) * 0.1)
                .collect();
            writer.append(&samples).expect("Failed to append samples");
        }

        let filename = writer.finalize().expect("Failed to finalize");

        // Verify total sample count
        let wav_path = test_dir.join(&filename);
        let reader = WavReader::open(&wav_path).expect("Failed to open WAV file");
        let samples_read: Vec<i16> = reader
            .into_samples::<i16>()
            .collect::<Result<Vec<_>, _>>()
            .expect("Failed to read samples");
        assert_eq!(samples_read.len(), chunk_size * 10);

        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_sample_conversion() {
        let test_dir = create_test_dir();
        let unix_ts = 1234567892;

        let mut writer = StreamingWavWriter::open_with_dir(test_dir.clone(), unix_ts)
            .expect("Failed to open writer");

        // Test edge cases for f32 to i16 conversion
        let samples = vec![
            -1.0,  // Should map to i16::MIN
            0.0,   // Should map to 0
            1.0,   // Should map to i16::MAX
            0.5,   // Should map to roughly i16::MAX / 2
            -0.5,  // Should map to roughly i16::MIN / 2
        ];

        writer.append(&samples).expect("Failed to append samples");
        let filename = writer.finalize().expect("Failed to finalize");

        // Read back and verify
        let wav_path = test_dir.join(&filename);
        let reader = WavReader::open(&wav_path).expect("Failed to open WAV file");
        let samples_read: Vec<i16> = reader
            .into_samples::<i16>()
            .collect::<Result<Vec<_>, _>>()
            .expect("Failed to read samples");

        // Verify conversions (with some tolerance for rounding)
        assert!(samples_read[0] <= i16::MIN + 1);
        assert!(samples_read[1].abs() < 100); // Near zero
        assert!(samples_read[2] >= i16::MAX - 1);

        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_empty_file() {
        let test_dir = create_test_dir();
        let unix_ts = 1234567893;

        let writer = StreamingWavWriter::open_with_dir(test_dir.clone(), unix_ts)
            .expect("Failed to open writer");

        // Finalize without appending any samples
        let filename = writer.finalize().expect("Failed to finalize");

        // Verify the file exists and is valid (even if empty)
        let wav_path = test_dir.join(&filename);
        assert!(wav_path.exists());

        let reader = WavReader::open(&wav_path).expect("Failed to open WAV file");
        let samples_read: Vec<i16> = reader
            .into_samples::<i16>()
            .collect::<Result<Vec<_>, _>>()
            .expect("Failed to read samples");
        assert_eq!(samples_read.len(), 0);

        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_temp_file_cleanup_on_finalize() {
        let test_dir = create_test_dir();
        let unix_ts = 1234567894;

        let writer = StreamingWavWriter::open_with_dir(test_dir.clone(), unix_ts)
            .expect("Failed to open writer");

        let tmp_path = test_dir.join(format!("handy-{}.tmp", unix_ts));
        assert!(tmp_path.exists(), "Temp file should exist before finalize");

        writer.finalize().expect("Failed to finalize");

        // After finalize, temp file should be gone
        assert!(!tmp_path.exists(), "Temp file should be removed after finalize");

        cleanup_test_dir(&test_dir);
    }
}
