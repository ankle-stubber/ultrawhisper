//! Destination routing - sends transcription results to configured outputs

use anyhow::Result;
use async_trait::async_trait;
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

    /// Route transcription to all configured destinations (Phase 0: stub)
    pub async fn route(&self, _ctx: &DestinationContext<'_>, _text: &str, _metadata: &Metadata) -> Result<()> {
        // Phase 0: Just a stub, no actual routing
        Ok(())
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
}
