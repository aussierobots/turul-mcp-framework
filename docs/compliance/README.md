# MCP 2026-07-28 Compliance Artifacts

Per-area records of what this framework implements, how each requirement was
verified, and which independent implementations it has actually interoperated
with. These are a release gate: a requirement is not "done" until this file
names the test that asserts it.

| Area | File |
|---|---|
| Base Protocol | [base-protocol.md](base-protocol.md) |
| Server Features | [server-features.md](server-features.md) |
| Client Features | [client-features.md](client-features.md) |
| Extensions | [extensions.md](extensions.md) |

Spec: <https://modelcontextprotocol.io/specification/2026-07-28>.
Pinned schema: `crates/turul-mcp-protocol-2026-07-28/schema/schema.ts`, vendored
from the released `schema/2026-07-28/schema.ts`. Provenance is enforced by
`scripts/check-schema-pin.sh`.

---

## How to read a row

**Status** — `Implemented`, `Partial`, `Not implemented`, `Deprecated-by-spec`,
`Removed-by-spec`, `Out-of-role` (the obligation belongs to a component this
framework is not, e.g. an authorization server), or `Unknown`.

**Verified by** — the *named test function that asserts this requirement*.
A test that merely constructs the type does not qualify and is recorded as
`construction-only`. `NOT FOUND` means the code exists but nothing asserts it —
which is a coverage gap, recorded rather than hidden.

**Interop** — whether an *independent* implementation has exercised this over
the wire. Values:

| Value | Meaning |
|---|---|
| `pass` | A named probe script drove this and asserted on proxy-captured bytes |
| `fail` | A peer exercised it and disagreed |
| `—` | Not exercised. **This is not a pass.** |
| `n/a` | Not applicable to that peer's role |

**A fix does not upgrade an interop cell.** When code changes so that a recorded
`fail` should no longer occur, the cell becomes `—` (not exercised since the
fix), never `pass`. Only a re-run probe can write `pass`. Applied in
[client-features.md](client-features.md) §4, where the GET SSE row's `fail`
cells were retired without being promoted.

`turul` is a **self**-column: our client against our server. It is recorded for
completeness but proves nothing about the wire contract, because both halves
share our assumptions. Only python/typescript/go columns are external evidence.

---

## Scorecard

Two numbers matter and they are deliberately kept apart.

**Self-verified.** The suite is green across the default 2026-07-28 lane, the
2025-11-25 opt-in lane, the Lambda Runtime API gate, the spec-mutex gate and the
docs gate, via `scripts/ci-gates.sh all`. Every one of those has turul code on
both ends of the wire.

**Measured 2026-07-29 by one `scripts/ci-gates.sh all` run: 68 gates, 3258 tests,
0 failures.** Up from 64 gates / 3213 tests before that day's behaviour slices.

**Externally verified.** Independent implementations that have completed a real
journey against this framework:

All cells below re-measured **2026-08-08**, except the two FastMCP rows,
re-measured **2026-08-15** against `4.0.0b3`.

The FastMCP pin sat on the superseded `4.0.0b2` for two releases while the
probe's own pin-currency check warned about it on every run. The check worked;
nothing acted on a warning. Treat a pin on a pre-release as a claim with a short
shelf life — and note the same version was pinned in two scripts, which drifted
apart the moment one was updated.

| Peer | Version | Stable? | Direction | Methods | Probe |
|---|---|---|---|---|---|
| FastMCP (Python) | 4.0.0b3 | beta | peer → turul | 9 + 5 negatives | `scripts/interop-fastmcp.sh` |
| FastMCP (Python) | 4.0.0b3 | beta | turul → peer | 9 driven, 8 answered | `scripts/interop-turul-client.sh` |
| **MCP Python SDK** | **2.0.0 (PyPI)** | **stable** | peer → turul | 9 + 5 negatives | `scripts/interop-python-sdk.sh` |
| MCP Go SDK | v1.7.0 | **stable** | peer → turul | 9 + 5 negatives | `scripts/interop-go-sdk.sh` |
| MCP TypeScript SDK | 2.0.0 (npm) | **stable** | peer → turul | 9 + 5 negatives | `scripts/interop-typescript-sdk.sh` |

