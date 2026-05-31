# Spec-Coexistence Decision Package (2026-05-31)

## 1. The decision (3 sentences)

The framework will adopt **DRAFT-2026-v1 as the default spec in 0.4.0**, with an opt-in `legacy-2025-11-25` cargo feature for consumers still on the prior spec; this is implemented via mutually-exclusive cargo features in server/builder crates (ADR-029) and a **bilingual default** in `turul-mcp-client` (ADR-030) since a client is per-connection, not per-process. The decision is constrained by branch lock on `feat/turul-mcp-protocol-2026-07-28` (no merge to main, no publish), so 0.4.0 churn is internal-only until the maintainer authorizes release. Headline cost: Phase 9.4 (flip the `turul-mcp-protocol` re-export alias from 2025-11-25 → 2026-07-28) becomes a **hard prerequisite** for shipping default-2026 honestly, and the framework consumer crates (`turul-mcp-server`, `turul-mcp-client`, `turul-http-mcp-server`, ~55 examples, derive macros) must migrate to 2026-07-28 types in the same release window.

## 2. Files to be created

| Path | Description | Words |
|---|---|---|
| `docs/adr/029-spec-coexistence-via-cargo-features.md` | ADR-029: spec-version coexistence via mutually-exclusive cargo features; default = DRAFT-2026-v1, opt-in `legacy-2025-11-25` | ~2400 |
| `docs/adr/030-turul-mcp-client-bilingual-spec-coexistence.md` | ADR-030: `turul-mcp-client` ships bilingual by default; per-connection version negotiation; opt-in `client-2025-only` / `client-2026-only` for binary-size narrowing | ~2050 |
| `docs/plans/2026-07-28-architecture-review.md` | Architecture review summarizing dual-spec strategy, server vs client asymmetry, and cutover decisions | ~2300 |
| `docs/plans/2026-07-28-feature-gating-rollout.md` | Concrete rollout plan: per-crate cargo feature wiring, CI matrix, migration order, gating checklist | ~2400 |

## 3. Files to be modified

8 existing ADR amendments. Each amendment is either a **revision-log append** or, for ADR-027 only, a **replace-section + insert-subsection + revision-log append** combination. The prior `Status`/`Decision` blocks are preserved verbatim across all 8; only the listed sections are touched. The exact files (from `git status` / `git diff --name-only docs/adr/`):

- `docs/adr/027-targeting-mcp-draft-2026-v1.md` — §Consequences replaced + new §"Status update (2026-05-31)" inserted + revision-log entries (this is the only multi-edit amendment)
- `docs/adr/006-streamable-http-compatibility.md` — append §"DRAFT-2026-v1: Stateless variant; GET SSE is 2025-only"
- `docs/adr/009-protocol-based-handler-routing.md` — append §"DRAFT-2026-v1: `McpProtocolVersion` becomes feature-exclusive"
- `docs/adr/023-tool-change-detection-and-notification.md` — append §"DRAFT-2026-v1: per-request fingerprint persistence"
- `docs/adr/001-lambda-mcp-integration-architecture.md` — append §"Stateless mode (2026-07-28)"
- `docs/adr/025-extract-turul-rpc.md` — revision-log entry
- `docs/adr/026-lambda-streaming-empty-body-contract.md` — revision-log entry (moot in 2026 mode)
- `docs/adr/028-extensions-strategy.md` — revision-log entry

Beyond ADRs, the rollout plan (`docs/plans/2026-07-28-feature-gating-rollout.md`) will, in subsequent slices (NOT this doc-only slice), touch every workspace `Cargo.toml` to add `[features]` entries and the re-export alias in `crates/turul-mcp-protocol/src/lib.rs:1-21`. This summary doc-slice does NOT modify those files.

## 4. Decision-phase evidence (concise)

### 4.1 2026 spec compliance verdict

