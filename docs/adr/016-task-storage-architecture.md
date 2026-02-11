# ADR-016: Task Storage Architecture

**Status**: Accepted

**Date**: 2026-02-11

## Context

MCP 2025-11-25 introduces Tasks as an experimental capability for tracking long-running
operations. Tasks are an experimental MCP 2025-11-25 capability; this framework provides
full implementation support (protocol types, storage, runtime, handlers, and tests).
A tool call that cannot complete synchronously returns a `Task` object instead of a
direct result; the client then polls for status updates and eventually retrieves the
outcome via `tasks/result`.

The framework already has a session storage layer (ADR-001) for session state and SSE
events, but tasks have fundamentally different requirements:

- **State machine enforcement** -- task statuses follow a strict lifecycle
  (`Working` -> terminal) that must be validated on every transition.
- **Result persistence** -- the outcome of the underlying request (success or
  JSON-RPC error) must be stored verbatim and returned exactly as the original
  request would have returned.
- **Session-scoped isolation** -- tasks are bound to a session for authorization;
  listing and access must be scoped accordingly.
- **Crash recovery** -- tasks that were in-progress when the server stopped must be
  recoverable on restart (marked as `Failed` rather than left in limbo).
- **TTL-based expiry** -- tasks must automatically expire after a configurable
  time-to-live to prevent unbounded storage growth.

Reusing `SessionStorage` was considered but rejected because the trait surface,
data model, and consistency guarantees differ enough that a combined interface
would be unwieldy. A dedicated `TaskStorage` trait keeps each concern focused.

## Decision

Create a new `turul-mcp-task-storage` crate that provides:

1. A `TaskStorage` trait with 14 async methods covering CRUD, listing, status
   transitions, result storage, cleanup, and recovery.
2. A centralized state machine module (`state_machine.rs`) that all backends
   delegate to for transition validation.
3. Four backend implementations behind feature flags, mirroring the session
   storage backend matrix.
4. A parity test suite that verifies identical behavior across all backends.

### TaskStorage Trait

```rust
#[async_trait]
pub trait TaskStorage: Send + Sync {
    fn backend_name(&self) -> &'static str;

    // CRUD
    async fn create_task(&self, task: TaskRecord) -> Result<TaskRecord, TaskStorageError>;
    async fn get_task(&self, task_id: &str) -> Result<Option<TaskRecord>, TaskStorageError>;
    async fn update_task(&self, task: TaskRecord) -> Result<(), TaskStorageError>;
    async fn delete_task(&self, task_id: &str) -> Result<bool, TaskStorageError>;

    // Listing (cursor-paginated)
    async fn list_tasks(&self, cursor: Option<&str>, limit: Option<u32>)
        -> Result<TaskListPage, TaskStorageError>;
    async fn list_tasks_for_session(&self, session_id: &str, cursor: Option<&str>, limit: Option<u32>)
        -> Result<TaskListPage, TaskStorageError>;

    // Status (state machine enforcement)
    async fn update_task_status(&self, task_id: &str, new_status: TaskStatus, status_message: Option<String>)
        -> Result<TaskRecord, TaskStorageError>;

    // Result storage
    async fn store_task_result(&self, task_id: &str, result: TaskOutcome) -> Result<(), TaskStorageError>;
    async fn get_task_result(&self, task_id: &str) -> Result<Option<TaskOutcome>, TaskStorageError>;

    // Cleanup
    async fn expire_tasks(&self) -> Result<Vec<String>, TaskStorageError>;
    async fn task_count(&self) -> Result<usize, TaskStorageError>;
    async fn maintenance(&self) -> Result<(), TaskStorageError>;

    // Recovery
    async fn recover_stuck_tasks(&self, max_age_ms: u64) -> Result<Vec<String>, TaskStorageError>;
}
```

### Data Model

