# Cross-Implementation Interop Test Plan

> **Purpose.** Almost every green result in this repo is our code on both ends of
> the wire. This plan replaces that closed loop with a matrix of independent
> implementations, in both roles — turul as server *and* turul as client.
>
> **Status: partially delivered.** Cells P→R and R→P pass against a real peer;
> R→R is the control. Probes for the Go and TypeScript SDKs are authored but
> their results are not yet recorded here.

---

## 1. Why this exists

Two facts bound the value of the current suite:

- **1276 + 423 + 400 tests pass, and all of them are self-referential.** The framework's own
  client talks to the framework's own server, or to wiremock. A contract both sides get
  wrong the same way is indistinguishable from a contract both sides get right.
- **Only 12 of 88 upstream fixture directories are modeled** (13.6%). The fixtures are the
  sole externally-authored bytes in the loop, and 86% of them are unexamined.

Interop testing is the cheapest instrument that can falsify "we implement 2026-07-28
correctly", because the other party has no shared assumptions with us.

It has already paid for itself once: `scripts/interop-fastmcp.sh` **failed on its first run**
and drove the content-negotiation fix in ADR-006. All 1276 local tests were blind to that,
because they asserted the framing we had chosen.

---

## 2. The implementations

| Short | Implementation | Role support | Availability | 2026-07-28? |
|---|---|---|---|---|
| **R** | turul (this repo, Rust) | client + server | local | yes (default lane) |
| **P** | FastMCP (Python) | client + server | `fastmcp==4.0.0b1` via `uv`, PyPI pre-release | yes |
| **T** | MCP TypeScript SDK | client + server | `v2.0.0-beta.1` **git tag only — not on npm** | yes ("modern era") |
| **G** | MCP Go SDK | client + server | `v1.7.0`, released 2026-07-28 | yes — **the only stable peer** |

The Go SDK matters disproportionately: it is the sole peer that is not a
pre-release. Its release notes describe the same rewrite this framework
implements — stateless model, per-request `_meta`, `server/discover` replacing
the handshake, MRTR replacing server-initiated calls, unified
`subscriptions/listen` — so it is the strongest available independent check.

Known peer defects to design around, both measured:

- FastMCP 4.0.0b1 segfaults inside CPython 3.14's asyncio C module *after*
  completing the exchange. Reproduced with FastMCP's **own** server as the peer,
  so it is not ours. The scripts try 3.14, then fall back to 3.12.
- The TS SDK v2 beta is unpublished; it must be built from the tag, which is
  slower than an install and can break on upstream refactors.

## 2a. The shared fixture

Every probe runs against `examples/interop-fixture-server`, whose surface is the
contract: tools `echo`/`add`, resource `file:///fixture/readme.md`, no templates,
prompt `greeting(name)`, and a completion provider for that prompt's argument.
Before it existed the probes ran against `minimal-server`, which exposes a single
tool — that alone capped interop at 3 of 22 methods. A shared fixture also means
a difference between two cells is a difference between the clients, not between
the servers they happened to be pointed at.

`examples/interop-client-probe` is the mirror image: it drives a *foreign* server
with `turul-mcp-client` and reports per-leg results without aborting, so a peer
that lacks prompts shows up as one failed leg rather than a probe that stopped.

## 3. The matrix

Rows are clients, columns are servers. **R→R is the control** — if a cell fails
but R→R passes for the same journey, the fault is at the boundary, not in our
logic.

| client ↓ / server → | **R** (turul) | **P** (FastMCP) | **T** (TS SDK) | **G** (Go SDK) |
|---|---|---|---|---|
| **R** (turul) | control — **pass** | **R→P — pass, 8 methods** | R→T — not built | R→G — not built |
| **P** (FastMCP) | **P→R — pass, 9 methods + 5 negatives** | peer control | n/a | n/a |
| **T** (TS SDK) | **T→R — fails at `connect()`** (see below) | n/a | peer control | n/a |
| **G** (Go SDK) | **G→R — pass, 9 methods + 5 negatives** | n/a | n/a | peer control |

Scripts: `interop-fastmcp.sh` (P→R), `interop-turul-client.sh` (R→R control and
R→P), `interop-typescript-sdk.sh` (T→R), `interop-go-sdk.sh` (G→R).

### T→R: a real wire disagreement

The TypeScript SDK **v2.0.0-beta.1** cannot complete `connect()` against a
2026-07-28 turul server. Its `DiscoverResultSchema` still requires a top-level
`serverInfo`, which the **released** schema removed — identity now travels in
`_meta["io.modelcontextprotocol/serverInfo"]`. Verified directly against the
pinned artifact: `DiscoverResult` in `schema/draft-schema.ts` declares
`supportedVersions`, `capabilities` and `instructions` only, and the sole
`serverInfo` occurrence in the whole schema is the `_meta` key.

The SDK's probe classifier treats the failed parse as "not modern evidence" and
falls back to the `initialize` handshake, which a 2026-only server rejects. So
the failure cascades from one stale field to a full connection failure.

**Disposition: no change on our side.** The beta predates the schema correction.
Re-run when the SDK moves; do not loosen the server to accommodate it.

Peer-to-peer cells are out of scope — not our contract to verify.

---

## 4. Journeys per cell

Each cell runs the same ladder, through a logging proxy so assertions are on **captured
bytes**, not a client's self-report.

