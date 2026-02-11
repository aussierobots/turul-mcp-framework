# turul-mcp-task-storage

Pluggable task storage backends for the turul-mcp-framework, providing durable state management for MCP 2025-11-25 long-running tasks.

## Overview

`turul-mcp-task-storage` provides the `TaskStorage` trait and multiple backend implementations for persisting MCP task state, results, and lifecycle metadata. Tasks track long-running operations (tool calls, sampling, elicitation) with a state machine that enforces valid status transitions.

The storage layer is **runtime-agnostic** — the `TaskStorage` trait has zero Tokio types in its public API. Runtime concerns (cancellation tokens, watch channels) live in the server crate's `TaskRuntime`.

## Backends

| Backend | Feature | External Dependency | Use Case |
|---------|---------|-------------------|----------|
| **InMemory** | `in-memory` (default) | None | Development, testing, single-instance |
| **SQLite** | `sqlite` | File-based | Single-server, embedded deployments |
| **PostgreSQL** | `postgres` | PostgreSQL server | Multi-server, production |
| **DynamoDB** | `dynamodb` | AWS DynamoDB | Serverless, AWS-native |

## Features

- **Pluggable Architecture** - Swap backends via the `TaskStorage` trait
- **State Machine Enforcement** - Only valid status transitions allowed
- **Result Storage** - Store success (`Value`) or error (JSON-RPC error) outcomes
- **Session Binding** - Tasks are scoped to sessions for isolation
- **Cursor Pagination** - Paginated task listing with deterministic `(created_at, task_id)` ordering
- **TTL Expiry** - Automatic cleanup of expired tasks
- **Stuck Task Recovery** - Fail tasks left in non-terminal state after restart
- **Optimistic Locking** - PostgreSQL uses a `version` column; DynamoDB uses conditional writes
- **Parity Test Suite** - Shared tests verify identical behavior across all backends
- **Runtime-Agnostic** - Zero Tokio in public API; backends use Tokio internally behind feature flags

## Quick Start

```toml
[dependencies]
turul-mcp-task-storage = "0.3"  # InMemory (default)
```

### In-Memory (Development)

```rust
use turul_mcp_task_storage::InMemoryTaskStorage;
use std::sync::Arc;

let storage = Arc::new(InMemoryTaskStorage::new());
```

### SQLite (Single Server)

```toml
[dependencies]
turul-mcp-task-storage = { version = "0.3", features = ["sqlite"] }
```

```rust,ignore
use turul_mcp_task_storage::{SqliteTaskConfig, SqliteTaskStorage};
use std::sync::Arc;

let config = SqliteTaskConfig {
    database_path: "mcp_tasks.db".into(),
    ..SqliteTaskConfig::default()
};
let storage = Arc::new(SqliteTaskStorage::with_config(config).await?);
```

### PostgreSQL (Multi-Server)

```toml
[dependencies]
turul-mcp-task-storage = { version = "0.3", features = ["postgres"] }
```

```rust,ignore
use turul_mcp_task_storage::{PostgresTaskConfig, PostgresTaskStorage};
use std::sync::Arc;

let config = PostgresTaskConfig {
    database_url: "postgres://user:pass@host:5432/mydb".to_string(),
    ..PostgresTaskConfig::default()
};
let storage = Arc::new(PostgresTaskStorage::with_config(config).await?);
```

### DynamoDB (Serverless)

```toml
[dependencies]
turul-mcp-task-storage = { version = "0.3", features = ["dynamodb"] }
```

```rust,ignore
use turul_mcp_task_storage::{DynamoDbTaskConfig, DynamoDbTaskStorage};
use std::sync::Arc;

let config = DynamoDbTaskConfig {
    table_name: "my-mcp-tasks".to_string(),
    ..DynamoDbTaskConfig::default()
};
let storage = Arc::new(DynamoDbTaskStorage::with_config(config).await?);
```

### With Server Builder

```rust
use turul_mcp_server::prelude::*;
use turul_mcp_task_storage::InMemoryTaskStorage;
use std::sync::Arc;

let server = McpServer::builder()
    .name("my-task-server")
    .version("0.3.0")
    .with_task_storage(Arc::new(InMemoryTaskStorage::new()))
    .tool(MyLongRunningTool::default())
    .build()?;
```

## Core Types

### `TaskStorage` Trait

The main trait for task persistence backends:

```rust
pub trait TaskStorage: Send + Sync {
    fn backend_name(&self) -> &'static str;

    // CRUD
    async fn create_task(&self, task: TaskRecord) -> Result<TaskRecord, TaskStorageError>;
    async fn get_task(&self, task_id: &str) -> Result<Option<TaskRecord>, TaskStorageError>;
    async fn update_task(&self, task: TaskRecord) -> Result<(), TaskStorageError>;
    async fn delete_task(&self, task_id: &str) -> Result<bool, TaskStorageError>;

    // Listing (paginated)
    async fn list_tasks(&self, cursor: Option<&str>, limit: Option<u32>)
        -> Result<TaskListPage, TaskStorageError>;

    // Status updates (state machine enforced)
    async fn update_task_status(&self, task_id: &str, new_status: TaskStatus,
        status_message: Option<String>) -> Result<TaskRecord, TaskStorageError>;

    // Result storage
    async fn store_task_result(&self, task_id: &str, result: TaskOutcome)
        -> Result<(), TaskStorageError>;
    async fn get_task_result(&self, task_id: &str)
        -> Result<Option<TaskOutcome>, TaskStorageError>;

    // Cleanup and maintenance
    async fn expire_tasks(&self) -> Result<Vec<String>, TaskStorageError>;
    async fn task_count(&self) -> Result<usize, TaskStorageError>;
    async fn maintenance(&self) -> Result<(), TaskStorageError>;

    // Session binding
    async fn list_tasks_for_session(&self, session_id: &str,
        cursor: Option<&str>, limit: Option<u32>)
        -> Result<TaskListPage, TaskStorageError>;

    // Recovery
    async fn recover_stuck_tasks(&self, max_age_ms: u64)
        -> Result<Vec<String>, TaskStorageError>;
}
```

