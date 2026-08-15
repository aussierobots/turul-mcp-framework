# tasks-e2e-inmemory-client (MCP 2025-11-25)

PASS/FAIL driver for the 2025-11-25 core task lifecycle against
`tasks-e2e-inmemory-server`.

## Spec lane and why

**Task types pinned to 2025-11-25.** The `turul-mcp-client` dependency is the
bilingual default (it negotiates per connection), but `turul-mcp-protocol` is
pinned to `protocol-2025-11-25` because `TaskStatus` and the `tasks/*` request
shapes this client uses are the core-spec types that 2026-07-28 moved into the
Tasks extension. For the 2026 client surface
(`call_tool_or_task` / `task_wait` / `task_update`) see the `ext-tasks-client`
binary in `ext-tasks-server`.

## Run

```bash
cargo run -p tasks-e2e-inmemory-server -- --port 53110      # terminal 1
cargo run -p tasks-e2e-inmemory-client -- --url http://127.0.0.1:53110/mcp
```

## What it asserts

| Check | Method |
|---|---|
| A task-augmented call returns a task handle rather than a result | `tools/call` with task augmentation |
| The task reaches a terminal status | `tasks/get` |
| The stored result is the one the sync call would have produced | `tasks/result` |
| The task is enumerable | `tasks/list` |
| A running task can be cancelled | `tasks/cancel` |
| The same tool still answers synchronously | plain `tools/call` |

## What to expect

```text
PASS: call_tool_with_task returns TaskCreated
PASS: tasks/get shows Completed status
PASS: tasks/result returns correct sum (5 + 3 = 8)
PASS: tasks/list returns 1 task(s)
PASS: tasks/cancel transitions to Cancelled
PASS: synchronous tools/call works without task augmentation

E2E task lifecycle tests complete.
```

Non-zero FAIL lines are printed, not thrown — read the output, don't just
check the exit status.

## Related

- `tasks-e2e-inmemory-server` — the server half
- `ext-tasks-server` — the 2026-07-28 Tasks extension replacement
