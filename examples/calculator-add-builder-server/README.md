# Calculator Add — Runtime Builder (Level 3)

Rung 3 of the four-rung tool-authoring ladder. All four rungs register the
*same* "add two numbers" tool through a different authoring API:

| Level | Example | Authoring API |
|---|---|---|
| 1 | `calculator-add-function-server` | `#[mcp_tool]` on an async fn |
| 2 | `calculator-add-simple-server-derive` | `#[derive(McpTool)]` on a struct |
| 3 | **calculator-add-builder-server** (this one) | `ToolBuilder` at runtime |
| 4 | `calculator-add-manual-server` | trait impls by hand |

## What this rung demonstrates that the macros cannot

Levels 1 and 2 fix the tool's name, description and schema **at compile time**.
`ToolBuilder` composes all of that from ordinary values, so the tool set can be
read from config, a database, or a remote registry at startup:

```rust
let add_tool = ToolBuilder::new("calculator_add_builder")
    .description("Add two numbers using builder pattern (Level 3)")
    .number_param("a", "First number")
    .number_param("b", "Second number")
    .number_output()                      // → {"result": number} outputSchema
    .execute(|args| async move {
        let a = args.get("a").and_then(|v| v.as_f64())
            .ok_or("Missing or invalid parameter 'a'")?;
        // ...
        Ok(json!({"result": a + b}))
    })
    .build()?;
```

Trade-off: the closure receives raw `serde_json::Value` arguments, so you do
your own extraction and your own error strings. The macros do that for you.

`.number_output()` declares the `outputSchema`, which is what makes the
framework emit `structuredContent`; the closure must return a value matching it.

## Spec lane

MCP **2026-07-28** (the workspace default). Stateless core: no
`initialize`/`notifications/initialized` handshake and no `Mcp-Session-Id`.

## Run

```bash
cargo run -p calculator-add-builder-server
# → http://127.0.0.1:8649/mcp
```

## Try it

```bash
META='"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"curl","version":"1.0"},"io.modelcontextprotocol/clientCapabilities":{}}'

# happy path → structuredContent {"result":8.0}
curl -s -X POST http://127.0.0.1:8649/mcp \
  -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: tools/call' -H 'Mcp-Name: calculator_add_builder' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{$META,\"name\":\"calculator_add_builder\",\"arguments\":{\"a\":5,\"b\":3}}}"

# drop "b" → the closure's own error, surfaced as JSON-RPC -32010
curl -s -X POST http://127.0.0.1:8649/mcp \
  -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: tools/call' -H 'Mcp-Name: calculator_add_builder' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\"params\":{$META,\"name\":\"calculator_add_builder\",\"arguments\":{\"a\":5}}}"
```

## See also

`examples/dynamic-tools-server` goes further: it activates and deactivates
registered tools while the server is running and emits
`notifications/tools/list_changed`.
