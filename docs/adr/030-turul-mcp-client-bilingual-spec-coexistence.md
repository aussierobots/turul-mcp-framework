# ADR-030: `turul-mcp-client` spec coexistence — bilingual default

**Status:** Accepted
**Date:** 2026-05-31
**Crate:** `turul-mcp-client`
**Branch:** `feat/turul-mcp-protocol-2026-07-28` (sub-branch of `2026-07-28-MCP-Specification`)
**Related:** ADR-027 (Targeting DRAFT-2026-v1), ADR-029 (server-side coexistence via `protocol-2025-11-25` feature), ADR-001 (protocol-alias-usage), ADR-025 (extract turul-rpc)

## Context

ADR-029 chose a single-spec-per-build strategy for `turul-mcp-server` and the rest of the server-side stack: 0.4.0 defaults to `DRAFT-2026-v1`, with `--features protocol-2025-11-25` flipping the build over to the older spec. That decision rests on a hard architectural constraint: a single server process cannot simultaneously host two mutually-exclusive handshake state machines. 2025-11-25 requires the `initialize` → `notifications/initialized` → `Mcp-Session-Id` flow; DRAFT-2026 removes `initialize` entirely, makes the wire stateless, and ships discovery via `server/discover` with `_meta` routing on every request. One process, one wire contract.

**The client has no equivalent constraint.** A client emits and receives bytes over a transport (HTTP POST/SSE GET in `crates/turul-mcp-client/src/transport.rs`; planned stdio). Whether those bytes carry a 2025-11-25 `initialize` handshake or a DRAFT-2026 `server/discover` discovery is a *per-connection* choice, decided when the client calls `connect()`, not a process-wide compile-time choice. Today the client unconditionally drives the 2025-11-25 flow (`crates/turul-mcp-client/src/client.rs:321-399` — `initialize` request, `Mcp-Session-Id` capture at line 344-360, `notifications/initialized` at line 391-396).

Four real deployment scenarios surface why a single-spec client is operationally inadequate:

- **A. CLI tool targeting a single known server.** The operator runs `mcp-cli --url https://srv.example/mcp`. They may have built the CLI against one spec, but their server fleet may upgrade unevenly. A 2026-only CLI strands every legacy 2025-11-25 server they manage; a 2025-only CLI cannot reach any 2026-07-28 server.
- **B. MCP gateway / aggregator.** A single binary fans out to many upstream MCP servers — some legacy, some upgraded. The gateway has no negotiation freedom: every server URL is configured by an operator, and the gateway must speak whichever spec each individual server speaks. Two single-spec gateway binaries side-by-side is not a tenable production answer.
- **C. IDE plugin talking to user-configured servers.** Users point the plugin at their own MCP server URL. The plugin author has no control over what spec each user's server speaks. Shipping two plugin binaries — one per spec — is a UX regression and an operations multiplier.
- **D. Integration test client.** Regression suites must exercise both specs in one test binary (e.g. proving that a forthcoming feature works against both 2025-11-25 *and* DRAFT-2026 servers). Two separate test binaries fragment CI and make cross-version assertions awkward.

In all four cases the connection-time decision is per-target, not per-build.

There is also a sequencing reality: per ADR-027 §"Open items" Phase 9.4, the workspace alias `turul-mcp-protocol` still re-exports `turul-mcp-protocol-2025-11-25`. `turul-mcp-client` currently imports through that alias (`crates/turul-mcp-client/src/client.rs:17-25` — `use turul_mcp_protocol::meta::Cursor;`, etc.; `crates/turul-mcp-client/Cargo.toml:13` — `turul-mcp-protocol = { workspace = true }`). When the alias flips (a separate cutover slice), every client consumer would otherwise break in one shot. Bilingual support gives us a non-flag-day path.

## Decision

**`turul-mcp-client` ships with bilingual spec support enabled by default.**

