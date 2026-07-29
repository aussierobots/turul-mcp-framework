# Server Features — MCP 2026-07-28

Column meanings and interop values: see [README.md](README.md). Interop columns
are `turul | python | typescript | go`; `—` means not exercised, never "pass".

Test paths are relative to the repo root. `c/` abbreviates `crates/`.

---

## 1. `server/discover`

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| Answers without a session, replacing `initialize` | MUST | Implemented | `c/turul-mcp-server/src/server.rs:1336` | `discover_stateless_2026.rs::server_discover_answers_without_a_session` | pass | pass | — | pass |
| `serverInfo` rides in `_meta`, never as a top-level field | MUST | Implemented | `server.rs:1371` stamps it once at dispatch | `discover_stateless_2026.rs::server_discover_answers_without_a_session` (asserts the raw body has no bare `"serverInfo":`) | pass | pass | **fail — peer is stale** | pass |
| Capabilities reflect only wired features | MUST | Implemented | `server.rs:1336-1386` | `discover_stateless_2026.rs::discover_advertises_registered_feature_capabilities`, `::discover_advertises_the_prompts_capability_with_truthful_list_changed`, `subscriptions_listen_2026.rs::resources_subscribe_capability_is_advertised_truthfully` | pass | — | — | — |
| Result is cacheable (`ttlMs`/`cacheScope`) | MUST | Implemented | `discover.rs` | `discover.rs::discover_result_round_trips` (unit); wire-asserted by `scripts/interop-fastmcp.sh` | pass | **pass** | — | pass |
| `instructions` reaches the wire when set | MAY | **Partial** | `discover.rs:127`, wired at `server.rs:1376` | `discover.rs::discover_result_serializes_instructions_when_present` — **serialization only**, no server e2e | — | — | — | — |

**The TypeScript SDK v2.0.0-beta.1 disagrees here, and it is wrong.** Its
`DiscoverResultSchema` still requires a top-level `serverInfo`; the released
schema removed it, and `DiscoverResult` in the pinned artifact declares only
`supportedVersions`, `capabilities` and `instructions`. The SDK's classifier
reads the failed parse as "not a modern server" and falls back to `initialize`,
which a 2026-only server rejects — so one stale field costs the whole
connection. Recorded as a peer defect; the server is not being loosened for it.

## 2. Tools

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| `tools/list` is deterministic across connections | MUST | Implemented | `server.rs` `ListToolsHandler` | `wire_edges_2026.rs::tools_list_is_deterministic_paginated_and_cacheable`, `::tools_list_is_invariant_across_independent_connections`, `::tools_list_is_unchanged_by_an_intervening_unrelated_request` | pass | pass | — | pass |
| `inputSchema` supports full JSON Schema 2020-12 (`oneOf`, `$defs`, `$ref`) — SEP-2106 | MUST | Implemented | `c/turul-mcp-protocol-2026-07-28/src/tools.rs:189` | `schema_fidelity_2026.rs::tools_list_carries_the_full_2020_12_schema` | pass | — | — | — |
| `outputSchema` unrestricted | MUST | Implemented | `tools.rs:192` | `schema_fidelity_2026.rs::structured_content_matches_the_advertised_output_schema` | pass | — | — | — |
| `structuredContent` present whenever `outputSchema` is declared | MUST | Implemented | `handlers/mod.rs` tool dispatch | `schema_fidelity_2026.rs::structured_content_matches_the_advertised_output_schema` | pass | — | — | — |
| `tools/call` returns `content[]`, optional `isError`, optional `_meta` | MUST | Implemented | `tools.rs` `CallToolResult` | `discover_stateless_2026.rs::tools_call_result_carries_server_info_meta` | pass | pass | — | pass |
| Unknown tool → `-32602`, not a fabricated success | MUST | Implemented | `server.rs` `ToolCallHandler` | `wire_edges_2026.rs::unknown_tool_is_invalid_params_on_the_wire` | pass | — | — | — |
| `tools/list` pagination; invalid cursor → `-32602` | MUST | Implemented | `server.rs` `ListToolsHandler` | `wire_edges_2026.rs::tools_list_is_deterministic_paginated_and_cacheable` | pass | — | — | — |
| `tools/list` is cacheable | MUST | Implemented | `tools.rs:313` | `wire_edges_2026.rs::tools_list_is_deterministic_paginated_and_cacheable` | pass | **pass** | — | pass |
| `notifications/tools/list_changed` only when a dynamic source exists | SHOULD | Implemented | `c/turul-mcp-server/src/tool_registry.rs:192-203` | `tool_registry.rs` in-crate tests | pass | — | — | — |
| `x-mcp-header` mirrors an argument into an HTTP header | MAY | Implemented | see Base Protocol §6 | `mcp_param_2026.rs::*` | pass | — | — | — |
| Tool `annotations` (readOnly/destructive/idempotent/openWorld) reach the wire | MAY | **Implemented, untested** | `tools.rs:195` | **NOT FOUND** | — | — | — | — |
| Domain failure returns `isError: true` rather than a JSON-RPC error | MUST | **Implemented, untested** | `tools.rs` `CallToolResult::error` | **NOT FOUND** — only the unknown-tool protocol-error path is asserted | — | — | — | — |

