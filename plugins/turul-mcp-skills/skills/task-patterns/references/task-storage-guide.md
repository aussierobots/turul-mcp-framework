# Task Storage Guide

Reference for the `TaskStorage` trait, `TaskRecord`, `TaskOutcome`, and backend configurations.

## Import

```rust
use turul_mcp_task_storage::{
    TaskStorage, TaskRecord, TaskOutcome, TaskListPage, InMemoryTaskStorage,
};
use turul_mcp_protocol::TaskStatus;
```

## TaskRecord

The persistence model for a task. Contains only serializable fields — runtime handles are managed by the executor.

```rust
pub struct TaskRecord {
    pub task_id: String,             // UUID v7 for temporal ordering
    pub session_id: Option<String>,  // Session this task belongs to
    pub status: TaskStatus,          // Current lifecycle state
    pub status_message: Option<String>,
    pub created_at: String,          // ISO 8601 datetime
    pub last_updated_at: String,     // ISO 8601 datetime
    pub ttl: Option<i64>,            // Time-to-live in milliseconds
    pub poll_interval: Option<u64>,  // Suggested polling interval in ms
    pub original_method: String,     // e.g., "tools/call"
    pub original_params: Option<Value>,
    pub result: Option<TaskOutcome>, // Populated on completion
    pub meta: Option<HashMap<String, Value>>,
}
```

### TaskStatus (from protocol crate)

```rust
pub enum TaskStatus {
    Working,        // Task is actively executing
    InputRequired,  // Task needs client input to continue
    Completed,      // Terminal: succeeded
    Failed,         // Terminal: failed
    Cancelled,      // Terminal: cancelled by client
}
```

## TaskOutcome

The result of a task's underlying operation, stored verbatim for `tasks/result`.

```rust
pub enum TaskOutcome {
    /// Successful result — Value is the response object (e.g., CallToolResult)
    Success(Value),
    /// Failed result — JSON-RPC error fields for verbatim reproduction
    Error {
        code: i64,
        message: String,
        data: Option<Value>,
    },
}
```

## TaskStorage Trait

All backends implement this trait. 14 methods covering CRUD, status updates, results, cleanup, and session binding.

### Task CRUD

```rust
async fn create_task(&self, task: TaskRecord) -> Result<TaskRecord, TaskStorageError>;
async fn get_task(&self, task_id: &str) -> Result<Option<TaskRecord>, TaskStorageError>;
async fn update_task(&self, task: TaskRecord) -> Result<(), TaskStorageError>;
async fn delete_task(&self, task_id: &str) -> Result<bool, TaskStorageError>;
```

### Listing (Paginated)

```rust
async fn list_tasks(
    &self, cursor: Option<&str>, limit: Option<u32>,
) -> Result<TaskListPage, TaskStorageError>;

async fn list_tasks_for_session(
    &self, session_id: &str, cursor: Option<&str>, limit: Option<u32>,
) -> Result<TaskListPage, TaskStorageError>;
```

`TaskListPage`:
```rust
pub struct TaskListPage {
    pub tasks: Vec<TaskRecord>,
    pub next_cursor: Option<String>,  // None = last page
}
```

### Status Updates (State Machine)

```rust
async fn update_task_status(
    &self, task_id: &str, new_status: TaskStatus, status_message: Option<String>,
) -> Result<TaskRecord, TaskStorageError>;
```

**Enforces the state machine.** Returns `TaskStorageError::InvalidTransition` or `TaskStorageError::TerminalState` for illegal transitions.

### Result Storage

```rust
async fn store_task_result(
    &self, task_id: &str, result: TaskOutcome,
) -> Result<(), TaskStorageError>;

async fn get_task_result(
    &self, task_id: &str,
) -> Result<Option<TaskOutcome>, TaskStorageError>;
```

### Cleanup & Recovery

```rust
async fn expire_tasks(&self) -> Result<Vec<String>, TaskStorageError>;
async fn task_count(&self) -> Result<usize, TaskStorageError>;
async fn maintenance(&self) -> Result<(), TaskStorageError>;
async fn recover_stuck_tasks(&self, max_age_ms: u64) -> Result<Vec<String>, TaskStorageError>;
```

- `expire_tasks()` — Remove tasks past their TTL
- `maintenance()` — Periodic cleanup/compaction
- `recover_stuck_tasks()` — Called on startup to mark stale `Working`/`InputRequired` tasks as `Failed`

### Backend Name

```rust
fn backend_name(&self) -> &'static str;  // e.g., "in-memory", "sqlite"
```

## Backend Quick Reference

### InMemory (default — no feature flag)

```rust
use turul_mcp_task_storage::InMemoryTaskStorage;
let storage = Arc::new(InMemoryTaskStorage::new());
```

- Zero configuration, ephemeral (lost on restart)
- Best for: development, testing, single-instance stateless servers

### SQLite (feature = "sqlite")

```toml
[dependencies]
turul-mcp-task-storage = { version = "0.3", features = ["sqlite"] }
```

```rust
use turul_mcp_task_storage::SqliteTaskStorage;
let storage = Arc::new(SqliteTaskStorage::new("sqlite://tasks.db").await?);
```

- Persistent, single-instance only
- Auto-creates tables on connect

### PostgreSQL (feature = "postgres")

```toml
[dependencies]
turul-mcp-task-storage = { version = "0.3", features = ["postgres"] }
```

```rust
use turul_mcp_task_storage::PostgresTaskStorage;
let storage = Arc::new(PostgresTaskStorage::new("postgres://localhost/mydb").await?);
```

- Persistent, multi-instance safe (optimistic locking via `version` column)
- Auto-creates tables on connect

### DynamoDB (feature = "dynamodb")

```toml
[dependencies]
turul-mcp-task-storage = { version = "0.3", features = ["dynamodb"] }
```

```rust
use turul_mcp_task_storage::DynamoDbTaskStorage;
let storage = Arc::new(DynamoDbTaskStorage::new("mcp-tasks").await?);
```

- Persistent, serverless-native, multi-instance safe
- Requires GSIs: `SessionIndex`, `StatusIndex`
- Uses DynamoDB native TTL
- Table must be pre-created (see `storage-backend-matrix` reference for schema)
