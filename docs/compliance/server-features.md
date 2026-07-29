# Server Features — MCP 2026-07-28

Column meanings and interop values: see [README.md](README.md). Interop columns
are `turul | python | typescript | go`; `—` means not exercised, never "pass".

Test paths are relative to the repo root. `c/` abbreviates `crates/`.

---

## 1. `server/discover`

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| Answers without a session, replacing `initialize` | MUST | Implemented | `c/turul-mcp-server/src/server.rs:1336` | `discover_stateless_2026.rs::server_discover_answers_without_a_session` | pass | pass | pass | pass |
| `serverInfo` rides in `_meta`, never as a top-level field | MUST | Implemented | `server.rs:1371` stamps it once at dispatch | `discover_stateless_2026.rs::server_discover_answers_without_a_session` (asserts the raw body has no bare `"serverInfo":`) | pass | pass | pass | pass |
| Capabilities reflect only wired features | MUST | Implemented | `server.rs:1336-1386` | `discover_stateless_2026.rs::discover_advertises_registered_feature_capabilities`, `::discover_advertises_the_prompts_capability_with_truthful_list_changed`, `subscriptions_listen_2026.rs::resources_subscribe_capability_is_advertised_truthfully` | pass | — | — | — |
| Result is cacheable (`ttlMs`/`cacheScope`) | MUST | Implemented | `discover.rs` | `discover.rs::discover_result_round_trips` (unit); wire-asserted by `scripts/interop-fastmcp.sh` | pass | **pass** | pass | pass |
| `instructions` reaches the wire when set | MAY | Implemented | `discover.rs:127`, wired at `server.rs:1376` | `e2e_2026_real_server.rs::progress_feed_and_discovered_server_accessors` — builder `.instructions(…)` → `server/discover` → `McpClient::server_instructions()`, asserted verbatim; unit backstop `discover.rs::discover_result_serializes_instructions_when_present` | pass | — | — | — |

This row was briefly recorded as a TypeScript disagreement. It was measured
against `v2.0.0-beta.1`, whose `DiscoverResultSchema` still required a top-level
`serverInfo`; the released npm build `2.0.0` accepts identity in `_meta` and
drives the row green. The lesson is in
[the interop matrix](../plans/interop-test-matrix.md) §3 — a pinned pre-release
peer is a claim about the outside world that goes stale silently.

## 2. Tools

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| `tools/list` is deterministic across connections | MUST | Implemented | `server.rs` `ListToolsHandler` | `wire_edges_2026.rs::tools_list_is_deterministic_paginated_and_cacheable`, `::tools_list_is_invariant_across_independent_connections`, `::tools_list_is_unchanged_by_an_intervening_unrelated_request` | pass | pass | pass | pass |
| `inputSchema` supports full JSON Schema 2020-12 (`oneOf`, `$defs`, `$ref`) — SEP-2106 | MUST | Implemented | `c/turul-mcp-protocol-2026-07-28/src/tools.rs:189` | `schema_fidelity_2026.rs::tools_list_carries_the_full_2020_12_schema` | pass | — | — | — |
| `outputSchema` unrestricted | MUST | Implemented | `tools.rs:192` | `schema_fidelity_2026.rs::structured_content_matches_the_advertised_output_schema` | pass | — | — | — |
| `structuredContent` present whenever `outputSchema` is declared | MUST | Implemented | `handlers/mod.rs` tool dispatch | `schema_fidelity_2026.rs::structured_content_matches_the_advertised_output_schema` | pass | — | — | — |
| `tools/call` returns `content[]`, optional `isError`, optional `_meta` | MUST | Implemented | `tools.rs` `CallToolResult` | `discover_stateless_2026.rs::tools_call_result_carries_server_info_meta` | pass | pass | pass | pass |
| Unknown tool → `-32602`, not a fabricated success | MUST | Implemented | `server.rs` `ToolCallHandler` | `wire_edges_2026.rs::unknown_tool_is_invalid_params_on_the_wire` | pass | — | — | — |
| `tools/list` pagination; invalid cursor → `-32602` | MUST | Implemented | `server.rs` `ListToolsHandler` | `wire_edges_2026.rs::tools_list_is_deterministic_paginated_and_cacheable` | pass | — | — | — |
| `tools/list` is cacheable | MUST | Implemented | `tools.rs:313` | `wire_edges_2026.rs::tools_list_is_deterministic_paginated_and_cacheable` | pass | **pass** | pass | pass |
| `notifications/tools/list_changed` only when a dynamic source exists | SHOULD | Implemented | `c/turul-mcp-server/src/tool_registry.rs:192-203` | `tool_registry.rs` in-crate tests | pass | — | — | — |
| `x-mcp-header` mirrors an argument into an HTTP header | MAY | Implemented | see Base Protocol §6 | `mcp_param_2026.rs::*` | pass | — | — | — |
| Tool `annotations` (readOnly/destructive/idempotent/openWorld) reach the wire | MAY | **Implemented, untested** | `tools.rs:195` | **NOT FOUND** | — | — | — | — |
| Domain failure returns `isError: true` rather than a JSON-RPC error | MUST | Implemented | `tools.rs` `CallToolResult::error` | `error_mapping_2026.rs::tool_domain_failure_is_is_error_not_a_json_rpc_error` — `isError: true` with **no** `error` member, asserted back to back with the unknown-tool `-32602` contrast on the same server | pass | — | — | — |

