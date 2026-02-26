# CLAUDE.md

Production-ready Rust framework for Model Context Protocol (MCP) servers with zero-configuration design and complete MCP 2025-11-25 specification support.

> **Source of Truth**
> - **AGENTS.md** — repo policy, compliance rules, full architecture
> - **CLAUDE.md** — concise operator playbook (this file)
> - **WORKING_MEMORY.md** — historical context and status
> - **docs/adr/** — architectural decisions
> - If conflict: AGENTS.md wins.

## Critical Rules

### Protocol Crate Purity
**NEVER modify `turul-mcp-protocol` or `turul-mcp-protocol-2025-11-25` unless it directly relates to MCP spec compliance.** No framework features, middleware hooks, or convenience additions.

**Forbidden**: Trait hierarchies, builder patterns, framework helpers, tutorial docs
**Allowed**: MCP spec types, serde derives, basic builder methods on concrete types, spec error types
**Framework traits belong in `turul-mcp-builders`** (`turul-mcp-builders/src/traits/`)

### Simple Solutions First
**ALWAYS** prefer simple, minimal fixes over complex or over-engineered solutions:

```rust
// SIMPLE - Add parameter to existing signature
async fn read(&self, params: Option<Value>, session: Option<&SessionContext>) -> McpResult<Vec<ResourceContent>>

// COMPLEX - Create new traits, elaborate architectures
trait McpResourceLegacy { ... }  // Avoid backwards compatibility layers
trait McpResourceV2 { ... }      // Avoid versioned APIs
```

**Key Principles:**
- **Work within existing architecture** - don't rebuild what works
- **Major changes are too costly** - fix problems with minimal impact
- **One obvious way to do it** - avoid multiple patterns for the same thing
- **If it compiles and tests pass** - you probably fixed it correctly

### Protocol Re-export Rule (MANDATORY)

**NEVER reference versioned protocol crates directly.** Always use the `turul-mcp-protocol` re-export crate.

```rust
// CORRECT
use turul_mcp_protocol::*;
use turul_mcp_protocol::elicitation::ElicitResult;

// WRONG - NEVER reference versioned crates directly
use turul_mcp_protocol_2025_11_25::*;   // FORBIDDEN
use turul_mcp_protocol_2025_06_18::*;   // FORBIDDEN
```

**Only exceptions**: `crates/turul-mcp-protocol/` (the re-export crate itself) and `crates/turul-mcp-protocol-2025-11-25/` (its own source).

**Import Hierarchy** (prefer top):
- `turul_mcp_server::prelude::*` — re-exports everything (protocol + builders + server types)
- `turul_mcp_builders::prelude::*` — framework traits + runtime builders
- `turul_mcp_protocol::*` — MCP spec types only (Tool, Resource, Prompt, McpError)

### Zero-Configuration Design
Users NEVER specify method strings - framework auto-determines from types:
```rust
// CORRECT
#[derive(McpTool)]
struct Calculator;  // Framework → tools/call

// WRONG
#[mcp_tool(method = "tools/call")]  // NO METHOD STRINGS!
```

### API Conventions
- **SessionContext**: Use `get_typed_state(key).await` and `set_typed_state(key, value).await?`
- **Builder Pattern**: `McpServer::builder()` not `McpServerBuilder::new()`
- **Error Handling**: Always use `McpError` types - NEVER create JsonRpcError directly in handlers
- **Session IDs**: Always `Uuid::now_v7().as_simple()` for temporal ordering (no-hyphen hex)

### JSON Naming: camelCase ONLY

**CRITICAL**: All JSON fields MUST use camelCase per MCP 2025-11-25.

```rust
// CORRECT - Always rename snake_case fields
#[serde(rename = "additionalProperties")]
additional_properties: Option<bool>,

// WRONG - Never serialize as snake_case
additional_properties: Option<bool>,  // becomes "additional_properties"
```

### Critical Error Handling Rules

**MANDATORY**: Handlers return domain errors only. Dispatcher owns protocol conversion.

**Key Rules:**
1. Handlers return `Result<Value, McpError>` ONLY
2. Dispatcher converts McpError → JsonRpcError automatically
3. Never create JsonRpcError, JsonRpcResponse in business logic
4. Use `McpError::InvalidParameters`, `McpError::ToolNotFound`, etc.

### MCP Tool Output Compliance

**Tools with `outputSchema` MUST provide `structuredContent`** - Framework handles automatically.

```rust
// Framework auto-generates structuredContent
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

### Streamable HTTP Requirements

**Accept Headers:**
- `Accept: application/json` - JSON responses
- `Accept: text/event-stream` - SSE streaming (required for progress notifications)
- `Accept: */*` - Accept all

**Session Initialization (Strict Mode):**
1. POST /mcp with `initialize` → capture session ID from response
2. POST /mcp with `notifications/initialized` → enable session (returns 202)
3. Include `MCP-Session-ID` header in all subsequent requests

**Testing:** All requests need valid Accept header (application/json, text/event-stream, or */*)

### MCP 2025-11-25 Compliance

**Notification method strings**: `notifications/*/list_changed` (underscore) — spec-compliant form. Server accepts legacy `listChanged` (camelCase) for backward compat only.

**JSON capability keys**: `listChanged` (camelCase) — correct per spec.

**ToolChoiceMode**: `"auto" | "none" | "required"`. Legacy `"any"` accepted on deserialize only.

**Role enum**: `User` and `Assistant` only — no `System` variant in MCP protocol.

**Progress fields**: `f64` (not `u64`). Use `as_f64()`, never `as_u64()`.

**structuredContent**: Auto-generated by framework when `outputSchema` exists.

**Session handshake**: `initialize` → `notifications/initialized` → `Mcp-Session-Id` header on all subsequent requests.

**Verify**: `cargo test -p turul-mcp-framework-integration-tests --test compliance`

## Quick Reference

### Tool Creation (macro-first)
```rust
// Recommended: Function macro
#[mcp_tool(name = "add", description = "Add two numbers")]
async fn add(a: f64, b: f64) -> McpResult<f64> { Ok(a + b) }

// Alternative: Derive macro
#[derive(McpTool)]
#[tool(name = "calc", description = "Calculate", output = CalcResult)]
struct CalcTool { a: f64, b: f64 }

// Runtime: Builder
let tool = ToolBuilder::new("calc").execute(|args| async { /*...*/ }).build()?;

