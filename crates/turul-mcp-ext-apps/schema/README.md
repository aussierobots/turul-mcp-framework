# Vendored MCP Apps (SEP-1865) spec — provenance

| File | Upstream source |
|---|---|
| `spec.types.ts` | `modelcontextprotocol/ext-apps` → `src/spec.types.ts` |
| `apps-draft.mdx` | `modelcontextprotocol/ext-apps` → `specification/draft/apps.mdx` |

- **Repository**: https://github.com/modelcontextprotocol/ext-apps
- **Commit**: `ca1d29894fabbd1558885a9ec8620dcb01d7457e` (2026-06-04)
- **Extension identifier**: `io.modelcontextprotocol/ui`
- **Apps protocol version**: `2026-01-26` (the extension versions independently of core MCP)

This crate binds the **MCP-side** surface only: the extension capability
(`McpUiClientCapabilities`), tool `_meta.ui` metadata (`McpUiToolMeta`), and
UI-resource `_meta.ui` metadata (`McpUiResourceMeta` + CSP/permissions). The
host↔view iframe protocol (`ui/*` methods over postMessage) belongs to app/host
SDKs, not a server framework, and is deliberately not bound here.

Re-pin by fetching the same paths at a newer commit and updating this table.
