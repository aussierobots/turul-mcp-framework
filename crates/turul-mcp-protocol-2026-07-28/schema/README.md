# Vendored MCP Draft Schema

This directory vendors the upstream MCP specification TypeScript schema **for offline reference**. The Rust types in this crate must serialize to JSON shapes that match `draft-schema.ts` exactly. Tests in `tests/compliance.rs` enforce this.

## Provenance

- **File**: `draft-schema.ts`
- **Upstream source**: <https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/schema/draft/schema.ts>
- **Raw URL used**: <https://raw.githubusercontent.com/modelcontextprotocol/modelcontextprotocol/main/schema/draft/schema.ts>
- **Vendored on**: 2026-06-10 (re-vendored — `ElicitationCompleteNotificationParams` extraction; previously 2026-06-07, 2026-05-24)
- **Upstream ETag at vendor time**: `fddd24701cf0e60f1acae2934e0c3f2b77e32389b98807ba45cb65413be97bd5` (was `0eeaed15c73cc8dd315c0cef519528263d46e1b6a33244f5ee0bdeb10f316473` at the 2026-06-07 cut)
- **Content sha256**: `1bf94a601817ab07fc04058a9ff2e031227f9b9384e198ea7f187e75eb4b9ec6`
- **Wire protocol version declared in vendored file**: `"2026-07-28"` (line 37, `LATEST_PROTOCOL_VERSION`)
- **License**: MIT (matches turul-mcp-framework dual MIT-or-Apache-2.0 licensing)

## ⚠ DRAFT-PATH WARNING

The upstream `LATEST_PROTOCOL_VERSION` has finalized to the stable date literal
`"2026-07-28"` (was `"DRAFT-2026-v1"` pre-finalization), and the crate's
`MCP_VERSION` / `McpVersion::V2026_07_28` serde rename now emit it (the draft
literal is still accepted on deserialize for back-compat). **The file still lives
under the upstream `schema/draft/` path** and may continue to receive field-level
revisions — re-vendor and regenerate compliance tests as it evolves.

The 2026-06-07 re-vendor carried exactly three substantive (non-comment) changes
from the 2026-05-24 cut: the version-string finalization above; `ResultType`
became an open union (`"complete" | "input_required" | string`); and
`DiscoverResult` now `extends CacheableResult` (`ttlMs`/`cacheScope`). See
`docs/adr/027-targeting-mcp-draft-2026-v1.md` for the re-pin trigger and revision log.

## Regenerating

```bash
curl -fsSL \
  https://raw.githubusercontent.com/modelcontextprotocol/modelcontextprotocol/main/schema/draft/schema.ts \
  -o crates/turul-mcp-protocol-2026-07-28/schema/draft-schema.ts

# Then update the ETag/date above and re-run:
cargo test -p turul-mcp-protocol-2026-07-28
```

If the regenerated schema differs from the version this crate was last built against, expect compliance tests to fail. Update Rust types to match, re-run, repeat.
