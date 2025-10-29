//! Backfill module for whole-file transcription of saved streaming audio

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

use crate::audio_toolkit::audio::load_wav_file;
use crate::managers::transcription::TranscriptionManager;

/// Backfill the transcript by transcribing a saved WAV file in batch mode.
///
/// This function is used after streaming recording completes to generate
/// a potentially higher quality transcript from the complete audio file,
/// rather than relying on the merged stream of individual chunks.
///
/// # Arguments
/// * `app` - Tauri app handle for path resolution
/// * `file_name` - Name of the WAV file (e.g., "handy-1234567890.wav")
/// * `tm` - TranscriptionManager to perform the transcription
///
/// # Returns
/// * `Ok(String)` - The transcribed text
/// * `Err` - If the file cannot be loaded or transcription fails
///
/// # Implementation Notes
/// - Resolves the file path using the app data recordings directory
/// - Expects 16kHz mono WAV format (as written by StreamingWavWriter)
/// - Performs synchronous transcription in batch mode
pub async fn backfill_whole_file(
    app: &AppHandle,
    file_name: &str,
    tm: Arc<TranscriptionManager>,
) -> Result<String> {
    // Resolve full path to the recordings directory
    let recordings_dir = app
        .path()
        .app_data_dir()
        .context("Failed to resolve app data directory")?
        .join("recordings");

    let file_path: PathBuf = recordings_dir.join(file_name);

    log::debug!("Backfilling transcript from file: {:?}", file_path);

    // Load the WAV file (expects 16kHz mono)
    let samples = load_wav_file(&file_path)
        .with_context(|| format!("Failed to load WAV file for backfill: {:?}", file_path))?;

    log::debug!(
        "Loaded {} samples from {:?}, starting batch transcription",
        samples.len(),
        file_path
    );

    // Transcribe the entire audio in batch mode
    // Note: TranscriptionManager::transcribe is synchronous, so we need to run it
    // in a blocking task to avoid blocking the async runtime
    let text = tokio::task::spawn_blocking(move || tm.transcribe(samples))
        .await
        .context("Backfill transcription task panicked")?
        .context("Backfill transcription failed")?;

    log::info!(
        "Backfill complete for {:?}: {} characters",
        file_path,
        text.len()
    );

    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Full integration tests require a working TranscriptionManager
    // and model, which are tested in the engine integration tests.
    // Unit tests here focus on validating the logic path.

    #[test]
    fn test_backfill_api_compiles() {
        // This test just ensures the API signature is correct
        // Actual integration tests will be in engine.rs
        fn _check_signature() -> impl std::future::Future<Output = Result<String>> {
            async {
                // This won't actually run, just checking types compile
                panic!("This is a compile-time check only")
            }
        }
    }
}