`turul-mcp-protocol-2026-07-28` is spec-aligned at the wire level: all 22 schema methods bound with correct spellings; `_meta` carriers match the schema's `RequestMetaObject` (required `io.modelcontextprotocol/protocolVersion`, `clientInfo`, `clientCapabilities`; optional `progressToken`, `log_level`); 342 tests pass (159 lib + 179 integration + 3 fixtures + 1 doctest, 0 warnings) per `crates/turul-mcp-protocol-2026-07-28/COMPLIANCE.md:1-230`; eight schema-fidelity defects A1–A8 are fixed with regression tests; SEP-2577 deprecation annotations on Roots/Sampling/Logging are in place; wire version string is `"DRAFT-2026-v1"`. **However**, the framework consumer crates (server, client, HTTP transport, examples, derive) have NOT migrated to the 2026 types — `crates/turul-mcp-protocol/src/lib.rs:1-21` still re-exports 2025-11-25. **Without Phase 9.4 alias flip, default=2026 is a protocol-crate-only claim; the framework cannot actually speak 2026-07-28 end-to-end.**

### 4.2 Devils-advocate stress test

**Verdict: `should-reconsider` (now resolved).** The original devils-advocate verdict flagged Phase 9.4 (alias flip) as a hard dependency of 0.4.0, not a deferred slice. The strategy is now committed in ADR-029 §"What the cutover slice ships" item 5: **flip-all-at-once**, with atomicity enforced by `compile_error!` guards on the feature-gated re-export. The earlier three-way choice (flip-all-at-once vs dual-import vs crate-by-crate) is closed. Remaining risks documented below: `legacy-2025-11-25` feature still lacks end-to-end tests (must be added in `docs/plans/2026-07-28-feature-gating-rollout.md` Phase 7); upstream RC churn unbounded (mitigation via `cargo run -p turul-mcp-protocol-2026-07-28 --bin mcp-compliance-2026-07-28 -- refresh`); CI surface doubling is now explicit (legacy matrix planned in rollout Phase 7).

### 4.3 Client architecture choice

**Bilingual default.** The server has a concrete architectural constraint: one process cannot maintain two protocol state machines simultaneously (2025-11-25's stateful `initialize` + `Mcp-Session-Id` header vs 2026-07-28's stateless `server/discover` + per-request `_meta`). The client has NO such constraint — it emits/receives bytes per-connection, and the Transport abstraction already separates transport choice from protocol semantics. ADR-030 proposes: compile both `turul-mcp-protocol-2025-11-25` and `turul-mcp-protocol-2026-07-28` into the client by default; per-connection version negotiation via (a) explicit `ConnectionConfig.mcp_protocol_version` hint, (b) try-`server/discover`-then-fallback-to-`initialize` auto-detect, (c) immutable per-connection lock once chosen; opt-in `client-2025-only` / `client-2026-only` cargo features for embedded/wasm binary-size narrowing only. Estimated ~1300–2000 LOC of additive code in `crates/turul-mcp-client/src/protocol/{mod,v2025,v2026}.rs` and `src/version.rs`. No breaking changes at initial release; the breaking change lands when ADR Phase 9.4 flips the `turul-mcp-protocol` alias.

## 5. Key technical risks (top 5, severity-ordered)

