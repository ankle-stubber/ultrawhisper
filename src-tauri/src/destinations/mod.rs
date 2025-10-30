//! Destination module - reusable output configurations
//!
//! This module provides:
//! - Type definitions for destinations (types.rs)
//! - CRUD storage operations (storage.rs)
//! - Destination adapters (active_window.rs, filesystem.rs, telegram.rs)
//! - Migration from legacy binding config (migration.rs)
//!
//! Destinations are entities that can be referenced by multiple workflows.
//! They define where transcribed text should be sent and how it should be formatted.

pub mod active_window;
pub mod filesystem;
pub mod migration;
pub mod storage;
pub mod telegram;
pub mod types;

pub use active_window::ActiveWindowDestination;
pub use filesystem::FileSystemDestination;
pub use migration::migrate_legacy_bindings_if_needed;
pub use storage::{seed_defaults_if_empty, DestinationStorage};
pub use telegram::TelegramDestination;
pub use types::{Destination, DestinationConfig};
