//! Bridges task storage with runtime execution state.
//!
//! `TaskRuntime` combines durable task storage (which persists across restarts)
//! with a pluggable `TaskExecutor` that manages how task work is actually executed.

use std::sync::Arc;

use tracing::{debug, info};

use turul_mcp_protocol::TaskStatus;
use turul_mcp_task_storage::{
    InMemoryTaskStorage, TaskOutcome, TaskRecord, TaskStorage, TaskStorageError,
};

use crate::task::executor::TaskExecutor;
use crate::task::tokio_executor::TokioTaskExecutor;

/// Bridges task storage with runtime execution state.
///
/// Owns both:
/// - A `TaskStorage` backend (durable, serializable)
/// - A `TaskExecutor` for running task work and managing cancellation
///
/// Lives in `turul-mcp-server` (not in `turul-mcp-task-storage`) because it combines
/// backend-agnostic storage with executor-specific runtime primitives.
pub struct TaskRuntime {
    /// The durable storage backend.
    storage: Arc<dyn TaskStorage>,
    /// The task executor for running work.
    executor: Arc<dyn TaskExecutor>,
    /// Recovery timeout for stuck tasks (milliseconds).
    recovery_timeout_ms: u64,
}

impl TaskRuntime {
    /// Creates a new task runtime with the given storage backend and executor.
    pub fn new(storage: Arc<dyn TaskStorage>, executor: Arc<dyn TaskExecutor>) -> Self {
        Self {
            storage,
            executor,
            recovery_timeout_ms: 300_000, // 5 minutes default
        }
    }

    /// Creates a new task runtime with the given storage and the default `TokioTaskExecutor`.
    pub fn with_default_executor(storage: Arc<dyn TaskStorage>) -> Self {
        Self::new(storage, Arc::new(TokioTaskExecutor::new()))
    }

    /// Creates a task runtime with a custom recovery timeout.
    pub fn with_recovery_timeout(mut self, timeout_ms: u64) -> Self {
        self.recovery_timeout_ms = timeout_ms;
        self
    }

    /// Creates a new task runtime with in-memory storage and the default `TokioTaskExecutor`.
    pub fn in_memory() -> Self {
        Self::with_default_executor(Arc::new(InMemoryTaskStorage::new()))
    }

    /// Gets a reference to the underlying storage.
    pub fn storage(&self) -> &dyn TaskStorage {
        self.storage.as_ref()
    }

    /// Gets a shared reference to the storage Arc.
    pub fn storage_arc(&self) -> Arc<dyn TaskStorage> {
        Arc::clone(&self.storage)
    }

    /// Gets a reference to the executor.
    pub fn executor(&self) -> &dyn TaskExecutor {
        self.executor.as_ref()
    }

    // === Task Lifecycle ===

    /// Registers a new task in storage.
    ///
    /// Returns the created record.
    ///
    /// Does NOT start execution — call `executor().start_task()` separately
    /// when the work is ready to run.
    pub async fn register_task(&self, task: TaskRecord) -> Result<TaskRecord, TaskStorageError> {
        let task_id = task.task_id.clone();

        // Persist in storage
        let created = self.storage.create_task(task).await?;

        debug!(task_id = %task_id, "Registered task in storage");

        Ok(created)
    }

    /// Updates a task's status in storage.
    pub async fn update_status(
        &self,
        task_id: &str,
        new_status: TaskStatus,
        status_message: Option<String>,
    ) -> Result<TaskRecord, TaskStorageError> {
        let updated = self
            .storage
            .update_task_status(task_id, new_status, status_message)
            .await?;

        Ok(updated)
    }

    /// Stores a task's result and updates its status.
    pub async fn complete_task(
        &self,
        task_id: &str,
        outcome: TaskOutcome,
        status: TaskStatus,
        status_message: Option<String>,
    ) -> Result<(), TaskStorageError> {
        // Store result first
        self.storage.store_task_result(task_id, outcome).await?;

        // Then update status
        self.update_status(task_id, status, status_message).await?;

        Ok(())
    }

    /// Cancels a task by delegating to the executor and updating the storage status.
    pub async fn cancel_task(&self, task_id: &str) -> Result<TaskRecord, TaskStorageError> {
        // Try to cancel via executor (ignore error if task not in executor — may have already completed)
        if let Err(e) = self.executor.cancel_task(task_id).await {
            debug!(task_id = %task_id, error = %e, "Executor cancel returned error (task may have already completed)");
        }

        // Update storage status
        self.update_status(
            task_id,
            TaskStatus::Cancelled,
            Some("Cancelled by client".to_string()),
        )
        .await
    }

    /// Waits until a task reaches terminal status via the executor.
    ///
    /// Returns `None` if the task is not tracked by the executor (already completed or not in-flight).
    pub async fn await_terminal(&self, task_id: &str) -> Option<TaskStatus> {
        self.executor.await_terminal(task_id).await
    }

