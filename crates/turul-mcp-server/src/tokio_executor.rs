//! Tokio-based task executor — default in-process execution using tokio::spawn.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::{RwLock, watch};
use tracing::debug;

use turul_mcp_protocol::TaskStatus;
use turul_mcp_task_storage::{TaskOutcome, TaskStorageError};

use crate::cancellation::CancellationHandle;
use crate::task_executor::{BoxedTaskWork, TaskExecutor, TaskHandle};

struct TokioTaskEntry {
    cancellation: CancellationHandle,
    status_tx: watch::Sender<TaskStatus>,
}

/// In-process task executor using Tokio runtime.
pub struct TokioTaskExecutor {
    entries: Arc<RwLock<HashMap<String, TokioTaskEntry>>>,
}

impl TokioTaskExecutor {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for TokioTaskExecutor {
    fn default() -> Self {
        Self::new()
    }
}

struct TokioTaskHandle {
    cancellation: CancellationHandle,
}

impl TaskHandle for TokioTaskHandle {
    fn cancel(&self) {
        self.cancellation.cancel();
    }
    fn is_cancelled(&self) -> bool {
        self.cancellation.is_cancelled()
    }
}

#[async_trait]
impl TaskExecutor for TokioTaskExecutor {
    async fn start_task(
        &self,
        task_id: &str,
        work: BoxedTaskWork,
    ) -> Result<Box<dyn TaskHandle>, TaskStorageError> {
        let cancellation = CancellationHandle::new();
        let (status_tx, _) = watch::channel(TaskStatus::Working);

        let entry = TokioTaskEntry {
            cancellation: cancellation.clone(),
            status_tx: status_tx.clone(),
        };
        self.entries
            .write()
            .await
            .insert(task_id.to_string(), entry);

        let cancel_clone = cancellation.clone();
        let task_id_owned = task_id.to_string();
        let entries = Arc::clone(&self.entries);

        tokio::spawn(async move {
            let outcome = tokio::select! {
                result = (work)() => result,
                _ = cancel_clone.cancelled() => {
                    TaskOutcome::Error {
                        code: -32800,
                        message: "Task cancelled".to_string(),
                        data: None,
                    }
                }
            };

            let terminal_status = match &outcome {
                TaskOutcome::Success(_) => TaskStatus::Completed,
                TaskOutcome::Error { .. } => TaskStatus::Failed,
            };
            if let Some(entry) = entries.read().await.get(&task_id_owned) {
                let _ = entry.status_tx.send(terminal_status);
            }
            // Small delay to let watchers receive the notification before cleanup
            tokio::task::yield_now().await;
            entries.write().await.remove(&task_id_owned);

            debug!(task_id = %task_id_owned, status = ?terminal_status, "Task execution completed");
        });

        Ok(Box::new(TokioTaskHandle { cancellation }))
    }

    async fn cancel_task(&self, task_id: &str) -> Result<(), TaskStorageError> {
        if let Some(entry) = self.entries.read().await.get(task_id) {
            entry.cancellation.cancel();
            Ok(())
        } else {
            Err(TaskStorageError::TaskNotFound(task_id.to_string()))
        }
    }

    async fn await_terminal(&self, task_id: &str) -> Option<TaskStatus> {
        let mut rx = {
            let entries = self.entries.read().await;
            entries.get(task_id)?.status_tx.subscribe()
        };
        loop {
            if rx.changed().await.is_err() {
                // Sender dropped — task entry was cleaned up, meaning it completed
                return None;
            }
            let status = *rx.borrow();
            if turul_mcp_task_storage::is_terminal(status) {
                return Some(status);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_start_and_complete_task() {
        let executor = TokioTaskExecutor::new();
        let handle = executor
            .start_task(
                "task-1",
                Box::new(|| {
                    Box::pin(async { TaskOutcome::Success(serde_json::json!({"result": 42})) })
                }),
            )
            .await
            .unwrap();

        // Task should complete
        let status = executor.await_terminal("task-1").await;
        assert!(matches!(status, Some(TaskStatus::Completed)));
        assert!(!handle.is_cancelled());
    }

    #[tokio::test]
    async fn test_cancel_task() {
        let executor = TokioTaskExecutor::new();
        let handle = executor
            .start_task(
                "task-2",
                Box::new(|| {
                    Box::pin(async {
                        // Simulate long-running work
                        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                        TaskOutcome::Success(serde_json::json!({}))
                    })
                }),
            )
            .await
            .unwrap();

        // Cancel it
        executor.cancel_task("task-2").await.unwrap();
        assert!(handle.is_cancelled());
    }

    #[tokio::test]
    async fn test_cancel_nonexistent_task() {
        let executor = TokioTaskExecutor::new();
        let result = executor.cancel_task("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_await_terminal_nonexistent() {
        let executor = TokioTaskExecutor::new();
        let result = executor.await_terminal("nonexistent").await;
        assert!(result.is_none());
    }
}
