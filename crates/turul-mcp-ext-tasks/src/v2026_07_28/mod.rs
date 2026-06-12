//! SEP-2663 Tasks extension surface for the 2026-07-28 spec line.

pub mod capability;
pub mod lifecycle;
pub mod types;

#[cfg(test)]
mod compliance_test;

pub use capability::{
    EXTENSION_IDENTIFIER, InvalidExtensionIdentifier, capability, declared_by_client,
    declared_by_server, validate_identifier,
};
pub use lifecycle::{
    CancelTaskParams, GetTaskParams, GetTaskResult, METHOD_NOTIFICATIONS_TASKS,
    METHOD_TASKS_CANCEL, METHOD_TASKS_GET, METHOD_TASKS_UPDATE, TaskAckResult,
    TaskStatusNotificationParams, TaskSubscriptionAcknowledgedNotifications,
    TaskSubscriptionNotifications, UpdateTaskParams,
};
pub use types::{
    CreateTaskResult, DetailedTask, Nullable, RESULT_TYPE_TASK, Task, TaskFields, TaskStatus,
};
