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

### Test Compliance

**Tests validate the MCP spec and intended contract — never change tests to preserve buggy behavior.**

- When code and tests disagree, verify against the MCP specification before changing either
- Never silently accept multiple wire formats in tests (e.g., `.strip_prefix("data: ")` to handle both SSE and JSON) — assert the expected Content-Type and body format explicitly
- Tests must assert wire-format compliance: Content-Type headers, HTTP status codes, JSON-RPC error codes, and response body shape

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

### Workspace Dependencies
All crate dependencies MUST use `workspace = true` references. Declare versions in root `Cargo.toml` `[workspace.dependencies]`, reference with `.workspace = true` in crate `Cargo.toml`. Add crate-specific features inline: `hyper = { workspace = true, features = ["http1"] }`.

### Feature Flags — Storage Backends
Default features: `["http", "sse"]` — in-memory only, no backend deps compiled. Storage backends are opt-in:

```toml
# In-memory only (default)
turul-mcp-server = "0.3"

# With DynamoDB backends
turul-mcp-server = { version = "0.3", features = ["dynamodb"] }

# With DynamoDB + dynamic tools
turul-mcp-server = { version = "0.3", features = ["dynamodb", "dynamic-tools"] }
```

Backend features (`sqlite`, `postgres`, `dynamodb`) forward to both `turul-mcp-session-storage` AND `turul-mcp-task-storage`. When `dynamic-tools` is enabled, they also forward to `turul-mcp-server-state-storage` via weak dep syntax (`?/`).

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

### Notification Wire Format: Always Use JsonRpcNotification

**CRITICAL**: Protocol notification types (e.g., `ToolListChangedNotification`, `ResourceListChangedNotification`) are **NOT wire-complete**. They contain MCP-specific fields (`method`, `params`) but lack the required `jsonrpc: "2.0"` envelope.

```rust
// CORRECT — wire-complete JSON-RPC notification for transport:
let notification = JsonRpcNotification::new("notifications/tools/list_changed".to_string());
// Serializes to: {"jsonrpc":"2.0","method":"notifications/tools/list_changed"}

// WRONG — missing jsonrpc field, will fail client-side validation:
let notification = ToolListChangedNotification::new();
// Serializes to: {"method":"notifications/tools/list_changed"}  ← BROKEN
```

This applies to ALL notification types sent via SSE/HTTP transport. The protocol `*Notification` types are for parsing/type safety, not for direct wire emission.

### Notification Persistence Architecture

**SessionManager is the single event bus.** All notification emitters (ToolRegistry, SessionContext) go through `SessionManager::broadcast_event()`. Guaranteed persistence is provided by the `SessionEventDispatcher` — an awaited trait installed at the SessionManager layer, not at individual emitters.

- `broadcast_event()` for Custom events enumerates targets from `storage.list_sessions()` (NOT the in-memory cache), filters terminated sessions, dispatches per-session via the awaited dispatcher
- `dispatch_custom_event(session_id)` is for per-session delivery (e.g., fingerprint mismatch) — storage-backed, not cache-gated
- `send_event_to_session()` is cache-backed (unchanged) — used only when the session is known to be attached in this process
- The dispatcher calls `StreamManager::broadcast_to_session()` which persists to session event storage AND delivers to active connections
- The SSE bridge task is observer-only for Custom events — NOT the persistence path
- Without a dispatcher (e.g., no HTTP server), events are best-effort only (in-memory channels)

**Do NOT add notification sinks or persistence hooks to individual emitters** — that splits the event architecture into competing delivery paths.

**Distributed session targeting** (see ADR-023): In Lambda/multi-instance, the in-memory `SessionManager.sessions` cache may not contain sessions created by other instances. Notification targeting for Custom events uses `storage.list_sessions()`, not the cache.

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

**Session Status Codes (Streamable HTTP):**
- Missing `Mcp-Session-Id` header → **401** (no session ID provided at all)
- Nonexistent session ID → **404** (MCP spec: client must start fresh `initialize`)
- Terminated session ID → **404** (MCP spec: treated same as nonexistent)
- Auth token invalid/expired → **401** (OAuth middleware, separate from session)
- Storage backend error → **500**

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
// Manual impls MUST include: impl HasExecution for MyTool {}
```

### Task Support (per-tool)

Tools can declare `task_support` to enable the "Run as Task" button in MCP Inspector:

```rust
// Function macro
#[mcp_tool(name = "slow_add", description = "Add with delay", task_support = "optional")]
async fn slow_add(a: f64, b: f64) -> McpResult<f64> { Ok(a + b) }

