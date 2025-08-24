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