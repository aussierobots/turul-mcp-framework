//! Server-side runtime for the Tasks extension (`io.modelcontextprotocol/tasks`,
//! SEP-2663) on the 2026-07-28 stateless lane.
//!
//! Architecture follows the upstream overview: the **Task Store** (durable
//! state, `turul_mcp_ext_tasks::TaskStore`) is written before the
//! `CreateTaskResult` is answered; the **Worker** (the spawned tool
//! execution) updates the store as it progresses and writes the final
//! result or error.
//!
//! MRTR bridge: a task-augmented tool that returns
//! [`McpError::InputRequired`] parks its task in `input_required`;
//! `tasks/update` collects the responses and, once every outstanding request
//! is answered, the worker resumes with the responses injected through the
//! SAME session-extension keys the synchronous MRTR retry leg uses — tool
//! code is identical under both execution models.
//!
//! Task ids are state handles (Security Best Practices "State Handle
//! Hijacking"): a task is bound at creation to the caller's authenticated
//! principal (the verified `sub` claim the OAuth middleware leaves at
//! `__turul_internal.auth_claims`, read via `owner_from_session`), and
//! `tasks/get`/`tasks/update`/`tasks/cancel` reject a task id presented by
//! any other principal, indistinguishably from an unknown id. When a
//! deployment has no authentication configured, no request carries a
//! principal, every task is created with no owner, and no cross-caller
//! isolation applies — this is a deliberate, documented consequence of
//! running the extension without authentication, not a silent gap: an
//! operator who needs isolation between mutually-untrusting callers must
//! configure OAuth.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::Value;
use tracing::{debug, warn};
use turul_mcp_ext_tasks::v2026_07_28::lifecycle::{
    CancelTaskParams, GetTaskParams, METHOD_NOTIFICATIONS_TASKS, UpdateTaskParams,
};
use turul_mcp_ext_tasks::v2026_07_28::store::{
    InputDelivery, TaskState, TaskStore, TaskStoreError,
};
use turul_mcp_ext_tasks::v2026_07_28::types::{
    CreateTaskResult, Nullable, RESULT_TYPE_TASK, Task, TaskFields, TaskStatus,
};
use turul_mcp_protocol::McpError;
use turul_mcp_protocol::input_required::InputResponses;
use turul_mcp_protocol::result_type::ResultType;

use crate::McpResult;
use crate::handlers::McpHandler;
use crate::session::SessionContext;
use crate::tool::McpTool;

/// Suggested polling interval handed to clients on every task.
const POLL_INTERVAL_MS: f64 = 500.0;

/// The caller's authenticated principal for this request, or `None` when
/// the request carries no verified identity (no OAuth configured, or the
/// claim is absent/empty). Reads the same `__turul_internal.auth_claims`
/// extension `OAuthResourceMiddleware` populates from a validated Bearer
/// token's `sub` claim.
fn owner_from_session(session: &Option<SessionContext>) -> Option<String> {
    session
        .as_ref()
        .and_then(|s| {
            s.get_typed_extension::<turul_mcp_oauth::TokenClaims>("__turul_internal.auth_claims")
        })
        .map(|claims| claims.sub)
        .filter(|sub| !sub.is_empty())
}

struct Waiter {
    sender: tokio::sync::oneshot::Sender<(InputResponses, Option<String>)>,
}

/// Runtime pairing the durable [`TaskStore`] with in-process workers
/// (abort handles for cooperative cancel, waiters for input resumption).
pub struct ExtTasksRuntime {
    store: Arc<dyn TaskStore>,
    aborts: Mutex<HashMap<String, tokio::task::AbortHandle>>,
    waiters: Mutex<HashMap<String, Waiter>>,
}

impl ExtTasksRuntime {
    pub fn new(store: Arc<dyn TaskStore>) -> Self {
        Self {
            store,
            aborts: Mutex::new(HashMap::new()),
            waiters: Mutex::new(HashMap::new()),
        }
    }

