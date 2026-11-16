# Vendored MCP Server Card schema — provenance

- **Repository**: https://github.com/modelcontextprotocol/ext-server-card
- **Upstream status**: Experimental, not an accepted MCP extension (tracks SEP-2127)
- **Schema family**: `v1` (`https://static.modelcontextprotocol.io/schemas/v1/server-card.schema.json`)

Upstream publishes no dated directory; its schema versions by the `vN` segment.
Every file is pinned to one immutable commit.

| File | Upstream path | Commit | Content sha256 | Fetched |
|---|---|---|---|---|
| `schema.ts` | `schema.ts` | `526201bbc80231daa40ffcdecfc9da4e54e5dc93` | `f8321e0ada31f32e372bb715c27f917229d02675d52b4fa9a14909df5905f2e9` | 2026-11-17 |
| `schema.json` | `schema.json` | `526201bbc80231daa40ffcdecfc9da4e54e5dc93` | `2c772b51edb367f154771d84ddbae87ddba00a624422c8e46f218a9ac03bf042` | 2026-11-17 |
| `examples/valid/minimal.json` | `examples/ServerCard/valid/minimal.json` | `526201bbc80231daa40ffcdecfc9da4e54e5dc93` | `cbfaa438626204593dc4eae0b7d0c125cf0e925fba9ad1ee659dfd8aaf71373a` | 2026-11-17 |
| `examples/valid/templated-remote.json` | `examples/ServerCard/valid/templated-remote.json` | `526201bbc80231daa40ffcdecfc9da4e54e5dc93` | `eb5c67aa82a816f0f8f408ac4f8e865bd2b2362adec801d20b20a7dc83465e42` | 2026-11-17 |
| `examples/invalid/bad-name-pattern.json` | `examples/ServerCard/invalid/bad-name-pattern.json` | `526201bbc80231daa40ffcdecfc9da4e54e5dc93` | `3596f81818bbbeece689364d7abde0024ab3056c028c739081948aaed9aed958` | 2026-11-17 |
| `examples/invalid/date-versioned-schema.json` | `examples/ServerCard/invalid/date-versioned-schema.json` | `526201bbc80231daa40ffcdecfc9da4e54e5dc93` | `96815bebf20c4b7b6ea31de82f2d6c23bdb4642d7343958d740dc7ce0a1d0f88` | 2026-11-17 |
| `examples/invalid/missing-name.json` | `examples/ServerCard/invalid/missing-name.json` | `526201bbc80231daa40ffcdecfc9da4e54e5dc93` | `da9bd30b635b3f58ae9e1ccfa22198792716a6c0cbf1a5d3adf3ad5aed5a8053` | 2026-11-17 |
| `examples/invalid/missing-schema.json` | `examples/ServerCard/invalid/missing-schema.json` | `526201bbc80231daa40ffcdecfc9da4e54e5dc93` | `6aa69827464545c01930344524920e10186619f294149d27952a9dc6e3afa1da` | 2026-11-17 |
| `examples/invalid/wrong-schema-name.json` | `examples/ServerCard/invalid/wrong-schema-name.json` | `526201bbc80231daa40ffcdecfc9da4e54e5dc93` | `2e71031d84e9b2f3663fc9d2762ba84acecd2f4f282d24d3f1e62a552152c1bd` | 2026-11-17 |

`scripts/check-schema-pin.sh` verifies every row's checksum offline and fails if
a file in this directory is not listed. To re-pin, fetch the files at a new
commit, update the table, and re-run the crate's tests.
