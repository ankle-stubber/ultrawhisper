//! Streaming session management
//!
//! Handles state accumulation and chunk merging for a single streaming transcription session

use super::overlap::{MergeStrategy, OverlapMerger};
use log::debug;
use std::time::Duration;

/// A streaming transcription session
///
/// Manages the accumulation of transcription chunks and merges them using OverlapMerger.
/// This component lives in the streaming module, NOT in TranscriptionManager, keeping
/// TranscriptionManager stateless as per Phase 1 architecture.
pub struct StreamingSession {
    session_id: String,
    merger: OverlapMerger,
    accumulated_transcript: String,
    chunks_processed: usize,
    total_audio_duration: f32,
}

impl StreamingSession {
    /// Create a new streaming session with default settings
    ///
    /// Uses 2-second overlap window and SuffixPrefix merge strategy by default.
    pub fn new() -> Self {
        Self::with_overlap(Duration::from_secs(2))
    }

    /// Create a new streaming session with custom overlap duration
    pub fn with_overlap(overlap_duration: Duration) -> Self {
        // Generate a simple session ID using timestamp + random component
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let random_component = (timestamp % 1000000) as u32; // Simple pseudo-random component
        let session_id = format!("stream-{}-{}", timestamp, random_component);

        debug!(
            "Creating new streaming session: {} with overlap: {:?}",
            session_id, overlap_duration
        );

        Self {
            session_id,
            merger: OverlapMerger::new(overlap_duration, MergeStrategy::SuffixPrefix),
            accumulated_transcript: String::new(),
            chunks_processed: 0,
            total_audio_duration: 0.0,
        }
    }

    /// Merge a new transcription chunk with the accumulated transcript
    ///
    /// Uses OverlapMerger to intelligently detect and remove duplicate text
    /// at chunk boundaries.
    ///
    /// Returns a reference to the updated accumulated transcript.
    pub fn merge_chunk(&mut self, new_text: &str, audio_duration_secs: f32) -> &str {
        debug!(
            "Session {}: Merging chunk {} ({:.2}s)",
            self.session_id,
            self.chunks_processed + 1,
            audio_duration_secs
        );

        if self.chunks_processed == 0 {
            // First chunk - no merging needed
            self.accumulated_transcript = new_text.to_string();
            debug!(
                "Session {}: First chunk, transcript length: {} chars",
                self.session_id,
                self.accumulated_transcript.len()
            );
        } else {
            // Subsequent chunks - use OverlapMerger
            // Note: overlap_samples parameter is not used by SuffixPrefix strategy
            self.accumulated_transcript = self
                .merger
                .merge(&self.accumulated_transcript, new_text, 0);

            debug!(
                "Session {}: After merge, transcript length: {} chars",
                self.session_id,
                self.accumulated_transcript.len()
            );
        }

        self.chunks_processed += 1;
        self.total_audio_duration += audio_duration_secs;

        &self.accumulated_transcript
    }

    /// Finalize the session and return the complete transcript
    ///
    /// Consumes the session and returns the final accumulated transcript.
    pub fn finalize(self) -> String {
        debug!(
            "Session {}: Finalizing - {} chunks processed, {:.2}s total duration, {} chars",
            self.session_id,
            self.chunks_processed,
            self.total_audio_duration,
            self.accumulated_transcript.len()
        );

        self.accumulated_transcript
    }

    /// Get the current session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Get the number of chunks processed so far
    pub fn chunks_processed(&self) -> usize {
        self.chunks_processed
    }

    /// Get the total audio duration processed so far
    pub fn total_audio_duration(&self) -> f32 {
        self.total_audio_duration
    }

    /// Get a reference to the current accumulated transcript (without finalizing)
    pub fn current_transcript(&self) -> &str {
        &self.accumulated_transcript
    }
}

