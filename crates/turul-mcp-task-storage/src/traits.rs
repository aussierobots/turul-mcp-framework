//! Core task storage trait and data models.
//!
//! Defines the `TaskStorage` trait and supporting types for persisting MCP tasks
//! across different backends (InMemory, SQLite, PostgreSQL, DynamoDB).

use crate::error::TaskStorageError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use turul_mcp_protocol::TaskStatus;

/// The outcome of a task's underlying request.
///
/// Stored by `TaskStorage`, returned verbatim by the `tasks/result` handler.
/// Distinguishes between successful results and JSON-RPC errors, since the spec
/// requires `tasks/result` to reproduce "exactly what the underlying request
/// would have returned."
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskOutcome {
    /// The underlying request succeeded — Value is the result object
    /// (e.g., `CallToolResult` serialized as JSON)
    Success(Value),
    /// The underlying request failed — contains the JSON-RPC error fields
    Error {
        code: i64,
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<Value>,
    },
}

/// Persistence model for a task.
///
/// Contains only serializable fields — runtime handles (cancellation tokens, status
/// watches) are managed separately by the `TaskExecutor` in the server crate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecord {
    /// Unique task identifier (UUID v7 for temporal ordering)
    pub task_id: String,
    /// Session ID this task is bound to (for authorization isolation)
    pub session_id: Option<String>,
    /// Current status of the task
    pub status: TaskStatus,
    /// Optional human-readable status message
    pub status_message: Option<String>,
    /// ISO 8601 datetime when the task was created
    pub created_at: String,
    /// ISO 8601 datetime when the task was last updated
    pub last_updated_at: String,
    /// Time-to-live in milliseconds from creation
    pub ttl: Option<i64>,
    /// Suggested polling interval in milliseconds
    pub poll_interval: Option<u64>,
    /// The method that created this task (e.g., "tools/call")
    pub original_method: String,
    /// The original request params (serialized)
    pub original_params: Option<Value>,
    /// The operation outcome (success Value OR JSON-RPC error)
    pub result: Option<TaskOutcome>,
    /// Additional metadata
    pub meta: Option<HashMap<String, Value>>,
}

impl TaskRecord {
    /// Convert this record to a protocol `Task` for wire transmission.
    pub fn to_protocol_task(&self) -> turul_mcp_protocol::Task {
        let mut task = turul_mcp_protocol::Task::new(
            &self.task_id,
            self.status,
            &self.created_at,
            &self.last_updated_at,
        );
        if let Some(ref msg) = self.status_message {
            task = task.with_status_message(msg);
        }
        if let Some(ttl) = self.ttl {
            task = task.with_ttl(ttl);
        }
        if let Some(interval) = self.poll_interval {
            task = task.with_poll_interval(interval);
        }
        if let Some(ref meta) = self.meta {
            task = task.with_meta(meta.clone());
        }
        task
    }
}

/// Paginated result for task listing.
#[derive(Debug, Clone)]
pub struct TaskListPage {
    /// Tasks in this page
    pub tasks: Vec<TaskRecord>,
    /// Cursor for the next page (None if this is the last page)
    pub next_cursor: Option<String>,
}

/// Core trait for task storage backends.
///
/// Implementations must be `Send + Sync` for use across async contexts.
/// Uses `TaskStorageError` as the unified error type.
#[async_trait]
pub trait TaskStorage: Send + Sync {
    /// Human-readable name of the storage backend (e.g., "in-memory", "sqlite")
    fn backend_name(&self) -> &'static str;

    // === Task CRUD ===

    /// Create a new task record. Returns the created record.
    async fn create_task(&self, task: TaskRecord) -> Result<TaskRecord, TaskStorageError>;

    /// Get a task by ID. Returns `None` if not found.
    async fn get_task(&self, task_id: &str) -> Result<Option<TaskRecord>, TaskStorageError>;

    /// Update an existing task record (full replacement).
    async fn update_task(&self, task: TaskRecord) -> Result<(), TaskStorageError>;

    /// Delete a task by ID. Returns `true` if deleted, `false` if not found.
    async fn delete_task(&self, task_id: &str) -> Result<bool, TaskStorageError>;

    // === Task Listing (paginated) ===

    /// List tasks with cursor-based pagination.
    async fn list_tasks(
        &self,
        cursor: Option<&str>,
        limit: Option<u32>,
    ) -> Result<TaskListPage, TaskStorageError>;

    // === Task Status Updates (state machine enforcement) ===

    /// Update a task's status with state machine validation.
    ///
    /// Returns the updated `TaskRecord` on success.
    /// Returns `TaskStorageError::InvalidTransition` or `TaskStorageError::TerminalState`
    /// if the transition is not allowed.
    async fn update_task_status(
        &self,
        task_id: &str,
        new_status: TaskStatus,
        status_message: Option<String>,
    ) -> Result<TaskRecord, TaskStorageError>;

    // === Result Storage ===

    /// Store the outcome of the underlying request for a task.
    async fn store_task_result(
        &self,
        task_id: &str,
        result: TaskOutcome,
    ) -> Result<(), TaskStorageError>;

    /// Get the stored outcome for a task.
    async fn get_task_result(&self, task_id: &str)
    -> Result<Option<TaskOutcome>, TaskStorageError>;

    // === Cleanup ===

    /// Expire tasks that have exceeded their TTL. Returns IDs of expired tasks.
    async fn expire_tasks(&self) -> Result<Vec<String>, TaskStorageError>;

    /// Get the total number of tasks in storage.
    async fn task_count(&self) -> Result<usize, TaskStorageError>;

    /// Perform periodic maintenance (cleanup, compaction, etc.)
    async fn maintenance(&self) -> Result<(), TaskStorageError>;

    // === Session Binding ===

    /// List tasks bound to a specific session, with cursor-based pagination.
    async fn list_tasks_for_session(
        &self,
        session_id: &str,
        cursor: Option<&str>,
        limit: Option<u32>,
    ) -> Result<TaskListPage, TaskStorageError>;

    // === Recovery ===

    /// Mark all non-terminal tasks older than `max_age_ms` as Failed.
    ///
    /// Called on server startup to recover from unclean shutdown.
    /// Returns the IDs of tasks that were marked as failed.
    async fn recover_stuck_tasks(&self, max_age_ms: u64) -> Result<Vec<String>, TaskStorageError>;
}
