# Function Macro Guide — `#[mcp_tool]`

The function macro is the simplest way to create MCP tools. Annotate an async function, and the framework generates all boilerplate: input schema, output schema, parameter extraction, and registration.

## Basic Usage

```rust
// turul-mcp-server v0.3.0
use turul_mcp_derive::mcp_tool;
use turul_mcp_server::McpResult;

#[mcp_tool(
    name = "greet",
    description = "Greet someone by name"
)]
async fn greet(
    #[param(description = "Person's name")] name: String,
) -> McpResult<String> {
    Ok(format!("Hello, {}!", name))
}
```

## Attributes

### Tool-Level Attributes

| Attribute | Required | Description |
|---|---|---|
| `name` | Yes | Tool name exposed via MCP `tools/list` |
| `description` | Yes | Human-readable description for MCP clients |
| `output_field` | No | JSON field name wrapping the output (default: `"result"`) |
| `task_support` | No | Per-tool task support: `"optional"`, `"required"`, or `"forbidden"` |

### Parameter Attributes

| Attribute | Required | Description |
|---|---|---|
| `#[param(description = "...")]` | Recommended | Parameter description for the input schema |

## Supported Parameter Types

The macro extracts parameters from the function signature. Supported types:

| Rust Type | JSON Schema Type | Notes |
|---|---|---|
| `String` | `string` | |
| `f64`, `f32` | `number` | |
| `i32`, `i64`, `u32`, `u64` | `integer` | |
| `bool` | `boolean` | |
| `Option<T>` | T (not required) | Field becomes optional in schema |
| `Vec<T>` | `array` | Items typed to T |
| `serde_json::Value` | `object` | Arbitrary JSON |

## Output Schema

The output schema is **automatically detected** from the return type:

```rust
// Primitive output — schema auto-generated
#[mcp_tool(name = "add", description = "Add numbers")]
async fn add(a: f64, b: f64) -> McpResult<f64> {
    Ok(a + b)
}

// Custom struct output — schemars auto-detected if derived
#[derive(serde::Serialize, schemars::JsonSchema)]
struct CalcResult { value: f64, operation: String }

#[mcp_tool(name = "calc", description = "Calculate")]
async fn calc(a: f64, b: f64) -> McpResult<CalcResult> {
    Ok(CalcResult { value: a + b, operation: "add".into() })
}
```

When the return type derives `schemars::JsonSchema`, the framework uses it automatically for detailed schema generation. No additional attributes needed.

For more on output schemas, see the **output-schemas** skill.

## Registration

Function macro tools use `.tool_fn()`:

```rust
let server = McpServer::builder()
    .name("my-server")
    .tool_fn(greet)       // Use the original function name
    .tool_fn(add)         // Can chain multiple
    .build()?;
```

**Common mistake:** Using `.tool()` instead of `.tool_fn()`. The `.tool()` method is for derive macro and builder instances.

## Customizing the Output Field

By default, the tool result is wrapped in `{"result": <value>}`. Customize with `output_field`:

```rust
#[mcp_tool(
    name = "calculator_add",
    description = "Add two numbers",
    output_field = "sum"  // Output: {"sum": 5.0}
)]
async fn calculator_add(a: f64, b: f64) -> McpResult<f64> {
    Ok(a + b)
}
```

## Error Handling

Return `McpError` variants — never construct `JsonRpcError` directly:

```rust
use turul_mcp_protocol::McpError;

#[mcp_tool(name = "divide", description = "Divide two numbers")]
async fn divide(a: f64, b: f64) -> McpResult<f64> {
    if b == 0.0 {
        return Err(McpError::tool_execution("Cannot divide by zero"));
    }
    Ok(a / b)
}
```

See: [CLAUDE.md — Critical Error Handling Rules](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#critical-error-handling-rules)

## Complete Example

See: `examples/function-macro-tool.rs` in this skill, or the full server at [`examples/calculator-add-function-server/src/main.rs`](https://github.com/aussierobots/turul-mcp-framework/blob/main/examples/calculator-add-function-server/src/main.rs) in the framework repository.

## Task Support

Declare `task_support` to enable long-running async execution via MCP tasks:

```rust
#[mcp_tool(
    name = "slow_process",
    description = "Process data with delay",
    task_support = "optional"  // Clients can run sync or as a task
)]
async fn slow_process(input: String) -> McpResult<String> {
    tokio::time::sleep(Duration::from_secs(10)).await;
    Ok(format!("Processed: {}", input))
}
```

**Values:** `"optional"` (sync or async), `"required"` (must run as task), `"forbidden"` (never as task). Omit the attribute for tools that don't support tasks.

**Server requirement:** The server must have `.with_task_storage()` configured for tools with task support.

## Limitations

- **No session access** — function macros cannot receive `SessionContext`. Use the derive macro if you need session state.
- **No struct state** — the function is stateless. If you need to share state between calls, use the derive macro with a struct.

When these limitations matter, upgrade to Level 2 (derive macro). See: `references/derive-macro-guide.md`.
