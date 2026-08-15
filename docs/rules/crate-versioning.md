# Crate Versioning Policy

**Each crate carries its own literal `version = "X.Y.Z"` in `Cargo.toml`.** No crate uses `version.workspace = true`. The 0.4.0 release is the first under this policy — all non-frozen crates ship at 0.4.0 together, but going forward they can be patched and published independently (only bumping the crate that changed, not the whole workspace).

- Frozen crates (`turul-mcp-protocol-2025-06-18`, `turul-mcp-protocol-2025-11-25`) stay at `0.3.47`. They are historical spec snapshots and don't move.
- All other crates start the 0.4.x line at `0.4.0`.
- `[workspace.package].version` exists but is **not authoritative** — it's a default for tooling. Per-crate `version = "..."` is the source of truth.
- `[workspace.dependencies]` pins the version for each internal crate path. When bumping a crate, bump it in the crate's `Cargo.toml` AND in the workspace dependency pin.

## Version References: what is stale, and what is not

This branch is 0.4. A **current-state claim** naming 0.3 is stale and must move —
"depend on `turul-mcp-server = "0.3"`", "target v0.3.x", "the 0.3 API does X".

These legitimately stay 0.3 and a blanket `0.3` → `0.4` sweep would corrupt them:

- **Frozen crates** — `turul-mcp-protocol-2025-06-18`, `turul-mcp-protocol-2025-11-25`
  and `turul-mcp-json-rpc-server` stay published at `0.3.47` and never move.
- **Since-markers** — "Since v0.3.27, backend features forward to both storage
  crates", "two streaming entry points (v0.3+)". Still true; the version records when
  it became true.
- **Changelog history** — CHANGELOG.md and the plugin README's release sections.
  Rewriting a shipped release's notes falsifies the record.
- **Incident citations** — the v0.3.40 → v0.3.41 and v0.3.42 references in
  [test-coverage-discipline.md](test-coverage-discipline.md) name when a specific bug
  shipped. Renumbering them destroys the evidence the rule rests on.
- **External crate versions** — `futures = "0.3"`, `tracing-subscriber = "0.3"`,
  `async-stream = "0.3"` have nothing to do with this workspace.

Disposition each hit; never sweep — and **grep both forms**. `v0.3` and `= "0.3"` are
different populations: searching only `v0.3` in `plugins/` found nine hits, eight of
them history. Searching `= "0.3"` found 50 more — dependency pins in skill examples,
`.version()` strings, and `scripts/scaffold-mcp-server.sh`, which *generates* a
`Cargo.toml` for users and was emitting `turul-mcp-server = "0.3"`. All 50 were live
instructions. The prose-only search made the problem look 5× smaller than it was and
pointed at the wrong files.

## Workspace Dependencies

External crate dependencies (`serde`, `tokio`, `hyper`, etc.) MUST use `workspace = true` references. Declare versions in root `Cargo.toml` `[workspace.dependencies]`, reference with `.workspace = true` in crate `Cargo.toml`. Add crate-specific features inline: `hyper = { workspace = true, features = ["http1"] }`.

## Feature Flags — Storage Backends

Default features: `["http", "sse"]` — in-memory only, no backend deps compiled. Storage backends are opt-in:

```toml
# In-memory only (default)
turul-mcp-server = "0.4"

# With DynamoDB backends
turul-mcp-server = { version = "0.4", features = ["dynamodb"] }

# With DynamoDB + dynamic tools
turul-mcp-server = { version = "0.4", features = ["dynamodb", "dynamic-tools"] }
```

Backend features (`sqlite`, `postgres`, `dynamodb`) forward to both `turul-mcp-session-storage` AND `turul-mcp-task-storage`. When `dynamic-tools` is enabled, they also forward to `turul-mcp-server-state-storage` via weak dep syntax (`?/`).
