# Extensions — MCP 2026-07-28

Column meanings and interop values: see [README.md](README.md). Interop columns
are `turul | python | typescript | go`; `—` means not exercised, never "pass".

Test paths are relative to the repo root. `c/` abbreviates `crates/`.

Extensions are **off by default** in this framework: `ext-tasks` is a non-default
Cargo feature, and `turul-mcp-ext-apps` is not wired into any crate. That matches
the spec's rule that SDKs must not enable an extension unless the operator opted
in, and since this slice it is asserted at runtime as well as guaranteed
structurally — a default build is checked to advertise no `extensions` map and to
answer 404 for every `tasks/*` method.

---

## 1. The `extensions` capability map

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| Server advertises extensions in `server/discover` capabilities | MUST | Implemented | `c/turul-mcp-server/src/builder.rs:1925` inserts the tasks identifier | `ext_tasks_2026.rs::discover_advertises_the_tasks_extension` | pass | — | — | — |
| Client declares extension support in `_meta.clientCapabilities.extensions` | MUST | Implemented | `c/turul-mcp-client/src/protocol/v2026_07_28.rs:42-47` | `ext_tasks_e2e_2026.rs::undeclared_client_gets_synchronous_outcome` (declared/undeclared behaviour) | pass | — | — | — |
| Extension identifiers validated at the negotiation boundary (SEP-2133) | SHOULD | Implemented | `c/turul-mcp-ext-tasks/src/v2026_07_28/capability.rs:47-60` | `capability.rs` in-crate tests | pass | — | — | — |
| Extensions disabled unless opted in | MUST | Implemented | `ext-tasks` is a non-default Cargo feature | `discover_stateless_2026.rs::a_default_build_advertises_no_extensions` — `capabilities.extensions` absent on a default build, and `tasks/get`/`update`/`cancel` answer 404 + `-32601`; the positive counterpart is `ext_tasks_2026.rs::discover_advertises_the_tasks_extension` under `--features ext-tasks` | pass | — | — | — |
| A third-party extension can be added without touching the protocol crate | SHOULD | Implemented (by construction) | the protocol crate's only extension surface is an opaque `HashMap<String, Value>` | — | — | — | — | — |

## 2. Tasks (SEP-2663) — `io.modelcontextprotocol/tasks`

Tasks moved out of core into an extension in 2026-07-28. The frozen
`turul-mcp-protocol-2025-11-25` correctly keeps them in core.

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| Methods are `tasks/get`, `tasks/update`, `tasks/cancel` — no `tasks/list` | MUST | Implemented | `c/turul-mcp-ext-tasks/src/v2026_07_28/lifecycle.rs`; dispatch at `builder.rs:1899-1926`; client at `client.rs:1438-1468` | `ext_tasks_e2e_2026.rs::task_outcome_polls_to_completion`, `::task_cancel_reaches_cancelled`, `::task_update_resumes_an_input_required_task` | pass | — | — | — |
| The server alone decides materialisation: synchronous unless the client declared *and* the tool requires | MUST | Implemented | `c/turul-mcp-server/src/ext_tasks.rs:425-438` | `ext_tasks_2026.rs::declared_call_returns_task_and_polls_to_completion`, `::undeclared_call_runs_synchronously` | pass | — | — | — |
| A required task tool called without the declared extension → `-32021` with `data.requiredCapabilities.extensions` | MUST | Implemented | `ext_tasks.rs:425-431` | `ext_tasks_2026.rs::required_tool_without_extension_is_32021` | pass | — | — | — |
| Task IDs are unguessable (UUIDv4, not the house UUIDv7) because they act as bearer tokens | MUST | Implemented | `c/turul-mcp-ext-tasks/src/v2026_07_28/store.rs` | `ext_tasks_2026.rs::task_ids_are_unguessable_uuids` | pass | — | — | — |
| Unknown task id → invalid params | MUST | Implemented | `lifecycle.rs` | `ext_tasks_2026.rs::unknown_task_id_is_invalid_params` | pass | — | — | — |
| Cancel flips working → cancelled and acknowledges the terminal state | MUST | Implemented | `lifecycle.rs` | `ext_tasks_2026.rs::cancel_flips_working_to_cancelled_and_acks_terminal` | pass | — | — | — |
| A task-augmented tool returning `InputRequired` parks in `input_required`; `tasks/update` resumes it | MUST | Implemented | `lifecycle.rs`, `store.rs` | `ext_tasks_e2e_2026.rs::task_update_resumes_an_input_required_task` | pass | — | — | — |
| `notifications/tasks` ride `subscriptions/listen`, filtered by `taskIds` | SHOULD | Implemented | transport filter | `ext_tasks_2026.rs::task_notifications_ride_listen_filtered_by_task_id` | pass | — | — | — |
| Exactly one encoded `Mcp-Name` header on a task-augmented call | MUST | Implemented | client transport | `ext_tasks_e2e_2026.rs::call_tool_or_task_emits_exactly_one_encoded_mcp_name_header` | pass | — | — | — |

