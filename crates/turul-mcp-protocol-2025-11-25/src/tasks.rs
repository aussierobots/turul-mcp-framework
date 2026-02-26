//! MCP Tasks Protocol Types
//!
//! This module defines the types used for the MCP tasks functionality per MCP 2025-11-25,
//! which enables long-running operation tracking via task lifecycle management.
//!
//! # Task Lifecycle
//!
//! Tasks are created implicitly when a request includes `task: TaskMetadata { ttl }` in
//! its params (e.g., `CallToolParams`, `CreateMessageParams`). There is **no `tasks/create`
//! method** — tasks are created by task-augmented requests.
//!
//! ```text
//! Working -> Completed       (success)
//! Working -> Failed          (error)
//! Working -> Cancelled       (user/system cancellation)
//! Working -> InputRequired   (needs user input)
//! InputRequired -> Working   (input received, resuming)
//! ```
//!
//! # Task Status
//!
//! ```rust
//! use turul_mcp_protocol_2025_11_25::tasks::*;
//!
//! let task = Task::new("task-abc-123", TaskStatus::Working,
//!     "2025-01-01T00:00:00Z", "2025-01-01T00:00:00Z")
//!     .with_status_message("Started processing");
//!
//! assert_eq!(task.task_id, "task-abc-123");
//! assert_eq!(task.status, TaskStatus::Working);
//! ```
//!
//! # CRUD Operations (get, cancel, list — no create)
//!
//! ```rust
//! use turul_mcp_protocol_2025_11_25::tasks::*;
//! use turul_mcp_protocol_2025_11_25::meta::Cursor;
//!
//! // Get
//! let get = GetTaskRequest::new("task-123");
//! assert_eq!(get.method, "tasks/get");
//!
//! // Cancel
//! let cancel = CancelTaskRequest::new("task-123");
//! assert_eq!(cancel.method, "tasks/cancel");
//!
//! // List with pagination
//! let list = ListTasksRequest::new()
//!     .with_limit(10)
//!     .with_cursor(Cursor::new("page-2"));
//! assert_eq!(list.method, "tasks/list");
//! ```

use crate::meta::Cursor;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

// === Core Types ===

/// Task status enum per MCP 2025-11-25.
/// See [MCP spec](https://modelcontextprotocol.io/specification/2025-11-25/server/utilities/tasks)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Task is currently working
    Working,
    /// Task needs user input to continue
    InputRequired,
    /// Task completed successfully
    Completed,
    /// Task failed
    Failed,
    /// Task was cancelled
    Cancelled,
}

/// Task metadata for task-augmented requests.
/// Added to `CallToolParams`, `CreateMessageParams`, `ElicitCreateParams` to
/// indicate that the operation should create a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskMetadata {
    /// Time-to-live in milliseconds — how long the server should keep the task
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<u64>,
}

impl TaskMetadata {
    pub fn new() -> Self {
        Self { ttl: None }
    }

    pub fn with_ttl(mut self, ttl: u64) -> Self {
        self.ttl = Some(ttl);
        self
    }
}

impl Default for TaskMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// Core task descriptor per MCP 2025-11-25.
/// See [MCP spec](https://modelcontextprotocol.io/specification/2025-11-25/server/utilities/tasks)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    /// Unique task identifier (serde: "taskId")
    pub task_id: String,
    /// Current status of the task
    pub status: TaskStatus,
    /// ISO 8601 datetime when the task was created (REQUIRED)
    pub created_at: String,
    /// ISO 8601 datetime when the task was last updated (REQUIRED)
    pub last_updated_at: String,
    /// Optional human-readable status message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_message: Option<String>,
    /// Time-to-live in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<i64>,
    /// Suggested poll interval in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll_interval: Option<u64>,
    /// Meta information (_meta field)
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

impl Task {
    pub fn new(
        task_id: impl Into<String>,
        status: TaskStatus,
        created_at: impl Into<String>,
        last_updated_at: impl Into<String>,
    ) -> Self {
        Self {
            task_id: task_id.into(),
            status,
            created_at: created_at.into(),
            last_updated_at: last_updated_at.into(),
            status_message: None,
            ttl: None,
            poll_interval: None,
            meta: None,
        }
    }

    pub fn with_status_message(mut self, message: impl Into<String>) -> Self {
        self.status_message = Some(message.into());
        self
    }

