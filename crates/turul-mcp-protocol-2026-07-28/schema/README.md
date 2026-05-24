# Vendored MCP Draft Schema

This directory vendors the upstream MCP specification TypeScript schema **for offline reference**. The Rust types in this crate must serialize to JSON shapes that match `draft-schema.ts` exactly. Tests in `src/compliance_test.rs` enforce this.

## Provenance

- **File**: `draft-schema.ts`
- **Upstream source**: <https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/schema/draft/schema.ts>
- **Raw URL used**: <https://raw.githubusercontent.com/modelcontextprotocol/modelcontextprotocol/main/schema/draft/schema.ts>
- **Vendored on**: 2026-05-24
- **Upstream ETag at vendor time**: `8bdd4ae5a9b3a8d2e611124cf7240d60cf0fcece9652eae51571ba5f1be0e0ef`
- **Wire protocol version declared in vendored file**: `"DRAFT-2026-v1"` (line 37, `LATEST_PROTOCOL_VERSION`)
- **License**: MIT (matches turul-mcp-framework dual MIT-or-Apache-2.0 licensing)

## ⚠ DRAFT WARNING

This schema is the **draft** schema — it is not yet the final 2026-07-28 release. The upstream file changes as the spec evolves toward RC lock (2026-05-21 per blog) and final publication (2026-07-28 per blog). Re-vendor and regenerate compliance tests as the schema evolves.

**When final ships**: Replace `draft-schema.ts` with `2026-07-28-schema.ts`, flip `LATEST_PROTOCOL_VERSION` references in `src/version.rs` and `src/lib.rs::MCP_VERSION`, re-run compliance tests, bump crate version. See `docs/adr/027-targeting-mcp-draft-2026-v1.md`.

## Regenerating

```bash
curl -fsSL \
  https://raw.githubusercontent.com/modelcontextprotocol/modelcontextprotocol/main/schema/draft/schema.ts \
  -o crates/turul-mcp-protocol-2026-07-28/schema/draft-schema.ts

# Then update the ETag/date above and re-run:
cargo test -p turul-mcp-protocol-2026-07-28
```

If the regenerated schema differs from the version this crate was last built against, expect compliance tests to fail. Update Rust types to match, re-run, repeat.
