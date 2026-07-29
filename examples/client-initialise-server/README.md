# client-initialise-server (MCP 2025-11-25)

A session-enabled MCP server used to exercise the 2025-11-25 stateful
lifecycle, and to prove *where session state actually lives* across four
storage backends.

## Spec lane and why

**Pinned to 2025-11-25** (`default-features = false, features = [...,
"protocol-2025-11-25"]` on every framework dep). Everything it exists to show
— `initialize`, `notifications/initialized`, the server-issued
`Mcp-Session-Id` header, `DELETE` termination — was removed by the 2026-07-28
stateless core. On the 2026 lane there is no session to inspect, so this
example cannot be ported; it stays as the previous spec's regression fixture.

## Run

```bash
cargo run -p client-initialise-server                              # InMemory, port 52950
cargo run -p client-initialise-server -- --port 52950 --storage-backend sqlite --create-tables
cargo run -p client-initialise-server -- --storage-backend postgres
cargo run -p client-initialise-server -- --storage-backend dynamodb --create-tables
```

Backends are cargo features (`sqlite`, `postgres`, `dynamodb`, all on by
default); asking for one that was not compiled in is a startup error rather
than a silent fallback.

The default port is **52950**, not the framework's usual 8641 — `minimal-server`
and `zero-config-getting-started` both bind 8641, and `bilingual-fleet-client`
needs this server and `minimal-server` up at the same time.

## Tools

| Tool | Purpose |
|---|---|
| `echo_sse` | Echoes text, and emits `notifications/progress` + `notifications/message` through `SessionContext` so an SSE stream has something to carry |
| `get_session_data` | Reads the session back **out of the storage backend** — id, capabilities, `is_initialized`, timestamps, state |
| `get_session_events` | Reads the stored SSE events for the session (the resumability record) |
| `get_table_info` | Names the tables/keys the selected backend uses |

The `data_source.backend_type` these tools report is the backend selected at
startup by `--storage-backend`, not a `cfg!(feature = ...)` guess — this
package compiles all four backends, so a compile-time check would name the
same one no matter what is running.

## What to expect

```bash
cargo run -p client-initialise-report -- --url http://127.0.0.1:52950/mcp
```

drives the whole lifecycle and prints a compliance report ending in
`FULLY MCP COMPLIANT`, including a live read-back:

```text
   • Session ID in storage: 019fad2cd44772618cb7e48303bb095c
   • Initialization status: true
      • Backend Type: InMemory
      • Session Table: in_memory
   • Total events stored: 4
```

## Related

- `client-initialise-report` — the raw-wire probe that drives this server
- `streamable-http-client-2025-11-25` — the same wire, focused on SSE framing
- `bilingual-fleet-client` — this server and a 2026 server in one sweep