    pub fn store(&self) -> &Arc<dyn TaskStore> {
        &self.store
    }

    /// Create the task durably, spawn the worker, and return the
    /// `CreateTaskResult` for the originating `tools/call`.
    pub async fn create_and_spawn(
        self: &Arc<Self>,
        tool: Arc<dyn McpTool>,
        args: Value,
        session: Option<SessionContext>,
    ) -> McpResult<CreateTaskResult> {
        // Bearer-token-grade id (upstream §Security: unguessable, no
        // enumeration) — v4's 122 random bits, not the timestamp-prefixed v7.
        let task_id = uuid::Uuid::new_v4().as_simple().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let fields = TaskFields {
            task_id: task_id.clone(),
            status_message: Some("executing tool".to_string()),
            created_at: now.clone(),
            last_updated_at: now,
            ttl_ms: Nullable(None),
            poll_interval_ms: Some(POLL_INTERVAL_MS),
        };
        let owner = owner_from_session(&session);

        // Durably created BEFORE the response is sent (upstream overview:
        // "A CreateTaskResult is not returned until the task is findable").
        self.store
            .create(TaskState {
                fields: fields.clone(),
                status: TaskStatus::Working,
                owner,
                input_requests: None,
                collected_responses: InputResponses::new(),
                request_state: None,
                result: None,
                error: None,
            })
            .await
            .map_err(|e| McpError::ToolExecutionError(format!("task store: {e}")))?;

        let runtime = Arc::clone(self);
        let worker_task_id = task_id.clone();
        let handle = tokio::spawn(async move {
            runtime.worker(worker_task_id, tool, args, session).await;
        });
        self.aborts
            .lock()
            .expect("aborts lock")
            .insert(task_id, handle.abort_handle());

        Ok(CreateTaskResult::new(Task {
            status: TaskStatus::Working,
            fields,
        }))
    }

    /// The worker loop: run the tool; `InputRequired` parks the task until
    /// `tasks/update` completes the round, then re-invokes the tool with the
    /// responses; any other outcome is terminal.
    async fn worker(
        self: Arc<Self>,
        task_id: String,
        tool: Arc<dyn McpTool>,
        args: Value,
        session: Option<SessionContext>,
    ) {
        let mut responses: Option<InputResponses> = None;
        let mut request_state: Option<String> = None;
        loop {
            // Inject this round's MRTR inputs exactly as the sync retry leg
            // does, so the tool sees input_responses()/mrtr_request_state().
            let ctx = session.clone().map(|mut s| {
                if let Some(ref r) = responses
                    && let Ok(v) = serde_json::to_value(r)
                {
                    s.extensions
                        .insert("mcp:mrtr:inputResponses".to_string(), v);
                }
                if let Some(ref st) = request_state {
                    s.extensions.insert(
                        "mcp:mrtr:requestState".to_string(),
                        Value::String(st.clone()),
                    );
                }
                s
            });

            match tool.call(args.clone(), ctx).await {
                Ok(result) => {
                    match serde_json::to_value(&result) {
                        Ok(v) => self.finish(&task_id, Ok(v), &session).await,
                        Err(e) => {
                            self.finish(
                                &task_id,
                                Err(serde_json::json!({
                                    "code": -32603,
                                    "message": format!("result serialization: {e}"),
                                })),
                                &session,
                            )
                            .await
                        }
                    }
                    return;
                }
                Err(McpError::InputRequired {
                    input_requests,
                    request_state: rs,
                }) => {
                    let Some(requests) = input_requests.filter(|r| !r.is_empty()) else {
                        // Neither-field InputRequired is a server bug on the
                        // sync path too — surface as a failed task.
                        self.finish(
                            &task_id,
                            Err(serde_json::json!({
                                "code": -32603,
                                "message": "tool demanded input without inputRequests",
                            })),
                            &session,
                        )
                        .await;
                        return;
                    };
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    self.waiters
                        .lock()
                        .expect("waiters lock")
                        .insert(task_id.clone(), Waiter { sender: tx });
                    match self
                        .store
                        .require_input(&task_id, requests, rs.clone())
                        .await
                    {
                        Ok(state) => self.notify(&session, &state).await,
                        Err(e) => {
                            warn!(task_id, error = %e, "require_input rejected (task likely cancelled)");
                            self.waiters.lock().expect("waiters lock").remove(&task_id);
                            return;
                        }
                    }
                    match rx.await {
                        Ok((delivered, state)) => {
                            responses = Some(delivered);
                            request_state = state;
                            continue;
                        }
                        // Waiter dropped: cancelled mid-input — nothing to do.
                        Err(_) => return,
                    }
                }
                Err(e) => {
                    let obj = e.to_error_object();
                    self.finish(
                        &task_id,
                        Err(serde_json::json!({
                            "code": obj.code,
                            "message": obj.message,
                            "data": obj.data,
                        })),
                        &session,
                    )
                    .await;
                    return;
                }
            }
        }
    }

