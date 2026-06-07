# ADR-029: Spec-version coexistence via mutually-exclusive cargo features

**Status:** Accepted
**Date:** 2026-05-31
**Crate:** `turul-mcp-protocol` (re-export crate) — cascades to every downstream consumer of the alias
**Branch:** `2026-07-28-MCP-Specification` (and sub-branches off it; current work on `feat/turul-mcp-protocol-2026-07-28`)
**Related:** ADR-027 (DRAFT-2026-v1 wire-string target), ADR-028 (extensions strategy), ADR-030 (client coexistence — bilingual by default), ADR-001 (`turul-mcp-protocol` alias usage), the maintainer "Branch Lock" in `CLAUDE.md`/`AGENTS.md`

## Context

`turul-mcp-protocol-2026-07-28` (v0.4.0) now passes 342 tests (159 lib + 179 integration + 3 fixture + 1 doctest), zero warnings, against the vendored `2026-07-28` schema (see `crates/turul-mcp-protocol-2026-07-28/COMPLIANCE.md:1-26`). All 22 schema-declared methods are bound at the canonical wire spelling; `_meta` carriers match the schema shape exactly; SEP-2577 deprecations are annotated on Roots/Sampling/Logging. The protocol crate is spec-aligned.

The framework crates that consume it are not. `turul-mcp-server`, `turul-http-mcp-server`, `turul-mcp-client`, `turul-mcp-aws-lambda`, the derive macros, and ~55 example crates still depend on `turul-mcp-protocol` — and that alias still re-exports `turul-mcp-protocol-2025-11-25` (see `crates/turul-mcp-protocol/src/lib.rs:71-76`). ADR-027 Phase 9.4 ("flip the alias") was originally parked pending a strategy decision; **this ADR makes that decision**: flip-all-at-once via mutually-exclusive cargo features at the alias boundary (see §"What the cutover slice ships" item 5 below).

### Why coexistence, not "just flip it"

The 2025-11-25 and 2026-07-28 specifications are not wire-format variants of the same protocol. They are different protocols. The cleavage points that prevent a single process from speaking both simultaneously:

1. **Stateful vs stateless core.** 2025-11-25 requires `initialize` → `notifications/initialized` → `Mcp-Session-Id` header on every subsequent request. The 2026-07-28 stateless core replaces `initialize` with `server/discover` (un-sessioned — `InitializeRequest` is absent from the pinned schema) and every request carries `_meta.io.modelcontextprotocol/clientInfo` + `_meta.io.modelcontextprotocol/clientCapabilities` (see `crates/turul-mcp-protocol-2026-07-28/COMPLIANCE.md:44-56` for the `_meta` carrier rules). A server cannot serve both handshakes through one router without bespoke per-request routing.
2. **`_meta` shape required, not optional.** `RequestParams._meta: RequestMetaObject` is REQUIRED in 2026-07-28, with three required namespaced fields (`io.modelcontextprotocol/protocolVersion`, `clientInfo`, `clientCapabilities`). In 2025-11-25 the equivalent fields are positional in the `initialize` payload. The Rust `RequestParams` type literally cannot be both at once.
3. **JSON-RPC error code drift.** 2026-07-28 uses `-32602` for invalid-params (`-32002` reserved differently). 2025-11-25 used `-32002`. A handler dispatcher binding both at once would serve the wrong error code to the wrong client.
4. **Tasks moved to extension (SEP-2663).** 2025-11-25 has tasks in core. 2026-07-28 moves them out of core to an extension — the protocol crate has no `Task`/`TaskStatus`/`TasksCapabilities`/`tasks/*` methods (see `docs/adr/028-extensions-strategy.md:21-28`). Tasks-using code authored against 2025 will not compile against 2026; it must move to the (not-yet-scaffolded) `turul-mcp-ext-tasks-2026-07-28` crate.

These are architectural differences, not surface deltas. The framework must pick one, per process. The decision space is therefore: which to pick by default, and how the other survives as an escape hatch.

### Constraints feeding the decision

