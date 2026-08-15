# MRTR-before-task composition (SEP-2663 × SEP-2322)

**Owner**: `turul-mcp-server`, `turul-mcp-ext-tasks`
**Task**: #86, final item
**Date**: 2026-08-15
**Governing text**: `crates/turul-mcp-ext-tasks/schema/sep-2663-tasks-extension.md`
(pinned at `9b44c6b4dcd2451bc49abd39e47eda36b396e8dd`), lines 302, 304, 592ff, 940.

## The premise in #86 is wrong, and that changes the work

#86 states: *"SEP-2663 composed with SEP-2322 **requires** the input round trip
to complete SYNCHRONOUSLY before a task is created. We create the task first."*

Both halves are wrong. The SEP says (line 304, restated 936):

> Server implementations that use multi round-trip requests in conjunction with
> task creation (for example, a tool that requires elicitation over
> `InputRequiredResult` before creating a task) **SHOULD** resolve all MRTR
> exchanges _synchronously_ before responding with a `CreateTaskResult`.

**SHOULD**, not MUST — and conditional on the server *choosing* that flow. The
SEP names two legitimate mechanisms, distinguished by *when* input is needed
(line 592ff):

| Input needed | Mechanism | Our status |
|---|---|---|
| **Before** the task exists (e.g. to decide whether to proceed) | MRTR on the original request; then `CreateTaskResult` | **Unreachable** — no API expresses it |
| **During** task execution | `inputRequests` via `tasks/get`, fulfilled via `tasks/update` | Implemented, correct ([`ext_tasks.rs:154`](../../crates/turul-mcp-server/src/ext_tasks.rs)) |

So this is a **missing capability**, not a wire defect. Nothing we emit today
violates a MUST. What is true is that a user cannot build the first flow with
this framework at all, which makes a SHOULD unsatisfiable rather than declined.
The conformance scenario `tasks-mrtr-composition` requires a *fixture*
exhibiting it, so it is also the thing standing between us and that scenario.

The SEP explains why it cannot be a MUST, and the reasoning constrains our
design: *"Prohibiting this would require imposing an artificial constraint with
no protocol-level mechanism to enforce it, since the client is unaware that the
server will create a task ahead of time."* The client cannot signal which flow
it wants. **The server must decide, and it must decide before running the tool.**

## Success criteria

1. A task-elected tool can resolve one or more MRTR rounds synchronously, then
   mint the task — reachable from the public builder API.
2. The existing flow is untouched: a tool registered as it is today still mints
   on the first call and parks via `tasks/update`. No existing registration
   changes behaviour.
3. `tasks-mrtr-composition` passes all three of its checks.
4. The behaviour is pinned by a Rust wire test, not only by the npx harness.
5. `wire-schema-valid`'s 7 task-scenario failures are separated into
   ours/upstream's with evidence, not written off.

## Design

### Part A — election is withheld until the MRTR round is answered

Election is a **registration-time** decision
([`builder.rs:448`](../../crates/turul-mcp-server/src/builder.rs)), not a
property of the tool type, so the marker belongs there. `ext_task_tools:
HashMap<String, bool>` becomes `HashMap<String, ExtTaskElection>`:

```rust
pub struct ExtTaskElection {
    /// Calls from clients that did not declare the extension are rejected
    /// with `-32021` rather than falling back to synchronous execution.
    pub required: bool,
    /// SEP-2663 line 304: withhold election until the request carries
    /// `inputResponses`, so this tool's MRTR round resolves synchronously
    /// and round 1 answers `InputRequiredResult` with no `taskId`.
    pub mrtr_first: bool,
}
```

`ext_task_tool` / `ext_task_tool_required` keep working unchanged
(`mrtr_first: false`); a new `ext_task_tool_with(tool, election)` reaches the
combination. At [`server.rs:2000`](../../crates/turul-mcp-server/src/server.rs)
election gains one condition — when `mrtr_first && params.inputResponses.is_none()`
it falls through to the existing synchronous path, where the tool's
`McpError::InputRequired` is already converted at `server.rs:2160`.

The discriminator is spec-grounded, not inferred from the harness:
`CallToolRequestParams extends InputResponseRequestParams`
(`schema.ts:1863`, fields at `:605`/`:608`), so `inputResponses` is a modelled
member of `tools/call` params.

**Rejected — a defaulted `McpTool::mrtr_preflight` hook.** It duplicates the
MRTR branch inside every such tool, and per #68 a derive-emitted
`impl McpTool` makes a defaulted method unoverridable (`E0119`), dragging both
proc macros into the slice for no gain.

