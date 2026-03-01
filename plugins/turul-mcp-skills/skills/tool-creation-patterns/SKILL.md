---
name: tool-creation-patterns
description: >
  This skill should be used when the user asks to "create a tool",
  "add a tool", "new tool", "which tool pattern", "compare tool patterns",
  "function macro vs derive", "mcp_tool macro", "#[mcp_tool]",
  "derive McpTool", "#[derive(McpTool)]", "ToolBuilder", "tool creation",
  or "function macro tool". Covers choosing between function macro
  (#[mcp_tool]), derive macro (#[derive(McpTool)]), and runtime builder
  (ToolBuilder) patterns in the Turul MCP Framework (Rust).
---

# Tool Creation Patterns — Turul MCP Framework

The framework provides three approaches to creating MCP tools, organized by complexity. Choose the simplest one that meets your requirements.

## Decision Flowchart

```
Need a tool?
├─ Tool definitions known at compile time? ───→ Use macros (L1 or L2)
│   ├─ Need per-session MCP state? ───────────→ Level 2: Derive Macro (#[derive(McpTool)])
│   └─ Otherwise ─────────────────────────────→ Level 1: Function Macro (#[mcp_tool])  ← DEFAULT
└─ Tools loaded from config/DB at runtime? ───→ Level 3: Builder (ToolBuilder)
```

**Start with Level 1 (function macro).** Most real-world tools — including those that query databases or call APIs — work with function macros. Shared application state (database pools, API clients) is passed via `OnceLock`, not closures. See [Shared Application State](#shared-application-state-oncelock) below.

## Level 1: Function Macro — `#[mcp_tool]` (Start Here)

**Best for:** Most tools. Simple, stateless functions with typed parameters.

```rust
// turul-mcp-server v0.3
use turul_mcp_derive::mcp_tool;
use turul_mcp_server::{McpResult, McpServer};

#[mcp_tool(
    name = "calculator_add",
    description = "Add two numbers",
    output_field = "sum"  // Optional: customize output JSON field (default: "result")
)]
async fn calculator_add(
    #[param(description = "First number")] a: f64,
    #[param(description = "Second number")] b: f64,
) -> McpResult<f64> {
    Ok(a + b)
}

// Register with .tool_fn()
let server = McpServer::builder()
    .name("my-server")
    .tool_fn(calculator_add)  // Note: .tool_fn() for function macros
    .build()?;
```

**Key points:**
- Parameters are extracted from the function signature automatically
- Use `#[param(description = "...")]` for parameter documentation
- Register with `.tool_fn(function_name)` (NOT `.tool()`)
- Output schema is auto-detected from the return type
- Schemars `JsonSchema` derive on the return type is auto-detected for detailed schemas

**See:** `references/function-macro-guide.md` for full details.

## Level 2: Derive Macro — `#[derive(McpTool)]`

**Best for:** Tools that need session access, complex state, or custom output types.

```rust
// turul-mcp-server v0.3
use turul_mcp_derive::McpTool;
use turul_mcp_server::{McpResult, McpServer, SessionContext};

#[derive(McpTool, Default)]
#[tool(
    name = "stateful_calc",
    description = "Calculator with history",
    output = CalculationResult  // REQUIRED for custom output types
)]
struct StatefulCalc {
    #[param(description = "First number")]
    a: f64,
    #[param(description = "Second number")]
    b: f64,
}

impl StatefulCalc {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<CalculationResult> {
        // Session access available here
        let result = CalculationResult { sum: self.a + self.b };
        Ok(result)
    }
}

// Register with .tool()
let server = McpServer::builder()
    .name("my-server")
    .tool(StatefulCalc::default())  // Note: .tool() for derive macros
    .build()?;
```

**Key points:**
- Implement an `async fn execute(&self, session: Option<SessionContext>) -> McpResult<T>` method
- `output = Type` attribute is **REQUIRED** when the output is not a primitive — derive macros cannot inspect the `execute` method's return type at compile time
- Register with `.tool(instance)` (NOT `.tool_fn()`)
- Session is `Option<SessionContext>` — `None` in stateless contexts

**See:** `references/derive-macro-guide.md` for full details.

## Level 3: Builder — `ToolBuilder`

**Best for:** Tools whose definitions are unknown at compile time (loaded from config files, databases, or plugin systems). Do NOT use Builder just because a tool needs a database connection — use `OnceLock` with macros instead.

```rust
// turul-mcp-server v0.3
use serde_json::json;
use turul_mcp_server::{McpServer, ToolBuilder};

let tool = ToolBuilder::new("dynamic_add")
    .description("Add two numbers dynamically")
    .number_param("a", "First number")
    .number_param("b", "Second number")
    .number_output()  // Generates {"result": number} schema
    .execute(|args| async move {
        let a = args.get("a").and_then(|v| v.as_f64())
            .ok_or("Missing parameter 'a'")?;
        let b = args.get("b").and_then(|v| v.as_f64())
            .ok_or("Missing parameter 'b'")?;
        Ok(json!({"result": a + b}))
    })
    .build()
    .map_err(|e| format!("Failed to build tool: {}", e))?;

// Register with .tool()
let server = McpServer::builder()
    .name("my-server")
    .tool(tool)  // Same as derive: .tool()
    .build()?;
```

**Key points:**
- Parameters defined with typed helpers: `.number_param()`, `.string_param()`, `.boolean_param()`
- No compile-time type safety on parameters — manual `args.get()` extraction
- Output schema set via `.number_output()`, `.string_output()`, `.object_output()`, or `.custom_output_schema()`
- Useful when tools are loaded from config files or databases

**See:** `references/builder-pattern-guide.md` for full details.

## Shared Application State (`OnceLock`)

**Most tools need shared dependencies** — database connections, API clients, configuration. Use `OnceLock<T>` for this. Do NOT use ToolBuilder just because a tool needs a database pool.

```rust
// turul-mcp-server v0.3
use std::sync::OnceLock;
use std::sync::Arc;
use sea_orm::DatabaseConnection;
use turul_mcp_derive::mcp_tool;
use turul_mcp_server::McpResult;
use turul_mcp_protocol::McpError;

// Module-level shared state — initialized once at startup
static DB: OnceLock<Arc<DatabaseConnection>> = OnceLock::new();

fn get_db() -> McpResult<&'static Arc<DatabaseConnection>> {
    DB.get().ok_or_else(|| McpError::tool_execution("Database not initialized"))
}

// Function macro tool that queries a database — NO Builder needed
#[mcp_tool(name = "get_profile", description = "Get user profile by username")]
async fn get_profile(
    #[param(description = "Username to look up")] username: String,
) -> McpResult<ProfileSummary> {
    let db = get_db()?;
    let profile = queries::latest_profile(db, &username).await
        .map_err(|e| McpError::tool_execution(e.to_string()))?;
    profile.ok_or_else(|| McpError::tool_execution(
        format!("No profile found for '{username}'")
    ))
}

// Initialize at startup, before building the server
DB.set(db_connection).expect("DB already initialized");

let server = McpServer::builder()
    .name("my-server")
    .tool_fn(get_profile)
    .build()?;
```

**This is the framework-idiomatic pattern.** Multiple framework examples use it: `audit-trail-server`, `dynamic-resource-server`, `elicitation-server`. The `OnceLock` is set once during startup and accessed by all macro-based tools.

**See:** `examples/shared-state-tool.rs` for a complete example.

## Task Support (Per-Tool)

Tools can declare `task_support` to enable long-running async execution via MCP tasks. This controls whether MCP Inspector shows a "Run as Task" button.

```rust
// Function macro
#[mcp_tool(name = "slow_op", description = "Long operation", task_support = "optional")]
async fn slow_op(input: String) -> McpResult<String> { /* ... */ }

// Derive macro
#[derive(McpTool)]
#[tool(name = "slow_calc", description = "Slow calc", task_support = "optional")]
struct SlowCalc { a: f64 }

// Builder
let tool = ToolBuilder::new("slow_tool")
    .execution(ToolExecution { task_support: Some(TaskSupport::Optional) })
    .build()?;
```

**Values:** `"optional"` (sync or async), `"required"` (must run as task), `"forbidden"` (never as task). Omit for no task support.

**Server requirement:** The server must have `.with_task_storage()` configured. `task_support = "required"` without a task runtime causes a build-time error.

## Quick Comparison

| Feature | Function Macro | Derive Macro | Builder |
|---|---|---|---|
| Complexity | Lowest | Medium | Highest |
| Type safety | Full | Full | Manual |
| Session access | Yes | Yes | No |
| Shared state (DB, API) | `OnceLock` | `OnceLock` | Closure capture |
| Output schema | Auto-detected | `output = Type` required | Explicit methods |
| Task support | `task_support = "..."` | `task_support = "..."` | `.execution()` |
| Registration | `.tool_fn()` | `.tool()` | `.tool()` |
| Best for | Most tools (default) | Per-session MCP state | Runtime-defined tools |

## Common Mistakes

1. **Using `ToolBuilder` for database-backed tools** — use function macros + `OnceLock` instead. Builder is only for tools whose definitions are unknown at compile time.
2. **Using `.tool()` for function macros** — use `.tool_fn(name)` instead
3. **Forgetting `output = Type` on derive macros** — schema will show inputs instead of outputs
4. **Putting `Arc<DatabaseConnection>` as a derive macro struct field** — all struct fields become MCP parameters. Use `OnceLock` for shared state.
5. **Creating `JsonRpcError` directly** — return `McpError` variants instead. See: [CLAUDE.md — Critical Error Handling Rules](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#critical-error-handling-rules)
6. **Adding method strings** — framework auto-determines from types. See: [CLAUDE.md — Zero-Configuration Design](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#zero-configuration-design)

## Beyond This Skill

**Resources or prompts?** → See the `resource-prompt-patterns` skill for `#[mcp_resource]`, `#[derive(McpResource)]`, `resource!{}`, `ResourceBuilder`, `#[derive(McpPrompt)]`, `prompt!{}`, and `PromptBuilder`.

**Output schemas, schemars, structuredContent?** → See the `output-schemas` skill.

**Client-side tool/resource/prompt invocation?** → See the `mcp-client-patterns` skill.

**Session state?** Use `session.get_typed_state(key).await` / `session.set_typed_state(key, value).await?`. See: [CLAUDE.md — API Conventions](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#api-conventions)

**Error handling?** Return `McpResult<T>` (alias for `Result<T, McpError>`). Never create `JsonRpcError` in handlers. See: [CLAUDE.md — Critical Error Handling Rules](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#critical-error-handling-rules)

**Server configuration?** Use `McpServer::builder()`. See: [CLAUDE.md — Basic Server](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#basic-server)

**Import hierarchy?** Prefer `turul_mcp_server::prelude::*`. See: [CLAUDE.md — Protocol Re-export Rule](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#protocol-re-export-rule-mandatory)
