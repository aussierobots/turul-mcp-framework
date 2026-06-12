# Tasks Extension (`io.modelcontextprotocol/tasks`, SEP-2663)

Durable poll handles instead of blocking on long-running tool calls — the
2026-07-28 replacement for the 2025 core task lifecycle (no `tasks/list`, no
blocking `tasks/result`; polling via `tasks/get`, mid-task input via the new
`tasks/update`).

```bash
cargo run -p ext-tasks-server                              # server, port 8645
cargo run -p ext-tasks-server --bin ext-tasks-client       # client walkthrough
```

## What the walkthrough shows (live-verified)

```text
→ crunch(7) with the extension declared
← task 45588cc8… (Working, pollIntervalMs 500) — polling…
← completed: "49"

→ deploy(billing-api)
← task 72434fe1… — polling for the approval request…
← input_required: Approve deployment of billing-api?
→ tasks/update: approved = true
← completed: "deployed billing-api ✅"

→ crunch(3) WITHOUT declaring the extension
← synchronous (blocked ~2s): "9"
```

## The lifecycle

1. **Declare** — the client puts `io.modelcontextprotocol/tasks` in every
   request's `_meta` `clientCapabilities.extensions`
   (`declared_capabilities.ext_tasks = true`). The server advertises the same
   in `server/discover`.
2. **Elect** — the server is the sole decider: a task-electing tool answers a
   declared client with `CreateTaskResult` (`resultType: "task"`), durably
   stored before the response. Undeclared clients get the ordinary
   synchronous result — same tool, progressive enhancement.
3. **Poll** — `tasks/get` until terminal, honoring `pollIntervalMs`
   (`McpClient::task_wait` does this).
4. **Mid-task input** — `deploy` returns `McpError::InputRequired` exactly
   like a synchronous MRTR tool; under task election the runtime parks the
   task in `input_required` and `tasks/update` resumes it. **Tool code is
   identical under both execution models.**
5. **Terminal** — `completed` carries the result the sync call would have
   returned; `failed` carries the JSON-RPC error; `tasks/cancel` is
   cooperative.

## Server side

```rust
McpServer::builder()
    .with_ext_tasks(Arc::new(InMemoryTaskStore::new()))  // advertises + registers tasks/*
    .ext_task_tool(CrunchTool::new())                    // elect when declared, sync otherwise
    .ext_task_tool(DeployTool::new())
    // .ext_task_tool_required(...)                      // -32003 when undeclared
```

Requires the `ext-tasks` cargo feature on `turul-mcp-server` (extensions are
off by default per SEP-2133).

## Client side

```rust
let mut config = ClientConfig::default();
config.declared_capabilities.ext_tasks = true;
// ...
match client.call_tool_or_task("crunch", json!({"n": 7})).await? {
    ToolCallOutcome::Task(t) => {
        let done = client.task_wait(&t.task.fields.task_id).await?;
    }
    ToolCallOutcome::Completed(r) => { /* server chose sync */ }
}
```

Requires the `ext-tasks` cargo feature on `turul-mcp-client`.

## See also

- `mrtr-elicitation-server` — the synchronous MRTR round trip the `deploy`
  tool also works under
- `crates/turul-mcp-server/tests/ext_tasks_2026.rs` — the wire contract suite
  (incl. `-32003` for required tools and `taskIds`-filtered
  `notifications/tasks` over `subscriptions/listen`)
- `tasks-e2e-inmemory-server` — the 2025-11-25 core task lifecycle this
  extension replaces (kept on the pinned 2025 lane)