### `TaskRecord`

The persistence model for a task:

| Field | Type | Description |
|-------|------|-------------|
| `task_id` | `String` | UUID v7 identifier |
| `session_id` | `Option<String>` | Bound session for isolation |
| `status` | `TaskStatus` | Current lifecycle status |
| `status_message` | `Option<String>` | Human-readable status detail |
| `created_at` | `String` | ISO 8601 creation time |
| `last_updated_at` | `String` | ISO 8601 last update time |
| `ttl` | `Option<i64>` | Time-to-live in milliseconds |
| `poll_interval` | `Option<u64>` | Suggested polling interval |
| `original_method` | `String` | e.g., `"tools/call"` |
| `original_params` | `Option<Value>` | Original request params |
| `result` | `Option<TaskOutcome>` | Success or error outcome |
| `meta` | `Option<HashMap<String, Value>>` | Arbitrary metadata |

### `TaskOutcome`

Distinguishes between successful and failed task results:

```rust
pub enum TaskOutcome {
    /// Underlying request succeeded — Value is the result object
    Success(Value),
    /// Underlying request failed — JSON-RPC error fields
    Error { code: i64, message: String, data: Option<Value> },
}
```

The `tasks/result` handler returns `Success` as a JSON-RPC result and `Error` as a JSON-RPC error, preserving the original error code.

### `TaskStorageError`

Unified error type with variants for all failure modes:

- `TaskNotFound` — task ID doesn't exist
- `InvalidTransition` — state machine violation (e.g., `Completed` -> `Working`)
- `TerminalState` — task already in terminal state
- `TaskExpired` — task exceeded TTL
- `MaxTasksReached` — storage capacity limit
- `ConcurrentModification` — optimistic locking conflict
- `DatabaseError`, `SerializationError`, `Generic` — backend-specific errors

## State Machine

Valid status transitions (enforced by `update_task_status`):

```
Working -> InputRequired | Completed | Failed | Cancelled
InputRequired -> Working | Completed | Failed | Cancelled
Completed/Failed/Cancelled -> ERROR (terminal, no transitions)
```

Any invalid transition returns `TaskStorageError::InvalidTransition`.

## Backend Details

### SQLite

- Shared in-memory cache for connection pooling (`:memory:` uses `file:{uuid}?mode=memory&cache=shared`)
- Background cleanup task for TTL expiry
- Indexes: `(created_at, task_id)` for pagination, `(session_id, created_at, task_id)` for session queries, `(status)` for recovery
- TTL computed via `julianday('now') - julianday(created_at)` in milliseconds

### PostgreSQL

- Connection pool with `PgPool` (configurable min/max connections, idle timeout, max lifetime)
- `version` column for optimistic locking on status updates — concurrent modifications return `ConcurrentModification`
- `JSONB` columns for `original_params`, `result`, and `meta`
- Partial index `idx_tasks_active` on `(last_updated_at) WHERE status IN ('working', 'input_required')` for efficient stuck task recovery
- Background cleanup task for TTL expiry

### DynamoDB

- Single table design with `task_id` as partition key
- Two GSIs: `SessionIndex` (PK: `session_id`, SK: `created_at`) and `StatusIndex` (PK: `status`, SK: `created_at`)
- Conditional writes for concurrency control (`attribute_not_exists` on create, `#status = :expected` on update)
- DynamoDB native TTL via `ttl_epoch` attribute for automatic expiry
- Global `list_tasks` uses Scan with best-effort ordering; `list_tasks_for_session` uses GSI Query with deterministic ordering

## Testing

```bash
# InMemory tests (default)
cargo test -p turul-mcp-task-storage

# SQLite tests (in-memory, no external deps)
cargo test -p turul-mcp-task-storage --features sqlite

# PostgreSQL tests (needs Docker postgres)
cargo test -p turul-mcp-task-storage --features postgres -- --ignored

# DynamoDB tests (needs AWS credentials)
cargo test -p turul-mcp-task-storage --features dynamodb -- --ignored

# All features
cargo test -p turul-mcp-task-storage --all-features

# Verify zero Tokio in public API
cargo check -p turul-mcp-task-storage --no-default-features
```

## Feature Flags

```toml
[features]
default = ["in-memory"]
in-memory = ["tokio"]       # InMemory backend (tokio::sync::RwLock)
sqlite = ["sqlx", "tokio"]  # SQLite backend
postgres = ["sqlx", "tokio"] # PostgreSQL backend
dynamodb = ["aws-config", "aws-sdk-dynamodb", "tokio", "base64"] # DynamoDB backend
```

With `--no-default-features`, only the `TaskStorage` trait, error types, and state machine are available — no runtime dependency.

## Architecture

This crate follows the same pluggable pattern as `turul-mcp-session-storage`:

- **Trait-based** — implement `TaskStorage` for any backend
- **Runtime-agnostic public API** — Tokio only used internally by backend implementations
- **Three-layer split** — storage (this crate) / executor (`turul-mcp-server`) / runtime (`TaskRuntime`)
- **Parity testing** — shared test suite (`parity_tests.rs`) verifies identical behavior across all backends

The executor and runtime layers live in `turul-mcp-server` because they involve Tokio-specific concerns (spawn, cancellation tokens, watch channels) that don't belong in a storage abstraction.

## License

Licensed under the MIT License. See [LICENSE](../../LICENSE) for details.
