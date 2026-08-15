# Vendored SEP-2663 (Tasks extension) — provenance

Two upstream repositories govern this crate, and they are **not interchangeable**:

- `modelcontextprotocol/ext-tasks` carries the **schema** — the wire shapes.
- `modelcontextprotocol/modelcontextprotocol` carries the **SEP prose** — the
  normative rules about *when* those shapes may be sent.

Several rules that this crate must satisfy exist only in the prose and appear
nowhere in the schema (see §Rules that live only in the prose). Vendoring the
schema alone would leave those rules resting on an unpinned web page, so the
SEP document is pinned here too.

| File | Upstream repo | Upstream path | Commit | Content sha256 | Fetched |
|---|---|---|---|---|---|
| `draft-schema.ts` | `modelcontextprotocol/ext-tasks` | `schema/draft/schema.ts` | `8966bea9c4f4e6d71060cc8284a539086e9e234f` | `2203cc75469e32a92a60f4b7b4de949577e25f18fafff69aa92ec06773ab70f6` | 2026-06-09 |
| `draft-schema.json` | `modelcontextprotocol/ext-tasks` | `schema/draft/schema.json` | `8966bea9c4f4e6d71060cc8284a539086e9e234f` | `b17cb4a2534379c214b17770bd5d3d54f69fde16a953bfb542c58235a61274bb` | 2026-06-09 |
| `sep-2663-tasks-extension.md` | `modelcontextprotocol/modelcontextprotocol` | `seps/2663-tasks-extension.md` | `9b44c6b4dcd2451bc49abd39e47eda36b396e8dd` | `2bd75e527a0796ffbc07ed34c47307a43c78de1e3001eada52e601051c09a385` | 2026-08-15 |

- **Extension identifier**: `io.modelcontextprotocol/tasks`
- **SEP**: https://modelcontextprotocol.io/seps/2663-tasks-extension
- **Status upstream**: Experimental / draft

## Why there is no dated release path

Unlike the core protocol crate, which pins an immutable
`schema/2026-07-28/schema.ts`, `modelcontextprotocol/ext-tasks` publishes only
`schema/draft/` and carries no tags at all. A commit pin plus a content
checksum is the strongest provenance the source admits. Wire shapes here can
therefore change without an upstream version bump.

**Schema-pin currency, verified 2026-08-15**: upstream head is `2c1425d`
(2026-07-15). The diff `8966bea..2c1425d` touches `package-lock.json` only, so
the vendored schema is byte-current with upstream head despite the older pin
date. Re-verify with:

```bash
curl -sS "https://api.github.com/repos/modelcontextprotocol/ext-tasks/compare/8966bea9c4f4e6d71060cc8284a539086e9e234f...HEAD" | jq -r '.files[]?.filename'
```

## Rules that live only in the prose

The schema types cannot express *sequencing*, so these are prose-only and are
the reason `sep-2663-tasks-extension.md` is vendored. Line numbers refer to the
pinned copy in this directory.

- **MRTR composition (line 304, restated 936)** — "Server implementations that
  use multi round-trip requests in conjunction with task creation … **SHOULD**
  resolve all MRTR exchanges *synchronously* before responding with a
  `CreateTaskResult`." Note **SHOULD**, and note the antecedent: it binds only
  servers that choose to gather input *before* task creation.
- **Two distinct input mechanisms (line 592ff)** — a server needing input
  *before* returning `CreateTaskResult` uses the MRTR flow on the original
  request; a server needing input *during* execution uses
  `inputRequests`/`inputResponses` over `tasks/get`/`tasks/update`. Both are
  legitimate; they are selected by *when* the input is needed, not by
  preference.
- **Separate state across the two phases (line 940)** — the MRTR phase ends
  when the server returns any non-`"input_required"` `resultType`, at which
  point its `inputRequests` keys are consumed. The task phase begins at
  `CreateTaskResult` with its *own* keys. MRTR keys MUST NOT be carried into
  the task's `inputRequests`.
- **Durability before response (line 302)** — a server MUST NOT return
  `CreateTaskResult` until a `tasks/get` for that `taskId` would resolve.

## Naming drift to be aware of

SEP-2663 commit `451f5e1e4` (2026-05-01, "Allow IncompleteResult before task
creation") introduced the composition section using `IncompleteResult` and
`resultType: "incomplete"`. Those names were **renamed** before the 2026-07-28
release and the pinned copy above already says `InputRequiredResult` /
`"input_required"`, which is what the released core `schema.ts` and this
workspace's `ResultType::InputRequired` use. Do not "correct" the code toward
the older wording found in that commit's diff or in stale mirrors.

## Re-pin procedure

1. Fetch the same paths at the newer commit(s) into this directory.
2. Update every cell in the table above — commit, checksum and fetch date.
3. Re-read §Rules that live only in the prose against the new document and fix
   the line numbers; a rule that changed is an ADR-level event, not a doc edit.
4. `./scripts/check-schema-pin.sh` — reads the checksums from this table.
5. `cargo test -p turul-mcp-ext-tasks` — `src/v2026_07_28/compliance_test.rs`
   asserts wire shapes against these files' type definitions.
