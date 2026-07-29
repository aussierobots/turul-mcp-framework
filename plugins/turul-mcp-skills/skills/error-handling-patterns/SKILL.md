---
name: error-handling-patterns
description: >
  This skill should be used when the user asks about "error handling",
  "McpError", "McpResult", "tool_execution", "missing_param",
  "invalid_param_type", "param_out_of_range", "JsonRpcError",
  "error code", "JSON-RPC error", "ToolExecutionError",
  "ResourceExecutionError", "PromptExecutionError", "error conversion",
  "From McpError", "to_error_object", "error handling architecture",
  or "which error variant". Covers the MCP error handling architecture,
  McpError decision tree, error code mapping, and From conversions in
  the Turul MCP Framework (Rust).
---

**Spec lane: MCP 2026-07-28 (current default).** The 3-layer architecture and `McpError` constructors below are unchanged from 2025-11-25, but the **JSON-RPC Error Code Table further down was fully renumbered** — 2026-07-28 reserves `-32020..-32099` for spec-assigned codes (`MissingRequiredClientCapability` = -32021, `UnsupportedProtocolVersion` = -32022, a header-mismatch code = -32020), so the framework's own implementation-defined codes were moved down into `-32000..-32019` to get out of the way. Not-found errors (`ToolNotFound`/`ResourceNotFound`/`PromptNotFound`) also moved from their own codes to the JSON-RPC standard `-32602` (Invalid Params).

# Error Handling Patterns — Turul MCP Framework

The framework uses a 3-layer error architecture. Handlers return domain errors; the framework converts them to JSON-RPC wire format automatically. You never need to construct JSON-RPC errors directly.

## Error Handling Architecture

```
Handler code
    │  returns McpResult<T>  (= Result<T, McpError>)
    ▼
Framework (dispatcher)
    │  calls McpError::to_error_object()
    ▼
JsonRpcError  →  HTTP/Lambda response
```

**Your code only touches the top layer.** The framework owns the middle and bottom layers.

## The Golden Rule

**Handlers return `McpResult<T>` ONLY. NEVER create `JsonRpcError` directly.**

```rust
// CORRECT — return McpError
async fn my_tool(input: String) -> McpResult<String> {
    if input.is_empty() {
        return Err(McpError::missing_param("input"));
    }
    Ok(input.to_uppercase())
}

// WRONG — never do this in a handler
let error = JsonRpcError::new(...); // FORBIDDEN in handler code
```

## Choosing the Right Variant — Decision Tree

```
What went wrong?
├─ Parameter problem?
│   ├─ Missing entirely? ──────────→ McpError::missing_param("name")
│   ├─ Wrong type? ────────────────→ McpError::invalid_param_type("name", "string", "number")
│   └─ Out of valid range? ────────→ McpError::param_out_of_range("count", "150", "max 100")
├─ Execution failed?
│   ├─ In a tool handler? ─────────→ McpError::tool_execution("DB connection failed")
│   ├─ In a resource handler? ─────→ McpError::resource_execution("File not found")
│   └─ In a prompt handler? ───────→ McpError::prompt_execution("Template render failed")
├─ Entity not found?
│   ├─ Tool? ──────────────────────→ McpError::ToolNotFound("calc".into())
│   ├─ Resource? ──────────────────→ McpError::ResourceNotFound("file:///x".into())
│   └─ Prompt? ────────────────────→ McpError::PromptNotFound("greet".into())
├─ Validation/config problem?
│   ├─ Input validation? ──────────→ McpError::validation("Email format invalid")
│   └─ Server configuration? ──────→ McpError::configuration("Missing API key in env")
└─ Quick string error in a tool? ──→ "msg".into()  (converts to ToolExecutionError)
```

## Parameter Validation Errors

All three parameter error constructors map to JSON-RPC code **-32602** (Invalid params):

```rust
// turul-mcp-server v0.4
use turul_mcp_server::prelude::*;

#[mcp_tool(name = "search", description = "Search with filters")]
async fn search(
    #[param(description = "Search query")] query: String,
    #[param(description = "Max results (1-100)")] limit: Option<f64>,
) -> McpResult<Vec<String>> {
    if query.is_empty() {
        return Err(McpError::missing_param("query"));
    }

    let limit = limit.unwrap_or(10.0);

    if limit.fract() != 0.0 {
        return Err(McpError::invalid_param_type("limit", "integer", "float"));
    }

    if limit < 1.0 || limit > 100.0 {
        return Err(McpError::param_out_of_range(
            "limit",
            &limit.to_string(),
            "must be between 1 and 100",
        ));
    }

    Ok(vec![format!("Result for '{query}'")])
}
```

## Execution Errors

Use the handler-specific execution error for the context you're in:

```rust
// turul-mcp-server v0.4
use turul_mcp_server::prelude::*;

// In a tool handler — use tool_execution (-32010)
#[mcp_tool(name = "fetch_data", description = "Fetch data from API")]
async fn fetch_data(url: String) -> McpResult<String> {
    let body = reqwest::get(&url).await
        .map_err(|e| McpError::tool_execution(&e.to_string()))?;
    let text = body.text().await
        .map_err(|e| McpError::tool_execution(&e.to_string()))?;
    Ok(text)
}

// In a resource handler — use resource_execution (-32012)
async fn read_file(path: &str) -> McpResult<String> {
    std::fs::read_to_string(path)
        .map_err(|e| McpError::resource_execution(&e.to_string()))
}

// In a prompt handler — use prompt_execution (-32013)
async fn render_template(name: &str) -> McpResult<String> {
    templates::get(name)
        .ok_or_else(|| McpError::prompt_execution(&format!("Template '{name}' not found")))
}
```

