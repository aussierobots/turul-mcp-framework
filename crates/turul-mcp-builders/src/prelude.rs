//! Prelude module for MCP builders
//!
//! This module provides a convenient way to import the most commonly used
//! builders for runtime MCP component construction (Level 3 of creation spectrum).
//!
//! # Usage
//!
//! ```rust
//! use turul_mcp_builders::prelude::*;
//!
//! // Now you have access to all builders and common types
//! ```

// Re-export all protocol prelude items for convenience
pub use turul_mcp_protocol::prelude::*;

// All builders for runtime construction
pub use crate::{
    CompletionBuilder, ElicitationBuilder, LoggingBuilder, MessageBuilder, NotificationBuilder,
    PromptBuilder, ResourceBuilder, RootBuilder, ToolBuilder,
};

// Additional builder types
pub use crate::{
    CancelledNotificationBuilder, ElicitResultBuilder, ListRootsRequestBuilder,
    ProgressNotificationBuilder, ResourceUpdatedNotificationBuilder, RootsNotificationBuilder,
    SetLevelBuilder,
};

// Common types used in builder patterns
pub use serde_json::{Value, json};
pub use std::collections::HashMap;

// Essential async trait for implementations
pub use async_trait::async_trait;

// Common serde types for serialization
pub use serde::{Deserialize, Serialize};
