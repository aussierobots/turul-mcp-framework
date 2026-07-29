# Base Protocol — MCP 2026-07-28

Column meanings and interop values: see [README.md](README.md). Interop columns
are `turul | python | typescript | go`; `—` means not exercised, never "pass".

Test paths are relative to the repo root. `c/` abbreviates `crates/`.

---

## 1. JSON-RPC message shape

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| `jsonrpc: "2.0"` on every message | MUST | Implemented | `c/turul-mcp-protocol-2026-07-28/src/json_rpc.rs:29` | `discover.rs::discover_result_response_wire_shape` | pass | pass | pass | pass |
| `RequestId` is string or number; bare `null` rejected | MUST | Implemented | schema `RequestId` | `wire_edges_2026.rs::null_request_id_is_rejected` | pass | — | — | — |
| Batch (JSON array) bodies rejected — batching removed in 2026-07-28 | MUST | Implemented | `c/turul-http-mcp-server/src/streamable_http.rs:1135` (`handle_post_streamable_http`) calls only the singular parser | `wire_edges_2026.rs::a_json_array_body_is_rejected_as_a_batch` | pass | — | — | — |
| `params._meta` is required, not optional | MUST | Implemented | `json_rpc.rs:36-46` | `json_rpc.rs::test_request_params_rejects_missing_meta` | pass | — | — | — |

Batch rejection is now asserted rather than inferred: a two-element array body
answers `-32600`, no element executes, and the response is one message rather
than an array of them.

