# ADR-027: Targeting MCP `DRAFT-2026-v1`; regenerate on final spec

**Status:** Accepted (in-flight)
**Date:** 2026-05-24
**Crate:** `turul-mcp-protocol-2026-07-28`
**Branch:** `2026-07-28-MCP-Specification` (and sub-branches off it)
**Related:** ADR-001 (protocol-alias-usage), the maintainer "Branch Lock" in `CLAUDE.md`/`AGENTS.md`

## Context

The maintainer authorized work on adopting the MCP 2026-07-28 release candidate (blog post: <https://blog.modelcontextprotocol.io/posts/2026-07-28-release-candidate/>). The crate `turul-mcp-protocol-2026-07-28` (v0.4.0) was scaffolded as a fork of `turul-mcp-protocol-2025-11-25` source.

When the upstream draft schema was vendored on 2026-05-24 (`crates/turul-mcp-protocol-2026-07-28/schema/draft-schema.ts`, upstream ETag `8bdd4ae5a9b3a8d2e611124cf7240d60cf0fcece9652eae51571ba5f1be0e0ef`), two things became clear:

1. The schema is explicitly marked draft. Its `LATEST_PROTOCOL_VERSION` constant is `"DRAFT-2026-v1"`, **not** `"2026-07-28"`. The blog post describes a release candidate with RC lock on 2026-05-21 and final publication on 2026-07-28, but the *wire* version string in the draft is the `DRAFT-2026-v1` marker.
2. The draft schema continues to evolve until RC lock and again between RC and final. Any Rust types we produce against it today may need adjustment when the final ships.

We must commit to one wire string and one regeneration trigger. We cannot hold both `"DRAFT-2026-v1"` and `"2026-07-28"` simultaneously without bespoke version-negotiation logic, and adding such logic now would be speculative.

## Decision

### Wire-string target: `"2026-07-28"` (finalized; was `"DRAFT-2026-v1"`)

The crate advertises and accepts the wire protocol version string exactly as the vendored schema declares it. Upstream finalized `LATEST_PROTOCOL_VERSION` from `"DRAFT-2026-v1"` to the stable date literal `"2026-07-28"` (re-pinned 2026-06-07 — see revision log). This is reflected in:

- `crates/turul-mcp-protocol-2026-07-28/src/version.rs` — `McpVersion::V2026_07_28` serde-renames to `"2026-07-28"` with `alias = "DRAFT-2026-v1"`; `FromStr` parses both (the draft literal is accepted on deserialize for back-compat).
- `crates/turul-mcp-protocol-2026-07-28/src/lib.rs::MCP_VERSION` — `"2026-07-28"`.
- `crates/turul-mcp-protocol-2026-07-28/schema/draft-schema.ts` — source-of-truth, vendored with ETag for provenance.

The Rust identifier `McpVersion::V2026_07_28` stays put. Only the serde rename and `MCP_VERSION` constant changed when the wire string flipped. The crate name itself (`turul-mcp-protocol-2026-07-28`) was chosen for the projected publication date and matches the finalized wire string.

### Regeneration trigger

The crate must be re-aligned with upstream **whenever** any of the following happens:

1. **Upstream `schema.ts` ETag changes.** The vendored ETag is recorded in `schema/README.md`. If a re-fetch yields a different ETag, types may have drifted and compliance tests must be re-run.
2. **Final 2026-07-28 specification publishes.** At that point: re-vendor as `schema/2026-07-28-schema.ts` (alongside the draft, do not delete the draft until tests pass against the new file), update `MCP_VERSION` and the serde rename to the new wire string (most likely `"2026-07-28"`), update compliance tests, bump crate version to `0.4.x+1`.
3. **A compliance test fails after re-vendoring.** Treat that as the canonical signal that types must change. Do not silence the test.

### What does NOT trigger regeneration

- Internal refactors (file moves, identifier renames) that don't change serialized shape.
- Workspace version bumps that don't touch this crate.
- Doc-only changes upstream (e.g. tightening prose without changing types).

When in doubt, re-vendor and re-run `cargo test -p turul-mcp-protocol-2026-07-28`. The compliance test suite is the contract.

### Capability flags in `McpVersion`

The flags in `src/version.rs` (e.g. `supports_streamable_http`, `supports_tasks`) describe what each spec version offers, not what *this crate* implements. While the implementation is in progress, the flag for `V2026_07_28` may temporarily disagree with the literal `MCP_VERSION` constant — that is acceptable transitional state but every disagreement must be tracked as an open task in `docs/plans/2026-07-28-compliance-plan.md`.

### Crate version

`turul-mcp-protocol-2026-07-28` is independently versioned starting at **0.4.0** (literal `version = "0.4.0"` in its `Cargo.toml`, not `version.workspace = true`). This is the first instance of per-crate version independence in the workspace. Policy: at the 0.4.0 release moment, all non-frozen crates synchronize to 0.4.0; after that release, individual crates may drift independently. The frozen `2025-06-18` and `2025-11-25` protocol crates stay at `0.3.47` indefinitely. See revision log entry **2026-05-31** for the one-time frozen-manifest touch that landed alongside the policy adoption.

## Consequences

- **0.4.0 ships with DRAFT-2026-v1 as the default and `legacy-2025-11-25` as an opt-in cargo feature.** Phase 9.4 (the alias cutover) is **part of 0.4.0**, not a deferred slice. The `turul-mcp-protocol` re-export crate flips its `pub use` to `turul-mcp-protocol-2026-07-28` in the same release. Consumers that need the prior spec enable `--features legacy-2025-11-25` on `turul-mcp-server`, `turul-mcp-client`, etc., which flips the re-export back to `turul-mcp-protocol-2025-11-25`. Both versioned crates stay in the workspace; the feature flag selects which one the alias points to. See ADR-029.
- **0.4.0 is internal-only while the RC is unstable.** The 2026-07-28 RC schema (ETag-pinned `c3e3f09e…`) is the wire string `"DRAFT-2026-v1"` and the upstream schema continues to evolve until RC lock. 0.4.0 is **not published to crates.io** in this state — work happens on the `2026-07-28-MCP-Specification` branch behind the maintainer's branch-lock. Publication of 0.4.0 is gated on (a) upstream stabilization (RC lock or final 2026-07-28 publication), (b) the maintainer's explicit go-ahead, and (c) Phase 9.4 cutover completion in the framework consumer crates.
- **Compliance tests are the contract.** They live in `crates/turul-mcp-protocol-2026-07-28/tests/compliance.rs` and must pass against `schema/draft-schema.ts` at the recorded ETag. CI will eventually gate publication on a matching ETag.
- **Frozen crates** (`turul-mcp-protocol-2025-11-25`, `turul-mcp-protocol-2025-06-18`) remain at `0.3.47` with no further edits beyond the one-time literal-`version` pin documented in the revision log.
- **Branch lock** on `2026-07-28-MCP-Specification` stays in force. Sub-branches off it (one per compliance slice) may merge back freely; only the branch → `main` direction is locked.
- **Client gets its own ADR.** `turul-mcp-client` connecting to mixed-spec servers (2025-11-25 vs DRAFT-2026-v1) is a distinct concern — version detection, fallback behavior, and per-connection routing are documented in ADR-030 (bilingual default).

## Status update (2026-05-31)

Four user-locked architectural decisions revise this ADR's original deferral posture:

1. **Server default = DRAFT-2026-v1.** Opt-in cargo feature `legacy-2025-11-25` on consumer crates (`turul-mcp-server`, `turul-mcp-client`, `turul-http-mcp-server`, `turul-mcp-aws-lambda`) flips the `turul-mcp-protocol` re-export back to the 2025-11-25 crate for callers who still need it. The default (no features) is DRAFT-2026-v1.
2. **0.4.0 ships with default = 2026.** The earlier framing of "flip the alias is a deliberate later slice" / "defer to 0.5.0 cutover" is **superseded**. Phase 9.4 (alias flip) is in scope for the 0.4.0 release.
3. **NOT publishing while RC is unstable.** 0.4.0 is internal work-in-progress on the feature branch. Crates.io publication is gated on upstream RC stabilization or final spec publication AND the maintainer's explicit go-ahead. No 0.4.x patch crate ships to crates.io until that gate clears.
4. **All planning and decision docs live under `docs/`.** PARKED.md, plan documents, compliance docs, and ADR amendments are permanent records under `docs/plans/` and `docs/adr/`. Nothing important lives in `/tmp`.
5. **`turul-mcp-client` gets its own ADR** (ADR-030). Because the client can connect to either spec without an architectural lock (HTTP transport is bidirectional bytes, not a process-global state machine), the client-side version-handling story is recorded in a dedicated ADR rather than as a paragraph in this one.

### Open prerequisites for shipping 0.4.0 (not blockers for the branch, are blockers for publication)

- **Phase 9.4 cutover work** in framework consumer crates (`turul-mcp-builders`, `turul-mcp-server`, `turul-mcp-client`, `turul-http-mcp-server`, `turul-mcp-aws-lambda`, all 55+ examples, derive macros). Status: parked. Will be re-scoped when the publication gate is reopened. See `docs/plans/2026-07-28-feature-gating-rollout.md`.
- **Workspace turul-rpc 0.1 → 0.2.x bulk migration.** Currently isolated to `turul-mcp-protocol-2026-07-28` and `turul-mcp-client` per ADR-025 revision log. The framework-wide bump lands atomically with Phase 9.4.
- **`legacy-2025-11-25` feature flag end-to-end coverage.** The opt-in legacy path needs at least one CI matrix run that builds + tests the framework with `--features legacy-2025-11-25` to prove the fallback compiles and the spec contract still passes for any consumer that opts in.
- **Optional extension crates** (`turul-mcp-ext-tasks-2026-07-28` per SEP-2663, `turul-mcp-ext-apps-2026-07-28` per SEP-1865) per ADR-028. Not blockers for 0.4.0 publication — the release notes will state that tasks and apps support require the respective extension crates (scaffolded post-0.4.0 unless prioritized).

## Open items

Live status is in `docs/plans/2026-07-28-compliance-plan.md`. Per-symbol coverage is in `docs/plans/2026-07-28-schema-coverage-matrix.md`. The "open items" that remain after the initial compliance push:

- Phase 5.2 — `turul-mcp-ext-tasks-2026-07-28` crate scaffolding (per ADR-028)
- Phase 5.3 — `turul-mcp-ext-apps-2026-07-28` crate scaffolding (per ADR-028)
- Phase 7 finalization — tighten transitional `Option<…>` cache fields to required when consumer paths are ready
- Phase 1.1b — refine `JsonRpcResponse` into separate Success/Error structs, type `RequestId` strictly
- Phase 9.4 — flip the `turul-mcp-protocol` alias from `2025-11-25` to `2026-07-28`. **Strategy committed per ADR-029: flip-all-at-once** (atomicity enforced by `compile_error!` guards on the feature-gated re-export). The three options this ADR originally flagged (flip-all-at-once / dual-import / crate-by-crate) collapse to one under the user-locked default-2026 decision; see ADR-029 §"What the cutover slice ships" item 5. Cascades through every downstream consumer.

## Revision log

- **2026-05-24** — initial. Wire string set to `"DRAFT-2026-v1"` per vendored draft schema.
- **2026-05-24** — type-migration substantially complete. 288 tests passing covering: foundational (json-rpc/meta/result-type/error-codes/input-required), stateless core (discover.rs, capabilities reshape, removed-methods drift detectors), per-area (tools/resources/prompts/elicitation/completion/content/empty-result), schema-removed type deletions (tasks.rs gone; PingRequest/SetLevelRequest/InitializedNotification/RootsListChanged*/TaskStatus*/Subscribe*/Unsubscribe*/InitializeRequest+Result/TasksXxxCapabilities/TaskSupport/ToolExecution all deleted), ToolSchema retyped for 2020-12 conformance, ToolOutputSchema added separately. Independent verification agent confirmed parity with 2025-11-25 approach and identified doc-drift items (now fixed).
- **2026-05-24** — ADR-028 (extensions strategy) authored per SEP-2133/SEP-2663 verified content. Phase 5 plan items added.
- **2026-05-31** — **Per-crate independent-versioning policy adopted.** Workspace cut from `0.3.x` to `0.4.0` (already landed on this branch in commit `064733e` / `c0737fb`). Every non-frozen crate's `Cargo.toml` migrated from `version.workspace = true` to a literal `version = "0.4.0"`. Frozen `2025-06-18` and `2025-11-25` manifests each received a single one-time literal pin to `version = "0.3.47"` (no other field touched) — without this they would inherit the new `[workspace.package].version = "0.4.0"`, which would silently bump the published version of the historical spec snapshots they are explicitly frozen against. The frozen-crate touch is bounded by this one-line policy adoption and does not reopen them to ongoing edits.
- **2026-05-31** — **Slice A' (schema-fidelity corrections)** landed against the 2026-07-28 protocol crate. Eight defects surfaced by an internal review and fixed:
   - **A1 — `meta::ProgressToken`** unified to untagged `string | number` enum (was String-only newtype). Single canonical type now used at both `RequestMetaObject.progressToken?` and `ProgressNotificationParams.progressToken`.
   - **A2 — `SubscriptionsListenRequestParams._meta`** upgraded from `Option<HashMap>` to required typed `RequestMetaObject`. Now matches sibling `RequestParams` extenders.
   - **A3 — `CancelledNotificationParams.request_id`** relaxed from required to `Option<RequestId>` per schema (`requestId?`).
   - **A4 — `ContentBlock::ResourceLink`** extended with `size` and `icons` fields per `ResourceLink extends Resource` (silent wire drop before).
   - **A5 — `ElicitationSchema`** extended with `$schema?: string` for JSON Schema 2020-12 dialect declaration.
   - **A6 — `ListRootsRequest.params`** swapped from bespoke `ListRootsParams { meta: Option<HashMap> }` to schema-anchored `Option<crate::json_rpc::RequestParams>`. The bespoke struct is removed.
   - **A7 — Seven notification traits in `traits.rs`** rebound from `JsonRpcNotificationTrait` to `RpcNotification` (matching the Rust struct split — inner payload only, no envelope) and renamed with a `*Trait` suffix to avoid struct-name collisions. All 7 traits now have live impls in `notifications.rs`.
   - **A8 — New trait coverage for DRAFT-2026 RPCs**: `DiscoverRequestTrait`, `SubscriptionsListenRequestTrait` + `HasSubscriptionsListenParams`, `HasInputRequiredResult` (SEP-2322).
   
   Test count 322 → 343 (+21 regression guards, including scientific-notation + overflow guards added during the P1 verifier sweep). See `crates/turul-mcp-protocol-2026-07-28/COMPLIANCE.md` §"Schema-fidelity corrections (Slice A' follow-up, 2026-05-31)". Note: A1 final form (untagged `String | serde_json::Number`) supersedes the initial `Number(i64)` after codex's P1 follow-up review caught that the latter rejected JSON floats like `1.5` — `serde_json::Number` losslessly preserves any JSON number.
- **2026-05-31** — **Slice A'' (SEP-2577 deprecation annotations)** landed. Roots, Sampling, and Logging features are now marked `#[deprecated(since = "0.4.0", note = "...")]` at every type definition site. Earliest removal is the first release on/after 2027-07-28; annotation-only this revision per SEP-2577. `LoggingLevel` (the value type for the non-deprecated `RequestMetaObject.log_level` replacement) is intentionally NOT deprecated. Pre-existing duplicate `LoggingMessageNotification` definition (`logging.rs` vs `notifications.rs`) reconciled — `logging.rs` duplicate removed, field-getter trait impls moved onto the spec-aligned `notifications::LoggingMessageNotificationParams`. `#[allow(deprecated)]` cascade applied at internal cross-reference sites (notably the SEP-2322 `InputRequest`/`InputResponse` enums that wrap the deprecated types during the migration window). Final test count: 342 (159 lib + 179 + 3 + 1), 0 warnings, 0 doc warnings. See `crates/turul-mcp-protocol-2026-07-28/COMPLIANCE.md` §"SEP-2577 deprecation annotations (Slice A'' follow-up, 2026-05-31)".

- **2026-05-31** — **Status update: 0.4.0 ships DRAFT-2026-v1 as the default; Phase 9.4 (alias flip) moves INTO 0.4.0; `legacy-2025-11-25` opt-in feature is added on consumer crates.** Earlier deferral phrasing in §Consequences ("Downstream consumers can't yet use this crate. Flipping the alias is a deliberate later slice... depends on this crate reaching full compliance first.") is **superseded** by the §"Status update (2026-05-31)" subsection. The crate IS the new default; consumers don't need to wait. Publication of 0.4.0 to crates.io remains gated on (a) upstream RC stabilization or final 2026-07-28 publication, (b) maintainer explicit go-ahead, (c) Phase 9.4 cutover work completing across consumer crates (parked but tracked). Client-side mixed-spec connectivity moves to a separate dedicated ADR rather than this ADR's scope (see §"Status update (2026-05-31)" #5).

- **2026-06-07** — **Wire string finalized: re-pinned to upstream `"2026-07-28"`.** Triggered by regeneration-trigger #1 (ETag change) and #2 (version-string finalization). Re-vendored `schema/draft-schema.ts` from `modelcontextprotocol/modelcontextprotocol@main` (HTTP ETag `0eeaed15…`, content sha256 `20df36f9…`; was `8bdd4ae5…`). **The 159-symbol export surface and the 22 method strings are byte-for-byte identical to the 2026-05-24 pin** — the stateless core is intact upstream (verified directly against live `main`: no `initialize`/`notifications/initialized`/`ping`/`resources/subscribe`/`resources/unsubscribe`/`tasks/*` reintroduced; live schema line comments confirm `subscriptions/listen` "replaces the former `resources/subscribe` RPC"). Exactly three substantive (non-comment) wire changes carried over and were applied to the bindings:
   1. `LATEST_PROTOCOL_VERSION` `"DRAFT-2026-v1"` → `"2026-07-28"`. `MCP_VERSION`, the `McpVersion::V2026_07_28` serde rename, and `FromStr` updated; the draft literal is retained as a serde `alias` / `FromStr` arm (deserialize-only back-compat).
   2. `ResultType` `"complete" | "input_required"` → **open union** `… | string`. Modeled as `ResultType::Other(String)` with hand-written `Serialize`/`Deserialize` so unknown discriminators round-trip verbatim instead of being rejected. `Copy` dropped (String payload); 11 `HasResultType` impls switched to `.clone()`. Contract-flip tests migrated (`accepts_unknown_discriminator_as_other`, `unknown_result_type_value_is_preserved_as_other`); the prior `rejects_unknown_*` assertions are gone.
   3. `DiscoverResult extends Result` → `extends CacheableResult`. Added required `ttl_ms`/`cache_scope` (camelCase `ttlMs`/`cacheScope`) mirroring `ListToolsResult`'s inline composition; `new()` defaults to immediately-stale public; wire-shape test asserts both keys.
   Also resolved the `clippy::large_enum_variant` gate (Codex P1) on the deprecated MRTR `InputRequest`/`InputResponse` unions via scoped `#[allow]` with rationale (boxing soon-to-be-removed deprecated variants buys no runtime benefit). Result: `clippy -D warnings` clean; 343 tests pass under `--features compliance` (160 lib + 179 + 3 + 1), 333 default. Revert-and-fail verified for the version-string and `ResultType` deltas. **Crate stays at 0.4.0** — this completes the spec line 0.4.0 was created to target (the trigger-#2 "bump to 0.4.x+1" guidance applies only once 0.4.0 has actually published; it has not). The examples fixture pin (`c3e3f09e…`) was left unchanged — the 8 modeled fixtures still round-trip green; a separate examples re-pin is tracked, not bundled here.
