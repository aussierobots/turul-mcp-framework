//! In-memory task storage backend.
//!
//! Suitable for development, testing, and single-instance deployments.
//! Tasks are stored in a `HashMap` behind an `RwLock`.

use crate::error::TaskStorageError;
use crate::state_machine;
use crate::traits::{TaskListPage, TaskOutcome, TaskRecord, TaskStorage};
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use turul_mcp_protocol::TaskStatus;
use uuid::Uuid;

/// Configuration for the in-memory task storage backend.
#[derive(Debug, Clone)]
pub struct InMemoryTaskConfig {
    /// Maximum number of tasks to store (0 = unlimited)
    pub max_tasks: usize,
    /// Default page size for list operations
    pub default_page_size: u32,
}

impl Default for InMemoryTaskConfig {
    fn default() -> Self {
        Self {
            max_tasks: 10_000,
            default_page_size: 50,
        }
    }
}

/// In-memory task storage backend.
///
/// Uses `Arc<RwLock<HashMap>>` for concurrent access.
/// Task IDs are UUID v7 for temporal ordering.
#[derive(Clone)]
pub struct InMemoryTaskStorage {
    tasks: Arc<RwLock<HashMap<String, TaskRecord>>>,
    config: InMemoryTaskConfig,
}

impl InMemoryTaskStorage {
    /// Create a new in-memory task storage with default configuration.
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            config: InMemoryTaskConfig::default(),
        }
    }

    /// Create a new in-memory task storage with custom configuration.
    pub fn with_config(config: InMemoryTaskConfig) -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Generate a new task ID using UUID v7 (temporal ordering).
    pub fn generate_task_id() -> String {
        Uuid::now_v7().to_string()
    }

    fn now_iso8601() -> String {
        Utc::now().to_rfc3339()
    }
}

