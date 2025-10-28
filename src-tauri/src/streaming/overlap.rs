//! Overlap merger for streaming transcription - handles chunk boundaries

use std::time::Duration;

// Phase 0: Allow dead code as these types are scaffolding for future phases
#[allow(dead_code)]
/// Merges overlapping transcription chunks
pub struct OverlapMerger {
    #[allow(dead_code)]
    overlap_duration: Duration,
    strategy: MergeStrategy,
}

/// Strategy for merging overlapping chunks
#[derive(Debug, Clone)]
pub enum MergeStrategy {
    /// Match suffix of chunk1 with prefix of chunk2
    SuffixPrefix,
    /// Use timestamps if available
    TimestampBased,
    /// Fuzzy matching within window
    FuzzyWindow { max_distance: usize },
}

impl OverlapMerger {
    /// Create a new overlap merger
    pub fn new(overlap_duration: Duration, strategy: MergeStrategy) -> Self {
        Self {
            overlap_duration,
            strategy,
        }
    }

    /// Merge two transcription chunks with overlap
    pub fn merge(&self, chunk1: &str, chunk2: &str, _overlap_samples: usize) -> String {
        match &self.strategy {
            MergeStrategy::SuffixPrefix => self.merge_suffix_prefix(chunk1, chunk2),
            MergeStrategy::TimestampBased => {
                // Phase 0: Just concatenate with space for timestamp-based
                format!("{} {}", chunk1, chunk2)
            }
            MergeStrategy::FuzzyWindow { max_distance } => {
                self.merge_fuzzy(chunk1, chunk2, *max_distance)
            }
        }
    }

    fn merge_suffix_prefix(&self, chunk1: &str, chunk2: &str) -> String {
        // Normalize both chunks before comparison
        let norm1 = Self::normalize(chunk1);
        let norm2 = Self::normalize(chunk2);
        let words1: Vec<&str> = norm1.split_whitespace().collect();
        let words2: Vec<&str> = norm2.split_whitespace().collect();

        // Try to find overlap of 1-5 words
        for overlap_size in (1..=5).rev() {
            if overlap_size > words1.len() || overlap_size > words2.len() {
                continue;
            }

            let suffix = &words1[words1.len() - overlap_size..];
            let prefix = &words2[..overlap_size];

            if suffix == prefix {
                // Found overlap - merge the original chunks (not normalized) at word boundaries
                // We need to find where the overlap actually occurs in the original strings
                let orig_words1: Vec<&str> = chunk1.split_whitespace().collect();
                let orig_words2: Vec<&str> = chunk2.split_whitespace().collect();

                if overlap_size > orig_words1.len() || overlap_size > orig_words2.len() {
                    continue;
                }

                // Reconstruct: take all but last N words from chunk1, then all words from chunk2
                let mut result = orig_words1[..orig_words1.len() - overlap_size].join(" ");
                if !result.is_empty() && !orig_words2.is_empty() {
                    result.push(' ');
                }
                result.push_str(&orig_words2.join(" "));
                return result;
            }
        }

        // No overlap found, just concatenate original strings
        if chunk1.is_empty() {
            return chunk2.to_string();
        }
        if chunk2.is_empty() {
            return chunk1.to_string();
        }
        format!("{} {}", chunk1, chunk2)
    }

    fn merge_fuzzy(&self, chunk1: &str, chunk2: &str, _max_distance: usize) -> String {
        // Phase 0: Simplified fuzzy matching - just fall back to suffix/prefix
        // TODO: Implement edit distance matching in Phase 1+
        self.merge_suffix_prefix(chunk1, chunk2)
    }

