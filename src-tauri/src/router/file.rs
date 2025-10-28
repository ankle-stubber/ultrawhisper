//! File destination - wraps existing file output functionality

use crate::workflow::destinations::{Destination, DestinationContext, Metadata};
use anyhow::Result;
use async_trait::async_trait;

// Phase 0: Allow dead code as these types are scaffolding for future phases
#[allow(dead_code)]
/// File destination that saves transcriptions to disk
pub struct FileDestination {
    #[allow(dead_code)]
    output_path: Option<String>,
}

impl FileDestination {
    /// Create a new file destination
    pub fn new(output_path: Option<String>) -> Self {
        Self { output_path }
    }
}

#[async_trait]
impl Destination for FileDestination {
    async fn send(&self, ctx: &DestinationContext<'_>, text: &str, _meta: &Metadata) -> Result<()> {
        // Delegate to existing file output functionality
        crate::file_output::save_transcription_to_file(text, ctx.app, self.output_path.clone())
            .map_err(|e| anyhow::anyhow!("File destination failed: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_destination_creation() {
        let dest = FileDestination::new(None);
        assert!(dest.output_path.is_none());

        let dest = FileDestination::new(Some("/custom/path".to_string()));
        assert_eq!(dest.output_path, Some("/custom/path".to_string()));
    }
}
