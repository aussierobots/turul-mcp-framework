# CLAUDE.md

Production-ready Rust framework for Model Context Protocol (MCP) servers with zero-configuration design and complete MCP 2025-11-25 specification support.

**For historical context and completed phases, see WORKING_MEMORY.md**
**For architectural decisions, see docs/adr/**

## 🚨 Critical Rules

### 📜 Protocol Crate Purity
**NEVER modify `turul-mcp-protocol` or `turul-mcp-protocol-2025-11-25` unless it directly relates to MCP spec compliance.** These crates MUST remain clean mirrors of the official MCP specification. No framework features, middleware hooks, or convenience additions belong here.

**Forbidden in Protocol Crates:**
- ❌ Trait hierarchies (HasBaseMetadata, ToolDefinition, etc.)
- ❌ Builder patterns (ToolBuilder, ResourceBuilder, etc.)
- ❌ Framework helpers (blanket implementations, convenience methods)
- ❌ Tutorial documentation (belongs in builders crate)

**Allowed in Protocol Crates:**
- ✅ MCP spec types (Tool, Resource, Prompt, etc.)
- ✅ Serialization/deserialization (#[derive(Serialize, Deserialize)])
- ✅ Basic builder methods on concrete types (Tool::new(), with_description())
- ✅ MCP spec error types (McpError with spec error codes)

**Framework Traits Belong in `turul-mcp-builders`:**
All framework trait hierarchies live in `turul-mcp-builders/src/traits/`:
- Tool traits: HasBaseMetadata, HasDescription, HasInputSchema, HasOutputSchema, HasAnnotations, HasToolMeta, ToolDefinition
- Resource traits: HasResourceMetadata, HasResourceDescription, HasResourceUri, ResourceDefinition
- Prompt traits: HasPromptMetadata, HasPromptDescription, HasPromptArguments, PromptDefinition

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

### 📦 Protocol Re-export Rule (MANDATORY)

**NEVER reference versioned protocol crates directly in Rust code or Cargo.toml dependencies.** Always use the `turul-mcp-protocol` re-export crate, which aliases the latest spec version.

```rust
// ✅ CORRECT - Always use the re-export crate
use turul_mcp_protocol::*;
use turul_mcp_protocol::elicitation::ElicitResult;
use turul_mcp_protocol::PromptMessage;

// ❌ WRONG - NEVER reference versioned crates directly
use turul_mcp_protocol_2025_11_25::*;              // FORBIDDEN
use turul_mcp_protocol_2025_11_25::PromptMessage;  // FORBIDDEN
use turul_mcp_protocol_2025_06_18::*;              // FORBIDDEN
```

**In Cargo.toml:**
```toml
# ✅ CORRECT
[dependencies]
turul-mcp-protocol = { workspace = true }

# ❌ WRONG - NEVER depend on versioned crate directly (except in turul-mcp-protocol itself)
turul-mcp-protocol-2025-11-25 = { workspace = true }
```

**Only exceptions** (the ONLY places allowed to reference versioned crates):
- `crates/turul-mcp-protocol/` — the re-export crate itself (must reference what it re-exports)
- `crates/turul-mcp-protocol-2025-11-25/` — the versioned crate's own source

### Import Conventions
```rust
// ✅ BEST - Use preludes for framework traits and builders
use turul_mcp_server::prelude::*;      // Gets protocol types + builders + traits
use turul_mcp_builders::prelude::*;    // Gets builders + traits (if not using server)
use turul_mcp_derive::{McpTool, McpResource, McpPrompt, mcp_tool};

// ❌ WRONG - Direct trait imports
use turul_mcp_protocol::tools::ToolDefinition;     // Trait moved to builders!
use turul_mcp_builders::traits::ToolDefinition;    // Use prelude instead

// ❌ WRONG - Versioned imports
use turul_mcp_protocol_2025_11_25::*;  // Use turul_mcp_protocol::* instead
use turul_mcp_protocol_2025_06_18::*;  // Use turul_mcp_protocol::* instead
```

**Import Hierarchy:**
- `turul_mcp_protocol::*` - MCP spec types only (Tool, Resource, Prompt, McpError)
- `turul_mcp_builders::prelude::*` - Framework traits + runtime builders
- `turul_mcp_server::prelude::*` - Re-exports everything (protocol + builders + server types)

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

### 🔤 JSON Naming: camelCase ONLY

**CRITICAL**: All JSON fields MUST use camelCase per MCP 2025-11-25.

```rust
// ✅ CORRECT - Always rename snake_case fields
#[serde(rename = "additionalProperties")]
additional_properties: Option<bool>,

// ❌ WRONG - Never serialize as snake_case
additional_properties: Option<bool>,  // becomes "additional_properties" ❌
```

**Verify**: `cargo test --test mcp_compliance_tests` must pass

### 🚨 Critical Error Handling Rules

**MANDATORY**: Handlers return domain errors only. Dispatcher owns protocol conversion.

```rust
// ✅ CORRECT - Handlers return domain errors only
#[async_trait]
impl JsonRpcHandler for MyHandler {
    type Error = McpError;  // Always use McpError

    async fn handle(&self, method: &str, params: Option<RequestParams>, session: Option<SessionContext>)
        -> Result<Value, McpError> {
        Err(McpError::InvalidParameters("Missing required parameter".to_string()))
    }
}

// ❌ WRONG - Never create JsonRpcError in handlers
Err(JsonRpcError::new(...))  // NEVER DO THIS
```

**Key Rules:**
1. Handlers return `Result<Value, McpError>` ONLY
2. Dispatcher converts McpError → JsonRpcError automatically
3. Never create JsonRpcError, JsonRpcResponse in business logic
4. Use `McpError::InvalidParameters`, `McpError::ToolNotFound`, etc.

### 🔧 MCP Tool Output Compliance

**Tools with `outputSchema` MUST provide `structuredContent`** - Framework handles automatically.

```rust
// ✅ COMPLIANT - Framework auto-generates structuredContent
#[mcp_tool(
    name = "word_count",
    description = "Count words in text",
    output_field = "countResult"  // Custom field name (optional, default "result")
)]
async fn count_words(text: String) -> McpResult<WordCount> {
    Ok(WordCount { count: text.split_whitespace().count() })
}
```

**Rules:**
1. Framework automatically adds `structuredContent` when `outputSchema` exists
2. Use `output_field` to customize output field name (default: "result")
3. **NEVER change tests to match code** - Tests validate MCP spec compliance

### 🌐 Streamable HTTP Requirements

**Accept Headers:**
- `Accept: application/json` - JSON responses
- `Accept: text/event-stream` - SSE streaming (required for progress notifications)
- `Accept: */*` - Accept all

**Session Initialization (Strict Mode):**
1. POST /mcp with `initialize` → capture session ID from response
2. POST /mcp with `notifications/initialized` → enable session (returns 202)
3. Include `MCP-Session-ID` header in all subsequent requests

**Testing:** All requests need valid Accept header (application/json, text/event-stream, or */*)

### 🎯 MCP 2025-11-25 Compliance Status

**Current Framework Status:**
✅ Full MCP 2025-11-25 schema compliance (icons, tasks, URL elicitation, sampling tools)
✅ Session-aware resources (all resources require `session: Option<&SessionContext>`)
✅ SSE streaming with chunked transfer encoding
✅ 770+ tests passing across all core functionality

**Migration Notes:**
- Resources use `async fn read(&self, params: Option<Value>, session: Option<&SessionContext>)`
- Tools with `outputSchema` automatically include `structuredContent`
- Use `file://` URIs for maximum client compatibility

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