// Manual trait implementation: reference-only — see examples/calculator-add-manual-server
```

### Output Types and Schemas

**IMPORTANT**: Tools with custom output types (including Vec<T>) MUST specify the `output` attribute:

```rust
#[derive(McpTool)]
#[tool(name = "search", description = "Search", output = Vec<SearchResult>)]
struct SearchTool { query: String }
// Without output attribute, schema shows tool inputs not output type!
```

**Why Required**: Derive macros cannot inspect the `execute` method's return type at compile time. The `output` attribute tells the macro what schema to generate.

**Schemars (automatic detection):**
If the output type derives `schemars::JsonSchema`, the framework automatically uses it for detailed schema generation — no additional schemars flag is needed. The `output = Type` attribute is still required on derive macros:
```rust
#[derive(schemars::JsonSchema, serde::Serialize)]
struct MyOutput { value: f64 }

#[derive(McpTool)]
#[tool(name = "calc", description = "...", output = MyOutput)]  // output = required
struct MyTool { a: f64 }

#[mcp_tool(name = "add", description = "Add numbers")]
async fn add(a: f64) -> McpResult<MyOutput> { Ok(MyOutput { value: a }) }  // auto-detected from return type
```

For manual `HasOutputSchema` implementation, see `examples/calculator-add-manual-server`.

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
cargo test -p turul-mcp-framework-integration-tests --test e2e_tests
```

**Why**: Incremental compilation caches string literals/errors across crates.

## Before Modifying Core Crates

- **Impact Analysis**: All examples, tests, user code affected?
- **Breaking changes documented** clearly
- **No panics** — `Result<T, McpError>` for all fallible operations
- **Zero warnings**: `cargo check` must be clean
- **Doctests**: Every ```rust block MUST compile — fix errors, don't convert to ```text
- **Extend existing** components, never create "enhanced" versions
- **Test with framework-native APIs**, not raw JSON manipulation

```rust
// Framework-native testing
let tool = CalculatorTool { a: 5.0, b: 3.0 };
let result = tool.call(json!({"a": 5.0, "b": 3.0}), None).await?;

// NOT raw JSON manipulation
let json_request = r#"{"method":"tools/call"}"#;
```

## Architecture

### Workspace Crates
- `turul-mcp-server/` - High-level framework (main entry point)
- `turul-mcp-protocol/` - Protocol re-export crate (always use this)
- `turul-mcp-protocol-2025-11-25/` - Versioned protocol types (internal only)
- `turul-mcp-protocol-2025-06-18/` - Legacy protocol (backward compat)
- `turul-mcp-builders/` - Runtime builders + framework traits
- `turul-mcp-derive/` - Proc macros (McpTool, McpResource, McpPrompt, mcp_tool)
- `turul-http-mcp-server/` - HTTP/SSE transport
- `turul-mcp-json-rpc-server/` - JSON-RPC dispatch layer
- `turul-mcp-client/` - Client library
- `turul-mcp-session-storage/` - Pluggable session storage (InMemory, SQLite, PostgreSQL, DynamoDB)
- `turul-mcp-task-storage/` - Task storage for long-running operations
- `turul-mcp-aws-lambda/` - AWS Lambda integration

### Session Management
- UUID v7 sessions with automatic cleanup
- Streamable HTTP with SSE notifications
- Pluggable storage (InMemory, SQLite, PostgreSQL, DynamoDB)

### HTTP Transport Routing
- **Protocol >= 2025-03-26**: `StreamableHttpHandler` (chunked SSE, MCP 2025-11-25)
- **Protocol <= 2024-11-05**: `SessionMcpHandler` (buffered JSON, legacy compatibility)

Routing in `crates/turul-http-mcp-server/src/server.rs`

## Generally Safe Dev Commands

The following are considered safe for automatic execution during development:
- `cargo build/check/test/run/clippy/fmt/clean/doc/bench/metadata/expand` — including with `--package`, `--test`, `--bin`, `--example` flags, environment variables (`RUST_LOG`, `RUST_BACKTRACE`, `CI_SANDBOX`), and `timeout` wrappers
- `cd <dir> && cargo <command>` — including `cd examples/<name> && cargo run`
- `curl`, `jq` — HTTP testing and JSON parsing (all variations auto-approved)
- `timeout`, `pkill`, `killall` — process management for testing
- `git add` — staging changes (commit only when user explicitly requests)
- `rustc`, `sed`, `grep`, `find`, `awk`, `cat`, `tee`, `echo` — standard dev tools
- Background processes (`&`, `wait`, `jobs`)
- Shell control flow (`while`, `for`, `if`)

These commands do not require interactive permission prompts. Use normal judgment about context and timing.

### Commands requiring explicit user approval:
```bash
git checkout      # Discards uncommitted work
git restore       # Discards uncommitted work
git reset --hard  # Irreversible reset
git clean -f      # Deletes untracked files
cargo publish     # Pushes to crates.io (irreversible)
git commit        # Only when user explicitly requests a commit
```
**These commands destroy uncommitted work and are IRREVERSIBLE. Always ask the user first.**
