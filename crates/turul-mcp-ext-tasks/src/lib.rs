//! # MCP Tasks Extension (`io.modelcontextprotocol/tasks`, SEP-2663)
//!
//! Spec-neutral host crate for the Tasks extension. Task support differs by
//! spec line — 2025-11-25 carries tasks in core, while 2026-07-28 moves them
//! to the `io.modelcontextprotocol/tasks` extension — so each lane lives in
//! its own feature-gated module rather than a per-spec crate fork.
//!
//! Currently implemented: the [`v2026_07_28`] module (SEP-2663 redesigned
//! lifecycle). The 2025-11-25 reconciliation rides the core protocol crates
//! today and is not re-hosted here yet.
//!
//! ## The 2026-07-28 lifecycle (SEP-2663)
//!
//! - polling via `tasks/get` (`pollIntervalMs` hint) — no blocking `tasks/result`
//! - `tasks/update` delivers input responses to `input_required` tasks
//! - no `tasks/list`
//! - unsolicited task handles allowed (`CreateTaskResult` with
//!   `resultType: "task"` in lieu of any standard result)
//! - optional `notifications/tasks` status pushes, subscribed via
//!   `subscriptions/listen` with `taskIds` filter fields
//!
//! Adding this crate as a dependency is the opt-in — extensions are disabled
//! by default. Server-side dispatch lives in `turul-mcp-server` behind its
//! `ext-tasks` feature (`.with_ext_tasks(store)` + `.ext_task_tool(...)`). Schema provenance: `schema/README.md` (vendored from
//! `modelcontextprotocol/ext-tasks` at a pinned commit).
//!
//! See the SEP: <https://modelcontextprotocol.io/seps/2663-tasks-extension>.

#[cfg(feature = "protocol-2026-07-28")]
pub mod v2026_07_28;

#[cfg(feature = "protocol-2026-07-28")]
pub use v2026_07_28::{
    CancelTaskParams, CreateTaskResult, DetailedTask, EXTENSION_IDENTIFIER, GetTaskParams,
    GetTaskResult, InMemoryTaskStore, InputDelivery, InvalidExtensionIdentifier,
    METHOD_NOTIFICATIONS_TASKS, METHOD_TASKS_CANCEL, METHOD_TASKS_GET, METHOD_TASKS_UPDATE,
    Nullable, RESULT_TYPE_TASK, Task, TaskAckResult, TaskFields, TaskState, TaskStatus,
    TaskStatusNotificationParams, TaskStore, TaskStoreError,
    TaskSubscriptionAcknowledgedNotifications, TaskSubscriptionNotifications, UpdateTaskParams,
    capability, declared_by_client, declared_by_server, validate_identifier,
};
