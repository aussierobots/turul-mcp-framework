# ADR-019: Lambda Task Integration

**Status**: Accepted

**Date**: 2026-02-12

## Context

The MCP 2025-11-25 specification introduces Tasks for long-running operations.
`McpServer::builder()` supports tasks via `.with_task_storage()` and
`.with_task_runtime()`, but the Lambda deployment path
(`LambdaMcpServerBuilder`) had no equivalent. Lambda MCP servers could not
participate in the task protocol at all.

AWS Lambda's execution model introduces constraints that do not exist in
long-running server processes:

1. **Stateless invocations** -- each request may execute on a different Lambda
   instance. In-memory state does not survive between invocations.
2. **Post-response freeze** -- after the response body is sent, Lambda may
   freeze the execution environment. Background `tokio::spawn` tasks are not
   guaranteed to complete.
3. **Cold starts** -- a new execution environment initializes from scratch,
   with no knowledge of prior invocations' in-flight work.
4. **15-minute timeout** -- Lambda has a hard maximum execution time.
5. **Duration-based billing** -- Lambda charges per millisecond of execution
   time (rounded up to the nearest 1ms). Blocking a Lambda invocation while
   waiting for a long-running tool to complete directly increases cost.
   Non-blocking task dispatch is not just a protocol requirement -- it is a
   cost optimization: return `CreateTaskResult` fast and free the Lambda
   invocation. Lambda billing is request + duration based; reducing
   invocation duration usually reduces cost.

These constraints determine which parts of the task system work reliably in
Lambda and which require external coordination.

## Decision

### Reuse the existing task architecture in Lambda

`LambdaMcpServerBuilder` exposes the same task API as `McpServerBuilder`:

- `.with_task_storage(Arc<dyn TaskStorage>)` -- creates a `TaskRuntime` with
  `TokioTaskExecutor` and the given durable storage backend
- `.with_task_runtime(Arc<TaskRuntime>)` -- accepts a pre-built runtime
- `.task_recovery_timeout_ms(u64)` -- configures stuck-task recovery threshold

No Lambda-specific executor is needed. The existing `TokioTaskExecutor` works
within a single invocation, and the `tasks/result` handler's fallback storage
polling (500ms interval, 5-minute timeout) handles cross-invocation queries.

### Capability auto-detection

When task storage is configured, `build()` automatically:

1. Sets `capabilities.tasks` with `list`, `cancel`, and
   `requests.tools.call` sub-capabilities
2. Registers handlers for `tasks/get`, `tasks/list`, `tasks/cancel`,
   `tasks/result` via `SessionAwareMcpHandlerBridge`
3. Wires `SessionAwareToolHandler.with_task_runtime()` for task-augmented
   `tools/call`

When task storage is NOT configured, behavior is identical to before --
no task capability advertised, no handlers registered.

### Cold-start recovery

`recover_stuck_tasks()` runs in `LambdaMcpServer::handler()`, which is called
once from `main()` during Lambda initialization. The returned
`LambdaMcpHandler` is `Clone`'d for each request via `service_fn`. This
guarantees exactly-once-per-cold-start recovery without requiring `AtomicBool`
or `Once` guards.

Tasks left in `Working` status from a prior frozen invocation are marked
`Failed` if they are older than `task_recovery_timeout_ms` (default: 5
minutes).

### Non-blocking tools/call

Task-augmented `tools/call` (when `params.task` is present) MUST return
`CreateTaskResult` immediately. The tool execution is dispatched to
`TokioTaskExecutor` via `tokio::spawn` and runs asynchronously. The Lambda
response is sent before tool work completes.

This is enforced by the existing `SessionAwareToolHandler` logic in
`turul-mcp-server` -- no Lambda-specific branching is needed.

### Durable storage requirement

`InMemoryTaskStorage` loses all state between Lambda invocations and is
unsuitable for production. DynamoDB is the recommended backend for Lambda:

- Native TTL support (no background cleanup goroutine needed)
- Session-scoped GSI for efficient `list_tasks_for_session`
- Conditional writes for safe concurrent access across Lambda instances

## Consequences

### Positive

- **Feature parity** -- Lambda MCP servers can now participate in the task
  protocol identically to long-running servers.
- **Zero new dependencies** -- all required types (`TaskRuntime`,
  `TasksGetHandler`, etc.) are already public from `turul-mcp-server`.
- **Zero new code in turul-mcp-server** -- Lambda integration reuses existing
  handlers, capability types, and the `SessionAwareToolHandler` task path.
- **Backward compatible** -- existing Lambda servers without task storage
  continue to work unchanged.

### Operational Limitations

These are inherent to Lambda's execution model and cannot be fixed by
framework changes. Operators MUST account for them when designing
task-augmented Lambda tools.

- **Post-response task completion is best-effort, not guaranteed.** The
  `tools/call` request path is non-blocking — it returns `CreateTaskResult`
  immediately and dispatches tool work via `tokio::spawn`. However, after
  the Lambda response body closes, Lambda may freeze the execution
  environment at any time. Background tool work that has not completed by
  freeze time will NOT persist its result. This means:
  - **Short-lived tools** (completing within the same invocation) work
    reliably — the executor auto-completes before response close.
  - **Long-running tools** MUST NOT rely on in-process completion. Their
    lifecycle must be driven by an external system (Step Functions, callback
    Lambda, worker) that writes results to durable task storage. See
    "External Task Lifecycle" below.
  - There is no way to make post-response `tokio::spawn` reliable in Lambda.
    This is a platform constraint, not a framework bug.
