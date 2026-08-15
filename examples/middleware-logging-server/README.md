# Middleware Logging Server (request timing)

The smallest useful `McpMiddleware`: stamp a start time in `before_dispatch`,
read it back in `after_dispatch`, log the measured duration and whether the
call succeeded.

## Spec lane

**MCP 2026-07-28** (the workspace default). Timing middleware is
spec-independent — it observes the dispatcher, not the wire contract.

## Run

```bash
cargo run -p middleware-logging-server            # port 8670
cargo run -p middleware-logging-server -- --port 9000
```

This server registers **no tools**; it exists to show the middleware hook, so
`tools/list` returns an empty list. Point any request at it and watch the log.

## Verified output

```bash
META='"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"curl","version":"1.0"},"io.modelcontextprotocol/clientCapabilities":{}}'

curl -s -X POST http://127.0.0.1:8670/mcp \
  -H 'Content-Type: application/json' -H 'Accept: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' -H 'Mcp-Method: server/discover' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"server/discover\",\"params\":{$META}}"
```

```
INFO middleware_logging_server: → server/discover starting
INFO middleware_logging_server: ← server/discover completed in 0.13ms (ok)
```

## The pattern

State that must survive from `before_dispatch` to `after_dispatch` goes on the
request context, not on the middleware struct — the middleware is a single
shared `Arc` handling concurrent requests, so a field would race:

```rust
// before_dispatch
ctx.add_metadata("timing_start_us", json!(start_us));

// after_dispatch — same request's context, threaded through by the dispatcher
let start = ctx.metadata().get("timing_start_us").and_then(|v| v.as_u64());
```

`after_dispatch` also receives `&mut DispatcherResult`, so it can inspect
(`result.is_success()`) or rewrite the outgoing result.