```rust
pub struct TaskRecord {
    pub task_id: String,           // UUID v7 for temporal ordering
    pub session_id: Option<String>,
    pub status: TaskStatus,
    pub status_message: Option<String>,
    pub created_at: String,        // ISO 8601
    pub last_updated_at: String,   // ISO 8601
    pub ttl: Option<i64>,          // milliseconds from creation
    pub poll_interval: Option<u64>,// suggested polling interval (ms)
    pub original_method: String,   // e.g. "tools/call"
    pub original_params: Option<Value>,
    pub result: Option<TaskOutcome>,
    pub meta: Option<HashMap<String, Value>>,
}

pub enum TaskOutcome {
    Success(Value),
    Error { code: i64, message: String, data: Option<Value> },
}

pub struct TaskListPage {
    pub tasks: Vec<TaskRecord>,
    pub next_cursor: Option<String>,
}
```

### State Machine

All backends delegate to a single `validate_transition()` function in
`state_machine.rs`:

```text
Working       -> InputRequired | Completed | Failed | Cancelled
InputRequired -> Working | Completed | Failed | Cancelled
Completed     -> ERROR (terminal)
Failed        -> ERROR (terminal)
Cancelled     -> ERROR (terminal)
```

Self-transitions (`Working -> Working`, `InputRequired -> InputRequired`) are
also rejected. Terminal states reject all outbound transitions with
`TaskStorageError::TerminalState`.

### Runtime-Agnostic Public API

The `TaskStorage` trait and all public types have zero Tokio dependency. The crate
compiles with `--no-default-features`:

```bash
cargo check -p turul-mcp-task-storage --no-default-features
```

Tokio is pulled in only by the concrete backend implementations via feature flags,
keeping the trait usable from any async runtime.

## Consequences

### Positive

- **Pluggable backends** -- the same `Arc<dyn TaskStorage>` interface works across
  InMemory, SQLite, PostgreSQL, and DynamoDB, matching the session storage pattern
  developers already know.
- **Centralized state machine** -- transition logic lives in one place. Backends
  cannot diverge on which transitions are legal.
- **Parity test suite** -- 11 shared test functions verify that all backends
  produce identical behavior for the same operations, catching subtle differences
  in SQL semantics or eventual consistency.
- **Zero-config default** -- `InMemoryTaskStorage` is enabled by the `default`
  feature, requiring no infrastructure for development and testing.
- **Crash recovery** -- `recover_stuck_tasks` provides a server-startup hook to
  fail-safe any tasks left in non-terminal states after an unclean shutdown.
- **Session isolation** -- `list_tasks_for_session` enforces authorization scoping
  at the storage layer, preventing cross-session task enumeration.

### Negative

- **Separate crate from session storage** -- task storage and session storage are
  independent crates with similar but distinct traits. This means two storage
  backends must be configured for a full deployment (though both default to
  in-memory).
- **PostgreSQL/DynamoDB tests require infrastructure** -- these tests are
  `#[ignore]` by default and only run in CI environments with the appropriate
  databases provisioned.
- **DynamoDB global list is best-effort** -- `list_tasks` on DynamoDB uses a
  `Scan` operation, which does not guarantee strict ordering. The session-scoped
  `list_tasks_for_session` uses a GSI Query and is deterministic.

### Risks

- **Storage/executor coupling** -- the `TaskRecord` stores `original_params` for
  replay, but the executor (in `turul-mcp-server`) owns cancellation handles and
  status watches. If the boundary between storage and execution is not maintained,
  runtime state could leak into the persistence layer.
  Mitigation: `TaskRecord` contains only serializable fields; runtime handles are
  managed by `TaskExecutor` in the server crate.
- **TTL clock skew** -- TTL expiry relies on wall-clock comparisons. In distributed
  deployments with clock skew, tasks may expire earlier or later than expected.
  Mitigation: DynamoDB uses native TTL (server-side), and SQLite/PostgreSQL use
  database-side time functions (`julianday()`, `now()`).
- **Optimistic locking contention** -- PostgreSQL uses a `version` column for
  optimistic concurrency control. Under high contention on a single task, retries
  may be needed. Mitigation: task updates are infrequent (status transitions, not
  high-frequency writes).

## Implementation

### Crate Structure

