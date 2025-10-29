//! Audio chunking for streaming transcription
//!
//! Splits incoming audio samples into fixed-duration chunks with overlap

use log::debug;

/// An audio chunk ready for transcription
#[derive(Debug, Clone)]
pub struct AudioChunk {
    pub audio: Vec<f32>,
    pub is_final: bool,
    /// Number of overlap samples at the start of `audio` carried from the previous chunk
    pub overlap_samples: usize,
}

/// Chunks audio samples into fixed-duration segments with overlap
pub struct AudioChunker {
    chunk_duration_ms: u32,
    overlap_duration_ms: u32,
    sample_rate: u32,
    current_buffer: Vec<f32>,
    overlap_buffer: Vec<f32>,
}

impl AudioChunker {
    /// Create a new audio chunker
    ///
    /// # Arguments
    /// * `chunk_duration_ms` - Duration of each chunk in milliseconds (e.g., 20000 for 20s)
    /// * `overlap_duration_ms` - Overlap with previous chunk in milliseconds (e.g., 2000 for 2s)
    /// * `sample_rate` - Audio sample rate (e.g., 16000 Hz)
    pub fn new(chunk_duration_ms: u32, overlap_duration_ms: u32, sample_rate: u32) -> Self {
        debug!(
            "Creating AudioChunker: chunk={}ms, overlap={}ms, rate={}Hz",
            chunk_duration_ms, overlap_duration_ms, sample_rate
        );

        Self {
            chunk_duration_ms,
            overlap_duration_ms,
            sample_rate,
            current_buffer: Vec::new(),
            overlap_buffer: Vec::new(),
        }
    }

    /// Add audio samples and potentially return a complete chunk
    ///
    /// This method accumulates samples until enough are available for a chunk.
    /// When a chunk is ready, it's extracted with overlap from the previous chunk.
    ///
    /// Returns Some(AudioChunk) when a chunk is ready, None otherwise.
    pub fn add_samples(&mut self, samples: &[f32]) -> Option<AudioChunk> {
        // Add new samples to current buffer
        self.current_buffer.extend_from_slice(samples);

        // Check if we have enough for a chunk
        let chunk_samples = self.samples_for_duration(self.chunk_duration_ms);

        if self.current_buffer.len() >= chunk_samples {
            // Extract chunk with overlap
            let overlap_samples = self.samples_for_duration(self.overlap_duration_ms);

            // Build the chunk: overlap from previous + new audio
            let mut chunk_audio = self.overlap_buffer.clone();
            chunk_audio.extend_from_slice(&self.current_buffer[..chunk_samples]);

            // Save overlap for next chunk (last N samples of current buffer)
            if chunk_samples >= overlap_samples {
                self.overlap_buffer = self.current_buffer[chunk_samples - overlap_samples..chunk_samples].to_vec();
            } else {
                self.overlap_buffer = self.current_buffer[..chunk_samples].to_vec();
            }

            // Remove processed samples from buffer
            self.current_buffer.drain(..chunk_samples);

            debug!(
                "Chunk extracted: {} samples ({:.2}s), overlap: {} samples ({:.2}s), remaining: {} samples",
                chunk_audio.len(),
                chunk_audio.len() as f32 / self.sample_rate as f32,
                self.overlap_buffer.len(),
                self.overlap_buffer.len() as f32 / self.sample_rate as f32,
                self.current_buffer.len()
            );

            return Some(AudioChunk {
                audio: chunk_audio,
                is_final: false,
                overlap_samples: self.overlap_buffer.len(),
            });
        }

        None
    }

    /// Flush any remaining audio as a final chunk
    ///
    /// Call this when recording stops to process any buffered audio
    /// that didn't reach the full chunk duration.
    ///
    /// Returns Some(AudioChunk) if there's remaining audio, None otherwise.
    pub fn flush_remaining(&mut self) -> Option<AudioChunk> {
        if self.current_buffer.is_empty() {
            debug!("No remaining audio to flush");
            return None;
        }

        // Build final chunk with overlap
        let mut chunk_audio = self.overlap_buffer.clone();
        chunk_audio.extend_from_slice(&self.current_buffer);

        let chunk_duration = chunk_audio.len() as f32 / self.sample_rate as f32;

        debug!(
            "Flushing final chunk: {} samples ({:.2}s)",
            chunk_audio.len(),
            chunk_duration
        );

        // Clear buffers
        self.current_buffer.clear();
        self.overlap_buffer.clear();

        Some(AudioChunk {
            audio: chunk_audio,
            is_final: true,
            overlap_samples: self.overlap_buffer.len(),
        })
    }