`Tool.execution` / task support is **not** a 2026-07-28 field; the 2026 `Tool`
interface has no `execution?:`. Async tools live in the Tasks extension — see
[extensions.md](extensions.md).

## 3. Resources

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| `resources/list` returns stable, absolute URIs | MUST | Implemented | `handlers/mod.rs:817` | `discover_stateless_2026.rs::resources_list_dispatches_statelessly_with_cacheable_result` | pass | pass | — | pass |
| `resources/read` returns `contents[]` with uri/mimeType/text-or-blob | MUST | Implemented | `handlers/mod.rs:918-1141` | `mrtr_2026.rs::mrtr_round_trip_on_resources_read` | pass | pass | — | pass |
| Missing resource → `-32602` (HTTP 200, error in body) | MUST | Implemented | `handlers/mod.rs` | `wire_edges_2026.rs::nonexistent_resource_is_invalid_params_on_the_wire`; status pinned by `scripts/interop-fastmcp.sh` J5 | pass | **pass** | — | pass |
| Non-base64 blob from a provider is an error, not a silent payload | MUST | Implemented | `handlers/mod.rs` | `wire_edges_2026.rs::invalid_base64_blob_is_rejected` | pass | — | — | — |
| `resources/templates/list` answers even with no templates registered | MUST | **Implemented — fixed this slice** | `builder.rs` now registers the handler unconditionally | `discover_stateless_2026.rs::resources_templates_list_answers_empty_rather_than_method_not_found` | pass | **pass** | — | pass |
| Resource links carry `size`/`icons`/`annotations` without duplicate wire keys | MUST | Implemented | `content.rs` `ResourceReference` | `content.rs::test_resource_link_round_trips_size_and_icons`, `::test_resource_reference_serialization_with_annotations_and_meta` | pass | — | — | — |
| `resources.subscribe` advertised truthfully | MUST | Implemented | `server.rs` `DiscoverHandler` | `subscriptions_listen_2026.rs::resources_subscribe_capability_is_advertised_truthfully` | pass | — | — | — |
| `resources/list` and `resources/read` cacheable | MUST | Implemented | `resources.rs` | `discover_stateless_2026.rs::resources_list_dispatches_statelessly_with_cacheable_result`; `resources/read` wire-asserted by `scripts/interop-fastmcp.sh` | pass | **pass** | — | pass |
| `resources/list` pagination | SHOULD | **Partial** | `handlers/mod.rs:817` | **NOT FOUND** — no cursor walk | — | — | — | — |

**Fixed this slice.** `resources/templates/list` was registered only when
template resources existed, so a server declaring the resources capability
answered `-32601` for a core method — telling a client the method does not
exist, which is a different claim from "there are none", and the one a
capability-driven client acts on by abandoning templates entirely. Found by
pointing the new interop fixture server at a foreign client. Registration is now
unconditional and `build()` swaps in the populated handler; revert-and-fail
verified.

## 4. Prompts

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| `prompts/list` returns descriptors with title/icons/`_meta` | MUST | Implemented | `handlers/mod.rs:529` | `wire_edges_2026.rs::prompt_descriptors_and_error_codes` | pass | pass | — | pass |
| `prompts/get` returns `messages[]`; unknown prompt → `-32602` | MUST | Implemented | `handlers/mod.rs:670` | `wire_edges_2026.rs::prompt_descriptors_and_error_codes`, `mrtr_2026.rs::mrtr_round_trip_on_prompts_get` | pass | pass | — | pass |
| `Mcp-Name` on `prompts/get` must equal `params.name` | MUST | Implemented | `streamable_http.rs:1380` | `wire_edges_2026.rs::prompt_descriptors_and_error_codes` | pass | pass | — | pass |
| `prompts/list` cacheable | MUST | Implemented | `prompts.rs` | `discover_stateless_2026.rs::prompts_list_dispatches_statelessly_with_cacheable_result` | pass | **pass** | — | pass |
| `notifications/prompts/list_changed` advertised only when dynamic | SHOULD | Implemented | `builder.rs:228` | `discover_stateless_2026.rs::discover_advertises_the_prompts_capability_with_truthful_list_changed` | pass | — | — | — |
| `arguments` is `map<string,string>` and substitutes into the render | MUST | **Implemented, untested** | `prompts.rs:240` | **NOT FOUND** in the 2026 lane — exercised only by `scripts/interop-fastmcp.sh` and `scripts/interop-turul-client.sh`, which render `greeting(name="Ada")` | — | **pass** | — | pass |
| `prompts/list` pagination | SHOULD | **Partial** | `handlers/mod.rs:529` | **NOT FOUND** — no cursor walk | — | — | — | — |

