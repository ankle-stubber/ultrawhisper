//! Destination module - reusable output configurations
//!
//! This module provides:
//! - Type definitions for destinations (types.rs)
//! - CRUD storage operations (storage.rs)
//!
//! Destinations are entities that can be referenced by multiple workflows.
//! They define where transcribed text should be sent and how it should be formatted.

pub mod storage;
pub mod types;

pub use storage::{create_default_destinations, seed_defaults_if_empty, DestinationStorage};
pub use types::{Destination, DestinationConfig, ValidationError};