    /// Normalize text by lowercasing and converting punctuation to spaces
    fn normalize(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        for ch in s.chars() {
            if ch.is_alphanumeric() || ch.is_whitespace() {
                out.push(ch.to_ascii_lowercase());
            } else if ",.;:!?'\"".contains(ch) {
                out.push(' ');
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_overlap_merge() {
        let merger = OverlapMerger::new(
            Duration::from_secs(2),
            MergeStrategy::SuffixPrefix,
        );

        let chunk1 = "hello world this is";
        let chunk2 = "this is a test";
        let result = merger.merge(chunk1, chunk2, 100);

        assert_eq!(result, "hello world this is a test");
    }

    #[test]
    fn test_no_overlap_merge() {
        let merger = OverlapMerger::new(
            Duration::from_secs(2),
            MergeStrategy::SuffixPrefix,
        );

        let chunk1 = "hello world";
        let chunk2 = "foo bar";
        let result = merger.merge(chunk1, chunk2, 100);

        assert_eq!(result, "hello world foo bar");
    }

    #[test]
    fn test_partial_word_overlap() {
        // Test that partial words don't create false overlaps
        let merger = OverlapMerger::new(
            Duration::from_secs(2),
            MergeStrategy::SuffixPrefix,
        );

        let chunk1 = "the cat sat";
        let chunk2 = "saturday morning";
        let result = merger.merge(chunk1, chunk2, 100);

        // "sat" and "saturday" should not match - they're different words
        assert_eq!(result, "the cat sat saturday morning");
    }

    #[test]
    fn test_punctuation_handling() {
        let merger = OverlapMerger::new(
            Duration::from_secs(2),
            MergeStrategy::SuffixPrefix,
        );

        let chunk1 = "Hello, world. This is";
        let chunk2 = "This is great!";
        let result = merger.merge(chunk1, chunk2, 100);

        // Should handle punctuation in overlap detection
        assert!(result.contains("Hello, world."));
        assert!(result.contains("great!"));
        // Normalized comparison should find "this is" overlap
        assert!(result.contains("This is great!"));
    }

    #[test]
    fn test_long_overlap() {
        let merger = OverlapMerger::new(
            Duration::from_secs(2),
            MergeStrategy::SuffixPrefix,
        );

        let chunk1 = "one two three four five";
        let chunk2 = "three four five six seven";
        let result = merger.merge(chunk1, chunk2, 100);

        assert_eq!(result, "one two three four five six seven");
    }

    #[test]
    fn test_empty_chunks() {
        let merger = OverlapMerger::new(
            Duration::from_secs(2),
            MergeStrategy::SuffixPrefix,
        );

        // Empty first chunk
        let result = merger.merge("", "hello world", 100);
        assert_eq!(result, "hello world");

        // Empty second chunk
        let result = merger.merge("hello world", "", 100);
        assert_eq!(result, "hello world");

        // Both empty
        let result = merger.merge("", "", 100);
        assert_eq!(result, "");
    }

    #[test]
    fn test_single_word_chunks() {
        let merger = OverlapMerger::new(
            Duration::from_secs(2),
            MergeStrategy::SuffixPrefix,
        );

        let chunk1 = "hello";
        let chunk2 = "hello";
        let result = merger.merge(chunk1, chunk2, 100);

        // Same single word should merge
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_case_insensitive_overlap() {
        let merger = OverlapMerger::new(
            Duration::from_secs(2),
            MergeStrategy::SuffixPrefix,
        );

        let chunk1 = "hello WORLD this";
        let chunk2 = "THIS is great";
        let result = merger.merge(chunk1, chunk2, 100);

        // Should find overlap despite case difference
        assert!(result.contains("WORLD"));
        assert!(result.contains("great"));
    }

    #[test]
    fn test_numbers_in_overlap() {
        let merger = OverlapMerger::new(
            Duration::from_secs(2),
            MergeStrategy::SuffixPrefix,
        );

        let chunk1 = "the number 42 is";
        let chunk2 = "42 is important";
        let result = merger.merge(chunk1, chunk2, 100);

        assert_eq!(result, "the number 42 is important");
    }

    #[test]
    fn test_multiple_spaces() {
        let merger = OverlapMerger::new(
            Duration::from_secs(2),
            MergeStrategy::SuffixPrefix,
        );

        let chunk1 = "hello  world  this";
        let chunk2 = "this  is  good";
        let result = merger.merge(chunk1, chunk2, 100);

        // Should handle multiple spaces gracefully
        assert!(result.contains("hello"));
        assert!(result.contains("good"));
    }

    #[test]
    fn test_timestamp_based_strategy() {
        let merger = OverlapMerger::new(
            Duration::from_secs(2),
            MergeStrategy::TimestampBased,
        );

        let chunk1 = "hello world";
        let chunk2 = "foo bar";
        let result = merger.merge(chunk1, chunk2, 100);

        // Phase 0: timestamp-based just concatenates
        assert_eq!(result, "hello world foo bar");
    }

    #[test]
    fn test_fuzzy_window_strategy() {
        let merger = OverlapMerger::new(
            Duration::from_secs(2),
            MergeStrategy::FuzzyWindow { max_distance: 2 },
        );

        let chunk1 = "hello world this is";
        let chunk2 = "this is a test";
        let result = merger.merge(chunk1, chunk2, 100);

        // Phase 0: fuzzy falls back to suffix/prefix
        assert_eq!(result, "hello world this is a test");
    }

    #[test]
    fn test_very_long_text() {
        let merger = OverlapMerger::new(
            Duration::from_secs(2),
            MergeStrategy::SuffixPrefix,
        );

        let long_text1 = (0..100).map(|i| format!("word{}", i)).collect::<Vec<_>>().join(" ");
        let overlap_words = "word98 word99";
        let long_text2 = format!("{} extra words", overlap_words);

        let result = merger.merge(&long_text1, &long_text2, 100);

        // Should handle long texts efficiently
        assert!(result.contains("word0"));
        assert!(result.contains("word99"));
        assert!(result.contains("extra"));
    }

    #[test]
    fn test_normalize_function() {
        let normalized = OverlapMerger::normalize("Hello, World! This is a TEST.");
        assert_eq!(normalized, "hello  world  this is a test ");

        let normalized = OverlapMerger::normalize("Numbers: 123, 456!");
        assert_eq!(normalized, "numbers  123  456 ");
    }

    #[test]
    fn test_quoted_text() {
        let merger = OverlapMerger::new(
            Duration::from_secs(2),
            MergeStrategy::SuffixPrefix,
        );

        let chunk1 = "he said \"hello world\"";
        let chunk2 = "\"hello world\" again";
        let result = merger.merge(chunk1, chunk2, 100);

        // Quotes should be normalized away for comparison
        assert!(result.contains("said"));
        assert!(result.contains("again"));
    }
}