## Automatic String Conversion

`McpError` implements `From<String>` and `From<&str>`, both producing `ToolExecutionError`:

```rust
// These are equivalent:
return Err("Something failed".into());
return Err(McpError::ToolExecutionError("Something failed".to_string()));
```

**Warning:** This implicit conversion ALWAYS produces `ToolExecutionError` (-32010). Use it only in tool handlers. In resource or prompt handlers, use the explicit `resource_execution()` or `prompt_execution()` constructors.

## The `?` Operator

Two error types have `From` impls and work with `?` directly:

```rust
// io::Error → McpError::IoError  (auto via From)
let content = std::fs::read_to_string("config.json")?;

// serde_json::Error → McpError::SerializationError  (auto via From)
let config: Config = serde_json::from_str(&content)?;
```

All other error types need `.map_err()`:

```rust
// reqwest::Error — no From impl, needs .map_err()
let response = reqwest::get(url).await
    .map_err(|e| McpError::tool_execution(&e.to_string()))?;

// sqlx::Error — no From impl
let row = sqlx::query("SELECT 1").fetch_one(&pool).await
    .map_err(|e| McpError::tool_execution(&e.to_string()))?;
```

## JSON-RPC Error Code Table

The schema reserves `-32000..-32019` as implementation-defined and `-32020..-32099` for its own sequential allocations. Framework codes below stay under `-32020` to leave the spec's range alone.

| McpError Variant | Code | Category |
|---|---|---|
| `MissingParameter` | -32602 | Invalid params (JSON-RPC standard) |
| `InvalidParameterType` | -32602 | Invalid params (JSON-RPC standard) |
| `ParameterOutOfRange` | -32602 | Invalid params (JSON-RPC standard) |
| `InvalidParameters` | -32602 | Invalid params (JSON-RPC standard) |
| `InvalidRequest` | -32602 | Invalid params (JSON-RPC standard) |
| `ToolNotFound` | -32602 | Invalid params (JSON-RPC standard) — unknown tool/prompt/resource names are "invalid parameters," not custom MCP errors |
| `ResourceNotFound` | -32602 | Invalid params (JSON-RPC standard) — **breaking change from 2025-11-25**, which used -32002 |
| `PromptNotFound` | -32602 | Invalid params (JSON-RPC standard) |
| `ToolExecutionError` | -32010 | Execution |
| `ResourceAccessDenied` | -32011 | Execution |
| `ResourceExecutionError` | -32012 | Execution |
| `PromptExecutionError` | -32013 | Execution |
| `ValidationError` | -32014 | Validation |
| `InvalidCapability` | -32015 | Validation |
| `VersionMismatch` | -32016 | Validation |
| `ConfigurationError` | -32017 | Config/Session |
| `SessionError` | -32018 | Config/Session |
| `TransportError` | -32019 | Transport |
| `JsonRpcProtocolError` | -32000 | Transport |
| `IoError` | -32603 | Internal (JSON-RPC standard internal error) |
| `SerializationError` | -32603 | Internal (JSON-RPC standard internal error) |
| `MissingRequiredClientCapability` | -32021 | Spec-assigned (2026-07-28) — client didn't declare a capability the server requires |
| `UnsupportedProtocolVersion` | -32022 | Spec-assigned (2026-07-28) — request's protocol version unsupported |
| `JsonRpcError { code, .. }` | pass-through | Preserves the original code/message/data verbatim (e.g. `tasks/result` reproducing the task's original error) |

Header/body protocol-version disagreement uses a separate spec-assigned constant, `ERROR_CODE_HEADER_MISMATCH = -32020` (not a named `McpError` variant — surfaced directly from the header-validation layer).

**See:** `references/mcperror-reference.md` for all variants with full constructor signatures.

## Common Mistakes

1. **Creating `JsonRpcError` directly in handlers** — Always return `McpError` variants. The framework converts automatically.

2. **Using `"err".into()` in a resource handler** — `From<&str>` produces `ToolExecutionError`, not `ResourceExecutionError`. Use `McpError::resource_execution("err")` explicitly.

3. **Using `.unwrap()` instead of `?`** — `.unwrap()` panics on error. Use `?` with `.map_err()` to convert to `McpError`.

4. **Using `InvalidRequest` for parameter issues** — `InvalidRequest` is for malformed JSON-RPC requests. For bad parameter values, use `missing_param`, `invalid_param_type`, or `param_out_of_range`.

5. **Using `ToolNotFound` inside a tool handler** — `ToolNotFound` is for the framework's dispatcher. Inside your tool, use `tool_execution("resource X not available")` for domain-level not-found errors.

6. **Missing context in error messages** — Include the parameter name, value, or constraint in error messages. Bad: `"invalid"`. Good: `"Parameter 'count' must be between 1 and 100, got 150"`.

## Beyond This Skill

**Middleware errors?** → See the `middleware-patterns` skill for `MiddlewareError` variants and the middleware conversion chain.

**Creating tools?** → See the `tool-creation-patterns` skill.

**Client-side error handling?** → See the `mcp-client-patterns` skill for `McpClientError` variants.