impl Default for InMemoryTaskStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TaskStorage for InMemoryTaskStorage {
    fn backend_name(&self) -> &'static str {
        "in-memory"
    }

    async fn create_task(&self, mut task: TaskRecord) -> Result<TaskRecord, TaskStorageError> {
        let mut tasks = self.tasks.write().await;

        if self.config.max_tasks > 0 && tasks.len() >= self.config.max_tasks {
            return Err(TaskStorageError::MaxTasksReached(self.config.max_tasks));
        }

        // Ensure timestamps are set
        if task.created_at.is_empty() {
            task.created_at = Self::now_iso8601();
        }
        if task.last_updated_at.is_empty() {
            task.last_updated_at = task.created_at.clone();
        }

        tasks.insert(task.task_id.clone(), task.clone());
        Ok(task)
    }

    async fn get_task(&self, task_id: &str) -> Result<Option<TaskRecord>, TaskStorageError> {
        let tasks = self.tasks.read().await;
        Ok(tasks.get(task_id).cloned())
    }

    async fn update_task(&self, task: TaskRecord) -> Result<(), TaskStorageError> {
        let mut tasks = self.tasks.write().await;
        if !tasks.contains_key(&task.task_id) {
            return Err(TaskStorageError::TaskNotFound(task.task_id.clone()));
        }
        tasks.insert(task.task_id.clone(), task);
        Ok(())
    }

    async fn delete_task(&self, task_id: &str) -> Result<bool, TaskStorageError> {
        let mut tasks = self.tasks.write().await;
        Ok(tasks.remove(task_id).is_some())
    }

    async fn list_tasks(
        &self,
        cursor: Option<&str>,
        limit: Option<u32>,
    ) -> Result<TaskListPage, TaskStorageError> {
        let tasks = self.tasks.read().await;
        let limit = limit.unwrap_or(self.config.default_page_size) as usize;

        // Sort by (created_at, task_id) for deterministic ordering
        let mut sorted: Vec<&TaskRecord> = tasks.values().collect();
        sorted.sort_by(|a, b| {
            a.created_at
                .cmp(&b.created_at)
                .then_with(|| a.task_id.cmp(&b.task_id))
        });

        // Apply cursor — find the cursor task_id, start after it.
        // If cursor doesn't exist, start from the beginning (graceful degradation).
        let start = if let Some(cursor_id) = cursor {
            sorted
                .iter()
                .position(|t| t.task_id == cursor_id)
                .map(|pos| pos + 1)
                .unwrap_or(0)
        } else {
            0
        };

        let page: Vec<TaskRecord> = sorted
            .iter()
            .skip(start)
            .take(limit)
            .map(|t| (*t).clone())
            .collect();

        let next_cursor = if start + limit < sorted.len() {
            page.last().map(|t| t.task_id.clone())
        } else {
            None
        };

        Ok(TaskListPage {
            tasks: page,
            next_cursor,
        })
    }

    async fn update_task_status(
        &self,
        task_id: &str,
        new_status: TaskStatus,
        status_message: Option<String>,
    ) -> Result<TaskRecord, TaskStorageError> {
        let mut tasks = self.tasks.write().await;

        let task = tasks
            .get_mut(task_id)
            .ok_or_else(|| TaskStorageError::TaskNotFound(task_id.to_string()))?;

        // Validate state machine transition
        state_machine::validate_transition(task.status, new_status)?;

        task.status = new_status;
        task.status_message = status_message;
        task.last_updated_at = Self::now_iso8601();

        Ok(task.clone())
    }

    async fn store_task_result(
        &self,
        task_id: &str,
        result: TaskOutcome,
    ) -> Result<(), TaskStorageError> {
        let mut tasks = self.tasks.write().await;

        let task = tasks
            .get_mut(task_id)
            .ok_or_else(|| TaskStorageError::TaskNotFound(task_id.to_string()))?;

        task.result = Some(result);
        task.last_updated_at = Self::now_iso8601();

        Ok(())
    }

    async fn get_task_result(
        &self,
        task_id: &str,
    ) -> Result<Option<TaskOutcome>, TaskStorageError> {
        let tasks = self.tasks.read().await;

        let task = tasks
            .get(task_id)
            .ok_or_else(|| TaskStorageError::TaskNotFound(task_id.to_string()))?;

        Ok(task.result.clone())
    }

    async fn expire_tasks(&self) -> Result<Vec<String>, TaskStorageError> {
        let mut tasks = self.tasks.write().await;
        let now = Utc::now();
        let mut expired = Vec::new();

        // Collect IDs of expired tasks
        let to_expire: Vec<String> = tasks
            .values()
            .filter(|t| {
                if let Some(ttl) = t.ttl {
                    if let Ok(created) = chrono::DateTime::parse_from_rfc3339(&t.created_at) {
                        let expiry =
                            created.with_timezone(&Utc) + chrono::Duration::milliseconds(ttl);
                        return now > expiry;
                    }
                }
                false
            })
            .map(|t| t.task_id.clone())
            .collect();

        for id in to_expire {
            tasks.remove(&id);
            expired.push(id);
        }

        Ok(expired)
    }

    async fn task_count(&self) -> Result<usize, TaskStorageError> {
        let tasks = self.tasks.read().await;
        Ok(tasks.len())
    }

    async fn maintenance(&self) -> Result<(), TaskStorageError> {
        self.expire_tasks().await?;
        Ok(())
    }

    async fn list_tasks_for_session(
        &self,
        session_id: &str,
        cursor: Option<&str>,
        limit: Option<u32>,
    ) -> Result<TaskListPage, TaskStorageError> {
        let tasks = self.tasks.read().await;
        let limit = limit.unwrap_or(self.config.default_page_size) as usize;

        // Filter by session_id, sort by (created_at, task_id) for deterministic ordering
        let mut sorted: Vec<&TaskRecord> = tasks
            .values()
            .filter(|t| t.session_id.as_deref() == Some(session_id))
            .collect();
        sorted.sort_by(|a, b| {
            a.created_at
                .cmp(&b.created_at)
                .then_with(|| a.task_id.cmp(&b.task_id))
        });

        let start = if let Some(cursor_id) = cursor {
            sorted
                .iter()
                .position(|t| t.task_id == cursor_id)
                .map(|pos| pos + 1)
                .unwrap_or(0)
        } else {
            0
        };

        let page: Vec<TaskRecord> = sorted
            .iter()
            .skip(start)
            .take(limit)
            .map(|t| (*t).clone())
            .collect();

        let next_cursor = if start + limit < sorted.len() {
            page.last().map(|t| t.task_id.clone())
        } else {
            None
        };

        Ok(TaskListPage {
            tasks: page,
            next_cursor,
        })
    }

    async fn recover_stuck_tasks(&self, max_age_ms: u64) -> Result<Vec<String>, TaskStorageError> {
        let mut tasks = self.tasks.write().await;
        let now = Utc::now();
        let mut recovered = Vec::new();

        for task in tasks.values_mut() {
            if state_machine::is_terminal(task.status) {
                continue;
            }

            if let Ok(created) = chrono::DateTime::parse_from_rfc3339(&task.last_updated_at) {
                let age_ms = (now - created.with_timezone(&Utc)).num_milliseconds();
                if age_ms > max_age_ms as i64 {
                    task.status = TaskStatus::Failed;
                    task.status_message = Some("Server restarted — task interrupted".to_string());
                    task.last_updated_at = Self::now_iso8601();
                    recovered.push(task.task_id.clone());
                }
            }
        }

        Ok(recovered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_task(task_id: &str, session_id: Option<&str>) -> TaskRecord {
        TaskRecord {
            task_id: task_id.to_string(),
            session_id: session_id.map(|s| s.to_string()),
            status: TaskStatus::Working,
            status_message: None,
            created_at: Utc::now().to_rfc3339(),
            last_updated_at: Utc::now().to_rfc3339(),
            ttl: None,
            poll_interval: None,
            original_method: "tools/call".to_string(),
            original_params: None,
            result: None,
            meta: None,
        }
    }

    fn make_task_with_time(task_id: &str, created_at: &str) -> TaskRecord {
        TaskRecord {
            task_id: task_id.to_string(),
            session_id: None,
            status: TaskStatus::Working,
            status_message: None,
            created_at: created_at.to_string(),
            last_updated_at: created_at.to_string(),
            ttl: None,
            poll_interval: None,
            original_method: "tools/call".to_string(),
            original_params: None,
            result: None,
            meta: None,
        }
    }

    #[tokio::test]
    async fn test_create_and_get_task() {
        let storage = InMemoryTaskStorage::new();
        let task = make_task("task-1", None);

        let created = storage.create_task(task).await.unwrap();
        assert_eq!(created.task_id, "task-1");
        assert_eq!(created.status, TaskStatus::Working);

        let fetched = storage.get_task("task-1").await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().task_id, "task-1");
    }

    #[tokio::test]
    async fn test_get_nonexistent_task() {
        let storage = InMemoryTaskStorage::new();
        let result = storage.get_task("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_task_lifecycle() {
        let storage = InMemoryTaskStorage::new();
        let task = make_task("task-life", None);
        storage.create_task(task).await.unwrap();

        // Working -> Completed
        let updated = storage
            .update_task_status("task-life", TaskStatus::Completed, Some("Done".to_string()))
            .await
            .unwrap();
        assert_eq!(updated.status, TaskStatus::Completed);
        assert_eq!(updated.status_message, Some("Done".to_string()));

        // Verify stored
        let fetched = storage.get_task("task-life").await.unwrap().unwrap();
        assert_eq!(fetched.status, TaskStatus::Completed);
    }

    #[tokio::test]
    async fn test_task_cancellation() {
        let storage = InMemoryTaskStorage::new();
        let task = make_task("task-cancel", None);
        storage.create_task(task).await.unwrap();

        let updated = storage
            .update_task_status(
                "task-cancel",
                TaskStatus::Cancelled,
                Some("User cancelled".to_string()),
            )
            .await
            .unwrap();
        assert_eq!(updated.status, TaskStatus::Cancelled);
    }

    #[tokio::test]
    async fn test_invalid_state_transition() {
        let storage = InMemoryTaskStorage::new();
        let task = make_task("task-invalid", None);
        storage.create_task(task).await.unwrap();

        // Complete the task
        storage
            .update_task_status("task-invalid", TaskStatus::Completed, None)
            .await
            .unwrap();

        // Completed -> Working should fail
        let result = storage
            .update_task_status("task-invalid", TaskStatus::Working, None)
            .await;
        assert!(result.is_err());
        match result.unwrap_err() {
            TaskStorageError::TerminalState(s) => assert_eq!(s, TaskStatus::Completed),
            other => panic!("Expected TerminalState, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_result_storage() {
        let storage = InMemoryTaskStorage::new();
        let task = make_task("task-result", None);
        storage.create_task(task).await.unwrap();

        let outcome = TaskOutcome::Success(json!({"content": [{"type": "text", "text": "done"}]}));
        storage
            .store_task_result("task-result", outcome)
            .await
            .unwrap();

        let result = storage.get_task_result("task-result").await.unwrap();
        assert!(result.is_some());
        match result.unwrap() {
            TaskOutcome::Success(v) => {
                assert_eq!(v["content"][0]["text"], "done");
            }
            _ => panic!("Expected Success"),
        }
    }

    #[tokio::test]
    async fn test_error_result_storage() {
        let storage = InMemoryTaskStorage::new();
        let task = make_task("task-err", None);
        storage.create_task(task).await.unwrap();

        let outcome = TaskOutcome::Error {
            code: -32010,
            message: "Tool failed".to_string(),
            data: Some(json!({"detail": "oops"})),
        };
        storage
            .store_task_result("task-err", outcome)
            .await
            .unwrap();

        let result = storage.get_task_result("task-err").await.unwrap().unwrap();
        match result {
            TaskOutcome::Error {
                code,
                message,
                data,
            } => {
                assert_eq!(code, -32010);
                assert_eq!(message, "Tool failed");
                assert_eq!(data.unwrap()["detail"], "oops");
            }
            _ => panic!("Expected Error"),
        }
    }

    #[tokio::test]
    async fn test_ttl_expiry() {
        let storage = InMemoryTaskStorage::new();

        // Create task with very short TTL and old timestamp
        let mut task = make_task("task-expire", None);
        task.ttl = Some(1); // 1ms TTL
        task.created_at = "2020-01-01T00:00:00Z".to_string();
        storage.create_task(task).await.unwrap();

        // Also create a task without TTL
        let task2 = make_task("task-keep", None);
        storage.create_task(task2).await.unwrap();

        let expired = storage.expire_tasks().await.unwrap();
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0], "task-expire");

        // Verify expired task is gone
        assert!(storage.get_task("task-expire").await.unwrap().is_none());
        // Verify other task still exists
        assert!(storage.get_task("task-keep").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_pagination() {
        let storage = InMemoryTaskStorage::new();

        // Create tasks with sequential timestamps for consistent ordering
        for i in 0..5 {
            let task =
                make_task_with_time(&format!("task-{}", i), &format!("2025-01-01T00:00:0{}Z", i));
            storage.create_task(task).await.unwrap();
        }

        // Page 1: limit 2
        let page1 = storage.list_tasks(None, Some(2)).await.unwrap();
        assert_eq!(page1.tasks.len(), 2);
        assert_eq!(page1.tasks[0].task_id, "task-0");
        assert_eq!(page1.tasks[1].task_id, "task-1");
        assert!(page1.next_cursor.is_some());

        // Page 2: using cursor from page 1
        let page2 = storage
            .list_tasks(page1.next_cursor.as_deref(), Some(2))
            .await
            .unwrap();
        assert_eq!(page2.tasks.len(), 2);
        assert_eq!(page2.tasks[0].task_id, "task-2");
        assert_eq!(page2.tasks[1].task_id, "task-3");

        // Page 3: last page
        let page3 = storage
            .list_tasks(page2.next_cursor.as_deref(), Some(2))
            .await
            .unwrap();
        assert_eq!(page3.tasks.len(), 1);
        assert_eq!(page3.tasks[0].task_id, "task-4");
        assert!(page3.next_cursor.is_none());
    }

    #[tokio::test]
    async fn test_session_binding() {
        let storage = InMemoryTaskStorage::new();

        storage
            .create_task(make_task("task-a", Some("session-1")))
            .await
            .unwrap();
        storage
            .create_task(make_task("task-b", Some("session-1")))
            .await
            .unwrap();
        storage
            .create_task(make_task("task-c", Some("session-2")))
            .await
            .unwrap();

        let session1_tasks = storage
            .list_tasks_for_session("session-1", None, None)
            .await
            .unwrap();
        assert_eq!(session1_tasks.tasks.len(), 2);

        let session2_tasks = storage
            .list_tasks_for_session("session-2", None, None)
            .await
            .unwrap();
        assert_eq!(session2_tasks.tasks.len(), 1);
        assert_eq!(session2_tasks.tasks[0].task_id, "task-c");

        let empty = storage
            .list_tasks_for_session("session-3", None, None)
            .await
            .unwrap();
        assert_eq!(empty.tasks.len(), 0);
    }

    #[tokio::test]
    async fn test_delete_task() {
        let storage = InMemoryTaskStorage::new();
        storage
            .create_task(make_task("task-del", None))
            .await
            .unwrap();

        assert!(storage.delete_task("task-del").await.unwrap());
        assert!(!storage.delete_task("task-del").await.unwrap()); // Already deleted
        assert!(storage.get_task("task-del").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_task_count() {
        let storage = InMemoryTaskStorage::new();
        assert_eq!(storage.task_count().await.unwrap(), 0);

        storage
            .create_task(make_task("task-1", None))
            .await
            .unwrap();
        assert_eq!(storage.task_count().await.unwrap(), 1);

        storage
            .create_task(make_task("task-2", None))
            .await
            .unwrap();
        assert_eq!(storage.task_count().await.unwrap(), 2);

        storage.delete_task("task-1").await.unwrap();
        assert_eq!(storage.task_count().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_max_tasks_limit() {
        let config = InMemoryTaskConfig {
            max_tasks: 2,
            ..Default::default()
        };
        let storage = InMemoryTaskStorage::with_config(config);

        storage
            .create_task(make_task("task-1", None))
            .await
            .unwrap();
        storage
            .create_task(make_task("task-2", None))
            .await
            .unwrap();

        let result = storage.create_task(make_task("task-3", None)).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            TaskStorageError::MaxTasksReached(n) => assert_eq!(n, 2),
            other => panic!("Expected MaxTasksReached, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_recover_stuck_tasks() {
        let storage = InMemoryTaskStorage::new();

        // Create a "stuck" working task with old timestamp
        let mut stuck = make_task("task-stuck", None);
        stuck.last_updated_at = "2020-01-01T00:00:00Z".to_string();
        storage.create_task(stuck).await.unwrap();

        // Create a recent working task that should NOT be recovered
        let recent = make_task("task-recent", None);
        storage.create_task(recent).await.unwrap();

        // Create a completed task that should NOT be touched
        let mut completed = make_task("task-done", None);
        completed.status = TaskStatus::Completed;
        completed.last_updated_at = "2020-01-01T00:00:00Z".to_string();
        storage.create_task(completed).await.unwrap();

        // Recover with 5 minute threshold
        let recovered = storage.recover_stuck_tasks(300_000).await.unwrap();
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0], "task-stuck");

        // Verify stuck task is now Failed
        let task = storage.get_task("task-stuck").await.unwrap().unwrap();
        assert_eq!(task.status, TaskStatus::Failed);
        assert_eq!(
            task.status_message,
            Some("Server restarted — task interrupted".to_string())
        );

        // Verify recent task is still Working
        let recent = storage.get_task("task-recent").await.unwrap().unwrap();
        assert_eq!(recent.status, TaskStatus::Working);

        // Verify completed task is untouched
        let done = storage.get_task("task-done").await.unwrap().unwrap();
        assert_eq!(done.status, TaskStatus::Completed);
    }

    #[tokio::test]
    async fn test_to_protocol_task() {
        let record = TaskRecord {
            task_id: "task-proto".to_string(),
            session_id: Some("sess-1".to_string()),
            status: TaskStatus::Working,
            status_message: Some("Processing".to_string()),
            created_at: "2025-01-01T00:00:00Z".to_string(),
            last_updated_at: "2025-01-01T00:00:01Z".to_string(),
            ttl: Some(60000),
            poll_interval: Some(5000),
            original_method: "tools/call".to_string(),
            original_params: None,
            result: None,
            meta: None,
        };

        let task = record.to_protocol_task();
        assert_eq!(task.task_id, "task-proto");
        assert_eq!(task.status, TaskStatus::Working);
        assert_eq!(task.status_message, Some("Processing".to_string()));
        assert_eq!(task.ttl, Some(60000));
        assert_eq!(task.poll_interval, Some(5000));
    }

    #[tokio::test]
    async fn test_task_outcome_serialization() {
        let success = TaskOutcome::Success(json!({"content": []}));
        let json = serde_json::to_string(&success).unwrap();
        let parsed: TaskOutcome = serde_json::from_str(&json).unwrap();
        match parsed {
            TaskOutcome::Success(v) => assert!(v["content"].is_array()),
            _ => panic!("Expected Success"),
        }

        let error = TaskOutcome::Error {
            code: -32603,
            message: "Internal error".to_string(),
            data: None,
        };
        let json = serde_json::to_string(&error).unwrap();
        let parsed: TaskOutcome = serde_json::from_str(&json).unwrap();
        match parsed {
            TaskOutcome::Error { code, message, .. } => {
                assert_eq!(code, -32603);
                assert_eq!(message, "Internal error");
            }
            _ => panic!("Expected Error"),
        }
    }

    #[tokio::test]
    async fn test_update_nonexistent_task() {
        let storage = InMemoryTaskStorage::new();
        let result = storage
            .update_task_status("nonexistent", TaskStatus::Completed, None)
            .await;
        assert!(result.is_err());
        match result.unwrap_err() {
            TaskStorageError::TaskNotFound(id) => assert_eq!(id, "nonexistent"),
            other => panic!("Expected TaskNotFound, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_input_required_transition() {
        let storage = InMemoryTaskStorage::new();
        storage
            .create_task(make_task("task-ir", None))
            .await
            .unwrap();

        // Working -> InputRequired
        storage
            .update_task_status(
                "task-ir",
                TaskStatus::InputRequired,
                Some("Need user input".to_string()),
            )
            .await
            .unwrap();

        // InputRequired -> Working (resume)
        storage
            .update_task_status("task-ir", TaskStatus::Working, Some("Resuming".to_string()))
            .await
            .unwrap();

        // Working -> Completed
        storage
            .update_task_status("task-ir", TaskStatus::Completed, None)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_generate_task_id() {
        let id1 = InMemoryTaskStorage::generate_task_id();
        let id2 = InMemoryTaskStorage::generate_task_id();
        assert_ne!(id1, id2);
        // UUID v7 should be parseable
        assert!(uuid::Uuid::parse_str(&id1).is_ok());
    }

    // === Parity tests ===

    #[tokio::test]
    async fn parity_create_and_retrieve() {
        let storage = InMemoryTaskStorage::new();
        crate::parity_tests::test_create_and_retrieve(&storage).await;
    }

    #[tokio::test]
    async fn parity_state_machine_enforcement() {
        let storage = InMemoryTaskStorage::new();
        crate::parity_tests::test_state_machine_enforcement(&storage).await;
    }

    #[tokio::test]
    async fn parity_terminal_state_rejection() {
        let storage = InMemoryTaskStorage::new();
        crate::parity_tests::test_terminal_state_rejection(&storage).await;
    }

    #[tokio::test]
    async fn parity_cursor_determinism() {
        let storage = InMemoryTaskStorage::new();
        crate::parity_tests::test_cursor_determinism(&storage).await;
    }

    #[tokio::test]
    async fn parity_session_scoping() {
        let storage = InMemoryTaskStorage::new();
        crate::parity_tests::test_session_scoping(&storage).await;
    }

    #[tokio::test]
    async fn parity_ttl_expiry() {
        let storage = InMemoryTaskStorage::new();
        crate::parity_tests::test_ttl_expiry(&storage).await;
    }

    #[tokio::test]
    async fn parity_task_result_round_trip() {
        let storage = InMemoryTaskStorage::new();
        crate::parity_tests::test_task_result_round_trip(&storage).await;
    }

    #[tokio::test]
    async fn parity_recover_stuck_tasks() {
        let storage = InMemoryTaskStorage::new();
        crate::parity_tests::test_recover_stuck_tasks(&storage).await;
    }

    #[tokio::test]
    async fn parity_max_tasks_limit() {
        let storage = InMemoryTaskStorage::with_config(InMemoryTaskConfig {
            max_tasks: 5,
            ..Default::default()
        });
        crate::parity_tests::test_max_tasks_limit(&storage, 5).await;
    }

    #[tokio::test]
    async fn parity_error_mapping() {
        let storage = InMemoryTaskStorage::new();
        crate::parity_tests::test_error_mapping_parity(&storage).await;
    }

    #[tokio::test]
    async fn parity_concurrent_status_updates() {
        let storage = std::sync::Arc::new(InMemoryTaskStorage::new());
        crate::parity_tests::test_concurrent_status_updates(storage).await;
    }
}
