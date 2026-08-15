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

All cells below re-measured **2026-08-08**.

| Peer | Version | Stable? | Direction | Methods | Probe |
|---|---|---|---|---|---|
| FastMCP (Python) | 4.0.0b2 | beta | peer → turul | 9 + 5 negatives | `scripts/interop-fastmcp.sh` |
| FastMCP (Python) | 4.0.0b2 | beta | turul → peer | 9 driven, 8 answered | `scripts/interop-turul-client.sh` |
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

A third measure sits behind all of it: **12 of 88 upstream fixture directories are
modeled** (13.6%). The fixtures are the only externally-authored bytes in the
compliance harness, and 86% of them are unexamined. A green suite does not
speak to those.

---

## Maintenance

These artifacts are reconciled **in the same slice** as any of:

- a schema re-pin (`mcp-compliance-2026-07-28 refresh --write`) — see
  AGENTS.md §Branch Lock → "Schema pin governance"
- a behaviour change to a governed requirement
- a new or moved test that changes a "Verified by" cell
- an interop probe run that changes an interop cell

A row whose "Verified by" cell names a test that no longer exists is a defect,
not a stale doc. `scripts/ci-gates.sh` is the source of truth for which suites
actually run.
