//! In-memory [`TaskStore`] — the reference backend.
//!
//! Single-process: tasks do not survive a restart, and a second instance sees
//! none of them. Correct for tests and single-node development; the durable
//! backends alongside it are what a deployment with more than one instance
//! needs.
//!
//! Note how little is here. Every status rule, owner check and `tasks/update`
//! key decision lives in [`super::traits`] as a pure transition; a backend is
//! only responsible for *load, apply, store* under its own atomicity
//! boundary. That is what makes cross-backend parity structural rather than
//! aspirational. Module layout mirrors `turul-mcp-session-storage` and
//! `turul-mcp-server-state-storage`.

use std::collections::HashMap;
use std::sync::RwLock;

use serde_json::Value;
use turul_mcp_protocol_2026_07_28::input_required::InputRequests;

use super::traits::{
    InputDelivery, RetentionPolicy, SweepAction, SweepReport, TaskState, TaskStore, TaskStoreError,
    apply_cancel, apply_complete, apply_fail, apply_provide_input, apply_require_input,
    now_rfc3339, owner_matches, sweep_action, sweep_error,
};
use super::types::TaskStatus;

/// In-memory [`TaskStore`] — single-process; tasks do not survive restarts.
#[derive(Default)]
pub struct InMemoryTaskStore {
    tasks: RwLock<HashMap<String, TaskState>>,
}

impl InMemoryTaskStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load, apply a transition, store — the shape every backend repeats
    /// against its own storage. The `RwLock` write guard is this backend's
    /// atomicity boundary; SQL backends use a transaction instead.
    fn with_task<T>(
        &self,
        task_id: &str,
        f: impl FnOnce(&mut TaskState) -> Result<T, TaskStoreError>,
    ) -> Result<T, TaskStoreError> {
        let mut tasks = self.tasks.write().expect("task store lock");
        let state = tasks
            .get_mut(task_id)
            .ok_or_else(|| TaskStoreError::NotFound(task_id.to_string()))?;
        f(state)
    }
}

#[async_trait::async_trait]
impl TaskStore for InMemoryTaskStore {
    async fn create(&self, state: TaskState) -> Result<(), TaskStoreError> {
        let mut tasks = self.tasks.write().expect("task store lock");
        tasks.insert(state.fields.task_id.clone(), state);
        Ok(())
    }

    async fn get(
        &self,
        task_id: &str,
        owner: Option<&str>,
    ) -> Result<Option<TaskState>, TaskStoreError> {
        Ok(self
            .tasks
            .read()
            .expect("task store lock")
            .get(task_id)
            .filter(|state| owner_matches(state, owner))
            .cloned())
    }

    async fn complete(&self, task_id: &str, result: Value) -> Result<TaskState, TaskStoreError> {
        self.with_task(task_id, |state| {
            apply_complete(state, result)?;
            Ok(state.clone())
        })
    }

    async fn fail(&self, task_id: &str, error: Value) -> Result<TaskState, TaskStoreError> {
        self.with_task(task_id, |state| {
            apply_fail(state, error)?;
            Ok(state.clone())
        })
    }

    async fn cancel(
        &self,
        task_id: &str,
        owner: Option<&str>,
    ) -> Result<Option<TaskState>, TaskStoreError> {
        self.with_task(task_id, |state| {
            Ok(apply_cancel(state, owner)?.then(|| state.clone()))
        })
    }

    async fn require_input(
        &self,
        task_id: &str,
        requests: InputRequests,
        request_state: Option<String>,
    ) -> Result<TaskState, TaskStoreError> {
        self.with_task(task_id, |state| {
            apply_require_input(state, requests, request_state)?;
            Ok(state.clone())
        })
    }

    async fn provide_input(
        &self,
        task_id: &str,
        owner: Option<&str>,
        responses: HashMap<String, Value>,
    ) -> Result<InputDelivery, TaskStoreError> {
        self.with_task(task_id, |state| {
            apply_provide_input(state, owner, responses)
        })
    }

    async fn sweep(
        &self,
        now: chrono::DateTime<chrono::Utc>,
        policy: &RetentionPolicy,
    ) -> Result<SweepReport, TaskStoreError> {
        let mut tasks = self.tasks.write().expect("task store lock");
        let mut report = SweepReport::default();

        tasks.retain(|id, state| match sweep_action(state, now, policy) {
            SweepAction::Delete => {
                report.deleted.push(id.clone());
                false
            }
            SweepAction::MarkFailed(reason) => {
                state.status = TaskStatus::Failed;
                state.error = Some(sweep_error(reason));
                state.input_requests = None;
                state.fields.last_updated_at = now_rfc3339();
                report.failed.push(id.clone());
                true
            }
            SweepAction::Keep => true,
        });
        Ok(report)
    }
}
