//! Bounded queue for audio chunks with backpressure handling

use super::chunker::AudioChunk;
use log::warn;
use serde::{Deserialize, Serialize};

/// Backpressure policy for when the queue is full
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum BackpressurePolicy {
    /// Block until space is available (may cause audio dropouts)
    Block,
    /// Drop the newest chunk when queue is full (non-blocking)
    /// Backward-compat alias: previously named "DropOldest" in settings
    #[serde(alias = "DropOldest")] // accept legacy persisted value
    DropNewest,
    /// Coalesce chunks by merging audio (future enhancement)
    Coalesce,
}

impl Default for BackpressurePolicy {
    fn default() -> Self {
        // Block ensures no chunk loss by default
        BackpressurePolicy::Block
    }
}

/// Result of trying to send a chunk to the queue
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendResult {
    /// Chunk was successfully queued
    Sent,
    /// Queue was full, newest chunk was dropped
    DroppedNewest,
    /// Queue was full and policy is Block (caller should retry)
    WouldBlock,
}

/// Statistics about queue usage
#[derive(Debug, Clone, Default)]
pub struct QueueStats {
    pub chunks_sent: u64,
    pub chunks_dropped: u64,
    pub current_size: usize,
    pub max_capacity: usize,
}

/// Create a bounded chunk queue
///
/// This is a convenience function that wraps tokio's mpsc channel
/// with our custom types and statistics tracking.
///
/// Returns (Sender, Receiver) tuple for the channel.
pub fn create_bounded_queue(
    capacity: usize,
) -> (
    tokio::sync::mpsc::Sender<AudioChunk>,
    tokio::sync::mpsc::Receiver<AudioChunk>,
) {
    tokio::sync::mpsc::channel(capacity)
}

