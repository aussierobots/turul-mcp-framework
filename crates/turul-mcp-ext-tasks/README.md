# turul-mcp-ext-tasks

Rust bindings and durable storage for the **MCP Tasks** extension
(`io.modelcontextprotocol/tasks`, SEP-2663) — the durable poll-handle lifecycle
that MCP 2026-07-28 moved out of the protocol core.

## Read this before depending on it

**Upstream is experimental, and its schema is not frozen.** The
`modelcontextprotocol/ext-tasks` repository labels itself an *experimental*
extension that "may change significantly or be discontinued", and it publishes
its schema only at the mutable path `schema/draft/schema.ts` — there is no
released, dated artifact to pin against, and the repository carries no tags.

That is a materially weaker guarantee than the core protocol crates, which pin
an immutable released path (`schema/2026-07-28/schema.ts`). This crate pins a
**specific upstream commit plus a content checksum** instead, which is the
strongest provenance available for a source with no tags. Exact commit,
checksum and re-pin procedure: [`schema/README.md`](schema/README.md).

Practical consequences:

- Wire shapes here can change without an upstream version bump, because there
  is no upstream version to bump.
- This crate versions independently of the framework (`0.1.0`, not `0.4.x`) per
  SEP-2133 §Evolution. The `0.x` is doing real work — treat breaking changes
  between minor releases as expected, not exceptional.
- Extensions are **off by default** (SEP-2133). Adding the dependency and
  enabling the server feature is the opt-in; nothing here activates implicitly.

If you need a stable task API today, the 2025-11-25 opt-in lane carries tasks
as a *core* capability with the frozen 2025-11-25 schema behind it.

## What this crate is

Serde wire types, a `TaskStore` trait, and an `InMemoryTaskStore`. The
2026-07-28 lifecycle it models (SEP-2663):

- polling via `tasks/get` with a `pollIntervalMs` hint — there is no blocking
  `tasks/result`
- `tasks/update` delivers input responses to `input_required` tasks
- **no `tasks/list`** — removed in this redesign
- unsolicited task handles: `CreateTaskResult` with `resultType: "task"` in
  lieu of a standard result
- optional `notifications/tasks` status pushes, subscribed through
  `subscriptions/listen` with `taskIds` filter fields

Unlike its sibling `turul-mcp-ext-apps`, this crate **is** wired end to end.
Server dispatch lives in `turul-mcp-server` behind its `ext-tasks` feature
(`.with_ext_tasks(store)` + `.ext_task_tool(tool)`); the client side is
`call_tool_or_task` plus `task_get` / `task_update` / `task_cancel` /
`task_wait`. Server election, the `-32003` capability gate, mid-task input and
task notifications are all implemented. Worked example:
`examples/ext-tasks-server`.

## Spec-neutral by design

The crate carries no date suffix. Task support differs by spec line — 2025-11-25
keeps tasks in core, 2026-07-28 moves them to this extension — so each lane gets
a feature-gated module rather than a per-spec crate fork. Today only
`v2026_07_28` exists; the 2025-11-25 reconciliation still rides the core
protocol crates and has not been re-hosted here.

## Feature flags

`protocol-2026-07-28` (default) compiles the SEP-2663 bindings against
`turul-mcp-protocol-2026-07-28`. With `--no-default-features` the crate exports
nothing and does not pull in a protocol crate.

## Verification status

Covered by `ext_tasks_2026.rs`, which has turul code on **both** ends of the
wire. No independent implementation has driven this extension against us, so
these shapes are self-verified only — a disagreement about how a peer declares
the extension would not be detected today. Tracked in
[`docs/compliance/README.md`](../../docs/compliance/README.md); do not read the
green suite as external interop evidence.

## Testing

```bash
cargo test -p turul-mcp-ext-tasks     # wire-shape + lifecycle tests
./scripts/check-schema-pin.sh         # vendored spec pin + checksum gate
```

## References

- SEP-2663: <https://modelcontextprotocol.io/seps/2663-tasks-extension>
- SEP-2133 (extensions, off-by-default): <https://modelcontextprotocol.io/seps/2133-extensions>
- `docs/adr/028-extensions-strategy.md` — why extensions get their own crates

## Storage backends

| Feature | Backend | Durability |
|---|---|---|
| *(default)* | `InMemoryTaskStore` | single process; lost on restart |
| `sqlite` | `SqliteTaskStore` | survives restart; shared by processes on one file |
| `postgres` | `PostgresTaskStore` | shared across instances (`SELECT … FOR UPDATE` per task) |
| `dynamodb` | `DynamoDbTaskStore` | shared across instances, incl. Lambda; optimistic concurrency |

All four implement one `TaskStore` trait and are held to **one** behaviour
contract — `parity`, 14 invariants, run against every backend by
`scripts/ext-tasks-backends.sh`. Every status rule, owner check and
`tasks/update` key decision lives once in `traits.rs` as a pure transition; a
backend only loads, applies and stores. That is deliberate: the superseded
2025 store had three backends that no test executed and no gate built.

## Retention

Nothing is swept unless asked. `RetentionPolicy::default()` is a no-op, so a
server that never configures retention behaves exactly as before:

```rust
use std::time::Duration;
use turul_mcp_ext_tasks::RetentionPolicy;

.with_ext_tasks(store)
.with_ext_tasks_retention(
    RetentionPolicy {
        orphan_after_ms: Some(15 * 60_000),          // presumed-dead worker → failed
        delete_terminal_after_ms: Some(24 * 60 * 60_000),
        honour_task_ttl: true,                       // per-task `ttlMs`; null = unlimited
    },
    Duration::from_secs(60),
)
```

That one call configures both the sweep loop **and** DynamoDB's native
`ttlEpoch` expiry — DynamoDB reclaims items itself, with no sweep job. Its
deletion is *eventual* (AWS documents up to 48h), so reads also filter on
expiry; without that this backend would keep serving tasks the others consider
gone.

SEP-2663 makes all retention OPTIONAL (line 342: servers "MAY mark a task as
`failed` at any point after the TTL elapses, and subsequently delete it at any
time"). None of it is required for compliance — it is required for a
deployment that does not want an unbounded table.