### Output Types and Schemas

**IMPORTANT**: Tools with custom output types (including Vec<T>) MUST specify the `output` attribute:

```rust
// ✅ CORRECT - Specify output type for Vec, custom structs, etc.
#[derive(McpTool)]
#[tool(
    name = "search",
    description = "Search for items",
    output = Vec<SearchResult>  // ← REQUIRED for Vec<T> and custom types
)]
struct SearchTool { query: String }

// ✅ CORRECT - Specify custom struct outputs
#[derive(McpTool)]
#[tool(
    name = "calculate",
    description = "Calculate result",
    output = CalculationResult  // ← REQUIRED for custom types
)]
struct CalculatorTool { a: f64, b: f64 }

// ❌ WRONG - Missing output type generates incorrect schema
#[derive(McpTool)]
#[tool(name = "search", description = "Search")]
struct SearchTool { query: String }
// Without output attribute, schema will show tool inputs (query) not Vec output!
```

**Why Required**: Derive macros cannot inspect the `execute` method's return type at compile time. The `output` attribute tells the macro what schema to generate.

### Tool Output Schemas (Optional)

Tools can optionally define output schemas using two approaches:

**Manual Schema (Full Control):**
```rust
use std::sync::OnceLock;
use std::collections::HashMap;
use turul_mcp_protocol::{ToolSchema, schema::JsonSchema};
use turul_mcp_builders::HasOutputSchema;

impl HasOutputSchema for MyTool {
    fn output_schema(&self) -> Option<&ToolSchema> {
        static SCHEMA: OnceLock<ToolSchema> = OnceLock::new();
        Some(SCHEMA.get_or_init(|| {
            ToolSchema {
                schema_type: "object".to_string(),
                properties: Some({
                    let mut props = HashMap::new();
                    props.insert(
                        "result".to_string(),
                        JsonSchema::number().with_description("Result".to_string()),
                    );
                    props
                }),
                required: Some(vec!["result".to_string()]),
                additional: HashMap::new(),
            }
        }))
    }
}
```