`Tool.execution` / task support is **not** a 2026-07-28 field; the 2026 `Tool`
interface has no `execution?:`. Async tools live in the Tasks extension — see
[extensions.md](extensions.md).

## 3. Resources

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| `resources/list` returns stable, absolute URIs | MUST | Implemented | `handlers/mod.rs:817` | `discover_stateless_2026.rs::resources_list_dispatches_statelessly_with_cacheable_result` | pass | pass | pass | pass |
| `resources/read` returns `contents[]` with uri/mimeType/text-or-blob | MUST | Implemented | `handlers/mod.rs:918-1141` | `mrtr_2026.rs::mrtr_round_trip_on_resources_read` | pass | pass | pass | pass |
| Missing resource → `-32602` (HTTP 200, error in body) | MUST | Implemented | `handlers/mod.rs` | `wire_edges_2026.rs::nonexistent_resource_is_invalid_params_on_the_wire`; status pinned by `scripts/interop-fastmcp.sh` J5 | pass | **pass** | pass | pass |
| Non-base64 blob from a provider is an error, not a silent payload | MUST | Implemented | `handlers/mod.rs` | `wire_edges_2026.rs::invalid_base64_blob_is_rejected` | pass | — | — | — |
| `resources/templates/list` answers even with no templates registered | MUST | **Implemented — fixed this slice** | `builder.rs` now registers the handler unconditionally | `discover_stateless_2026.rs::resources_templates_list_answers_empty_rather_than_method_not_found` | pass | **pass** | pass | pass |
| Resource links carry `size`/`icons`/`annotations` without duplicate wire keys | MUST | Implemented | `content.rs` `ResourceReference` | `content.rs::test_resource_link_round_trips_size_and_icons`, `::test_resource_reference_serialization_with_annotations_and_meta` | pass | — | — | — |
| `resources.subscribe` advertised truthfully | MUST | Implemented | `server.rs` `DiscoverHandler` | `subscriptions_listen_2026.rs::resources_subscribe_capability_is_advertised_truthfully` | pass | — | — | — |
| `resources/list` and `resources/read` cacheable | MUST | Implemented | `resources.rs` | `discover_stateless_2026.rs::resources_list_dispatches_statelessly_with_cacheable_result`; `resources/read` wire-asserted by `scripts/interop-fastmcp.sh` | pass | **pass** | pass | pass |
| `resources/read` reports the same `mimeType` `resources/list` advertised | MUST | Implemented | `ResourceContent::with_mime_type` (`c/turul-mcp-protocol-2026-07-28/src/resources.rs:449`) lets a provider set a content mime type other than `ResourceContent::text`'s `text/plain` default | `resource_mime_type_2026.rs::read_reports_the_mime_type_that_list_advertises` — every URI in `resources/list` is read back and compared, across a `text/markdown` override and a `application/json` body | pass | — | — | — |
| `resources/list` and `resources/templates/list` pagination | SHOULD | Implemented | `handlers/mod.rs:817` | `list_pagination_2026.rs::resources_list_paginates_and_rejects_an_invalid_cursor`, `::resource_templates_list_paginates_and_rejects_an_invalid_cursor`, `::the_two_resource_listings_do_not_overlap` | pass | — | — | — |

