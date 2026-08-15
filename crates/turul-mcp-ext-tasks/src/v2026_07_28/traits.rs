//! Durable task state and the `TaskStore` abstraction (the upstream
//! overview's "Task Store": reachable by `tasks/get` even if the worker or
//! connection has died; a `CreateTaskResult` is not returned until the task
//! is findable here).
//!
//! No tokio in the public API — implementations choose their own runtime.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use turul_mcp_protocol_2026_07_28::input_required::{InputRequests, InputResponses};

use super::types::{DetailedTask, TaskFields, TaskStatus};

/// Full stored state of one task; [`TaskState::to_detailed`] renders the
/// wire `DetailedTask` for `tasks/get` and `notifications/tasks`.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// A response for a key the task IS waiting on could not be read as an
    /// `InputResponse`. Distinct from a response for a key it is not waiting
    /// on, which is inert and ignored — see `TaskStore::provide_input`.
    #[error("input response for {key:?} is not a valid InputResponse: {detail}")]
    InvalidInputResponse { key: String, detail: String },
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

    /// Record `tasks/update` responses. Partial delivery keeps the task
    /// `input_required`, full delivery transitions it back to `working` and
    /// hands the responses to the caller for worker resumption. A task bound
    /// to a different `owner` yields `Err(TaskStoreError::NotFound)`, the same
    /// error an unknown task id produces.
    ///
    /// `responses` arrives **untyped** because outstanding-ness is task state:
    /// only here, holding the task, can a key be judged. Two cases that look
    /// alike on the wire are handled differently, and deliberately:
    ///
    /// - **A key the task is not waiting on is ignored**, and delivery still
    ///   succeeds. Such a key is inert — it cannot change task state — and
    ///   SEP-2663's ack-only design (vendored SEP line 930) reserves errors
    ///   for "clearly invalid requests — such as an unknown `taskId`". The
    ///   spec's "each key MUST correspond" binds the *client*; answering an
    ///   error gives it nothing it can act on. Implementations SHOULD log it.
    /// - **A malformed response for a key the task IS waiting on is an
    ///   error** ([`TaskStoreError::InvalidInputResponse`]), because it
    ///   genuinely prevents the round from completing, and the error names the
    ///   key so the client can fix the right one.
    async fn provide_input(
        &self,
        task_id: &str,
        owner: Option<&str>,
        responses: std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<InputDelivery, TaskStoreError>;

    /// Apply `policy`, marking abandoned/expired tasks `failed` and deleting
    /// old terminal ones. Returns what it did.
    ///
    /// Deliberately **not** defaulted: a backend that quietly grew forever is
    /// what this trait had before, and a default no-op would let the next
    /// backend do the same without anyone noticing. `now` is a parameter so
    /// the boundary conditions are testable without waiting.
    ///
    /// This is a maintenance operation, not a hot path — callers should run
    /// it on a timer, not per request.
    async fn sweep(
        &self,
        now: chrono::DateTime<chrono::Utc>,
        policy: &RetentionPolicy,
    ) -> Result<SweepReport, TaskStoreError>;

    /// Tell the backend the retention policy in force, so one that can expire
    /// items *itself* configures that at write time rather than relying on
    /// [`Self::sweep`].
    ///
    /// Called once by the server builder when retention is configured, so
    /// there is ONE place an operator sets this. DynamoDB needs it — its TTL
    /// is a per-item attribute written on every put, not a query-time policy —
    /// and without this hook a server configured through the builder would
    /// silently never write that attribute, leaving native expiry off while
    /// looking configured. Backends that only sweep ignore it.
    fn configure_retention(&self, _policy: &RetentionPolicy) {}
}

/// True when `state` may be accessed by `owner`: an owned task only answers
/// to its own owner; an unowned task (created without an authenticated
/// principal) answers to anyone, matching the deployment's own no-isolation
/// posture rather than pretending to isolate callers it cannot identify.
pub(super) fn owner_matches(state: &TaskState, owner: Option<&str>) -> bool {
    match state.owner.as_deref() {
        Some(bound) => Some(bound) == owner,
        None => true,
    }
}

// ---------------------------------------------------------------------------
// The state machine, defined once.
//
// Every backend loads a `TaskState`, applies one of these, and stores it back.
// They are pure and synchronous: no I/O, no clock beyond `now_rfc3339`, no
// storage assumptions. That is deliberate — SEP-2663's status rules, the
// owner binding and the `tasks/update` key handling are ONE algorithm, and a
// backend that re-implemented them would drift silently from the others. The
// parity harness proves the wiring; these functions remove most of what could
// have gone wrong in the first place.
// ---------------------------------------------------------------------------