**J1 — modern core** (baseline; `P→R` covers this today)
`server/discover` → `tools/list` → `tools/call`. Assert `MCP-Protocol-Version: 2026-07-28`
on every request; assert **absence** of `initialize`, `notifications/initialized`,
`Mcp-Session-Id`.

**J2 — full read surface**
`resources/list` → `resources/read` → `resources/templates/list` → `prompts/list` →
`prompts/get` → `completion/complete`. Assert `resultType` on every result, and `ttlMs` +
`cacheScope` present on all five cacheable results.

**J3 — MRTR round trip**
Tool returns `resultType: "input_required"` with `inputRequests`; client retries the
*original* request with `inputResponses`. Assert no server-initiated `elicitation/create`
and no `notifications/elicitation/complete` anywhere in the capture.

**J4 — streaming and notifications**
`subscriptions/listen` with an opt-in filter → assert
`notifications/subscriptions/acknowledged` first, then that every notification carries
`_meta["io.modelcontextprotocol/subscriptionId"]`. Separately: a `tools/call` declaring
`_meta.progressToken` receives SSE framing with `notifications/progress`; one declaring
neither token nor `logLevel` receives a single JSON object (ADR-006, 2026-07-29).

**J5 — negative paths** (the half interop probes usually skip)
Unsupported version → `-32022`; missing `MCP-Protocol-Version` → `-32020`; `Mcp-Name`
disagreeing with the body → `-32020`; unknown method → HTTP 404 + `-32601`; unknown
resource → `-32602`.

**J6 — era negotiation** (T only, and the reason T matters most)
The TS SDK v2's `versionNegotiation: { mode: 'auto' }` probes with `server/discover`.
Point it at a turul **2026** server → expect era `modern`. Point it at a turul
**2025-11-25 opt-in** server → expect fallback to the `initialize` handshake and era
`legacy`. This is the only external test of ADR-030's bilingual contract, and nothing in
the repo tests the fallback direction against foreign code.

Coverage today: **J1 only, in one cell.** 3 of 22 methods, one happy path, zero negatives.

---

## 5. Phasing

| Phase | Work | State |
|---|---|---|
| **1** | Extend `interop-fastmcp.sh` from J1 to J1+J2+J5 | **done** — 3 methods → 9, plus 5 negatives |
| **1a** | A shared fixture server so every peer hits one surface | **done** — `examples/interop-fixture-server` |
| **2** | `scripts/interop-typescript-sdk.sh`, J1+J2+J5+J6 | **done, cell fails** — the SDK beta's stale `DiscoverResult` schema blocks `connect()` |
| **3** | R→P: drive a FastMCP server with `turul-mcp-client` | **done** — 8 methods, with an R→R control |
| **3a** | G→R: the Go SDK v1.7.0, the only stable peer | **done** — J1+J2+J5 green, no wire disagreement |
| **4** | J3 (MRTR) and J4 (subscriptions/progress) across live cells | not started — the two headline 2026 features remain self-verified only |
| **5** | One runner, one matrix report, per-cell skip when a peer is unavailable | not started — currently four ad-hoc scripts |

Coverage today, measured rather than estimated: **9 of 22 methods** exercised by
each of two independent clients (FastMCP and the Go SDK), **8** driven by our
client against an independent server (R→P), 5 negative paths confirmed twice
over, and **zero** coverage of MRTR, subscriptions or progress by any peer.

---

## 6. How this fits the testing strategy

Three tiers, and interop is deliberately the slowest and least blocking:

| Tier | What | When | Blocking |
|---|---|---|---|
| 1 | Unit + protocol compliance + fixture round-trip | every push | yes |
| 2 | Our own wire-level E2E (20 `*_2026*.rs` suites, ~128 tests) | every push | yes |
| 3 | **Cross-implementation interop (this plan)** | pre-release, manual | **no** |

Tier 3 must not gate a push: it needs network access and pins two pre-release peers, so a
red result is as likely to mean "the beta moved" as "we regressed". Its job is to catch what
tiers 1 and 2 *structurally cannot* — assumptions baked into both ends of our own wire.

Standing checks worth automating cheaply, because both peers are moving:

- `npm view @modelcontextprotocol/sdk version` and the `dist-tags` — when v2 leaves beta and
  reaches npm, phase 2 gets simpler and this becomes the primary peer.
- `pip index versions fastmcp --pre` — when FastMCP 4 goes stable, the Python 3.14 fallback
  in the script should be re-tested and probably removed.

---

## 7. Success criteria

- Every live cell passes J1 and J5, with assertions on proxy-captured bytes.
- `turul-mcp-client` has completed a full journey against **at least one** foreign server —
  currently zero.
- J6 passes both directions (modern and legacy fallback) against the TS SDK.
- Each script exits non-zero on failure, names the cell and journey, and prints the wire
  capture on both success and failure.
- Peer versions are pinned in the scripts and recorded in the CHANGELOG when bumped —
  an interop result is only meaningful against a named peer version.

## 8. Explicit non-goals

- Peer-to-peer (P↔T) verification — not our contract.
- Making tier 3 a blocking CI gate while both peers are pre-release.
- Interop for the deprecated surfaces (Roots/Sampling/Logging) or the 2025-11-25 lane beyond
  J6's fallback leg. The frozen lane's contract is not changing.
