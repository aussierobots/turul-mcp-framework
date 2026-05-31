# ADR-009: Protocol-Based Handler Routing

**Status**: Accepted

**Date**: 2025-09-25

## Context

The turul-mcp-framework HTTP server needs to support multiple MCP protocol versions with different transport mechanisms:

- **MCP 2024-11-05 and earlier**: Traditional HTTP+SSE with session-based event storage and replay
- **MCP 2025-03-26+**: Streamable HTTP transport with chunked Transfer-Encoding and direct JSON-RPC frame streaming (current default: 2025-11-25)

The challenge was implementing a single HTTP server that can route requests to the appropriate handler based on the `MCP-Protocol-Version` header while maintaining backward compatibility and optimal performance for each protocol variant.

## Decision

Implement **protocol-based handler routing** in the HTTP server with two specialized handlers:

### 1. SessionMcpHandler (Legacy Protocols)
- Handles MCP 2024-11-05 and earlier versions
- Uses session storage for request/response persistence
- Implements traditional SSE event replay via `Last-Event-ID`
- Maintains full backward compatibility

### 2. StreamableHttpHandler (MCP 2025-03-26+)
- Handles MCP 2025-03-26 and later Streamable HTTP transport (including 2025-06-18 and 2025-11-25)
- Uses `Transfer-Encoding: chunked` for progressive responses
- Streams JSON-RPC frames directly (Progress, PartialResult, FinalResult)
- Bypasses session storage for tool call responses (performance optimization)
- Still uses session storage for session metadata and SSE notifications

### Routing Logic
```rust
// Extract MCP protocol version from headers
let protocol_version_str = req
    .headers()
    .get("MCP-Protocol-Version")
    .and_then(|h| h.to_str().ok())
    .unwrap_or("2025-11-25"); // Default to latest

let protocol_version = McpProtocolVersion::parse_version(protocol_version_str)
    .unwrap_or(McpProtocolVersion::V2025_11_25);

// Route based on protocol capabilities
if protocol_version.supports_streamable_http() {
    // Use StreamableHttpHandler for MCP 2025-03-26+ clients
    handler.streamable_handler.handle_request(req).await
} else {
    // Use SessionMcpHandler for legacy clients
    handler.session_handler.handle_mcp_request(req).await
}
```

## Consequences

### Positive
- **Protocol Compliance**: Each handler optimized for its specific MCP version requirements
- **Performance**: Streamable HTTP clients (2025-03-26+) get direct streaming without session storage overhead
- **Backward Compatibility**: Legacy clients continue working without changes
- **Clean Architecture**: Separation of concerns between protocol versions
- **Maintainability**: Protocol-specific logic isolated in dedicated handlers

### Negative
- **Code Duplication**: Some common functionality duplicated between handlers
- **Complexity**: Two separate code paths to maintain and test
- **Memory Usage**: Both handlers instantiated even if only one protocol used

### Risks
- **Protocol Detection**: Clients not sending `MCP-Protocol-Version` header default to latest version
- **Feature Parity**: Risk of features being implemented in only one handler
- **Testing Coverage**: Need comprehensive tests for both routing paths

## Implementation

### Key Components

#### McpRequestHandler (Combined Router)
```rust
#[derive(Clone)]
struct McpRequestHandler {
    session_handler: SessionMcpHandler,      // For legacy protocols
    streamable_handler: StreamableHttpHandler, // For MCP 2025-03-26+
}
```

#### Protocol Version Detection
```rust
pub enum McpProtocolVersion {
    V2024_11_05,
    V2025_03_26,
    V2025_06_18,
    V2025_11_25,
}

impl McpProtocolVersion {
    pub fn supports_streamable_http(&self) -> bool {
        !matches!(self, McpProtocolVersion::V2024_11_05)
    }
}
```

#### Handler Registration
Both handlers share the same `JsonRpcDispatcher` instance to ensure consistent method registration and business logic handling.

### Debugging and Testing

Protocol routing can be debugged with explicit logging:
```rust
debug!("MCP request: protocol_version={}, method={}",
       protocol_version.as_str(), method);
println!("ROUTING TO {} HANDLER",
         if streamable { "STREAMABLE" } else { "SESSION" });
```

### Binary Cache Considerations

When testing protocol routing changes, ensure fresh binaries:
```bash
cargo clean -p tools-test-server && cargo build --bin tools-test-server
cargo test --test streamable_http_e2e
```

Stale binaries can mask routing changes and cause test failures.

## See Also

