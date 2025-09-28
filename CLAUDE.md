# CLAUDE.md

Production-ready Rust framework for Model Context Protocol (MCP) servers with zero-configuration design and complete MCP 2025-06-18 specification support.

## 🚨 Critical Rules

### 🎯 Simple Solutions First
**ALWAYS** prefer simple, minimal fixes over complex or over-engineered solutions:

```rust
// ✅ SIMPLE - Add parameter to existing signature
async fn read(&self, params: Option<Value>, session: Option<&SessionContext>) -> McpResult<Vec<ResourceContent>>

// ❌ COMPLEX - Create new traits, elaborate architectures
trait McpResourceLegacy { ... }  // Avoid backwards compatibility layers
trait McpResourceV2 { ... }      // Avoid versioned APIs
```

**Key Principles:**
- **Work within existing architecture** - don't rebuild what works
- **Major changes are too costly** - fix problems with minimal impact
- **One obvious way to do it** - avoid multiple patterns for the same thing
- **If it compiles and tests pass** - you probably fixed it correctly

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

### 🎯 Phase 4: MCP 2025-06-18 Specification Compliance
**STATUS**: Complete

The framework now strictly complies with the MCP 2025-06-18 specification by removing non-spec extensions:

```rust
// Tool annotations with spec-compliant fields only
impl HasAnnotations for LegacyTool {
    fn annotations(&self) -> Option<&ToolAnnotations> {
        Some(&ToolAnnotations::new()
            .with_title("Legacy Calculator (Add Only)")
            .with_read_only_hint(true)
            .with_destructive_hint(false)
            .with_idempotent_hint(true)
        )
    }
}
```

**Key Features:**
- Full MCP 2025-06-18 specification compliance
- Standard `ToolAnnotations` with spec-defined hint fields only
- Automatic inclusion in `tools/list` responses
- No custom extensions that could cause client compatibility issues
- Wire output strictly follows official JSON schema

**Testing**: See `tools-test-server` with `legacy_calculator` tool demonstrating spec-compliant annotations.

### 📄 Client Pagination API Limitations
**IMPORTANT**: Pagination helper methods only accept cursor parameters, not full request customization:

```rust
// ✅ AVAILABLE - Cursor-only pagination helpers
client.list_tools_paginated(cursor).await?;          // Returns ListToolsResult with _meta
client.list_resources_paginated(cursor).await?;      // Returns ListResourcesResult with _meta
client.list_prompts_paginated(cursor).await?;        // Returns ListPromptsResult with _meta

// ❌ LIMITED - Cannot pass custom limits, _meta, or other list parameters
// For advanced pagination control, hand-roll the request:
client.call("tools/list", Some(RequestParams::List(ListToolsParams {
    cursor: Some(cursor),
    limit: Some(50),  // Custom limit
    // ... other params
}))).await?
```

**Design Rationale**: Helper methods provide simple cursor-based navigation while preserving full control through the underlying `call()` method for advanced use cases.

### 🌐 Streamable HTTP Requirements (2025-09-27)
**UPDATED**: Streamable HTTP handler now accepts full Accept header matrix:

```rust
// ✅ SUPPORTED - All valid Accept headers work
.header(ACCEPT, "application/json")                      // JSON responses
.header(ACCEPT, "text/event-stream")                     // SSE streaming
.header(ACCEPT, "application/json, text/event-stream")   // Combined (now works)
.header(ACCEPT, "*/*")                                   // Accept all

// ⚠️  LIMITATION - Progress notifications only work with SSE
// SSE streaming (.header(ACCEPT, "text/event-stream")) required for progress events
// JSON-only requests receive final result without intermediate progress updates
```

