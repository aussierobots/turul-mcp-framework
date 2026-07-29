# Calculator Add — Derive Macro (Level 2)

Rung 2 of the four-rung tool-authoring ladder. All four rungs register the
*same* "add two numbers" tool through a different authoring API:

| Level | Example | Authoring API |
|---|---|---|
| 1 | `calculator-add-function-server` | `#[mcp_tool]` on an async fn |
| 2 | **calculator-add-simple-server-derive** (this one) | `#[derive(McpTool)]` on a struct |
| 3 | `calculator-add-builder-server` | `ToolBuilder` at runtime |
| 4 | `calculator-add-manual-server` | trait impls by hand |

## What this rung demonstrates over Level 1

- The tool is a **struct** whose fields are the arguments, so it can hold state
  and be constructed/cloned like any other type
- `#[tool(output = AdditionResult)]` gives the tool a **typed output struct**
  instead of a bare scalar
- The output schema is generated from `schemars::JsonSchema`, so nested
  structure and doc comments reach `tools/list` — see the `description` on
  `additionResult` in the response below
- `execute(&self, session)` is a plain inherent method; the derive macro wires
  it into `McpTool::call`

## The output field name

`#[derive(McpTool)]` derives the wrapper field name from the output **type**:
`AdditionResult` → `additionResult`. A bare scalar output would be `output`.
Set `output_field = "..."` on the `#[tool(...)]` attribute to choose your own.

> `outputSchema` comes exclusively from the `output = Type` attribute plus that
> type's `schemars::JsonSchema` impl. The derive macro cannot see `execute`'s
> return type, so **omitting `output = Type` yields no `outputSchema` at all**.

## Spec lane

MCP **2026-07-28** (the workspace default). Stateless core: no
`initialize`/`notifications/initialized` handshake and no `Mcp-Session-Id`.

## Run

```bash
cargo run -p calculator-add-simple-server-derive
# → http://127.0.0.1:8647/mcp
```

## Try it

```bash
META='"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"curl","version":"1.0"},"io.modelcontextprotocol/clientCapabilities":{}}'

curl -s -X POST http://127.0.0.1:8647/mcp \
  -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: tools/call' \
  -H 'Mcp-Name: calculator_add_derive' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{$META,\"name\":\"calculator_add_derive\",\"arguments\":{\"a\":5,\"b\":3}}}"
```

Expect `structuredContent` to be `{"additionResult":{"sum":8.0}}`.