**Fixed this slice: `resources/read` mislabelled every text body `text/plain`.**
`ResourceContent::text()` hardcodes `text/plain` and offered no way to set
another, so a resource declaring `text/markdown` in `resources/list` reported
`text/plain` on read — one property of one resource, two contradictory answers,
and no way for a client to tell which is authoritative. The interop fixture
server carried the mismatch as a documented "known discrepancy"; it is now a
single `FIXTURE_MIME` constant feeding both sides. `with_mime_type` is a builder
method on a concrete spec type implementing the schema's existing optional
`mimeType` field — no new contract. The handler deliberately does **not**
back-fill the resource-level declaration into content that omits one: a content
item may legitimately carry a different type from the resource that lists it
(embedded sub-resources), so filling it in would be a guess.

**Fixed in an earlier slice.** `resources/templates/list` was registered only when
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
| `prompts/list` returns descriptors with title/icons/`_meta` | MUST | Implemented | `handlers/mod.rs:529` | `wire_edges_2026.rs::prompt_descriptors_and_error_codes` | pass | pass | pass | pass |
| `prompts/get` returns `messages[]`; unknown prompt → `-32602` | MUST | Implemented | `handlers/mod.rs:670` | `wire_edges_2026.rs::prompt_descriptors_and_error_codes`, `mrtr_2026.rs::mrtr_round_trip_on_prompts_get` | pass | pass | pass | pass |
| `Mcp-Name` on `prompts/get` must equal `params.name` | MUST | Implemented | `streamable_http.rs:1380` | `wire_edges_2026.rs::prompt_descriptors_and_error_codes` | pass | pass | pass | pass |
| `prompts/list` cacheable | MUST | Implemented | `prompts.rs` | `discover_stateless_2026.rs::prompts_list_dispatches_statelessly_with_cacheable_result` | pass | **pass** | pass | pass |
| `notifications/prompts/list_changed` advertised only when dynamic | SHOULD | Implemented | `builder.rs:228` | `discover_stateless_2026.rs::discover_advertises_the_prompts_capability_with_truthful_list_changed` | pass | — | — | — |
| `arguments` is `map<string,string>` and substitutes into the render | MUST | Implemented | `prompts.rs:240` | `wire_edges_2026.rs::prompts_get_substitutes_the_supplied_argument` — the argument is advertised in `prompts/list`, substituted in `prompts/get`, and omitting it yields the provider default rather than an unresolved placeholder | pass | **pass** | pass | pass |
| `prompts/list` pagination | SHOULD | Implemented | `handlers/mod.rs:529` | `list_pagination_2026.rs::prompts_list_paginates_and_rejects_an_invalid_cursor` | pass | — | — | — |

## 5. Completion

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| `completion/complete` routes to registered providers | MUST | Implemented | `handlers/mod.rs:329` | `discover_stateless_2026.rs::completion_complete_routes_to_registered_provider` | pass | pass | pass | pass |
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

1. **Tool `annotations` are never asserted on the `tools/list` wire.**
   `readOnlyHint`/`destructiveHint`/`idempotentHint`/`openWorldHint` are bound at
   `tools.rs:195` and nothing checks they survive to a listing.
2. **`SubscriptionsListenResult` teardown is unimplemented** — no shutdown
   signal exists in the transport to emit it from. Spec-legal (SHOULD), already
   logged as a known gap in the crate's own COMPLIANCE.md.
3. **`completion/complete` `context.arguments` is unexercised on the server
   side.** The client now drives it
   (`completion_and_cancellation_2026.rs::complete_passes_the_context_arguments_through`
   asserts the provider receives it), but no server-lane test asserts the
   server's own handling in isolation. Narrower than it was, not closed.
4. **`resources/templates/list` is covered for listing and pagination only** —
   no test reads a templated URI back through `resources/read`.

**Closed this slice:** `tools/call` domain-failure `isError` wire test; cursor
walks for `resources/list`, `resources/templates/list` and `prompts/list`;
`instructions` end-to-end; prompt argument substitution on the 2026 lane; and
the `resources/list` ↔ `resources/read` `mimeType` disagreement (a behaviour
fix, not just coverage).
