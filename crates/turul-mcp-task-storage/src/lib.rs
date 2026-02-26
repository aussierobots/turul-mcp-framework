//! # Task Storage Abstractions and Implementations
//!
//! **Pluggable task storage backends for MCP servers with durable state machines.**
//!
//! MCP 2025-11-25 introduces Tasks — durable state machines for long-running operations
//! (tool calls, sampling, elicitation). This crate provides the `TaskStorage` trait and
//! an in-memory implementation for development and testing.
//!
//! ## Quick Start
//!
//! ```rust
//! use turul_mcp_task_storage::prelude::*;
//!
//! # async fn example() -> Result<(), TaskStorageError> {
//! let storage = InMemoryTaskStorage::new();
//!
//! // Create a task
//! let task = TaskRecord {
//!     task_id: InMemoryTaskStorage::generate_task_id(),
//!     session_id: Some("session-123".to_string()),
//!     status: turul_mcp_protocol::TaskStatus::Working,
//!     status_message: Some("Processing...".to_string()),
//!     created_at: chrono::Utc::now().to_rfc3339(),
//!     last_updated_at: chrono::Utc::now().to_rfc3339(),
//!     ttl: Some(60_000),
//!     poll_interval: Some(5_000),
//!     original_method: "tools/call".to_string(),
//!     original_params: None,
//!     result: None,
//!     meta: None,
//! };
//!
//! let created = storage.create_task(task).await?;
//!
//! // Update status with state machine enforcement
//! let completed = storage.update_task_status(
//!     &created.task_id,
//!     turul_mcp_protocol::TaskStatus::Completed,
//!     Some("Done!".to_string()),
//! ).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Architecture
//!
//! - **`TaskStorage` trait**: Core abstraction for task CRUD, status updates, pagination
//! - **`TaskRecord`**: Persistence model (serializable, no runtime handles)
//! - **`TaskOutcome`**: Success/Error result stored for `tasks/result` retrieval
//! - **State machine**: Validates transitions per MCP spec lifecycle

// Core modules
pub mod error;
#[cfg(feature = "in-memory")]
pub mod in_memory;
pub mod prelude;
pub mod state_machine;
pub mod traits;

// Durable storage backends
#[cfg(feature = "dynamodb")]
pub mod dynamodb;
#[cfg(feature = "postgres")]
pub mod postgres;
#[cfg(feature = "sqlite")]
pub mod sqlite;

// Parity test suite (shared across all backends)
#[cfg(test)]
#[allow(dead_code)]
pub(crate) mod parity_tests;

// Re-exports for convenience
pub use error::TaskStorageError;
#[cfg(feature = "in-memory")]
pub use in_memory::{InMemoryTaskConfig, InMemoryTaskStorage};
pub use state_machine::{is_terminal, validate_transition};
pub use traits::{TaskListPage, TaskOutcome, TaskRecord, TaskStorage};

#[cfg(feature = "dynamodb")]
pub use dynamodb::{DynamoDbTaskConfig, DynamoDbTaskStorage};
#[cfg(feature = "postgres")]
pub use postgres::{PostgresTaskConfig, PostgresTaskStorage};
#[cfg(feature = "sqlite")]
pub use sqlite::{SqliteTaskConfig, SqliteTaskStorage};

/// Create a default in-memory task storage instance for development and testing.
#[cfg(feature = "in-memory")]
pub fn create_default_storage() -> InMemoryTaskStorage {
    InMemoryTaskStorage::new()
}

/// Create an in-memory task storage with custom configuration.
#[cfg(feature = "in-memory")]
pub fn create_memory_storage(config: InMemoryTaskConfig) -> InMemoryTaskStorage {
    InMemoryTaskStorage::with_config(config)
}
