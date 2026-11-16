# Extensions — MCP 2026-07-28

Column meanings and interop values: see [README.md](README.md). Interop columns
are `turul | python | typescript | go`; `—` means not exercised, never "pass".

Test paths are relative to the repo root. `c/` abbreviates `crates/`.

Extensions are **off by default** in this framework: `ext-tasks` is a non-default
Cargo feature, and MCP Apps is advertised only by a server that calls
`extension_capability` itself. That matches
the spec's rule that SDKs must not enable an extension unless the operator opted
in, and since this slice it is asserted at runtime as well as guaranteed
structurally — a default build is checked to advertise no `extensions` map and to
answer 404 for every `tasks/*` method.

**Supported extensions** (the list the extensions overview asks SDKs to publish):

| Extension | Identifier | turul surface | Off by default via |
|---|---|---|---|
| Tasks | `io.modelcontextprotocol/tasks` | `turul-mcp-ext-tasks`; server feature `ext-tasks` (`with_ext_tasks`); client feature `ext-tasks` | Cargo feature |
| MCP Apps | `io.modelcontextprotocol/ui` | `turul-mcp-ext-apps` helpers over the server's generic hooks; client feature `ext-apps` | the server calls `extension_capability` itself |
| Skills | `io.modelcontextprotocol/skills` | `turul-mcp-ext-skills`; server feature `ext-skills` (`with_ext_skills`); client feature `ext-skills` | Cargo feature |
| Server Cards (**experimental** upstream) | none (no negotiation, SEP-2127) | `turul-mcp-ext-server-card`; server and Lambda feature `ext-server-card` (`with_server_card`) | Cargo feature |

Not implemented: OAuth Client Credentials, Enterprise-Managed Authorization.

---

## 1. The `extensions` capability map

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| Server advertises extensions in `server/discover` capabilities | MUST | Implemented | `c/turul-mcp-server/src/builder.rs:1925` inserts the tasks identifier | `ext_tasks_2026.rs::discover_advertises_the_tasks_extension` | pass | — | — | — |
| Client declares extension support in `_meta.clientCapabilities.extensions` | MUST | Implemented | `c/turul-mcp-client/src/protocol/v2026_07_28.rs:42-47` | `ext_tasks_e2e_2026.rs::undeclared_client_gets_synchronous_outcome` (declared/undeclared behaviour) | pass | — | — | — |
| Extension identifiers follow SEP-2133 naming | SHOULD | **Not validated, by decision** | each extension crate owns its identifier as a constant (`EXTENSION_IDENTIFIER`); `extension_capability` accepts any key the operator names (ADR-028 §4) | — | — | — | — | — |
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
| Task IDs are unguessable (UUIDv4, not the house UUIDv7) because they act as bearer tokens | MUST | Implemented | `c/turul-mcp-server/src/ext_tasks.rs` (`Uuid::new_v4`) | `ext_tasks_2026.rs::task_ids_are_unguessable_uuids` | pass | — | — | — |
| Unknown task id → invalid params | MUST | Implemented | `lifecycle.rs` | `ext_tasks_2026.rs::unknown_task_id_is_invalid_params` | pass | — | — | — |
| Cancel flips working → cancelled and acknowledges the terminal state | MUST | Implemented | `lifecycle.rs` | `ext_tasks_2026.rs::cancel_flips_working_to_cancelled_and_acks_terminal` | pass | — | — | — |
| A task-augmented tool returning `InputRequired` parks in `input_required`; `tasks/update` resumes it | MUST | Implemented | `lifecycle.rs`, `traits.rs` | `ext_tasks_e2e_2026.rs::task_update_resumes_an_input_required_task` | pass | — | — | — |
| `notifications/tasks` ride `subscriptions/listen`, filtered by `taskIds` | SHOULD | Implemented | transport filter | `ext_tasks_2026.rs::task_notifications_ride_listen_filtered_by_task_id` | pass | — | — | — |
| `taskIds` requested by a client that did not declare the extension → `-32021` with `data.requiredCapabilities.extensions` | MUST | Implemented | `c/turul-http-mcp-server/src/streamable_http.rs` (`handle_subscriptions_listen`) via `turul_mcp_ext_tasks::missing_capability_error` | `ext_tasks_2026.rs::listen_task_ids_without_declaration_is_32021` | pass | — | — | — |
| Client sets `Mcp-Name` = `params.taskId` on `tasks/get`, `tasks/update`, `tasks/cancel` | MUST | Implemented | `c/turul-mcp-client/src/transport/http.rs` (`apply_request_metadata_headers`) | `transport::http::tests::task_methods_set_mcp_name_to_task_id` | pass | — | — | — |
| Wire types validate against the released `schema/2026-07-28/schema.json` | MUST | Implemented | `c/turul-mcp-ext-tasks/schema/` pinned at `0d0a6bd4` / `899c1bbc` | `released_schema_test.rs` (every result and status) | pass | — | — | — |
| Exactly one encoded `Mcp-Name` header on a task-augmented call | MUST | Implemented | client transport | `ext_tasks_e2e_2026.rs::call_tool_or_task_emits_exactly_one_encoded_mcp_name_header` | pass | — | — | — |
| `notifications/progress` and `notifications/message` never ride a task's `subscriptions/listen` stream | MUST NOT | Implemented | transport filter | `ext_tasks_2026.rs::progress_and_message_never_ride_a_task_listen_stream` (the tool provably emits both) | pass | — | — | — |