/// The wire name of a status, for error messages.
pub fn status_name(s: TaskStatus) -> &'static str {
    match s {
        TaskStatus::Working => "working",
        TaskStatus::InputRequired => "input_required",
        TaskStatus::Completed => "completed",
        TaskStatus::Failed => "failed",
        TaskStatus::Cancelled => "cancelled",
    }
}

/// Millisecond-precision UTC, the format every timestamp on this surface uses.
pub fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn invalid(task_id: &str, status: TaskStatus, operation: &'static str) -> TaskStoreError {
    TaskStoreError::InvalidStatus {
        task_id: task_id.to_string(),
        status: status_name(status),
        operation,
    }
}

/// `working`/`input_required` → `completed`.
pub fn apply_complete(state: &mut TaskState, result: Value) -> Result<(), TaskStoreError> {
    if state.status.is_terminal() {
        return Err(invalid(&state.fields.task_id, state.status, "complete"));
    }
    state.status = TaskStatus::Completed;
    state.result = Some(result);
    state.input_requests = None;
    state.fields.last_updated_at = now_rfc3339();
    Ok(())
}

/// `working`/`input_required` → `failed`.
pub fn apply_fail(state: &mut TaskState, error: Value) -> Result<(), TaskStoreError> {
    if state.status.is_terminal() {
        return Err(invalid(&state.fields.task_id, state.status, "fail"));
    }
    state.status = TaskStatus::Failed;
    state.error = Some(error);
    state.input_requests = None;
    state.fields.last_updated_at = now_rfc3339();
    Ok(())
}

/// Cooperative cancel. `Ok(false)` means the task was already terminal and
/// nothing changed — the caller acks without writing.
pub fn apply_cancel(state: &mut TaskState, owner: Option<&str>) -> Result<bool, TaskStoreError> {
    if !owner_matches(state, owner) {
        return Err(TaskStoreError::NotFound(state.fields.task_id.clone()));
    }
    if state.status.is_terminal() {
        return Ok(false);
    }
    state.status = TaskStatus::Cancelled;
    state.input_requests = None;
    state.fields.last_updated_at = now_rfc3339();
    Ok(true)
}

/// `working` → `input_required`.
pub fn apply_require_input(
    state: &mut TaskState,
    requests: InputRequests,
    request_state: Option<String>,
) -> Result<(), TaskStoreError> {
    if state.status != TaskStatus::Working {
        return Err(invalid(
            &state.fields.task_id,
            state.status,
            "require_input",
        ));
    }
    state.status = TaskStatus::InputRequired;
    state.input_requests = Some(requests);
    state.collected_responses = InputResponses::new();
    state.request_state = request_state;
    state.fields.last_updated_at = now_rfc3339();
    Ok(())
}

/// Record `tasks/update` responses. See [`TaskStore::provide_input`] for why
/// inert keys are ignored and malformed outstanding ones are not.
pub fn apply_provide_input(
    state: &mut TaskState,
    owner: Option<&str>,
    responses: HashMap<String, Value>,
) -> Result<InputDelivery, TaskStoreError> {
    if !owner_matches(state, owner) {
        return Err(TaskStoreError::NotFound(state.fields.task_id.clone()));
    }
    if state.status != TaskStatus::InputRequired {
        return Err(invalid(
            &state.fields.task_id,
            state.status,
            "provide_input",
        ));
    }
    let outstanding = state.input_requests.clone().unwrap_or_default();

    // Keys the task is not waiting on are inert: drop them and carry on.
    // Only here, holding the task, is "outstanding" knowable at all.
    let mut typed = InputResponses::new();
    for (key, value) in responses {
        if !outstanding.contains_key(&key) {
            tracing::debug!(
                task_id = %state.fields.task_id,
                key,
                "tasks/update carried a response for a key this task is not waiting on; ignored"
            );
            continue;
        }
        let parsed =
            serde_json::from_value(value).map_err(|e| TaskStoreError::InvalidInputResponse {
                key: key.clone(),
                detail: e.to_string(),
            })?;
        typed.insert(key, parsed);
    }
    state.collected_responses.extend(typed);
    state.fields.last_updated_at = now_rfc3339();

    if outstanding
        .keys()
        .all(|k| state.collected_responses.contains_key(k))
    {
        state.status = TaskStatus::Working;
        state.input_requests = None;
        // The responses stay IN the stored state rather than being moved out.
        // The worker that resumes the task may be on another instance, and it
        // learns the round completed by reading the store — so the answers
        // have to still be there when it looks. `apply_require_input` clears
        // them at the start of the next round, so they cannot leak into it.
        Ok(InputDelivery::Complete {
            responses: state.collected_responses.clone(),
            request_state: state.request_state.clone(),
        })
    } else {
        // Partial fulfilment: drop the keys just answered so a following
        // `tasks/get` advertises only what is STILL outstanding. Leaving them
        // in told the client to answer questions it had already answered, with
        // no way to tell the difference. Found by `tasks-mrtr-input`.
        if let Some(reqs) = state.input_requests.as_mut() {
            reqs.retain(|k, _| !state.collected_responses.contains_key(k));
        }
        Ok(InputDelivery::Partial)
    }
}

