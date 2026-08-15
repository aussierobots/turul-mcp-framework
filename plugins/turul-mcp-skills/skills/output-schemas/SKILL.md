---
name: output-schemas
description: >
  This skill should be used when the user asks about "output schema",
  "outputSchema", "structuredContent", "schemars", "JsonSchema derive",
  "output_field", "output = Type", "Vec output", "tool returns a struct",
  "output type", or "schema shows inputs not outputs". Covers the required
  output = Type attribute on derive macros, automatic schemars detection,
  Vec<T> output patterns, output_field customization, and structuredContent
  auto-generation in the Turul MCP Framework (Rust).
---

# Output Schemas — Turul MCP Framework

**Spec lane: MCP 2026-07-28 (current default).** For the 2025-11-25 opt-in build (`--no-default-features --features protocol-2025-11-25`), `structuredContent` must be an object and `inputSchema`/`outputSchema` are draft-07-shaped; the rest of this skill's mechanics (the `output = Type` gotcha, schemars detection, the Vec\<T\> wrapper-struct requirement) apply unchanged to both lanes.

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

## JSON Schema 2020-12 (2026-07-28)

2026-07-28 moves both schema surfaces to JSON Schema 2020-12:

- **`inputSchema`** stays object-rooted, but properties may now be arbitrary 2020-12 shapes — `oneOf`, `anyOf`, `allOf`, `$ref`, `$defs` are all valid inside a property, not just flat primitives/objects/arrays. `schemars::JsonSchema` derives already emit 2020-12-shaped output, so this is transparent if you're already using schemars.
- **`outputSchema`** is wire-unrestricted at the protocol level — a tool may declare an array, string, or scalar root schema (`ToolOutputSchema` in `turul-mcp-protocol-2026-07-28`), and `structuredContent` may be any JSON value, not just an object.

That wire-level flexibility does **not** currently reach the derive/function macro compile-time path — see the next section.

## Vec\<T\> Output — Still Use Wrapper Structs

**Do NOT return bare `Vec<T>` from tools, even under 2026-07-28.** Wrap arrays in a response struct:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchResult {
    pub title: String,
    pub score: f64,
}

// RECOMMENDED: Wrapper struct with Vec<T> field
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchResponse {
    /// The matching results
    pub results: Vec<SearchResult>,
    /// Optional pagination cursor
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

// Derive macro: output = SearchResponse (NOT Vec<SearchResult>)
#[derive(McpTool, Default)]
#[tool(
    name = "search",
    description = "Search items",
    output = SearchResponse
)]
struct SearchTool {
    #[param(description = "Search query")]
    query: String,
}

// Function macro: return SearchResponse
#[mcp_tool(name = "search_fn", description = "Search items")]
async fn search(
    #[param(description = "Search query")] query: String,
) -> McpResult<SearchResponse> {
    Ok(SearchResponse {
        results: vec![SearchResult { title: query, score: 1.0 }],
        next_cursor: None,
    })
}
```

### Why Not Bare Vec\<T\>?

The 2026-07-28 wire format *can* represent an array-root `outputSchema` (via `ToolOutputSchema`, which has no root-type constraint). The blocker is the framework's compile-time schema path, which is spec-version agnostic:

1. **`HasOutputSchema::output_schema()`** (generated by both the function and derive macros, in `turul-mcp-derive`) returns `Option<&ToolSchema>` — the same object-root-constrained type used for `inputSchema` — not the unrestricted `ToolOutputSchema`. This applies on both spec lanes; the macros don't currently branch on protocol version for output schema shape.
2. **`ToolSchema::from_schemars()`** (`turul-mcp-builders`) rejects any non-object root schema outright — `schema_for!(Vec<T>)` produces an array root and is rejected before it reaches the wire.
3. **Derive macro without schemars**: the static fallback schema can show `"type": "object"` instead of `"array"`, causing client-side validation failures (FastMCP, MCP Inspector).

Closing this gap — wiring `output = Type` through to `ToolOutputSchema` for array/scalar roots — is framework work, not something a tool author can opt into today.

**Wrapper structs work reliably with all tool patterns** (function macro, derive macro, builder) and additionally give you a natural place to add pagination fields (`next_cursor`, `total_count`).

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

Tools with `outputSchema` must provide `structuredContent` in the response. The framework handles this automatically:

1. If your tool declares an `outputSchema` (via `output = Type`, schemars, or builder schema methods), the framework generates `structuredContent` from your return value.
2. Just return the Rust type from `execute` — the framework serializes it into both `content` (text) and `structuredContent` (typed JSON).
3. **Never construct `structuredContent` yourself** in handler code.
4. Under 2026-07-28, `structuredContent` (`CallToolResult::structured_content` in `turul-mcp-protocol-2026-07-28`) is typed as `Option<Value>` — any JSON value, not just an object. This widening isn't yet reachable from `output = Type` (see [Vec\<T\> Output](#vect-output--still-use-wrapper-structs) above) but applies if you set `structuredContent` through lower-level builder/manual paths.

See: [CLAUDE.md — MCP Tool Output Compliance](https://github.com/aussierobots/turul-mcp-framework/blob/main/docs/rules/wire-format-compliance.md#mcp-tool-output-compliance)

## Complete Decision Table

| Scenario | Pattern | output Attribute | Schemars |
|---|---|---|---|
| Simple f64/String return | Function macro | Not needed | Optional |
| Custom struct return (fn macro) | Function macro | Not needed | Recommended |
| Custom struct return (derive) | Derive macro | **Required** | Recommended |
| Array return | Any | Use wrapper struct (e.g., `SearchResponse`) | Recommended |
| Dynamic/runtime | Builder | `.custom_output_schema()` | N/A |

**Array returns:** Always wrap `Vec<T>` in a response struct — the framework's compile-time output-schema path is still object-root-constrained. See [Vec\<T\> Output](#vect-output--still-use-wrapper-structs) above.

## Beyond This Skill

**Which tool pattern to use?** → See the `tool-creation-patterns` skill for choosing between function macro, derive, and builder.

**Server configuration?** Use `McpServer::builder()`. See: [CLAUDE.md — Basic Server](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#basic-server)

**Release validation of schemas?** Run `cargo test -p turul-mcp-derive schemars_integration_test` and `cargo test --test schema_tests mcp_vec_result_schema_test`. See: [AGENTS.md — Release Readiness Notes](https://github.com/aussierobots/turul-mcp-framework/blob/main/AGENTS.md#release-readiness-notes-2025-10-01)
