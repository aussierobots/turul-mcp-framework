# ADR-017: Task Runtime-Executor Boundary

**Status**: Accepted

**Date**: 2026-02-11

## Context

The MCP 2025-11-25 specification introduces Tasks for tracking long-running operations
(tool calls, sampling, elicitation). Implementing tasks requires three distinct concerns:

1. **Persistence** -- storing task records, their status, results, and TTL metadata
   across restarts and across multiple server instances.
2. **Execution** -- spawning the actual work, racing it against cancellation, and
   notifying waiters when the work reaches a terminal state.
3. **Coordination** -- wiring persistence and execution together so that handlers
   like `tasks/get`, `tasks/cancel`, and `tasks/result` have a single entry point.

Early prototypes placed all three concerns in a single crate with a hard Tokio
dependency. This prevented the storage abstraction from being used in environments
that do not run a Tokio runtime (e.g., AWS Lambda custom runtimes, WASM targets,
or test harnesses that want synchronous storage verification). It also meant that
every storage backend (InMemory, SQLite, PostgreSQL, DynamoDB) pulled in
`tokio::sync::watch` and `tokio::spawn` even though those primitives are irrelevant
to persistence.

## Decision

Split task support into three layers with clear ownership boundaries:

### Layer 1: TaskStorage (persistence)

**Crate**: `turul-mcp-task-storage`

A trait with 14 methods (1 sync, 13 async) covering CRUD, pagination, status
transitions with state-machine enforcement, result storage, TTL expiry, session
binding, and stuck-task recovery. The trait uses only `async_trait` -- zero Tokio
primitives in the public API. This allows `cargo check -p turul-mcp-task-storage
--no-default-features` to succeed without Tokio on the dependency graph.

Key types: `TaskStorage`, `TaskRecord`, `TaskOutcome`, `TaskListPage`,
`TaskStorageError`.

Backends: `InMemoryTaskStorage`, `SqliteTaskStorage` (feature `sqlite`),
`PostgresTaskStorage` (feature `postgres`), `DynamoDbTaskStorage` (feature
`dynamodb`).

### Layer 2: TaskExecutor (execution)

**Crate**: `turul-mcp-server` (module `task::executor`)

A trait with 3 async methods:

- `start_task(task_id, BoxedTaskWork) -> Result<Box<dyn TaskHandle>, TaskStorageError>`
- `cancel_task(task_id) -> Result<(), TaskStorageError>`
- `await_terminal(task_id) -> Option<TaskStatus>`

`BoxedTaskWork` is defined as:
```rust
Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = TaskOutcome> + Send>> + Send>
```

`TaskHandle` is a sub-trait with two synchronous methods: `cancel()` and
`is_cancelled()`.

The default implementation is `TokioTaskExecutor` (module `task::tokio_executor`),
which uses:

- `tokio::spawn` to run work on the Tokio runtime
- `tokio::select!` to race work against a `CancellationHandle`
- `tokio::sync::watch` channels to notify `await_terminal` callers when a task
  reaches a terminal status
- `HashMap<String, TokioTaskEntry>` behind `Arc<RwLock>` for in-flight tracking

### Layer 3: TaskRuntime (coordination)

**Crate**: `turul-mcp-server` (module `task::runtime`)

A concrete struct that owns `Arc<dyn TaskStorage>` + `Arc<dyn TaskExecutor>` and
provides the unified API consumed by MCP request handlers. Convenience constructors:

- `TaskRuntime::new(storage, executor)` -- full control
- `TaskRuntime::with_default_executor(storage)` -- uses `TokioTaskExecutor`
- `TaskRuntime::in_memory()` -- `InMemoryTaskStorage` + `TokioTaskExecutor`

Exposes lifecycle methods (`register_task`, `update_status`, `complete_task`,
`cancel_task`, `await_terminal`) that coordinate between storage and executor,
plus direct delegation to storage for read-only queries (`get_task`,
`get_task_result`, `list_tasks`, `list_tasks_for_session`).

### CancellationHandle (runtime primitive)

**Crate**: `turul-mcp-server` (module `cancellation`)

Wraps a `tokio::sync::watch<bool>` channel. Clone-friendly -- both the executor
and the spawned work hold copies. Methods: `new()`, `cancel()`, `is_cancelled()`,
`cancelled()` (async wait). Lives in the server crate, NOT the storage crate,
because `tokio::sync::watch` is a runtime-specific primitive.

## Consequences

### Positive

- **Storage crate is runtime-agnostic.** It compiles without Tokio, enabling use
  in WASM, embedded, or alternative async runtime environments. Each durable
  backend (SQLite, PostgreSQL, DynamoDB) opts into Tokio only via its own feature
  flag.