**Three stable peers now agree with this framework on the same 14 checks** — the
Python SDK, the Go SDK and the TypeScript SDK each drove `server/discover`,
`tools/list`, `tools/call`, `resources/list`, `resources/read`,
`resources/templates/list`, `prompts/list`, `prompts/get` and
`completion/complete`, every request carrying `MCP-Protocol-Version: 2026-07-28`
and correct `Mcp-Method`/`Mcp-Name` headers, with no `initialize` and no session
id anywhere, plus all five negative paths. None found a wire disagreement.

The reference Python SDK is the newest and most load-bearing of the three:
`mcp` is published by the protocol authors, so agreement with it is a stronger
signal than agreement with any framework built on top of MCP.

Two accounting notes, both deliberate:

- **turul → FastMCP reads "9 driven, 8 answered".** Our client now issues all
  nine; the peer returns `-32601` for `completion/complete` on its server side.
  The R→R control passes that leg, which places the gap at the peer. Counting
  the drive as coverage would credit a leg no peer ever served.
- **The Go cell was not reproducible until 2026-08-08.** Its pin-currency check
  referenced `GO_SDK_VERSION` above the line that assigns it, so under `set -u`
  the probe aborted on every run while the recorded result stayed green. Fixed,
  re-run, genuinely passing. Skips in the Go and turul-client probes now exit
  **77** rather than 0, so an unrunnable cell can no longer read as a pass.

An earlier revision of this file recorded the TypeScript cell as failing and
called the Go SDK "the only stable peer". Both were wrong, from one cause: the
probe pinned a superseded `v2.0.0-beta.1` git tag while `@modelcontextprotocol/client@2.0.0`
was already published, and the freshness watch pointed at
`@modelcontextprotocol/sdk`, which carries only the 1.x line. Each probe now
asserts its pin against the registry.

That fourth peer — the reference Python SDK `mcp==2.0.0` — was recorded here as
**untested** until 2026-08-08. It is now covered by `scripts/interop-python-sdk.sh`
and passed on its first run, finding no wire disagreement.

**The two headline features are no longer self-verified.** As of 2026-08-08 the
reference Python SDK cell also covers:

- **J3 — MRTR (SEP-2322).** The `-32021` capability gate fires for a client that
  did not declare `elicitation`, and a client that did completes the two-leg
  round trip — with the **SDK driving the retry itself**, so a foreign client
  finished MRTR unaided. No `elicitation/create` and no elicitation-complete
  notification appear anywhere in the capture.
- **J4 — the notification surface, both halves.** Request-scoped progress: SSE
  framing plus three `notifications/progress`, each echoing the token the
  *client* declared; the probe asserts that match, because a token a client
  cannot correlate is noise. And `subscriptions/listen`: acknowledged first,
  then opt-in filtering — the fixture broadcasts four flavours and only the
  requested one is delivered, each frame carrying a `subscriptionId`.

**Still uncovered by any peer**, stated plainly so the absence is not read as
coverage:

- **The tasks extension (SEP-2663).** No interop probe drives it. Our own
  `ext_tasks_2026.rs` has turul code on both ends, so it cannot detect a
  disagreement about how a client declares the extension. Task #71.
- **`turul-mcp-client` against a real 2025-11-25 server.** The 2026 lane is
  covered against a real server; the 2025 lane is covered only against wiremock
  stubs, which cannot disagree with the client that shares their author. This
  also blocks J6's legacy-fallback leg. Task #72.

See [`../plans/interop-test-matrix.md`](../plans/interop-test-matrix.md) §4–§5.

### Conformance score — the strongest external signal we have

**37 of 37 scored scenarios pass**, measured **2026-08-15**.

| | |
|---|---|
| Harness | `@modelcontextprotocol/conformance@**0.2.0-alpha.11**` |
| Invocation | `server --requirements 2026-07-28 --url …` |
| Fixture server | `examples/conformance-fixture-server` |
| Full run | 183 checks passed, 8 failed |
| **Scored** | **37 scenarios, 0 failing** |

**The pin is an alpha, and that qualifies the number.** `0.2.0-alpha.11` is the
newest alpha and the `alpha` dist-tag; npm `latest` is `0.1.16`, which predates
2026-07-28. So the alpha is the only harness that can score this revision — but
a pre-release pin is a claim with a short shelf life, and this file has already
been burned once by exactly that (see the FastMCP `4.0.0b2` note above). Re-run
before quoting the number.

**Read "scored" precisely.** The harness itself excludes 13 of the 50 scenarios
it runs, labelling them *"Not scored for 2026-07-28 … These do not affect
conformance"*:

