# Derive Macro Guide — `#[derive(McpTool)]`

The derive macro generates MCP tool boilerplate from a struct definition. It supports session access, complex state, and custom output types.

## Basic Usage

```rust
// turul-mcp-server v0.3
use turul_mcp_derive::McpTool;
use turul_mcp_server::{McpResult, SessionContext};

#[derive(McpTool, Default)]
#[tool(
    name = "calculator",
    description = "Perform arithmetic",
    output = CalculationResult  // REQUIRED for non-primitive outputs
)]
struct Calculator {
    #[param(description = "First number")]
    a: f64,
    #[param(description = "Second number")]
    b: f64,
}

impl Calculator {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<CalculationResult> {
        Ok(CalculationResult { sum: self.a + self.b })
    }
}
```

## Struct-Level Attributes

| Attribute | Required | Description |
|---|---|---|
| `name` | Yes | Tool name exposed via MCP `tools/list` |
| `description` | Yes | Human-readable description |
| `output` | **Required for non-primitives** | Output type for schema generation |
| `task_support` | No | Per-tool task support: `"optional"`, `"required"`, or `"forbidden"` |
| `title` | No | Display title → `Tool.title` (via `HasBaseMetadata`) |
| `annotation_title` | No | Title inside `ToolAnnotations.title` (rare, see note below) |
| `read_only` | No | `bool` → `readOnlyHint` in JSON (MCP default: `false`) |
| `destructive` | No | `bool` → `destructiveHint` in JSON (MCP default: `true`) |
| `idempotent` | No | `bool` → `idempotentHint` in JSON (MCP default: `false`) |
| `open_world` | No | `bool` → `openWorldHint` in JSON (MCP default: `true`) |

### Why `output = Type` Is Required

Rust proc macros operate at compile time on the struct definition. They **cannot inspect** the `execute` method's return type. Without `output = Type`, the macro has no way to know what schema to generate — and will fall back to generating a schema from the *input* struct fields, which is wrong.

```rust
// WRONG — schema will show {a: number, b: number} as OUTPUT
#[derive(McpTool)]
#[tool(name = "calc", description = "...")]
struct Calc { a: f64, b: f64 }

// CORRECT — schema shows {sum: number}
#[derive(McpTool)]
#[tool(name = "calc", description = "...", output = CalcResult)]
struct Calc { a: f64, b: f64 }
```

This is the most common gotcha with derive macros. For details, see the **output-schemas** skill.

## Field-Level Attributes

| Attribute | Description |
|---|---|
| `#[param(description = "...")]` | Parameter description in input schema |

Fields become required input parameters by default. Use `Option<T>` for optional parameters.

## The `execute` Method

The derive macro expects a method with this exact signature:

```rust
impl MyTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<OutputType> {
        // Your logic here
    }
}
```

**Key rules:**
- Method name must be `execute`
- Takes `&self` and `session: Option<SessionContext>`
- Returns `McpResult<T>` where `T` matches the `output` attribute type
- Session is `None` when no session context is available

## Session Access

The derive macro's main advantage over function macros is session access:

```rust
use turul_mcp_server::SessionContext;

impl StatefulTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
        if let Some(session) = session {
            // Read typed state
            let count: Option<u64> = session.get_typed_state("call_count").await;
            let new_count = count.unwrap_or(0) + 1;

            // Write typed state
            session.set_typed_state("call_count", new_count).await?;

            Ok(format!("Call #{}", new_count))
        } else {
            Ok("No session available".to_string())
        }
    }
}
```