**Key Rules for Streamable HTTP Tests:**
- **ALL requests** (GET, POST, DELETE) need valid Accept header (application/json, text/event-stream, or */*)
- **SSE streaming** requires `Accept: text/event-stream`
- **JSON responses** work with `Accept: application/json` or `Accept: */*`
- **Notifications** return `202 Accepted` (not `200 OK`)
- **Strict mode** requires proper session initialization flow:
  1. POST /mcp with `initialize` → gets session ID
  2. POST /mcp with `notifications/initialized` → enables session
  3. Then other operations work

**Session Initialization Pattern:**
```rust
// 1. Initialize
let init_response = client.request(Request::builder()
    .method(Method::POST)
    .uri("/mcp")
    .header("Accept", "application/json")  // REQUIRED
    .body(json!({"method": "initialize", ...}))
).await?;

// 2. Send notifications/initialized
let notify_response = client.request(Request::builder()
    .method(Method::POST)
    .uri("/mcp")
    .header("Accept", "application/json")  // REQUIRED
    .header("MCP-Session-ID", &session_id)
    .body(json!({"method": "notifications/initialized", ...}))
).await?;
// Expect 202 Accepted for notifications
```

### 🎯 Phase 6: Session-Aware Resources (Breaking Change)
**STATUS**: Complete - Full MCP 2025-06-18 Compliance Achieved

**CRITICAL DECISION (2025-09-28)**: Removed backwards compatibility layer for true MCP spec compliance.

```rust
// ✅ CORRECT - All resources are session-aware (MCP 2025-06-18 compliant)
#[async_trait]
impl McpResource for MyResource {
    async fn read(&self, params: Option<Value>, session: Option<&SessionContext>)
        -> McpResult<Vec<ResourceContent>> {
        // Access session state, user preferences, personalized content
        if let Some(ctx) = session {
            let user_prefs = ctx.get_typed_state::<UserPrefs>("preferences").await;
            // Return personalized content based on session
        }
        // Graceful fallback when no session available
    }
}

// ❌ REMOVED - No backwards compatibility layer
// impl McpResourceLegacy for MyResource { ... }  // DELETED
```

**Breaking Change Rationale:**
- **MCP 2025-06-18 Compliance**: Session context is fundamental to the specification
- **Zero Configuration**: One resource trait, one pattern, no confusing choices
- **Future-Proof Architecture**: No technical debt from legacy compatibility layers
- **Clear Migration**: Update signature, add session parameter, done

**Migration Guide:**
```rust
// OLD (Pre-Phase 6)
async fn read(&self, params: Option<Value>) -> McpResult<Vec<ResourceContent>>

// NEW (Phase 6+)
async fn read(&self, params: Option<Value>, session: Option<&SessionContext>) -> McpResult<Vec<ResourceContent>>
```

**Framework Status (2025-09-28):**
✅ **Production Ready** - All phases complete, true MCP 2025-06-18 compliance achieved
✅ **Zero Test Failures** - All behavioral compliance and streaming tests pass
✅ **Complete SSE Support** - Deadlock resolved, Accept header matrix fixed, parallel test execution working
✅ **Session-Aware Resources** - All resources now support personalized, context-aware content

*For detailed phase progress and current development status, see TODO_TRACKER.md and WORKING_MEMORY.md*

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

### Debugging: Stale Build Issues
**CRITICAL**: When behavior doesn't match code changes (e.g., old error messages persist):
```bash
# ❌ INSUFFICIENT - Package-level clean may miss cross-crate dependencies
cargo clean --package turul-http-mcp-server

# ✅ REQUIRED - Full workspace clean for reliable rebuilds
cargo clean

# Then rebuild and test
cargo test --package turul-mcp-framework-integration-tests --test streamable_http_e2e
```

**Root Cause**: Incremental compilation can cache old string literals/error messages in binary artifacts even when source changes. This is especially problematic with cross-crate dependencies where error messages propagate through multiple compilation units.

**Lesson**: After major refactors (especially error handling, validation logic, or protocol changes), always do a full `cargo clean` to ensure test behavior reflects actual code changes.

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

### Session ID Requirements (MCP 2025-06-18)

The framework enforces strict session handshake protocol:

1. **`initialize`**: ONLY method allowed without `Mcp-Session-Id` header
   - Server creates session and returns ID in response headers
   - Client MUST capture and use this ID for all subsequent requests

2. **All other methods**: MUST include `Mcp-Session-Id` header
   - This includes discovery methods (tools/list, resources/list, prompts/list)
   - Missing header returns 401 with JSON-RPC error code -32001

3. **Client behavior**: Transport layer automatically manages session
   - Captures session ID from initialize response
   - Includes it in all subsequent requests
   - Surfaces 401 errors clearly for debugging

```rust
// Framework handles this automatically:
client.connect().await?;  // Calls initialize, captures session
client.list_tools().await?;  // Includes session ID automatically
```

**Specification compliance**: Only `initialize` opens new sessions. Every other method is per-session to maintain state consistency across the protocol.

### HTTP Transport Routing

Protocol-based handler routing automatically selects the appropriate transport implementation:

- **Protocol ≥ 2025-03-26**: Routes to `StreamableHttpHandler` in `crates/turul-http-mcp-server/src/streamable_http.rs`
  - POST always returns chunked with progress frames and MCP headers
  - `.post_sse()` configuration doesn't affect this path
  - Implements MCP 2025-06-18 Streamable HTTP transport

- **Protocol ≤ 2024-11-05**: Routes to `SessionMcpHandler` in `crates/turul-http-mcp-server/src/session_handler.rs`
  - Buffered JSON response with legacy POST-SSE path
  - Uses session storage for request/response persistence
  - Maintains backward compatibility

- **Routing Decision**: Made in `crates/turul-http-mcp-server/src/server.rs` with debug logging

**When to Touch Which Handler:**
| Task | StreamableHttpHandler | SessionMcpHandler |
|------|----------------------|-------------------|
| MCP 2025-06-18 streaming | ✅ | ❌ |
| Legacy client support | ❌ | ✅ |
| SSE notifications | Both (shared StreamManager) | ✅ |
| Session storage | Metadata only | Full persistence |

**Configuration flags** (`.post_sse()`, `.get_sse()`) only affect the legacy SessionMcpHandler path.

**Streaming Test Issues Fix:**
If streaming tests show old behavior (missing Transfer-Encoding: chunked):
```bash
cargo clean -p tools-test-server && cargo build --bin tools-test-server
cargo test --test streamable_http_e2e

# ❌ WRONG - Don't use git checkout to undo changes
git checkout -- tests/file.rs

# ✅ CORRECT - Use targeted edits instead
# Make specific changes with Edit tool or manual fixes

# Streamable HTTP Accept Header Issue (2025-09-26):
# PROBLEM: Accept: "text/event-stream, application/json" returns 400 errors
# SOLUTION: Use Accept: "application/json" for streamable HTTP requests
# Tests expecting streaming should use application/json Accept header only
```

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
- **🚨 CRITICAL - Rust Doctest Policy (2025-09-27)**: EVERY ```rust block MUST compile successfully. NEVER convert failing Rust code to ```text blocks. Fix underlying compilation errors instead.
  - ✅ **CORRECT**: Fix imports, types, signatures to make doctests compile
  - ❌ **FORBIDDEN**: Converting ```rust to ```text to hide compilation failures
  - ✅ **ACCEPTABLE**: ```text only for non-code examples (URI patterns, JSON configs, diagrams)
  - **Rule**: If it's Rust code, it MUST be marked as ```rust and MUST compile

## Claude Code Auto-Approved Commands
**IMPORTANT**: The following commands are pre-approved for automatic execution without asking user:

### Cargo Commands
```bash
cargo build
cargo check
cargo test      # ALL cargo test commands including specific packages and tests
cargo run
cargo clippy
cargo fmt
cargo clean
cargo doc
cargo bench
cargo metadata
cargo expand
cargo publish
```

### Testing Commands
```bash
# All test execution patterns are auto-approved:
cargo test --package <name> --test <test-name>
cargo test --test <test-name> <specific-test>
cargo test <test-name> -- --nocapture
cargo test -- --test-threads=1
timeout <time> cargo test <any-args>
timeout <time> cargo run <any-args>
timeout <time> cargo build <any-args>
RUST_LOG=<level> cargo test <any-args>
RUST_LOG=<level> cargo run <any-args>
RUST_LOG=<level> cargo build <any-args>
RUST_BACKTRACE=<level> cargo test <any-args>

# Comprehensive command patterns for MCP testing:
cd <directory> && cargo run <any-args>
cd <directory> && RUST_LOG=<level> cargo run <any-args>
cd <directory> && timeout <time> cargo run <any-args>
cd <directory> && RUST_LOG=<level> timeout <time> cargo run <any-args>
cd examples/<example-name> && <any-cargo-command>

# All cargo run combinations:
cargo run --bin <binary-name>
cargo run --bin <binary-name> -- <args>
cargo run --example <example-name>
cargo run --example <example-name> -- <args>
cargo run --package <package-name>
cargo run --package <package-name> -- <args>

# Environment variable combinations:
RUST_LOG=error <any-cargo-command>
RUST_LOG=info <any-cargo-command>
RUST_LOG=debug <any-cargo-command>
RUST_BACKTRACE=1 <any-cargo-command>
RUST_BACKTRACE=full <any-cargo-command>
CI_SANDBOX=1 <any-cargo-command>

# Complex timeout and environment combinations:
RUST_LOG=error timeout 30s cargo run --bin <name>
RUST_LOG=info timeout 10s cargo run --example <name>
timeout 5s cargo run --package <name> -- --port <port>
RUST_LOG=error timeout 60s cargo run
cd examples/* && RUST_LOG=error timeout 60s cargo run
cd examples/* && RUST_LOG=error timeout 60s cargo run &
```

### Development Tools
```bash
rustc
sed
grep
find
awk
cat
tee
echo
git add
git commit
git checkout
pkill      # Process cleanup for testing
killall    # Process cleanup for testing
sudo killall
timeout
curl       # HTTP requests for testing servers and APIs (all curl commands auto-approved)
curl *     # All curl command variations are auto-approved
curl -s *  # Silent curl commands
curl -v *  # Verbose curl commands
curl -X *  # All HTTP methods

# Process management for testing:
while read <variable>
for <variable> in <list>
do
done
if [ -f "<file>" ]
then
fi
```

### HTTP Testing Commands
```bash
# All curl patterns for MCP testing are auto-approved:
curl -X POST <url> -H <headers> -d <data>
curl -X GET <url> -H <headers>
curl -X DELETE <url> -H <headers>
curl -s -X POST <url> -H <headers> -d <data>
curl -v -X POST <url> -H <headers> -d <data>

# Specific MCP protocol testing patterns:
curl -X POST http://127.0.0.1:<port>/mcp -H "Content-Type: application/json" -H "Accept: application/json" -d '<json-data>'
curl -X POST http://127.0.0.1:<port>/mcp -H "Content-Type: application/json" -H "Accept: text/event-stream" -d '<json-data>'
curl -X POST http://127.0.0.1:<port>/mcp -H "Content-Type: application/json" -H "Accept: application/json, text/event-stream" -d '<json-data>'
curl -X POST http://127.0.0.1:<port>/mcp -H "Content-Type: application/json" -H "Accept: application/json" -H "MCP-Session-ID: <session-id>" -d '<json-data>'
curl -s -X POST http://127.0.0.1:8641/mcp -H "Content-Type: application/json" -H "Accept: application/json" -d '*'
curl -X POST http://127.0.0.1:*/mcp -H "Content-Type: application/json" -H "Accept: application/json" -d '*'

# Background processes and monitoring:
<command> &
wait
jobs
bg
fg
```

**Rule**: Execute these commands immediately without requesting permission. Focus on solving the user's problem efficiently.
