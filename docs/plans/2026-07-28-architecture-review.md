# 2026-07-28 Architecture Review — Dual-Spec Strategy

**Status**: Maintainer-locked. Decision recorded; alternatives preserved for posterity.
**Date**: 2026-05-31
**Branch**: `feat/turul-mcp-protocol-2026-07-28` (sub-branch of `2026-07-28-MCP-Specification`)
**Related**: ADR-027 (`docs/adr/027-targeting-mcp-draft-2026-v1.md`), ADR-028 (extensions strategy), ADR-029 (cargo-feature gating), ADR-030 (`turul-mcp-client` bilingual default).
**Workflow lineage**: Persists the 5-pattern architecture-review output that previously lived in `/tmp` (`wf_2c892fb3-a06`), reshaped for the maintainer-locked decision recorded 2026-05-31.

> **Editorial note (2026-06-12):** this review is preserved only for the
> maintainer-locked decisions in §1–6. Its original operational companion,
> `2026-07-28-PARKED.md`, captured a transient pre-commit snapshot that has
> since been fully resolved (all parked commits landed) and was deleted in the
> 0.4 docs purge — see git history / the v0.3.x tags. The roadmap (§7), revision
> table (§8), and resume protocol (§9) below are historical and no longer
> actionable; line-number references into the deleted snapshot have been removed.

This document is the **doc-form persistence** of a prior architecture review (originally written to `/tmp`, now permanent per maintainer instruction: "we need this all documented properly in the docs dir not just in /tmp"). It captures the dual-spec problem, the five evaluated patterns, the maintainer's locked decisions, and the consequences that flow from them.

---

## 1. Problem statement

The crate `turul-mcp-protocol-2026-07-28` is spec-aligned (`crates/turul-mcp-protocol-2026-07-28/COMPLIANCE.md:1-230`, 342 tests green at review date, 0 warnings, 20/20 modeled fixtures pass; see COMPLIANCE.md for current test counts). The framework's consumer crates (`turul-mcp-server`, `turul-mcp-client`, `turul-http-mcp-server`, `turul-mcp-aws-lambda`, the derive macros, 55+ examples) still depend on the `turul-mcp-protocol` re-export alias, which today points at `turul-mcp-protocol-2025-11-25` (`crates/turul-mcp-protocol/src/lib.rs:1-21`).

DRAFT-2026-v1 is **not** wire-compatible with 2025-11-25:

- Stateless core: no `initialize`/`notifications/initialized` handshake; no `Mcp-Session-Id` header (`crates/turul-mcp-protocol-2026-07-28/COMPLIANCE.md:57-79`).
- `_meta` is REQUIRED on every request, with typed named fields (`io.modelcontextprotocol/protocolVersion`, `clientInfo`, `clientCapabilities`) (`COMPLIANCE.md:46-56`).
- New discovery RPC: `server/discover` replaces `initialize` for capability advertisement.
- Error code changed: `-32602` (was `-32002`).
- Tasks moved to extension crate (SEP-2663); `Subscribe*`/`Unsubscribe*` removed.
- Roots, Sampling, Logging marked `#[deprecated]` per SEP-2577 (annotation-only this revision).
- `inputSchema` adopts JSON Schema 2020-12 dialect (`$schema` field added on `ElicitationSchema`).

A single Rust process cannot hold both protocol state machines simultaneously without bespoke version-negotiation logic at the handler layer. The two handshake flows are mutually exclusive at runtime for any given server. The client has no such constraint (per-connection version is a transport-level decision, not a process-level one — see §6 and ADR-030).

The question this review answered: **how does the framework offer both specs without forcing a flag-day cutover, and what is the default?**

---

## 2. Five patterns evaluated

| # | Pattern | One-line summary | Verdict |
|---|---|---|---|
| **A** | **Cargo-feature gating** | Single workspace; consumer crates compile against exactly one protocol crate at a time, chosen by feature flag. Default = 2026; opt-in `legacy-2025-11-25`. | ✅ **Selected** |
| **B** | Runtime feature negotiation in one binary | Both protocol crates compiled into the same binary; handler dispatches per-request based on detected version. | ❌ Rejected |
| **C** | Two parallel framework forks | `turul-mcp-server-2026` alongside `turul-mcp-server-2025`. Each example/consumer picks one. | ❌ Rejected |
| **D** | Crate-by-crate staged migration | Migrate each consumer crate independently; tolerate a window where `turul-mcp-server` uses 2026 but `turul-mcp-client` still uses 2025. | ❌ Rejected |
| **E** | Hard cutover with no legacy escape | Flip the alias atomically, delete `turul-mcp-protocol-2025-11-25` from the workspace, force all consumers onto 2026. | ❌ Rejected |

(Full pattern analysis preserved in the published doc; see §§2.A–2.E.)

---

## 3. Maintainer-locked decisions (2026-05-31)

