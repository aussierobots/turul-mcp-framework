# Plan: Feature-gating rollout for DRAFT-2026-v1 across the framework

**Branch**: `feat/turul-mcp-protocol-2026-07-28` (sub-branch of `2026-07-28-MCP-Specification`; branch lock binding — no merge to `main` without express maintainer authorization).

**Goal**: Wire a workspace-wide cargo feature topology so the framework can compile and run against either MCP 2025-11-25 **or** DRAFT-2026-v1, with **DRAFT-2026-v1 as the 0.4.0 default**. The 2025-11-25 surface lives behind opt-in cargo feature `protocol-2025-11-25`. `turul-mcp-client` ships **bilingual by default** per ADR-030 with narrowing features `client-2026-only` / `client-2025-only`.

**Source decisions** (locked by maintainer, not re-debated in this plan):

1. Server default = DRAFT-2026-v1.
2. 0.4.0 ships with default = 2026.
3. NOT publishing while the RC is unstable — internal branch work only.
4. All docs go to `docs/`.
5. Client gets its own ADR (ADR-030) for cross-spec connectivity.

**Why a phased rollout?** The branch-tip architecture review identified roughly 400–600 `#[cfg(feature = ...)]` sites needed, concentrated in three files: `turul-http-mcp-server/src/streamable_http.rs` (2,312 LOC, dual handshake + `Mcp-Session-Id` header), `turul-mcp-server/src/session.rs` (1,990 LOC, `initialize` handler + capability negotiation), and `turul-mcp-session-storage/src/traits.rs` (557 LOC, `SessionInfo.is_initialized` + `initialize_session()`). Doing this in one slice would be unreviewable and unbisectable. Each phase below has its own bisect point.

**Sequencing principles**:

