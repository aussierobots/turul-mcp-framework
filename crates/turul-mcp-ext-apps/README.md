# turul-mcp-ext-apps

Rust bindings for the **MCP Apps** extension (`io.modelcontextprotocol/ui`,
SEP-1865) — the wire shapes an MCP server uses to declare interactive HTML
views alongside its tools and resources.

Apps protocol version: **2026-01-26**. The extension versions independently of
core MCP; this crate's `v2026_07_28` module names the *core* spec lane it pairs
with, not the Apps version. Vendored spec and its pins: [`schema/README.md`](schema/README.md).

## What this crate is

Serde type definitions and three helper functions. Nothing else.

| Type | Wire location |
|---|---|
| `UiClientCapabilities` | `capabilities.extensions["io.modelcontextprotocol/ui"]` |
| `UiToolMeta` | a tool's `_meta.ui` — `resourceUri`, `visibility` |
| `UiResourceMeta` | a UI resource's `_meta.ui` — `csp`, `permissions`, `domain`, `prefersBorder` |
| `UiResourceCsp`, `UiResourcePermissions`, `UiToolVisibility`, `EmptyObject` | nested in the above |

`EXTENSION_IDENTIFIER`, `MCP_APP_HTML_MIME` and `META_KEY_UI` carry the spec's
string literals. `declared_by_client` / `client_supports_html_views` read the
capability back out of a `ClientCapabilities` value you already hold.

The crate's own tests assert that these types serialize to the bytes the spec
document shows. That is the full extent of what is verified.

## What this crate is **not**

**It is not wired into anything.** A repo-wide grep finds no reference to
`turul-mcp-ext-apps` from `turul-mcp-server`, `turul-mcp-client`, or any
example — only the workspace member list and documentation. Concretely:

- The server does **not** advertise or negotiate the Apps extension.
- The server does **not** dispatch, validate, or filter on `_meta.ui`.
- `tools/list` does **not** hide tools whose `visibility` omits `"model"`, and
  `tools/call` does **not** reject app-only calls. Those are MUSTs in the Apps
  spec, and this crate does not enforce them.
- The client does **not** declare `io.modelcontextprotocol/ui` in its
  capabilities.
- Nothing serves `ui://` resources or applies the CSP/permissions metadata.

This is a deliberate intermediate step, recorded in `docs/adr/028-extensions-strategy.md`,
and its unwired status is tracked in `docs/compliance/extensions.md`. It is
recorded here too so that the state is visible from the crate itself rather
than only from a document a consumer may never open.

## What a consumer still has to build

Everything between these types and a working MCP App:

1. **Read the client's declared capability.** On the 2026-07-28 core lane,
   client capabilities ride `_meta["io.modelcontextprotocol/clientCapabilities"]`
   on every request rather than an `initialize` handshake. Extract the
   `ClientCapabilities`, then call `client_supports_html_views` to decide
   whether to expose UI-enabled tools at all.
2. **Attach the metadata.** Serialize `UiToolMeta` into each tool's `_meta`
   under the `META_KEY_UI` key, and `UiResourceMeta` into the UI resource's
   `_meta`. This crate provides the values; placing them in the outgoing
   `Tool` / `Resource` is yours.
3. **Serve the view.** Register a resource whose URI uses the `ui://` scheme
   and whose `mimeType` is `MCP_APP_HTML_MIME`, returning an HTML5 document
   via `text` or `blob`.
4. **Enforce visibility.** Filter `visibility: ["app"]` tools out of
   `tools/list`, and reject app-originated `tools/call` for tools that do not
   include `"app"`.
5. **Everything host-side.** Sandboxing, CSP header construction and the
   `ui/*` postMessage protocol between host and view belong to the host
   application, not to a server framework. This crate deliberately binds none
   of it.

## Feature flags

`protocol-2026-07-28` (default) compiles the bindings against
`turul-mcp-protocol-2026-07-28`. With `--no-default-features` the crate exports
nothing and does not pull in the protocol crate.

## Testing

```bash
cargo test -p turul-mcp-ext-apps      # wire-shape tests
./scripts/check-schema-pin.sh         # vendored spec pin + checksum gate
```