1. **Server default = DRAFT-2026-v1.** Opt-in cargo feature `legacy-2025-11-25` for the old spec.
2. **0.4.0 ships with default = 2026.** No "0.5.0 cutover" — prior recommendation overruled.
3. **NOT publishing while RC is unstable.** Internal work-in-progress on the feature branch only.
4. **All docs go to `docs/`.** Never `/tmp`. Permanent records.
5. **Client gets its own ADR** (ADR-030) addressing per-connection version selection.

---

## 4. Why the prior recommendation (default = 2025-11-25) was overruled

- Crates.io UX argument is **void** because decision (3) prevents publication. There is no `cargo add` user to surprise.
- RC churn risk is **honest, not hidden**. Pinning legacy as the default while internally building 2026 support frames the future as experimental when it's actually the development target.
- Legacy-as-default treats the future as experimental — wrong framing for a branch whose explicit purpose is adopting 2026.

The "safety" the prior recommendation preserved is preserved equivalently by the not-publishing constraint, which is stronger than a default-flag choice.

---

## 5. Consequences of the locked decision

- `turul-mcp-protocol` alias flips from re-exporting `2025-11-25` to re-exporting `2026-07-28` as part of consumer migration (ADR-027 Phase 9.4). Pattern A makes flip-all-at-once tractable.
- Frozen crates (`turul-mcp-protocol-2025-11-25@0.3.47`, `turul-mcp-protocol-2025-06-18@0.3.47`) remain untouched.
- Consumer crates gain `default = ["2026-07-28"]`, opt-in `legacy-2025-11-25`.
- `turul-rpc` workspace pin bumps from `0.1` to `0.2.2` atomically with the alias flip.
- Examples with version-specific shapes partitioned via `required-features`. CI matrix runs both gates.
- **No publish, no merge, no branch deletion** without express maintainer authority.

---

## 6. Client strategy summary (ADR-030)

The client has no process-wide protocol-state-machine constraint. Bilingual by default:

1. **Explicit hint preferred:** caller passes `mcp_protocol_version` in `ConnectionConfig`.
2. **Try-discover-then-fallback (default):** send `server/discover` (2026 RPC); on `-32601 Method Not Found`, fall back to `initialize` (2025 RPC). If both fail, `ServerUnsupported` error.

Narrowing features `client-2025-only` and `client-2026-only` strip the opposite crate's codegen for embedded/wasm. Default omits these. Full contract recorded in ADR-030.

---

## 7. Migration roadmap (post-decision)

Roadmap collapses because we are not publishing. The original 12–14 step publish/cutover sequence is **replaced with documentation + feature-gating + internal verification**:

1. Persist the architecture review (this document).
2. Author ADR-029 (cargo-feature gating).
3. Author ADR-030 (`turul-mcp-client` bilingual default).
4. Land Slice A' + A'' + B commits. *(Done.)*
5. Feature-flag consumer crates (`default = ["2026-07-28"]`, opt-in `legacy-2025-11-25`).
6. Flip the alias in `crates/turul-mcp-protocol/src/lib.rs` under feature gate.
7. Bump workspace `turul-rpc` pin from `0.1` to `0.2.2`.
8. CI matrix runs both feature paths green.
9. End-to-end test of the `legacy-2025-11-25` feature flag (devil's-advocate flagged as untested in parked state).

**Out of scope for this branch:** crates.io publish, merge to `main`, branch deletion, extension-crate scaffolding (SEP-2663/SEP-1865 deferred per ADR-028).

---

## 8. Revision table

| Date | Revision | Affected docs |
|---|---|---|
| 2026-05-31 | Architecture review persisted (this document) | `docs/plans/2026-07-28-architecture-review.md` (new) |
| 2026-05-31 | ADR-029 (cargo-feature gating) planned | `docs/adr/029-...md` (planned) |
| 2026-05-31 | ADR-030 (`turul-mcp-client` bilingual default) planned | `docs/adr/030-...md` (planned) |
| 2026-05-31 | ADR-027 revision log already covers Slice A' + per-crate versioning | `docs/adr/027-targeting-mcp-draft-2026-v1.md:73-90` |
| 2026-05-31 | Slice A' + A'' + B commits sequenced (since landed) | (tracked in the deleted PARKED snapshot — git history) |

---

## 9. Verification

- Protocol crate compliance: `cargo test -p turul-mcp-protocol-2026-07-28 --features compliance` — 342 pass, 0 warn.
- Branch state (2026-05-31): 107 uncommitted entries; all since committed.
- Schema pin: `crates/turul-mcp-protocol-2026-07-28/schema/EXAMPLES_PIN.md` (commit `c3e3f09e...`, 2026-05-24).
- No publish: `git log main..HEAD` is local-only.

Resume protocol (historical): the parked work described here was committed and the PARKED snapshot deleted in the 0.4 docs purge. The branch lock remains binding regardless of test state.