**Rejected — running `execute` speculatively to see what it returns.** Either
the work runs twice or the response blocks for its full duration. The server
cannot distinguish "will ask for input" from "is slow" by observation; only the
registration can say.

### Part B — seed the worker, into the tool context and *not* into task state

**Outcome: no code change was needed.** The prediction below was wrong in one
direction and right in the other, and the test proves which.

Predicted: `create_and_spawn` starts its worker with `responses = None`
([`ext_tasks.rs:161`](../../crates/turul-mcp-server/src/ext_tasks.rs)), so on
round 2 the worker would re-ask and park the task forever.

Actual: [`server.rs:1832`](../../crates/turul-mcp-server/src/server.rs) injects
`inputResponses`/`requestState` into the session extensions **before** election
runs, and the worker only ever *inserts* into that map — so the session handed
to `create_and_spawn` already carries them and the tool sees
`input_responses()` on its first invocation inside the task. Verified by the
"Alice" assertion, which fails if the answer does not reach the worker.

**Where it is seeded matters, and the SEP is explicit** (line 940): the MRTR
phase's `inputRequests` keys are *consumed* when that phase ends, and the task
phase maintains its own keys independently — clients must not have to
deduplicate across the two. The existing plumbing already satisfies this: the
answers ride the **tool invocation context** (session extensions, surfaced via
`SessionContext::input_responses()`), while `TaskState.input_requests` /
`collected_responses` are initialised empty at
[`ext_tasks.rs:126`](../../crates/turul-mcp-server/src/ext_tasks.rs). A test
asserts the created task carries no `inputRequests` from the MRTR phase, so
this stays true rather than being true by accident.

### Part C — the fixture

[`main.rs:1532`](../../examples/conformance-fixture-server/src/main.rs) returns
`"Task with input complete"`. The scenario requires the final task text to
contain the value gathered during the MRTR phase (`"Alice"`, sent as
`{action:"accept", content:{name:"Alice"}}`). The fixture must extract and echo
it, and register via `ext_task_tool_with(.., ExtTaskElection { required: true,
mrtr_first: true })`.

### Part D — `wire-schema-valid`: separated, with evidence

The released **core** schema declares (`schema.ts:1849`):

```ts
export interface CallToolResultResponse extends JSONRPCResultResponse {
  result: CallToolResult | InputRequiredResult;
}
```

`CreateTaskResult` is not in that union — it is defined in the *extension*
schema, which the core union does not reference, and which `wire-schema-valid`
does not compose in. **Any** server implementing SEP-2663 fails this check on
exactly the responses that mint a task, however clean its envelope. This is a
spec-level gap upstream (the core schema does not model extension result types
on `tools/call`), not a harness bug and not our defect.

That is a justification for an `--expected-failures` entry, **not** a blanket
pass. The entry is written only after confirming the violation list contains
nothing but `CallToolResult: must have required property 'content'` on
task-minting responses. Precedent for not trusting the reading: `ttlMs: null`
in #86 was ours, and was fixed. Worth reporting upstream.

## Tests

New wire test in `crates/turul-mcp-server/tests/ext_tasks_2026.rs`, pinning all
three rounds without npx:

- round 1 → `resultType: "input_required"`, carries `inputRequests`, **no** `taskId`
- round 2 → `resultType: "task"`, top-level `taskId`, **no** `requestState`, **no** `inputRequests`
- `tasks/get` → terminal `completed`, `result` reflects the value gathered in round 1
- the created `TaskState` carries no `inputRequests` inherited from the MRTR phase (Part B / SEP line 940)
- **regression**: a tool registered *without* `mrtr_first` still mints on round 1 and parks via `tasks/update`

## Known sharp edge

Registering `mrtr_first` on a tool that never returns `InputRequired` executes
it synchronously and mints no task — silently. Guarded by the doc comment plus a
`warn!` on that path. Not a hard error: it is a mis-registration, not a wire
violation, and failing the call would be worse than completing it.

## Versioning

Patch bump `turul-mcp-server` 0.4.3 → 0.4.4 plus CHANGELOG. The change is
additive (`ExtTaskElection`, `ext_task_tool_with`); no existing signature or
behaviour moves. `turul-mcp-ext-tasks` bumps only if Part B alters its public
surface.

## Out of scope

- #71 (no peer drives the extension client-side) stays blocked and unaffected.
- The `mrtr_first` marker is not exposed through the derive macros. It is a
  server-side registration concern, reachable from the builder for both derived
  and manual tools, so #68's "macro-unreachable surface" objection does not
  apply.
