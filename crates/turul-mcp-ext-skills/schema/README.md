# Vendored Skills extension specification — provenance

| File | Upstream repo | Upstream path | Commit | Content sha256 | Fetched |
|---|---|---|---|---|---|
| `skills.mdx` | `modelcontextprotocol/ext-skills` | `specification/beta/skills.mdx` | `ff224337b01f0c9b18b81dcf99eefe66e9a52b5c` | `6e33a33762783a02dc4c13c889dda5b8d7c525649012cf77da12d80c04ce6851` | 2026-11-14 |

- **Extension identifier**: `io.modelcontextprotocol/skills` (SEP-2640; moved to `beta/` by SEP-3392)
- **Status upstream**: Beta
- **Governing ADR**: [ADR-033](../../../docs/adr/033-skills-over-mcp.md)

## Why there is no dated release path or schema file

`modelcontextprotocol/ext-skills` carries no tags, no dated `specification/<YYYY-MM-DD>/`
directory and no `schema.ts`; the TypeScript blocks inside `skills.mdx` are the
schema. A commit pin plus a content checksum is the strongest provenance the source
admits. `specification/beta/skills.md` is a six-line VitePress include of the `.mdx`
and is not vendored.

Pinned commit `ff22433` is the last commit to touch `skills.mdx`; upstream head at
fetch time was `649f7aee87d631ac4d1fdd3b12eef0c6264fdb3a` (2026-10-30), at which the
file is byte-identical.

## Re-pin procedure

1. Fetch `specification/beta/skills.mdx` at the newer commit into this directory.
2. Update every cell of the table above, and the Pin table in ADR-033.
3. Re-read ADR-033 §"What the specification asks of a server" against the new text;
   a changed clause is an ADR revision, not a doc edit.
4. `./scripts/check-schema-pin.sh` — reads the checksum from this table.
5. `cargo test -p turul-mcp-ext-skills` — `compliance_test.rs` round-trips every
   JSON example in the vendored file.
