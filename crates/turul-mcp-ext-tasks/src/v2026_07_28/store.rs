//! Durable task state and the `TaskStore` abstraction (the upstream
//! overview's "Task Store": reachable by `tasks/get` even if the worker or
//! connection has died; a `CreateTaskResult` is not returned until the task
//! is findable here).
//!
//! No tokio in the public API — implementations choose their own runtime.

use std::collections::HashMap;
use std::sync::RwLock;

use serde_json::Value;
use turul_mcp_protocol_2026_07_28::input_required::{InputRequests, InputResponses};

use super::types::{DetailedTask, TaskFields, TaskStatus};

/// Full stored state of one task; [`TaskState::to_detailed`] renders the
/// wire `DetailedTask` for `tasks/get` and `notifications/tasks`.
#[derive(Debug, Clone)]
pub struct TaskState {
    pub fields: TaskFields,
    pub status: TaskStatus,
    /// The authenticated principal (verified token subject) this task is
    /// bound to, or `None` when the request that created it carried no
    /// authenticated principal — see [`TaskStore::get`], [`TaskStore::cancel`],
    /// and [`TaskStore::provide_input`] for how this is enforced.
    pub owner: Option<String>,
    /// Outstanding server→client requests while `input_required`.
    pub input_requests: Option<InputRequests>,
    /// Responses collected so far for the current `input_required` round
    /// (the server may accept partial responses; the task stays
    /// `input_required` until all outstanding keys are answered).
    pub collected_responses: InputResponses,
    /// Opaque MRTR state the tool attached to its input demand, replayed to
    /// the tool when the worker resumes.
    pub request_state: Option<String>,
    /// Final result when `completed` — the structure the original request
    /// would have returned synchronously.
    pub result: Option<Value>,
    /// JSON-RPC error object when `failed`.
    pub error: Option<Value>,
}

impl TaskState {
    pub fn to_detailed(&self) -> DetailedTask {
        let fields = self.fields.clone();
        match self.status {
            TaskStatus::Working => DetailedTask::Working { fields },
            TaskStatus::InputRequired => DetailedTask::InputRequired {
                fields,
                input_requests: self.input_requests.clone().unwrap_or_default(),
            },
            TaskStatus::Completed => DetailedTask::Completed {
                fields,
                result: self.result.clone().unwrap_or(Value::Null),
            },
            TaskStatus::Failed => DetailedTask::Failed {
                fields,
                error: self.error.clone().unwrap_or(Value::Null),
            },
            TaskStatus::Cancelled => DetailedTask::Cancelled { fields },
        }
    }
}

/// Errors from [`TaskStore`] operations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TaskStoreError {
    #[error("task {0:?} not found")]
    NotFound(String),
    #[error("task {task_id:?} is {status:?}; {operation} is not valid in that status")]
    InvalidStatus {
        task_id: String,
        status: &'static str,
        operation: &'static str,
    },
    #[error("input response key {0:?} does not match an outstanding input request")]
    UnknownInputKey(String),
    #[error("storage backend error: {0}")]
    Backend(String),
}

/// Outcome of delivering input responses via `tasks/update`.
#[derive(Debug, Clone)]
pub enum InputDelivery {
    /// Every outstanding input request is now answered: the full response
    /// set (plus the tool's echoed `requestState`) is ready for the worker
    /// to resume with; the task has transitioned back to `working`.
    Complete {
        responses: InputResponses,
        request_state: Option<String>,
    },
    /// Some outstanding requests remain unanswered; the task stays
    /// `input_required`.
    Partial,
}

