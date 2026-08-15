# roots-server (MCP 2025-11-25)

A minimal server that publishes five root directories over `roots/list`. It
is the fixture the `mcp-roots-tests` E2E suite drives.

## Spec lane and why

**Pinned to 2025-11-25** (`default-features = false`, `protocol-2025-11-25`
on every framework dep). **Roots is deprecated in MCP 2026-07-28** (SEP-2577,
12-month window) and is not served on this branch's 2026 default lane. That
is the entire reason for the pin — build it on the default lane and there is
no `roots/list` to answer.

## What it actually does

It registers five `Root` values and serves `roots/list`. That is all.

```rust
McpServer::builder()
    .root(Root::new("file:///workspace").with_name("Project Workspace"))
    .root(Root::new("file:///data").with_name("Data Storage"))
    .root(Root::new("file:///tmp").with_name("Temporary Files"))
    .root(Root::new("file:///config").with_name("Configuration Files"))
    .root(Root::new("file:///logs").with_name("Log Files"))
```

There are **no tools**, no path validation, no permission enforcement and no
file access of any kind. Roots are a declaration of intended boundaries that
a *client* is expected to respect; publishing them is not enforcement. Any
access control has to live in whatever actually touches the filesystem.

## Run

```bash
cargo run -p roots-server -- --port 8050     # `--port 0` picks an ephemeral port
```

## What to expect

2025-11-25 is stateful, so `roots/list` needs a session — handshake first:

```bash
# 1. initialize; capture the Mcp-Session-Id RESPONSE header
curl -i -X POST http://127.0.0.1:8050/mcp \
  -H 'Content-Type: application/json' -H 'Accept: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{
        "protocolVersion":"2025-11-25","capabilities":{},
        "clientInfo":{"name":"probe","version":"1.0"}}}'

# 2. acknowledge (strict lifecycle mode rejects everything else until this)
curl -X POST http://127.0.0.1:8050/mcp \
  -H 'Content-Type: application/json' -H 'Accept: application/json' \
  -H "Mcp-Session-Id: $SID" \
  -d '{"jsonrpc":"2.0","method":"notifications/initialized"}'

# 3. list
curl -X POST http://127.0.0.1:8050/mcp \
  -H 'Content-Type: application/json' -H 'Accept: application/json' \
  -H "Mcp-Session-Id: $SID" \
  -d '{"jsonrpc":"2.0","id":2,"method":"roots/list","params":{}}'
```

Verified response — roots come back sorted by URI, with a `_meta` count:

```json
{"jsonrpc":"2.0","id":2,"result":{
  "_meta":{"hasMore":false,"total":5},
  "roots":[
    {"name":"Configuration Files","uri":"file:///config"},
    {"name":"Data Storage","uri":"file:///data"},
    {"name":"Log Files","uri":"file:///logs"},
    {"name":"Temporary Files","uri":"file:///tmp"},
    {"name":"Project Workspace","uri":"file:///workspace"}]}}
```

Skipping step 2 gets you `-32031 Session not initialized` — the strict
lifecycle gate, not a roots problem.

## Tests

```bash
cargo test -p mcp-roots-tests
```

Those suites launch this binary through `TestServerManager::start_roots_server()`.
