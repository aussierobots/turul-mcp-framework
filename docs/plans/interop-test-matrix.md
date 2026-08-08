# Cross-Implementation Interop Test Plan

> **Purpose.** Almost every green result in this repo is our code on both ends of
> the wire. This plan replaces that closed loop with a matrix of independent
> implementations, in both roles — turul as server *and* turul as client.
>
> **Status: partially delivered.** Four cells pass against real peers — P→R,
> T→R, G→R and R→P — with R→R as the control. The Go and TypeScript results are
> recorded in §3 (an earlier revision of this banner said they were not).
> Uncovered: MRTR (J3), subscriptions and progress (J4), and the legacy leg of
> J6.

---

## 1. Why this exists

Two facts bound the value of the current suite:

- **The whole local suite passes, and all of it is self-referential.** The framework's own
  client talks to the framework's own server, or to wiremock. A contract both sides get
  wrong the same way is indistinguishable from a contract both sides get right.
  (The per-lane counts previously quoted here — 1276 + 423 + 400 — predate several
  behaviour slices and are not restated; the current figure belongs in
  `docs/compliance/README.md` §Scorecard, from one `ci-gates.sh all` run.)
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
| **P** | FastMCP (Python) | client + server | `fastmcp==4.0.0b2` via `uv`, PyPI pre-release | yes |
| **P2** | **MCP Python SDK** (reference) | client + server | `mcp==2.0.0` on PyPI | yes — **stable** |
| **T** | MCP TypeScript SDK | client + server | `@modelcontextprotocol/client@2.0.0` on npm | yes — **stable** |
| **G** | MCP Go SDK | client + server | `v1.7.0`, released 2026-07-28 | yes — **stable** |

Three of the four tested peers are stable releases, not pre-releases: the Go
SDK v1.7.0, the TypeScript SDK 2.0.0, and the Python SDK `mcp==2.0.0`. Only
FastMCP is still a beta.

**P2 matters more than its row suggests.** FastMCP is a third-party framework
that happens to speak MCP; `mcp` is the reference implementation published by
the protocol authors. Until 2026-08-08 it had never been pointed at this
framework — "Python interop" was covered only by FastMCP. A wire disagreement
with the reference client is a stronger signal than one with any other peer.

An earlier revision of this document called the Go SDK "the only stable peer".
That was false when written: `@modelcontextprotocol/core@2.0.0` (npm,
2026-07-27) and `mcp==2.0.0` (PyPI, 2026-07-28) both predate it. The error came
from watching the wrong npm package — see §6.

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
| **R** (turul) | control — **pass** | **R→P — 9 driven, 8 answered** (see below) | R→T — not built | R→G — not built |
| **P** (FastMCP) | **P→R — pass, 9 methods + 5 negatives** | peer control | n/a | n/a |
| **P2** (Python SDK) | **P2→R — pass, 9 methods + 5 negatives** | n/a | n/a | n/a |
| **T** (TS SDK) | **T→R — pass, 9 methods + 5 negatives** | n/a | peer control | n/a |
| **G** (Go SDK) | **G→R — pass, 9 methods + 5 negatives** | n/a | n/a | peer control |

Scripts: `interop-fastmcp.sh` (P→R), `interop-python-sdk.sh` (P2→R),
`interop-turul-client.sh` (R→R control and R→P), `interop-typescript-sdk.sh`
(T→R), `interop-go-sdk.sh` (G→R).

All five cells re-measured 2026-08-08 against the pins recorded in §2.

### G→R had silently stopped running

The pin-currency check added under §6 was placed **above** the
`GO_SDK_VERSION` assignment it reads. Under `set -u` that aborted
`interop-go-sdk.sh` at line 42 on every invocation, so the cell recorded as
"pass" here was a stale measurement that nothing could reproduce. Fixed
2026-08-08 by moving the check below the assignment; the cell then passed for
real (J1+J2+J5, `sessionId=""` on all nine requests).

Two lessons, both now applied to the scripts:

- **A skipped cell exited 0.** `interop-go-sdk.sh` with no Go toolchain, and
  `interop-turul-client.sh` with no `uv`, both returned success. An absent peer
  was indistinguishable from a green run for anything reading the status code.
  Both now exit **77**.