The crate compiles `turul-mcp-protocol-2025-11-25` and `turul-mcp-protocol-2026-07-28` types side-by-side, behind feature-narrowed code paths. A single `McpClient` instance negotiates the wire spec at `connect()` time and locks the chosen version for that instance's lifetime. Different `McpClient` instances in the same process may speak different versions concurrently; the transport layer remains spec-agnostic.

### Cargo topology

Bilingual is the **explicit default feature**: `default = ["http", "sse", "client-bilingual"]`. An "implicit default via absence of narrowing flag" was tried in an earlier revision (see Revision log entry for codex P1-4) but is not executable: `optional = true` deps are only linked when a feature activates them via `dep:` syntax, and Cargo features can only ADD deps, never REMOVE them. So bilingual MUST be an explicit feature that pulls both protocol crates in.

The trade-off: narrowing to one protocol requires `--no-default-features` on the leaf consumer. This is standard Cargo idiom (it is how `serde`, `tokio`, `reqwest` all handle mutually-exclusive default features). The `compile_error!` mutex catches the common footgun (`cargo build --features client-2025-11-25-only` without `--no-default-features` → both `client-bilingual` and `client-2025-11-25-only` active → compile error with explanatory message).

```toml
# crates/turul-mcp-client/Cargo.toml
[dependencies]
turul-rpc                      = { workspace = true }   # spec-neutral JSON-RPC envelope
turul-mcp-protocol-2025-11-25  = { workspace = true, optional = true }
turul-mcp-protocol-2026-07-28  = { workspace = true, optional = true }
# (the `turul-mcp-protocol` alias is NOT imported by the client; the client
#  pulls each versioned crate explicitly so it can speak both at once.)

[features]
default          = ["http", "sse", "client-bilingual"]
http             = []
sse              = ["tokio-util"]
stdio            = ["tokio-util"]

# Bilingual = both protocol crates linked. The DEFAULT mode for this crate.
client-bilingual = [
    "dep:turul-mcp-protocol-2025-11-25",
    "dep:turul-mcp-protocol-2026-07-28",
]

# Narrowing features: each links exactly ONE protocol crate. Use
# --no-default-features when enabling these (otherwise client-bilingual is
# also active and the compile_error! mutex fires).
client-2025-11-25-only = ["dep:turul-mcp-protocol-2025-11-25"]
client-2026-07-28-only = ["dep:turul-mcp-protocol-2026-07-28"]
```

All three protocol features are mutually exclusive (any two simultaneously is a build error):

```rust
// crates/turul-mcp-client/src/lib.rs
#[cfg(any(
    all(feature = "client-bilingual",  feature = "client-2025-11-25-only"),
    all(feature = "client-bilingual",  feature = "client-2026-07-28-only"),
    all(feature = "client-2025-11-25-only",  feature = "client-2026-07-28-only"),
))]
compile_error!(
    "turul-mcp-client: `client-bilingual` (default), `client-2025-11-25-only`, and \
     `client-2026-07-28-only` are mutually exclusive. Narrowing usage:  \
     `cargo build --no-default-features --features http,sse,client-2025-11-25-only`. \
     Bilingual is the default; just `cargo build` is enough."
);

#[cfg(not(any(feature = "client-bilingual", feature = "client-2025-11-25-only", feature = "client-2026-07-28-only")))]
compile_error!(
    "turul-mcp-client: enable exactly one of `client-bilingual` (default), \
     `client-2025-11-25-only`, or `client-2026-07-28-only`. If you used \
     `--no-default-features`, add one of these explicitly."
);
```

Usage matrix:

- `cargo add turul-mcp-client` → bilingual (default; both protocols linked)
- `cargo add turul-mcp-client --no-default-features --features http,sse,client-2025-11-25-only` → 2025-only; binary excludes the 2026 protocol crate from the dep graph
- `cargo add turul-mcp-client --no-default-features --features http,sse,client-2026-07-28-only` → 2026-only; binary excludes 2025
- `cargo add turul-mcp-client --features client-2025-11-25-only` (NO `--no-default-features`) → **compile error** with the message above explaining the fix
- `cargo add turul-mcp-client --no-default-features --features http,sse` (no protocol feature) → **compile error** demanding one be chosen