    /// Calculate number of samples for a given duration in milliseconds
    fn samples_for_duration(&self, duration_ms: u32) -> usize {
        (self.sample_rate as u64 * duration_ms as u64 / 1000) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_at_exact_boundary() {
        let mut chunker = AudioChunker::new(1000, 100, 16000);
        let samples = vec![0.5; 16000]; // Exactly 1 second

        let chunk = chunker.add_samples(&samples);
        assert!(chunk.is_some());

        let chunk = chunk.unwrap();
        assert_eq!(chunk.audio.len(), 16000);
        assert!(!chunk.is_final);
    }

    #[test]
    fn test_chunk_preserves_overlap() {
        let mut chunker = AudioChunker::new(1000, 100, 16000);

        // First chunk - all 1.0
        let samples1 = vec![1.0; 16000];
        let chunk1 = chunker.add_samples(&samples1).unwrap();
        assert_eq!(chunk1.audio.len(), 16000);

        // Second chunk - all 2.0
        let samples2 = vec![2.0; 16000];
        let chunk2 = chunker.add_samples(&samples2).unwrap();

        // First 1600 samples (100ms at 16kHz) should be from previous chunk (1.0)
        assert_eq!(chunk2.audio[0], 1.0);
        assert_eq!(chunk2.audio[1599], 1.0);

        // Rest should be from new chunk (2.0)
        assert_eq!(chunk2.audio[1600], 2.0);
        assert_eq!(chunk2.audio[chunk2.audio.len() - 1], 2.0);
    }

    #[test]
    fn test_no_chunk_when_insufficient_samples() {
        let mut chunker = AudioChunker::new(1000, 100, 16000);
        let samples = vec![0.5; 8000]; // Half a second

        let chunk = chunker.add_samples(&samples);
        assert!(chunk.is_none());
    }

    #[test]
    fn test_flush_partial_chunk() {
        let mut chunker = AudioChunker::new(1000, 100, 16000);
        let samples = vec![0.5; 8000]; // Half a second

        // No chunk yet
        let chunk = chunker.add_samples(&samples);
        assert!(chunk.is_none());

        // Flush should return the partial chunk
        let final_chunk = chunker.flush_remaining();
        assert!(final_chunk.is_some());

        let final_chunk = final_chunk.unwrap();
        assert_eq!(final_chunk.audio.len(), 8000);
        assert!(final_chunk.is_final);
    }

    #[test]
    fn test_flush_empty_returns_none() {
        let mut chunker = AudioChunker::new(1000, 100, 16000);
        let final_chunk = chunker.flush_remaining();
        assert!(final_chunk.is_none());
    }

    #[test]
    fn test_multiple_chunks_in_sequence() {
        let mut chunker = AudioChunker::new(1000, 100, 16000);

        // Process 3 chunks
        for i in 0..3 {
            let samples = vec![(i + 1) as f32; 16000];
            let chunk = chunker.add_samples(&samples);
            assert!(chunk.is_some());

            let chunk = chunk.unwrap();
            if i == 0 {
                // First chunk has no overlap
                assert_eq!(chunk.audio.len(), 16000);
            } else {
                // Subsequent chunks include overlap (16000 + 1600)
                assert_eq!(chunk.audio.len(), 17600);
            }
        }
    }

    #[test]
    fn test_flush_after_complete_chunk() {
        let mut chunker = AudioChunker::new(1000, 100, 16000);

        // Add exactly one chunk
        let samples = vec![1.0; 16000];
        let chunk = chunker.add_samples(&samples);
        assert!(chunk.is_some());

        // Add partial for next chunk
        let samples2 = vec![2.0; 5000];
        let chunk2 = chunker.add_samples(&samples2);
        assert!(chunk2.is_none());

        // Flush should return the partial with overlap from first chunk
        let final_chunk = chunker.flush_remaining();
        assert!(final_chunk.is_some());

        let final_chunk = final_chunk.unwrap();
        // Should have: 1600 (overlap) + 5000 (new) = 6600 samples
        assert_eq!(final_chunk.audio.len(), 6600);
        assert!(final_chunk.is_final);
    }

    #[test]
    fn test_samples_for_duration_calculation() {
        let chunker = AudioChunker::new(1000, 100, 16000);

        assert_eq!(chunker.samples_for_duration(1000), 16000);  // 1 second
        assert_eq!(chunker.samples_for_duration(500), 8000);    // 0.5 seconds
        assert_eq!(chunker.samples_for_duration(100), 1600);    // 0.1 seconds
    }

    #[test]
    fn test_incremental_sample_addition() {
        let mut chunker = AudioChunker::new(1000, 100, 16000);

        // Add samples incrementally (1000 samples at a time)
        for i in 0..16 {
            let samples = vec![0.5; 1000];
            let chunk = chunker.add_samples(&samples);

            if i < 15 {
                // Not enough yet
                assert!(chunk.is_none());
            } else {
                // 16th addition completes the chunk
                assert!(chunk.is_some());
            }
        }
    }

    #[test]
    fn test_different_sample_rates() {
        // Test with 48kHz sample rate
        let mut chunker = AudioChunker::new(1000, 100, 48000);
        let samples = vec![0.5; 48000]; // 1 second at 48kHz

        let chunk = chunker.add_samples(&samples);
        assert!(chunk.is_some());
        assert_eq!(chunk.unwrap().audio.len(), 48000);
    }

    #[test]
    fn test_long_overlap() {
        // Test with longer overlap (500ms out of 1000ms chunk)
        let mut chunker = AudioChunker::new(1000, 500, 16000);

        let samples1 = vec![1.0; 16000];
        let chunk1 = chunker.add_samples(&samples1).unwrap();

        let samples2 = vec![2.0; 16000];
        let chunk2 = chunker.add_samples(&samples2).unwrap();

        // First 8000 samples (500ms) should be overlap from first chunk
        assert_eq!(chunk2.audio[0], 1.0);
        assert_eq!(chunk2.audio[7999], 1.0);
        assert_eq!(chunk2.audio[8000], 2.0);
    }
}
