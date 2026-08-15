# Calculator Add — Manual Trait Impls (Level 4)

Rung 4 of the four-rung tool-authoring ladder — the reference for what the
macros generate. All four rungs register the *same* "add two numbers" tool:

| Level | Example | Authoring API |
|---|---|---|
| 1 | `calculator-add-function-server` | `#[mcp_tool]` on an async fn |
| 2 | `calculator-add-simple-server-derive` | `#[derive(McpTool)]` on a struct |
| 3 | `calculator-add-builder-server` | `ToolBuilder` at runtime |
| 4 | **calculator-add-manual-server** (this one) | trait impls by hand |

## What this rung demonstrates

The full trait set a tool must satisfy. `ToolDefinition` is a blanket impl over
these, so implementing them is all that registration needs:

| Trait | This example returns |
|---|---|
| `HasBaseMetadata` | `name` = `calculator_add_manual`, `title` = `Manual Calculator` |
| `HasDescription` | a description |
| `HasInputSchema` | a `ToolSchema` built once in a `OnceLock` |
| `HasOutputSchema` | `None` — deliberately no output schema |
| `HasAnnotations` | `None` |
| `HasToolMeta` | `None` |
| `HasIcons` | default impl |
| `HasExecution` | default impl (no task support) |
| `McpTool::call` | extracts args from raw `Value`, returns `CallToolResult` |

## Read this rung for the no-`outputSchema` case

Because `HasOutputSchema::output_schema()` returns `None`, this tool returns
**only** `content: [{"type":"text", ...}]` — no `structuredContent`, and no
`outputSchema` in `tools/list`. Compare with levels 1–3, which all declare an
output schema and therefore all get `structuredContent` filled in for them.
That is the contract: the framework emits `structuredContent` exactly when an
`outputSchema` exists.

Note the trait `impl` blocks with empty bodies (`impl HasIcons for … {}`) — a
manual tool must still name every trait, even the ones it takes defaults for.

## Spec lane

MCP **2026-07-28** (the workspace default). Stateless core: no
`initialize`/`notifications/initialized` handshake and no `Mcp-Session-Id`.

## Run

```bash
cargo run -p calculator-add-manual-server
# → http://127.0.0.1:8646/mcp
```

## Try it

```bash
META='"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"curl","version":"1.0"},"io.modelcontextprotocol/clientCapabilities":{}}'

curl -s -X POST http://127.0.0.1:8646/mcp \
  -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: tools/call' \
  -H 'Mcp-Name: calculator_add_manual' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{$META,\"name\":\"calculator_add_manual\",\"arguments\":{\"a\":5,\"b\":3}}}"
```

Expect `content: [{"text":"Sum: 8","type":"text"}]` and **no**
`structuredContent` key.