## 5. Completion

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| `completion/complete` routes to registered providers | MUST | Implemented | `handlers/mod.rs:329` | `discover_stateless_2026.rs::completion_complete_routes_to_registered_provider` | pass | pass | — | pass |
| Unsupported (no providers) → `-32601` rather than an empty success | SHOULD | Implemented | `handlers/mod.rs` | `wire_edges_2026.rs::completion_unsupported_is_method_not_found` | pass | — | — | — |
| Values capped at 100, with `hasMore`/`total` reflecting truncation | MUST | Implemented | `completion.rs` | `discover_stateless_2026.rs::completion_values_are_capped_at_100` | pass | — | — | — |
| Malformed params → `-32602` | MUST | Implemented | `handlers/mod.rs` | `discover_stateless_2026.rs::malformed_completion_params_are_rejected_with_32602` | pass | — | — | — |
| `context.arguments` (previously-resolved variables) accepted | MAY | **Implemented, untested** | `completion.rs:102` | **NOT FOUND** | — | — | — | — |

## 6. Subscriptions

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| `subscriptions/listen` opens an SSE stream | MUST | Implemented | `streamable_http.rs` `handle_subscriptions_listen` | `subscriptions_listen_2026.rs::listen_acks_first_then_delivers_only_requested_types` | pass | — | — | — |
| Acknowledgement is the first frame, honoring only the requested subset | MUST | Implemented | `subscriptions.rs` | `subscriptions_listen_2026.rs::listen_acks_first_then_delivers_only_requested_types`, `::listen_ack_omits_unsupported_types` | pass | — | — | — |
| Every stream notification carries `subscriptionId` in `_meta` | MUST | Implemented | `meta.rs` `META_KEY_SUBSCRIPTION_ID` | `subscriptions_listen_2026.rs::listen_acks_first_then_delivers_only_requested_types` | pass | — | — | — |
| Only requested types and URIs are delivered | MUST | Implemented | `c/turul-http-mcp-server/src/stream_manager.rs` | `subscriptions_listen_2026.rs::listen_acks_first_then_delivers_only_requested_types` | pass | — | — | — |
| Requires `Accept: text/event-stream` | MUST | Implemented | `streamable_http.rs:1896` | `subscriptions_listen_2026.rs::listen_requires_sse_accept` | pass | — | — | — |
| Concurrent subscriptions are isolated; one dropping leaves others delivering | MUST | Implemented | `stream_manager.rs` | `subscriptions_listen_2026.rs::concurrent_subscriptions_receive_their_own_subsets`, `::dropping_one_subscription_leaves_others_delivering` | pass | — | — | — |
| Stream frames are labelled (`event:`, and `id:` on delivered events) | SHOULD | Implemented | `streamable_http.rs:2019,2084` | `streaming_e2e_2026.rs::listen_frames_are_labelled_and_delivered_events_carry_a_cursor` | pass | — | — | — |
| `SubscriptionsListenResult` graceful teardown on server shutdown | SHOULD | **Not implemented** | type bound only | NOT FOUND — `turul-http-mcp-server` has no shutdown-signal path to emit it from | — | — | — | — |

The acknowledgement frame carries `event: message` but **no** `id:`; only
delivered notifications carry a cursor. That asymmetry is what the test pins,
rather than an invented uniformity requirement.

## 7. Logging — deprecated (SEP-2577)

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| `logging/setLevel`, the `logging` capability and message notifications are deprecated | — | Deprecated-by-spec | `#[deprecated(since = "0.4.0")]` on the affected types | — | n/a | n/a | n/a | n/a |
| Per-request `_meta.logLevel` replaces `logging/setLevel` | MUST | Implemented | `meta.rs` `RequestMetaObject.log_level` | `log_gating_2026.rs::message_notifications_require_a_declared_log_level` | pass | — | — | — |
| The declared level is a severity threshold | MUST | Implemented | `handlers/mod.rs` | `log_gating_2026.rs::declared_level_is_the_severity_threshold` | pass | — | — | — |
| Unrecognised level → `-32602` | SHOULD | Implemented | `handlers/mod.rs` | `log_gating_2026.rs::unrecognized_log_level_is_rejected_with_32602` | pass | — | — | — |
| A server emitting `notifications/message` declares the `logging` capability | MUST | Implemented | `server.rs` `DiscoverHandler` | `wire_edges_2026.rs::discover_declares_the_logging_capability` | pass | — | — | — |

Annotation-only this revision. Earliest removal is 2027-07-28 per the
deprecation window.

---

## Gap register

1. **`tools/call` domain failure (`isError: true`) has no wire test.** Only the
   unknown-tool protocol-error path is asserted, and the two are exactly what a
   client must tell apart.
2. **Tool `annotations` are never asserted on the `tools/list` wire.**
3. **Pagination cursor walks exist for `tools/list` only** — `resources/list`,
   `resources/templates/list` and `prompts/list` have none.
4. **`SubscriptionsListenResult` teardown is unimplemented** — no shutdown
   signal exists in the transport to emit it from. Spec-legal (SHOULD), already
   logged as a known gap in the crate's own COMPLIANCE.md.
5. **`instructions` is never asserted end-to-end** from builder to
   `server/discover` response.
6. **`completion/complete` `context.arguments` is bound but unexercised.**
7. **Prompt argument substitution has no 2026-lane test** — the only evidence is
   the two interop probes.
