//! Method bindings for the Tasks extension lifecycle (SEP-2663):
//! `tasks/get` (polling), `tasks/update` (input responses), `tasks/cancel`,
//! and the optional `notifications/tasks` status notification.
//!
//! The redesigned lifecycle is poll-based: there is no `tasks/list` and no
//! blocking `tasks/result`. Clients poll `tasks/get` (honoring
//! `pollIntervalMs`) or subscribe to `notifications/tasks` via
//! `subscriptions/listen` with the [`TaskSubscriptionNotifications`] filter
//! fields.

use serde::{Deserialize, Serialize};
use turul_mcp_protocol_2026_07_28::input_required::InputResponses;
use turul_mcp_protocol_2026_07_28::meta::MetaObject;
use turul_mcp_protocol_2026_07_28::result_type::ResultType;

use super::types::DetailedTask;

/// `tasks/get` — retrieve the state of a task.
pub const METHOD_TASKS_GET: &str = "tasks/get";
/// `tasks/update` — provide input responses to an `input_required` task.
pub const METHOD_TASKS_UPDATE: &str = "tasks/update";
/// `tasks/cancel` — request cooperative cancellation.
pub const METHOD_TASKS_CANCEL: &str = "tasks/cancel";
/// `notifications/tasks` — optional server status notification.
pub const METHOD_NOTIFICATIONS_TASKS: &str = "notifications/tasks";

/// Params for [`METHOD_TASKS_GET`] — `GetTaskRequest.params`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetTaskParams {
    /// The task identifier to query.
    #[serde(rename = "taskId")]
    pub task_id: String,
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaObject>,
}

/// Result of [`METHOD_TASKS_GET`] — `GetTaskResult` (`Result & DetailedTask`).
/// Carries the [`DetailedTask`] variant for the task's current status;
/// `resultType` MUST be `"complete"` (the *get* itself completed — the task
/// inside may be in any state).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetTaskResult {
    #[serde(rename = "resultType", default)]
    pub result_type: ResultType,
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaObject>,
    #[serde(flatten)]
    pub task: DetailedTask,
}

impl GetTaskResult {
    pub fn new(task: DetailedTask) -> Self {
        Self {
            result_type: ResultType::Complete,
            meta: None,
            task,
        }
    }
}

/// Params for [`METHOD_TASKS_UPDATE`] — `UpdateTaskRequest.params`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTaskParams {
    /// The task identifier to update.
    #[serde(rename = "taskId")]
    pub task_id: String,
    /// Responses to outstanding `inputRequests`; each key MUST correspond to
    /// a currently-outstanding input-request key.
    #[serde(rename = "inputResponses")]
    pub input_responses: InputResponses,
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaObject>,
}

/// Params for [`METHOD_TASKS_CANCEL`] — `CancelTaskRequest.params`.
/// Cancellation is cooperative and eventually consistent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelTaskParams {
    /// The task identifier to cancel.
    #[serde(rename = "taskId")]
    pub task_id: String,
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaObject>,
}

/// Empty acknowledgement shared by `tasks/update` and `tasks/cancel`
/// (`UpdateTaskResult` / `CancelTaskResult` — both plain `Result` with
/// `resultType: "complete"`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskAckResult {
    #[serde(rename = "resultType", default)]
    pub result_type: ResultType,
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaObject>,
}

/// Params of [`METHOD_NOTIFICATIONS_TASKS`] — `TaskStatusNotificationParams`
/// (`NotificationParams & DetailedTask`). Servers are not required to send
/// these; clients subscribe via `subscriptions/listen`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStatusNotificationParams {
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaObject>,
    #[serde(flatten)]
    pub task: DetailedTask,
}

/// Task-specific fields for the `subscriptions/listen` request filter —
/// `TaskSubscriptionNotifications`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TaskSubscriptionNotifications {
    /// Subscribe to `notifications/tasks` for these task IDs.
    #[serde(rename = "taskIds", skip_serializing_if = "Option::is_none")]
    pub task_ids: Option<Vec<String>>,
}

/// Task-specific fields for `notifications/subscriptions/acknowledged` —
/// `TaskSubscriptionAcknowledgedNotifications`: the task IDs the server
/// agreed to send status notifications for.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TaskSubscriptionAcknowledgedNotifications {
    #[serde(rename = "taskIds", skip_serializing_if = "Option::is_none")]
    pub task_ids: Option<Vec<String>>,
}
