//! SEP-2663 Tasks extension surface for the 2026-07-28 spec line.

pub mod capability;
pub mod lifecycle;
pub mod types;

// Storage: one module for the contract, one per backend — the same layout as
// `turul-mcp-session-storage` and `turul-mcp-server-state-storage`, so a
// reader who knows one storage crate in this workspace knows all of them.
pub mod in_memory;
pub mod traits;

#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(feature = "postgres")]
pub mod postgres;

#[cfg(feature = "dynamodb")]
pub mod dynamodb;

/// The behaviour contract every [`store::TaskStore`] backend must satisfy,
/// written once and run against all of them. Enabled by `parity-harness`
/// (implied by every backend feature, and by `cfg(test)` in this crate) so
/// out-of-workspace implementors can hold their own backends to it.
#[cfg(any(test, feature = "parity-harness"))]
pub mod parity;

#[cfg(test)]
mod compliance_test;

pub use capability::{
    EXTENSION_IDENTIFIER, InvalidExtensionIdentifier, capability, declared_by_client,
    declared_by_server, validate_identifier,
};
pub use in_memory::InMemoryTaskStore;
pub use lifecycle::{
    CancelTaskParams, GetTaskParams, GetTaskResult, METHOD_NOTIFICATIONS_TASKS,
    METHOD_TASKS_CANCEL, METHOD_TASKS_GET, METHOD_TASKS_UPDATE, TaskAckResult,
    TaskStatusNotificationParams, TaskSubscriptionAcknowledgedNotifications,
    TaskSubscriptionNotifications, UpdateTaskParams,
};
pub use traits::{
    InputDelivery, RetentionPolicy, SweepAction, SweepReport, TaskState, TaskStore, TaskStoreError,
};
pub use types::{
    CreateTaskResult, DetailedTask, Nullable, RESULT_TYPE_TASK, Task, TaskFields, TaskStatus,
};
