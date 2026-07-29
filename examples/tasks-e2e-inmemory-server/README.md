# tasks-e2e-inmemory-server (MCP 2025-11-25)

Server for the 2025-11-25 **core** task lifecycle: `tasks/get`,
`tasks/result`, `tasks/list`, `tasks/cancel`, backed by
`InMemoryTaskStorage`.

## Spec lane and why

**Pinned to 2025-11-25.** 2026-07-28 moved Tasks out of the core into the
`io.modelcontextprotocol/tasks` extension (SEP-2663) with an incompatible,
stateless lifecycle — no `tasks/list`, no blocking `tasks/result`, plus a new
`tasks/update` for mid-task input. This example is the record of the API the
extension replaced; the 2026 version is `ext-tasks-server`.

## Run

```bash
cargo run -p tasks-e2e-inmemory-server -- --port 53110
cargo run -p tasks-e2e-inmemory-client -- --url http://127.0.0.1:53110/mcp
```

The default port is 8080, which `oauth-resource-server` also defaults to —
pass `--port` when running more than one example.

## Tools

| Tool | `task_support` | Purpose |
|---|---|---|
| `slow_add` | `optional` | Sleeps (default 2 s) then adds — long enough to observe `Working` before `Completed` |
| `slow_cancelable` | `optional` | Sleeps in cancellation-aware steps so `tasks/cancel` can land mid-flight |

`task_support = "optional"` means the same tool answers synchronously to a
client that does not ask for task augmentation.

## What to expect

```text
PASS: call_tool_with_task returns TaskCreated
PASS: tasks/get shows Completed status
PASS: tasks/result returns correct sum (5 + 3 = 8)
PASS: tasks/list returns 1 task(s)
PASS: tasks/cancel transitions to Cancelled
PASS: synchronous tools/call works without task augmentation
```

## Related

- `tasks-e2e-inmemory-client` — the PASS/FAIL driver
- `ext-tasks-server` — the 2026-07-28 Tasks extension that supersedes this