Tasks is the best-tested area in the framework by ratio of asserting tests to
requirements — and the least externally verified: **no peer has exercised it.**
FastMCP's client does not drive the extension, so every green cell above is
turul-on-turul.

## 3. MCP Apps (SEP-1865) — `io.modelcontextprotocol/ui`

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| Server declares `io.modelcontextprotocol/ui: {}` | convention (core schema: `{}` = support, no settings) | **Implemented** (generic hook) | `extension_capability` + `ext_apps::capability()` | `c/turul-mcp-server/tests/ext_apps_2026.rs::adr028_server_declares_ui_extension_via_extension_capability` | ✅ | — | — | — |
| Tool `_meta.ui` → `UiToolMeta` | — | **Implemented** (generic hook) | `HasToolMeta` / `ToolBuilder::meta` + `UiToolMeta::to_meta` | `ext_apps_2026.rs::apps_visibility_is_a_host_duty_server_lists_app_only_tools_with_ui_meta` | ✅ | — | — | — |
| UI resource: `ui://` URI, `text/html;profile=mcp-app`, `_meta.ui` on the read content item | MUST | **Implemented** (generic hook) | `McpResource::read` + `UiResourceMeta::to_meta` | `ext_apps_2026.rs::apps_ui_resource_reads_with_mcp_app_mime_and_content_item_ui_meta` | ✅ | — | — | — |
| Tools return meaningful `content` with or without UI; server SHOULD check client capability | MUST / SHOULD | **Implemented** (in the call) | `SessionContext::client_capabilities` + `client_supports_html_views` | `ext_apps_2026.rs::apps_graceful_degradation_tool_returns_text_content_with_or_without_ui_capability` | ✅ | — | — | — |
| Hide non-`model` tools from the agent; reject app `tools/call` for non-`app` tools | MUST (host) | **Out of scope** | Apps spec §Visibility assigns both to the host; the server lists every tool | n/a | n/a | n/a | n/a | n/a |
| Client `mimeTypes` capability including `text/html;profile=mcp-app` | MUST (host) | **Implemented** (opt-in `ext-apps` feature) | `DeclaredCapabilities::ext_apps` → `clientCapabilities.extensions` on every 2026 request | `c/turul-mcp-client/tests/ext_apps_client_2026.rs::apps_client_declares_ui_extension_with_mime_types_only_when_opted_in` | ✅ | — | — | — |
| Host reads `_meta.ui` and splits app-only tools from model-visible ones | MUST (host) | **Helpers only** | `turul_mcp_client::ext_apps::{tool_ui_meta, resource_ui_meta, partition_tools}`; enforcing the split is the host application's job | `ext_apps_client_2026.rs::apps_visibility_split_separates_app_only_tools_from_model_visible_ones`, `::apps_resource_ui_meta_is_read_from_the_read_content_item` | ✅ | — | — | — |
| Host↔view iframe protocol (`ui/*` over postMessage) | MUST | **Out of scope** | belongs to app/host SDKs, not a server framework | n/a | n/a | n/a | n/a | n/a |

