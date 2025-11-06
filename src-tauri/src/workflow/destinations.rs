//! Destination routing - sends transcription results to configured outputs

use anyhow::Result;
use async_trait::async_trait;
use log::{debug, error, info, warn};
use std::path::PathBuf;
use tauri::AppHandle;

// Phase 0: Allow dead code as these types are scaffolding for future phases
#[allow(dead_code)]
/// Execution context for destinations
pub struct DestinationContext<'a> {
    pub app: &'a AppHandle,
    pub output_base: Option<PathBuf>,
    pub audio_path: Option<PathBuf>,
}

/// Metadata about the transcription
pub struct Metadata {
    pub workflow_id: String,
    pub timestamp: u64,
    pub duration_ms: u64,
    pub model_used: String,
}

/// Result of sending to a destination
#[derive(Debug, Clone)]
pub enum DestinationResult {
    /// Destination succeeded
    Success,
    /// Destination failed with error message
    Failed(String),
}

/// Trait for sending transcription results to destinations
#[async_trait]
pub trait Destination: Send + Sync {
    /// Send transcription result to this destination
    async fn send(&self, ctx: &DestinationContext<'_>, text: &str, metadata: &Metadata) -> Result<()>;
}

/// Routes transcription results to multiple destinations
pub struct DestinationRouter {
    destinations: Vec<Box<dyn Destination>>,
}

impl DestinationRouter {
    /// Create a new destination router
    pub fn new() -> Self {
        Self {
            destinations: Vec::new(),
        }
    }

    /// Add a destination to the router
    pub fn add_destination(&mut self, dest: Box<dyn Destination>) {
        self.destinations.push(dest);
    }

    /// Route transcription to all configured destinations
    ///
    /// This method iterates through all destinations and attempts to send to each one.
    /// It collects the results and returns them all, never failing the entire operation
    /// even if individual destinations fail.
    pub async fn route(
        &self,
        ctx: &DestinationContext<'_>,
        text: &str,
        metadata: &Metadata,
    ) -> Result<Vec<DestinationResult>> {
        use log::{debug, error};

        let mut results = Vec::new();

        debug!("Routing transcription to {} destination(s)", self.destinations.len());

        let total = self.destinations.len();
        for (idx, destination) in self.destinations.iter().enumerate() {
            debug!("Attempting to send to destination {}", idx + 1);

            let result = match destination.send(ctx, text, metadata).await {
                Ok(()) => {
                    debug!("Destination {} succeeded", idx + 1);
                    DestinationResult::Success
                }
                Err(e) => {
                    error!("Destination {} failed: {}", idx + 1, e);
                    DestinationResult::Failed(e.to_string())
                }
            };

            results.push(result);
        }

        let successes = results
            .iter()
            .filter(|r| matches!(r, DestinationResult::Success))
            .count();
        let failures = total.saturating_sub(successes);

        if failures == 0 {
            info!(
                "routed transcription '{}' to {} destination(s) successfully",
                metadata.workflow_id, total
            );
        } else {
            warn!(
                "routed transcription '{}' with {} success(es) and {} failure(s)",
                metadata.workflow_id, successes, failures
            );
        }

        Ok(results)
    }
}

impl Default for DestinationRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    // Mock destination that always succeeds
    struct MockSuccessDestination {
        call_count: Arc<Mutex<usize>>,
    }

    #[async_trait]
    impl Destination for MockSuccessDestination {
        async fn send(&self, _ctx: &DestinationContext<'_>, _text: &str, _metadata: &Metadata) -> Result<()> {
            let mut count = self.call_count.lock().unwrap();
            *count += 1;
            Ok(())
        }
    }

    // Mock destination that always fails
    struct MockFailDestination {
        error_message: String,
    }

    #[async_trait]
    impl Destination for MockFailDestination {
        async fn send(&self, _ctx: &DestinationContext<'_>, _text: &str, _metadata: &Metadata) -> Result<()> {
            Err(anyhow::anyhow!("{}", self.error_message))
        }
    }

    #[test]
    fn test_router_construction() {
        let router = DestinationRouter::new();
        assert_eq!(router.destinations.len(), 0);
    }

    #[test]
    fn test_router_default() {
        let router = DestinationRouter::default();
        assert_eq!(router.destinations.len(), 0);
    }

    #[test]
    fn test_metadata_construction() {
        let metadata = Metadata {
            workflow_id: "test_workflow".to_string(),
            timestamp: 1234567890,
            duration_ms: 5000,
            model_used: "whisper-small".to_string(),
        };

        assert_eq!(metadata.workflow_id, "test_workflow");
        assert_eq!(metadata.timestamp, 1234567890);
        assert_eq!(metadata.duration_ms, 5000);
        assert_eq!(metadata.model_used, "whisper-small");
    }

    #[tokio::test]
    async fn test_router_single_destination_success() {
        // Note: This test requires a Tauri AppHandle which we can't easily create in unit tests
        // In practice, this would be tested in integration tests with a real Tauri app
        // For now, we test the router construction and destination addition
        let mut router = DestinationRouter::new();
        let call_count = Arc::new(Mutex::new(0));

        router.add_destination(Box::new(MockSuccessDestination {
            call_count: Arc::clone(&call_count),
        }));

        assert_eq!(router.destinations.len(), 1);
    }

    #[tokio::test]
    async fn test_router_multiple_destinations() {
        let mut router = DestinationRouter::new();
        let call_count1 = Arc::new(Mutex::new(0));
        let call_count2 = Arc::new(Mutex::new(0));

        router.add_destination(Box::new(MockSuccessDestination {
            call_count: Arc::clone(&call_count1),
        }));
        router.add_destination(Box::new(MockSuccessDestination {
            call_count: Arc::clone(&call_count2),
        }));

        assert_eq!(router.destinations.len(), 2);
    }

    #[test]
    fn test_destination_result_variants() {
        let success = DestinationResult::Success;
        let failed = DestinationResult::Failed("test error".to_string());

        match success {
            DestinationResult::Success => {}
            _ => panic!("Expected Success variant"),
        }

        match failed {
            DestinationResult::Failed(msg) => assert_eq!(msg, "test error"),
            _ => panic!("Expected Failed variant"),
        }
    }

    // Note: Full integration tests for router.route() with actual destinations
    // are in the seam tests, as they require a Tauri AppHandle
}