    pub fn with_ttl(mut self, ttl: i64) -> Self {
        self.ttl = Some(ttl);
        self
    }

    pub fn with_poll_interval(mut self, interval: u64) -> Self {
        self.poll_interval = Some(interval);
        self
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

// === tasks/get ===

/// Parameters for tasks/get request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTaskParams {
    /// The task ID to retrieve (serde: "taskId")
    pub task_id: String,
    /// Meta information (optional _meta field inside params)
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

impl GetTaskParams {
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            meta: None,
        }
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

/// Complete tasks/get request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTaskRequest {
    /// Method name (always "tasks/get")
    pub method: String,
    /// Request parameters
    pub params: GetTaskParams,
}

impl GetTaskRequest {
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            method: "tasks/get".to_string(),
            params: GetTaskParams::new(task_id),
        }
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.params = self.params.with_meta(meta);
        self
    }
}

/// Result for tasks/get — flattens Task fields per TS `Result & Task`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTaskResult {
    /// The retrieved task (flattened into the result)
    #[serde(flatten)]
    pub task: Task,
}

impl GetTaskResult {
    pub fn new(task: Task) -> Self {
        Self { task }
    }
}

// === tasks/cancel ===

/// Parameters for tasks/cancel request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelTaskParams {
    /// The task ID to cancel (serde: "taskId")
    pub task_id: String,
    /// Meta information (optional _meta field inside params)
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

impl CancelTaskParams {
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            meta: None,
        }
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

/// Complete tasks/cancel request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelTaskRequest {
    /// Method name (always "tasks/cancel")
    pub method: String,
    /// Request parameters
    pub params: CancelTaskParams,
}

impl CancelTaskRequest {
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            method: "tasks/cancel".to_string(),
            params: CancelTaskParams::new(task_id),
        }
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.params = self.params.with_meta(meta);
        self
    }
}

/// Result for tasks/cancel — flattens Task fields per TS `Result & Task`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelTaskResult {
    /// The cancelled task (flattened into the result)
    #[serde(flatten)]
    pub task: Task,
}

impl CancelTaskResult {
    pub fn new(task: Task) -> Self {
        Self { task }
    }
}

// === tasks/list ===

/// Parameters for tasks/list request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListTasksParams {
    /// Optional cursor for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<Cursor>,
    /// Optional limit for page size
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    /// Meta information (optional _meta field inside params)
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

impl ListTasksParams {
    pub fn new() -> Self {
        Self {
            cursor: None,
            limit: None,
            meta: None,
        }
    }

    pub fn with_cursor(mut self, cursor: Cursor) -> Self {
        self.cursor = Some(cursor);
        self
    }

    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

impl Default for ListTasksParams {
    fn default() -> Self {
        Self::new()
    }
}

/// Complete tasks/list request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListTasksRequest {
    /// Method name (always "tasks/list")
    pub method: String,
    /// Request parameters
    pub params: ListTasksParams,
}

impl Default for ListTasksRequest {
    fn default() -> Self {
        Self::new()
    }
}

impl ListTasksRequest {
    pub fn new() -> Self {
        Self {
            method: "tasks/list".to_string(),
            params: ListTasksParams::new(),
        }
    }

    pub fn with_cursor(mut self, cursor: Cursor) -> Self {
        self.params = self.params.with_cursor(cursor);
        self
    }

    pub fn with_limit(mut self, limit: u32) -> Self {
        self.params = self.params.with_limit(limit);
        self
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.params = self.params.with_meta(meta);
        self
    }
}

/// Result for tasks/list (extends PaginatedResult)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListTasksResult {
    /// Available tasks
    pub tasks: Vec<Task>,
    /// Optional cursor for next page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<Cursor>,
    /// Meta information (from PaginatedResult)
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

impl ListTasksResult {
    pub fn new(tasks: Vec<Task>) -> Self {
        Self {
            tasks,
            next_cursor: None,
            meta: None,
        }
    }

