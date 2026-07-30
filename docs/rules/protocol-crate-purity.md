# Protocol Crate Purity

**Precedence — settled 2026-07-31: the MCP spec, the ADRs and the code outrank this
rule's wording.** This section describes an intent in prose; prose generalises badly.
When the phrasing here appears to forbid something the schema requires, an ADR
decided, or the code already does correctly, **the rule is what is wrong** — fix the
wording, do not "fix" the code to satisfy it. In that order: the spec settles what the
protocol crate must contain, an ADR settles a decision already taken, the code settles
what is actually true today.

This is not a licence to ignore the rule. It resolves *conflicts*, and a conflict
means the spec/ADR/code demonstrably says otherwise — cite the schema type, the ADR
number, or the file. "It seemed convenient" is not a conflict.

**NEVER modify `turul-mcp-protocol` or `turul-mcp-protocol-2026-07-28` unless it directly relates to MCP spec compliance.** No framework features, middleware hooks, or convenience additions.

**Forbidden**: *Invented* trait hierarchies, builder patterns, framework helpers, tutorial docs
**Allowed**: MCP spec types, serde derives, basic builder methods on concrete types, spec error types
**Framework traits belong in `turul-mcp-builders`** (`turul-mcp-builders/src/traits/`)

**Schema-mirroring traits are not the forbidden kind.** Each protocol crate carries
75–80 traits in `traits.rs` (`HasMethod`, `HasParams`, `HasMeta`, …) that exist only
because the MCP schema is TypeScript and uses interface inheritance —
`ProgressNotificationParams extends NotificationParams` has no direct Rust
equivalent. Their names follow the schema's own interface names, so they are part of
the 1:1 mapping, not machinery layered on top. This is the shape in all three
protocol crates and predates 0.4.

The distinction is **transcribed vs. invented**. If the trait exists because the
schema declares that relationship, it belongs in the protocol crate. If it exists to
make the framework nicer to use — `HasInputSchema`, `HasExecution`, `HasIcons` — it
belongs in `turul-mcp-builders`. No trait name is defined in both places; keep it
that way.

This is the worked example of the precedence above. Read literally, "Forbidden: trait
hierarchies" condemned 75–80 traits that the schema itself declares — and on
2026-07-31 that reading cost a round trip, escalated as an architectural question when
the actual defect was one mislabelled doc comment. The spec won; the wording moved.

`scripts/check-protocol-purity.sh` enforces the crude parts of this and runs in
`gate_default`. It greps for the word "Framework" in `//` comments, so describing
protocol traits as "framework traits" trips it — correctly, since that phrasing
misfiles them. A grep is a proxy, not the rule: if it flags something the schema
requires, the fix is the label or the grep, never deleting spec-mandated code.

## Frozen Protocol Crates (DO NOT MODIFY)

**`turul-mcp-protocol-2025-06-18` and `turul-mcp-protocol-2025-11-25` are FROZEN at 0.3.x.** They are historical spec snapshots and must never be edited again — no patches, no version bumps, no doc updates, no dependency changes. New MCP spec work lives in `turul-mcp-protocol-2026-07-28` (0.4.x line). The only permitted touch is workspace `Cargo.toml` metadata if a workspace-wide rename forces it.
