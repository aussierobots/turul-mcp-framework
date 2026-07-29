# Resource Server

Demonstrates `#[derive(McpResource)]` across four resource shapes: a JSON
config with a `#[content]` field, a unit struct, a multi-field profile, and a
tuple struct serving log text.

## Resources (as registered by `src/main.rs`)

| Resource | URI | `mimeType` | Shape |
|---|---|---|---|
| `config` | `file:///tmp/config.json` | `application/json` | Named-field struct |
| `system_status` | `system://status` | `application/json` | Unit struct |
| `user_profile` | `data://user-profile` | `application/json` | Returns two `contents[]` entries |
| `app_log` | `file:///tmp/app.log` | `text/plain` | Tuple struct |

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
    description = "Main application configuration file",
    mime_type = "application/json"
)]
struct ConfigResource {
    config_data: String,
}

#[async_trait]
impl McpResource for ConfigResource {
    async fn read(&self, _params: Option<Value>, _session: Option<&SessionContext>)
        -> McpResult<Vec<ResourceContent>>
    {
        Ok(vec![ResourceContent::json(self.uri(), self.config_data.clone())])
    }
}
```

- `#[resource(name, uri, description, mime_type)]` generates the `Has*`
  metadata traits — that is *all* the derive does. It does not read the
  struct's fields, so `read()` is always hand-written.
- `mime_type` is what `resources/list` advertises. Make `read()` agree:
  `ResourceContent::json()` for JSON, `ResourceContent::text()` for
  `text/plain`, and `ResourceContent::text(..).with_mime_type(..)` for
  anything else — `text()` alone always reports `text/plain`.
- `ResourceContent::blob()` is for **base64** payloads. Passing raw text to it
  produces a `blob` field that is not base64, which the schema requires.
- Register with `McpServer::builder().resource(...)` — no separate
  `.with_resources()` call is needed

`user_profile` shows that one `resources/read` may return several
`contents[]` entries, each with its own URI and `mimeType`.

## See also

- `function-resource-server` — `.resource_fn()` registration with URI
  templates and template-variable extraction