```
crates/turul-mcp-task-storage/
  Cargo.toml
  src/
    lib.rs              # Re-exports, feature gates
    traits.rs           # TaskStorage trait, TaskRecord, TaskOutcome, TaskListPage
    error.rs            # TaskStorageError enum
    state_machine.rs    # validate_transition(), is_terminal()
    prelude.rs          # Convenience re-exports
    in_memory.rs        # InMemoryTaskStorage (default feature)
    sqlite.rs           # SqliteTaskStorage (sqlite feature)
    postgres.rs         # PostgresTaskStorage (postgres feature)
    dynamodb.rs         # DynamoDbTaskStorage (dynamodb feature)
    parity_tests.rs     # 11 shared test functions for cross-backend verification
```

### Feature Flags

```toml
[features]
default    = ["in-memory"]
in-memory  = ["tokio"]
sqlite     = ["sqlx", "tokio"]
postgres   = ["sqlx", "tokio"]
dynamodb   = ["aws-config", "aws-sdk-dynamodb", "tokio", "base64"]
```

### Backend Support Matrix

| Backend    | Feature Flag | Storage Engine            | Concurrency       | TTL Mechanism            | Cursor Strategy           | Test Status       |
|------------|-------------|---------------------------|--------------------|--------------------------|---------------------------|-------------------|
| InMemory   | `in-memory` | `Arc<RwLock<HashMap>>`    | RwLock             | Manual expiry check      | `(created_at, task_id)`   | Always runs       |
| SQLite     | `sqlite`    | `sqlx::SqlitePool`        | Database locks     | `julianday()` comparison | `(created_at, task_id)`   | Always runs       |
| PostgreSQL | `postgres`  | `sqlx::PgPool`            | Optimistic locking (`version` column) | `now()` comparison | `(created_at, task_id)` | `#[ignore]` (needs infra) |
| DynamoDB   | `dynamodb`  | `aws-sdk-dynamodb` client | Conditional writes | Native DynamoDB TTL      | Base64-encoded compound key | `#[ignore]` (needs infra) |

### Parity Test Functions

The following 10 test functions are defined in `parity_tests.rs` and invoked from
each backend's test module:

1. `test_create_and_retrieve` -- round-trip create/get
2. `test_state_machine_enforcement` -- valid transitions succeed
3. `test_terminal_state_rejection` -- terminal states reject all transitions
4. `test_cursor_determinism` -- paginated listing produces stable ordering
5. `test_session_scoping` -- session-bound listing isolates tasks
6. `test_ttl_expiry` -- expired tasks are cleaned up
7. `test_task_result_round_trip` -- `TaskOutcome` survives store/retrieve
8. `test_recover_stuck_tasks` -- non-terminal tasks are failed on recovery
9. `test_max_tasks_limit` -- pagination limits are respected
10. `test_error_mapping_parity` -- error types are consistent across backends

### Integration with Server

The `turul-mcp-server` crate consumes `TaskStorage` via `TaskRuntime`:

```rust
// Default (in-memory)
let runtime = TaskRuntime::in_memory();

// Custom backend
let storage = Arc::new(SqliteTaskStorage::new(config).await?);
let runtime = TaskRuntime::with_default_executor(storage);

// Full control
let runtime = TaskRuntime::new(storage, executor);
```

The server registers handlers for `tasks/get`, `tasks/list`, `tasks/cancel`, and
`tasks/result`, and advertises task capabilities automatically when a `TaskRuntime`
is configured via the builder.

## See Also

- [ADR-001: Session Storage Architecture](./001-session-storage-architecture.md) -- pluggable session storage (same backend pattern)
- [ADR-015: MCP 2025-11-25 Protocol Crate Strategy](./015-mcp-2025-11-25-protocol-crate.md) -- protocol types that define `TaskStatus`, `Task`
- [ADR-017: Task Runtime-Executor Boundary](./017-task-runtime-executor-boundary.md) -- three-layer split: storage / executor / runtime
- [ADR-018: Task Pagination Cursor Contract](./018-task-pagination-cursor-contract.md) -- deterministic cursor-based pagination across backends