- The probe that guards against a stale pin was itself the thing that broke the
  probe. A currency check is not free; it is code, and it needs the same
  once-through execution check as anything else.

### T→R: a resolved disagreement, and the lesson from it

The probe originally ran against git tag `v2.0.0-beta.1` and **failed at
`connect()`**: that beta's `DiscoverResultSchema` required a top-level
`serverInfo` which the released schema had removed, and its classifier read the
failed parse as "not a modern server" and fell back to the `initialize`
handshake a 2026-only server rejects. One stale field cost the whole connection.

The disposition then was "no change on our side, re-run when the SDK moves". The
SDK had already moved — **`@modelcontextprotocol/client@2.0.0` was on npm before
that measurement was taken**. Re-run against it: `connect()` succeeds,
`getProtocolEra()` reports `modern`, and all 9 methods plus 5 negatives pass.

The failure was never the peer's *current* behaviour; it was ours for pinning a
superseded pre-release and never checking whether it was still current. That is
why every probe now asserts its pinned peer version against the registry's
`dist-tags.latest` (§6).

### R→P: the client gained two methods the probe cannot yet drive

`turul-mcp-client` grew `complete()` and `cancel_request()` on 2026-07-29.
`completion/complete` was previously recorded as UNSUPPORTED in the R→P leg
because **no client method existed** — that was our gap, not the peer's, and it
is now closed on our side. The 8-method R→P result predates both methods and has
not been re-run, so this section records what is *reachable*, not what passed:

- **`completion/complete` — measured 2026-08-08, and the ceiling is the peer's.**
  Our client now *drives* 9 methods, so the client-side gap is closed. The peer
  answers 8: FastMCP returns `-32601 Method not found` for
  `completion/complete` on its server side. The R→R control passes the same leg,
  which is exactly what the control is for — it places the fault at the peer, not
  in our client. **Record R→P as "9 driven, 8 answered", not as "9 methods"**;
  reporting the drive count as coverage would credit us with a leg no peer ever
  served.
- **`notifications/cancelled` — reachable, and outside the ladder.** J1–J6 have
  no cancellation journey. Adding one would be new scope, not a re-run.

Separately, the GET SSE defect that FastMCP surfaced (`docs/compliance/client-features.md`
§4 — the client issued a GET the revision removed and took a 405 on every
connection) is fixed. **The probe has not been re-run against the fixed client**,
so the interop cell recording it reads `—`, not `pass`. Re-running
`scripts/interop-turul-client.sh` is the cheapest outstanding interop action in
this document: it would confirm one fix and fill one cell in a single pass.

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
`cacheScope` present on the **four** cacheable results in this journey —
`resources/list`, `resources/read`, `resources/templates/list`, `prompts/list`.
`GetPromptResult` and `CompleteResult` do not extend `CacheableResult`; the other
two of the spec's six (`DiscoverResult`, `ListToolsResult`) are asserted in J1.
An earlier revision said "all five", which matched neither set.

Also assert that `resources/read` reports the same `mimeType` the listing
advertised for that URI. The shared fixture declared `text/markdown` and read back
`text/plain` until 2026-07-29 and carried the mismatch as a documented known
discrepancy — a probe author who trusted the fixture would have encoded the bug.

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

Coverage today: **J1, J2 and J5 across five cells**, plus **J3 and J4 in the
Python SDK cell** — see §5 for the measured numbers. Only J6's legacy leg is
still untouched by any peer. (An earlier revision of this line read "J1 only, in
one cell — 3 of 22 methods", which described the state before Phase 1; a later
one said J3 and J4 were untouched, true until 2026-08-08.)

---

## 5. Phasing