Bilingual is the only configuration tested against the full deployment-scenario matrix; the `*-only` features are opt-in narrowing for binary-size-sensitive consumers (embedded, wasm, legacy CLI).

Internal modules use the `#[cfg(feature = "...")]` gates directly on their `pub use` and `mod` lines, e.g.:

```rust
#[cfg(any(feature = "client-bilingual", feature = "client-2025-11-25-only"))]
pub(crate) mod v2025;

#[cfg(any(feature = "client-bilingual", feature = "client-2026-07-28-only"))]
pub(crate) mod v2026;

#[cfg(feature = "client-bilingual")]
pub(crate) mod bilingual_dispatch;
```

This is how a single-narrowing build genuinely excludes the other protocol's types from compilation.

### Internal module layout

```
crates/turul-mcp-client/src/
├── client.rs                 // public McpClient API (unchanged surface)
├── version.rs                // McpVersion enum, detection, fallback flow (NEW)
├── protocol/                 // (NEW)
│   ├── mod.rs                //   re-exports + version-routing helpers
│   ├── v2025.rs              //   2025-11-25 request/response serialization
│   └── v2026.rs              //   DRAFT-2026 request/response serialization
└── transport/                // unchanged; transports stay spec-agnostic
```

Per-connection state lives on `McpClient`:

```rust
pub struct McpClient {
    transport: Arc<BoxedTransport>,
    session:   Arc<SessionManager>,
    // ... existing fields ...
    protocol_version: Arc<RwLock<Option<McpVersion>>>,  // NEW; set at connect()
}
```

Hot-path request builders read `protocol_version` and dispatch to the appropriate `protocol/v2025` or `protocol/v2026` serializer. `Transport::send_request` / `send_notification` / `send_delete` / SSE GET listener are not modified — they continue to move opaque JSON envelopes.

### Version detection mechanism

A hybrid scheme, with explicit caller hint taking precedence over auto-probe:

1. **Explicit hint (preferred path).** `ClientConfig` gains an optional field:

   ```rust
   pub mcp_protocol_version: Option<McpVersion>,
   ```

   When set, `connect()` skips probing and drives the configured handshake directly. The caller guarantees server conformance.

2. **Try-discover-then-fallback (default when hint is `None`).**
   - On `connect()`, send `server/discover` (a DRAFT-2026 RPC; no `Mcp-Session-Id`, `_meta` per spec).
   - If the server returns a valid `DiscoverResult`, lock the connection to `McpVersion::V2026_07_28`. Cache the result for subsequent metadata calls.
   - **Fallback to 2025-11-25 is triggered ONLY by a valid JSON-RPC response carrying `error.code == -32601` (Method Not Found).** This is the unambiguous signal that the server lacks the `server/discover` method — i.e. it speaks the older spec. The retry sends `initialize`, captures `Mcp-Session-Id` (`crates/turul-mcp-client/src/client.rs:344-348`), sends `notifications/initialized` (line 391-396), and locks the connection to `McpVersion::V2025_11_25`.
   - **HTTP 4xx responses MUST NOT trigger fallback.** Auth failures (401/403), missing-route errors (404 from a misconfigured gateway), method-not-allowed (405), missing required `_meta` headers, and other 4xx outcomes are *transport or authorization failures*, not protocol-version signals. They surface as `McpClientError::ConnectionFailed { status, reason }` and abort the connect; they do NOT silently downgrade.
   - **Other JSON-RPC error codes MUST NOT trigger fallback.** `-32700` (Parse Error), `-32600` (Invalid Request), `-32602` (Invalid Params), and `-32603` (Internal Error) all indicate the server *understood* `server/discover` and rejected it for an unrelated reason. Surface as `McpClientError::ProtocolError`; abort the connect.
   - **Why the narrow rule**: under DRAFT-2026, `server/discover` is REQUIRED. A spec-conformant 2026 server behind a misconfigured gateway returning HTTP 4xx is NOT a 2025 server — it's a 2026 server with a deployment problem. Falling back to 2025-11-25 would silently downgrade the protocol the user asked for, defeating the purpose of feature-gating the legacy surface as opt-in.
   - **Escape hatch for known-broken gateways**: `ClientConfig.allow_legacy_gateway_fallback: bool` (default `false`). When set, broadens the fallback trigger to additionally accept HTTP 404 / 405 — for operators behind gateways that return those codes for unknown methods rather than tunneling the JSON-RPC envelope. This is opt-in and documented as a security-relevant knob: enabling it weakens the protocol-downgrade resistance the default rule provides.
   - If both probes fail with non-recoverable errors, return `McpClientError::ProtocolNegotiationFailed { tried: [V2026_07_28, V2025_11_25], last_error: ... }`.

