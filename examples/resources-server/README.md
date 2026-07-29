# Development Team Resource Server

Five hand-written `McpResource` implementations that load their bodies from
external files under `data/`, showing the pattern a team resource hub uses:
content lives in markdown/JSON/SQL files that non-Rust contributors can edit,
and the resource is a thin loader with a fallback when the file is missing.

Spec lane: **2026-07-28** (workspace default). Stateless — no
`initialize`/`notifications/initialized`, no `Mcp-Session-Id`. Every request
carries its own `_meta` plus the `MCP-Protocol-Version` header, and
`resources/read` also requires `Mcp-Name` set to the URI being read.

## Resources (as registered by `src/main.rs`)

| URI | `mimeType` | Source |
|---|---|---|
| `file:///docs/project.md` | `text/markdown` | workspace `README.md` + inline overview |
| `file:///docs/api.md` | `text/markdown` | `data/api_docs.md` |
| `file:///config/app.json` | `application/json` | `data/app_config.json` |
| `file:///schema/database.sql` | `text/plain` | `data/database_schema.sql` |
| `file:///status/system.json` | `application/json` | generated at read time |

Several resources return **more than one `contents[]` entry** for the same URI
— e.g. the config resource returns the JSON document plus a markdown
environment-variable guide. Each entry declares its own `mimeType`, which is
why the guide comes back as `text/markdown` even though the resource's
`resources/list` entry says `application/json`.

## Run

```bash
# data/ is resolved relative to the working directory
cd examples/resources-server
cargo run -p resources-server
# → http://127.0.0.1:8041/mcp
```

## Try it

```bash
META='"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"curl","version":"1.0"},"io.modelcontextprotocol/clientCapabilities":{}}'

# What the server offers
curl -s -X POST http://127.0.0.1:8041/mcp \
  -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: server/discover' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"server/discover\",\"params\":{$META}}" | jq

# List the five resources
curl -s -X POST http://127.0.0.1:8041/mcp \
  -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: resources/list' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"resources/list\",\"params\":{$META}}" \
  | jq '.result.resources[] | {uri, mimeType}'

# Read one — note Mcp-Name must carry the URI
curl -s -X POST http://127.0.0.1:8041/mcp \
  -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: resources/read' \
  -H 'Mcp-Name: file:///config/app.json' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"resources/read\",\"params\":{$META,\"uri\":\"file:///config/app.json\"}}" \
  | jq '.result.contents[] | {uri, mimeType}'

# No templates are registered → empty list, not an error
curl -s -X POST http://127.0.0.1:8041/mcp \
  -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: resources/templates/list' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":4,\"method\":\"resources/templates/list\",\"params\":{$META}}" \
  | jq '.result.resourceTemplates'

# Unknown URI → -32602
curl -s -X POST http://127.0.0.1:8041/mcp \
  -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: resources/read' \
  -H 'Mcp-Name: docs://project' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":5,\"method\":\"resources/read\",\"params\":{$META,\"uri\":\"docs://project\"}}" \
  | jq '.error'
```

## Declaring content types

`ResourceContent::text()` hardcodes `text/plain` and `ResourceContent::json()`
hardcodes `application/json`. Anything else — markdown here — must say so
explicitly, or `resources/read` contradicts the `mimeType` that
`resources/list` advertises for the same URI:

```rust
ResourceContent::text("file:///docs/api.md", content)
    .with_mime_type("text/markdown")
```

The server also enforces a MIME allowlist on `resources/read`, auto-derived
from the file extensions of the registered resource URIs (`.md`, `.json`,
`.txt`, `.csv`, `.html`, `.xml`, `.pdf`, `.png`, `.jpg`). A type outside that
set — `application/sql`, say — is advertised by `resources/list` and then
rejected by `resources/read` with `-32602`, which is why the schema resource
here declares `text/plain`.

## See also

- `resource-server` — the same surface via `#[derive(McpResource)]`
- `function-resource-server` — `.resource_fn()` plus a URI template