    async fn finish(
        &self,
        task_id: &str,
        outcome: Result<Value, Value>,
        session: &Option<SessionContext>,
    ) {
        let stored = match outcome {
            Ok(result) => self.store.complete(task_id, result).await,
            Err(error) => self.store.fail(task_id, error).await,
        };
        self.aborts.lock().expect("aborts lock").remove(task_id);
        match stored {
            Ok(state) => {
                debug!(task_id, "task reached terminal status");
                self.notify(session, &state).await;
            }
            Err(e) => warn!(task_id, error = %e, "terminal write rejected (task likely cancelled)"),
        }
    }

    /// Deliver `tasks/update` responses; resumes the worker when the round
    /// completes. `owner` must match the task's bound owner (see
    /// [`TaskStore::provide_input`]) or the store reports the task unknown.
    pub async fn deliver_input(
        &self,
        task_id: &str,
        owner: Option<&str>,
        responses: InputResponses,
    ) -> Result<(), TaskStoreError> {
        match self.store.provide_input(task_id, owner, responses).await? {
            InputDelivery::Complete {
                responses,
                request_state,
            } => {
                if let Some(waiter) = self.waiters.lock().expect("waiters lock").remove(task_id) {
                    // A dropped receiver means the worker died; the store
                    // already shows `working`, and the task will sit there —
                    // acceptable for the in-memory store (process-local).
                    let _ = waiter.sender.send((responses, request_state));
                }
                Ok(())
            }
            InputDelivery::Partial => Ok(()),
        }
    }

    /// Cooperative cancel: flips non-terminal tasks to `cancelled`, aborts a
    /// running worker, and drops any input waiter. Terminal tasks are left
    /// untouched (callers ack regardless). `owner` must match the task's
    /// bound owner (see [`TaskStore::cancel`]) or the store reports the task
    /// unknown.
    pub async fn cancel(
        &self,
        task_id: &str,
        owner: Option<&str>,
    ) -> Result<Option<TaskState>, TaskStoreError> {
        let cancelled = self.store.cancel(task_id, owner).await?;
        if cancelled.is_some() {
            if let Some(handle) = self.aborts.lock().expect("aborts lock").remove(task_id) {
                handle.abort();
            }
            self.waiters.lock().expect("waiters lock").remove(task_id);
        }
        Ok(cancelled)
    }

