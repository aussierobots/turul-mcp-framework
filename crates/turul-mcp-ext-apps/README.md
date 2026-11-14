# turul-mcp-ext-apps

Rust bindings for the **MCP Apps** extension (`io.modelcontextprotocol/ui`,
SEP-1865) — the wire shapes an MCP server uses to declare interactive HTML
views alongside its tools and resources.

Apps protocol version: **2026-01-26**. The extension versions independently of
core MCP; this crate's `v2026_07_28` module names the *core* spec lane it pairs
with, not the Apps version. Vendored spec and its pins: [`schema/README.md`](schema/README.md).

## What this crate is

Serde types, constants and helpers. `turul-mcp-server` serves Apps through its
generic hooks, so no server feature is needed.

| Type | Wire location |
|---|---|
| `UiClientCapabilities` | client `capabilities.extensions["io.modelcontextprotocol/ui"]` |
| `UiToolMeta` | a tool's `_meta.ui` — `resourceUri`, `visibility` |
| `UiResourceMeta` | `_meta.ui` on a UI resource's `resources/read` content item — `csp`, `permissions`, `domain`, `prefersBorder` |
| `UiResourceCsp`, `UiResourcePermissions`, `UiToolVisibility`, `EmptyObject` | nested in the above |

`EXTENSION_IDENTIFIER`, `MCP_APP_HTML_MIME`, `META_KEY_UI` and `UI_URI_SCHEME`
carry the spec's string literals. `capability()` is the server's
`capabilities.extensions` entry (`{}`); `declared_by_server` reads it back.
`declared_by_client` / `client_supports_html_views` read a client's declaration.
`UiToolMeta::to_meta` and `UiResourceMeta::to_meta` build the `{ "ui": … }`
`_meta` map.

## Serving an MCP App with `turul-mcp-server`

1. **Advertise** the extension:
   `.extension_capability(EXTENSION_IDENTIFIER, capability())`.
2. **Link tools** to the view: return `UiToolMeta::to_meta()` from
   `HasToolMeta::tool_meta`, or pass it to `ToolBuilder::meta`. List every
   tool, including `visibility: ["app"]` ones; apps call those through
   `tools/call`.
3. **Serve the view** from an `McpResource` whose URI starts with
   `UI_URI_SCHEME`, whose `mimeType` is `MCP_APP_HTML_MIME`, and whose `read`
   returns an HTML5 document with `_meta` set to `UiResourceMeta::to_meta()`.
4. **Degrade in the call.** On 2026-07-28 the client's capabilities ride every
   request's `_meta`; read them with `SessionContext::client_capabilities()` and
   `client_supports_html_views`. Return meaningful text `content` either way.

`examples/ext-apps-server` does all four, and
`crates/turul-mcp-server/tests/ext_apps_2026.rs` checks them on the wire.

## What belongs to the host, not the server

- Hiding tools whose `visibility` omits `"model"` from the agent, and rejecting
  app-originated `tools/call` for tools without `"app"`. The Apps spec
  (§Visibility) makes both host MUSTs.
- Sandboxing, building CSP from `_meta.ui.csp`, and the `ui/*` postMessage
  protocol between host and view. This crate binds none of it.

`turul-mcp-client` does not declare the Apps capability: it renders nothing.

## Feature flags

`protocol-2026-07-28` (default) compiles the bindings against
`turul-mcp-protocol-2026-07-28`. With `--no-default-features` the crate exports
nothing and does not pull in the protocol crate.

## Testing

```bash
cargo test -p turul-mcp-ext-apps      # wire-shape tests
./scripts/check-schema-pin.sh         # vendored spec pin + checksum gate
```
