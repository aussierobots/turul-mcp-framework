# ext-apps-server

An MCP Apps (`io.modelcontextprotocol/ui`) server built from `turul-mcp-server`'s
generic hooks and `turul-mcp-ext-apps`. No server feature is involved.

```bash
cargo run -p ext-apps-server   # http://127.0.0.1:8660/mcp
```

- `ui://weather/view.html` is the view, with `mimeType: text/html;profile=mcp-app`
  and `_meta.ui` (CSP, `prefersBorder`) on the `resources/read` content item. Its
  Refresh button calls `refresh_weather` through the host with the MCP Apps SDK.
- `get_weather` is linked to the view by `_meta.ui.resourceUri`. It returns text
  `content` either way, and notes the view only when the request's
  `clientCapabilities.extensions["io.modelcontextprotocol/ui"].mimeTypes` lists
  the profile.
- `refresh_weather` has `visibility: ["app"]`. The server still lists it: hiding
  it from the model is the host's job.

## The SDK the view loads

The view imports `@modelcontextprotocol/ext-apps` 2.1.0 (`dist/src/app-with-deps.js`)
from one exact-version jsDelivr URL. Its import map carries the file's sha384,
so the browser refuses the script if those bytes ever change. The view's
`_meta.ui.csp.resourceDomains` admits only `https://cdn.jsdelivr.net`.

To move the pin, change the version in both URLs and recompute the hash:

```bash
curl -s https://cdn.jsdelivr.net/npm/@modelcontextprotocol/ext-apps@<version>/dist/src/app-with-deps.js \
  | openssl dgst -sha384 -binary | openssl base64 -A
```

## Checks

- `scripts/interop-ext-apps.sh` runs upstream `basic-host`'s server-facing code
  (the official TypeScript SDK and ext-apps packages) against this server.
- Upstream `basic-host` renders the view in a browser once its client uses
  `versionNegotiation: { mode: 'auto' }`
  ([modelcontextprotocol/ext-apps#857](https://github.com/modelcontextprotocol/ext-apps/issues/857)).