    pub fn with_next_cursor(mut self, cursor: Cursor) -> Self {
        self.next_cursor = Some(cursor);
        self
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

// === tasks/result ===

/// Parameters for tasks/result request (retrieves the original operation's result)
///
/// The response is NOT a custom type — it returns the original request's result verbatim
/// (e.g., `CallToolResult` for `tools/call`), with `_meta.io.modelcontextprotocol/related-task`
/// injected by the handler.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTaskPayloadParams {
    /// The task ID whose result to retrieve (serde: "taskId")
    pub task_id: String,
    /// Meta information (optional _meta field inside params)
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

impl GetTaskPayloadParams {
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            meta: None,
        }
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

/// Complete tasks/result request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTaskPayloadRequest {
    /// Method name (always "tasks/result")
    pub method: String,
    /// Request parameters
    pub params: GetTaskPayloadParams,
}

impl GetTaskPayloadRequest {
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            method: "tasks/result".to_string(),
            params: GetTaskPayloadParams::new(task_id),
        }
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.params = self.params.with_meta(meta);
        self
    }
}

// === CreateTaskResult ===

/// Returned when a task-augmented request is accepted (instead of the operation's direct result).
///
/// The client receives this as the JSON-RPC result and can then poll with `tasks/get`,
/// retrieve the outcome with `tasks/result`, or cancel with `tasks/cancel`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskResult {
    /// The newly created task
    pub task: Task,
    /// Meta information (follows MCP Result interface)
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "_meta",
        rename = "_meta"
    )]
    pub meta: Option<HashMap<String, Value>>,
}

impl CreateTaskResult {
    pub fn new(task: Task) -> Self {
        Self { task, meta: None }
    }

    pub fn with_meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

// === Trait Implementations ===

use crate::traits::*;

// -- GetTaskParams --
impl Params for GetTaskParams {}

impl HasMetaParam for GetTaskParams {
    fn meta(&self) -> Option<&HashMap<String, Value>> {
        self.meta.as_ref()
    }
}

// -- GetTaskRequest --
impl HasMethod for GetTaskRequest {
    fn method(&self) -> &str {
        &self.method
    }
}

impl HasParams for GetTaskRequest {
    fn params(&self) -> Option<&dyn Params> {
        Some(&self.params)
    }
}

// -- GetTaskResult --
impl HasData for GetTaskResult {
    fn data(&self) -> HashMap<String, Value> {
        serde_json::to_value(&self.task)
            .ok()
            .and_then(|v| v.as_object().cloned())
            .map(|obj| obj.into_iter().collect())
            .unwrap_or_default()
    }
}

impl HasMeta for GetTaskResult {
    fn meta(&self) -> Option<HashMap<String, Value>> {
        self.task.meta.clone()
    }
}

impl RpcResult for GetTaskResult {}

// -- CancelTaskParams --
impl Params for CancelTaskParams {}

impl HasMetaParam for CancelTaskParams {
    fn meta(&self) -> Option<&HashMap<String, Value>> {
        self.meta.as_ref()
    }
}

// -- CancelTaskRequest --
impl HasMethod for CancelTaskRequest {
    fn method(&self) -> &str {
        &self.method
    }
}

impl HasParams for CancelTaskRequest {
    fn params(&self) -> Option<&dyn Params> {
        Some(&self.params)
    }
}

// -- CancelTaskResult --
impl HasData for CancelTaskResult {
    fn data(&self) -> HashMap<String, Value> {
        serde_json::to_value(&self.task)
            .ok()
            .and_then(|v| v.as_object().cloned())
            .map(|obj| obj.into_iter().collect())
            .unwrap_or_default()
    }
}

impl HasMeta for CancelTaskResult {
    fn meta(&self) -> Option<HashMap<String, Value>> {
        self.task.meta.clone()
    }
}

impl RpcResult for CancelTaskResult {}

// -- ListTasksParams --
impl Params for ListTasksParams {}

impl HasMetaParam for ListTasksParams {
    fn meta(&self) -> Option<&HashMap<String, Value>> {
        self.meta.as_ref()
    }
}

// -- ListTasksRequest --
impl HasMethod for ListTasksRequest {
    fn method(&self) -> &str {
        &self.method
    }
}

impl HasParams for ListTasksRequest {
    fn params(&self) -> Option<&dyn Params> {
        Some(&self.params)
    }
}

// -- ListTasksResult --
impl HasData for ListTasksResult {
    fn data(&self) -> HashMap<String, Value> {
        let mut data = HashMap::new();
        data.insert(
            "tasks".to_string(),
            serde_json::to_value(&self.tasks).unwrap_or(Value::Null),
        );
        if let Some(ref next_cursor) = self.next_cursor {
            data.insert(
                "nextCursor".to_string(),
                Value::String(next_cursor.as_str().to_string()),
            );
        }
        data
    }
}

impl HasMeta for ListTasksResult {
    fn meta(&self) -> Option<HashMap<String, Value>> {
        self.meta.clone()
    }
}

impl RpcResult for ListTasksResult {}

// -- GetTaskPayloadParams --
impl Params for GetTaskPayloadParams {}

impl HasMetaParam for GetTaskPayloadParams {
    fn meta(&self) -> Option<&HashMap<String, Value>> {
        self.meta.as_ref()
    }
}

// -- GetTaskPayloadRequest --
impl HasMethod for GetTaskPayloadRequest {
    fn method(&self) -> &str {
        &self.method
    }
}

impl HasParams for GetTaskPayloadRequest {
    fn params(&self) -> Option<&dyn Params> {
        Some(&self.params)
    }
}

// -- CreateTaskResult --
impl HasData for CreateTaskResult {
    fn data(&self) -> HashMap<String, Value> {
        let mut data = HashMap::new();
        data.insert(
            "task".to_string(),
            serde_json::to_value(&self.task).unwrap_or(Value::Null),
        );
        data
    }
}

impl HasMeta for CreateTaskResult {
    fn meta(&self) -> Option<HashMap<String, Value>> {
        self.meta.clone()
    }
}

impl RpcResult for CreateTaskResult {}

// === Tests ===

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const TIMESTAMP: &str = "2025-01-01T00:00:00Z";