- **Not publishing while RC is unstable.** `DRAFT-2026-v1` schema continues to evolve (Slice A' caught eight schema-fidelity defects on 2026-05-31 alone; see `docs/adr/027-targeting-mcp-draft-2026-v1.md:79-89`). The ETag will change again between now and final 2026-07-28 publication. Internal-only feature-branch work eliminates accidental-pick risk from crates.io.
- **User-locked decision.** Server default is `2026-07-28`. `0.4.0` ships with default = 2026. No "0.5.0 deferred cutover" — that artificial deferral is closed. The escape hatch for 2025-11-25 is a cargo feature, opt-in.
- **Client lives under a different constraint.** Clients can speak both protocols per-connection (no process-wide state machine collision). Coexistence for the client is "bilingual by default" — see ADR-030.

## Decision

### The mechanism: mutually-exclusive cargo features at the `turul-mcp-protocol` boundary

The `turul-mcp-protocol` re-export crate (`crates/turul-mcp-protocol/src/lib.rs`) is the single point where the framework chooses which versioned protocol crate it speaks. We replace the unconditional `pub use turul_mcp_protocol_2025_11_25::*;` (line 71) with feature-gated re-exports:

```rust
// crates/turul-mcp-protocol/src/lib.rs (post-cutover)

#[cfg(all(feature = "protocol-2026-07-28", feature = "protocol-2025-11-25"))]
compile_error!(
    "turul-mcp-protocol: features `protocol-2026-07-28` and `protocol-2025-11-25` \
     are mutually exclusive. Pick exactly one."
);

#[cfg(not(any(feature = "protocol-2026-07-28", feature = "protocol-2025-11-25")))]
compile_error!(
    "turul-mcp-protocol: enable exactly one of `protocol-2026-07-28` (default) \
     or `protocol-2025-11-25`."
);

#[cfg(feature = "protocol-2026-07-28")]
pub use turul_mcp_protocol_2026_07_28::*;

#[cfg(feature = "protocol-2026-07-28")]
pub mod prelude {
    pub use turul_mcp_protocol_2026_07_28::prelude::*;
}

#[cfg(feature = "protocol-2025-11-25")]
pub use turul_mcp_protocol_2025_11_25::*;

#[cfg(feature = "protocol-2025-11-25")]
pub mod prelude {
    pub use turul_mcp_protocol_2025_11_25::prelude::*;
}
```

`Cargo.toml`:

```toml
[features]
default = ["protocol-2026-07-28"]
protocol-2026-07-28 = ["dep:turul-mcp-protocol-2026-07-28"]
protocol-2025-11-25 = ["dep:turul-mcp-protocol-2025-11-25"]
```

Both versioned crates are `optional = true` workspace dependencies. The compile-time `compile_error!` macros guarantee that any consumer who manages to enable both, or neither, gets a build-time diagnostic — not a silent type-soup link error.

### Default = `protocol-2026-07-28`

Per the user-locked decision: `0.4.0` defaults to 2026-07-28. A bare `turul-mcp-protocol = "0.4"` dependency in a downstream `Cargo.toml` resolves to the 2026 types. The `protocol-2025-11-25` feature is the explicit opt-out for consumers who need to stay on the old spec without pinning to the frozen `0.3.x` line.

### Cascade rule for the downstream framework crates

Every framework crate that depends on `turul-mcp-protocol` (`turul-mcp-server`, `turul-http-mcp-server`, `turul-mcp-builders`, `turul-mcp-aws-lambda`, `turul-mcp-derive`, `turul-mcp-session-storage`, `turul-mcp-task-storage`, examples) MUST declare its dependency as `{ workspace = true, default-features = false }` and MUST forward the spec choice explicitly through its own features. A bare `turul-mcp-protocol.workspace = true` is forbidden on this branch: under Cargo feature unification it pins `protocol-2026-07-28` ON for every transitive consumer (via the alias crate's default features), so any attempt to enable `protocol-2025-11-25` at the leaf would activate BOTH features simultaneously and trip the `compile_error!` mutex at line 41 of `crates/turul-mcp-protocol/src/lib.rs`. A leaf binary's `--no-default-features` only disables the leaf's own default features; it does not cascade through transitive deps.

Each consumer crate's `[features]` section therefore looks like:

```toml
[features]
default = ["protocol-2026-07-28"]
protocol-2026-07-28 = ["turul-mcp-protocol/protocol-2026-07-28"]
protocol-2025-11-25 = ["turul-mcp-protocol/protocol-2025-11-25"]
```

And its dep line is:

```toml
turul-mcp-protocol = { workspace = true, default-features = false }
```

This topology means the bare `cargo build` path still picks 2026 (via the consumer crate's own default), and switching to legacy is `cargo build --no-default-features --features protocol-2025-11-25` at the leaf — which propagates through the forwarding chain. The `compile_error!` mutex is satisfied in both configurations because exactly one feature reaches the alias crate.

Crates that aggregate multiple framework crates (e.g. a Lambda binary depending on both `turul-mcp-server` and `turul-mcp-aws-lambda`) MUST forward `protocol-2025-11-25` to every transitive turul crate they depend on:

```toml
[features]
default = ["protocol-2026-07-28"]
protocol-2026-07-28 = [
  "turul-mcp-server/protocol-2026-07-28",
  "turul-mcp-aws-lambda/protocol-2026-07-28",
]
protocol-2025-11-25 = [
  "turul-mcp-server/protocol-2025-11-25",
  "turul-mcp-aws-lambda/protocol-2025-11-25",
]
```

`turul-mcp-client` is the exception to this cascade. It compiles both versioned crates simultaneously (bilingual by default — see ADR-030) and depends on the versioned protocol crates directly, not through the `turul-mcp-protocol` alias. The cascade rule above does not apply to it; ADR-030 §"Feature topology" governs the client manifest.

### What the cutover slice ships

The atomic cutover slice (the "flip" of ADR-027 Phase 9.4) ships:

1. The feature-gated `crates/turul-mcp-protocol/src/lib.rs` shown above, plus `Cargo.toml` features.
2. Workspace-wide `turul-rpc` pin bumped from `0.1` to `0.2.2` (currently isolated to `turul-mcp-protocol-2026-07-28`; required for the 2026 protocol types to flow through every consumer — see `docs/plans/2026-07-28-PARKED.md:121`).
3. Source updates in every consumer crate for the breaking surface deltas: `initialize` handshake → `server/discover`; `Mcp-Session-Id` header path → `_meta`-carried capability handshake; error code `-32002` → `-32602`; tasks calls routed to `turul-mcp-ext-tasks-2026-07-28` (scaffolding follows; until then, tasks-using code is gated behind `protocol-2025-11-25`).
4. Roots/Sampling/Logging deprecation cascade: `#[allow(deprecated)]` on every framework-internal site that touches those types (consumers see the `#[deprecated]` warnings; the framework itself does not emit them spuriously).
5. The Phase 9.4 strategy commitment: **flip-all-at-once.** The three options ADR-027 flagged (flip-all-at-once / dual-import / crate-by-crate) collapse to one under the user-locked decision — dual-import contradicts "one source of truth per process," crate-by-crate contradicts "0.4.0 ships with default = 2026 today, not staged." The atomicity is enforced by the `compile_error!` macros: a half-migrated workspace will not compile.

### Feature-gating rollout plan

A separate plan document (`docs/plans/2026-07-28-feature-gating-rollout.md`, to be authored as part of the cutover slice) enumerates the per-crate `#[cfg(feature = "...")]` gates needed for downstream sources to compile under both feature configurations. Initial estimate: ~400–600 `#[cfg]` gates across `turul-mcp-server`, `turul-http-mcp-server`, `turul-mcp-builders`, derive macros, and the example fleet. The plan document is the verification artifact; the CI matrix (two configurations: default and `--no-default-features --features protocol-2025-11-25`) is the gate.

### CI surface

CI will run two matrices for each PR touching the protocol surface:

1. **Default (`protocol-2026-07-28`):** `cargo test --workspace`, `cargo check --workspace`, `cargo doc --no-deps --workspace`.
2. **Legacy (`protocol-2025-11-25`):** the legacy feature is declared on every consumer crate (per the §"Cascade rule" above), so the CI command activates each leaf's own forwarding feature, NOT the alias-crate feature directly. The correct invocations are:

   ```bash
   # Workspace-wide: every member crate that has a protocol-2025-11-25 feature
   # forwards through to the alias. Cargo unification activates the chain.
   cargo test --workspace --no-default-features --features protocol-2025-11-25

   # Or target a specific leaf:
   cargo test -p turul-mcp-server --no-default-features --features protocol-2025-11-25
   cargo test -p turul-mcp-aws-lambda --no-default-features --features protocol-2025-11-25
   ```

   ❌ **Do not use** `--features turul-mcp-protocol/protocol-2025-11-25` (the alias-crate feature only). That activates the alias's gate but does NOT activate each consumer crate's own `#[cfg(feature = "protocol-2025-11-25")]` blocks — those gates live in the leaf crate's source and require the leaf crate's feature to be enabled, not just the alias's.

Without the legacy matrix, the legacy feature rots silently (called out in the architecture review at `docs/plans/2026-07-28-architecture-review.md` as a high-severity hidden risk). The matrix doubles CI time on the protocol surface; this is the accepted cost.

## Alternatives considered

The architecture-review phase enumerated five coexistence patterns. The mutually-exclusive cargo feature mechanism (this ADR, internally tagged "Pattern A") was chosen; the rejected alternatives:

- **Pattern B — Runtime routing inside one process.** A single binary speaks both 2025-11-25 and DRAFT-2026 simultaneously, picking the handshake per-request from header sniffing. Rejected: the four cleavage points (stateful vs stateless, `_meta` required vs optional, error code drift, Tasks-in-core vs extension) are runtime-incompatible. Two state machines in one process means two `RequestParams` types in one process means type duplication on every dispatch site. ~3× the code complexity vs feature-gating.
- **Pattern C — Separate published crates per spec.** Ship `turul-mcp-server-2025` and `turul-mcp-server-2026` as independent crates. Rejected: doubles the surface area to maintain forever; consumers can't transition a binary by toggling a feature, they must rewrite imports. Worsens the "single source of truth" problem instead of solving it.
- **Pattern D — Adapter layer translating wire shapes.** A `turul-mcp-protocol-compat` crate sitting above both protocol crates, exposing a normalized API. Rejected: the cleavage points are NOT translatable — there is no normalized `_meta` that is required-on-2026 and absent-on-2025 simultaneously. An adapter that hides the difference would lie about the wire contract.
- **Pattern E — Hard cutover, no legacy feature.** Drop 2025-11-25 entirely. `0.4.0` is 2026-only. Anyone on 2025 pins to `0.3.x` permanently. Rejected (for now) on the grounds that the legacy feature provides a documented, in-tree escape hatch for the small population of consumers who cannot move to 2026 in this release cycle. **However**, if the legacy feature proves untested in practice (a real risk flagged by the architecture review), the next ADR revision may collapse this to Pattern E — the escape hatch must work to justify its existence.

The user-locked steelman from the decision phase (`docs/plans/2026-07-28-architecture-review.md` §"steelman_for_hard_cutover") is the strongest case against this ADR's decision. We accept Pattern A on the condition that CI exercises both matrices; if the legacy matrix doesn't ship in the cutover slice, the legacy feature should be removed.

## Consequences

### Positive

- **Single source of truth per binary.** Each downstream process picks one protocol at build time. No runtime branches, no dispatcher complexity, no two-state-machines-in-one-process hazard.
- **Lambda simplification.** The Lambda handler stack does not multiplex protocols. One Lambda = one feature = one protocol. Cold-start init paths, session-storage shapes, and `Mcp-Session-Id` parsing all collapse to one branch.
- **Honest RC tracking.** The 2026 path is the default; CHANGELOG and release notes can state "this release tracks 2026-07-28 as the primary target." Consumers who pin to 2025-11-25 do so by explicit feature opt-in, not by accident.
- **The `compile_error!` macros prevent silent breakage.** A misconfigured downstream Cargo.toml that ends up with both features (e.g. via transitive cargo unification on a test-only dep) gets a clean diagnostic at build time, not a link error from duplicated symbols.
- **Aligns with ADR-028 extensions strategy.** Extensions (`turul-mcp-ext-tasks-2026-07-28`, `turul-mcp-ext-apps-2026-07-28`) are opt-in by cargo dependency. Spec-version selection is opt-in by cargo feature. Same idiom at both layers.

### Negative

- **2× CI matrix on the protocol surface.** Every PR that touches `turul-mcp-protocol` or any downstream crate must pass both feature configurations. CI runtime roughly doubles for those PRs. Accepted cost; without the second matrix, the legacy feature is unverified.
- **~400–600 `#[cfg(feature = "...")]` gates across the framework.** Tasks-related code paths (still present in 2025-11-25, removed in 2026) are the largest concentration. The session/handshake machinery is the second. Each gate is a maintenance burden; the rollout plan (`docs/plans/2026-07-28-feature-gating-rollout.md`) enumerates them so the burden is bounded and auditable.
- **No incremental migration.** A consumer on 2025-11-25 who wants to move to 2026 must update their source for every breaking surface delta (no `initialize`, `_meta` required, tasks via extension, etc.) in one slice. The cutover is binary at the binary level. Documented in CHANGELOG with migration recipes.
- **Coverage gap inherited.** 78 of 86 upstream fixture directories remain `Kind::NotModeled` in `compliance/coverage.rs` (per `docs/plans/2026-07-28-PARKED.md:119`). Default = 2026 means consumers' wire shape compliance rests on the modeled 8; the unmodeled 78 are best-effort. Documented in CHANGELOG and COMPLIANCE.md §"Known follow-ups".

### Neutral

- **Wire string in `turul-mcp-protocol::MCP_VERSION` flips with the feature.** Under default (`protocol-2026-07-28`), it is `"2026-07-28"` (the wire string finalized; the earlier `"DRAFT-2026-v1"` snapshot label is retained only as a deserialize-only alias — see ADR-027). Under `protocol-2025-11-25`, it is `"2025-11-25"`. Consumers reading `MCP_VERSION` at runtime see the spec their binary was built against.
- **Frozen crates unaffected.** `turul-mcp-protocol-2025-11-25@0.3.47` and `turul-mcp-protocol-2025-06-18@0.3.47` remain frozen per CLAUDE.md §"Frozen Protocol Crates". The `protocol-2025-11-25` feature pulls in the frozen `0.3.47` crate; it does not reopen those crates to ongoing edits.
- **Per-crate independent versioning (per ADR-027 §Crate version) continues unchanged.** The feature gating lives in the re-export crate; the versioned protocol crates remain on independent semvers.

## Connection to sibling ADRs

- **ADR-027 (Targeting DRAFT-2026-v1)** established the wire-string target and the per-crate `0.4.0` version policy. Phase 9.4 ("flip the alias") was the parked open question; this ADR is the answer (flip-all-at-once, gated by mutually-exclusive features, default = 2026).
- **ADR-028 (Extensions strategy)** established that tasks live in `turul-mcp-ext-tasks-2026-07-28` (not in the protocol crate). Under the default feature, framework code that used to call `tasks/list` etc. must move to the extension crate; under `protocol-2025-11-25`, tasks remain in core. The migration recipe in §"Migration from 2025-11-25 task users" of ADR-028 is the consumer guide.
- **ADR-030 (Client coexistence — bilingual by default)** governs `turul-mcp-client`. The client does NOT use the `turul-mcp-protocol` alias; it imports both `turul-mcp-protocol-2025-11-25` and `turul-mcp-protocol-2026-07-28` directly and routes per-connection. The server constraints in this ADR do not apply to the client. ADR-030 is the load-bearing decision for client architecture.

## Open items

- **CI matrix configuration** for the two-feature-configuration build. Add to GitHub Actions / Cirrus workflow. **Partially addressed:** the integration-test crates are pinned to the 2025-11-25 opt-in and a small 2025 regression suite (tasks-e2e pair + pinned logging/sampling/elicitation/client/lambda examples) exercises the fallback. The full workspace `--no-default-features --features protocol-2025-11-25` CI matrix run is still to be wired into the pipeline.
- **End-to-end test of the `protocol-2025-11-25` feature.** **Addressed:** the 2025-11-25 opt-in path is covered by the pinned regression suite and the integration-test crates; the fallback is no longer untested. (Pattern E collapse is therefore not triggered.)
- **`turul-mcp-ext-tasks-2026-07-28` scaffolding.** Required for the default-feature build to compile any tasks-using consumer code. In the landed cutover, tasks are gated to the 2025-11-25 opt-in (the task runtime is `#[cfg(feature = "protocol-2025-11-25")]`); the standalone 2026 extension crate is still to be scaffolded. ADR-028 Phase 5.2.
- **Final 2026-07-28 spec publication** will re-trigger schema regeneration per ADR-027 §"Regeneration trigger". The wire string is already finalized to `"2026-07-28"` (2026-06-07); a final-spec re-vendor is internal to `turul-mcp-protocol-2026-07-28` and does NOT change the feature-gating mechanism in this ADR.

## Revision log

- **2026-05-31** — initial. Decision: mutually-exclusive cargo features (`protocol-2026-07-28` default, `protocol-2025-11-25` opt-in) at the `turul-mcp-protocol` re-export boundary. `compile_error!` macros guard both-on and neither-on. Phase 9.4 strategy committed to flip-all-at-once. Cutover slice scope, CI matrix doubling, and feature-gating rollout plan called out as required-before-cutover. Architecture-review evidence at `docs/plans/2026-07-28-architecture-review.md`; parked-branch state at `docs/plans/2026-07-28-PARKED.md`.
- **2026-05-31 (correction)** — §"Cascade rule for the downstream framework crates" rewritten. The initial wording ("downstream crates do not declare their own protocol features; the choice flows up from the leaf binary") was internally inconsistent with the planned `compile_error!` mutex under Cargo feature unification. With bare workspace deps, `default = ["protocol-2026-07-28"]` would always be active transitively, so enabling `protocol-2025-11-25` at the leaf would trip the mutex. Corrected to mandate `default-features = false` on every consumer's `turul-mcp-protocol` dep, plus a per-crate `protocol-2025-11-25` forwarding feature. The rollout plan (`docs/plans/2026-07-28-feature-gating-rollout.md`) already assumed this topology in Phases 1/2/3/5 (forwarding via `turul-mcp-protocol/protocol-2025-11-25`); the ADR now matches.


- **2026-06-07 (amendment — symmetric feature names)** — **Coexistence feature identifiers are symmetric spec-version names; the `legacy-` framing is dropped.** The opt-in feature is `protocol-2025-11-25` (was `legacy-2025-11-25`) — neither feature is named `default` or `legacy`; both are `protocol-<spec-version>`. The Cargo `default = ["protocol-2026-07-28"]` array is retained (the maintainer-locked "server default = 2026", §"Status update") — it *points at* a spec-version feature, it does not *name* one "default". The title parenthetical and any "legacy" prose in the body are superseded by this naming. The wire string also finalized `"DRAFT-2026-v1"` → `"2026-07-28"` (ADR-027 2026-06-07), so `DRAFT-2026-v1` in this ADR's prose now denotes the same spec by its finalized literal. No code exists for these features yet (Phase 9.4 unbuilt), so the rename is doc-only.

- **2026-06-07 (cutover layer 2: framework cascade + two Cargo constraints)** — The protocol feature topology was cascaded from the alias to every framework crate (`turul-mcp-session-storage`/`-task-storage`/`-builders`/`-derive`/`turul-http-mcp-server`/`turul-mcp-server`/`turul-mcp-aws-lambda`): each forwards `protocol-2025-11-25` (default) / `protocol-2026-07-28` to the alias and its framework deps. Two Cargo realities revised the §"Cascade rule" mechanics:
   1. **`default-features = false` cannot override a workspace-inherited dependency** (Cargo 1.96: *"`default-features = false` cannot override workspace's `default-features`"*). The cascade rule's per-consumer `default-features = false` is therefore declared via EXPLICIT path+version deps (`{ path = "../X", version = "0.4.0", default-features = false }`) for the internal framework deps — a localized deviation from the workspace-deps convention, scoped to the 7 framework crates. Examples keep `{ workspace = true }` and inherit the alias default — which after the 2026-06-07 default flip is `protocol-2026-07-28` (see the cutover revision-log entry below).
   2. **`default-features = false` is all-or-nothing** — it also strips non-protocol defaults (storage `in-memory`, `turul-http-mcp-server` `sse`). Consumers must re-forward those; `turul-mcp-server` re-forwards the storage `in-memory` on both protocol features (its `InMemory*` imports are unconditional).
   Default build / clippy / tests are unchanged (green, 229 server tests pass); the framework `compile_error!` mutex fires. **Remaining (the first-party 2026 server):** a `protocol-2026-07-28` build of `builders` / `server` / `http-server` needs real type ADAPTATION — 2026 result types carry `resultType` + `CacheableResult` (`ttlMs`/`cacheScope`), so `builders` alone has ~40 type-mismatch errors under 2026. That adaptation + the `server/discover` handler + stateless transport path + cross-process acceptance is the bulk of the cutover, scoped in the lane-1 gap inventory.

- **2026-06-07 (cutover landed — default flipped to 2026-07-28, branch-scoped)** — The 0.4 cutover this ADR governs has **LANDED on `2026-07-28-MCP-Specification`** (not merged to `main`). What shipped:
   1. **Default flipped.** `crates/turul-mcp-protocol/Cargo.toml` now declares `default = ["protocol-2026-07-28"]`; the alias re-exports the 2026-07-28 crate by default, with `protocol-2025-11-25` as the opt-in (`--no-default-features --features protocol-2025-11-25`). The `[workspace] default-members` set lists the 2026-buildable crates and the migrated examples.
   2. **First-party 2026 server.** A `server/discover` handler plus a stateless 2026 request path (per-request `_meta`-carried capabilities, no `Mcp-Session-Id`) landed in `turul-mcp-server`/`turul-http-mcp-server`; the transport advertises `MCP-Protocol-Version: 2026-07-28` on the wire.
   3. **Builders + dynamic tools under 2026.** The ~40 type-mismatch adaptation flagged in the prior entry is done — `ToolBuilder` and dynamic-tools compile and run under the 2026 default.
   4. **Bilingual client builds under the 2026-default workspace** while still speaking either spec per connection (ADR-030).
   5. **Tasks gated to 2025-11-25.** The task runtime is `#[cfg(feature = "protocol-2025-11-25")]`; under the 2026 default, tasks are a (not-yet-scaffolded) extension per ADR-028.
   6. **Consumer fleet migrated.** 43 examples migrated to the 2026 default; 8 redundant duplicate examples removed (builders-showcase, comprehensive-server, sampling-with-tools-showcase, task-types-showcase, client-task-lifecycle, dynamic-tools-test-client, performance-testing, lambda-mcp-server-streaming); a small 2025-11-25 regression suite pinned (tasks-e2e pair + logging/sampling/elicitation/client/lambda examples held at the 2025 opt-in), and the integration-test crates pinned to the 2025-11-25 opt-in.
   Default-members build is green at 0 warnings; the framework `compile_error!` mutex fires correctly under both configurations. Publication to crates.io remains gated per ADR-027 (upstream final-spec publication + maintainer go-ahead).
- **2026-06-08 (doc reconciliation + 2025 regression coverage)** — Current-state prose in this ADR that still named the 2026 spec `DRAFT-2026-v1` (Context status line, the §"Why coexistence" cleavage points, the §Constraints "Server default" line, the §Decision "0.4.0 defaults to…" line, the §Consequences release-notes wording) was reconciled to the finalized wire literal `2026-07-28`. Remaining `DRAFT-2026-v1` mentions are deliberate history: ADR-027's title/subject references and the dated RC-instability rationale. Separately, `roots-server` was found pinned to the 2026 default (its `Root` type resolved to the deprecated 2026 binding), so the 2025 `mcp-roots-tests` e2e suite could not handshake it — all 14 tests failed at server start. Pinning `roots-server` to the 2025 opt-in (matching `sampling-server`/`elicitation-server`) turns the suite green; the roots/sampling/elicitation suites and a `client-initialise-server` build were added to the opt-in-2025 CI lane (they had been absent). `client-initialise-server` itself, an inherently-stateful `initialize`/`Mcp-Session-Id` demo, was moved out of `default-members` and pinned to the 2025 opt-in.
