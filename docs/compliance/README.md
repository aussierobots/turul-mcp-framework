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

`turul` is a **self**-column: our client against our server. It is recorded for
completeness but proves nothing about the wire contract, because both halves
share our assumptions. Only python/typescript/go columns are external evidence.

---

## Scorecard

Two numbers matter and they are deliberately kept apart.

**Self-verified.** The suite is green: **64 gates, 3213 tests**, across the
default 2026-07-28 lane, the 2025-11-25 opt-in lane, the Lambda Runtime API
gate, the spec-mutex gate and the docs gate, via `scripts/ci-gates.sh all`.
Every one of those has turul code on both ends of the wire.

**Externally verified.** Independent implementations that have completed a real
journey against this framework:

| Peer | Version | Stable? | Direction | Methods | Probe |
|---|---|---|---|---|---|
| FastMCP (Python) | 4.0.0b1 | beta | peer → turul | 9 + 5 negatives | `scripts/interop-fastmcp.sh` |
| FastMCP (Python) | 4.0.0b1 | beta | turul → peer | 8 | `scripts/interop-turul-client.sh` |
| MCP Go SDK | v1.7.0 | **stable** | peer → turul | **9 + 5 negatives** | `scripts/interop-go-sdk.sh` |
| MCP TypeScript SDK | v2.0.0-beta.1 | beta | peer → turul | **0 — fails at `connect()`** | `scripts/interop-typescript-sdk.sh` |

The **Go SDK result is the strongest external evidence in the repo**, because
that peer is the only one that is not a pre-release. Its client completed
`server/discover`, `tools/list`, `tools/call` (twice), `resources/list`,
`resources/read`, `resources/templates/list`, `prompts/list`, `prompts/get` and
`completion/complete` — every request carrying `MCP-Protocol-Version:
2026-07-28` and correct `Mcp-Method`/`Mcp-Name` headers, with no `initialize`
and no session id anywhere — plus all five negative paths. No wire disagreement
was found.

The TypeScript cell is a **finding, not a defect on our side**. That SDK's
`DiscoverResultSchema` still requires a top-level `serverInfo`, which the
released schema removed — identity moved into
`_meta["io.modelcontextprotocol/serverInfo"]`. Its probe classifier reads the
failed parse as "not a modern server" and falls back to the `initialize`
handshake, which a 2026-only server rejects. Verified against the pinned
artifact; see [the interop matrix](../plans/interop-test-matrix.md) §3. Do not
loosen the server to accommodate a stale beta.

A third measure sits behind both: **12 of 88 upstream fixture directories are
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