- **Executor is replaceable.** Future executors (SQS-backed, Step Functions,
  EventBridge worker models) can implement `TaskExecutor` without modifying
  storage backends or the runtime coordinator.
- **Clean testability.** Storage backends can be tested with a mock executor (or
  no executor at all). The executor can be tested with in-memory storage. Neither
  layer needs the other's internals.
- **No runtime leak into persistence.** Cancellation tokens, watch channels, and
  spawn handles stay in the server crate where they belong. Storage backends never
  see `tokio::sync` types.
- **Composable constructors.** `TaskRuntime::in_memory()` provides zero-config
  getting started. `TaskRuntime::new(postgres_storage, sqs_executor)` provides
  full production customization.

### Negative

- **Two-step task start.** Callers must first `register_task` (storage) then
  `executor.start_task` (execution) rather than a single atomic call. This is
  intentional -- the task record must exist before execution begins so that
  `tasks/get` can return status immediately -- but it requires callers to handle
  both steps.
- **Error code wrapping.** When `tasks/result` returns an error, the original
  JSON-RPC error code from the underlying request may be wrapped in
  `McpError::ToolExecutionError`, losing the original code. This is a known
  pre-existing issue, not caused by the layer split.
- **Three modules instead of one.** Developers must understand which layer owns
  what. Mitigation: `TaskRuntime` is the single entry point for handlers, so most
  code interacts only with that type.

## Implementation

### Key File Paths

| Layer | Path |
|-------|------|
| TaskStorage trait | `crates/turul-mcp-task-storage/src/traits.rs` |
| InMemoryTaskStorage | `crates/turul-mcp-task-storage/src/in_memory.rs` |
| SqliteTaskStorage | `crates/turul-mcp-task-storage/src/sqlite.rs` |
| PostgresTaskStorage | `crates/turul-mcp-task-storage/src/postgres.rs` |
| DynamoDbTaskStorage | `crates/turul-mcp-task-storage/src/dynamodb.rs` |
| State machine | `crates/turul-mcp-task-storage/src/state_machine.rs` |
| Parity tests | `crates/turul-mcp-task-storage/src/parity_tests.rs` |
| TaskExecutor trait | `crates/turul-mcp-server/src/task/executor.rs` |
| TokioTaskExecutor | `crates/turul-mcp-server/src/task/tokio_executor.rs` |
| CancellationHandle | `crates/turul-mcp-server/src/cancellation.rs` |
| TaskRuntime | `crates/turul-mcp-server/src/task/runtime.rs` |
| Task handlers | `crates/turul-mcp-server/src/task/handlers.rs` |

### Dependency Direction

```
turul-mcp-server
  ├── turul-mcp-task-storage  (TaskStorage trait, TaskRecord, TaskOutcome)
  ├── turul-mcp-protocol      (TaskStatus, Task)
  └── tokio                    (spawn, watch, select!, RwLock)

turul-mcp-task-storage
  ├── turul-mcp-protocol      (TaskStatus)
  ├── async-trait
  ├── serde / serde_json
  └── [optional] tokio, sqlx, aws-sdk-dynamodb  (per feature flag)
```

Dependencies flow downward only. The storage crate never depends on the server
crate. The executor trait lives in the server crate and depends on storage types
(`TaskOutcome`, `TaskStorageError`) for its method signatures.

## Alternatives Considered

### Single crate with Tokio everywhere

Place `TaskStorage`, `TaskExecutor`, and `TaskRuntime` in one crate with a hard
Tokio dependency. Rejected because it prevents portability to non-Tokio runtimes
and forces every storage backend to pull in runtime primitives it does not use.

### Storage owns cancellation

Put `CancellationHandle` in `turul-mcp-task-storage` alongside `TaskRecord`.
Rejected because `CancellationHandle` wraps `tokio::sync::watch`, which is a
runtime-specific primitive. Placing it in the storage crate would violate the
"zero Tokio in public API" guarantee and prevent the storage crate from compiling
without Tokio.

### Executor in its own crate

Create a third crate `turul-mcp-task-executor` for the executor trait and Tokio
implementation. Rejected as premature -- the executor trait has only 3 methods and
one implementation today. If a second executor (e.g., SQS-backed) is added, the
trait can be extracted into its own crate at that time without breaking the public
API.

## See Also

- [ADR-015: MCP 2025-11-25 Protocol Crate Strategy](./015-mcp-2025-11-25-protocol-crate.md) -- protocol types that define `TaskStatus`, `Task`
- [ADR-016: Task Storage Architecture](./016-task-storage-architecture.md) -- storage trait design, backend parity testing, state machine enforcement
- [ADR-018: Task Pagination Cursor Contract](./018-task-pagination-cursor-contract.md) -- deterministic cursor-based pagination across backends
