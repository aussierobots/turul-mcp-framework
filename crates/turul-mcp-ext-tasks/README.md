# turul-mcp-ext-tasks

Rust bindings and an in-memory store for the **MCP Tasks** extension
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
