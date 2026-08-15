# Streamable HTTP Client — MCP 2025-11-25, raw wire

Speaks the 2025-11-25 Streamable HTTP wire directly with `reqwest`. No
`turul-mcp-client`, no protocol crate — every header and envelope the spec
requires is visible in this example's own source. It is the reference for
anyone debugging their own 2025-11-25 client.

## Which spec lane, and why

**Deliberately pinned to MCP 2025-11-25.** Everything it demonstrates — the
`initialize` handshake, the `Mcp-Session-Id` header carried on every
subsequent request, and `DELETE` session termination — was **removed** by the
2026-07-28 stateless core. This example cannot be ported to 2026; the 2026
equivalent is a different program, [`streamable-http-client`](../streamable-http-client/),
which uses the high-level bilingual client.

## Run

```bash
# Terminal 1 — a 2025-11-25 server exposing the `echo_sse` tool
cargo run -p client-initialise-server -- --port 52950

# Terminal 2
cargo run -p streamable-http-client-2025-11-25 -- --url http://127.0.0.1:52950/mcp
```

Flags: `--tool` (default `echo_sse`), `--args` (JSON object, default
`{"text": "Hello from Streamable HTTP!"}`), `--verbose`.

## What it demonstrates

1. **Session lifecycle** — `initialize` → read `Mcp-Session-Id` from the
   **response header** → `notifications/initialized` (expects HTTP 202) →
   header on every later request → `DELETE` to terminate.
2. **Accept negotiation** — `Accept: application/json, text/event-stream`
   lets the server choose. A tool that emits notifications answers
   `Content-Type: text/event-stream`; one that does not answers JSON. Both are
   spec-legal and the client handles each.
3. **Progress opt-in** — `params._meta.progressToken`. Without it the server
   has no token to echo and emits no progress at all.
4. **Concurrent SSE processing** — one task parses SSE frames while a second
   drains progress notifications, so updates are observed as they arrive
   rather than after the final result.

## Verified run

```
🔑 Mcp-Session-Id: 019fad2f2e077050b13230d1036e142f
✅ notifications/initialized accepted (202) — session enabled
🤝 Server negotiated protocolVersion: 2025-11-25
📥 HTTP 200 OK • Content-Type: text/event-stream
📈 progress: ProgressUpdate { progress: Some(50.0), ... }
📈 progress: ProgressUpdate { progress: Some(100.0), ... }
📡 SSE stream ended after 5 events
👋 DELETE session … → HTTP 200 OK
```

## It checks the server, not just itself

The client sends `progressToken: "streamable-demo-1"` and asserts the server
echoes that exact token back on every `notifications/progress`, per the spec.
Against `client-initialise-server` it currently reports:

```
⚠️  Server did NOT echo progressToken 'streamable-demo-1' — saw ["echo_processing", "echo_processing"]
```

That is a real defect in that server's tool, which calls
`SessionContext::notify_progress` with an invented token instead of the one
the request supplied. The warning is the example doing its job — do not
"fix" it by removing the check.
