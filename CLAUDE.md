# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is the **mcp-framework** - a standalone, production-ready Rust framework for building Model Context Protocol (MCP) servers. This framework is designed to eventually supersede previous MCP implementations with a clean, modular architecture.

### Key Features
- **Complete MCP 2025-06-18 Specification Support**: Full protocol compliance with latest features
- **Streamable HTTP Transport**: Integrated SSE support for real-time notifications
- **Session Management**: UUID v7-based sessions with automatic cleanup
- **Rich Trait System**: Comprehensive trait coverage for all MCP operations
- **Derive Macros**: Automatic tool generation with schema validation
- **Multi-Transport Support**: HTTP, WebSocket, and future transport layers

## Architecture

### Core Crates Structure
```
mcp-framework/
├── crates/
│   ├── mcp-protocol-2025-06-18/  # Protocol types and traits
│   ├── mcp-server/               # High-level server framework
│   ├── http-mcp-server/         # HTTP transport layer
│   ├── json-rpc-server/         # JSON-RPC dispatch
│   └── mcp-derive/              # Procedural macros
└── examples/                    # Example servers
```

### Session Management
- **UUID Version**: Always use UUID v7 (`Uuid::now_v7()`) for session IDs - provides temporal ordering and better performance
- **Session Cleanup**: Automatic cleanup every 60 seconds, 30-minute expiry
- **SSE Integration**: Sessions provide broadcast channels for real-time notifications

### MCP Protocol Version Support
- **V2024_11_05**: Basic MCP without streamable HTTP
- **V2025_03_26**: Streamable HTTP support 
- **V2025_06_18**: Full feature set with _meta, cursor, progressToken, elicitation

### MCP TypeScript Specification Compliance
**CRITICAL**: All types in `mcp-protocol-2025-06-18` crate MUST exactly match the MCP TypeScript Schema specification. This includes:
- **Request Pattern**: Every MCP request type must follow `XxxRequest { method, params: XxxParams }` pattern
- **Params Pattern**: Every params type includes method-specific fields PLUS optional `_meta` field  
- **Response Pattern**: Every response type includes method-specific fields PLUS optional top-level `_meta` field
- **Field Naming**: All fields must use exact camelCase names from TypeScript schema
- **Optional Fields**: Use `Option<T>` with proper `skip_serializing_if` attributes for optional TypeScript fields
- **Inheritance**: Rust structs must replicate TypeScript interface inheritance via composition
- **Trait Implementation**: ALL request/response types MUST implement corresponding traits from `traits.rs` for compile-time specification compliance

**Example Pattern**:
```rust
// Matches: export interface CallToolRequest extends Request
pub struct CallToolRequest {
    pub method: String,  // "tools/call" from Request.method
    pub params: CallToolParams,  // from Request.params
}

// Matches: params: { name: string; arguments?: {...}; _meta?: {...} }
pub struct CallToolParams {
    pub name: String,
    pub arguments: Option<HashMap<String, Value>>,
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

// MUST implement traits for compile-time compliance
impl Params for CallToolParams {}
impl HasCallToolParams for CallToolParams { /* trait methods */ }
impl CallToolRequest for CallToolRequest { /* trait methods */ }
```

## Build Commands

### Standard Development
```bash
# Build entire workspace
cargo build

# Check compilation
cargo check

# Run tests
cargo test

# Format and lint
cargo fmt
cargo clippy

# Run specific example
cargo run --example minimal-server
```

### HTTP Transport Testing
```bash
# Test with curl (initialize)
curl -X POST http://127.0.0.1:8000/mcp \
  -H "Content-Type: application/json" \
  -H "MCP-Protocol-Version: 2025-06-18" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'

# Test SSE connection (requires session ID from initialize)  
curl -N -H "Accept: text/event-stream" \
  -H "Mcp-Session-Id: <session-id>" \
  http://127.0.0.1:8000/mcp
```

## MCP TypeScript Specification Compliance

The MCP framework now fully implements the MCP TypeScript specification (2025-06-18) with comprehensive trait-based validation and compile-time compliance checking.

### Request/Params Pattern
All MCP requests follow the TypeScript schema pattern:
```rust
// TypeScript: { method: string, params: { ...fields, _meta?: {...} } }
pub struct CallToolRequest {
    pub method: String,  // "tools/call"
    pub params: CallToolParams,
}

pub struct CallToolParams {
    pub name: String,
    pub arguments: Option<Value>,
    #[serde(rename = "_meta")]
    pub meta: Option<HashMap<String, Value>>,
}
```

### Notification Pattern
Notifications follow the TypeScript pattern:
```rust
// TypeScript: { method: string, params?: { _meta?: {...}, [key: string]: unknown } }
pub struct ResourcesListChangedNotification {
    pub method: String,  // "notifications/resources/listChanged"
    pub params: Option<NotificationParams>,
}
```

### Trait-Based Validation
All types implement corresponding traits for compile-time specification compliance:
- `HasMethod`, `HasParams`, `HasMetaParam` for requests
- `HasData`, `HasMeta` for responses  
- `JsonRpcRequestTrait`, `JsonRpcNotificationTrait`, `JsonRpcResponseTrait` for JSON-RPC

### Testing Compliance
Run the MCP TypeScript specification compliance tests:
```bash
cargo test --package mcp-protocol-2025-06-18 compliance_test::tests
```

## Key Implementation Guidelines

### UUID Usage
- **ALWAYS use UUID v7**: `Uuid::now_v7()` for all session IDs and temporal identifiers
- **Never use UUID v4**: Provides no temporal ordering benefits

### Error Handling  
- Use `JsonRpcError` framework types instead of custom errors
- Proper MCP error codes per specification
- Structured error responses with detailed context

### Session Architecture
- Single HTTP endpoint `/mcp` with method-based routing
- GET + `Accept: text/event-stream` + `Mcp-Session-Id` = SSE connection
- POST = JSON-RPC requests
- DELETE = Session cleanup
- OPTIONS = CORS preflight

### Protocol Version Detection
- Extract `MCP-Protocol-Version` header
- Feature flags based on protocol capabilities
- Graceful fallback to latest supported version

## Testing with MCP Inspector
The framework must return structured JSON data, not generic "Tool Result: Success" messages:

```bash
# Good: Returns actual JSON data structure
{"result": {"value": 42, "message": "calculated"}}

# Bad: Generic success message  
"Tool Result: Success"
```

## Recent Architecture Changes
- **Removed separate SSE manager**: Integrated directly into session management
- **Protocol version detection**: Automatic feature flag selection
- **Streamable HTTP focus**: SSE as integral part, not separate transport
- **Trait system overhaul**: Rich functional traits vs empty marker traits
- **UUID v7 adoption**: All session IDs use temporal UUIDs for better ordering

## Development Standards
- **No GPS project references**: This is a standalone framework
- **Clean separation**: Each crate has focused responsibilities  
- **Future-proof design**: Architecture supports additional transports
- **Production ready**: Performance, security, and reliability first

## Testing Strategy
- Unit tests for all core functionality
- Integration tests with MCP Inspector
- Protocol compliance testing across all supported versions
- Performance benchmarks for session management and SSE streaming