| Phase | Work | State |
|---|---|---|
| **1** | Extend `interop-fastmcp.sh` from J1 to J1+J2+J5 | **done** — 3 methods → 9, plus 5 negatives |
| **1a** | A shared fixture server so every peer hits one surface | **done** — `examples/interop-fixture-server` |
| **2** | `scripts/interop-typescript-sdk.sh`, J1+J2+J5+J6 | **done** — 9 methods + 5 negatives against npm `@modelcontextprotocol/client@2.0.0`; J6 modern leg passes, legacy leg untested |
| **3** | R→P: drive a FastMCP server with `turul-mcp-client` | **done** — 8 methods, with an R→R control |
| **3a** | G→R: the Go SDK v1.7.0, the only stable peer | **done** — J1+J2+J5 green, no wire disagreement |
| **3b** | P2→R: the reference MCP Python SDK `mcp==2.0.0` | **done 2026-08-08** — J1+J2+J5 green on its first run |
| **4** | J3 (MRTR) and J4 (progress) against a live peer | **done 2026-08-08** — both green in the P2→R cell; see below |
| **4a** | J4 subscriptions (`subscriptions/listen` + filtered delivery) | not started — the progress half of J4 is covered, the subscription half is not |
| **5** | One runner, one matrix report, per-cell skip when a peer is unavailable | not started — currently five ad-hoc scripts |

### Phase 4: what J3 and J4 actually proved

Both run in `scripts/interop-python-sdk.sh` against the shared fixture, which
gained a `confirm` tool (MRTR) and a `count` tool (progress) for the purpose.

**J3 — MRTR (SEP-2322), two assertions:**

- *The capability gate.* A client that declares no `elicitation` capability is
  refused with `-32021`, carrying `data.requiredCapabilities.elicitation`. The
  server must not demand an input the client cannot answer. This one fired
  unprompted on the first run, against a probe that had simply forgotten to
  declare the capability — the gate works.
- *The round trip.* A client that does declare it gets `input_required` with
  `inputRequests` and an opaque `requestState`, and the **MCP Python SDK drives
  the retry itself** — the wire capture shows two consecutive `tools/call`
  frames and a final `resultType: "complete"` carrying the elicited answer. A
  foreign client completed MRTR unaided.
- Negatively: no `elicitation/create` and no `notifications/elicitation/complete`
  appear anywhere in the capture, which is what the stateless core requires.

**J4 — request-scoped progress:** a `tools/call` declaring
`_meta.progressToken` is answered with SSE framing and three
`notifications/progress` frames, **each echoing the client's own token**. The
probe asserts the token matches the one the request declared — a token a client
cannot match to its own request is noise, not correlation. The no-token case
correctly gets plain JSON and zero notifications (ADR-006).

Coverage today, measured rather than estimated: **9 of 22 methods** exercised by
each of four independent clients (FastMCP, the Go SDK, the TypeScript SDK and
the reference Python SDK), **9 driven / 8 answered** by our client against an
independent server (R→P — the peer does not serve `completion/complete`), 5
negative paths confirmed four times over, and MRTR + progress confirmed once,
by the reference Python SDK. Still uncovered by any peer: `subscriptions/listen`
and J6's legacy-fallback leg.

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

- **Each probe checks its own pin.** All four interop scripts compare their
  pinned peer version against the registry — npm `dist-tags.latest`, the newest
  PyPI upload including pre-releases, and the Go module proxy's `@latest` — and
  **warn** when the pin has fallen behind. Warn, not fail: a probe's job is to
  test the version it pinned, not to refuse to run because the peer shipped. This replaces a watch on `npm view @modelcontextprotocol/sdk`
  that was structurally blind: the v2 line ships as `@modelcontextprotocol/core`,
  `/client` and `/server`, and `@modelcontextprotocol/sdk` carries only 1.x. That
  one wrong package name produced two false published claims — "not on npm" and
  "the Go SDK is the only stable peer" — and neither was catchable by any test in
  this repo.
- `pip index versions fastmcp --pre` — when FastMCP 4 goes stable, the Python 3.14 fallback
  in the script should be re-tested and probably removed.

---

## 7. Success criteria

- Every live cell passes J1 and J5, with assertions on proxy-captured bytes.
- `turul-mcp-client` has completed a full journey against **at least one** foreign server —
  met by R→P (8 methods against FastMCP, with an R→R control). An earlier
  revision of this line still read "currently zero", contradicting §3 and §5 in
  the same document. The remaining shortfall is scope, not existence: R→P covers
  8 of the 9 methods the foreign clients drive against us, and `completion/complete`
  — the ninth — became reachable from our client on 2026-07-29.
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
