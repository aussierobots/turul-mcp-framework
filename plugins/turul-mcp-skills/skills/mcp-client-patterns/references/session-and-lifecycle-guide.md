# Connection Negotiation and Lifecycle Guide

Deep-dive reference for MCP client connection negotiation and lifecycle. `turul-mcp-client` is bilingual by default (`client-bilingual` feature): it negotiates the wire spec **per connection**, at `connect()` time, rather than being built for one spec. `--no-default-features --features client-2025-11-25-only` / `client-2026-07-28-only` narrow a build to exactly one lane.

## Connection States

The `SessionState` enum tracks the client's local connection lifecycle — it exists on both spec lanes, even though 2026-07-28 has no server-side session:

```
Uninitialized ──→ Initializing ──→ Active ──→ Terminated
                      │                │
                      └── Error(msg) ◄─┘──→ Reconnecting ──→ Active
```

| State | Meaning |
|---|---|
| `Uninitialized` | Fresh client, not yet connected |
| `Initializing` | `connect()` called, negotiation in progress |
| `Active` | Negotiation complete, ready for operations |
| `Reconnecting` | Connection lost, attempting recovery |
| `Terminated` | `disconnect()` called or fatal error |
| `Error(String)` | Non-fatal error with description |

## Connection Lifecycle

### connect()

`client.connect().await?` performs these steps:

1. **Transport connect** — `transport.connect()` marks the transport as logically ready (no network I/O); first real network validation occurs at step 2
2. **Negotiate the wire spec** (`negotiate_protocol()`):
   - If `ClientConfig::mcp_protocol_version` names an explicit version, skip probing and use it directly.
   - Otherwise, probe with `server/discover` (a 2026-07-28-shaped request, `_meta` advertising `2026-07-28`). A server that answers is treated as 2026-07-28 — no further handshake; the client marks the session `Active` immediately (there is nothing else to negotiate on a stateless core).
   - A server that rejects the probe (JSON-RPC method-not-found, or an HTTP status indicating an older server) falls back to the 2025-11-25 path: the client restores the `MCP-Protocol-Version` header to `2025-11-25` and runs the legacy `initialize` → capture `Mcp-Session-Id` → `notifications/initialized` handshake.
   - `client-2025-11-25-only` / `client-2026-07-28-only` builds skip the probe and run only their one path (the narrowed build errors out if the server doesn't speak that exact lane).
3. **Mark active** — session state transitions to `Active` once negotiation succeeds

After `connect()` returns `Ok(())`, the client is ready for tool calls, resource reads, and prompt operations — the call surface (`list_tools`, `call_tool`, etc.) is identical regardless of which spec was negotiated.

**There is no `initialize` step on a 2026-07-28 connection.** The framework injects `MCP-Protocol-Version`, `Mcp-Method`, and `Mcp-Name` headers automatically on every request via the transport (`apply_request_metadata_headers`); you never construct these by hand.

### disconnect()

`client.disconnect().await?` performs:

1. **Send DELETE** — only if a session ID was captured (2025-11-25 fallback connections); no wire effect on a 2026-07-28 stateless connection, since there is no session to clean up
2. **Transport disconnect** — tears down the underlying connection (event listener task, connection pool)
3. **Mark terminated** — session state transitions to `Terminated`

**Note:** `McpClient` implements `Drop`, which spawns a best-effort background cleanup task. Always prefer explicit `disconnect()` for reliable cleanup. `disconnect()` is idempotent — safe to call multiple times, and the implicit `Drop` cleanup becomes a no-op after an explicit call.

### is_ready()

`client.is_ready().await` returns `true` only when:
- Transport reports connected (`is_connected() == true`)
- Session state is `Active`

Use this to verify the client is operational before making calls.

### connection_status()

`client.connection_status().await` returns a `ConnectionStatus` struct:

```rust
ConnectionStatus {
    transport_connected: bool,
    session_state: SessionState,
    transport_type: TransportType,
    endpoint: String,
    session_id: Option<String>,      // None on a negotiated 2026-07-28 connection
    protocol_version: Option<String>, // "2026-07-28" or "2025-11-25"
}
```

Useful for diagnostics and health checks. `status.summary()` returns a human-readable string.

## negotiated_version()

`client.negotiated_version().await` returns `Option<McpVersion>` — `None` before `connect()`, then whichever version negotiation settled on. Use this when application code needs to branch on which lane a connection landed on (e.g. deciding whether `subscriptions/listen` is available).

## 404 Recovery (2025-11-25 fallback connections only)

A negotiated 2025-11-25 connection can hit HTTP 404 mid-session if the server's session storage evicted it. `McpClient` recovers automatically: `McpClientError::is_session_expired()` (true on HTTP 404) triggers `session.reset()`, clears the stale `Mcp-Session-Id` from the transport, re-runs `initialize`, and retries the original request once. This entire path is unreachable on a 2026-07-28 connection — there is no session to expire.

## Capability Negotiation

- **2025-11-25 fallback**: client capabilities are sent in the `initialize` request and the server's `ServerCapabilities` are captured from the `initialize` response, validated via `validate_server_capabilities()`.
- **2026-07-28**: capabilities travel in `_meta` on every request rather than being negotiated once at handshake time; the `server/discover` probe response carries the server's declared capabilities and `instructions`, retained on the client for priming an agent's system prompt.

Access post-connect (2025-11-25 fallback):

```rust
let info = client.session_info().await;
if let Some(caps) = &info.server_capabilities {
    if caps.tools.is_some() {
        println!("Server supports tools");
    }
}
```

## Monitoring

Use `client.transport_stats().await` for operational metrics:

```rust
TransportStatistics {
    requests_sent: u64,
    responses_received: u64,
    notifications_sent: u64,
    events_received: u64,
    errors: u64,
    avg_response_time_ms: f64,
    last_error: Option<String>,
}
```