    // === Delegation to storage ===

    /// Gets a task by ID from storage.
    pub async fn get_task(&self, task_id: &str) -> Result<Option<TaskRecord>, TaskStorageError> {
        self.storage.get_task(task_id).await
    }

    /// Gets a task's stored result.
    pub async fn get_task_result(
        &self,
        task_id: &str,
    ) -> Result<Option<TaskOutcome>, TaskStorageError> {
        self.storage.get_task_result(task_id).await
    }

    /// Lists tasks with pagination.
    pub async fn list_tasks(
        &self,
        cursor: Option<&str>,
        limit: Option<u32>,
    ) -> Result<turul_mcp_task_storage::TaskListPage, TaskStorageError> {
        self.storage.list_tasks(cursor, limit).await
    }

    /// Lists tasks for a specific session.
    pub async fn list_tasks_for_session(
        &self,
        session_id: &str,
        cursor: Option<&str>,
        limit: Option<u32>,
    ) -> Result<turul_mcp_task_storage::TaskListPage, TaskStorageError> {
        self.storage
            .list_tasks_for_session(session_id, cursor, limit)
            .await
    }

    // === Recovery ===

    /// Recovers stuck tasks on startup.
    ///
    /// Called during server initialization.
    pub async fn recover_stuck_tasks(&self) -> Result<Vec<String>, TaskStorageError> {
        let recovered = self
            .storage
            .recover_stuck_tasks(self.recovery_timeout_ms)
            .await?;

        if !recovered.is_empty() {
            info!(
                count = recovered.len(),
                timeout_ms = self.recovery_timeout_ms,
                "Recovered stuck tasks on startup"
            );
        }

        Ok(recovered)
    }

    /// Runs periodic maintenance (TTL expiry, cleanup).
    pub async fn maintenance(&self) -> Result<(), TaskStorageError> {
        self.storage.maintenance().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use turul_mcp_task_storage::{InMemoryTaskStorage, TaskOutcome, TaskRecord};

    fn create_working_task() -> TaskRecord {
        TaskRecord {
            task_id: InMemoryTaskStorage::generate_task_id(),
            session_id: Some("session-1".to_string()),
            status: TaskStatus::Working,
            status_message: Some("Processing".to_string()),
            created_at: chrono::Utc::now().to_rfc3339(),
            last_updated_at: chrono::Utc::now().to_rfc3339(),
            ttl: Some(60_000),
            poll_interval: Some(5_000),
            original_method: "tools/call".to_string(),
            original_params: None,
            result: None,
            meta: None,
        }
    }

    #[tokio::test]
    async fn test_register_and_get_task() {
        let runtime = TaskRuntime::in_memory();
        let task = create_working_task();
        let task_id = task.task_id.clone();

        let created = runtime.register_task(task).await.unwrap();
        assert_eq!(created.task_id, task_id);
        assert_eq!(created.status, TaskStatus::Working);

        let fetched = runtime.get_task(&task_id).await.unwrap().unwrap();
        assert_eq!(fetched.task_id, task_id);
    }

    #[tokio::test]
    async fn test_update_status() {
        let runtime = TaskRuntime::in_memory();
        let task = create_working_task();
        let task_id = task.task_id.clone();

        runtime.register_task(task).await.unwrap();

        let updated = runtime
            .update_status(&task_id, TaskStatus::Completed, Some("Done".to_string()))
            .await
            .unwrap();
        assert_eq!(updated.status, TaskStatus::Completed);
    }

    #[tokio::test]
    async fn test_complete_task() {
        let runtime = TaskRuntime::in_memory();
        let task = create_working_task();
        let task_id = task.task_id.clone();

        runtime.register_task(task).await.unwrap();

        let outcome = TaskOutcome::Success(serde_json::json!({"answer": 42}));
        runtime
            .complete_task(&task_id, outcome, TaskStatus::Completed, None)
            .await
            .unwrap();

        let result = runtime.get_task_result(&task_id).await.unwrap().unwrap();
        match result {
            TaskOutcome::Success(v) => assert_eq!(v["answer"], 42),
            _ => panic!("Expected Success outcome"),
        }
    }

    #[tokio::test]
    async fn test_cancel_task() {
        let runtime = TaskRuntime::in_memory();
        let task = create_working_task();
        let task_id = task.task_id.clone();

        runtime.register_task(task).await.unwrap();

        let cancelled = runtime.cancel_task(&task_id).await.unwrap();
        assert_eq!(cancelled.status, TaskStatus::Cancelled);
    }

    #[tokio::test]
    async fn test_list_tasks() {
        let runtime = TaskRuntime::in_memory();

        let task1 = create_working_task();
        let task2 = create_working_task();

        runtime.register_task(task1).await.unwrap();
        runtime.register_task(task2).await.unwrap();

        let page = runtime.list_tasks(None, None).await.unwrap();
        assert_eq!(page.tasks.len(), 2);
    }
}
