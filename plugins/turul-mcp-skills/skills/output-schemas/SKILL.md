---
name: output-schemas
description: >
  This skill should be used when the user asks about "output schema",
  "structuredContent", "schemars", "output_field", "Vec output", or
  "schema shows inputs not outputs". Covers the required output = Type
  attribute on derive macros, automatic schemars detection, Vec<T> output
  patterns, output_field customization, and structuredContent auto-generation
  in the Turul MCP Framework (Rust).
triggers:
  - "output schema"
  - "structuredContent"
  - "schemars"
  - "output_field"
  - "Vec output"
  - "tool returns a struct"
  - "schema shows inputs not outputs"
  - "output type"
  - "JsonSchema derive"
---

# Output Schemas — Turul MCP Framework

MCP tools can declare an output schema so clients know the shape of the result. The framework auto-generates `structuredContent` when an output schema exists — never create it manually.

## The #1 Gotcha: `output = Type` on Derive Macros

**Problem:** Your tool's `tools/list` response shows the *input* parameters as the output schema instead of the actual return type.

**Cause:** Derive macros operate on the struct definition at compile time. They cannot inspect the `execute` method's return type.

**Fix:** Add the `output` attribute:

```rust
// WRONG — schema shows {a: number, b: number} as output
#[derive(McpTool)]
#[tool(name = "calc", description = "Calculate")]
struct Calc { a: f64, b: f64 }

// CORRECT — schema shows {sum: number}
#[derive(McpTool)]
#[tool(name = "calc", description = "Calculate", output = CalcResult)]
struct Calc { a: f64, b: f64 }
```

**Function macros (`#[mcp_tool]`) do NOT need this** — they auto-detect the return type.

See: [CLAUDE.md — Output Types and Schemas](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#output-types-and-schemas)

## Schemars Auto-Detection

When your output type derives `schemars::JsonSchema`, the framework automatically generates a detailed JSON schema including nested objects, arrays, and optional fields:

```rust
use schemars::JsonSchema;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CalculationResult {
    /// The result of the calculation
    pub value: f64,
    /// The operation that was performed
    pub operation: String,
}
```

### How Detection Works

- **Function macros** (`#[mcp_tool]`): Automatically detected from the return type. If the return type derives `JsonSchema`, the detailed schema is used.
- **Derive macros** (`#[derive(McpTool)]`): Detected from the `output = Type` attribute. The type must derive `JsonSchema`.

No additional flags or attributes are needed — just derive `JsonSchema` on your output type.

### Required Derives

For schemars to work, your output type needs:

```rust
#[derive(
    Debug,                    // Standard
    Clone,                    // Standard
    serde::Serialize,         // Required for JSON serialization
    serde::Deserialize,       // Required for JSON deserialization
    schemars::JsonSchema,     // Enables detailed schema generation
)]
struct MyOutput {
    pub value: f64,
}
```

See: `references/schemars-integration.md` for advanced schemars patterns.

## Vec\<T\> Output Pattern

Tools that return arrays need special attention:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchResult {
    pub title: String,
    pub score: f64,
}

// Derive macro: specify Vec<SearchResult> as the output type
#[derive(McpTool, Default)]
#[tool(
    name = "search",
    description = "Search items",
    output = Vec<SearchResult>  // Vec<T> is supported directly
)]
struct SearchTool {
    #[param(description = "Search query")]
    query: String,
}

// Function macro: auto-detected from return type
#[mcp_tool(name = "search_fn", description = "Search items")]
async fn search(
    #[param(description = "Search query")] query: String,
) -> McpResult<Vec<SearchResult>> {
    Ok(vec![SearchResult { title: query, score: 1.0 }])
}
```

The generated schema will have `"type": "array"` with `"items"` containing the item schema.

## output_field Customization

By default, the tool result is wrapped in `{"result": <value>}`. Customize with `output_field`:

```rust
// Function macro
#[mcp_tool(
    name = "word_count",
    description = "Count words",
    output_field = "countResult"  // Output: {"countResult": 42}
)]
async fn word_count(text: String) -> McpResult<usize> {
    Ok(text.split_whitespace().count())
}
```

The `output_field` affects the JSON key name in the `structuredContent` response.

## structuredContent — Never Create Manually

The MCP 2025-11-25 spec requires that tools with `outputSchema` provide `structuredContent` in the response. The framework handles this automatically:

1. If your tool declares an `outputSchema` (via `output = Type`, schemars, or builder schema methods), the framework generates `structuredContent` from your return value.
2. Just return the Rust type from `execute` — the framework serializes it into both `content` (text) and `structuredContent` (typed JSON).
3. **Never construct `structuredContent` yourself** in handler code.

See: [CLAUDE.md — MCP Tool Output Compliance](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#mcp-tool-output-compliance)

## Complete Decision Table

| Scenario | Pattern | output Attribute | Schemars |
|---|---|---|---|
| Simple f64/String return | Function macro | Not needed | Optional |
| Custom struct return (fn macro) | Function macro | Not needed | Recommended |
| Custom struct return (derive) | Derive macro | **Required** | Recommended |
| Vec\<T\> return (fn macro) | Function macro | Not needed | Recommended on T |
| Vec\<T\> return (derive) | Derive macro | **Required**: `output = Vec<T>` | Recommended on T |
| Dynamic/runtime | Builder | `.custom_output_schema()` | N/A |

## Beyond This Skill

**Which tool pattern to use?** → See the `tool-creation-patterns` skill for choosing between function macro, derive, and builder.

**Server configuration?** Use `McpServer::builder()`. See: [CLAUDE.md — Basic Server](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#basic-server)

**Release validation of schemas?** Run `cargo test -p turul-mcp-derive schemars_integration_test` and `cargo test --test schema_tests mcp_vec_result_schema_test`. See: [AGENTS.md — Release Readiness Notes](https://github.com/aussierobots/turul-mcp-framework/blob/main/AGENTS.md#release-readiness-notes-2025-10-01)
