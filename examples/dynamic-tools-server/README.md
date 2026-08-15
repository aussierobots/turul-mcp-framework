# Dynamic Tools Server

Tools can be activated and deactivated **while the server is running**. The
registry emits `notifications/tools/list_changed` on every change, and the next
`tools/list` reflects the new set.

Everywhere else in `examples/`, the tool set is fixed at
`McpServer::builder()` time. This is the example that isn't.

## How it works

```rust
let server = McpServer::builder()
    .tool_change_mode(ToolChangeMode::Dynamic)   // ← opts into a live registry
    // ...
    .build()?;

let registry = server.tool_registry().expect("Dynamic must have registry");
registry.deactivate_tool("multiply").await?;     // → notifications/tools/list_changed
registry.activate_tool("multiply").await?;
```

`ToolChangeMode::Dynamic` also flips the advertised `tools.listChanged`
capability to `true` — the static default is `false`, and the framework keeps
that truthful rather than always claiming support.

`activate_multiply` and `deactivate_multiply` are ordinary derive-macro tools
that reach the registry through a `OnceLock` set in `main`. They toggle each
other too, so exactly one of the pair is ever listed.

## Spec lane: 2025-11-25 (deliberate)

The manifest pins `protocol-2025-11-25` on every framework dependency, so this
example does **not** build under the workspace 2026-07-28 default. That is
intentional, not lag: it is the fixture behind `tests/dynamic_tools_e2e.rs`,
which asserts the stateful-lane contract — a client re-running `initialize`
gets a session baselined on the *live* registry fingerprint rather than the
startup one. The 2026-07-28 core is stateless (no `initialize`, no
`Mcp-Session-Id`), so that particular assertion has nothing to attach to there.

Consequently the walkthrough below uses the 2025-11-25 handshake:
`initialize` → `notifications/initialized` → `Mcp-Session-Id` on every request.

## Run

```bash
cargo run -p dynamic-tools-server                       # multiply starts active
cargo run -p dynamic-tools-server -- --multiply-inactive
cargo run -p dynamic-tools-server -- --port 9000
# → http://127.0.0.1:8484/mcp
```

## Try it

```bash
# 1. initialize, capture the session id from the response headers
SID=$(curl -s -D - -o /dev/null -X POST http://127.0.0.1:8484/mcp \
  -H 'Content-Type: application/json' -H 'Accept: application/json, text/event-stream' \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"curl","version":"1.0"}}}' \
  | grep -i '^mcp-session-id:' | tr -d '\r' | awk '{print $2}')

# 2. complete the handshake (202)
curl -s -X POST http://127.0.0.1:8484/mcp \
  -H 'Content-Type: application/json' -H 'Accept: application/json, text/event-stream' \
  -H "Mcp-Session-Id: $SID" \
  -d '{"jsonrpc":"2.0","method":"notifications/initialized"}'

call() { curl -s -X POST http://127.0.0.1:8484/mcp \
  -H 'Content-Type: application/json' -H 'Accept: application/json, text/event-stream' \
  -H "Mcp-Session-Id: $SID" -d "$1"; }

# 3. add, deactivate_multiply, greet, multiply
call '{"jsonrpc":"2.0","id":2,"method":"tools/list"}'

# 4. toggle — the response stream carries notifications/tools/list_changed
call '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"deactivate_multiply","arguments":{}}}'

# 5. activate_multiply, add, greet — multiply and deactivate_multiply are gone
call '{"jsonrpc":"2.0","id":4,"method":"tools/list"}'
```

## See also

- `tests/dynamic_tools_e2e.rs` — the E2E contract this example backs
- `examples/calculator-add-builder-server` — building a tool at runtime, but
  registering it once at startup
