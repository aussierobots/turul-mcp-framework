//! Task state types for the MCP Tasks extension (SEP-2663).
//!
//! Maps directly to the vendored extension schema (`schema/draft-schema.ts`):
//! - `TaskStatus`    → [`TaskStatus`]
//! - `Task`          → [`Task`] (status + [`TaskFields`])
//! - `DetailedTask`  → [`DetailedTask`] (status-tagged union with
//!   status-specific fields inlined)
//! - `CreateTaskResult` → [`CreateTaskResult`] (`Result & Task`, flat, with
//!   `resultType: "task"`)

use serde::{Deserialize, Serialize};
use serde_json::Value;
use turul_mcp_protocol_2026_07_28::input_required::InputRequests;
use turul_mcp_protocol_2026_07_28::meta::MetaObject;
use turul_mcp_protocol_2026_07_28::result_type::ResultType;

/// `resultType` discriminator value a server uses when it elects to process
/// a request asynchronously — see `CreateTaskResult` in the extension schema.
pub const RESULT_TYPE_TASK: &str = "task";

/// A REQUIRED-but-nullable wire field (`T | null`). Unlike a plain
/// `Option<T>` struct field — which serde silently defaults to `None` when
/// the key is absent — a missing key is a parse error here, matching the
/// schema's `required` list. (The manual `Deserialize` exists because any
/// type that answers serde's `deserialize_option` probe gets the implicit
/// missing-key-tolerant treatment, including under `#[serde(flatten)]`.)
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Nullable<T>(pub Option<T>);

impl<'de, T: serde::de::DeserializeOwned> Deserialize<'de> for Nullable<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let v = Value::deserialize(deserializer)?;
        if v.is_null() {
            return Ok(Nullable(None));
        }
        T::deserialize(v)
            .map(|t| Nullable(Some(t)))
            .map_err(serde::de::Error::custom)
    }
}

impl<T> Nullable<T> {
    pub fn null() -> Self {
        Self(None)
    }
    pub fn as_option(&self) -> Option<&T> {
        self.0.as_ref()
    }
}

impl<T> From<T> for Nullable<T> {
    fn from(v: T) -> Self {
        Self(Some(v))
    }
}

impl<T> From<Option<T>> for Nullable<T> {
    fn from(v: Option<T>) -> Self {
        Self(v)
    }
}

/// The status of a task — see `TaskStatus` in the extension schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// The request is currently being processed.
    Working,
    /// The task is waiting for input (e.g., elicitation or sampling).
    InputRequired,
    /// The request completed successfully and results are available.
    Completed,
    /// The associated request failed due to a JSON-RPC error during execution.
    Failed,
    /// The request was cancelled before completion.
    Cancelled,
}

impl TaskStatus {
    /// Terminal statuses can never change again.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled
        )
    }
}

/// The status-independent fields shared by every task shape — `Task` in the
/// extension schema minus its `status` discriminator, so the same fields can
/// be flattened into both [`Task`] and each [`DetailedTask`] variant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskFields {
    /// The task identifier.
    #[serde(rename = "taskId")]
    pub task_id: String,

    /// Optional human-readable message describing the current task state.
    #[serde(rename = "statusMessage", skip_serializing_if = "Option::is_none")]
    pub status_message: Option<String>,

    /// ISO 8601 timestamp when the task was created.
    #[serde(rename = "createdAt")]
    pub created_at: String,

    /// ISO 8601 timestamp when the task was last updated.
    #[serde(rename = "lastUpdatedAt")]
    pub last_updated_at: String,

    /// Time-to-live from creation in milliseconds; `null` for unlimited.
    /// REQUIRED and nullable on the wire (the schema lists `ttlMs` in
    /// `Task.required`): `Nullable(None)` serializes as an explicit `null`,
    /// and a payload MISSING the key fails to parse.
    #[serde(rename = "ttlMs")]
    pub ttl_ms: Nullable<f64>,

    /// Suggested polling interval in milliseconds. Clients SHOULD honor this
    /// to avoid overwhelming the server.
    #[serde(rename = "pollIntervalMs", skip_serializing_if = "Option::is_none")]
    pub poll_interval_ms: Option<f64>,
}

/// Data associated with a task — `Task` in the extension schema.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Task {
    /// Current task status.
    pub status: TaskStatus,
    #[serde(flatten)]
    pub fields: TaskFields,
}

/// A task with status-specific fields inlined — `DetailedTask` in the
/// extension schema. Used by `tasks/get` responses and `notifications/tasks`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum DetailedTask {
    /// `WorkingTask` — a task in a normal working state.
    Working {
        #[serde(flatten)]
        fields: TaskFields,
    },
    /// `InputRequiredTask` — waiting for client input (MRTR over tasks).
    InputRequired {
        #[serde(flatten)]
        fields: TaskFields,
        /// Outstanding server-to-client requests; keys are arbitrary
        /// identifiers, unique over the lifetime of a single task.
        #[serde(rename = "inputRequests")]
        input_requests: InputRequests,
    },
    /// `CompletedTask` — the final result is available.
    Completed {
        #[serde(flatten)]
        fields: TaskFields,
        /// The final result; its structure matches the result type of the
        /// original request (e.g. a `CallToolResult` for a `tools/call` task).
        result: Value,
    },
    /// `FailedTask` — a JSON-RPC error occurred during execution.
    Failed {
        #[serde(flatten)]
        fields: TaskFields,
        /// The JSON-RPC error that caused the task to fail.
        error: Value,
    },
    /// `CancelledTask` — cancelled before completion.
    Cancelled {
        #[serde(flatten)]
        fields: TaskFields,
    },
}

impl DetailedTask {
    pub fn status(&self) -> TaskStatus {
        match self {
            DetailedTask::Working { .. } => TaskStatus::Working,
            DetailedTask::InputRequired { .. } => TaskStatus::InputRequired,
            DetailedTask::Completed { .. } => TaskStatus::Completed,
            DetailedTask::Failed { .. } => TaskStatus::Failed,
            DetailedTask::Cancelled { .. } => TaskStatus::Cancelled,
        }
    }

    pub fn fields(&self) -> &TaskFields {
        match self {
            DetailedTask::Working { fields }
            | DetailedTask::InputRequired { fields, .. }
            | DetailedTask::Completed { fields, .. }
            | DetailedTask::Failed { fields, .. }
            | DetailedTask::Cancelled { fields } => fields,
        }
    }
}

/// The result a server returns in lieu of the standard result shape when it
/// elects to process a request asynchronously — `CreateTaskResult` in the
/// extension schema (`Result & Task`, flat). `resultType` MUST be `"task"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskResult {
    /// Discriminator — always the literal `"task"` ([`RESULT_TYPE_TASK`]).
    #[serde(rename = "resultType")]
    pub result_type: ResultType,

    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaObject>,

    #[serde(flatten)]
    pub task: Task,
}

impl CreateTaskResult {
    pub fn new(task: Task) -> Self {
        Self {
            result_type: ResultType::Other(RESULT_TYPE_TASK.to_string()),
            meta: None,
            task,
        }
    }

    /// True when the discriminator carries the required `"task"` value.
    pub fn has_task_discriminator(&self) -> bool {
        self.result_type.as_str() == RESULT_TYPE_TASK
    }
}
