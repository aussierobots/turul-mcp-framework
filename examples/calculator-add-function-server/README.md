# Calculator Add — Function Macro (Level 1)

Rung 1 of the four-rung tool-authoring ladder. All four rungs register the
*same* "add two numbers" tool through a different authoring API, so you can
diff them against each other:

| Level | Example | Authoring API |
|---|---|---|
| 1 | **calculator-add-function-server** (this one) | `#[mcp_tool]` on an async fn |
| 2 | `calculator-add-simple-server-derive` | `#[derive(McpTool)]` on a struct |
| 3 | `calculator-add-builder-server` | `ToolBuilder` at runtime |
| 4 | `calculator-add-manual-server` | trait impls by hand |

## What this rung demonstrates

- `#[mcp_tool(name, description)]` turns an ordinary async fn into a tool
- `#[param(description = ...)]` supplies each argument's JSON Schema
- `output_field = "sum"` names the key in `outputSchema` and `structuredContent`
  (the default for a bare `f64` return is `output`)
- `.tool_fn(calculator_add)` registers it by the **original function name**

The whole tool is 11 lines:

```rust
#[mcp_tool(
    name = "calculator_add_function",
    description = "Add two numbers using function macro (Level 1)",
    output_field = "sum"
)]
async fn calculator_add(
    #[param(description = "First number")] a: f64,
    #[param(description = "Second number")] b: f64,
) -> McpResult<f64> {
    Ok(a + b)
}
```

## Spec lane

MCP **2026-07-28** (the workspace default). Stateless core: no
`initialize`/`notifications/initialized` handshake and no `Mcp-Session-Id` —
every request carries its own `_meta` and the `MCP-Protocol-Version` header.

## Run

```bash
cargo run -p calculator-add-function-server
# → http://127.0.0.1:8648/mcp
```

## Try it

```bash
META='"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"curl","version":"1.0"},"io.modelcontextprotocol/clientCapabilities":{}}'

curl -s -X POST http://127.0.0.1:8648/mcp \
  -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: tools/call' \
  -H 'Mcp-Name: calculator_add_function' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{$META,\"name\":\"calculator_add_function\",\"arguments\":{\"a\":5,\"b\":3}}}"
```

Expect:

```json
{"content":[{"text":"{\"sum\":8.0}","type":"text"}],"isError":false,"structuredContent":{"sum":8.0}}
```

`tools/list` advertises `outputSchema: {"properties":{"sum":{"type":"number"}},"required":["sum"]}`,
and the framework fills `structuredContent` to match it automatically.
