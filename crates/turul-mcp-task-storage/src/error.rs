//! Unified error types for task storage operations.

use turul_mcp_protocol::TaskStatus;

/// Unified error type for task storage operations.
///
/// Mirrors the pattern used in `turul-mcp-session-storage` for consistency.
#[derive(Debug, thiserror::Error)]
pub enum TaskStorageError {
    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Invalid state transition: {current:?} -> {requested:?}")]
    InvalidTransition {
        current: TaskStatus,
        requested: TaskStatus,
    },

    #[error("Task is in terminal state: {0:?}")]
    TerminalState(TaskStatus),

    #[error("Task has expired: {0}")]
    TaskExpired(String),

    #[error("Maximum tasks limit reached: {0}")]
    MaxTasksReached(usize),

    #[error("Concurrent modification: {0}")]
    ConcurrentModification(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Generic storage error: {0}")]
    Generic(String),
}

impl From<serde_json::Error> for TaskStorageError {
    fn from(err: serde_json::Error) -> Self {
        TaskStorageError::SerializationError(err.to_string())
    }
}

#[cfg(any(feature = "sqlite", feature = "postgres"))]
impl From<sqlx::Error> for TaskStorageError {
    fn from(err: sqlx::Error) -> Self {
        TaskStorageError::DatabaseError(err.to_string())
    }
}