    #[test]
    fn test_task_status_serialization() {
        assert_eq!(
            serde_json::to_value(TaskStatus::Working).unwrap(),
            json!("working")
        );
        assert_eq!(
            serde_json::to_value(TaskStatus::InputRequired).unwrap(),
            json!("input_required")
        );
        assert_eq!(
            serde_json::to_value(TaskStatus::Completed).unwrap(),
            json!("completed")
        );
        assert_eq!(
            serde_json::to_value(TaskStatus::Failed).unwrap(),
            json!("failed")
        );
        assert_eq!(
            serde_json::to_value(TaskStatus::Cancelled).unwrap(),
            json!("cancelled")
        );
    }

    #[test]
    fn test_task_status_deserialization() {
        let working: TaskStatus = serde_json::from_value(json!("working")).unwrap();
        assert_eq!(working, TaskStatus::Working);

        let input_required: TaskStatus = serde_json::from_value(json!("input_required")).unwrap();
        assert_eq!(input_required, TaskStatus::InputRequired);

        let completed: TaskStatus = serde_json::from_value(json!("completed")).unwrap();
        assert_eq!(completed, TaskStatus::Completed);

        let failed: TaskStatus = serde_json::from_value(json!("failed")).unwrap();
        assert_eq!(failed, TaskStatus::Failed);

        let cancelled: TaskStatus = serde_json::from_value(json!("cancelled")).unwrap();
        assert_eq!(cancelled, TaskStatus::Cancelled);
    }

    #[test]
    fn test_task_matches_ts_spec() {
        let task = Task::new("t1", TaskStatus::Working, TIMESTAMP, TIMESTAMP);
        let json = serde_json::to_value(&task).unwrap();

        // Verify TS field names
        assert!(json.get("taskId").is_some(), "TS spec uses taskId, not id");
        assert!(json.get("id").is_none(), "id is wrong, should be taskId");
        assert_eq!(
            json["status"], "working",
            "TS spec uses working, not running"
        );
        assert!(json.get("createdAt").is_some(), "required field");
        assert!(json.get("lastUpdatedAt").is_some(), "required field");
    }

    #[test]
    fn test_task_creation() {
        let task = Task::new("task-123", TaskStatus::Working, TIMESTAMP, TIMESTAMP)
            .with_status_message("Processing data")
            .with_ttl(30000)
            .with_poll_interval(5000);

        assert_eq!(task.task_id, "task-123");
        assert_eq!(task.status, TaskStatus::Working);
        assert_eq!(task.status_message, Some("Processing data".to_string()));
        assert_eq!(task.ttl, Some(30000));
        assert_eq!(task.poll_interval, Some(5000));
        assert_eq!(task.created_at, TIMESTAMP);
        assert_eq!(task.last_updated_at, TIMESTAMP);
    }

