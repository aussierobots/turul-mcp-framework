---
name: tool-creation-patterns
description: >
  Choose the right tool creation approach for Turul MCP Framework (Rust).
  Covers function macro (#[mcp_tool]), derive macro (#[derive(McpTool)]),
  and runtime builder (ToolBuilder) patterns. Use when creating MCP tools,
  choosing between tool patterns, or understanding macro vs builder tradeoffs.
triggers:
  - "create a tool"
  - "mcp_tool macro"
  - "derive McpTool"
  - "ToolBuilder"
  - "which tool pattern"
  - "add a tool"
  - "function macro tool"
  - "tool creation"
  - "new tool"
---

# Tool Creation Patterns — Turul MCP Framework

The framework provides three approaches to creating MCP tools, organized by complexity. Choose the simplest one that meets your requirements.

## Decision Flowchart

```
Need a tool?
├─ Stateless + simple params? ──────────→ Level 1: Function Macro (#[mcp_tool])
├─ Need session access or struct state? ─→ Level 2: Derive Macro (#[derive(McpTool)])
└─ Dynamic/runtime tool construction? ──→ Level 3: Builder (ToolBuilder)
```

## Level 1: Function Macro — `#[mcp_tool]` (Start Here)

**Best for:** Most tools. Simple, stateless functions with typed parameters.

```rust
// turul-mcp-server v0.3.0
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
// turul-mcp-server v0.3.0
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

**Best for:** Dynamic tools, runtime configuration, plugin systems.

```rust
// turul-mcp-server v0.3.0
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

## Quick Comparison

| Feature | Function Macro | Derive Macro | Builder |
|---|---|---|---|
| Complexity | Lowest | Medium | Highest |
| Type safety | Full | Full | Manual |
| Session access | No | Yes | No |
| Output schema | Auto-detected | `output = Type` required | Explicit methods |
| Registration | `.tool_fn()` | `.tool()` | `.tool()` |
| Best for | Simple tools | Stateful tools | Dynamic tools |

## Common Mistakes

1. **Using `.tool()` for function macros** — use `.tool_fn(name)` instead
2. **Forgetting `output = Type` on derive macros** — schema will show inputs instead of outputs
3. **Creating `JsonRpcError` directly** — return `McpError` variants instead. See: [CLAUDE.md — Critical Error Handling Rules](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#critical-error-handling-rules)
4. **Adding method strings** — framework auto-determines from types. See: [CLAUDE.md — Zero-Configuration Design](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#zero-configuration-design)

## Beyond This Skill

**Output schemas, schemars, structuredContent?** → See the `output-schemas` skill.

**Session state?** Use `session.get_typed_state(key).await` / `session.set_typed_state(key, value).await?`. See: [CLAUDE.md — API Conventions](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#api-conventions)

**Error handling?** Return `McpResult<T>` (alias for `Result<T, McpError>`). Never create `JsonRpcError` in handlers. See: [CLAUDE.md — Critical Error Handling Rules](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#critical-error-handling-rules)

**Server configuration?** Use `McpServer::builder()`. See: [CLAUDE.md — Basic Server](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#basic-server)

**Import hierarchy?** Prefer `turul_mcp_server::prelude::*`. See: [CLAUDE.md — Protocol Re-export Rule](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#protocol-re-export-rule-mandatory)
