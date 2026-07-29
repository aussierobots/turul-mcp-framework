# Extensions — MCP 2026-07-28

Column meanings and interop values: see [README.md](README.md). Interop columns
are `turul | python | typescript | go`; `—` means not exercised, never "pass".

Test paths are relative to the repo root. `c/` abbreviates `crates/`.

Extensions are **off by default** in this framework: `ext-tasks` is a non-default
Cargo feature, and `turul-mcp-ext-apps` is not wired into any crate. That matches
the spec's rule that SDKs must not enable an extension unless the operator opted
in — but it is a structural guarantee (the code is not compiled) rather than a
runtime-asserted one.

---

## 1. The `extensions` capability map

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| Server advertises extensions in `server/discover` capabilities | MUST | Implemented | `c/turul-mcp-server/src/builder.rs:1925` inserts the tasks identifier | `ext_tasks_2026.rs::discover_advertises_the_tasks_extension` | pass | — | — | — |
| Client declares extension support in `_meta.clientCapabilities.extensions` | MUST | Implemented | `c/turul-mcp-client/src/protocol/v2026_07_28.rs:42-47` | `ext_tasks_e2e_2026.rs::undeclared_client_gets_synchronous_outcome` (declared/undeclared behaviour) | pass | — | — | — |
| Extension identifiers validated at the negotiation boundary (SEP-2133) | SHOULD | Implemented | `c/turul-mcp-ext-tasks/src/v2026_07_28/capability.rs:47-60` | `capability.rs` in-crate tests | pass | — | — | — |
| Extensions disabled unless opted in | MUST | Implemented (structural) | `ext-tasks` is a non-default Cargo feature | — | — | — | — | — |
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

**`turul-mcp-ext-apps` is unwired.** A repo-wide grep finds the crate referenced
only by its own files and the workspace member list — zero references from
`turul-mcp-server` or `turul-mcp-client`. It defines and self-tests the wire
shapes and nothing consumes them: no server-side capability advertisement, no
client-side `_meta.ui` handling. It is a published crate that cannot currently be
used through the framework.

---

## Gap register

1. **`turul-mcp-ext-apps` is not wired into the server or the client.** Either
   wire it or state plainly in its README that it is a types-only crate, so
   nobody adds the dependency expecting the extension to work.
2. **No peer has exercised the Tasks extension.** Its coverage is thorough and
   entirely self-referential. The Go SDK v1.7.0 is the most promising peer for
   closing this, being the only stable 2026-07-28 implementation.
3. **"Extensions off unless opted in" is structural, not asserted.** Nothing
   fails if a future change starts advertising an extension by default.