- **10 `tasks-*`** — the `io.modelcontextprotocol/tasks` extension (SEP-2663).
  Two now pass outright; the other eight fail on **one check each**, and that
  check is the same one in every case (see below). This is not incidental: no
  SDK implements SEP-2663 client-side, so this suite is the *only* external
  check that could exist for our tasks wire format. Task #71.
- **3 `pending`** — all three passing.

Quoting "37/37" without that denominator would overstate it.

#### The 8 remaining unscored failures are all one check, and it is upstream's

Counted by check id, not by scenario, because all eight scenarios fail on the
*same* check:

| Count | Check | Owner |
|---|---|---|
| 8 | `wire-schema-valid` — `CallToolResult: must have required property 'content'` | **Upstream** |

**Nothing of ours fails the suite.** Everything the harness raised against this
implementation has been fixed: the last one was `tasks/update` rejecting an
entire request over a key the task was not waiting on (task #88).

The eight are **structural and not fixable by any server.** The released core
schema declares (`crates/turul-mcp-protocol-2026-07-28/schema/schema.ts:1849`):

```ts
export interface CallToolResultResponse extends JSONRPCResultResponse {
  result: CallToolResult | InputRequiredResult;
}
```

`CreateTaskResult` is absent from that union — it is defined in the *tasks
extension* schema, which the core union never references and which
`wire-schema-valid` does not compose in. So every response that correctly mints
a task fails a check derived from a schema that cannot describe it. Any
conforming SEP-2663 server fails these eight, however clean its envelope.

That reading was earned, not assumed. The same investigation found three
failures in the identical batch that were genuinely ours and are now fixed:
`ttlMs: null` (0.4.3), a fixture registered `optional` where the scenario needs
`required`, and the `tasks/update` key handling above. Worth reporting upstream.

**What the run found.** Three defects, none of which the 3600-test internal
suite could see, because it has turul code on both ends of every wire:

| Defect | Where |
|---|---|
| Macro-authored tools reported a tool's own failure as a JSON-RPC error, not `isError: true` | 0.4.2 |
| `resources/read` rejected a resource's own declared mimeType | 0.4.2 |
| **A matching `Host` header defeated Origin validation — DNS rebinding unblocked** | `683b925` |
| Undeclared-extension `tasks/*` answered `-32602` where SEP-2663 requires `-32021` | 0.4.3 |
| `ttlMs: null` on `CreateTaskResult`; answered MRTR keys not dropped; no SEP-2243 header validation on the tasks surface | 0.4.3 |
| MRTR-before-task composition was unreachable, so SEP-2663's SHOULD could not be satisfied | 0.4.4 |
| `tasks/update` rejected the whole request — `taskId` included — over a key the task was not waiting on | ext-tasks 0.1.2 |

The DNS-rebinding one was a live vulnerability, same class as the TypeScript SDK's
GHSA-w48q-cv73-mx4w, and it had an ADR *specifying* the defective rule. That is
the case for this suite in one line.

**`--expected-failures` is deliberately not in use.** Nothing is currently
suppressed; every scored scenario genuinely passes and every unscored failure
is visible above. Adopt the file only when a failure is provably upstream's,
with a one-line justification per entry — an unexplained entry is a hidden
failure, not a known one.

The eight `wire-schema-valid` failures now meet that bar, and the file is
*still* not adopted: they sit in scenarios the harness already excludes from
conformance, so suppressing them would buy nothing and cost visibility. The
separation above is the artifact; a YAML entry would only hide it.

A fourth measure sits behind all of it: **12 of 88 upstream fixture directories
are modeled** (13.6%). Those are a different artifact from the scenarios scored
above — the compliance harness's own vendored fixtures — and 86% of them remain
unexamined. A green suite does not speak to those.

---

## Maintenance

These artifacts are reconciled **in the same slice** as any of:

- a schema re-pin (`mcp-compliance-2026-07-28 refresh --write`) — see
  AGENTS.md §Released: 0.4.0 → "Schema pin governance"
- a behaviour change to a governed requirement
- a new or moved test that changes a "Verified by" cell
- an interop probe run that changes an interop cell

A row whose "Verified by" cell names a test that no longer exists is a defect,
not a stale doc. `scripts/ci-gates.sh` is the source of truth for which suites
actually run.
