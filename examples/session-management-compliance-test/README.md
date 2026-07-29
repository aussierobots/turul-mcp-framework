# Session Management Compliance Test

A wire-pinned client that asserts a server's **MCP 2025-11-25 session
management** behaviour. Unlike a demo client, every step checks a specific
requirement and reports pass/fail.

## Which spec lane, and why

**2025-11-25 ONLY by design.** Every request body hardcodes
`"protocolVersion": "2025-11-25"` and the whole subject is the stateful
session contract — `initialize`, the `Mcp-Session-Id` header, and `DELETE`
termination — all of which the 2026-07-28 stateless core **removed**. There
is no 2026 counterpart because there is no 2026 session to test. This is the
session regression client for the `protocol-2025-11-25` opt-in lane.

Because it speaks raw HTTP with `reqwest`, it links **no** framework crates
(see `Cargo.toml`) and so cannot drift with the protocol crates.

## Run

```bash
# Terminal 1 — any 2025-11-25 server; pick a storage backend
cargo run -p client-initialise-server -- --port 52950 --storage-backend inmemory
# …or --storage-backend sqlite --create-tables / postgres / dynamodb

# Terminal 2
RUST_LOG=info cargo run -p session-management-compliance-test -- http://127.0.0.1:52950/mcp
```

The URL argument is optional; it defaults to `http://127.0.0.1:52950/mcp`.

## What it asserts

| Test | Requirement |
|---|---|
| Session ID generation | `initialize` returns an `Mcp-Session-Id` response header |
| Session ID format | Visible-ASCII, globally unique |
| Header requirement | Non-initialize requests without the header are rejected |
| Nonexistent session | Unknown session id → **404** (client must re-`initialize`) |
| DELETE termination | `DELETE` succeeds; the deleted session then returns **404** |
| Session isolation | Two sessions get distinct ids and do not share state |
| Client DELETE handling | Server cleans up on explicit `DELETE` |

Verified green against `client-initialise-server` on the in-memory backend.

For automatic client-side DROP → DELETE (the client library releasing a
session when it goes out of scope), run the companion:

```bash
cargo run -p turul-mcp-client --example test-client-drop -- http://127.0.0.1:52950/mcp
```

## Related

| Want | Use |
|---|---|
| See the raw 2025 wire, with SSE and progress | [`streamable-http-client-2025-11-25`](../streamable-http-client-2025-11-25/) |
| The 2026 stateless client | [`streamable-http-client`](../streamable-http-client/) |
| Session state in tools | [`stateful-server`](../stateful-server/) |