3. **Per-connection immutability.** Once a version is locked for an `McpClient` instance, it does not change. Callers that need to talk to a server that has been upgraded mid-session must `disconnect()` and construct a new client. This is consistent with the existing 404-driven re-initialization (`crates/turul-mcp-client/src/client.rs:452-466`) which already requires fresh handshake on session loss.

### `client-2025-11-25-only` / `client-2026-07-28-only` narrowing

When either single-spec feature is active, the version field becomes `Arc<RwLock<McpVersion>>` (no `Option`, no probe). `connect()` drives the corresponding handshake directly and treats spec mismatch as a hard error (`McpClientError::ServerUnsupported`). This is the legacy-CLI / embedded path and intentionally trades flexibility for binary size.

## Consequences

**Positive:**

- A single `turul-mcp-client` binary can talk to the entire MCP server ecosystem during the 2025-11-25 → DRAFT-2026 transition window. No two-binaries deployment story.
- Per-connection version detection scales naturally to the gateway / aggregator case (B) without requiring new public API.
- Decouples client release cadence from the server-side `turul-mcp-protocol` alias flip (ADR-027 Phase 9.4). Clients can ship bilingual support before the alias flips, with no breaking change at the alias-flip moment.
- The existing `set_bearer` token-rotation surface (`crates/turul-mcp-client/src/client.rs:282-294`) and 404 re-init contract (line 452-466) compose orthogonally with version selection — no rewiring needed.
- Test infrastructure can exercise both specs in one binary (use case D), strengthening the regression net for the cutover.

**Negative / trade-offs:**

- Client binary is meaningfully larger by default. Both protocol crates are linked. Estimated overhead: 1.3-2.0k LOC of routing/serialization glue plus the two protocol crates' types. Mitigation: build the leaf binary with `cargo build --no-default-features --features http,sse,client-2025-11-25-only` (or `…,client-2026-07-28-only`) to narrow. The `--no-default-features` is mandatory: without it the `client-bilingual` default also activates, all three protocol features are mutually exclusive, and the `compile_error!` mutex fires.
- Probe-then-fallback adds round-trip latency to first `connect()` against a 2025-11-25 server (one extra failed `server/discover` request before the `initialize` retry). Mitigation: callers who know their server's spec set `mcp_protocol_version` explicitly to skip the probe.
- Two protocol-type universes inside one crate raises the cost of refactors that touch protocol types — internal helpers must be aware of which version they're dispatching to. Mitigation: keep the `protocol/v2025.rs` and `protocol/v2026.rs` modules cleanly separated; public client API stays version-agnostic.
- Behavior-difference matrix between specs (stateful vs stateless, session-id vs `_meta`, deprecated `notifications/cancelled` semantics, error code `-32602` vs `-32002`) must be encoded in the routing layer. Each behavior is documented inline with a citation to the relevant ADR-027 §"Schema-fidelity corrections" entry.

**Failure modes documented (codex P1-5 tightened, 2026-05-31):**