Tasks is the best-tested area in the framework by ratio of asserting tests to
requirements — and the least externally verified: **no peer has exercised it.**
FastMCP's client does not drive the extension, so every green cell above is
turul-on-turul.

## 3. MCP Apps (SEP-1865) — `io.modelcontextprotocol/ui`

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| Tool `_meta.ui` → `UiToolMeta`; UI-resource `_meta.ui` → `UiResourceMeta` | MUST | **Not implemented** (types only) | `c/turul-mcp-ext-apps/src/v2026_07_28/types.rs` | `compliance_test.rs` — wire shapes only, crate-internal | — | — | — | — |
| Client `mimeTypes` capability including `text/html;profile=mcp-app` | MUST | **Not implemented** | `capability.rs` | crate-internal only | — | — | — | — |
| Host↔view iframe protocol (`ui/*` over postMessage) | MUST | **Out of scope** | belongs to app/host SDKs, not a server framework | n/a | n/a | n/a | n/a | n/a |

**`turul-mcp-ext-apps` is unwired, and now says so.** A repo-wide grep finds the
crate referenced only by its own files, the workspace member list and
documentation — zero references from `turul-mcp-server` or `turul-mcp-client`. It
defines and self-tests the wire shapes and nothing consumes them: no server-side
capability advertisement, no client-side `_meta.ui` handling; `tools/list` does
not hide non-`model` tools and `tools/call` does not reject app-only calls, both
Apps-spec MUSTs. The crate's `README.md` now states this in its own words, lists
the five things a consumer must build, and notes that `ClientCapabilities` has to
be read from `_meta["io.modelcontextprotocol/clientCapabilities"]` because
2026-07-28 is stateless and there is no handshake to read them from. The
disclosure does not make the extension usable — it stops someone adding the
dependency expecting it to be.

**The vendored spec was the wrong artifact and is now the right one.** The crate
shipped `apps-draft.mdx`, a byte-exact copy of upstream `specification/draft/apps.mdx`
at `ca1d2989` — proven by re-fetching that path at that commit and hashing it,
not inferred from a size difference. It is replaced by `apps-2026-01-26.mdx` from
the released dated path `specification/2026-01-26/apps.mdx` at
`298e884ec3f02daba085acdb02042d73bd00b355` (tag `v1.0.0`), the commit that created
the file and which upstream has never modified since — so the pin is immutable by
construction, not merely by SHA. `spec.types.ts` moved to `v1.7.5` and is recorded
as a **convenience reference, not authority**; where it and the dated `.mdx`
disagree, the `.mdx` wins. All six bound interfaces were diffed between the two
commits: identical property sets and types, JSDoc prose only.

**The Rust types were correct; one doc claim was not.** `UiResourceMeta` asserted
that "when present on both, the content-item value wins; hosts MUST check both
locations." That MUST exists only in the *draft*'s "Metadata Location" section —
the released 2026-01-26 spec has no such section. The invented normativity is
removed. `scripts/check-schema-pin.sh` now covers this crate and rejects any
`*.mdx` row whose upstream source is not a dated `specification/<YYYY-MM-DD>/`
path, so an honestly-recorded pin at a floating path fails the gate.

---

## Gap register

1. **`turul-mcp-ext-apps` is not wired into the server or the client.** It is a
   types-only crate. That is now disclosed in its README rather than discovered,
   but the extension still cannot be used through the framework: wiring it is a
   design decision nobody has taken.
2. **No peer has exercised the Tasks extension.** Its coverage is thorough and
   entirely self-referential. Three peers now drive the core surface — the Go
   SDK, the TypeScript SDK and FastMCP — and none of them touches the extension,
   so the harness to close this already exists.
3. **The Tasks extension's `taskId` is a bearer token with no owner binding.**
   Recorded in full in [base-protocol.md](base-protocol.md) §12; named here
   because it is an extension defect, not a core one — `tasks/get`/`update`/`cancel`
   take no principal, so possession of the handle is the entire access check.
4. **`spec.types.ts` and `apps-2026-01-26.mdx` are pinned to different upstream
   commits** (`v1.7.5` and `v1.0.0`). The core crate's rule is one immutable
   commit for both artifacts. The split is deliberate and argued — upstream
   publishes no dated `spec.types.ts`, and the bound field sets are identical at
   both commits — but it is a deviation from the house rule, and it is reversible
   by re-fetching `src/spec.types.ts@298e884` and updating one table row.

**Closed this slice:** the floating-draft misvendoring (re-pinned to the released
dated path, with a gate that now catches the class of defect rather than the
instance) and "extensions off unless opted in" being structural — it is asserted
now, in both directions.