- [MCP 2025-11-25 Specification](https://modelcontextprotocol.io/specification/2025-11-25)
- [ADR-005: MCP Message Notifications Architecture](./005-mcp-message-notifications-architecture.md)
- [CLAUDE.md: HTTP Transport Routing](../../CLAUDE.md#http-transport-routing)
## DRAFT-2026-v1: McpProtocolVersion becomes feature-exclusive

**Status: Added 2026-05-31. Relevant for the 0.4.0 release per ADR-027.**

### The change

In 0.3.x, `McpProtocolVersion` is a single enum that lists every wire version the server can speak (`V2024_11_05`, `V2025_03_26`, `V2025_06_18`, `V2025_11_25`). The HTTP server's request dispatcher reads `MCP-Protocol-Version` from the request header and picks a handler based on what the enum variant supports (`supports_streamable_http()`).

In 0.4.0, **`McpProtocolVersion` becomes feature-exclusive**. The protocol version that any given build of the framework speaks is fixed at compile time by the cargo feature flag:

| Cargo features on consumer crate | `McpProtocolVersion` variant available | `turul-mcp-protocol` alias resolves to |
|---|---|---|
| (default, no features) | `V2026_07_28` (wire string `"DRAFT-2026-v1"`) | `turul-mcp-protocol-2026-07-28` |
| `legacy-2025-11-25` | `V2025_11_25` (wire string `"2025-11-25"`) | `turul-mcp-protocol-2025-11-25` |

**These are mutually exclusive.** A single build of the framework can only host one protocol type hierarchy at a time. Reasons:

1. The handshake state machines differ. 2025-11-25 has `initialize` → `notifications/initialized` → `Mcp-Session-Id`-tagged requests. DRAFT-2026-v1 has `server/discover` + per-request `_meta` carrier. The dispatcher cannot serve both simultaneously without per-request branching to the right schema-validated payload deserializer — which would require carrying both protocol crates in every consumer binary.
2. Types diverge structurally. `RequestParams` in 2026-07-28 has a required `_meta: RequestMetaObject` field; in 2025-11-25 it has an optional `meta: Option<HashMap>` (different name, different shape, different requiredness). The `Tool`, `Resource`, `Prompt`, `ContentBlock`, and `ServerCapabilities` types similarly diverge. Linking both via re-exports under the same module path is not viable.
3. The Cargo dependency tree is the natural cutover boundary. Each consumer crate's `Cargo.toml` selects the protocol crate via `turul-mcp-protocol` (the alias) and the `legacy-2025-11-25` feature; the rest of the consumer's code is unchanged.

### Routing in DRAFT-2026-v1 mode

With the default (DRAFT-2026-v1) feature set, the `MCP-Protocol-Version` header check still happens but resolves to a single handler:

```rust
// Pseudocode for 0.4.0 default (DRAFT-2026-v1) build:
let header_version = req
    .headers()
    .get("MCP-Protocol-Version")
    .and_then(|h| h.to_str().ok())
    .unwrap_or("DRAFT-2026-v1");

match header_version {
    "DRAFT-2026-v1" => handler.streamable_handler.handle_request(req).await,
    "2026-07-28" => /* same as above; alias for the same wire string when final spec lands */ handler.streamable_handler.handle_request(req).await,
    other => /* reject: 426 Upgrade Required or 400 Bad Request; the build doesn't speak that version */,
}
```

**`SessionMcpHandler` is gone in the default build.** Its responsibilities (HTTP+SSE with `Last-Event-ID` replay, session-stored request/response persistence) are concepts that don't exist in DRAFT-2026-v1. The 2024-11-05 legacy path is unreachable from a 2026 build.

### Routing in `legacy-2025-11-25` mode

With `--features legacy-2025-11-25`, the original ADR-009 routing logic applies unchanged. `SessionMcpHandler` for ≤2024-11-05 (still supported because the 2025-11-25 protocol crate carries `V2024_11_05`), `StreamableHttpHandler` for 2025-11-25.

### Cross-version client connectivity

A client that needs to talk to both a 2025-11-25 server and a DRAFT-2026-v1 server cannot do so with one build of the framework. The client must either:

1. **Ship one build per target.** Two binaries, two `Cargo.toml`s, one with `legacy-2025-11-25` and one without.
2. **Use a future bilingual client design.** A planned client-only ADR (separate, TBD-numbered) may carve out cross-version support inside `turul-mcp-client` because the client side does not have the same compile-time-fixed handshake constraints the server has. The server's `McpProtocolVersion` is process-global; the client's is per-connection.

Until that client ADR is decided, treat the protocol version as a per-build constant.

### Migration impact

- Consumers of `McpProtocolVersion::*` enum variants that don't exist in the active feature set get a `not found in scope` compile error. Match arms for `V2024_11_05` in default builds either compile-gate (`#[cfg(feature = "legacy-2025-11-25")]`) or get removed.
- Tests that iterate over all variants need feature-gate awareness. The compliance test in the 2026-07-28 protocol crate iterates the 2026 variants; the equivalent in the 2025-11-25 crate is unchanged.
- HTTP client integration tests that assert against `"2025-11-25"` in default 0.4.0 builds will fail until updated — the default header is `"DRAFT-2026-v1"`.

### Why not runtime selection

A runtime selector (e.g., "if the request header is `DRAFT-2026-v1`, decode as 2026; if `2025-11-25`, decode as 2025") would require both protocol crates linked into every binary, doubling the dependency footprint and forcing every consumer to ship code for a spec they may never use. Cargo feature gating localizes the cost to the consumer's actual deployment target. Per ADR-027 §"Status update (2026-05-31)" #2, this is the intended cutover model for 0.4.0.

### References

- ADR-006 §"DRAFT-2026-v1: Stateless variant; GET SSE is 2025-only" — transport behavior under the new default.
- ADR-027 §"Status update (2026-05-31)" — feature flag mechanics.
- ADR-023 §"DRAFT-2026-v1: per-request fingerprint persistence" — tool change detection in stateless mode.
