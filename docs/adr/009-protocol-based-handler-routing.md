# ADR-009: Protocol-Based Handler Routing

**Status**: Accepted

**Date**: 2025-09-25

## Context

The turul-mcp-framework HTTP server needs to support multiple MCP protocol versions with different transport mechanisms:

- **MCP 2024-11-05 and earlier**: Traditional HTTP+SSE with session-based event storage and replay
- **MCP 2025-06-18**: Streamable HTTP transport with chunked Transfer-Encoding and direct JSON-RPC frame streaming

The challenge was implementing a single HTTP server that can route requests to the appropriate handler based on the `MCP-Protocol-Version` header while maintaining backward compatibility and optimal performance for each protocol variant.

## Decision

Implement **protocol-based handler routing** in the HTTP server with two specialized handlers:

### 1. SessionMcpHandler (Legacy Protocols)
- Handles MCP 2024-11-05 and earlier versions
- Uses session storage for request/response persistence
- Implements traditional SSE event replay via `Last-Event-ID`
- Maintains full backward compatibility

### 2. StreamableHttpHandler (MCP 2025-06-18)
- Handles MCP 2025-06-18 Streamable HTTP transport
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
    .unwrap_or("2025-06-18"); // Default to latest

let protocol_version = McpProtocolVersion::parse_version(protocol_version_str)
    .unwrap_or(McpProtocolVersion::V2025_06_18);

// Route based on protocol capabilities
if protocol_version.supports_streamable_http() {
    // Use StreamableHttpHandler for MCP 2025-06-18 clients
    handler.streamable_handler.handle_request(req).await
} else {
    // Use SessionMcpHandler for legacy clients
    handler.session_handler.handle_mcp_request(req).await
}
```

## Consequences

### Positive
- **Protocol Compliance**: Each handler optimized for its specific MCP version requirements
- **Performance**: MCP 2025-06-18 clients get direct streaming without session storage overhead
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
    streamable_handler: StreamableHttpHandler, // For MCP 2025-06-18
}
```

#### Protocol Version Detection
```rust
pub enum McpProtocolVersion {
    V2024_11_05,
    V2025_06_18,
}

impl McpProtocolVersion {
    pub fn supports_streamable_http(&self) -> bool {
        matches!(self, McpProtocolVersion::V2025_06_18)
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

- [MCP 2025-06-18 Specification](https://spec.modelcontextprotocol.io/specification/transports/http/)
- [ADR-005: MCP Message Notifications Architecture](./005-mcp-message-notifications-architecture.md)
- [CLAUDE.md: HTTP Transport Routing](../../CLAUDE.md#http-transport-routing)