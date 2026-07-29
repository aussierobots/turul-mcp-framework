# Cross-Implementation Interop Test Plan

> **Purpose.** Every green result in this repo except one is our code on both ends of the
> wire. This plan replaces that closed loop with a matrix of independent implementations,
> in both roles — turul as server *and* turul as client.
>
> **Status: proposed.** Only cell **P→R** exists today (`scripts/interop-fastmcp.sh`).

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

Both peers are **betas**. That is a real constraint, not a footnote: it dictates that these
run as a pre-release manual gate, never as a blocking per-push CI check (see §6).

Known peer defects to design around, both already measured:

- FastMCP 4.0.0b1 segfaults inside CPython 3.14's asyncio C module *after* completing the
  exchange. Reproduced with FastMCP's **own** server as the peer, so it is not ours. The
  existing script tries 3.14 then falls back to 3.12.
- The TS SDK v2 beta is unpublished; it must be built from the tag, which is slower than an
  install and can break on upstream refactors.

---

## 3. The matrix

Rows are clients, columns are servers. **R↔R is the control** — if a cell fails but R↔R
passes for the same journey, the fault is at the boundary, not in our logic.

| client ↓ / server → | **R** (turul) | **P** (FastMCP) | **T** (TS SDK) |
|---|---|---|---|
| **R** (turul) | control — existing E2E | **R→P** — gap | **R→T** — gap |
| **P** (FastMCP) | **P→R** — *exists, 3 methods* | peer control | n/a |
| **T** (TS SDK) | **T→R** — gap | n/a | peer control |

**The most valuable missing row is R→\*.** `turul-mcp-client` has never been pointed at a
foreign server. It exposes ~30 public methods (`list_tools`, `call_tool`,
`call_tool_with_progress`, `call_tool_with_input_responses`, `read_resource`, `get_prompt`,
paginated variants, task methods…) and every one of them is validated only against our own
server. ADR-030 makes it deliberately *bilingual*, which is precisely the behaviour most
likely to disagree with a real peer.

Peer-to-peer cells (P↔T) are out of scope — not our contract to verify.

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

Ordered by evidence-per-unit-effort. Each phase is independently landable.

| Phase | Work | Closes |
|---|---|---|
| **1** | Extend `interop-fastmcp.sh` from J1 to J1+J2+J5 | Method coverage 3/22 → ~12/22 in the one cell that exists |
| **2** | `scripts/interop-typescript-sdk.sh` — build the client from the `v2.0.0-beta.1` tag, run J1+J2+J5+**J6** | Adds the *reference* implementation; first external test of bilingual negotiation |
| **3** | `R→P` and `R→T`: stand up FastMCP and TS SDK **servers**, drive them with `turul-mcp-client` | The entirely untested row — our client against foreign servers |
| **4** | J3 (MRTR) and J4 (subscriptions/progress) across all live cells | The two headline 2026 features, currently self-verified only |
| **5** | Harness consolidation: one runner, one matrix report, per-cell skip when a peer is unavailable | Makes it maintainable rather than three ad-hoc scripts |

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
