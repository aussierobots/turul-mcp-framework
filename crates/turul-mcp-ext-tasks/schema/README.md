# Vendored Tasks extension (`io.modelcontextprotocol/tasks`) — provenance

The normative artifacts are the **released 2026-07-28** files of
`modelcontextprotocol/ext-tasks`: its dated `specification/2026-07-28/tasks.md`
and `schema/2026-07-28/schema.{ts,json}`. Upstream marks the dated directories
immutable; the schema files are pinned at the commit that created them
(unmodified since), `tasks.md` at its latest change (`899c1bbc`, an
example-only fix: two `tasks/get` examples now show `resultType: "complete"`).

The SEP-2663 document from `modelcontextprotocol/modelcontextprotocol` stays
pinned for two sequencing rules that `tasks.md` does not restate (see
§Rules that live only in the prose). Where the two disagree, `tasks.md` wins.

| File | Upstream repo | Upstream path | Commit | Content sha256 | Fetched |
|---|---|---|---|---|---|
| `tasks.md` | `modelcontextprotocol/ext-tasks` | `specification/2026-07-28/tasks.md` | `899c1bbc869567670093face3591b39c28ce3219` | `5fc22b9b5ff248703927c69855f751a061c2d6170f4dc3923204452f859f91a5` | 2026-11-14 |
| `schema.ts` | `modelcontextprotocol/ext-tasks` | `schema/2026-07-28/schema.ts` | `0d0a6bd4c258b35caa3c810a1dd506cf105b1501` | `35a4ae593e60410716196773b5b63e8cb4677ddb49edc1afb8bbb43753dda4e7` | 2026-11-14 |
| `schema.json` | `modelcontextprotocol/ext-tasks` | `schema/2026-07-28/schema.json` | `0d0a6bd4c258b35caa3c810a1dd506cf105b1501` | `bf30afb7ac251e3e22c037b7a685f60ef6603031b5484c0d08b1fa0bbe86d460` | 2026-11-14 |
| `sep-2663-tasks-extension.md` | `modelcontextprotocol/modelcontextprotocol` | `seps/2663-tasks-extension.md` | `9b44c6b4dcd2451bc49abd39e47eda36b396e8dd` | `2bd75e527a0796ffbc07ed34c47307a43c78de1e3001eada52e601051c09a385` | 2026-08-15 |

- **Extension identifier**: `io.modelcontextprotocol/tasks`
- **Stage upstream**: Beta, headed to Core ([extensions overview](https://modelcontextprotocol.io/extensions/overview))
- `schema.ts` imports its core types from upstream's `spec.types.ts`, which is
  the core 2026-07-28 schema this workspace already pins in
  `turul-mcp-protocol-2026-07-28`; it is not vendored a second time.

## What changed from the draft pin (`8966bea9`)

The released schema differs from the earlier `schema/draft/` copy in four ways;
the bindings already matched each one, so no `pub` type changed:

- `InputRequests` / `InputResponses` come from the core schema instead of a
  local union.
- `FailedTask.error` is a JSON-RPC error object (`code`, `message`, `data?`)
  rather than an open map. The server fills it from `McpError::to_error_object`.
- Every result carries a fixed `resultType`: `"task"` on `CreateTaskResult`,
  `"complete"` on `tasks/get`, `tasks/update` and `tasks/cancel` results.
- Interfaces became intersection types; the wire shape is unchanged.

## Rules that live only in the prose

The schema types cannot express *sequencing*. Two such rules are in `tasks.md`;
two more appear only in the SEP, which is why it stays vendored.

In `tasks.md`:

- **Durability before response (line 300)**: a server MUST NOT return
  `CreateTaskResult` until a `tasks/get` for that `taskId` would resolve.
- **MRTR composition (line 302)**: servers combining MRTR with task creation
  **SHOULD** resolve all MRTR exchanges *synchronously* before responding with
  a `CreateTaskResult`. It binds only servers that gather input *before* task
  creation.

Only in `sep-2663-tasks-extension.md`:

- **Two distinct input mechanisms (line 592ff)**: input needed *before*
  `CreateTaskResult` uses the MRTR flow on the original request; input needed
  *during* execution uses `inputRequests`/`inputResponses` over
  `tasks/get`/`tasks/update`. They are selected by *when* the input is needed.
- **Separate state across the two phases (line 940)**: the MRTR phase ends when
  the server returns any non-`"input_required"` `resultType`, consuming its
  `inputRequests` keys. The task phase begins at `CreateTaskResult` with its
  *own* keys; MRTR keys MUST NOT be carried into the task's `inputRequests`.

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