**Served through generic hooks, with no server feature.** `turul-mcp-server` has
no Apps code: a server advertises the extension with `extension_capability`,
attaches `_meta.ui` through `HasToolMeta` or `ToolBuilder::meta`, serves the view
from an ordinary `McpResource`, and degrades inside the tool by reading the
per-request client capabilities. `examples/ext-apps-server` does this, and its
`server/discover`, `tools/list`, `resources/read` and `tools/call` responses were
read back live with curl. Every ✅ above is turul-on-turul: no peer SDK and no
upstream conformance scenario exercises Apps.

**UI tools are always listed.** The Apps spec's SHOULD (§Server Behavior: check
client capabilities "before registering UI-enabled tools") assumes a
registration-time handshake; 2026-07-28 has none, and varying `tools/list` by
caller would contradict its `ttlMs` / `cacheScope`. A host without Apps support
ignores `_meta.ui`, and the tool's text `content` covers it.

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

## 4. Skills (SEP-2640) — `io.modelcontextprotocol/skills`

Governing ADR: ADR-033. Skill parsing is owned by `turul-agent-skills`;
`turul-mcp-ext-skills` holds the wire types and `turul-mcp-server` the handlers.

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| Declare `capabilities.resources` and the skills extension in `server/discover` | MUST | Implemented | `with_ext_skills` forces `resources` | `c/turul-mcp-server/tests/ext_skills_2026.rs::s_g1_discover_advertises_skills_and_resources`, `::s_g1_empty_catalog_still_declares_resources` | pass | — | — | — |
| Extension off unless opted in | MUST | Implemented | non-default `ext-skills` feature | `ext_skills_2026.rs::default_build_advertises_nothing` | pass | — | — | — |
| `skills/list` paginates without splitting an entry | MUST | Implemented | server handler | `ext_skills_2026.rs::s_g2_list_paginates_atomic_entries` | pass | — | — | — |
| `skills/get` by URI; unknown URI → `-32602` | MUST | Implemented | server handler | `ext_skills_2026.rs::s_g2_get_by_uri_and_unknown_is_invalid_params` | pass | — | — | — |
| `resources` manifest digest and size match the bytes `resources/read` serves | MUST | Implemented | catalog built from on-disk bytes | `ext_skills_2026.rs::s_g6_read_bytes_match_manifest` | pass | — | — | — |
| Frontmatter served as typed JSON where YAML 1.1 and 1.2 agree; disagreement refuses the skill | MUST | Implemented | `turul-agent-skills` 0.1.4 resolver | `turul-agent-skills` tests; client `ext_skills_client_2026.rs::c5_typed_frontmatter_verifies`, `::c5_schema_disagreement_fails_closed` | pass | — | — | — |
| Nested skills publish flat | MUST | Implemented | catalog | `ext_skills_2026.rs::nested_skill_publishes_flat` | pass | — | — | — |
| A manifest URI the resource blocklist would refuse is refused at build, never advertised | MUST | Implemented | build-time check (ADR-033) | `ext_skills_2026.rs::q10_blocklisted_manifest_uri_refused_at_build` | pass | — | — | — |
| `resources/directory/read` when `directoryRead: true` | MUST if declared | Implemented | server handler | `ext_skills_2026.rs::s_g7_*` (6 tests) | pass | — | — | — |
| Generated skills (`resources: "dynamic"`) | MAY | Implemented | `SkillCatalog::add_dynamic` | `ext_skills_2026.rs::dynamic_*` (6 tests) | pass | — | — | — |
| Host verifies size, digest and frontmatter; reads only manifest URIs; no prefetch | MUST (host) | Implemented (client `ext-skills`) | `turul-mcp-client` | `c/turul-mcp-client/tests/ext_skills_client_2026.rs::c1_*`…`c7_*` | pass | — | — | — |
| Served on Lambda | — | Implemented | `LambdaMcpServerBuilder` over `McpServerBuilder` (ADR-035) | `scripts/e2e-lambda-skills.sh` | pass | — | — | — |

