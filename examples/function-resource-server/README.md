# Function Resource Server

Registers resources with `.resource_fn(constructor)` — you hand the builder a
`fn() -> impl McpResource` rather than an instance. The builder decides from the
resource's URI whether it belongs in `resources/list` or
`resources/templates/list`: a URI containing `{placeholders}` is a template,
everything else is a static resource.

## Resources (as registered by `src/main.rs`)

| Constructor | URI | Listed under |
|---|---|---|
| `create_config_resource` | `file:///config.json` | `resources/list` |
| `create_system_status_resource` | `system://status` | `resources/list` |
| `create_user_profile_resource` | `file:///users/{user_id}.json` | `resources/templates/list` |

Template variables arrive in `read()`'s `params` under `template_variables`, so
reading `file:///users/42.json` gives the handler `user_id = "42"`.

## Content type matters

Each resource declares `mimeType: application/json` in its listing, so
`resources/read` must return content that agrees. Use
`ResourceContent::json(uri, text)` for that (or `ResourceContent::text(...)`,
which hardcodes `text/plain`, plus `.with_mime_type(...)` to override).

`ResourceContent::blob(...)` is for **base64-encoded binary only** — the server
validates it and rejects raw text with
`resource <uri> returned blob contents that are not valid base64`.

## Spec lane

MCP **2026-07-28** (the workspace default). Stateless core: no
`initialize`/`notifications/initialized` handshake and no `Mcp-Session-Id`.

## Run

```bash
cargo run -p function-resource-server
# → http://127.0.0.1:8008/mcp
```

## Try it

`resources/read` requires the `Mcp-Name` header to carry the URI being read,
and it must agree with `params.uri`.

```bash
META='"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"curl","version":"1.0"},"io.modelcontextprotocol/clientCapabilities":{}}'

# static resources
curl -s -X POST http://127.0.0.1:8008/mcp \
  -H 'Content-Type: application/json' -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: resources/list' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"resources/list\",\"params\":{$META}}"

# templates live in their own list, never in resources/list
curl -s -X POST http://127.0.0.1:8008/mcp \
  -H 'Content-Type: application/json' -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: resources/templates/list' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"resources/templates/list\",\"params\":{$META}}"

# resolve the template at read time
curl -s -X POST http://127.0.0.1:8008/mcp \
  -H 'Content-Type: application/json' -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: resources/read' -H 'Mcp-Name: file:///users/42.json' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"resources/read\",\"params\":{$META,\"uri\":\"file:///users/42.json\"}}"

# unknown URI → -32602 (2026-07-28 replaced the old -32002 resource-not-found code)
curl -s -X POST http://127.0.0.1:8008/mcp \
  -H 'Content-Type: application/json' -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: resources/read' -H 'Mcp-Name: file:///nope.json' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":4,\"method\":\"resources/read\",\"params\":{$META,\"uri\":\"file:///nope.json\"}}"
```

## See also

- `examples/resource-server`, `examples/resources-server` — the `.resource(...)`
  instance form of the same registration
