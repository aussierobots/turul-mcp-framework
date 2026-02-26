//! Task Executor — abstraction for how task work is executed.
//!
//! Separates *how tasks run* from *how tasks are stored*.
//! Default: `TokioTaskExecutor` (in-process async).
//! Future: EventBridge, SQS, Step Functions worker models.

use std::future::Future;
use std::pin::Pin;

use async_trait::async_trait;

use turul_mcp_protocol::TaskStatus;
use turul_mcp_task_storage::TaskStorageError;

/// Opaque handle returned when a task is started.
pub trait TaskHandle: Send + Sync {
    /// Request cancellation of the running task.
    fn cancel(&self);
    /// Check if cancellation has been requested.
    fn is_cancelled(&self) -> bool;
}

/// Boxed async work unit — the actual operation to execute.
pub type BoxedTaskWork = Box<
    dyn FnOnce() -> Pin<Box<dyn Future<Output = turul_mcp_task_storage::TaskOutcome> + Send>>
        + Send,
>;

/// Trait for executing task work and managing runtime lifecycle.
#[async_trait]
pub trait TaskExecutor: Send + Sync {
    /// Start executing a task. Returns an opaque handle for cancellation.
    async fn start_task(
        &self,
        task_id: &str,
        work: BoxedTaskWork,
    ) -> Result<Box<dyn TaskHandle>, TaskStorageError>;

    /// Cancel a running task by ID.
    async fn cancel_task(&self, task_id: &str) -> Result<(), TaskStorageError>;

    /// Block until a task reaches terminal status.
    /// Returns the terminal status, or None if the task is not tracked by this executor.
    async fn await_terminal(&self, task_id: &str) -> Option<TaskStatus>;
}
