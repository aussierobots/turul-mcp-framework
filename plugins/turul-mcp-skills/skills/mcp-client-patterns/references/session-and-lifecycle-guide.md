# Session and Lifecycle Guide

Deep-dive reference for MCP client session management and connection lifecycle.

## Session States

The `SessionState` enum tracks the client's lifecycle:

```
Uninitialized ──→ Initializing ──→ Active ──→ Terminated
                      │                │
                      └── Error(msg) ◄─┘──→ Reconnecting ──→ Active
```

| State | Meaning |
|---|---|
| `Uninitialized` | Fresh client, not yet connected |
| `Initializing` | `connect()` called, performing MCP handshake |
| `Active` | Handshake complete, ready for operations |
| `Reconnecting` | Connection lost, attempting recovery |
| `Terminated` | `disconnect()` called or fatal error |
| `Error(String)` | Non-fatal error with description |

## Connection Lifecycle

### connect()

`client.connect().await?` performs these steps:

1. **Transport connect** — `transport.connect()` establishes the underlying connection
2. **Mark initializing** — session state transitions to `Initializing`
3. **Send `initialize`** — sends the MCP `initialize` request with client capabilities and info
4. **Capture server response** — stores `ServerCapabilities`, `protocolVersion`, and `Mcp-Session-Id`
5. **Send `notifications/initialized`** — tells the server the client is ready
6. **Mark active** — session state transitions to `Active`

After `connect()` returns `Ok(())`, the client is ready for tool calls, resource reads, and prompt operations.

### disconnect()

`client.disconnect().await?` performs:

1. **Send DELETE** — sends HTTP DELETE with `Mcp-Session-Id` to clean up the server session
2. **Transport disconnect** — tears down the underlying connection
3. **Mark terminated** — session state transitions to `Terminated`

**Note:** `McpClient` implements `Drop`, which spawns a best-effort background cleanup task. Always prefer explicit `disconnect()` for reliable cleanup.

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
    session_id: Option<String>,
    protocol_version: Option<String>,
}
```

Useful for diagnostics and health checks. `status.summary()` returns a human-readable string.

## SessionManager

The `SessionManager` is an internal component (not directly accessed by users) that:

- Manages `SessionInfo` behind an `Arc<RwLock<...>>`
- Creates the `initialize` request with client capabilities
- Validates server capabilities during handshake
- Tracks session metadata: `session_id`, `created_at`, `last_activity`, `connection_attempts`

### SessionInfo

```rust
SessionInfo {
    session_id: Option<String>,
    state: SessionState,
    client_capabilities: Option<ClientCapabilities>,
    server_capabilities: Option<ServerCapabilities>,
    protocol_version: Option<String>,
    created_at: Instant,
    last_activity: Instant,
    connection_attempts: u32,
    metadata: Value,
}
```

Access via `client.session_info().await`.

**Useful methods:**
- `info.is_active()` — state is `Active`
- `info.is_ready()` — state is `Active` (same as `is_active`)
- `info.duration()` — time since session creation
- `info.idle_time()` — time since last activity
- `info.needs_initialization()` — state is `Uninitialized`

## Capability Negotiation

During `connect()`, the client advertises its capabilities and receives the server's:

- **Client capabilities**: Created via `SessionManager::create_client_capabilities()`, includes roots and sampling support
- **Server capabilities**: Received in the `initialize` response, validated via `validate_server_capabilities()`
- **Protocol version**: Negotiated during handshake, stored in `SessionInfo.protocol_version`

Access server capabilities post-connect:

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
