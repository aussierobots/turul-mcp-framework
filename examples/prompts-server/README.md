# Prompts Server

Manual `McpPrompt` trait implementation — the reference for hand-rolled prompt
handlers built from the fine-grained `Has*` traits with template substitution
and argument validation.

## Prompts (as registered by `src/main.rs`)

| Prompt | Required arguments | Optional arguments |
|---|---|---|
| `generate_code` | `language`, `requirements` | `style`, `framework` |
| `review_code` | `code`, `language` | `focus` |
| `architecture_guidance` | `project_type`, `requirements` | `scale`, `technology_stack` |

## Run

```bash
cargo run -p prompts-server
# → http://127.0.0.1:8006/mcp
```

## Try it (2026-07-28 stateless)

Every request carries its own per-request `_meta` and the
`MCP-Protocol-Version: 2026-07-28` header — there is no handshake and no
session header.

```bash
META='"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"curl","version":"1.0"},"io.modelcontextprotocol/clientCapabilities":{}}'

# List the three prompts
curl -s -X POST http://127.0.0.1:8006/mcp \
  -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: prompts/list' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"prompts/list\",\"params\":{$META}}"

# Render a prompt with template substitution
curl -s -X POST http://127.0.0.1:8006/mcp \
  -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: prompts/get' \
  -H 'Mcp-Name: generate_code' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"prompts/get\",\"params\":{$META,\"name\":\"generate_code\",\"arguments\":{\"language\":\"rust\",\"requirements\":\"a CLI that counts words\",\"style\":\"functional\"}}}"

# Missing a required argument → -32602
curl -s -X POST http://127.0.0.1:8006/mcp \
  -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: prompts/get' \
  -H 'Mcp-Name: review_code' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"prompts/get\",\"params\":{$META,\"name\":\"review_code\",\"arguments\":{\"code\":\"fn main() {}\"}}}"
```

## What this demonstrates

- Manual `McpPrompt` implementation from the fine-grained traits
  (`HasPromptMetadata`, `HasPromptDescription`, `HasPromptArguments`, ...)
- `{placeholder}` template substitution from `prompts/get` arguments
- Required-argument validation returning `-32602` on missing arguments
- Registering prompts with `McpServer::builder().prompt(...)`