1. **[CLOSED] Phase 9.4 alias flip strategy.** ~~Was a blocker pending strategy choice (flip-all-at-once vs dual-import vs crate-by-crate).~~ **Resolved in ADR-029 §"What the cutover slice ships" item 5: flip-all-at-once.** Atomicity is enforced by the `compile_error!` guards on the feature-gated re-export at `crates/turul-mcp-protocol/src/lib.rs`. The remaining work — actually executing the flip + migrating consumer crates — is scoped in `docs/plans/2026-07-28-feature-gating-rollout.md` Phases 0-7 and is still required-before-0.4.0-publication; it is no longer an open *strategy* decision.
2. **[HIGH] Downstream consumer migration not started.** `turul-mcp-server`, `turul-mcp-client`, `turul-http-mcp-server`, 55+ examples, and derive macros still use 2025-11-25 types; the framework cannot actually serve or speak 2026-07-28. *Mitigation: ADR-029 + feature-gating-rollout.md scope this work; codex should verify the plan is executable in the actual coupling that exists in the code.*
3. **[HIGH] Upstream RC churn unbounded.** The DRAFT-2026-v1 schema has churned 8 times already this month; ETag will flip again before final 2026-07-28 publication; shipping 0.4.0 against a draft exposes users to type breakage. *Mitigation: CHANGELOG must mark 0.4.0 as targeting DRAFT spec; reserve 0.4.1 type-shift headroom; OR defer 0.4.0 until final spec publishes.*
4. **[HIGH] `legacy-2025-11-25` feature flag is untested.** No end-to-end tests, no CI matrix coverage, no integration tests. Users opting into the fallback hit untested code. *Mitigation: feature-gating-rollout.md must include CI matrix that exercises BOTH `--features legacy-2025-11-25` and default paths through server+client+example flows.*
5. **[MEDIUM] Workspace `turul-rpc` version skew (0.1 → 0.2.2).** The 2026-07-28 protocol crate isolates `turul-rpc` 0.2.2 per ADR-025; the rest of the workspace pins 0.1. Until bulk migration lands atomically with Phase 9.4, transitive dep conflicts loom. *Mitigation: scope bulk pin migration in PARKED.md addendum and land atomically with the cutover.*

(Lower-severity items from the agent reports: ContentBlock union over-modeling, ResourceReference/Resource duplication, 78/86 fixture coverage gap, `turul-mcp-ext-tasks-2026-07-28` / `turul-mcp-ext-apps-2026-07-28` extension crates not scaffolded — documented in COMPLIANCE.md §"Known follow-ups", deferrable.)

## 6. Codex-review focus areas

1. **Does ADR-029 cite the right code locations** for the proposed `default = ["protocol-2026-07-28"]` + `legacy-2025-11-25` feature wiring? Specifically: (a) does it reference `crates/turul-mcp-protocol/src/lib.rs:1-21` (the alias re-export site that must change), (b) does it correctly identify the coupling points in `crates/turul-mcp-server`, `crates/turul-http-mcp-server`, and `crates/turul-mcp-builders` where protocol types are imported, (c) does it acknowledge that mutually-exclusive features require `compile_error!` guards?
2. **Is ADR-030's bilingual-detection mechanism actually feasible?** Specifically: (a) does the `server/discover` → fallback-to-`initialize` flow correctly handle JSON-RPC `-32601 Method Not Found` semantics in the existing `crates/turul-mcp-client/src/client.rs` connect path (~lines 135-229 per MEMORY context), (b) does the per-connection `Arc<RwLock<McpVersion>>` proposal compose with the existing `Transport` trait `&self` refactor from v0.3.33, (c) is the ~1300–2000 LOC estimate realistic given the actual size of `crates/turul-mcp-client/src/`?
3. **Do the 8 existing-ADR amendments preserve original ADRs' Status correctly?** The amendments are revision-log appends dated 2026-05-31; codex should verify NO original `Status:`/`Decision:` block was overwritten or silently rewritten, and that each appended entry cites ADR-029/ADR-030 as the downstream cause.
4. **Does `docs/plans/2026-07-28-feature-gating-rollout.md` match the actual coupling in the code?** Specifically: (a) does the migration order respect the actual dep graph (`json-rpc-server → protocol crates → builders → storage → derive → http-server → server → client → aws-lambda → oauth` per CLAUDE.md §"Pre-Release Checklist"), (b) does the CI matrix actually exercise `--features legacy-2025-11-25` end-to-end (not just `cargo check`), (c) does the plan scope the `turul-rpc` 0.1 → 0.2.2 bulk pin migration as a prerequisite?
5. **Are there spec-2026 features still missing that block default-2026?** Cross-check `crates/turul-mcp-protocol-2026-07-28/COMPLIANCE.md` §"Known follow-ups" against ADR-029's "what 0.4.0 ships" statement. Specifically: are `turul-mcp-ext-tasks-2026-07-28` (SEP-2663) and `turul-mcp-ext-apps-2026-07-28` (SEP-1865) correctly scoped as out-of-0.4.0 with explicit release-notes language, or are they implicit-and-broken?
6. **Does the doc-slice respect the Branch Lock?** Per CLAUDE.md §"Branch Lock", this slice must not merge → main, must not rebase-onto-main, must not be treated as "complete" without express maintainer authorization. The ADR + plan files land on `feat/turul-mcp-protocol-2026-07-28`, NOT main. Codex should verify the slice does not include any unauthorized cutover work.

