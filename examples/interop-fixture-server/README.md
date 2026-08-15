# Interop Fixture Server

One fixed MCP 2026-07-28 surface that every cross-implementation probe runs
against — FastMCP (Python), the MCP TypeScript SDK, the MCP Go SDK, and turul's
own client — so a difference between two runs is a difference between the
clients, not between the servers they happened to be pointed at.

It exists because `minimal-server` exposes only a tool: the read surface
(resources, prompts, completion) cannot be exercised against it, which capped
interop coverage at 3 of 22 methods.

## The surface is a contract

**Every name and value below is asserted by `scripts/interop-*.sh`.** Changing
one means changing those scripts in the same slice.

| Kind | Identity | Detail |
|---|---|---|
| Tool | `echo` | `text: String` → `"Echo: <text>"` |
| Tool | `add` | `a: f64`, `b: f64` → `a + b` (numeric args exercise peer schema handling) |
| Resource | `file:///fixture/readme.md` | `text/markdown`, stable body |
| Prompt | `greeting` | argument `name` → one user message `"Hello, <name>!"` |
| Completion | `ref/prompt` `greeting`, argument `name` | prefix-filtered over `ada`, `alan`, `grace` |

The resource declares `text/markdown` and its `read()` sets the same type
explicitly with `.with_mime_type(FIXTURE_MIME)` — `ResourceContent::text()`
alone reports `text/plain`, which would make `resources/read` contradict the
`resources/list` entry for the same URI.

No resource templates are registered, so `resources/templates/list` answers
with an empty `resourceTemplates` array — not `-32601`.

## Run

```bash
cargo run -p interop-fixture-server -- --port 8700
# → http://127.0.0.1:8700/mcp
```

## Try it (2026-07-28 stateless)

No handshake, no `Mcp-Session-Id`. Each request carries its own `_meta` and the
`MCP-Protocol-Version` header; `Mcp-Method` must agree with the body's method,
and `resources/read` / `prompts/get` / `tools/call` also need `Mcp-Name`.

```bash
META='"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"curl","version":"1.0"},"io.modelcontextprotocol/clientCapabilities":{}}'

curl -s -X POST http://127.0.0.1:8700/mcp \
  -H 'Content-Type: application/json' -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: resources/read' -H 'Mcp-Name: file:///fixture/readme.md' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"resources/read\",\"params\":{$META,\"uri\":\"file:///fixture/readme.md\"}}" \
  | jq '.result.contents[] | {uri, mimeType}'

curl -s -X POST http://127.0.0.1:8700/mcp \
  -H 'Content-Type: application/json' -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: completion/complete' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"completion/complete\",\"params\":{\"ref\":{\"type\":\"ref/prompt\",\"name\":\"greeting\"},\"argument\":{\"name\":\"name\",\"value\":\"a\"},$META}}" \
  | jq '.result.completion'
```

## See also

- `examples/interop-client-probe` — turul's own client leg of the probe set