- **2026 client → 2025 server.** `server/discover` returns valid JSON-RPC `-32601 Method Not Found`. Client auto-falls-back to `initialize`. Connection succeeds at 2025-11-25.
- **2026 client → 2026 server behind misconfigured auth gateway.** Gateway returns HTTP 401 or 403. Client surfaces `McpClientError::ConnectionFailed { status: 401, ... }`; **does NOT downgrade.** The user fixes the gateway, not the protocol.
- **2026 client → 2026 server behind misconfigured route.** Gateway returns HTTP 404 or 405. Client surfaces `McpClientError::ConnectionFailed { status: 404, ... }`; **does NOT downgrade.** Escape: set `ClientConfig.allow_legacy_gateway_fallback = true` to broaden the fallback trigger, with the caveat that this weakens downgrade resistance.
- **2026 client → server that responds with `-32700`/`-32600`/`-32602`/`-32603`** for `server/discover`. The server understood the request and rejected it. Client surfaces `McpClientError::ProtocolError`; **does NOT downgrade.**
- **2025-only client → 2026 server** *(only when the client was built with `--no-default-features --features http,sse,client-2025-11-25-only`)*. `initialize` returns `-32601` (the 2026 server has no such method). Client fails with `McpClientError::ServerUnsupported { suggested: V2026_07_28 }`. Caller must rebuild without `--no-default-features` (gets bilingual) or with `client-2026-07-28-only` instead.
- **2026 client → 2026 server, but server is mid-deploy** *(stateless server quirks)*. `server/discover` returns a transient 5xx. Existing retry policy in transport applies; `connect()` propagates the eventual terminal error per `McpClientError::ConnectionFailed`.
- **Bilingual client → 2025 server, then operator upgrades server mid-session.** The locked-at-`connect()` version becomes wrong. Subsequent requests fail with 404 (session lost on the now-stateless server). Existing 404 path tears down the session (`crates/turul-mcp-client/src/client.rs:452-466`); but bilingual auto-redetection on re-init is **out of scope for the initial slice** — caller must `disconnect()` and reconnect for the new spec to be negotiated. Tracked as a follow-up.

## Alternatives considered

1. **Feature-gated mirror of the server's `protocol-2025-11-25` choice (rejected).** Apply ADR-029's pattern symmetrically: the client is also single-spec-per-build, default 2026, with `--features protocol-2025-11-25` for the old wire. Rejected because it forces operators of CLI / gateway / IDE / test-client scenarios to ship and deploy two separate client binaries, with no runtime way to pick the right one for a given target server. The server-side argument (one process, one handshake state machine) does not transfer — a client has no state machine of its own that the build must commit to.

2. **2025-only client, no fallback (rejected).** Ship `turul-mcp-client 0.4.0` continuing to speak only 2025-11-25 and defer 2026 support to a later release. Rejected because it strands every user who wants to talk to a 2026 server during the transition, and it conflicts with the user-locked decision (workflow context) that 0.4.0 defaults to 2026 on the server side. Symmetry argues for client support of 2026 in the same release.

3. **2026-only client, no fallback (rejected).** Drop 2025-11-25 support entirely in `turul-mcp-client 0.4.0`. Rejected because it strands every user with a still-running 2025-11-25 server — which is the entire `main`-branch installed base. The transition window is non-zero; a client that cannot speak the legacy wire is operationally dead-on-arrival for anyone who has not also rebuilt their servers.

