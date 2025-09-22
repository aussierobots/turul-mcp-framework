# CLAUDE.md

Production-ready Rust framework for Model Context Protocol (MCP) servers with zero-configuration design and complete MCP 2025-06-18 specification support.

## 🚨 Critical Rules

### Import Conventions
```rust
// ✅ BEST - Use preludes
use turul_mcp_server::prelude::*;
use turul_mcp_derive::{McpTool, McpResource, McpPrompt, mcp_tool};

// ❌ WRONG - Versioned imports
use turul_mcp_protocol_2025_06_18::*;  // Use turul_mcp_protocol::* instead
```

### Zero-Configuration Design
Users NEVER specify method strings - framework auto-determines from types:
```rust
// ✅ CORRECT
#[derive(McpTool)]
struct Calculator;  // Framework → tools/call

// ❌ WRONG
#[mcp_tool(method = "tools/call")]  // NO METHOD STRINGS!
```

### API Conventions
- **SessionContext**: Use `get_typed_state(key).await` and `set_typed_state(key, value).await?`
- **Builder Pattern**: `McpServer::builder()` not `McpServerBuilder::new()`
- **Error Handling**: Always use `McpError` types - NEVER create JsonRpcError directly in handlers
- **Session IDs**: Always `Uuid::now_v7()` for temporal ordering

### 🚨 Critical Error Handling Rules (2025-09-22)

**MANDATORY**: Use the new unified error handling architecture. Never implement workarounds.

```rust
// ✅ CORRECT - Handlers return domain errors only
#[async_trait]
impl JsonRpcHandler for MyHandler {
    type Error = McpError;  // Always use McpError

    async fn handle(&self, method: &str, params: Option<RequestParams>, session: Option<SessionContext>)
        -> Result<Value, McpError> {  // Domain errors only

        // Return domain errors - dispatcher converts to JSON-RPC
        Err(McpError::InvalidParameters("Missing required parameter".to_string()))
    }
}

// ❌ WRONG - Never create JsonRpcError in handlers
impl MyHandler {
    async fn handle(&self, ...) -> Result<Value, JsonRpcError> {  // NO!
        Err(JsonRpcError::new(...))  // NEVER DO THIS
    }
}

// ❌ WRONG - Never use JsonRpcProcessingError (removed in 0.2.0)
use turul_mcp_json_rpc_server::error::JsonRpcProcessingError;  // NO! Doesn't exist

// ✅ CORRECT - Dispatcher owns all protocol conversion
JsonRpcDispatcher<McpError>::new()  // Type-safe dispatcher
```

**Key Rules:**
1. Handlers return `Result<Value, McpError>` ONLY
2. Dispatcher converts McpError → JsonRpcError automatically
3. Never create JsonRpcError, JsonRpcResponse in business logic
4. Use `McpError::InvalidParameters`, `McpError::ToolNotFound`, etc.

### JSON-RPC Response Format (JSON-RPC 2.0 Compliance)
The framework uses **separate response types** for success and error cases:

```rust
// ✅ SUCCESS RESPONSE
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    // ... success data
  }
}

// ✅ ERROR RESPONSE
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32603,
    "message": "Error description"
  }
}

// ❌ WRONG - Never wrap errors in result field
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "error": { ... }  // This violates JSON-RPC 2.0
  }
}
```

**Framework Implementation**: Uses `JsonRpcMessage` enum with `JsonRpcResponse` and `JsonRpcError` variants to ensure spec compliance.

### 🔒 URI Security Requirements
**CRITICAL**: Custom URI schemes may be blocked by security middleware. Use standard file:// paths:

```rust
// ✅ SECURE - Use file:// scheme
"file:///memory/data.json"     // Instead of memory://data
"file:///empty/content.txt"    // Instead of cache://items
"file:///session/info.json"    // Instead of session://info
"file:///tmp/test.txt"         // Standard file paths

// ❌ BLOCKED - Custom schemes
"memory://data"                // May fail security checks
"session://info"               // May fail security checks
"custom://resource"            // May fail security checks
```

**Production Rule**: Always use `file://` URIs for maximum compatibility with security middleware.

## Quick Reference

### Tool Creation (4 Levels)
```rust
// Level 1: Function
#[mcp_tool(name = "add")]
async fn add(a: f64, b: f64) -> McpResult<f64> { Ok(a + b) }

// Level 2: Derive
#[derive(McpTool)]
struct Calculator { a: f64, b: f64 }

// Level 3: Builder
let tool = ToolBuilder::new("calc").execute(|args| async { /*...*/ }).build()?;

// Level 4: Manual trait implementation
```

### Basic Server
```rust
use turul_mcp_server::prelude::*;

let server = McpServer::builder()
    .name("my-server")
    .tool(Calculator::default())
    .build()?;

server.run().await
```

### Development Commands
```bash
cargo build
cargo test
cargo run --example minimal-server

# MCP Testing
cargo run --example client-initialise-server -- --port 52935
cargo run --example client-initialise-report -- --url http://127.0.0.1:52935/mcp
```

## Core Modification Rules

### 🚨 Production Safety
- **NO PANICS**: Use `Result<T, McpError>` for fallible operations
- **Error Handling**: Graceful degradation, proper MCP error types
- **Builder Stability**: Changes require breaking change analysis
- **Zero-Config**: Framework handles invalid inputs gracefully

### Before Core Changes
1. **Impact Analysis**: All examples, tests, user code affected?
2. **Backwards Compatibility**: Breaking changes documented clearly
3. **Production Safety**: No crashes from user input
4. **Testing**: Framework-native APIs, not JSON manipulation

## Architecture

### Core Crates
- `turul-mcp-server/` - High-level framework
- `turul-mcp-protocol/` - Protocol types/traits
- `turul-mcp-builders/` - Runtime builders
- `turul-http-mcp-server/` - HTTP transport
- `turul-mcp-derive/` - Macros

### Session Management
- UUID v7 sessions with automatic cleanup
- Streamable HTTP with SSE notifications
- Pluggable storage (InMemory, SQLite, PostgreSQL, DynamoDB)

### Testing Philosophy
```rust
// ✅ Framework-native
let tool = CalculatorTool { a: 5.0, b: 3.0 };
let result = tool.call(json!({"a": 5.0, "b": 3.0}), None).await?;

// ❌ Raw JSON manipulation
let json_request = r#"{"method":"tools/call"}"#;
```

## Key Guidelines
- **Extend existing** components, never create "enhanced" versions
- **Component consistency**: Use existing patterns and conventions
- **Documentation accuracy**: All examples must compile and work
- **MCP Compliance**: Only official 2025-06-18 spec methods
- **Zero warnings**: `cargo check` must be clean