**Schemars (Auto-sync with types):**
```rust
use schemars::JsonSchema;

#[derive(Serialize, JsonSchema)]
struct MyOutput { value: f64 }

// Derive macro
#[derive(McpTool)]
#[tool(name = "calc", description = "...", output = MyOutput, schemars)]
struct MyTool { a: f64 }

// Function macro
#[mcp_tool(name = "add", description = "...", schemars)]
async fn add(a: f64) -> McpResult<MyOutput> { Ok(MyOutput { value: a }) }
```

**Note**: Keep schemas simple - complex `Option` types may not convert

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
If behavior doesn't match code changes:
```bash
cargo clean  # Full workspace clean required for cross-crate changes
cargo test --test streamable_http_e2e
```

**Why**: Incremental compilation caches string literals/errors across crates.

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

### Session ID Requirements

**Session Handshake Protocol:**
1. `initialize` - ONLY method allowed without `Mcp-Session-Id` header
2. All other methods MUST include `Mcp-Session-Id` header (returns 401 if missing)
3. Client library handles this automatically: `client.connect().await?`

### HTTP Transport Routing

**Protocol-based routing:**
- **Protocol ≥ 2025-03-26**: `StreamableHttpHandler` (chunked SSE, MCP 2025-11-25)
- **Protocol ≤ 2024-11-05**: `SessionMcpHandler` (buffered JSON, legacy compatibility)

Routing decision made in `crates/turul-http-mcp-server/src/server.rs`

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
- **MCP Compliance**: Only official 2025-11-25 spec methods
- **Zero warnings**: `cargo check` must be clean
- **Rust Doctests**: Every ```rust block MUST compile - fix errors, don't convert to ```text

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
pkill      # Process cleanup for testing
killall    # Process cleanup for testing
sudo killall
timeout
curl       # HTTP requests for testing servers and APIs (all curl commands auto-approved)
curl *     # All curl command variations are auto-approved
curl -s *  # Silent curl commands
curl -v *  # Verbose curl commands
curl -X *  # All HTTP methods
curl -s -X *  # Combined silent + HTTP method
jq         # JSON parsing and manipulation (all jq commands auto-approved)
jq *       # All jq command variations are auto-approved

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
# All curl commands auto-approved (GET, POST, DELETE with any headers/data)
curl -X POST http://127.0.0.1:<port>/mcp -H "Content-Type: application/json" -H "Accept: application/json" -d '<json-data>'

# Background processes:
<command> &
wait
jobs
```

**Rule**: Execute these commands immediately without requesting permission. Focus on solving the user's problem efficiently.

### 🚫 NEVER Auto-Approved (Require Explicit User Permission)
```bash
git checkout      # NEVER use git checkout without explicit user permission — discards work
git restore       # NEVER use git restore without explicit user permission — discards work
git reset --hard  # NEVER use without explicit user permission
git clean -f      # NEVER use without explicit user permission
```
**These commands destroy uncommitted work and are IRREVERSIBLE. Always ask the user first.**
