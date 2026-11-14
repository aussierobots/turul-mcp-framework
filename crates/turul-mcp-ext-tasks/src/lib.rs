//! Types and storage for the MCP Tasks extension (`io.modelcontextprotocol/tasks`),
//! which lets a server hand back a task handle that a client polls.
//!
//! Most users want [`turul-mcp-server`](https://docs.rs/turul-mcp-server). Extensions
//! are off by default there; enable its `ext-tasks` feature, then call
//! `.with_ext_tasks(store)` and `.ext_task_tool(...)` on the builder. This crate
//! supplies the wire types, the [`TaskStore`] trait and [`InMemoryTaskStore`].
//! Clients poll with `tasks/get`, answer input requests with `tasks/update` and
//! stop work with `tasks/cancel`; there is no `tasks/list`.
//!
//! # Example
//!
//! Create a task, complete it and read it back:
//!
//! ```toml
//! [dependencies]
//! turul-mcp-ext-tasks = "0.1"
//! serde_json = "1"
//! tokio = { version = "1", features = ["full"] }
//! ```
//!
//! ```rust
//! use turul_mcp_ext_tasks::{
//!     InMemoryTaskStore, Nullable, TaskFields, TaskState, TaskStatus, TaskStore,
//! };
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let store = InMemoryTaskStore::new();
//! let now = "2026-08-15T00:00:00Z".to_string();
//! store
//!     .create(TaskState {
//!         fields: TaskFields {
//!             task_id: "t1".into(),
//!             status_message: None,
//!             created_at: now.clone(),
//!             last_updated_at: now,
//!             ttl_ms: Nullable(None),
//!             poll_interval_ms: Some(1000.0),
//!         },
//!         status: TaskStatus::Working,
//!         owner: None,
//!         input_requests: None,
//!         collected_responses: Default::default(),
//!         request_state: None,
//!         result: None,
//!         error: None,
//!     })
//!     .await?;
//! store.complete("t1", serde_json::json!({ "ok": true })).await?;
//! let task = store.get("t1", None).await?.ok_or("task missing")?;
//! assert_eq!(task.status, TaskStatus::Completed);
//! # Ok(())
//! # }
//! ```
//!
//! # Features
//!
//! | Feature | Default | Enables |
//! |---|---|---|
//! | `protocol-2026-07-28` | yes | The extension for MCP 2026-07-28 |
//! | `sqlite`, `postgres`, `dynamodb` | no | Durable [`TaskStore`] backends |
//! | `parity-harness` | no | Test suite to hold your own [`TaskStore`] to the same contract |
//!
//! # Where next
//!
//! - [`TaskStore`] for the contract a backend implements
//! - [`ext-tasks-server`](https://github.com/aussierobots/turul-mcp-framework/tree/main/examples/ext-tasks-server) example
//! - [SEP-2663](https://modelcontextprotocol.io/seps/2663-tasks-extension), the extension specification

#[cfg(feature = "protocol-2026-07-28")]
pub mod v2026_07_28;

#[cfg(feature = "protocol-2026-07-28")]
pub use v2026_07_28::{
    CancelTaskParams, CreateTaskResult, DetailedTask, EXTENSION_IDENTIFIER, GetTaskParams,
    GetTaskResult, InMemoryTaskStore, InputDelivery, InvalidExtensionIdentifier,
    METHOD_NOTIFICATIONS_TASKS, METHOD_TASKS_CANCEL, METHOD_TASKS_GET, METHOD_TASKS_UPDATE,
    Nullable, RESULT_TYPE_TASK, RetentionPolicy, SweepAction, SweepReport, Task, TaskAckResult,
    TaskFields, TaskState, TaskStatus, TaskStatusNotificationParams, TaskStore, TaskStoreError,
    TaskSubscriptionAcknowledgedNotifications, TaskSubscriptionNotifications, UpdateTaskParams,
    capability, declared_by_client, declared_by_server, missing_capability_error,
    validate_identifier,
};
