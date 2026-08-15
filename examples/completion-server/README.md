# IDE Auto-Completion Server

Demonstrates the **real MCP completion protocol** — `completion/complete` —
served by an `McpCompletion` provider registered with
`.completion_provider()`, alongside a plain tool for contrast.

Two different surfaces:

| Surface | Method | For |
|---|---|---|
| Completion provider | `completion/complete` | Argument autocomplete while a user edits a prompt/template (IDE-style) |
| Tool | `tools/call` (`ide_completion`) | Model-invoked suggestion lookups |

The provider completes the `language` argument of the `code_review` prompt:
the routing handler matches the request's `ref` against the provider's
declared reference, prefix-filters, and the framework enforces the spec's
100-item response cap.

## Run

```bash
cargo run -p completion-server
# → http://127.0.0.1:8042/mcp
```

## Try it (2026-07-28 stateless)

```bash
META='"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"curl","version":"1.0"},"io.modelcontextprotocol/clientCapabilities":{}}'

# Argument completion: "ru" → ["ruby","rust"]
curl -s -X POST http://127.0.0.1:8042/mcp \
  -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: completion/complete' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"completion/complete\",\"params\":{\"ref\":{\"type\":\"ref/prompt\",\"name\":\"code_review\"},\"argument\":{\"name\":\"language\",\"value\":\"ru\"},$META}}"

# The prompt the completion serves
curl -s -X POST http://127.0.0.1:8042/mcp \
  -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: prompts/get' -H 'Mcp-Name: code_review' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"prompts/get\",\"params\":{\"name\":\"code_review\",\"arguments\":{\"language\":\"rust\"},$META}}"

# The contrast tool
curl -s -X POST http://127.0.0.1:8042/mcp \
  -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: tools/call' -H 'Mcp-Name: ide_completion' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"tools/call\",\"params\":{\"name\":\"ide_completion\",\"arguments\":{\"category\":\"language\",\"prefix\":\"py\"},$META}}"
```

## Contract notes

- A server with completion providers advertises the `completions`
  capability in `server/discover`; one without answers
  `completion/complete` with 404 + `-32601`.
- Malformed params (missing `argument`, unknown `ref` type) → `-32602`.
- Provider output is capped at 100 values (`total`/`hasMore` reflect a cut).