impl Default for StreamingSession {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = StreamingSession::new();
        assert_eq!(session.chunks_processed(), 0);
        assert_eq!(session.total_audio_duration(), 0.0);
        assert_eq!(session.current_transcript(), "");
        assert!(!session.session_id().is_empty());
    }

    #[test]
    fn test_first_chunk_no_merge() {
        let mut session = StreamingSession::new();

        let text1 = "Hello world. This is the first chunk.";
        let result = session.merge_chunk(text1, 5.0);

        assert_eq!(result, text1);
        assert_eq!(session.chunks_processed(), 1);
        assert_eq!(session.total_audio_duration(), 5.0);
    }

    #[test]
    fn test_merge_with_overlap() {
        let mut session = StreamingSession::new();

        // First chunk
        let text1 = "Hello world. This is";
        session.merge_chunk(text1, 5.0);

        // Second chunk with overlap
        let text2 = "This is the second chunk.";
        let result = session.merge_chunk(text2, 5.0);

        // Should detect "This is" overlap and merge correctly
        assert_eq!(result, "Hello world. This is the second chunk.");
        assert_eq!(session.chunks_processed(), 2);
        assert_eq!(session.total_audio_duration(), 10.0);
    }

    #[test]
    fn test_merge_multiple_chunks() {
        let mut session = StreamingSession::new();

        // Chunk 1
        session.merge_chunk("The quick brown", 3.0);

        // Chunk 2
        session.merge_chunk("brown fox jumps", 3.0);

        // Chunk 3
        let result = session.merge_chunk("jumps over the lazy dog", 3.0);

        // Should merge all three with overlap detection
        assert!(result.contains("The quick brown"));
        assert!(result.contains("fox"));
        assert!(result.contains("lazy dog"));
        assert_eq!(session.chunks_processed(), 3);
        assert_eq!(session.total_audio_duration(), 9.0);
    }

    #[test]
    fn test_merge_no_overlap() {
        let mut session = StreamingSession::new();

        // Chunk 1
        session.merge_chunk("Hello world.", 2.0);

        // Chunk 2 with no overlap
        let result = session.merge_chunk("Goodbye now.", 2.0);

        // Should concatenate with space
        assert_eq!(result, "Hello world. Goodbye now.");
        assert_eq!(session.chunks_processed(), 2);
    }

    #[test]
    fn test_finalize_returns_transcript() {
        let mut session = StreamingSession::new();

        session.merge_chunk("First chunk.", 2.0);
        session.merge_chunk("chunk. Second chunk.", 2.0);

        let final_transcript = session.finalize();

        assert_eq!(final_transcript, "First chunk. Second chunk.");
    }

    #[test]
    fn test_finalize_empty_session() {
        let session = StreamingSession::new();
        let final_transcript = session.finalize();

        assert_eq!(final_transcript, "");
    }

    #[test]
    fn test_current_transcript_non_destructive() {
        let mut session = StreamingSession::new();

        session.merge_chunk("Test text", 1.0);

        // Getting current transcript shouldn't affect state
        let current1 = session.current_transcript();
        let current2 = session.current_transcript();

        assert_eq!(current1, current2);
        assert_eq!(session.chunks_processed(), 1);
    }

    #[test]
    fn test_session_id_uniqueness() {
        let session1 = StreamingSession::new();
        // Ensure different timestamp for uniqueness
        std::thread::sleep(std::time::Duration::from_millis(2));
        let session2 = StreamingSession::new();

        assert_ne!(session1.session_id(), session2.session_id());
    }

    #[test]
    fn test_custom_overlap_duration() {
        let session = StreamingSession::with_overlap(Duration::from_secs(5));
        assert!(!session.session_id().is_empty());
    }

    #[test]
    fn test_empty_chunk_handling() {
        let mut session = StreamingSession::new();

        session.merge_chunk("First chunk", 1.0);
        let result = session.merge_chunk("", 0.0);

        // Merging empty chunk should not break things
        assert!(!result.is_empty());
        assert_eq!(session.chunks_processed(), 2);
    }

    #[test]
    fn test_punctuation_in_overlap() {
        let mut session = StreamingSession::new();

        session.merge_chunk("Hello, world! This is", 2.0);
        let result = session.merge_chunk("This is great.", 2.0);

        // Should handle punctuation in overlap detection
        assert!(result.contains("Hello, world!"));
        assert!(result.contains("great."));
    }

    #[test]
    fn test_long_transcript_accumulation() {
        let mut session = StreamingSession::new();

        // Simulate many chunks
        for i in 0..10 {
            let text = format!("Chunk {} content goes here.", i);
            session.merge_chunk(&text, 2.0);
        }

        assert_eq!(session.chunks_processed(), 10);
        assert_eq!(session.total_audio_duration(), 20.0);

        let final_transcript = session.finalize();
        assert!(final_transcript.contains("Chunk 0"));
        assert!(final_transcript.contains("Chunk 9"));
    }

    #[test]
    fn test_case_insensitive_overlap_detection() {
        let mut session = StreamingSession::new();

        session.merge_chunk("Hello WORLD this", 2.0);
        let result = session.merge_chunk("THIS is great", 2.0);

        // OverlapMerger should detect overlap despite case difference
        assert!(result.contains("WORLD"));
        assert!(result.contains("great"));
    }

    #[test]
    fn test_numbers_in_transcript() {
        let mut session = StreamingSession::new();

        session.merge_chunk("The number 42 is", 2.0);
        let result = session.merge_chunk("42 is important", 2.0);

        assert_eq!(result, "The number 42 is important");
    }

    #[test]
    fn test_default_trait() {
        let session = StreamingSession::default();
        assert_eq!(session.chunks_processed(), 0);
    }

    #[test]
    fn test_audio_duration_accumulation() {
        let mut session = StreamingSession::new();

        session.merge_chunk("First", 1.5);
        session.merge_chunk("Second", 2.3);
        session.merge_chunk("Third", 3.7);

        // Should sum to 7.5 (with floating point tolerance)
        assert!((session.total_audio_duration() - 7.5).abs() < 0.001);
    }
}