// ---------------------------------------------------------------------------
// Retention.
//
// The in-memory store solved retention by accident: a restart lost every
// task. A durable backend has no such luck — a table only grows — so the
// policy has to be explicit, and it has to be the same policy everywhere.
//
// SEP-2663 makes all of this OPTIONAL (line 165: "The server may discard the
// task after the TTL elapses"; line 342: "servers MAY mark a task as `failed`
// at any point after the TTL elapses, and subsequently delete it at any
// time"). Nothing here is required for compliance. It is required for a
// deployment that does not want an unbounded table.
// ---------------------------------------------------------------------------

/// What a sweep may do. Every field is opt-in: `RetentionPolicy::default()`
/// changes nothing, which keeps the previous behaviour for anyone who does
/// not ask for retention.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RetentionPolicy {
    /// A non-terminal task untouched for this long is presumed abandoned by
    /// an instance that died mid-execution, and is marked `failed`. This is
    /// the 2026 equivalent of the 2025 store's `recover_stuck_tasks`.
    ///
    /// Must exceed the longest legitimate gap between worker writes, or a
    /// slow-but-healthy task gets killed. There is no way for the store to
    /// tell those apart — only elapsed silence is observable.
    pub orphan_after_ms: Option<u64>,
    /// A terminal task older than this is deleted outright.
    pub delete_terminal_after_ms: Option<u64>,
    /// Honour each task's own `ttlMs`: past `createdAt + ttlMs`, mark it
    /// `failed`. `ttlMs: null` means unlimited and is never swept.
    pub honour_task_ttl: bool,
}

/// What [`sweep_action`] decided for one task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SweepAction {
    /// Leave it alone.
    Keep,
    /// Mark `failed` with this reason, which becomes the JSON-RPC error
    /// message a later `tasks/get` reports.
    MarkFailed(&'static str),
    /// Remove the row entirely.
    Delete,
}

/// What a sweep did, so a caller can log or assert on it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SweepReport {
    /// Non-terminal tasks moved to `failed` (orphaned, or TTL-expired).
    pub failed: Vec<String>,
    /// Terminal tasks deleted.
    pub deleted: Vec<String>,
}

/// Decide one task's fate. Pure, so every backend agrees by construction and
/// the parity harness can pin the boundaries rather than each backend's
/// interpretation of them.
///
/// Order matters: deletion of terminal tasks is considered before TTL, since
/// a task that is both terminal and past its TTL should go, not be re-failed.
pub fn sweep_action(
    state: &TaskState,
    now: chrono::DateTime<chrono::Utc>,
    policy: &RetentionPolicy,
) -> SweepAction {
    let age_ms = |stamp: &str| -> Option<i64> {
        chrono::DateTime::parse_from_rfc3339(stamp)
            .ok()
            .map(|t| (now - t.with_timezone(&chrono::Utc)).num_milliseconds())
    };

    if state.status.is_terminal() {
        if let Some(limit) = policy.delete_terminal_after_ms
            && let Some(age) = age_ms(&state.fields.last_updated_at)
            && age >= limit as i64
        {
            return SweepAction::Delete;
        }
        // A terminal task is never re-failed, whatever its TTL says.
        return SweepAction::Keep;
    }

    if policy.honour_task_ttl
        && let Some(ttl) = state.fields.ttl_ms.0
        && let Some(age) = age_ms(&state.fields.created_at)
        && age >= ttl as i64
    {
        return SweepAction::MarkFailed("task exceeded its ttlMs");
    }

    if let Some(limit) = policy.orphan_after_ms
        && let Some(age) = age_ms(&state.fields.last_updated_at)
        && age >= limit as i64
    {
        return SweepAction::MarkFailed(
            "task was abandoned: no progress within the orphan threshold",
        );
    }

    SweepAction::Keep
}

/// The JSON-RPC error a swept task reports from then on.
pub fn sweep_error(reason: &str) -> Value {
    serde_json::json!({ "code": -32603, "message": reason })
}