// Derive macro
#[derive(McpTool)]
#[tool(name = "slow_calc", description = "Slow calc", task_support = "optional")]
struct SlowCalcTool { a: f64 }
```

**Values**: `"optional"` (sync or async), `"required"` (must run as task), `"forbidden"` (never as task). Omit for no task support.

**Server requirement**: The server must have a task runtime configured (`.with_task_storage()`) for tools with task support. `task_support = "required"` without a runtime causes a build-time error.

**Manual impls**: Override `HasExecution::execution()` to return `Some(ToolExecution { task_support: Some(TaskSupport::Optional) })`.

**Capability truthfulness**: When no task runtime is configured, the server strips `execution` from `tools/list` responses and rejects task-augmented `tools/call` requests.

### Tool Annotations (per-tool)

MCP 2025-11-25 behavior hints. All attributes are optional — omit for `None`.

```rust
// Function macro
#[mcp_tool(name = "search", description = "Search the web",
           title = "Web Search", read_only = true, open_world = true)]
async fn search(query: String) -> McpResult<String> { Ok(query) }

// Derive macro
#[derive(McpTool)]
#[tool(name = "delete_file", description = "Delete a file",
       title = "File Deleter", read_only = false, destructive = true,
       idempotent = true, open_world = false)]
struct DeleteFileTool { path: String }

// Builder
let tool = ToolBuilder::new("delete")
    .annotations(ToolAnnotations::new()
        .with_read_only_hint(false)
        .with_destructive_hint(true))
    .build()?;
```

**Attributes**: `title` (→ `Tool.title`), `annotation_title` (→ `ToolAnnotations.title`, rare), `read_only` (→ `readOnlyHint`), `destructive` (→ `destructiveHint`), `idempotent` (→ `idempotentHint`), `open_world` (→ `openWorldHint`).

**Not `Annotations`**: Tool annotations (`ToolAnnotations`) are separate from resource/prompt `Annotations` (`audience`/`priority`).

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

# Specific test suites
cargo test -p turul-mcp-server --features dynamic-tools     # Dynamic tools + registry tests
cargo test -p turul-mcp-framework-integration-tests --test event_dispatcher_persistence  # Notification persistence
cargo test -p turul-mcp-framework-integration-tests --test compliance  # MCP spec compliance

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

### Scope Discipline

- **Stay inside the approved plan and stated requirement** — do not broaden scope by changing adjacent contracts, tests, or semantics unless directly required
- **If a fix forces unrelated API behavior changes or test expectation changes, stop and reassess** — that's a signal you're modifying the wrong layer
- **If scope or architecture becomes ambiguous, stop and ask** — do not improvise
- **`replace_all` edits must be scoped precisely** — never use `replace_all` on patterns that appear in unrelated code paths

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

## Pre-Release Checklist

Before publishing a new version:

1. **Workspace version**: Update `version` in `Cargo.toml` `[workspace.package]` and all internal crate dependency versions
2. **Example server versions**: Update `.version("x.y.z")` strings in `examples/*/src/main.rs`
3. **Plugin skill versions**: Skills use generic minor version (`v0.3`, not `v0.3.13`) — do NOT bump on patch releases. Only update when the minor version changes (e.g., `v0.3` → `v0.4`).
4. **CHANGELOG.md**: Add release entry with date and comparison links
5. **Stale version scan**: `grep -rn 'v0\.[0-9]\.[0-9]' plugins/ examples/ .claude/` — fix any outdated references
6. **Publish order** (dependency-first):
   ```
   json-rpc-server → protocol-2025-06-18 → protocol-2025-11-25 → protocol → builders →
   session-storage → task-storage → server-state-storage → derive* → http-server → server → client → aws-lambda → oauth
   ```
   *`turul-mcp-derive` has circular dev-deps on `turul-mcp-server` — temporarily comment out dev-deps, publish with `--allow-dirty`, restore*
7. **Git tag**: `git tag v0.x.y && git push origin v0.x.y`

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
- `turul-mcp-server-state-storage/` - Server-global state for dynamic tool coordination
- `turul-mcp-aws-lambda/` - AWS Lambda integration
- `turul-mcp-oauth/` - OAuth 2.1 Resource Server support

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

### Commit Message Style
- **No `Co-Authored-By` attribution** — omit Claude/AI co-author trailers
- **Succinct** — one-line summary, optional body only if non-obvious