4. **Dual-import via re-export alias (rejected).** Have `turul-mcp-protocol` (the alias crate) export both `mod v2025` and `mod v2026` and let downstream consumers select per-call site. Rejected because it bleeds bilingual complexity to every consumer of the alias (server crates included, where ADR-029's single-spec assumption depends on a single set of imported protocol types). Bilingual concerns belong inside the one crate that needs them — the client.

5. **Stateless 2025/2026 sniff via response shape (rejected).** Send a neutral ping and inspect the response shape to infer the server's spec. Rejected because there is no fully neutral RPC in MCP — every method that exists in both specs has subtle param/result differences, and the `_meta` carrier is required on 2026 requests but absent on 2025. A probe that distinguishes by method existence (`server/discover` vs `initialize`) is cleaner and more robust.

## Relationship to ADR-027 and ADR-029

- **ADR-027** locks the protocol crate (`turul-mcp-protocol-2026-07-28`) to wire string `"DRAFT-2026-v1"`. The workspace alias flip (Phase 9.4) is committed per ADR-029 §"What the cutover slice ships" item 5 (flip-all-at-once). This ADR is independent of the alias flip: `turul-mcp-client` imports both versioned crates directly, not through the alias, so the alias-flip moment is invisible to client consumers.
- **ADR-029** chooses single-spec-per-build for `turul-mcp-server` / `turul-http-mcp-server` / `turul-mcp-aws-lambda`. This ADR diverges from that choice for the client. The divergence is deliberate and is justified by the architectural asymmetry between server and client (process-state-machine constraint vs per-connection byte-emission flexibility).
- The bilingual client is a pre-requisite for cross-version integration testing during the cutover. Once Phase 9.4 lands and ADR-029's server-side cutover is complete, the bilingual client lets us run a single test binary that exercises both legacy and current servers from one process.

## Revision log

- **2026-05-31** — initial. Bilingual-default strategy for `turul-mcp-client`; per-connection version detection (explicit hint then try-discover-then-fallback); features `client-2025-11-25-only` / `client-2026-07-28-only` for binary-size narrowing; failure modes and out-of-scope items documented. ADR authored to complement ADR-029 (server-side single-spec) and ADR-027 (protocol-crate targeting). Implementation slice not yet scheduled — to be tracked in `docs/plans/2026-07-28-PARKED.md`.
- **2026-05-31 (correction, codex P1-4) — SUPERSEDED by the 2026-05-31 re-correction entry below.** §"Cargo topology" was rewritten at this point to drop `bilingual` from the default features list and treat it as the implicit default (absence of any narrowing flag). That intermediate design did not survive the next codex pass and is documented here only for audit history; the operative design lives in the current §"Cargo topology" and in the re-correction entry below.
- **2026-05-31 (correction, codex P1-5)** — §"Version detection mechanism" and §"Failure modes documented" tightened. Initial wording allowed fallback to 2025-11-25 on JSON-RPC `-32601` *or HTTP 4xx indicating an unknown method*; that latter clause silently downgrades on auth failures, gateway misroutes, and missing-header errors. Corrected: fallback is triggered **only** by a valid JSON-RPC response carrying `error.code == -32601`. HTTP 4xx surfaces as `McpClientError::ConnectionFailed` without downgrade. An opt-in `ClientConfig.allow_legacy_gateway_fallback` escape hatch broadens the rule for known-broken gateways with explicit operator acknowledgment.
- **2026-05-31 (re-correction, codex P0 second pass) — this is the OPERATIVE design.** §"Cargo topology" was rewritten *again*. The intermediate codex P1-4 design (see superseded entry above) was not executable: `optional = true` deps are only linked when a feature activates them via `dep:` syntax, and Cargo features cannot REMOVE deps — without a feature flag pulling them in, the default build would link NEITHER protocol crate. The operative design is the **explicit `client-bilingual` default feature** (the standard Cargo idiom used by `serde`, `tokio`, `reqwest` for mutually-exclusive defaults). Narrowing requires `--no-default-features` on the leaf; the `compile_error!` mutex catches the common footgun (forgetting `--no-default-features` → both the default `client-bilingual` and a narrowing feature active → compile error with explanatory message). A second `compile_error!` catches the no-protocol-feature case.
- **2026-06-01 (codex third pass, P2 narrowing-shorthand cleanup)** — §"Negative / trade-offs" mitigation bullet and §"Failure modes documented" entry for "2025-only client → 2026 server" both used stale shorthand (`--features client-2025-11-25-only`) without `--no-default-features`. That phrasing pointed readers at the exact footgun the mutex was designed to catch. Both occurrences now spell out `cargo build --no-default-features --features http,sse,client-2025-11-25-only` (and `…client-2026-07-28-only`) and document that `--no-default-features` is mandatory because the default `client-bilingual` is mutually exclusive with the narrowing features. The §"Cargo topology" section itself was already correct; this is purely a shorthand-in-prose cleanup.
- **2026-06-01 (codex fourth pass, P3 revision-log hygiene)** — The codex P1-4 and codex-P0-second-pass entries above were literally quoting the intermediate design's broken Cargo snippets, which read identically to operative content to a naive grep. Both entries are now labeled "SUPERSEDED" / "OPERATIVE" inline and the literal `default = ["http", "sse"]` and `implicit default` strings have been paraphrased away. The audit history is preserved but no future reader (or grep gate) can mistake them for current guidance.


- **2026-06-07 (client dual-spec captured as a gate; feasibility confirmed)** — The "one deployed client speaks both 2026-07-28 and 2025-11-25" requirement (this ADR's bilingual default) is now a 0.4.0 publication-gate condition in ADR-027 (condition d), with an acceptance test: one `McpClient` (`client-bilingual`) round-trips `tools/list` against both a 2026 stateless server and a 2025-11-25 stateful server in the same process. Feasibility de-risked by a throwaway build — a single crate links both versioned protocol crates and compiles. After the 2026-06-07 `turul-rpc` 0.2 standardization (ADR-025) the whole branch is single-major on `turul-rpc`, so the earlier "two `turul-rpc` majors in one binary" worry is moot. Implementation (Phase 4 of the rollout) remains unscheduled; the client today is still single-spec 2025-11-25. Open follow-up: ADR-001 §"Protocol Re-export Rule" will need a third exception when the client is built (it imports both versioned crates directly, bypassing the `turul-mcp-protocol` alias) — flagged, not yet applied.

- **2026-06-07 (bilingual client IMPLEMENTED — supersedes the "unscheduled / still single-spec" note in the entry above)** — The bilingual client landed on `feat/turul-mcp-protocol-2026-07-28`. It links both versioned protocol crates (`client-bilingual` default; `client-2025-11-25-only` / `client-2026-07-28-only` narrowing; `compile_error!` mutex), negotiates the wire spec per connection at `connect()` (`server/discover` → 2026; JSON-RPC `-32601` → fall back to `initialize` → 2025; HTTP 4xx and all other JSON-RPC errors abort WITHOUT downgrade; opt-in `allow_legacy_gateway_fallback` broadens to 404/405), and locks `McpVersion` for the connection lifetime. All core client operations route through `protocol/v2026_07_28` on a 2026 connection (per-request `_meta` + 2026 result shape): `tools/list`/`tools/call`, `resources/list`/`read`/`templates/list`, `prompts/list`/`get`, and the `*_paginated` variants. Removed-from-core methods (`ping`, `tasks/*`) are rejected on a 2026 connection and retained on 2025-11-25. Acceptance: `tests/bilingual_negotiation.rs` (negotiation + per-spec round-trip) and `tests/bilingual_2026_operations.rs` (every op against a mock 2026 server with `_meta` wire enforcement + removed-method rejection). **As-built module layout** is `protocol/mod.rs` + `protocol/v2026_07_28.rs`, with the 2025 path inline in `client.rs` — NOT the `v2025.rs`/`v2026.rs` pair the §"Internal module layout" sketch shows. The ADR-001 third re-export exception flagged in the prior entry IS now applied (ADR-001 revision log + CLAUDE.md §"Protocol Re-export Rule"). **Still pending** (tracked in the lane-1 gap inventory): MRTR `InputRequiredResult` union arm on `tools/call`/`resources/read`/`prompts/get`, a `completion/complete` client op, and server-initiated elicitation handling.