/// Async task store. Implementations enforce the SEP-2663 state machine:
/// `working ⇄ input_required`, both → `completed`/`failed`/`cancelled`,
/// terminal states immutable.
///
/// A task id is a state handle within the meaning of the Security Best
/// Practices "State Handle Hijacking" guidance: possession of the id MUST
/// NOT be treated as authentication. The three client-facing entry points
/// ([`get`](TaskStore::get), [`cancel`](TaskStore::cancel),
/// [`provide_input`](TaskStore::provide_input)) therefore take the caller's
/// `owner` — the authenticated principal from the current request, or
/// `None` when the deployment has no authentication — and MUST reject
/// (indistinguishably from "unknown task") a task bound to a different
/// owner. `create`, `complete`, `fail`, and `require_input` are driven by
/// the worker that owns the task, not by an inbound client request, so they
/// carry no owner check.
#[async_trait::async_trait]
pub trait TaskStore: Send + Sync {
    /// Persist a new task (status `working`). The caller must not answer the
    /// originating request until this returns.
    async fn create(&self, state: TaskState) -> Result<(), TaskStoreError>;

    /// Fetch a task for `tasks/get`. Returns `Ok(None)` both when the task
    /// does not exist and when it exists but is bound to a different
    /// `owner` — the two cases MUST be indistinguishable to the caller.
    async fn get(
        &self,
        task_id: &str,
        owner: Option<&str>,
    ) -> Result<Option<TaskState>, TaskStoreError>;

    /// `working`/`input_required` → `completed` with the final result.
    async fn complete(&self, task_id: &str, result: Value) -> Result<TaskState, TaskStoreError>;

    /// `working`/`input_required` → `failed` with a JSON-RPC error object.
    async fn fail(&self, task_id: &str, error: Value) -> Result<TaskState, TaskStoreError>;

    /// Cooperative cancel for `tasks/cancel`: non-terminal → `cancelled`
    /// (returns the new state); already-terminal returns `Ok(None)` —
    /// callers ack either way. A task bound to a different `owner` yields
    /// `Err(TaskStoreError::NotFound)`, the same error an unknown task id
    /// produces.
    async fn cancel(
        &self,
        task_id: &str,
        owner: Option<&str>,
    ) -> Result<Option<TaskState>, TaskStoreError>;

    /// `working` → `input_required` with the given outstanding requests.
    async fn require_input(
        &self,
        task_id: &str,
        requests: InputRequests,
        request_state: Option<String>,
    ) -> Result<TaskState, TaskStoreError>;

    /// Record `tasks/update` responses. Keys must match outstanding input
    /// requests; partial delivery keeps the task `input_required`, full
    /// delivery transitions it back to `working` and hands the responses to
    /// the caller for worker resumption. A task bound to a different
    /// `owner` yields `Err(TaskStoreError::NotFound)`, the same error an
    /// unknown task id produces.
    async fn provide_input(
        &self,
        task_id: &str,
        owner: Option<&str>,
        responses: InputResponses,
    ) -> Result<InputDelivery, TaskStoreError>;
}

/// True when `state` may be accessed by `owner`: an owned task only answers
/// to its own owner; an unowned task (created without an authenticated
/// principal) answers to anyone, matching the deployment's own no-isolation
/// posture rather than pretending to isolate callers it cannot identify.
fn owner_matches(state: &TaskState, owner: Option<&str>) -> bool {
    match state.owner.as_deref() {
        Some(bound) => Some(bound) == owner,
        None => true,
    }
}

/// In-memory [`TaskStore`] — single-process; tasks do not survive restarts.
#[derive(Default)]
pub struct InMemoryTaskStore {
    tasks: RwLock<HashMap<String, TaskState>>,
}

impl InMemoryTaskStore {
    pub fn new() -> Self {
        Self::default()
    }
}