**Status inconsistency the batch test pinned, now resolved.** A body the
transport cannot parse as a JSON-RPC message (malformed JSON, or a batch
array) now answers **HTTP 400**, the same status as the adjacent null-id check
(`streamable_http.rs:1161-1184`) for the same class of envelope violation, and
consistent with §11's "the transport rejects before dispatch → 4xx." The spec
does not pin an HTTP status to a parse failure specifically — [Streamable
HTTP's backward-compatibility
guidance](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/streamable-http#backward-compatibility)
uses 400 as the general status a modern server returns for a request it
rejects before dispatch, and that same page's client-detection heuristic
depends on 400 (not 200) marking a rejected request — so 400 was chosen for
consistency with the rest of the transport's pre-dispatch rejections rather
than pinned by explicit spec text.

## 2. Lifecycle — the stateless core

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| `initialize` / `notifications/initialized` are removed | MUST | Implemented | `c/turul-mcp-server/src/server.rs:494` gates registration to the 2025 lane | `wire_edges_2026.rs::initialize_error_names_supported_versions` | pass | pass | pass | pass |
| `Mcp-Session-Id` never minted or echoed | MUST | Implemented | `streamable_http.rs:65` | `stateless_2026_http_surface.rs::responses_never_mint_session_ids`, `::inbound_mcp_session_id_is_ignored_and_never_echoed` | pass | pass | pass | pass |
| GET and DELETE on the endpoint answer 405 | MUST | Implemented | `streamable_http.rs:453-467` | `stateless_2026_http_surface.rs::get_returns_405_method_not_allowed`, `::delete_returns_405_method_not_allowed`, `::get_with_last_event_id_returns_405` | pass | — | — | — |
| `server/discover` is the bootstrap method | MUST | Implemented | `server.rs:1328-1386` | `discover_stateless_2026.rs::server_discover_answers_without_a_session` | pass | pass | pass | pass |
| Lambda transport enforces the same stateless contract | Parity | Implemented | `c/turul-mcp-aws-lambda/src/handler.rs` | `scripts/e2e-lambda-local.sh` (10 assertions through the real Runtime API) | pass | n/a | n/a | n/a |

An earlier revision recorded a "known residue" here: the `notifications/initialized`
string literals in `server.rs:836,891` sitting behind `cfg!()` rather than
`#[cfg]`. That was a mislabelled non-issue. `cfg!()` expands to a literal `false`
on the 2026 lane and const-folds, so the guard is not a runtime check and the
comparison is not evaluated; handler *registration* (`server.rs:495`) is behind a
real `#[cfg(feature = "protocol-2025-11-25")]`, so no 2026 build serves the
method. Removed from the register rather than carried forward.

## 3. Transports

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| POST answers `application/json` when the request opted into nothing | MUST | Implemented | `streamable_http.rs:780,898` | `streaming_e2e_2026.rs::json_replies_are_a_single_object_with_no_event_framing` | pass | — | — | — |
| POST answers `text/event-stream` when the request declared `progressToken` | MUST | Implemented | `streamable_http.rs:965,1033` | `progress_2026.rs::combined_accept_uses_json_without_a_token_and_sse_with_one` | pass | — | — | — |
| SSE bodies are well-formed event-stream grammar | MUST | Implemented | `streamable_http.rs:2407` | `streaming_e2e_2026.rs::sse_body_matches_the_event_stream_grammar` | pass | — | — | — |
| SSE responses declare an unbuffered stream (no Content-Length, `no-cache`) | SHOULD | Implemented | `streamable_http.rs:2502-2511` | `streaming_e2e_2026.rs::sse_response_headers_declare_an_unbuffered_stream` | pass | — | — | — |
| The result frame ends the stream | MUST | Implemented | `streamable_http.rs` dispatch | `streaming_e2e_2026.rs::the_result_frame_is_last_and_closes_the_stream` | pass | — | — | — |
| Origin absent → allowed; loopback → allowed; same-host → allowed | MUST | Implemented | `c/turul-http-mcp-server/src/origin.rs:82-146` | `origin_validation_2026.rs::origin_absent_is_allowed`, `::loopback_origin_is_allowed_by_default`, `::same_host_origin_is_allowed_by_default` | pass | — | — | — |
| Cross-origin → 403 before body parsing or auth | MUST | Implemented | `streamable_http.rs:439-451` | `origin_validation_2026.rs::cross_origin_is_rejected_with_403_by_default` | pass | — | — | — |
| OPTIONS preflight exempt; the following real request is still gated | MUST | Implemented | `streamable_http.rs:427-433` | `origin_validation_2026.rs::options_preflight_is_exempt_but_actual_request_is_gated` | pass | — | — | — |
| `.well-known/*` exempt from Origin validation | MUST | Implemented | `server.rs:581` dispatches before the transport | `oauth_2026.rs::hostile_origin_does_not_block_the_well_known_metadata` | pass | — | — | — |
| stdio transport | MAY | **Not implemented** | — | — | — | — | — | — |

The `.well-known` row is now driven from both sides in one test: an
`Origin: http://attacker.example` still gets 200 from both RFC 9728 metadata
routes while the MCP endpoint on the same server answers 403 for the same
header. Asserting only the exemption would have passed on a server with Origin
validation switched off entirely.

An earlier revision recorded a defect here: `turul-mcp-client` declared Cargo
features `stdio` and `all-transports = ["http","sse","stdio"]` with no stdio
module behind them. Both are deleted from the manifest. The row above stays
`Not implemented` because stdio genuinely is not — the difference is that the
crate no longer claims otherwise.

FastMCP's `pass` on the two framing rows is notable evidence, not a formality:
it negotiated into **SSE framing for eight of nine requests** and JSON for the
first, and parsed both — see the wire capture in `scripts/interop-fastmcp.sh`.
The "client MUST support both framings" rule is confirmed by a client we did not
write.

## 4. Versioning

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| `MCP-Protocol-Version` required on every POST | MUST | Implemented | `streamable_http.rs:1309-1362` | `mcp_headers_2026.rs::missing_protocol_version_header_is_rejected` | pass | pass | pass | pass |
| Header disagreeing with `_meta.protocolVersion` → 400 + `-32020` | MUST | Implemented | `streamable_http.rs:1309-1362` | `discover_stateless_2026.rs::header_body_protocol_version_mismatch_is_rejected_with_32020` | pass | — | — | — |
| Unsupported version → 400 + `-32022`, naming supported versions | MUST | Implemented | `streamable_http.rs:1328-1359` | `discover_stateless_2026.rs::unsupported_protocol_version_header_is_rejected_with_32022` | pass | pass | pass | pass |
| Unrecognised version is never silently downgraded | MUST | Implemented | `c/turul-http-mcp-server/src/server.rs` | `mcp_headers_2026.rs::headerless_initialize_rejection_names_supported_versions` | pass | — | — | — |
| Lambda shares the same negotiation logic | Parity | Implemented | `c/turul-mcp-aws-lambda` builds `StreamableHttpHandler` | `lambda_2026_07_28_wire_compliance.rs::unsupported_protocol_version_returns_recognized_modern_error` | pass | n/a | n/a | n/a |

## 5. Authorization

The framework is an OAuth 2.1 **resource server**. Several 2026 auth SEPs bind
the client or the authorization server; those are marked `Out-of-role` rather
than counted as gaps, because implementing them here would be wrong.

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| JWKS signature validation, alg pinned, `alg:none` rejected | MUST | Implemented | `c/turul-mcp-oauth/src/jwt.rs:124` | `jwt.rs::test_alg_none_rejected`, `::test_valid_jwt_accepted` | pass | — | — | — |
| Expired token rejected | MUST | Implemented | `jwt.rs:157,171` | `jwt.rs::test_expired_jwt_rejected_401` | pass | — | — | — |
| Audience validated always, no opt-out | MUST | Implemented | `jwt.rs:93,163-167` | `jwt.rs::test_audience_always_validated`, `::test_wrong_audience_rejected` | pass | — | — | — |
| Token `iss` validated when an issuer is configured | SHOULD | Implemented | `jwt.rs:159-161` | `jwt.rs::test_wrong_issuer_rejected` | pass | — | — | — |
| Single-AS issuer policy, no silent fallback | SHOULD | Implemented | `c/turul-mcp-oauth/src/lib.rs:100-126` | `lib.rs::test_single_as_ok`, `::test_multiple_as_rejected` | pass | — | — | — |
| RFC 9728 Protected Resource Metadata, root and path form | MUST | Implemented | `well_known.rs:30-44`, `metadata.rs:185-198` | `oauth_2026.rs::protected_resource_metadata_is_served_on_well_known_routes`, `well_known.rs::test_path_form_endpoint_returns_same_metadata` | pass | — | — | — |
| Missing bearer → 401 + `WWW-Authenticate` | MUST | Implemented | `middleware.rs` | `oauth_2026.rs::missing_bearer_gets_401_with_www_authenticate_challenge` | pass | — | — | — |
| Malformed Authorization header → 400 `invalid_request` | MUST | Implemented | `middleware.rs:389` | `middleware.rs::malformed_authorization_returns_400_invalid_request` | pass | — | — | — |
| Insufficient scope → 403 `insufficient_scope` | SHOULD | Implemented | `middleware.rs` | `middleware.rs::insufficient_scope_returns_403_challenge` | pass | — | — | — |
| 401 outranks `_meta` validation 400 | SHOULD | Implemented | `streamable_http.rs:1309` | `oauth_2026.rs::auth_401_outranks_meta_validation_400` | pass | — | — | — |
| `Cache-Control: no-store` on auth challenges | SHOULD | Implemented | `build_http_challenge_response` (`streamable_http.rs:2907-2929`) sets it on every challenge it builds, reached from the pre-session middleware branch at `streamable_http.rs:1219` | `oauth_2026.rs::challenges_are_not_cacheable` | pass | — | — | — |
| TLS enforced on JWKS / issuer URIs | SHOULD | **Unknown** | no scheme check in `JwtValidator::new` | NOT FOUND | — | — | — | — |
| RFC 9207 `iss` in the authorization response (SEP-2468) | MUST | **Out-of-role** | absent by design — an RS never handles the authorization response | n/a | n/a | n/a | n/a | n/a |
| OIDC `application_type` on DCR (SEP-837) | MUST | **Out-of-role** | MUST is to specify `application_type` at all during DCR; SHOULD applies only to the `native`/`web` choice. Binds MCP clients performing DCR; `oauth/src/lib.rs:24-27` states this crate never implements a DCR surface | n/a | n/a | n/a | n/a | n/a |
| Refresh Tokens — RS half (SEP-2207) | SHOULD | **Partial** | RS (Protected Resource) half implemented: `c/turul-mcp-oauth/src/metadata.rs:112-136` filters `offline_access` out of advertised scopes / `WWW-Authenticate`, per "MCP Servers (Protected Resources) SHOULD NOT include offline_access in WWW-Authenticate scope or Protected Resource Metadata scopes_supported." Client half (advertising `refresh_token` in `grant_types`, requesting `offline_access`) is out-of-role — this crate is RS-only | `metadata.rs::offline_access_is_filtered_from_scopes` | pass | — | — | — |
| Scope accumulation across incremental auth (SEP-2350) | SHOULD | **Out-of-role** | AS/client concern | n/a | n/a | n/a | n/a | n/a |

The `no-store` row was `Unknown` / "claimed by ADR-021, not located in code"
until this slice. The header was always emitted; nothing had looked for it. The
test covers the three statuses the challenge builder actually reaches on the
2026 path — 401 missing bearer, 401 `invalid_token`, 400 `invalid_request`. The
403 `insufficient_scope` challenge shares the same builder, so it carries the
header by construction, but no wire test drives it; that is a narrower gap than
the one it replaces and is recorded as such rather than claimed green.

**Two auth-failure paths, not one, by design.** `turul-mcp-oauth`'s bearer-token
rejections (missing/invalid/expired token, insufficient scope) construct
`MiddlewareError::HttpChallenge` (`c/turul-mcp-oauth/src/middleware.rs`), which
the transport short-circuits into a raw HTTP 401/403 + `WWW-Authenticate`
response before JSON-RPC dispatch — an `unreachable!()` guard in
`map_middleware_error_to_jsonrpc` (`c/turul-http-mcp-server/src/middleware/error.rs`)
enforces that this variant never reaches the JSON-RPC error path. Separately,
`MiddlewareError::Unauthenticated` is a general-purpose "auth required" signal
any custom (non-OAuth) middleware can return; it maps to `-32001` inside a 200
response (`session_handler.rs:1325`, `streamable_http.rs:2877`), the same "well-formed
request that fails inside processing → 200 with the error in the JSON-RPC body"
rule §11 documents for every other handler-level failure. The two are not the
same failure wearing different HTTP clothes: an OAuth Bearer challenge is an
HTTP-native mechanism RFC 6750/9728 mandates status codes and headers for;
generic middleware authentication is a JSON-RPC domain error like any other
`McpError` variant, with no such mandate. A custom middleware that wants
OAuth-grade HTTP semantics returns `HttpChallenge`, not `Unauthenticated` — the
frozen `-32001` allocation is not a gap to close.

**Documentation gap:** ADR-021 mentions RFC 9207 but the governing 2026 ADR
(ADR-027) does not discuss any of the six auth-hardening SEPs that AGENTS.md
headlines in the Branch Lock (`AGENTS.md:227`). CLAUDE.md's Branch Lock section
names the same area only as "RFC 9207 auth," with no SEP numbers. The *code*
posture is defensible; the ADR is silent on its own stated scope.
`grep -rl 9207 crates/` returns nothing — 9207 appears only in `docs/`.

## 6. Request metadata headers (SEP-2243)

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| `Mcp-Method` required on every request and notification, must match the body | MUST | Implemented | `streamable_http.rs:1364-1372` | `mcp_headers_2026.rs::missing_mcp_method_header_is_rejected`, `::mismatched_mcp_method_header_is_rejected`, `::notifications_also_require_mcp_method` | pass | pass | pass | pass |
| `Mcp-Name` required where the method names a target, must match | MUST | Implemented | `streamable_http.rs:1374-1418` | `mcp_headers_2026.rs::missing_mcp_name_on_tools_call_is_rejected`, `::resources_read_requires_uri_as_mcp_name`, `::methods_without_name_need_no_mcp_name` | pass | pass | pass | pass |
| Base64 sentinel decoding for non-ASCII `Mcp-Name` | MUST | Implemented | `headers.rs:305-357` | `mcp_headers_2026.rs::base64_encoded_mcp_name_decodes_and_matches`, `::base64_encoded_mcp_name_mismatch_is_rejected` | pass | — | — | — |
| `x-mcp-header` annotations discovered and enforced as `Mcp-Param-*` | MUST | Implemented | `headers.rs:109-180`, `server.rs:1899-1949` | `mcp_param_2026.rs::matching_param_header_passes_validation`, `::omitted_param_header_with_body_value_is_rejected`, `::mismatched_param_header_is_rejected`, `::integer_params_compare_numerically` | pass | — | — | — |
| Misplaced `x-mcp-header` excluded from `tools/list` | MUST | Implemented | detector `headers.rs:195-303`; exclusion in the client at `c/turul-mcp-client/src/protocol/v2026_07_28.rs:200-223` (`tool_is_admissible`, applied at `:242` before the remap) | `bilingual_2026_operations.rs::misplaced_x_mcp_header_under_items_excludes_the_tool_from_tools_list`, `verify_x_mcp_header_placement_client_2026.rs::prefix_items_excluded`, `::pattern_properties_excluded`, `::defs_referenced_excluded` (+9 more applicators, 5 precision keep-cases) | pass | — | — | — |

## 7. `_meta` general fields

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| `protocolVersion` required in request `_meta` | MUST | Implemented | `meta.rs:403-409` | `wire_edges_2026.rs::incomplete_meta_branches_are_rejected` | pass | pass | pass | pass |
| `clientCapabilities` required in request `_meta` | MUST | Implemented | `meta.rs:416-422` | `wire_edges_2026.rs::incomplete_meta_branches_are_rejected` | pass | pass | pass | pass |
| `clientInfo` is OPTIONAL | MUST | Implemented | `meta.rs:411-414` | `discover_stateless_2026.rs::request_without_client_info_is_served`, `wire_edges_2026.rs::absent_client_info_is_not_an_incomplete_meta` | pass | — | — | — |
| A present-but-malformed `clientInfo` is rejected | MUST | Implemented | `handlers/mod.rs` | `discover_stateless_2026.rs::malformed_client_info_is_rejected` | pass | — | — | — |
| `serverInfo` stamped on results, in `_meta`, never top-level | SHOULD | Implemented | `meta.rs:272-281`, stamped at `server.rs:1371` | `meta.rs::valid_server_info_round_trips_through_the_wire`, `progress_2026.rs::sse_result_frame_carries_server_info_meta` | pass | pass | pass | pass |
| Reserved `io.modelcontextprotocol/*` keys cannot be shadowed by `extra` | MUST | Implemented | `meta.rs:499-545` | `meta.rs::request_meta_object_extra_cannot_shadow_protocol_version` | pass | — | — | — |
| `ProgressToken` is string-or-number and preserves its JSON type | MUST | Implemented | `meta.rs` `ProgressToken` | `progress_2026.rs::progress_preserves_a_numeric_token`, `meta::tests::test_progress_token_integer_round_trips` | pass | — | — | — |
| `NotificationMetaObject`'s `subscriptionId` key round-trips on notification `_meta` | SHOULD | Implemented | `notifications.rs` binds `_meta` as `HashMap<String, Value>` — both `MetaObject` and `NotificationMetaObject` are `Record<string, unknown>` with named keys layered on, so the loose binding accepts and round-trips the named key | `subscriptions_listen_2026.rs::listen_acks_first_then_delivers_only_requested_types` | pass | — | — | — |

The `NotificationMetaObject` row was carried as a gap. It is a **documented
faithful loosening, not a deviation**: the crate's own binding records the
rationale at `notifications.rs:21-28`, the wire bytes are identical, and the
named key is asserted end to end. Dropped from the register.

`crates/turul-mcp-protocol-2026-07-28/COMPLIANCE.md` listed the reserved-key
guard under "Known gaps" while it was implemented and tested. Corrected in the
same slice as this reconciliation, along with that document's test-gate counts
(measured: 426 with `--features compliance`, 414 default) and its `turul-rpc`
version, all three of which had drifted.

## 8. Utilities

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| `ping` removed from core | MUST | Implemented | `builder.rs:144` gated to the 2025 lane | `error_mapping_2026.rs::methods_absent_from_the_2026_schema_get_404` | pass | — | — | — |
| `logging/setLevel` removed; replaced by `_meta.logLevel` | MUST | Implemented | `builder.rs:166` | `log_gating_2026.rs::message_notifications_require_a_declared_log_level` | pass | — | — | — |
| `notifications/cancelled` is client→server only, `requestId` required | MUST | Implemented | `notifications.rs` | `notifications.rs::test_cancelled_notification_always_emits_request_id`, `::test_cancelled_notification_rejects_missing_request_id` | pass | — | — | — |
| Closing the stream is treated as cancellation | MUST | Implemented | `streamable_http.rs` | `cancellation_2026.rs::client_disconnect_cancels_the_in_flight_request` | pass | — | — | — |
| Inbound client `notifications/progress` no longer dispatched | MUST | Implemented | `builder.rs:198` gated to the 2025 lane | `notifications_2026.rs::inbound_progress_notification_still_gets_202_with_no_dispatch_entry` | pass | — | — | — |
| Server progress only for a request that declared a token, in order, stopping at completion | SHOULD | Implemented | `session.rs:370-420` | `progress_2026.rs::progress_echoes_the_request_string_token`, `::progress_stops_after_completion`, `streaming_e2e_2026.rs::progress_frames_carry_increasing_values_before_the_result` | pass | — | — | — |
| Progress correlation is available on the 2025-11-25 lane too | SHOULD | Implemented | `SessionContext::progress_token`/`notify_request_progress`/`..._with_message` are lane-neutral; the token is populated from the typed `_meta` field on 2026-07-28 and by key from the untyped `meta` map on 2025-11-25 (`server.rs` tools/call, `handlers/mod.rs` resources/read) | `progress_token_match_2025_11_25.rs::a_progress_notification_carries_the_requests_own_token` | pass | — | — | — |
| Cursor pagination on list results | MUST | Implemented | `tools.rs`/`resources.rs`/`prompts.rs` | `wire_edges_2026.rs::tools_list_is_deterministic_paginated_and_cacheable` (tools); `list_pagination_2026.rs::resources_list_paginates_and_rejects_an_invalid_cursor`, `::resource_templates_list_paginates_and_rejects_an_invalid_cursor`, `::prompts_list_paginates_and_rejects_an_invalid_cursor` | pass | — | — | — |

All four paginated list methods now have a cursor walk. Each walk asserts three
properties a client relies on when it cannot inspect the token: the `limit=1`
walk reproduces the unpaginated listing exactly, it takes exactly `len()` pages
(so a server that ignores `limit` fails rather than passing by returning
everything at once), and an invalid cursor answers `-32602`.
`list_pagination_2026.rs::the_two_resource_listings_do_not_overlap` additionally
pins that templates never leak into `resources/list`.

## 9. `resultType` discriminator

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| Open union `"complete" \| "input_required" \| string`; unknown values preserved | MUST | Implemented | `result_type.rs:26-77` | `result_type.rs::accepts_unknown_discriminator_as_other`, `::other_round_trips_verbatim` | pass | — | — | — |
| Absent on deserialize defaults to `"complete"` | MUST | Implemented | `result_type.rs:38-43` | `result_type.rs::complete_is_default_per_backward_compat_rule` | pass | — | — | — |
| Every result carries it on the wire | MUST | Implemented | field on all 12 result structs | `scripts/interop-fastmcp.sh` asserts it on all 9 responses a foreign client received | pass | **pass** | pass | pass |

Per-struct wire assertions exist only for `DiscoverResult` and
`InputRequiredResult`. The interop probe is currently the broadest check that
every result actually carries the field.

## 10. Cacheability (SEP-2549)

Cacheable results, confirmed by grepping `extends CacheableResult` in the pinned
schema — six, not "all list results": `DiscoverResult`, `ListToolsResult`,
`ListResourcesResult`, `ListResourceTemplatesResult`, `ReadResourceResult`,
`ListPromptsResult`. Notably **not** `CallToolResult`, `GetPromptResult` or
`CompleteResult`.

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| `ttlMs` and `cacheScope` required on all six | MUST | Implemented | `caching.rs:64-100` | `caching.rs::cacheable_result_rejects_missing_ttl_ms`, `::cacheable_result_rejects_missing_cache_scope` | pass | — | — | — |
| Present on the wire for all six | MUST | Implemented | dispatch | `wire_edges_2026.rs::tools_list_is_deterministic_paginated_and_cacheable`, `discover_stateless_2026.rs::resources_list_dispatches_statelessly_with_cacheable_result`, `::prompts_list_dispatches_statelessly_with_cacheable_result`, `::resources_templates_list_answers_empty_rather_than_method_not_found`; all six asserted by `scripts/interop-fastmcp.sh` | pass | **pass** | pass | pass |
| HTTP `Cache-Control`/`ETag` derived from `ttlMs`/`cacheScope` | MAY | Not implemented | — | — | — | — | — | — |

## 11. Error codes

The spec partitions the JSON-RPC server-error range. `-32020..-32099` is
reserved for the specification. `-32000..-32019` is **legacy**, not a free
implementation range: "New codes MUST NOT be allocated in this sub-range, and
new implementations SHOULD NOT use codes from this sub-range at all." New codes
for purposes the spec does not define "SHOULD be allocated outside the JSON-RPC
reserved range (`-32768` to `-32000`)."

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| `-32020` header mismatch | MUST | Implemented | `headers.rs:58` | `mcp_headers_2026.rs::*` (16 wire sites) | pass | pass | pass | pass |
| `-32021` missing required client capability | MUST | Implemented | `lib.rs:480-486` | `mrtr_2026.rs::undeclared_capability_is_rejected_with_32021` | pass | — | — | — |
| `-32022` unsupported protocol version | MUST | Implemented | `lib.rs:487-497` | `discover_stateless_2026.rs::unsupported_protocol_version_header_is_rejected_with_32022` | pass | pass | pass | pass |
| Missing resource is `-32602`, no longer `-32002` | MUST | Implemented | `lib.rs:464-475` | `wire_edges_2026.rs::nonexistent_resource_is_invalid_params_on_the_wire` | pass | pass | pass | pass |
| Unknown method → HTTP 404 + `-32601` | MUST | Implemented | dispatch | `error_mapping_2026.rs::unknown_method_gets_http_404_with_method_not_found`, `::methods_absent_from_the_2026_schema_get_404` | pass | pass | pass | pass |
| Framework-internal codes stay out of the spec-reserved `-32020..-32099` | MUST | Implemented | `lib.rs:529-570` | `lib.rs::framework_internal_errors_are_legacy_allocations_or_outside_the_reserved_range`, `::no_two_framework_internal_errors_share_a_code` | pass | — | — | — |
| No **new** code allocated in the legacy `-32000..-32019` sub-range | MUST | Implemented | frozen `LEGACY_ALLOCATIONS` set in `lib.rs`; frozen constants in `c/turul-http-mcp-server/src/middleware/error.rs:9-40` | `lib.rs::framework_internal_errors_are_legacy_allocations_or_outside_the_reserved_range` (a non-grandfathered sub-range code now fails), `error.rs::middleware_codes_are_frozen_legacy_allocations` | pass | — | — | — |
| New implementations SHOULD NOT use `-32000..-32019` at all | SHOULD | **Deviation** | 14 pre-policy codes retained: `McpError` emits `-32000`, `-32010`..`-32019` (`lib.rs:507-570`); middleware emits `-32001`/`-32003`/`-32005` (`middleware/error.rs`). `-32005` is a *new* allocation in the closed sub-range, taken because the spec's recommended home is unreachable through `JsonRpcErrorObject::server_error`'s assert | the two guards above pin the set; no test moves them | — | — | — | — |
| `-32002` MUST NOT be emitted by implementations of this version | MUST | Implemented | `MiddlewareError::Unauthorized` → `-32005` (`middleware/error.rs`). The legacy ≤2024-11-05 `session_handler.rs` also emitted the literal `-32002` for a missing `Mcp-Session-Id`; that file is not `cfg`-gated and is reachable by protocol-version routing, so it applied on the 2026 lane too. Now `UNAUTHENTICATED` (`-32001`), matching what `streamable_http.rs` already returned for the same condition | `error_code_wire_2026.rs::permission_denial_and_missing_resource_are_distinguishable_and_neither_is_32002`, `error.rs::middleware_codes_are_frozen_legacy_allocations`, `error.rs::no_source_file_emits_the_forbidden_resource_not_found_code` (source scan — the per-constant guard missed the literal) | pass | — | — | — |
| `-32042` MUST NOT be emitted by implementations of this version | MUST | Implemented | never allocated — the only `32042` in `crates/` is the prose note in the vendored `schema/schema.ts:424` | NOT FOUND | — | — | — | — |

**HTTP status is layered, and the split is a contract:** a request the transport
rejects before dispatch (bad or missing headers, unsupported version) answers
4xx; an unknown method answers 404; a well-formed request that fails inside a
handler answers **200** with the error in the JSON-RPC body. Both halves are
pinned by `scripts/interop-fastmcp.sh` J5 and, since this slice, in Rust:
`error_mapping_2026.rs::handler_level_failure_is_http_200_with_the_error_in_the_body`
asserts 200 + `-32602` + id echo back to back with the 404 branch on the same
server, so the two are discriminated rather than each asserted alone. A body
the transport cannot parse as a JSON-RPC message (malformed JSON, or a batch
array) also follows the "rejects before dispatch → 4xx" half of the split,
answering 400 like the null-id and header-validation checks; see §1.

## 12. Security

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| `inputSchema` validated at build time (dialect, size, depth) — SEP-2106 | MUST | Implemented | `c/turul-mcp-schema-validation/src/lib.rs:78`, wired at `builder.rs:1658` | `lib.rs::unsupported_dialect_is_rejected`, `::oversized_schema_names_the_limit_in_its_message` | pass | — | — | — |
| No hardcoded credentials | SHOULD | Implemented | config passed to `JwtValidator::new` | `jwt.rs::test_audience_always_validated` | pass | — | — | — |
| Rate limiting | SHOULD | **Partial** | example only (`examples/middleware-rate-limit-server`) | NOT FOUND | — | — | — | — |

### State handle hijacking

2026-07-28 replaced 2025-11-25's session-ID binding requirement with a narrower
one keyed on *state handles* — the explicit identifiers a stateless server mints
and the client passes back on later requests
([Security Best Practices → State Handle Hijacking](https://modelcontextprotocol.io/docs/2026-07-28/tutorials/security/security_best_practices#state-handle-hijacking)).
Three turul-issued identifiers were assessed against it. Only the Tasks
extension's `taskId` is a state handle the framework itself mints and owns.

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| Tasks `taskId` — handles SHOULD be non-deterministic, from a secure RNG | SHOULD | Implemented | `c/turul-mcp-server/src/ext_tasks.rs:79` mints `Uuid::new_v4` (122 random bits), not the timestamp-prefixed v7 used for sessions | `ext_tasks_2026.rs::task_ids_are_unguessable_uuids` | pass | — | — | — |
| Tasks `taskId` — servers SHOULD bind handles server-side to the authenticated user (`<user_id>:<handle>`) and reject a handle presented by another principal | SHOULD | **Not implemented** | `c/turul-mcp-ext-tasks/src/v2026_07_28/store.rs:96-130` — every `TaskStore` method keys on `task_id` alone, and `TaskState` carries no owner field | NOT FOUND | — | — | — | — |
| Tasks `taskId` — servers that implement authorization MUST NOT treat possession of a handle as authentication | MUST | **Gap** | `tasks/get`/`tasks/update`/`tasks/cancel` implement `McpHandler::handle(&self, params)` (`ext_tasks.rs:330,370,409`) — the signature carries no session or auth context, so the handlers structurally cannot compare caller to owner. `notifications/tasks` delivery gates on the client-supplied `taskIds` filter alone (`streamable_http.rs:2052-2060`) | NOT FOUND | — | — | — | — |
| Tasks `taskId` — expiring handles reduce risk | SHOULD | **Not implemented** | `ext_tasks.rs:86` creates every task with `ttl_ms: Nullable(None)`; no store expires a task | NOT FOUND | — | — | — | — |
| `subscriptions/listen` subscription ID | SHOULD | **Not applicable** | Not a state handle: `streamable_http.rs:2001` sets it to the client's own JSON-RPC request id, and it is only ever emitted outbound in `_meta` — no method accepts it as an inbound lookup key. The spec scopes listen state "to the request itself, not to the connection underneath" ([Statelessness](https://modelcontextprotocol.io/specification/2026-07-28/basic/index#statelessness)), so nothing spans requests to be hijacked | n/a | n/a | n/a | n/a | n/a |
| MRTR `requestState` | SHOULD | **Unknown** | Handle-shaped (opaque, minted server-side, echoed by the client on the retry) but the framework never mints, stores or interprets it: the value originates in the tool's `McpError::InputRequired` and is passed back verbatim through session extensions (`server.rs:1840-1843`, `handlers/mod.rs:629-631,947-949`). Binding is therefore the tool author's obligation, and nothing documents that. The verified principal *is* reachable from a tool — `turul-mcp-oauth/src/middleware.rs:135-138` writes validated claims into request extensions, which thread into `SessionContext` | `session.rs::test_extensions_thread_from_json_rpc_to_framework` (plumbing only — no test binds `requestState` to a principal) | — | — | — | — |

---

## Gap register

Ordered by consequence, not by area.

1. **Possession of a `taskId` is the entire access check for the Tasks
   extension.** `tasks/get`/`update`/`cancel` and `notifications/tasks` delivery
   never compare the caller to the handle's owner, and `McpHandler::handle`
   gives them no principal to compare against — so a turul server with OAuth
   wired cannot satisfy "MUST NOT treat possession of a state handle as
   authentication" even if the operator wants to. Unguessable v4 ids raise the
   bar but are not the requirement.
2. **Framework-internal codes remain in a sub-range the spec closes.**
   `UNAUTHORIZED` moved off the forbidden `-32002` to `-32005` on 2026-07-29,
   which ends the `MUST NOT` violation, but `-32005` is still inside the legacy
   `-32000..-32019` band that new implementations SHOULD NOT use. The spec's
   recommended home above `-32099` is unreachable: both
   `map_middleware_error_to_jsonrpc` sites build through
   `JsonRpcErrorObject::server_error`, whose `assert!` demands
   `-32099..=-32000`, so moving there panics rather than failing. Closing this
   properly needs a change in the sibling `turul-rpc` crate.
3. **TLS is not enforced on JWKS / issuer URIs.** `JwtValidator::new` performs
   no scheme check, so a misconfigured `http://` JWKS endpoint is accepted
   silently. ADR-021 claims the posture; no code implements it and no test
   looks. (The `Cache-Control: no-store` half of this entry closed — see §5.)
4. **The 403 `insufficient_scope` challenge has no wire test.** It shares
   `build_http_challenge_response` with the three challenges
   `oauth_2026.rs::challenges_are_not_cacheable` drives, so the header is
   present by construction, but construction is not an assertion.
5. **ADR-027 does not document the six auth-hardening SEPs** the Branch Lock
   headlines.
6. **`MiddlewareError::InvalidRequest`/`Internal`/`Custom` panic on any
   middleware rejection through `map_middleware_error_to_jsonrpc`.** They map to
   `-32600`/`-32603` and are passed to `JsonRpcErrorObject::server_error`, whose
   `assert!` requires `-32099..=-32000`. Latent, pre-existing, and the same
   constructor that blocks gap 2 — one slice would close both.

**Closed this slice** (recorded so a reader does not re-open them): phantom
`stdio`/`all-transports` Cargo features deleted; misplaced `x-mcp-header`
exclusion located and tested; cursor walks added for the three non-`tools`
list methods; batch-array rejection tested; `.well-known/*` Origin exemption
tested; `Cache-Control: no-store` on auth challenges located and tested; the
handler-level HTTP 200 case tested; the crate's own `COMPLIANCE.md` corrected on
the reserved-key guard, its test-gate counts and its `turul-rpc` version; the
JSON-RPC-parse-failure branch changed from HTTP 200 to 400 so it agrees with
the null-id and header-validation branches for the same class of pre-dispatch
envelope violation.
**Dropped as mislabelled**: the
`NotificationMetaObject` row (documented faithful loosening, wire-identical,
now asserted) and the `cfg!()` `notifications/initialized` residue (`cfg!`
const-folds; it is not a runtime check).