/// Try to send a chunk with the specified backpressure policy
///
/// This function handles the backpressure policy when the queue is full.
/// For DropNewest policy, it uses try_send and logs a warning on failure.
///
/// Note: The DropNewest policy drops the incoming (newest) chunk when the queue is full.
/// For Phase 2, we use try_send which returns an error when full, and the
/// caller can decide to log and continue (effectively dropping the newest chunk).
pub fn try_send_with_policy(
    sender: &tokio::sync::mpsc::Sender<AudioChunk>,
    chunk: AudioChunk,
    policy: BackpressurePolicy,
) -> SendResult {
    match policy {
        BackpressurePolicy::Block => {
            // For blocking policy, indicate that caller should use blocking send
            // This is handled at the call site with sender.send().await
            SendResult::WouldBlock
        }
        BackpressurePolicy::DropNewest => {
            match sender.try_send(chunk) {
                Ok(()) => SendResult::Sent,
                Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                    // Queue is full - with DropNewest policy, we log and drop the NEW chunk
                    warn!(
                        "Chunk queue full (capacity: {}), dropping newest chunk due to DropNewest policy",
                        sender.max_capacity()
                    );
                    SendResult::DroppedNewest
                }
                Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                    warn!("Chunk queue closed, cannot send chunk");
                    SendResult::DroppedNewest
                }
            }
        }
        BackpressurePolicy::Coalesce => {
            // Future enhancement: merge chunks
            // For Phase 2, fall back to DropNewest behavior
            warn!("Coalesce policy not yet implemented, falling back to DropNewest");
            try_send_with_policy(sender, chunk, BackpressurePolicy::DropNewest)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_backpressure_policy() {
        assert_eq!(BackpressurePolicy::default(), BackpressurePolicy::Block);
    }

    #[test]
    fn test_backpressure_policy_serialization() {
        let policy = BackpressurePolicy::DropNewest;
        let json = serde_json::to_string(&policy).unwrap();
        let deserialized: BackpressurePolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(policy, deserialized);
    }

    #[tokio::test]
    async fn test_create_bounded_queue() {
        let (tx, mut rx) = create_bounded_queue(5);

        // Send a test chunk
        let chunk = AudioChunk {
            audio: vec![0.5; 1000],
            is_final: false,
            overlap_samples: 0,
        };

        tx.send(chunk).await.unwrap();

        // Receive and verify
        let received = rx.recv().await.unwrap();
        assert_eq!(received.audio.len(), 1000);
        assert!(!received.is_final);
    }

    #[tokio::test]
    async fn test_queue_capacity_enforcement() {
        let (tx, mut rx) = create_bounded_queue(2);

        // Fill the queue
        let chunk1 = AudioChunk { audio: vec![1.0; 1000], is_final: false, overlap_samples: 0 };
        let chunk2 = AudioChunk { audio: vec![2.0; 1000], is_final: false, overlap_samples: 0 };

        tx.send(chunk1).await.unwrap();
        tx.send(chunk2).await.unwrap();

        // Queue is now full (capacity 2)
        // try_send should fail
        let chunk3 = AudioChunk { audio: vec![3.0; 1000], is_final: false, overlap_samples: 0 };

        let result = tx.try_send(chunk3);
        assert!(result.is_err());
        match result {
            Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                // Expected
            }
            _ => panic!("Expected TrySendError::Full"),
        }

        // Drain one chunk to make space
        let _ = rx.recv().await.unwrap();

        // Now we should be able to send again
        let chunk4 = AudioChunk {
            audio: vec![4.0; 1000],
            is_final: false,
            overlap_samples: 0,
        };
        tx.try_send(chunk4).unwrap();
    }

    #[tokio::test]
    async fn test_try_send_with_policy_drop_newest() {
        let (tx, mut rx) = create_bounded_queue(2);

        // Fill the queue
        let chunk1 = AudioChunk {
            audio: vec![1.0; 1000],
            is_final: false,
            overlap_samples: 0,
        };
        let chunk2 = AudioChunk {
            audio: vec![2.0; 1000],
            is_final: false,
            overlap_samples: 0,
        };

        tx.send(chunk1).await.unwrap();
        tx.send(chunk2).await.unwrap();

        // Try to send when full with DropNewest policy
        let chunk3 = AudioChunk {
            audio: vec![3.0; 1000],
            is_final: false,
            overlap_samples: 0,
        };

        let result = try_send_with_policy(&tx, chunk3, BackpressurePolicy::DropNewest);
        assert_eq!(result, SendResult::DroppedNewest);

        // Drain the queue - we should only see the first two chunks
        let received1 = rx.recv().await.unwrap();
        assert_eq!(received1.audio[0], 1.0);

        let received2 = rx.recv().await.unwrap();
        assert_eq!(received2.audio[0], 2.0);

        // No more chunks (chunk3 was dropped)
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn test_try_send_with_policy_block() {
        let (tx, _rx) = create_bounded_queue(2);

        let chunk = AudioChunk { audio: vec![1.0; 1000], is_final: false, overlap_samples: 0 };

        // Block policy should indicate caller should use blocking send
        let result = try_send_with_policy(&tx, chunk, BackpressurePolicy::Block);
        assert_eq!(result, SendResult::WouldBlock);
    }

    #[tokio::test]
    async fn test_try_send_with_policy_success() {
        let (tx, mut rx) = create_bounded_queue(5);

        let chunk = AudioChunk { audio: vec![1.0; 1000], is_final: false, overlap_samples: 0 };

        let result = try_send_with_policy(&tx, chunk, BackpressurePolicy::DropNewest);
        assert_eq!(result, SendResult::Sent);

        // Verify chunk was received
        let received = rx.recv().await.unwrap();
        assert_eq!(received.audio[0], 1.0);
    }

    #[tokio::test]
    async fn test_channel_close_behavior() {
        let (tx, rx) = create_bounded_queue(5);

        // Close receiver
        drop(rx);

        // Try to send after close
        let chunk = AudioChunk {
            audio: vec![1.0; 1000],
            is_final: false,
            overlap_samples: 0,
        };

        let result = try_send_with_policy(&tx, chunk, BackpressurePolicy::DropNewest);
        assert_eq!(result, SendResult::DroppedNewest);
    }

    #[tokio::test]
    async fn test_final_chunk_flag() {
        let (tx, mut rx) = create_bounded_queue(5);

        // Send a final chunk
        let chunk = AudioChunk { audio: vec![1.0; 1000], is_final: true, overlap_samples: 0 };

        tx.send(chunk).await.unwrap();

        let received = rx.recv().await.unwrap();
        assert!(received.is_final);
    }

    #[tokio::test]
    async fn test_queue_stats_structure() {
        let stats = QueueStats {
            chunks_sent: 100,
            chunks_dropped: 5,
            current_size: 2,
            max_capacity: 5,
        };

        assert_eq!(stats.chunks_sent, 100);
        assert_eq!(stats.chunks_dropped, 5);
        assert_eq!(stats.current_size, 2);
        assert_eq!(stats.max_capacity, 5);
    }

    #[tokio::test]
    async fn test_coalesce_policy_fallback() {
        let (tx, mut rx) = create_bounded_queue(2);

        // Fill queue
        tx.send(AudioChunk { audio: vec![1.0; 1000], is_final: false, overlap_samples: 0 })
        .await
        .unwrap();
        tx.send(AudioChunk { audio: vec![2.0; 1000], is_final: false, overlap_samples: 0 })
        .await
        .unwrap();

        // Coalesce should fall back to DropNewest for Phase 2
        let chunk3 = AudioChunk { audio: vec![3.0; 1000], is_final: false, overlap_samples: 0 };

        let result = try_send_with_policy(&tx, chunk3, BackpressurePolicy::Coalesce);
        assert_eq!(result, SendResult::DroppedNewest);
    }

    #[test]
    fn test_send_result_variants() {
        let results = vec![
            SendResult::Sent,
            SendResult::DroppedNewest,
            SendResult::WouldBlock,
        ];

        for result in results {
            // Just ensure all variants can be created and compared
            match result {
                SendResult::Sent => {}
                SendResult::DroppedNewest => {}
                SendResult::WouldBlock => {}
            }
        }
    }
}