fn status_name(s: TaskStatus) -> &'static str {
    match s {
        TaskStatus::Working => "working",
        TaskStatus::InputRequired => "input_required",
        TaskStatus::Completed => "completed",
        TaskStatus::Failed => "failed",
        TaskStatus::Cancelled => "cancelled",
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
        let mut tasks = self.tasks.write().expect("task store lock");
        let state = tasks
            .get_mut(task_id)
            .ok_or_else(|| TaskStoreError::NotFound(task_id.to_string()))?;
        if state.status.is_terminal() {
            return Err(TaskStoreError::InvalidStatus {
                task_id: task_id.to_string(),
                status: status_name(state.status),
                operation: "complete",
            });
        }
        state.status = TaskStatus::Completed;
        state.result = Some(result);
        state.input_requests = None;
        state.fields.last_updated_at = now_rfc3339();
        Ok(state.clone())
    }

    async fn fail(&self, task_id: &str, error: Value) -> Result<TaskState, TaskStoreError> {
        let mut tasks = self.tasks.write().expect("task store lock");
        let state = tasks
            .get_mut(task_id)
            .ok_or_else(|| TaskStoreError::NotFound(task_id.to_string()))?;
        if state.status.is_terminal() {
            return Err(TaskStoreError::InvalidStatus {
                task_id: task_id.to_string(),
                status: status_name(state.status),
                operation: "fail",
            });
        }
        state.status = TaskStatus::Failed;
        state.error = Some(error);
        state.input_requests = None;
        state.fields.last_updated_at = now_rfc3339();
        Ok(state.clone())
    }

    async fn cancel(
        &self,
        task_id: &str,
        owner: Option<&str>,
    ) -> Result<Option<TaskState>, TaskStoreError> {
        let mut tasks = self.tasks.write().expect("task store lock");
        let state = tasks
            .get_mut(task_id)
            .ok_or_else(|| TaskStoreError::NotFound(task_id.to_string()))?;
        if !owner_matches(state, owner) {
            return Err(TaskStoreError::NotFound(task_id.to_string()));
        }
        if state.status.is_terminal() {
            return Ok(None);
        }
        state.status = TaskStatus::Cancelled;
        state.input_requests = None;
        state.fields.last_updated_at = now_rfc3339();
        Ok(Some(state.clone()))
    }

    async fn require_input(
        &self,
        task_id: &str,
        requests: InputRequests,
        request_state: Option<String>,
    ) -> Result<TaskState, TaskStoreError> {
        let mut tasks = self.tasks.write().expect("task store lock");
        let state = tasks
            .get_mut(task_id)
            .ok_or_else(|| TaskStoreError::NotFound(task_id.to_string()))?;
        if state.status != TaskStatus::Working {
            return Err(TaskStoreError::InvalidStatus {
                task_id: task_id.to_string(),
                status: status_name(state.status),
                operation: "require_input",
            });
        }
        state.status = TaskStatus::InputRequired;
        state.input_requests = Some(requests);
        state.collected_responses = InputResponses::new();
        state.request_state = request_state;
        state.fields.last_updated_at = now_rfc3339();
        Ok(state.clone())
    }

    async fn provide_input(
        &self,
        task_id: &str,
        owner: Option<&str>,
        responses: InputResponses,
    ) -> Result<InputDelivery, TaskStoreError> {
        let mut tasks = self.tasks.write().expect("task store lock");
        let state = tasks
            .get_mut(task_id)
            .ok_or_else(|| TaskStoreError::NotFound(task_id.to_string()))?;
        if !owner_matches(state, owner) {
            return Err(TaskStoreError::NotFound(task_id.to_string()));
        }
        if state.status != TaskStatus::InputRequired {
            return Err(TaskStoreError::InvalidStatus {
                task_id: task_id.to_string(),
                status: status_name(state.status),
                operation: "provide_input",
            });
        }
        let outstanding = state.input_requests.clone().unwrap_or_default();
        for key in responses.keys() {
            if !outstanding.contains_key(key) {
                return Err(TaskStoreError::UnknownInputKey(key.clone()));
            }
        }
        state.collected_responses.extend(responses);
        state.fields.last_updated_at = now_rfc3339();

        if outstanding
            .keys()
            .all(|k| state.collected_responses.contains_key(k))
        {
            state.status = TaskStatus::Working;
            state.input_requests = None;
            let responses = std::mem::take(&mut state.collected_responses);
            Ok(InputDelivery::Complete {
                responses,
                request_state: state.request_state.clone(),
            })
        } else {
            Ok(InputDelivery::Partial)
        }
    }
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}