`scripts/ext-skills-conformance.sh` runs the upstream host scenarios against
`examples/ext-skills-server`, the only check here not written by turul.

## 5. Server Cards (SEP-2127) — experimental, no identifier

Governing ADR: ADR-038. Upstream `ext-server-card` marks this experimental and
not an accepted extension; the pin is `526201b`.

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| Card conforms to the v1 schema | MUST | Implemented | `ServerCard::validate` | `v1::tests::every_valid_example_passes_both_and_round_trips`, `::every_invalid_example_fails_both`, `::validate_agrees_with_the_schema_on_each_constraint` (against the vendored `schema.json`) | pass | — | — | — |
| Served at `<streamable-http-url>/server-card` as `application/mcp-server-card+json` | MAY / SHOULD | Implemented | `with_server_card` route | `server_card_2026.rs::get_serves_the_card_with_media_type_cors_and_caching` | pass | — | — | — |
| CORS: `*`, `GET`, `Content-Type, If-None-Match`, expose `ETag` | MUST | Implemented | route sets them; transports keep a route's own CORS | `server_card_2026.rs::route_cors_survives_the_servers_own_cors`, `::head_options_and_other_methods`; Lambda `a_route_that_sets_cors_keeps_it_and_answers_its_preflight` | pass | — | — | — |
| `Cache-Control: public, max-age=3600`, `ETag`, 304 on `If-None-Match` | SHOULD | Implemented | `ServerCardDocument` + route | `server_card_2026.rs::matching_if_none_match_is_304` | pass | — | — | — |
| Identity, version, descriptive fields and protocol versions do not contradict `server/discover` | SHOULD NOT | Implemented (build error) | `ext_server_card::reconcile` | `server_card_2026.rs::card_fields_equal_server_discover`, `::a_card_contradicting_server_discover_fails_build` | pass | — | — | — |
| No credentials or private endpoints in a card | MUST NOT | Caller's duty | documented in the crate README | — | — | — | — | — |
| HTTPS in production; rate limiting | MUST / SHOULD | Deployment's duty | TLS terminates in front; middleware | — | — | — | — | — |

`examples/server-card-server` was read back live with curl: `GET`, `If-None-Match`
→ 304, `OPTIONS` → 204, `POST` → 405, and the card's version, title and protocol
versions equal its `server/discover`.

## Gap register

1. **No Apps host exists in this repo.** The client can declare Apps and read
   `_meta.ui` (opt-in `ext-apps` feature) but renders nothing, so every Apps ✅ in
   §3 is turul-on-turul. Upstream `basic-host` is the only independent host.
2. **No peer has exercised the Tasks extension.** Its coverage is thorough and
   entirely self-referential. Three peers now drive the core surface — the Go
   SDK, the TypeScript SDK and FastMCP — and none of them touches the extension,
   so the harness to close this already exists.
3. **`spec.types.ts` and `apps-2026-01-26.mdx` are pinned to different upstream
   commits** (`v1.7.5` and `v1.0.0`). The core crate's rule is one immutable
   commit for both artifacts. The split is deliberate and argued — upstream
   publishes no dated `spec.types.ts`, and the bound field sets are identical at
   both commits — but it is a deviation from the house rule, and it is reversible
   by re-fetching `src/spec.types.ts@298e884` and updating one table row.

**Closed this slice:** the floating-draft misvendoring (re-pinned to the released
dated path, with a gate that now catches the class of defect rather than the
instance) and "extensions off unless opted in" being structural — it is asserted
now, in both directions.