    /// Best-effort `notifications/tasks` push (wire-complete JSON-RPC
    /// notification; delivered to `subscriptions/listen` streams whose
    /// honored `taskIds` filter matches).
    async fn notify(&self, session: &Option<SessionContext>, state: &TaskState) {
        let Some(any) = session.as_ref().and_then(|s| s.broadcaster.clone()) else {
            return;
        };
        let Some(broadcaster) = any
            .downcast_ref::<turul_http_mcp_server::notification_bridge::SharedNotificationBroadcaster>()
        else {
            return;
        };
        let Ok(Value::Object(params)) = serde_json::to_value(state.to_detailed()) else {
            return;
        };
        let notification = turul_rpc::JsonRpcNotification::new_with_object_params(
            METHOD_NOTIFICATIONS_TASKS.to_string(),
            params.into_iter().collect(),
        );
        if let Err(e) = broadcaster.broadcast_to_all_sessions(notification).await {
            debug!(error = %e, "notifications/tasks broadcast skipped");
        }
    }
}

/// `tasks/get` — poll the store; the get itself always completes
/// (`resultType: "complete"`), whatever state the task inside is in.
pub struct ExtTasksGetHandler {
    runtime: Arc<ExtTasksRuntime>,
}

impl ExtTasksGetHandler {
    pub fn new(runtime: Arc<ExtTasksRuntime>) -> Self {
        Self { runtime }
    }
}

impl ExtTasksGetHandler {
    async fn get(&self, params: Option<Value>, owner: Option<&str>) -> McpResult<Value> {
        require_declared(&params)?;
        let params: GetTaskParams = parse_params(params)?;
        let state = self
            .runtime
            .store()
            .get(&params.task_id, owner)
            .await
            .map_err(|e| McpError::ToolExecutionError(format!("task store: {e}")))?
            .ok_or_else(|| {
                McpError::InvalidParameters(format!("unknown task {:?}", params.task_id))
            })?;
        let mut v =
            serde_json::to_value(state.to_detailed()).map_err(McpError::SerializationError)?;
        if let Some(obj) = v.as_object_mut() {
            obj.insert(
                "resultType".to_string(),
                Value::String(ResultType::Complete.as_str().to_string()),
            );
        }
        Ok(v)
    }
}

#[async_trait]
impl McpHandler for ExtTasksGetHandler {
    async fn handle(&self, params: Option<Value>) -> McpResult<Value> {
        self.get(params, None).await
    }

    async fn handle_with_session(
        &self,
        params: Option<Value>,
        session: Option<SessionContext>,
    ) -> McpResult<Value> {
        let owner = owner_from_session(&session);
        self.get(params, owner.as_deref()).await
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["tasks/get".to_string()]
    }
}

/// `tasks/update` — deliver input responses to an `input_required` task.
pub struct ExtTasksUpdateHandler {
    runtime: Arc<ExtTasksRuntime>,
}

impl ExtTasksUpdateHandler {
    pub fn new(runtime: Arc<ExtTasksRuntime>) -> Self {
        Self { runtime }
    }
}

impl ExtTasksUpdateHandler {
    async fn update(&self, params: Option<Value>, owner: Option<&str>) -> McpResult<Value> {
        require_declared(&params)?;
        let params: UpdateTaskParams = parse_params(params)?;
        self.runtime
            .deliver_input(&params.task_id, owner, params.input_responses)
            .await
            .map_err(|e| match e {
                TaskStoreError::NotFound(id) => {
                    McpError::InvalidParameters(format!("unknown task {id:?}"))
                }
                TaskStoreError::UnknownInputKey(k) => McpError::InvalidParameters(format!(
                    "input response key {k:?} does not match an outstanding input request"
                )),
                TaskStoreError::InvalidStatus { status, .. } => McpError::InvalidParameters(
                    format!("task is {status}; tasks/update requires input_required"),
                ),
                other => McpError::ToolExecutionError(format!("task store: {other}")),
            })?;
        // Empty ack, resultType "complete".
        Ok(serde_json::json!({ "resultType": "complete" }))
    }
}

#[async_trait]
impl McpHandler for ExtTasksUpdateHandler {
    async fn handle(&self, params: Option<Value>) -> McpResult<Value> {
        self.update(params, None).await
    }

