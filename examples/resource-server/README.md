# Resource Server

Demonstrates `#[derive(McpResource)]` across four resource shapes: a JSON
config with a `#[content]` field, a unit struct, a multi-field profile, and a
tuple struct serving log text.

## Resources (as registered by `src/main.rs`)

| Resource | URI | Shape |
|---|---|---|
| `config` | `file:///tmp/config.json` | Struct with `#[content]` JSON field |
| `system_status` | `system://status` | Unit struct, default implementation |
| `user_profile` | `data://user-profile` | Multiple content fields |
| `app_log` | `file:///tmp/app.log` | Tuple struct serving text |

## Run

```bash
cargo run -p resource-server
# → http://127.0.0.1:8007/mcp
```

## Try it (2026-07-28 stateless)

```bash
META='"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"curl","version":"1.0"},"io.modelcontextprotocol/clientCapabilities":{}}'

# List the four resources
curl -s -X POST http://127.0.0.1:8007/mcp \
  -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: resources/list' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"resources/list\",\"params\":{$META}}"

# Read the config resource
curl -s -X POST http://127.0.0.1:8007/mcp \
  -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: resources/read' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"resources/read\",\"params\":{$META,\"uri\":\"file:///tmp/config.json\"}}"
```

## Declaring a resource

```rust
#[derive(McpResource, Serialize, Deserialize, Clone)]
#[resource(
    name = "config",
    uri = "file:///tmp/config.json",
    description = "Main application configuration file"
)]
struct ConfigResource {
    #[content]
    settings: serde_json::Value,
}
```

- `#[resource(name, uri, description)]` declares the resource metadata
- `#[content]` marks the field served as the resource body
- Register with `McpServer::builder().resource(...)` — no separate
  `.with_resources()` call is needed

## See also

- `function-resource-server` — `.resource_fn()` registration with URI
  templates and template-variable extraction
