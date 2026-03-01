# Task Runtime Guide

Reference for `TaskRuntime`, `TaskExecutor`, `TokioTaskExecutor`, and cancellation handling.

## Import

```rust
use turul_mcp_server::task::TaskRuntime;
// TaskExecutor and TokioTaskExecutor are internal — accessed via TaskRuntime
```

## TaskRuntime

Bridges durable storage with runtime execution. Owns both a `TaskStorage` backend and a `TaskExecutor`.

### Constructors

```rust
// Full control: custom storage + custom executor
let runtime = TaskRuntime::new(storage, executor);

// Default executor (TokioTaskExecutor) with custom storage
let runtime = TaskRuntime::with_default_executor(storage);

// Convenience: InMemoryTaskStorage + TokioTaskExecutor
let runtime = TaskRuntime::in_memory();

// With custom recovery timeout (default: 5 minutes / 300,000 ms)
let runtime = TaskRuntime::with_default_executor(storage)
    .with_recovery_timeout(600_000);  // 10 minutes
```

### Server Integration

```rust
use std::sync::Arc;

// Option A: Let the builder create the runtime from storage
let server = McpServer::builder()
    .with_task_storage(Arc::new(InMemoryTaskStorage::new()))
    .build()?;

// Option B: Provide a pre-built runtime
let runtime = Arc::new(TaskRuntime::in_memory());
let server = McpServer::builder()
    .with_task_runtime(runtime)
    .build()?;
```

`.with_task_storage()` internally calls `TaskRuntime::with_default_executor()` and wraps it in `Arc`. Use `.with_task_runtime()` when you need custom executor or recovery timeout.

### Task Lifecycle Methods

| Method | Description |
|---|---|
| `register_task(task)` | Persist a new task in storage. Does NOT start execution. |
| `update_status(task_id, status, message)` | Update task status with state machine enforcement. |
| `complete_task(task_id, outcome, status, message)` | Store result + update status atomically. |
| `cancel_task(task_id)` | Signal executor to cancel + update storage to `Cancelled`. |
| `await_terminal(task_id)` | Block until the task reaches a terminal status. Returns `None` if not tracked. |

### Storage Delegation

| Method | Description |
|---|---|
| `get_task(task_id)` | Get task record from storage. |
| `get_task_result(task_id)` | Get stored `TaskOutcome`. |
| `list_tasks(cursor, limit)` | Paginated task listing. |
| `list_tasks_for_session(session_id, cursor, limit)` | Paginated listing for a session. |

### Recovery

| Method | Description |
|---|---|
| `recover_stuck_tasks()` | Mark stale non-terminal tasks as `Failed`. Called on server startup. |
| `maintenance()` | Run periodic cleanup (TTL expiry, compaction). |

## TaskExecutor Trait

Abstraction for how task work is executed. The default implementation is `TokioTaskExecutor`.

```rust
#[async_trait]
pub trait TaskExecutor: Send + Sync {
    async fn start_task(
        &self,
        task_id: &str,
        work: BoxedTaskWork,
    ) -> Result<Box<dyn TaskHandle>, TaskStorageError>;

    async fn cancel_task(&self, task_id: &str) -> Result<(), TaskStorageError>;

    async fn await_terminal(&self, task_id: &str) -> Option<TaskStatus>;
}
```

- `start_task()` — Spawn the work and return a `TaskHandle` for cancellation
- `cancel_task()` — Signal cancellation to a running task
- `await_terminal()` — Block until the task finishes

## TokioTaskExecutor

The default executor. Uses `tokio::spawn` + `tokio::select!` for cancellation.

```rust
// Created automatically by TaskRuntime::with_default_executor()
// or TaskRuntime::in_memory()
```

### How Cancellation Works

When `cancel_task(task_id)` is called:

1. The executor looks up the running task by ID
2. It signals the internal `CancellationToken`
3. The `tokio::select!` branch detects cancellation
4. The task transitions to `Cancelled` state

```
                 ┌──────────────────────┐
                 │   tokio::select! {   │
                 │     outcome = work    │──→ Completed/Failed
                 │     _ = cancel_rx    │──→ Cancelled
                 │   }                  │
                 └──────────────────────┘
```

**Tool code does not need to check for cancellation.** The executor handles it externally by racing the work future against the cancel signal.

## TaskHandle Trait

Opaque handle for cancellation, returned by `start_task()`.

```rust
pub trait TaskHandle: Send + Sync {
    fn cancel(&self);
    fn is_cancelled(&self) -> bool;
}
```

This is internal to the executor — tool code never interacts with `TaskHandle` directly.

## BoxedTaskWork

The work unit passed to `start_task()`:

```rust
pub type BoxedTaskWork = Box<
    dyn FnOnce() -> Pin<Box<dyn Future<Output = TaskOutcome> + Send>>
        + Send,
>;
```

The framework constructs this from your tool's `execute` method — you don't create it manually.

## Recovery on Startup

When the server starts with a task runtime, it calls `recover_stuck_tasks()`:

1. Queries storage for non-terminal tasks older than `recovery_timeout_ms`
2. Marks them as `Failed` with message `"Recovered on startup"`
3. Logs the count of recovered tasks

This handles ungraceful shutdowns where tasks were `Working` but the server crashed.

```rust
let runtime = TaskRuntime::with_default_executor(storage)
    .with_recovery_timeout(600_000);  // Mark stuck tasks after 10 min
```

Default recovery timeout: 300,000 ms (5 minutes).