## 7. Open questions for the maintainer (not codex)

1. ~~**Phase 9.4 cutover strategy:** flip-all-at-once, dual-import, or crate-by-crate?~~ **CLOSED.** ADR-029 §"What the cutover slice ships" item 5 commits to **flip-all-at-once** (atomicity enforced by `compile_error!` guards on the feature-gated re-export). The three options ADR-027 originally flagged collapse to one under the user-locked default-2026 decision. This is no longer an open question.
2. **0.4.0 release timing vs final 2026-07-28 spec publication:** ship 0.4.0 against DRAFT (accepting type-shift risk in 0.4.1), or hold 0.4.0 until upstream publishes final 2026-07-28 and the ETag stabilizes?
3. **`legacy-2025-11-25` feature scope:** is it a real escape hatch (must be CI-tested) or a documentation-only feature flag (warn users it's untested)?
4. **Extension crates (`turul-mcp-ext-tasks-2026-07-28`, `turul-mcp-ext-apps-2026-07-28`):** scaffold alongside 0.4.0, or defer to 0.4.x point releases? Release-notes language depends on the answer.
5. **CI matrix doubling:** if default=2026 + opt-in legacy is the strategy, CI must exercise both paths. Acceptable cost in build time?

## 8. Roadmap (what happens after this slice lands)

1. Commit Slice A' + A'' + B (~110 dirty files) as documented in `docs/plans/2026-07-28-PARKED.md` to `feat/turul-mcp-protocol-2026-07-28`.
2. Commit this doc slice (ADR-029, ADR-030, `2026-07-28-architecture-review.md`, `2026-07-28-feature-gating-rollout.md`, 8 existing-ADR revision-log amendments) to the same branch.
3. Maintainer decides timing of 0.4.0 vs final spec publication (per §7 question 2). Phase 9.4 cutover strategy is already decided per ADR-029 §"What the cutover slice ships" item 5 (flip-all-at-once); no further maintainer input needed on that axis.
4. Scope and land the `turul-rpc` 0.1 → 0.2.2 workspace bulk pin migration as a prerequisite slice (independently mergeable per PARKED.md).
5. Wire cargo feature gates per `docs/plans/2026-07-28-feature-gating-rollout.md` across the workspace (server, client, http-server, builders, derive, examples).
6. Add CI matrix for `--features legacy-2025-11-25` end-to-end coverage (per §6 focus area 4, §7 question 3, §7 question 5).
7. Execute Phase 9.4 alias flip **per ADR-029 §"What the cutover slice ships" item 5: flip-all-at-once**; migrate framework consumer crates to `turul-mcp-protocol-2026-07-28` types in one atomic slice (atomicity enforced by the feature-gated re-export's `compile_error!` guards).
8. (Optional) Scaffold `turul-mcp-ext-tasks-2026-07-28` and `turul-mcp-ext-apps-2026-07-28` extension crates per §7 question 4.
9. Maintainer authorizes branch merge → main, version bumps, CHANGELOG entries, and publish per CLAUDE.md §"Pre-Release Checklist" + Branch Lock release.
