# ext-apps-server

An MCP Apps (`io.modelcontextprotocol/ui`) server built from `turul-mcp-server`'s
generic hooks and `turul-mcp-ext-apps`. No server feature is involved.

```bash
cargo run -p ext-apps-server   # http://127.0.0.1:8660/mcp
```

- `ui://weather/view.html` — the view: `mimeType: text/html;profile=mcp-app`,
  `_meta.ui` (CSP, `prefersBorder`) on the `resources/read` content item.
- `get_weather` — `_meta.ui.resourceUri` links it to the view. It returns text
  `content` either way, and notes the view only when the request's
  `clientCapabilities.extensions["io.modelcontextprotocol/ui"].mimeTypes` lists
  the profile.
- `refresh_weather` — `visibility: ["app"]`. It is listed: hiding it from the
  model is the host's job.

Read it back:

```bash
curl -s http://127.0.0.1:8660/mcp \
  -H 'Accept: application/json' -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' -H 'Mcp-Method: resources/read' \
  -H 'Mcp-Name: ui://weather/view.html' \
  -d '{"jsonrpc":"2.0","id":1,"method":"resources/read","params":{"uri":"ui://weather/view.html","_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}}}}'
```