See: [CLAUDE.md — API Conventions](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#api-conventions)

## Registration

Derive macro tools use `.tool()` with an instance:

```rust
let server = McpServer::builder()
    .name("my-server")
    .tool(Calculator::default())     // Needs an instance
    .tool(StatefulTool::default())   // Can chain multiple
    .build()?;
```

**Common mistake:** Using `.tool_fn()` — that's for function macros only.

## Output Types with Schemars

When your output type derives `schemars::JsonSchema`, the framework auto-detects it:

```rust
use schemars::JsonSchema;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct CalculationResult {
    sum: f64,
}

#[derive(McpTool, Default)]
#[tool(
    name = "calc",
    description = "Calculate",
    output = CalculationResult  // schemars auto-detected
)]
struct Calculator { a: f64, b: f64 }
```

For types that don't derive `JsonSchema`, the framework generates a basic schema from the `output` type's fields.

## Error Handling

Same rules as all tools — return `McpError`, never `JsonRpcError`:

```rust
use turul_mcp_protocol::McpError;

impl Calculator {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<CalculationResult> {
        if self.b == 0.0 {
            return Err(McpError::tool_execution("Division by zero"));
        }
        Ok(CalculationResult { sum: self.a / self.b })
    }
}
```

See: [CLAUDE.md — Critical Error Handling Rules](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#critical-error-handling-rules)

## Complete Example

See: `examples/derive-macro-tool.rs` in this skill, or the full server at [`examples/calculator-add-simple-server-derive/src/main.rs`](https://github.com/aussierobots/turul-mcp-framework/blob/main/examples/calculator-add-simple-server-derive/src/main.rs) in the framework repository.

## Task Support

Declare `task_support` to enable long-running async execution via MCP tasks:

```rust
#[derive(McpTool, Default)]
#[tool(
    name = "slow_calc",
    description = "Slow calculation",
    output = CalcResult,
    task_support = "optional"
)]
struct SlowCalc {
    #[param(description = "Input value")]
    value: f64,
}
```

**Values:** `"optional"` (sync or async), `"required"` (must run as task), `"forbidden"` (never as task). Omit the attribute for tools that don't support tasks.

**Server requirement:** The server must have `.with_task_storage()` configured for tools with task support. `task_support = "required"` without a task runtime causes a build-time error.

## Tool Annotations

Annotations are MCP 2025-11-25 hints that tell clients about a tool's behavior. All are optional — omitting them preserves `None` (backward compatible).

> **Not to be confused with resource/prompt `Annotations`** (which have `audience` and `priority`). Tool annotations use the separate `ToolAnnotations` type with hint fields.

```rust
#[derive(McpTool, Default)]
#[tool(
    name = "delete_file",
    description = "Delete a file from disk",
    title = "File Deleter",          // → Tool.title (primary display name)
    read_only = false,               // → readOnlyHint in JSON
    destructive = true,              // → destructiveHint in JSON
    idempotent = true,               // → idempotentHint in JSON
    open_world = false,              // → openWorldHint in JSON
)]
struct DeleteFileTool {
    #[param(description = "Path to delete")]
    path: String,
}
```

### `title` vs `annotation_title`

- `title = "..."` → sets `Tool.title` (the primary display name shown by MCP clients). This is the one you almost always want.
- `annotation_title = "..."` → sets `ToolAnnotations.title` only. Use this only if you need a different title inside the annotations object (rare).

Both can be set independently. If you only set one, use `title`.

### Macro Attributes → JSON Wire Format

Macros accept short snake_case attribute names. The framework generates camelCase JSON keys per MCP spec:

| Attribute | JSON key | Note |
|---|---|---|
| `read_only` | `readOnlyHint` | Serde rename on `ToolAnnotations` |
| `destructive` | `destructiveHint` | |
| `idempotent` | `idempotentHint` | |
| `open_world` | `openWorldHint` | |

Unset annotations are omitted from JSON entirely (via `skip_serializing_if`).

## Shared Application State (`OnceLock`)

For application-wide dependencies (database connections, API clients), use `OnceLock` — NOT session state. Session state is for per-session MCP data (shopping carts, user preferences).

```rust
use std::sync::OnceLock;

static DB: OnceLock<Arc<DatabaseConnection>> = OnceLock::new();

fn get_db() -> McpResult<&'static Arc<DatabaseConnection>> {
    DB.get().ok_or_else(|| McpError::tool_execution("Database not initialized"))
}

impl Calculator {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<CalcResult> {
        let db = get_db()?;  // Application state via OnceLock
        // ...
    }
}
```

**When to use session vs `OnceLock`:**

| State type | Pattern | Example |
|---|---|---|
| Application-wide (DB, config, API client) | `OnceLock<T>` | Database pool, API key |
| Per-session MCP state | `session.get_typed_state()` | Shopping cart, call counter |

**Important:** All derive macro struct fields become MCP input parameters. Do NOT put `Arc<DatabaseConnection>` or other dependencies as struct fields — they will appear in the tool's parameter schema.

## When to Use Builder Instead

Use Level 3 (builder) only when tools are **not known at compile time**:
- Loaded from configuration files at startup
- Created dynamically based on runtime conditions
- Part of a plugin system

Do NOT use Builder just because a tool needs a database connection — use `OnceLock` with macros instead.

See: `references/builder-pattern-guide.md`.
