//! Unified error types for task storage operations.

use turul_mcp_protocol::TaskStatus;

/// Unified error type for task storage operations.
///
/// Mirrors the pattern used in `turul-mcp-session-storage` for consistency.
#[derive(Debug, thiserror::Error)]
pub enum TaskStorageError {
    /// No task exists with the given task ID (the payload).
    #[error("Task not found: {0}")]
    TaskNotFound(String),

    /// The requested status change is not permitted from the task's current status.
    #[error("Invalid state transition: {current:?} -> {requested:?}")]
    InvalidTransition {
        /// The task's status before the attempted transition.
        current: TaskStatus,
        /// The status the caller tried to move the task to.
        requested: TaskStatus,
    },

    /// The task has already reached a terminal status and can no longer change; carries that status.
    #[error("Task is in terminal state: {0:?}")]
    TerminalState(TaskStatus),

    /// The task's time-to-live has elapsed; carries the task ID.
    #[error("Task has expired: {0}")]
    TaskExpired(String),

    /// Creating the task would exceed the configured task limit; carries that limit.
    #[error("Maximum tasks limit reached: {0}")]
    MaxTasksReached(usize),

    /// Another writer changed the task during the operation; carries a description of the conflict.
    #[error("Concurrent modification: {0}")]
    ConcurrentModification(String),

    /// The backing database reported a failure; carries its message.
    #[error("Database error: {0}")]
    DatabaseError(String),

    /// A task record could not be serialized or deserialized; carries the underlying error message.
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// A storage failure not covered by another variant; carries a description.
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
