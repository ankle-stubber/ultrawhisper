//! Clipboard destination - wraps existing clipboard functionality

use crate::workflow::destinations::{Destination, DestinationContext, Metadata};
use anyhow::Result;
use async_trait::async_trait;

// Phase 0: Allow dead code as these types are scaffolding for future phases
#[allow(dead_code)]
/// Clipboard destination that pastes text to active window
pub struct ClipboardDestination {
    paste_immediately: bool,
}

impl ClipboardDestination {
    /// Create a new clipboard destination
    pub fn new(paste_immediately: bool) -> Self {
        Self { paste_immediately }
    }
}

#[async_trait]
impl Destination for ClipboardDestination {
    async fn send(&self, ctx: &DestinationContext<'_>, text: &str, _meta: &Metadata) -> Result<()> {
        if !self.paste_immediately {
            return Ok(());
        }

        // Respect main-thread constraints similar to existing utils::paste
        let app = ctx.app.clone();
        let text_owned = text.to_string();
        let app_for_closure = app.clone();

        app.run_on_main_thread(move || {
            let _ = crate::utils::paste(text_owned, app_for_closure);
        })
        .map_err(|e| anyhow::anyhow!("Failed to run paste on main thread: {:?}", e))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_destination_creation() {
        let dest = ClipboardDestination::new(true);
        assert!(dest.paste_immediately);

        let dest = ClipboardDestination::new(false);
        assert!(!dest.paste_immediately);
    }
}
