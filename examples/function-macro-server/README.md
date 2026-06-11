# Function Macro Server

Demonstrates the `#[mcp_tool]` **function attribute macro**: write an ordinary
async function, annotate its parameters with `#[param]`, and the framework
generates the tool struct, input schema, and registration glue.

## Tools (as registered by `src/main.rs`)

| Tool | Signature highlights |
|---|---|
| `add` | Two `f64` params |
| `string_repeat` | `String` + `i32` with range validation (`McpError::param_out_of_range`) |
| `boolean_logic` | `bool` params + string-enum operation (`and`/`or`/`xor`) |
| `greet` | `Option<String>` optional param via `#[param(optional)]` |

## Run

```bash
cargo run -p function-macro-server
# → http://127.0.0.1:8003/mcp
```

## Try it (2026-07-28 stateless)

```bash
META='"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"curl","version":"1.0"},"io.modelcontextprotocol/clientCapabilities":{}}'

curl -s -X POST http://127.0.0.1:8003/mcp \
  -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: tools/call' \
  -H 'Mcp-Name: add' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{$META,\"name\":\"add\",\"arguments\":{\"a\":5,\"b\":3}}}"
```

## Writing a function tool

```rust
#[mcp_tool(name = "add", description = "Add two numbers together")]
async fn add_numbers(
    #[param(description = "First number")] a: f64,
    #[param(description = "Second number")] b: f64,
) -> McpResult<String> {
    Ok(format!("{} + {} = {}", a, b, a + b))
}
```

- `#[mcp_tool(name, description)]` on the function generates a `<Name>ToolImpl`
  struct registered via `.tool(AddNumbersToolImpl)`
- `#[param(description = ...)]` feeds each parameter's JSON Schema
- `#[param(optional)]` + `Option<T>` makes a parameter optional
- Return `McpResult<T>`; domain errors like `McpError::param_out_of_range`
  map to JSON-RPC errors automatically

## See also

- `calculator-add-function-server` — the calculator-ladder rung for this
  macro, including `output_field`
- `derive-macro-server` — the `#[derive(McpTool)]` struct alternative