    #[test]
    fn test_task_camel_case_serialization() {
        let task = Task::new("task-1", TaskStatus::Working, TIMESTAMP, TIMESTAMP)
            .with_status_message("Working")
            .with_ttl(60000)
            .with_poll_interval(1000);

        let json_value = serde_json::to_value(&task).unwrap();

        // Verify camelCase fields
        assert!(json_value.get("taskId").is_some());
        assert!(json_value.get("status").is_some());
        assert!(json_value.get("createdAt").is_some());
        assert!(json_value.get("lastUpdatedAt").is_some());
        assert!(json_value.get("statusMessage").is_some());
        assert!(json_value.get("pollInterval").is_some());

        // Verify no snake_case fields
        assert!(json_value.get("task_id").is_none());
        assert!(json_value.get("created_at").is_none());
        assert!(json_value.get("last_updated_at").is_none());
        assert!(json_value.get("status_message").is_none());
        assert!(json_value.get("poll_interval").is_none());
    }

    #[test]
    fn test_task_roundtrip() {
        let task = Task::new("task-1", TaskStatus::Completed, TIMESTAMP, TIMESTAMP)
            .with_status_message("Done");

        let json = serde_json::to_string(&task).unwrap();
        let parsed: Task = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.task_id, "task-1");
        assert_eq!(parsed.status, TaskStatus::Completed);
        assert_eq!(parsed.status_message, Some("Done".to_string()));
        assert_eq!(parsed.created_at, TIMESTAMP);
        assert_eq!(parsed.last_updated_at, TIMESTAMP);
    }

    #[test]
    fn test_task_metadata() {
        let metadata = TaskMetadata::new().with_ttl(30000);
        let json = serde_json::to_value(&metadata).unwrap();
        assert_eq!(json["ttl"], 30000);
    }

    #[test]
    fn test_get_task_request() {
        let request = GetTaskRequest::new("task-123");

        assert_eq!(request.method, "tasks/get");
        assert_eq!(request.params.task_id, "task-123");
    }

    #[test]
    fn test_get_task_request_serialization() {
        let request = GetTaskRequest::new("task-123");
        let json_value = serde_json::to_value(&request).unwrap();

        assert_eq!(json_value["method"], "tasks/get");
        assert_eq!(json_value["params"]["taskId"], "task-123");
        // Verify NOT "id"
        assert!(json_value["params"].get("id").is_none());
    }

    #[test]
    fn test_cancel_task_request() {
        let request = CancelTaskRequest::new("task-123");

        assert_eq!(request.method, "tasks/cancel");
        assert_eq!(request.params.task_id, "task-123");
    }

    #[test]
    fn test_cancel_task_request_serialization() {
        let request = CancelTaskRequest::new("task-456");
        let json_value = serde_json::to_value(&request).unwrap();

        assert_eq!(json_value["method"], "tasks/cancel");
        assert_eq!(json_value["params"]["taskId"], "task-456");
        // Verify NOT "id"
        assert!(json_value["params"].get("id").is_none());
    }

    #[test]
    fn test_cancel_task_result() {
        let task = Task::new("task-456", TaskStatus::Cancelled, TIMESTAMP, TIMESTAMP);
        let result = CancelTaskResult::new(task);

        let json_value = serde_json::to_value(&result).unwrap();
        // Flattened — taskId at top level, not nested under "task"
        assert_eq!(json_value["taskId"], "task-456");
        assert_eq!(json_value["status"], "cancelled");
    }

    #[test]
    fn test_list_tasks_request() {
        let request = ListTasksRequest::new()
            .with_cursor(Cursor::new("page-2"))
            .with_limit(10);

        assert_eq!(request.method, "tasks/list");
        assert_eq!(request.params.cursor.as_ref().unwrap().as_str(), "page-2");
        assert_eq!(request.params.limit, Some(10));
    }

    #[test]
    fn test_list_tasks_request_serialization() {
        let request = ListTasksRequest::new()
            .with_cursor(Cursor::new("cursor-abc"))
            .with_limit(25);

        let json_value = serde_json::to_value(&request).unwrap();

        assert_eq!(json_value["method"], "tasks/list");
        assert_eq!(json_value["params"]["cursor"], "cursor-abc");
        assert_eq!(json_value["params"]["limit"], 25);
    }

    #[test]
    fn test_list_tasks_result() {
        let tasks = vec![
            Task::new("task-1", TaskStatus::Working, TIMESTAMP, TIMESTAMP),
            Task::new("task-2", TaskStatus::Completed, TIMESTAMP, TIMESTAMP),
        ];
        let result = ListTasksResult::new(tasks).with_next_cursor(Cursor::new("next-page"));

        assert_eq!(result.tasks.len(), 2);
        assert_eq!(result.next_cursor.as_ref().unwrap().as_str(), "next-page");
    }

    #[test]
    fn test_list_tasks_result_camel_case() {
        let tasks = vec![Task::new(
            "task-1",
            TaskStatus::Working,
            TIMESTAMP,
            TIMESTAMP,
        )];
        let result = ListTasksResult::new(tasks).with_next_cursor(Cursor::new("page-2"));

        let json_value = serde_json::to_value(&result).unwrap();

        // Verify camelCase
        assert!(json_value.get("tasks").is_some());
        assert!(json_value.get("nextCursor").is_some());
        assert_eq!(json_value["nextCursor"], "page-2");

        // Verify tasks use taskId
        assert!(json_value["tasks"][0].get("taskId").is_some());
        assert!(json_value["tasks"][0].get("id").is_none());

        // Verify no snake_case
        assert!(json_value.get("next_cursor").is_none());
    }

    #[test]
    fn test_list_tasks_result_roundtrip() {
        let tasks = vec![
            Task::new("task-1", TaskStatus::Working, TIMESTAMP, TIMESTAMP)
                .with_status_message("In progress"),
            Task::new("task-2", TaskStatus::Failed, TIMESTAMP, TIMESTAMP)
                .with_status_message("Error occurred"),
        ];

        let mut meta = HashMap::new();
        meta.insert("totalCount".to_string(), json!(42));

        let result = ListTasksResult::new(tasks)
            .with_next_cursor(Cursor::new("cursor-xyz"))
            .with_meta(meta);

        let json = serde_json::to_string(&result).unwrap();
        let parsed: ListTasksResult = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.tasks.len(), 2);
        assert_eq!(parsed.tasks[0].task_id, "task-1");
        assert_eq!(parsed.tasks[0].status, TaskStatus::Working);
        assert_eq!(parsed.tasks[1].task_id, "task-2");
        assert_eq!(parsed.tasks[1].status, TaskStatus::Failed);
        assert_eq!(parsed.next_cursor.as_ref().unwrap().as_str(), "cursor-xyz");
        assert!(parsed.meta.is_some());
    }

    #[test]
    fn test_skip_serializing_none_fields() {
        let task = Task::new("task-1", TaskStatus::Working, TIMESTAMP, TIMESTAMP);
        let json_value = serde_json::to_value(&task).unwrap();

        // Required fields should be present
        assert!(json_value.get("taskId").is_some());
        assert!(json_value.get("status").is_some());
        assert!(json_value.get("createdAt").is_some());
        assert!(json_value.get("lastUpdatedAt").is_some());

        // Optional fields should be absent (not null)
        assert!(json_value.get("statusMessage").is_none());
        assert!(json_value.get("ttl").is_none());
        assert!(json_value.get("pollInterval").is_none());
        assert!(json_value.get("_meta").is_none());
    }

    #[test]
    fn test_trait_implementations() {
        // Verify HasMethod returns correct method strings
        let get_req = GetTaskRequest::new("task-1");
        assert_eq!(HasMethod::method(&get_req), "tasks/get");

        let cancel_req = CancelTaskRequest::new("task-1");
        assert_eq!(HasMethod::method(&cancel_req), "tasks/cancel");

        let list_req = ListTasksRequest::new();
        assert_eq!(HasMethod::method(&list_req), "tasks/list");
    }

    #[test]
    fn test_has_data_trait() {
        let task = Task::new("task-1", TaskStatus::Working, TIMESTAMP, TIMESTAMP);
        let result = GetTaskResult::new(task);

        let data = HasData::data(&result);
        // Flattened — data contains taskId directly
        assert_eq!(data["taskId"], json!("task-1"));
        assert_eq!(data["status"], json!("working"));
    }

    #[test]
    fn test_list_tasks_has_data_trait() {
        let tasks = vec![Task::new(
            "task-1",
            TaskStatus::Working,
            TIMESTAMP,
            TIMESTAMP,
        )];
        let result = ListTasksResult::new(tasks).with_next_cursor(Cursor::new("next"));

        let data = HasData::data(&result);
        assert!(data.contains_key("tasks"));
        assert!(data.contains_key("nextCursor"));
        assert_eq!(data["nextCursor"], Value::String("next".to_string()));
    }

    // === Phase A: New type tests ===

    #[test]
    fn test_get_task_payload_request() {
        let request = GetTaskPayloadRequest::new("task-789");
        assert_eq!(request.method, "tasks/result");
        assert_eq!(request.params.task_id, "task-789");
    }

    #[test]
    fn test_get_task_payload_request_serialization() {
        let request = GetTaskPayloadRequest::new("task-789");
        let json_value = serde_json::to_value(&request).unwrap();

        assert_eq!(json_value["method"], "tasks/result");
        assert_eq!(json_value["params"]["taskId"], "task-789");
        assert!(json_value["params"].get("id").is_none());
    }

    #[test]
    fn test_get_task_payload_request_roundtrip() {
        let request = GetTaskPayloadRequest::new("task-rt");
        let json = serde_json::to_string(&request).unwrap();
        let parsed: GetTaskPayloadRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.method, "tasks/result");
        assert_eq!(parsed.params.task_id, "task-rt");
    }

    #[test]
    fn test_get_task_payload_trait_implementations() {
        let request = GetTaskPayloadRequest::new("task-1");
        assert_eq!(HasMethod::method(&request), "tasks/result");
    }

    #[test]
    fn test_create_task_result() {
        let task = Task::new("task-new", TaskStatus::Working, TIMESTAMP, TIMESTAMP)
            .with_ttl(60000)
            .with_poll_interval(5000);
        let result = CreateTaskResult::new(task);

        let json_value = serde_json::to_value(&result).unwrap();

        // CreateTaskResult wraps task in a "task" field (NOT flattened)
        assert!(
            json_value.get("task").is_some(),
            "task field should be present"
        );
        assert_eq!(json_value["task"]["taskId"], "task-new");
        assert_eq!(json_value["task"]["status"], "working");
        assert_eq!(json_value["task"]["ttl"], 60000);
        assert_eq!(json_value["task"]["pollInterval"], 5000);
    }

    #[test]
    fn test_create_task_result_roundtrip() {
        let task = Task::new("task-rt", TaskStatus::Working, TIMESTAMP, TIMESTAMP);
        let result = CreateTaskResult::new(task);

        let json = serde_json::to_string(&result).unwrap();
        let parsed: CreateTaskResult = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.task.task_id, "task-rt");
        assert_eq!(parsed.task.status, TaskStatus::Working);
    }

    #[test]
    fn test_create_task_result_has_data_trait() {
        let task = Task::new("task-d", TaskStatus::Working, TIMESTAMP, TIMESTAMP);
        let result = CreateTaskResult::new(task);
        let data = HasData::data(&result);
        assert!(data.contains_key("task"));
    }

    #[test]
    fn test_task_support_serialization() {
        assert_eq!(
            serde_json::to_value(crate::tools::TaskSupport::Required).unwrap(),
            json!("required")
        );
        assert_eq!(
            serde_json::to_value(crate::tools::TaskSupport::Optional).unwrap(),
            json!("optional")
        );
        assert_eq!(
            serde_json::to_value(crate::tools::TaskSupport::Forbidden).unwrap(),
            json!("forbidden")
        );
    }

    #[test]
    fn test_task_support_deserialization() {
        let required: crate::tools::TaskSupport =
            serde_json::from_value(json!("required")).unwrap();
        assert_eq!(required, crate::tools::TaskSupport::Required);

        let optional: crate::tools::TaskSupport =
            serde_json::from_value(json!("optional")).unwrap();
        assert_eq!(optional, crate::tools::TaskSupport::Optional);

        let forbidden: crate::tools::TaskSupport =
            serde_json::from_value(json!("forbidden")).unwrap();
        assert_eq!(forbidden, crate::tools::TaskSupport::Forbidden);

        // Backward compat: "supported" alias deserializes to Optional
        let legacy: crate::tools::TaskSupport = serde_json::from_value(json!("supported")).unwrap();
        assert_eq!(legacy, crate::tools::TaskSupport::Optional);
    }

    #[test]
    fn test_call_tool_params_with_task() {
        let params = crate::tools::CallToolParams::new("my_tool")
            .with_task(TaskMetadata::new().with_ttl(60000));

        let json_value = serde_json::to_value(&params).unwrap();

        assert_eq!(json_value["name"], "my_tool");
        assert_eq!(json_value["task"]["ttl"], 60000);
    }

    #[test]
    fn test_call_tool_params_without_task_backward_compat() {
        let params = crate::tools::CallToolParams::new("my_tool");
        let json_value = serde_json::to_value(&params).unwrap();

        assert_eq!(json_value["name"], "my_tool");
        // task field should be absent (not null)
        assert!(json_value.get("task").is_none());
    }

    #[test]
    fn test_create_message_params_with_task() {
        let params = crate::sampling::CreateMessageParams::new(vec![], 100)
            .with_task(TaskMetadata::new().with_ttl(30000));

        let json_value = serde_json::to_value(&params).unwrap();
        assert_eq!(json_value["task"]["ttl"], 30000);
    }

    #[test]
    fn test_create_message_params_without_task_backward_compat() {
        let params = crate::sampling::CreateMessageParams::new(vec![], 100);
        let json_value = serde_json::to_value(&params).unwrap();
        assert!(json_value.get("task").is_none());
    }

    #[test]
    fn test_elicit_create_params_with_task() {
        let schema = crate::elicitation::ElicitationSchema::new();
        let params = crate::elicitation::ElicitCreateParams::new("test", schema)
            .with_task(TaskMetadata::new().with_ttl(15000));

        let json_value = serde_json::to_value(&params).unwrap();
        assert_eq!(json_value["task"]["ttl"], 15000);
    }

    #[test]
    fn test_elicit_create_params_without_task_backward_compat() {
        let schema = crate::elicitation::ElicitationSchema::new();
        let params = crate::elicitation::ElicitCreateParams::new("test", schema);
        let json_value = serde_json::to_value(&params).unwrap();
        assert!(json_value.get("task").is_none());
    }

    #[test]
    fn test_tasks_capabilities_structured_serialization() {
        use crate::initialize::*;

        let caps = TasksCapabilities {
            list: Some(TasksListCapabilities::default()),
            cancel: Some(TasksCancelCapabilities::default()),
            requests: Some(TasksRequestCapabilities {
                tools: Some(TasksToolCapabilities {
                    call: Some(TasksToolCallCapabilities::default()),
                    extra: Default::default(),
                }),
                extra: Default::default(),
            }),
            extra: Default::default(),
        };

        let json_value = serde_json::to_value(&caps).unwrap();

        // Verify structured shape: {"list":{},"cancel":{},"requests":{"tools":{"call":{}}}}
        assert!(json_value.get("list").is_some());
        assert!(json_value.get("cancel").is_some());
        assert!(json_value["requests"]["tools"]["call"].is_object());
    }

    #[test]
    fn test_tasks_capabilities_empty_signals_support() {
        use crate::initialize::*;

        // Empty sub-structs still serialize as `{}`
        let list_caps = TasksListCapabilities::default();
        let json = serde_json::to_value(&list_caps).unwrap();
        assert!(json.is_object());
        assert_eq!(json.as_object().unwrap().len(), 0);
    }

    #[test]
    fn test_tasks_capabilities_roundtrip() {
        use crate::initialize::*;

        let caps = TasksCapabilities {
            list: Some(TasksListCapabilities::default()),
            cancel: None,
            requests: Some(TasksRequestCapabilities {
                tools: Some(TasksToolCapabilities {
                    call: Some(TasksToolCallCapabilities::default()),
                    extra: Default::default(),
                }),
                extra: Default::default(),
            }),
            extra: Default::default(),
        };

        let json = serde_json::to_string(&caps).unwrap();
        let parsed: TasksCapabilities = serde_json::from_str(&json).unwrap();

        assert!(parsed.list.is_some());
        assert!(parsed.cancel.is_none());
        assert!(parsed.requests.is_some());
        assert!(parsed.requests.unwrap().tools.unwrap().call.is_some());
    }
}