- **Cross-invocation cancellation is best-effort.** `tasks/cancel` updates
  storage status, but cannot signal a frozen Lambda invocation. Work started
  in a prior invocation may complete after cancellation. Operators should
  treat cancellation as advisory, not guaranteed.
- **`tasks/result` latency for cross-invocation queries.** When the executor
  doesn't track a task (different invocation), the handler falls back to 500ms
  storage polling with a 5-minute timeout. This adds latency compared to the
  non-Lambda path where `await_terminal` resolves via in-memory watch channel.

### External Task Lifecycle (future Phase B)

For Lambda + long-running tools, the recommended pattern is:

1. `tools/call` with `task` creates a task record and dispatches to an
   external orchestrator (e.g., Step Functions `StartExecution`)
2. The external orchestrator drives the work to completion
3. The completion handler writes the result to durable task storage via
   `TaskStorage::store_task_result()` + `update_task_status()`
4. Clients poll `tasks/get` or wait on `tasks/result` (storage-driven)

This requires a `tool_with_external_lifecycle()` builder method that bypasses
the executor's auto-complete path. Deferred to a separate PR (Phase B).

## Implementation

### Key Integration Points

| Component | Location |
|-----------|----------|
| Builder fields + methods | `crates/turul-mcp-aws-lambda/src/builder.rs` |
| Capability auto-detection | `LambdaMcpServerBuilder::build()` |
| Task handler registration | `LambdaMcpServerBuilder::build()` |
| Cold-start recovery | `LambdaMcpServer::handler()` |
| Tool handler wiring | `LambdaMcpServer::handler()` |
| Task runtime field | `crates/turul-mcp-aws-lambda/src/server.rs` |

### Wiring Diagram

```
LambdaMcpServerBuilder
  .with_task_storage(dyn TaskStorage)
       │
       ▼
  TaskRuntime::with_default_executor(storage)
       │
       ├─── Arc<dyn TaskStorage>     (DynamoDB recommended)
       └─── Arc<TokioTaskExecutor>   (in-process async)
       │
       ▼
  LambdaMcpServer::handler()
       │
       ├─── recover_stuck_tasks()    (once per cold start)
       ├─── SessionAwareToolHandler  (.with_task_runtime())
       └─── SessionAwareMcpHandlerBridge
              ├── tasks/get     → TasksGetHandler
              ├── tasks/list    → TasksListHandler
              ├── tasks/cancel  → TasksCancelHandler
              └── tasks/result  → TasksResultHandler
```

### Lambda Execution Timeline

```
Cold Start:
  main() → LambdaMcpServerBuilder::build() → LambdaMcpServer::handler()
                                                  │
                                                  ├── recover_stuck_tasks()
                                                  └── return LambdaMcpHandler

Request (task-augmented tools/call):
  service_fn(handler.clone()) → handle(req)
       │
       ├── Parse tools/call with params.task
       ├── Create TaskRecord in storage (status: Working)
       ├── tokio::spawn(tool work + auto-complete)
       ├── Return CreateTaskResult immediately  ◄─── Response sent here
       │
       └── [Background] tool.call() executes...
              ├── On success: store_task_result + update_status(Completed)
              └── On Lambda freeze: task stays Working until recovery

Subsequent Request (tasks/result):
  service_fn(handler.clone()) → handle(req)
       │
       ├── Parse tasks/result { taskId }
       ├── Check storage: task status?
       │     ├── Terminal → return stored result immediately
       │     └── Non-terminal → poll storage every 500ms (up to 5 min)
       └── Return result or timeout error
```

## Alternatives Considered

### Lambda-specific TaskExecutor

Create a `LambdaTaskExecutor` that wraps Step Functions or SQS dispatch.
Rejected for Phase A -- adds complexity without clear benefit when
`TokioTaskExecutor` handles the within-invocation case correctly. The
external lifecycle pattern (Phase B) is a better abstraction for
cross-invocation orchestration.

### Disable tasks in Lambda entirely

Reject task-augmented requests in Lambda with an error. Rejected because
short-lived tools work correctly within a single invocation, and the
infrastructure for cross-invocation polling already exists in
`TasksResultHandler`.

### Eager tool execution (block until done, return result)

For short-lived tools, execute inline and return `CallToolResult` even when
`params.task` is present. Rejected because this violates the MCP spec --
task-augmented requests MUST return `CreateTaskResult`, not `CallToolResult`.
Clients opt into the task protocol and expect async semantics.

## See Also

- [ADR-017: Task Runtime-Executor Boundary](./017-task-runtime-executor-boundary.md) -- three-layer architecture that Lambda reuses
- [ADR-016: Task Storage Architecture](./016-task-storage-architecture.md) -- DynamoDB backend used for Lambda task persistence
- [ADR-018: Task Pagination Cursor Contract](./018-task-pagination-cursor-contract.md) -- cursor semantics for `tasks/list`