- **Boundary first, downstream later.** The `turul-mcp-protocol` alias is the boundary; flip it once, gate consumers from there. (Phase 0.)
- **One concern per phase.** Storage, server lifecycle, transport, client, Lambda each get their own phase. No cross-cutting mega-commits.
- **No destructive storage migration.** Stale columns (`is_initialized`) are tolerated, not dropped — same DB can be read by either spec build.
- **Truthful capability negotiation.** A 2026-build server MUST NOT advertise `initialize` support; a 2025-build server MUST NOT advertise `server/discover`. Each phase verifies this on the wire, not just in code.
- **Revert-and-fail per phase** (CLAUDE.md §"Test Coverage Discipline" #4): new tests must fail when the phase's `#[cfg]` gate is reverted.

**Verification gate** (each phase): both feature variants compile, both test suites green, no leaked references across the gate. CI matrix lands in Phase 7.

---

## Phase 0 — Feature topology at the `turul-mcp-protocol` boundary

**Scope**: Establish the cargo-feature contract that all downstream phases consume. The alias crate `turul-mcp-protocol` becomes the single switch, AND every consumer crate's `Cargo.toml` is rewritten to forward the feature choice explicitly. The bare-workspace-dep topology (`turul-mcp-protocol.workspace = true`) is incompatible with the `compile_error!` mutex under Cargo feature unification — see ADR-029 §"Cascade rule for the downstream framework crates" for the topology proof. All consumer manifests MUST be rewritten as part of Phase 0; deferring them to later phases would leave Phase 1/2/3/5 verification commands non-functional.

**Files touched**:

- `crates/turul-mcp-protocol/Cargo.toml` — add `[features]` section with mutually-exclusive `protocol-2026-07-28` (default) and `protocol-2025-11-25`. Drop the unconditional `turul-mcp-protocol-2025-11-25.workspace = true`; make both protocol-version deps `optional = true` and gated.
- `crates/turul-mcp-protocol/src/lib.rs` — replace the unconditional `pub use turul_mcp_protocol_2025_11_25::*` with two `#[cfg(feature = "...")]` re-export blocks. Add `compile_error!` mutex if both features are enabled simultaneously, and a second `compile_error!` if neither is enabled. `CURRENT_VERSION` const + the `tests` module move under matching cfg blocks.
- `Cargo.toml` (root) — `turul-mcp-protocol-2026-07-28` becomes a `workspace.dependencies` pin alongside the existing `turul-mcp-protocol-2025-11-25` pin. No change to other workspace pins.

**LOC estimate**: ~60 LOC across 3 files. Small but load-bearing.

**Dependencies**: None — this is the foundation. Slice A' + A'' + B must commit first so the 2026 protocol crate is on disk.

**Verification**:

```bash
# Default (2026): both compile and test
cargo check -p turul-mcp-protocol
cargo test -p turul-mcp-protocol

# Legacy (2025): both compile and test
cargo check -p turul-mcp-protocol --no-default-features --features protocol-2025-11-25
cargo test -p turul-mcp-protocol --no-default-features --features protocol-2025-11-25

# Mutex enforcement: both at once must fail at compile time
! cargo check -p turul-mcp-protocol --features protocol-2025-11-25
# Neither set: must fail at compile time
! cargo check -p turul-mcp-protocol --no-default-features
```

Add an integration test asserting `turul_mcp_protocol::CURRENT_VERSION == "DRAFT-2026-v1"` under default features and `== "2025-11-25"` under `protocol-2025-11-25`.

---

## Phase 1 — Session storage gating

**Scope**: Feature-gate the 2025-only session-lifecycle surface on the storage trait. `SessionInfo.is_initialized` (the boolean tracking `notifications/initialized` receipt), `SessionManager::initialize_session()`, and any `is_initialized` callback hook are 2025-only. The 2026 stateless core has no handshake to record. **Storage backends MUST tolerate stale columns** — no destructive migration. A 2026 build writing to an existing SQLite DB will leave the `is_initialized` column alone; a 2025 build reading from the same DB sees the old value.

**Files touched**:

- `crates/turul-mcp-session-storage/src/traits.rs` — `pub is_initialized: bool` (line 38), default values (lines 60, 75), test (line 484) all gated `#[cfg(feature = "protocol-2025-11-25")]`. Trait methods that flip the flag wrapped similarly.
- `crates/turul-mcp-session-storage/src/{in_memory,sqlite,postgres,dynamodb}.rs` — backend impls of the gated methods become no-ops or removed under 2026. Storage schema (CREATE TABLE statements) **stays unchanged** — the `is_initialized` column remains in the DDL so cross-spec rollback is non-destructive.
- `crates/turul-mcp-session-storage/Cargo.toml` — add `[features]` section with `protocol-2025-11-25` opt-in (default off), forwarded from `turul-mcp-protocol` via the `turul-mcp-protocol/protocol-2025-11-25` feature link.
- `crates/turul-mcp-session-storage/src/session_view.rs` — `StorageBackedSessionView::is_initialized()` gated; 2026 callers must not invoke it. Provide a `compile_error!` stub in the 2026 path if anyone tries.

**LOC estimate**: ~150–200 LOC of `#[cfg]` gates across 7 files; ~50 LOC of fallback no-ops. Plus ~30 LOC of new tests.

**Dependencies**: Phase 0 (feature topology must exist before storage can forward it).

**Verification**:

```bash
# Default (2026): no is_initialized in public surface
cargo check -p turul-mcp-session-storage
! grep -rn 'is_initialized' crates/turul-mcp-session-storage/src/ | grep -v 'cfg(feature'
cargo test -p turul-mcp-session-storage

# Legacy: is_initialized works
cargo test -p turul-mcp-session-storage --no-default-features --features protocol-2025-11-25,sqlite

# Cross-spec rollback: write under 2026, read under 2025, no panic
cargo test -p turul-mcp-session-storage --features protocol-2025-11-25 --test cross_spec_rollback
```

New test `cross_spec_rollback.rs` writes a session under 2026 build, opens the same SQLite file under 2025 build, asserts `is_initialized == false` (default), and confirms no migration error.

---

## Phase 2 — Server lifecycle gating

**Scope**: `crates/turul-mcp-server/src/session.rs` (1,990 LOC) and adjacent handlers carry the stateful 2025 lifecycle (`initialize` request → `notifications/initialized` notification → `Mcp-Session-Id` issued). Feature-gate the entire 2025 handshake path. Add the 2026 `server/discover` handler (response-only, no state change). Move capability negotiation timing from "at handshake" (2025) to "per-request, from `_meta`" (2026).

**Files touched**:

- `crates/turul-mcp-server/src/session.rs` — `initialize` request handler gated `#[cfg(feature = "protocol-2025-11-25")]`. Strict-lifecycle pre-initialize rejection (the `-32031 SessionError` path per AGENTS.md §"Release Readiness Notes") is gated. The `notifications/initialized` consumer (lines that set `is_initialized = true`) is gated.
- `crates/turul-mcp-server/src/handlers/` — new `discover.rs` (2026-only, `#[cfg(not(feature = "protocol-2025-11-25"))]`) implementing `server/discover`. Existing `initialize.rs` handler (if separate) gated 2025-only.
- `crates/turul-mcp-server/src/builder.rs` — `McpServer::builder().with_strict_lifecycle(...)` becomes a 2025-only method. `with_discover_capabilities(...)` is the 2026 equivalent (capabilities published in the discover response, not in `InitializeResult`).
- `crates/turul-mcp-server/src/server.rs` — capability advertisement code branches: 2025 publishes via `InitializeResult`, 2026 publishes via `DiscoverResult` AND echoes the spec-relevant capability subset in every response's `_meta`.
- `crates/turul-mcp-server/src/dispatch/` — request dispatcher routing: `"initialize"` method → 2025 handler; `"server/discover"` → 2026 handler. Method-not-found error MUST surface for the wrong-spec call.
- `crates/turul-mcp-server/Cargo.toml` — propagate `protocol-2025-11-25` feature; forward to `turul-mcp-protocol/protocol-2025-11-25` and `turul-mcp-session-storage/protocol-2025-11-25`.

**LOC estimate**: ~300–400 LOC of `#[cfg]` gates concentrated in `session.rs`, plus ~150 LOC of new `discover.rs`, plus ~80 LOC of capability-routing branching. Approximately 70–100 individual gate sites.

**Dependencies**: Phases 0, 1.

**Verification**:

```bash
# Default (2026): server/discover handled; initialize returns method-not-found (-32601)
cargo test -p turul-mcp-server --test discover_handshake
cargo test -p turul-mcp-server --test no_initialize_under_2026

# Legacy (2025): initialize handshake works; server/discover returns method-not-found
cargo test -p turul-mcp-server --no-default-features --features protocol-2025-11-25 --test initialize_handshake
cargo test -p turul-mcp-server --no-default-features --features protocol-2025-11-25 --test no_discover_under_2025

# Capability truthfulness: 2026 server's discover response must NOT advertise initialize support
cargo test -p turul-mcp-server --test capability_truthfulness
```

Revert-and-fail check: remove the `#[cfg(not(feature = "protocol-2025-11-25"))]` gate on the discover handler — the 2025 build must then fail to compile because of duplicate-handler registration.

---

## Phase 3 — Transport gating

**Scope**: `crates/turul-http-mcp-server/src/streamable_http.rs` (2,312 LOC) carries every wire-format difference between specs: the `Mcp-Session-Id` header (2025-only), GET-SSE listener (2025-only — held open for elicitation/sampling), and the new `subscriptions/listen` POST stream (2026-only — replaces GET-SSE for the multi-round-trip flows). The required new headers `MCP-Protocol-Version`, `Mcp-Method`, `Mcp-Name` (2026) need their own validation path.

**Files touched**:

- `crates/turul-http-mcp-server/src/streamable_http.rs` — every `Mcp-Session-Id` reference (~12 sites per grep at lines 160, 162, 260, 262, 280, 296, 549, 992, 1086, 1295, 1310, 1372, 1438, 1447) gated `#[cfg(feature = "protocol-2025-11-25")]`. GET-SSE handler (lines ~250–600) gated 2025-only. New `subscriptions/listen` POST-stream handler added under 2026-only gate.
- `crates/turul-http-mcp-server/src/session_handler.rs` — entire legacy-protocol (≤ 2024-11-05) GET-SSE pathway becomes 2025-only.
- `crates/turul-http-mcp-server/src/mcp_session.rs` — strict-lifecycle 400 vs 404 distinction (AGENTS.md §"Session Status Codes") gated 2025-only; 2026 has no missing-session-header error because there is no session header.
- `crates/turul-http-mcp-server/src/notification_bridge.rs` — `notifications/initialized` reception path gated 2025-only.
- `crates/turul-http-mcp-server/src/stream_manager.rs` — 2025 uses session-id-keyed streams; 2026 uses subscription-token-keyed streams (per the new `SubscriptionsListenRequestParams.filter`). Both paths coexist behind gates.
- `crates/turul-http-mcp-server/Cargo.toml` — feature propagation.

**LOC estimate**: ~600–800 LOC of `#[cfg]` gates in `streamable_http.rs` alone (this is the densest gate site in the workspace); ~200 LOC of new subscriptions/listen handler; ~100 LOC of header-validation routing. Approximately 200+ individual gate sites.

**Dependencies**: Phases 0, 1, 2. Cannot proceed without the protocol-crate boundary or the server-level lifecycle gates.

**Verification**:

```bash
# Default (2026): subscriptions/listen wires through; no Mcp-Session-Id required
cargo test -p turul-http-mcp-server --test subscriptions_listen_e2e

# 2026 wire: a POST without Mcp-Session-Id MUST succeed (no required header)
cargo test -p turul-http-mcp-server --test no_session_header_under_2026

# Legacy (2025): GET SSE works; missing Mcp-Session-Id returns 400
cargo test -p turul-http-mcp-server --no-default-features --features protocol-2025-11-25 --test get_sse_handshake
cargo test -p turul-http-mcp-server --no-default-features --features protocol-2025-11-25 --test missing_session_header_400

# Wire-layer rule (CLAUDE.md §"Test Coverage Discipline" #3): exercise the bytes hitting hyper
cargo test -p turul-http-mcp-server --test wire_bytes_per_spec
```

The `wire_bytes_per_spec` test drains a real HTTP response body and asserts the byte sequence matches the spec for the active feature — not the framework's internal types.

---

## Phase 4 — Client gating per ADR-030

**Scope**: `turul-mcp-client` is **bilingual by default** per ADR-030: both protocol versions compile into one binary, version is negotiated per-connection (try-discover-then-fallback, or explicit `ConnectionConfig.mcp_protocol_version`). Narrowing features `client-2026-only` and `client-2025-only` strip the other spec for embedded / WASM size-constrained builds.

**Files touched**:

- `crates/turul-mcp-client/Cargo.toml` — depend on **both** `turul-mcp-protocol-2025-11-25` and `turul-mcp-protocol-2026-07-28` directly (not via the `turul-mcp-protocol` alias), both `optional = true`. Add `[features]` block per ADR-030 §"Cargo topology":
  - `default = ["http", "sse", "client-bilingual"]` — explicit bilingual default (the implicit-default design was tried and rejected because `optional = true` deps cannot be activated by absence of a feature; see ADR-030 revision log entry "re-correction, codex P0 second pass").
  - `client-bilingual = ["dep:turul-mcp-protocol-2025-11-25", "dep:turul-mcp-protocol-2026-07-28"]` — pulls both protocol crates into the build.
  - `client-2025-only = ["dep:turul-mcp-protocol-2025-11-25"]` — narrowing; pulls in 2025 only.
  - `client-2026-only = ["dep:turul-mcp-protocol-2026-07-28"]` — narrowing; pulls in 2026 only.
  - All three protocol features are mutually exclusive (any pair active is a build error). Narrowing requires `--no-default-features` on the leaf (e.g. `cargo build --no-default-features --features http,sse,client-2025-only`). The standard Cargo idiom used by `serde`/`tokio`/`reqwest`.
- `crates/turul-mcp-client/src/protocol/` — new module directory: `mod.rs` (version-selection router), `v2025.rs`, `v2026.rs`. Each contains the encode/decode adapters for that spec.
- `crates/turul-mcp-client/src/version.rs` — new file: `McpVersion` enum, try-discover-then-fallback negotiation logic, `ConnectionConfig.mcp_protocol_version: Option<McpVersion>`.
- `crates/turul-mcp-client/src/client.rs` — `connect()` calls into the version-negotiation flow; stores the negotiated version on `McpClient` for the lifetime of that client; per-request routing dispatches to the version-specific adapter.
- `crates/turul-mcp-client/src/transport/{http,sse,stdio}.rs` — transport remains version-agnostic. The `Authorization` header / bearer-rotation patches from v0.3.44 stay intact across both versions.
- `crates/turul-mcp-client/src/error.rs` — add `McpClientError::ServerUnsupported(String)` variant per ADR-030 fallback.

**LOC estimate**: ~1,300–2,000 LOC total per ADR-030 §code_impact (~800–1,200 LOC for version detection + per-connection routing + fallback; ~300–500 LOC for protocol-specific adapters; ~200–300 LOC for fallback-scenario tests). No `Transport` trait changes — version selection is orthogonal to transport selection.

**Dependencies**: Phase 0 (the protocol-version pins must exist in the workspace). Phases 1–3 not strictly required, but the client's wire-format tests will fail without them — keep this phase ordered after Phase 3.

**Verification**:

```bash
# Bilingual default: connect to either spec server
cargo test -p turul-mcp-client --test version_negotiation
cargo test -p turul-mcp-client --test fallback_2026_to_2025
cargo test -p turul-mcp-client --test fallback_2025_to_2026_unsupported

# 2026-only narrowing: 2025 server connection fails with ServerUnsupported
cargo test -p turul-mcp-client --no-default-features --features http,sse,client-2026-only --test rejects_2025_server

# 2025-only narrowing: 2026 server connection fails with ServerUnsupported
cargo test -p turul-mcp-client --no-default-features --features http,sse,client-2025-only --test rejects_2026_server

# Mutex: every pair of protocol features simultaneously must fail to compile.
# (Triple-guard catches the common footgun of forgetting --no-default-features.)
! cargo check -p turul-mcp-client --features client-2025-only,client-2026-only
! cargo check -p turul-mcp-client --features client-2025-only             # client-bilingual still in default → conflict
! cargo check -p turul-mcp-client --features client-2026-only             # client-bilingual still in default → conflict
! cargo check -p turul-mcp-client --no-default-features --features http,sse   # no protocol feature → conflict
```

ADR-030 must land **before** this phase begins; the version-detection mechanism is decided there, not here.

---

## Phase 5 — Lambda gating

**Scope**: `crates/turul-mcp-aws-lambda` currently routes by `Mcp-Session-Id` (2025-only mechanism — pinning a request to a warm instance carrying that session). The 2026 stateless core makes this routing irrelevant: every request is self-contained via `_meta`, and any instance can serve any request. Add a 2026-only stateless Lambda handler variant; existing session-routing handler stays gated 2025-only.

**Files touched**:

- `crates/turul-mcp-aws-lambda/src/handler.rs` — current handler (session-routed) gated `#[cfg(feature = "protocol-2025-11-25")]`. New `stateless_handler.rs` (2026-only) implementing the simpler request-response loop.
- `crates/turul-mcp-aws-lambda/src/builder.rs` — `LambdaMcpServerBuilder::with_session_routing(...)` becomes 2025-only. 2026 builder skips routing config entirely.
- `crates/turul-mcp-aws-lambda/src/streaming.rs` — Lambda Runtime API streaming wire bytes (the v0.3.42 hot site per CLAUDE.md §"Test Coverage Discipline" footnote) unchanged at the byte level; only the trigger for keeping a stream open differs (GET-SSE under 2025, `subscriptions/listen` POST stream under 2026 — routed at the transport layer in Phase 3).
- `crates/turul-mcp-aws-lambda/src/adapter.rs` — adapter dispatch table gates `initialize` and `notifications/initialized` paths.
- `crates/turul-mcp-aws-lambda/Cargo.toml` — feature propagation.

**LOC estimate**: ~250–350 LOC of `#[cfg]` gates and the new stateless handler file (~150 LOC).

**Dependencies**: Phases 0, 1, 2, 3. Phase 4 (client) is orthogonal to Lambda server gating, but the Lambda-mcp-client example pins both — so if Phase 4 lands after, also revisit example pins (Phase 6).

**Verification**:

```bash
# Default (2026): stateless Lambda handler routes all requests, no session affinity
cargo test -p turul-mcp-aws-lambda --test stateless_handler

# Legacy (2025): session-routed Lambda handler honors Mcp-Session-Id stickiness
cargo test -p turul-mcp-aws-lambda --no-default-features --features protocol-2025-11-25 --test session_routing

# Cold start under 2026 must not need to recover any session state
cargo test -p turul-mcp-aws-lambda --test cold_start_stateless
```

---

## Phase 6 — Examples and test crates

**Scope**: All 62 example crates and 8 root integration test crates currently pin against the implicit 2025-11-25 baseline. Each manifest needs to explicitly opt into a spec version. The general rule: **examples that demonstrate 2025-only features (lifecycle gating, GET-SSE elicitation hold-open) pin to legacy**; everything else stays on default (2026). New examples for 2026-only features (`server/discover` walkthroughs, `subscriptions/listen` demos, MRTR elicitation per SEP-2322) get added.

**Files touched**:

- 62× `examples/*/Cargo.toml` — each gets an explicit `default-features = false, features = ["protocol-2025-11-25"]` block on its `turul-mcp-*` deps if it's a 2025-only example, or no change (default 2026) if it's spec-neutral or 2026-targeted. Estimated split: ~10 examples pin legacy explicitly (lifecycle / handshake demos), ~50 work under both, ~5 new 2026-only examples added.
- 8× `tests/*/Cargo.toml` (`tests/elicitation`, `tests/prompts`, `tests/resources`, `tests/roots`, `tests/sampling`, `tests/tools`, `tests/shared`, `tests/test_helpers`) — likewise pin explicitly.
- `tests/consolidated/` — split into `consolidated_2025.rs` and `consolidated_2026.rs` where the spec divergence is unbridgeable (e.g., lifecycle E2E tests).
- `examples/client-initialise-server/` (the canonical handshake example) — split or rename. Under default it becomes `client-discover-server` demonstrating `server/discover`; under legacy it remains the initialize-handshake demo.
- 62× example `src/main.rs` files — `.version("0.4.0")` strings unchanged (per CLAUDE.md §"Pre-Release Checklist" #2); imports gated where they reference 2025-only types (`Mcp-Session-Id` extraction, etc.).

**LOC estimate**: ~200 LOC across 70 Cargo manifests (purely additive `[features]` sections / `default-features = false` flips); ~100–300 LOC of new example bodies for 2026-only features.

**Dependencies**: Phases 1–5. Examples are the user-facing surface and can't be flipped until the underlying crates support both modes.

**Verification**:

```bash
# Every example compiles under default
cargo build --workspace --examples

# Every example compiles under legacy
cargo build --workspace --examples --features protocol-2025-11-25

# Integration tests pass under both modes
cargo test --workspace
cargo test --workspace --features protocol-2025-11-25

# No example accidentally pins the wrong spec — explicit check
grep -L 'turul-mcp-protocol' examples/*/Cargo.toml  # all should match (none missing)
```

---

## Phase 7 — CI matrix

**Scope**: The branch-tip review (workflow `wf_d6984699-a5a` and follow-ups) flagged that CI currently exercises one path. Once the legacy feature is real, CI must cover both code paths or the legacy surface will rot silently. Add a matrix dimension for `protocol-2025-11-25` vs default.

**Files touched**:

- `.github/workflows/ci.yml` (or equivalent CI manifest — confirm the actual filename when this phase starts) — add a `protocol` matrix axis: `[default, protocol-2025-11-25]`. Every existing job (build, test, clippy, doc) runs under both.
- `scripts/test_middleware_live.sh` — add a `--legacy` flag that adds `--features protocol-2025-11-25` to the cargo invocations.
- `scripts/ci_matrix.sh` (new) — helper script invoking each variant for local-CI parity.

**LOC estimate**: ~80 LOC of YAML + ~50 LOC of shell. Small in code, **doubles compute cost** of CI — call this out explicitly.

**Documented cost**: CI build/test wall-clock doubles. Cache hit-rates may improve over time, but the steady-state cost is real. If the legacy feature is later deprecated or removed (e.g., 2027 when SEP-2577 removal window expires), drop this matrix axis to recover the cost.

**Dependencies**: Phases 1–6 must be green before CI matrix can pass.

**Verification**:

```bash
# Local matrix parity check
bash scripts/ci_matrix.sh

# Confirm both axes exist in CI manifest
grep -E 'protocol:|protocol-2025-11-25' .github/workflows/ci.yml
```

---

## Phase 8 — Slice Completion Gate verification

**Scope**: Apply CLAUDE.md §"Slice Completion Gate" to the whole rollout. No claim of "feature gating is complete" without the verification greps below returning the expected counts. Confirm no leaked references across the gate, every doc reflects the new state, and no tombstone narratives entered the source tree.

**Files touched**: None substantively — this is a verification phase. May add `scripts/verify_feature_gating.sh` to make the greps re-runnable.

**Verification (all counts MUST be 0 except where noted)**:

```bash
# No 2025-only symbol references in default (2026) public surface
# Run under default features; output should be empty
grep -rEn 'is_initialized|notifications/initialized|Mcp-Session-Id' \
  crates/turul-mcp-server/src/ \
  crates/turul-http-mcp-server/src/ \
  crates/turul-mcp-aws-lambda/src/ \
  | grep -v 'cfg(feature' | grep -v 'cfg(not(feature'

# No 2026-only symbol references leaking into legacy public surface
grep -rEn 'server/discover|subscriptions/listen|InputRequiredResult' \
  crates/turul-mcp-server/src/ \
  crates/turul-http-mcp-server/src/ \
  | grep -v 'cfg(feature' | grep -v 'cfg(not(feature'

# Examples Cargo.toml all explicitly opt one way or the other (none ambiguous)
# Expected: every match either has `protocol-2025-11-25` listed or it doesn't — both are fine,
# but a Cargo.toml that depends on turul-mcp-protocol without an explicit features block is suspect.
for f in examples/*/Cargo.toml tests/*/Cargo.toml; do
  if grep -q 'turul-mcp-protocol' "$f" && ! grep -qE 'features.*=|default-features' "$f"; then
    echo "AMBIGUOUS: $f"
  fi
done

# No tombstone-style narratives in plans / src
grep -rEn 'was removed|no longer:|formerly known|deleted with' \
  docs/plans/2026-07-28-feature-gating-rollout.md \
  crates/turul-mcp-server/src/ \
  crates/turul-http-mcp-server/src/

# COMPLIANCE.md still accurate under default; CHANGELOG mentions the new feature
grep -E 'protocol-2026-07-28|protocol-2025-11-25' CHANGELOG.md

# CI matrix actually built (double-check after Phase 7)
test -f .github/workflows/ci.yml && grep -c 'protocol-2025-11-25' .github/workflows/ci.yml
```

For each non-zero hit (other than the expected ones), surface it in the phase summary with explicit disposition (intentional historical reference vs gate leak vs missing `#[cfg]`). Never silently let an unexpected hit pass — this is exactly the failure mode CLAUDE.md §"Slice Completion Gate" exists to prevent.

---

## Cross-phase risks and open questions

These are flagged from the prior architecture-review output and the devils-advocate review embedded in this workflow's context. They are **not** phase-specific scope but must be tracked across the rollout.

1. **Upstream RC churn.** DRAFT-2026-v1's ETag will change between now and the final 2026-07-28 publication. Slice A' already absorbed 8 schema-fidelity defects. If the upstream schema shifts again, every phase below may need follow-up. Mitigation: pin tightly, refresh on a deliberate slice, never silently re-vendor.
2. **Legacy feature testedness.** PARKED.md called out that the 2025-only path has no end-to-end test today. Phase 7 (CI matrix) is the proof — until Phase 7 lands, treat the legacy feature as "compiles, may not work."
3. **`turul-rpc` 0.1 → 0.2.2 workspace bump.** Currently isolated to the 2026 protocol crate (ADR-025). Once Phase 3 (transport) needs the 0.2.2 wire-message union, this bump becomes a prerequisite. Sequence it as Phase 3 prep work, not a separate phase — but call it out in the Phase 3 commit message.
4. **Protocol-alias flip vs feature-gating.** This plan does NOT touch the alias semantics — `turul-mcp-protocol` always re-exports one or the other based on its own features. ADR-027 Phase 9.4 ("flip the alias") is a separate, **already-resolved** concern: under this plan, the alias defaults to 2026, and `protocol-2025-11-25` is the escape hatch. There is no separate "alias flip" slice.
5. **Extension crates** (`turul-mcp-ext-tasks-2026-07-28` per SEP-2663, `turul-mcp-ext-apps-2026-07-28` per SEP-1865) are out of scope for this rollout. They are additive opt-ins; the framework's core feature gating does not block them.

---

## Order-of-operations summary

| Phase | Effort | Crates touched | Blocks | Blocked by |
|---|---|---|---|---|
| 0 — Protocol boundary | S | 1 | All | A'/A''/B committed |
| 1 — Session storage | M | 1 | 2, 3, 5 | 0 |
| 2 — Server lifecycle | L | 1 | 3, 5, 6 | 0, 1 |
| 3 — Transport | L | 1 | 5, 6 | 0, 1, 2 |
| 4 — Client (ADR-030) | M | 1 | 6 | 0 (ADR-030 first) |
| 5 — Lambda | M | 1 | 6 | 0, 1, 2, 3 |
| 6 — Examples + tests | M | 70 manifests | 7, 8 | 1–5 |
| 7 — CI matrix | S | 1 (workflow) | 8 | 1–6 |
| 8 — Slice Completion Gate | S | none (verification) | release | 1–7 |

**Total rough order**: ~3,000–5,000 LOC of `#[cfg]` gates and new code across 6 phases of code work, plus 2 phases of manifest/CI/verification. Spread across 8 bisectable commits (one per phase).

**Branch lock reminder**: this entire rollout lives on the `2026-07-28-MCP-Specification` family of branches. Do not merge to `main`, do not open a release PR, do not push to a publicly-tracked branch without express maintainer authorization. The not-publishing constraint applies for the duration.

