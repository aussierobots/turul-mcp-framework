# Vendored MCP Apps (SEP-1865) spec — provenance

- **Repository**: https://github.com/modelcontextprotocol/ext-apps
- **Extension identifier**: `io.modelcontextprotocol/ui`
- **Apps protocol version**: `2026-01-26` (the extension versions independently of core MCP)

| File | Upstream source | Upstream commit pin | Content sha256 | Fetched |
|---|---|---|---|---|
| `apps-2026-01-26.mdx` | `specification/2026-01-26/apps.mdx` | `298e884ec3f02daba085acdb02042d73bd00b355` (tag `v1.0.0`) | `ee452a7d1b9b7fb900acfeb4d6932d3963375b0f3f37d196a4b93eb80312af0e` | 2026-07-29 |
| `spec.types.ts` | `src/spec.types.ts` | `92f46a574568a3ddac7600343b7d3c4c4ed7b588` (tag `v1.7.5`) | `2ae52b6156f0f1fd2387717f15a8de968501d264e200d5409f09055297f8bc24` | 2026-07-29 |

## Which artifact is normative

`apps-2026-01-26.mdx` is the **released, dated** Apps specification. It is the
authority for every wire shape this crate binds. Upstream created that file in
`298e884e` and has not modified it since, so the pin is immutable by
construction. Upstream also carries `specification/draft/apps.mdx` — that is the
**next** Apps cycle's floating pointer, not this release, and vendoring it
produces a copy that cannot be reproduced and does not describe `2026-01-26`.

`spec.types.ts` is **not** a dated artifact. Upstream publishes no
`specification/2026-01-26/spec.types.ts` (nor `schema.ts` / `types.ts` under
that directory — all 404), so the SDK's own `src/spec.types.ts` is the only
machine-readable form of these types that exists, and it keeps moving after the
dated release ships. It is pinned here to the `v1.7.5` release tag so the copy
is reproducible, and it is a **convenience reference, not authority**: where it
and `apps-2026-01-26.mdx` disagree, the dated `.mdx` wins.

Both pins were checked against the field set this crate binds
(`McpUiClientCapabilities`, `McpUiToolMeta`, `McpUiToolVisibility`,
`McpUiResourceMeta`, `McpUiResourceCsp`, `McpUiResourcePermissions`): the
properties and their types are identical at `v1.0.0` (the released-spec commit)
and at `v1.7.5`. Only JSDoc prose differs, plus `McpUiToolMeta.csp`/
`.permissions` gaining explicit `never` guards after the release. Nothing this
crate binds depends on post-release SDK drift.

## Consulted but not vendored

The strictness of the permission values — each present key is `{}` and nothing
else — comes from upstream `src/generated/schema.json`, which declares
`{"type":"object","properties":{},"additionalProperties":false}` per key. The
`.mdx` and `.ts` forms both spell it `camera?: {}`, which in TypeScript means
"any non-nullish value" and does not carry that constraint. That file is not
vendored; the constraint holds identically at both pinned commits above.

## Scope of the binding

This crate binds the **MCP-side** surface only: the extension capability
(`McpUiClientCapabilities`), tool `_meta.ui` metadata (`McpUiToolMeta`), and
UI-resource `_meta.ui` metadata (`McpUiResourceMeta` + CSP/permissions). The
host↔view iframe protocol (`ui/*` methods over postMessage) belongs to app/host
SDKs, not a server framework, and is deliberately not bound here.

## Re-pinning

Fetch the same paths at a newer commit **by SHA, never by branch name** — a
branch-sourced copy cannot be reproduced later. For `apps-*.mdx` that commit
must resolve a dated `specification/<YYYY-MM-DD>/` path. Update every cell of
the table above, including the checksums, and re-run
`./scripts/check-schema-pin.sh` and `cargo test -p turul-mcp-ext-apps`.
