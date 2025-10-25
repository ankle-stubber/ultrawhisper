use anyhow::{Context, Result};
use hound::{SampleFormat, WavReader};
use std::path::Path;
use std::time::Duration;

use super::resampler::FrameResampler;

/// Load a WAV file and convert it to 16kHz mono f32 samples
/// suitable for Whisper transcription.
///
/// This function:
/// - Reads the WAV file using hound
/// - Converts samples to f32 format
/// - Downmixes stereo to mono if needed
/// - Resamples to 16kHz if the source sample rate differs
///
/// # Arguments
/// * `path` - Path to the WAV file
///
/// # Returns
/// * `Result<Vec<f32>>` - Audio samples at 16kHz, mono, normalized to [-1.0, 1.0]
pub fn load_wav_file(path: &Path) -> Result<Vec<f32>> {
    let mut reader = WavReader::open(path)
        .with_context(|| format!("Failed to open WAV file: {:?}", path))?;

    let spec = reader.spec();

    // Read samples and convert to f32
    let mut samples: Vec<f32> = match spec.sample_format {
        SampleFormat::Int => {
            // Integer samples - read as i16 or i32 based on bits per sample
            match spec.bits_per_sample {
                16 => {
                    reader
                        .samples::<i16>()
                        .map(|s| {
                            s.map(|v| v as f32 / i16::MAX as f32)
                                .context("Failed to read i16 sample")
                        })
                        .collect::<Result<Vec<f32>>>()?
                }
                32 => {
                    reader
                        .samples::<i32>()
                        .map(|s| {
                            s.map(|v| v as f32 / i32::MAX as f32)
                                .context("Failed to read i32 sample")
                        })
                        .collect::<Result<Vec<f32>>>()?
                }
                _ => {
                    return Err(anyhow::anyhow!(
                        "Unsupported bits per sample: {}",
                        spec.bits_per_sample
                    ));
                }
            }
        }
        SampleFormat::Float => {
            reader
                .samples::<f32>()
                .map(|s| s.context("Failed to read f32 sample"))
                .collect::<Result<Vec<f32>>>()?
        }
    };

    // Handle stereo -> mono downmix if needed
    if spec.channels == 2 {
        samples = downmix_stereo_to_mono(samples);
    } else if spec.channels > 2 {
        return Err(anyhow::anyhow!(
            "Unsupported channel count: {}. Only mono and stereo are supported.",
            spec.channels
        ));
    }

    // Resample to 16kHz if needed using existing FrameResampler
    if spec.sample_rate != 16000 {
        // Use FrameResampler with a frame duration that processes all audio
        // We use a small frame duration to avoid memory issues
        let frame_duration = Duration::from_millis(100); // 100ms frames
        let mut resampler = FrameResampler::new(
            spec.sample_rate as usize,
            16000,
            frame_duration,
        );

        let mut resampled = Vec::new();
        resampler.push(&samples, |frame| {
            resampled.extend_from_slice(frame);
        });

        // Finish resampling to get any remaining samples
        resampler.finish(|frame| {
            resampled.extend_from_slice(frame);
        });

        samples = resampled;
    }

    Ok(samples)
}

/// Downmix stereo audio to mono by averaging left and right channels
fn downmix_stereo_to_mono(stereo: Vec<f32>) -> Vec<f32> {
    stereo
        .chunks(2)
        .map(|lr| {
            if lr.len() == 2 {
                (lr[0] + lr[1]) / 2.0
            } else {
                // Handle odd number of samples (shouldn't happen with valid stereo)
                lr[0]
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_downmix_stereo_to_mono() {
        let stereo = vec![0.5, -0.5, 1.0, 0.0, -1.0, 1.0];
        let mono = downmix_stereo_to_mono(stereo);

        assert_eq!(mono.len(), 3);
        assert_eq!(mono[0], 0.0); // (0.5 + -0.5) / 2
        assert_eq!(mono[1], 0.5); // (1.0 + 0.0) / 2
        assert_eq!(mono[2], 0.0); // (-1.0 + 1.0) / 2
    }

    #[test]
    fn test_downmix_odd_samples() {
        let stereo = vec![0.5, -0.5, 1.0];
        let mono = downmix_stereo_to_mono(stereo);

        assert_eq!(mono.len(), 2);
        assert_eq!(mono[0], 0.0);
        assert_eq!(mono[1], 1.0); // Last sample, no pair
    }
}