    async fn handle_with_session(
        &self,
        params: Option<Value>,
        session: Option<SessionContext>,
    ) -> McpResult<Value> {
        let owner = owner_from_session(&session);
        self.update(params, owner.as_deref()).await
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["tasks/update".to_string()]
    }
}

/// `tasks/cancel` — cooperative, eventually consistent; always an empty ack.
pub struct ExtTasksCancelHandler {
    runtime: Arc<ExtTasksRuntime>,
}

impl ExtTasksCancelHandler {
    pub fn new(runtime: Arc<ExtTasksRuntime>) -> Self {
        Self { runtime }
    }
}

impl ExtTasksCancelHandler {
    async fn cancel(&self, params: Option<Value>, owner: Option<&str>) -> McpResult<Value> {
        require_declared(&params)?;
        let params: CancelTaskParams = parse_params(params)?;
        match self.runtime.cancel(&params.task_id, owner).await {
            Ok(_) => Ok(serde_json::json!({ "resultType": "complete" })),
            Err(TaskStoreError::NotFound(id)) => {
                Err(McpError::InvalidParameters(format!("unknown task {id:?}")))
            }
            Err(e) => Err(McpError::ToolExecutionError(format!("task store: {e}"))),
        }
    }
}

#[async_trait]
impl McpHandler for ExtTasksCancelHandler {
    async fn handle(&self, params: Option<Value>) -> McpResult<Value> {
        self.cancel(params, None).await
    }

    async fn handle_with_session(
        &self,
        params: Option<Value>,
        session: Option<SessionContext>,
    ) -> McpResult<Value> {
        let owner = owner_from_session(&session);
        self.cancel(params, owner.as_deref()).await
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["tasks/cancel".to_string()]
    }
}

/// The `-32021` payload for a task-requiring tool called without the
/// declared extension — `data.requiredCapabilities.extensions` shape per the
/// upstream overview.
pub fn missing_capability_error() -> McpError {
    McpError::MissingRequiredClientCapability {
        required: serde_json::json!({
            "extensions": { turul_mcp_ext_tasks::EXTENSION_IDENTIFIER: {} }
        }),
    }
}

/// True when this request's declared capabilities activate the extension.
pub fn declared(caps: &turul_mcp_protocol::initialize::ClientCapabilities) -> bool {
    turul_mcp_ext_tasks::declared_by_client(caps)
}

fn parse_params<T: serde::de::DeserializeOwned>(params: Option<Value>) -> McpResult<T> {
    serde_json::from_value(params.unwrap_or(Value::Null))
        .map_err(|e| McpError::InvalidParameters(format!("invalid task params: {e}")))
}

/// Refuse `tasks/*` from a client that never declared the extension.
///
/// SEP-2663: the task methods exist only for a client that opted in, so
/// calling one without the declaration is a missing-capability error
/// (`-32021`), NOT invalid params. All three handlers previously fell through
/// to whatever the task lookup said — typically `-32602 unknown task` — which
/// tells the client its task id was wrong when the real problem is that it
/// never negotiated the extension.
///
/// Found by the conformance scenario `tasks-capability-negotiation`:
/// "tasks/get MUST return -32021; got -32602" (and the same for update and
/// cancel). Checked BEFORE param parsing so a malformed body cannot mask it.
fn require_declared(params: &Option<Value>) -> McpResult<()> {
    let caps = params
        .as_ref()
        .and_then(|p| p.get("_meta"))
        .and_then(|m| m.get(turul_mcp_protocol::meta::META_KEY_CLIENT_CAPABILITIES))
        .cloned()
        .map(serde_json::from_value::<turul_mcp_protocol::initialize::ClientCapabilities>)
        .and_then(Result::ok)
        .unwrap_or_default();
    if declared(&caps) {
        Ok(())
    } else {
        Err(missing_capability_error())
    }
}

// Keep the wire literal in one place for tests.
const _: () = assert!(!RESULT_TYPE_TASK.is_empty());
