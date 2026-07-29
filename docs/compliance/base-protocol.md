# Base Protocol — MCP 2026-07-28

Column meanings and interop values: see [README.md](README.md). Interop columns
are `turul | python | typescript | go`; `—` means not exercised, never "pass".

Test paths are relative to the repo root. `c/` abbreviates `crates/`.

---

## 1. JSON-RPC message shape

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| `jsonrpc: "2.0"` on every message | MUST | Implemented | `c/turul-mcp-protocol-2026-07-28/src/json_rpc.rs:29` | `discover.rs::discover_result_response_wire_shape` | pass | pass | — | — |
| `RequestId` is string or number; bare `null` rejected | MUST | Implemented | schema `RequestId` | `wire_edges_2026.rs::null_request_id_is_rejected` | pass | — | — | — |
| Batch (JSON array) bodies rejected — batching removed in 2026-07-28 | MUST | Implemented | `c/turul-http-mcp-server/src/streamable_http.rs:769` calls only the singular parser | **NOT FOUND** | — | — | — | — |
| `params._meta` is required, not optional | MUST | Implemented | `json_rpc.rs:36-46` | `json_rpc.rs::test_request_params_rejects_missing_meta` | pass | — | — | — |

The batch-rejection guarantee is **structural** — the batch-capable parser is
simply never called — not test-proven. Nothing posts a JSON array body.

## 2. Lifecycle — the stateless core

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| `initialize` / `notifications/initialized` are removed | MUST | Implemented | `c/turul-mcp-server/src/server.rs:494` gates registration to the 2025 lane | `wire_edges_2026.rs::initialize_error_names_supported_versions` | pass | pass | — | — |
| `Mcp-Session-Id` never minted or echoed | MUST | Implemented | `streamable_http.rs:65` | `stateless_2026_http_surface.rs::responses_never_mint_session_ids`, `::inbound_mcp_session_id_is_ignored_and_never_echoed` | pass | pass | — | — |
| GET and DELETE on the endpoint answer 405 | MUST | Implemented | `streamable_http.rs:453-467` | `stateless_2026_http_surface.rs::get_returns_405_method_not_allowed`, `::delete_returns_405_method_not_allowed`, `::get_with_last_event_id_returns_405` | pass | — | — | — |
| `server/discover` is the bootstrap method | MUST | Implemented | `server.rs:1328-1386` | `discover_stateless_2026.rs::server_discover_answers_without_a_session` | pass | pass | — | — |
| Lambda transport enforces the same stateless contract | Parity | Implemented | `c/turul-mcp-aws-lambda/src/handler.rs` | `scripts/e2e-lambda-local.sh` (10 assertions through the real Runtime API) | pass | n/a | n/a | n/a |

**Known residue:** `server.rs` still contains `notifications/initialized` string
literals behind a *runtime* `cfg!()` check rather than a compile-time `#[cfg]`,
so they are compiled into the default 2026 binary and are always false. Dead but
present.

## 3. Transports

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| POST answers `application/json` when the request opted into nothing | MUST | Implemented | `streamable_http.rs:780,898` | `streaming_e2e_2026.rs::json_replies_are_a_single_object_with_no_event_framing` | pass | pass | — | — |
| POST answers `text/event-stream` when the request declared `progressToken` | MUST | Implemented | `streamable_http.rs:965,1033` | `progress_2026.rs::combined_accept_uses_json_without_a_token_and_sse_with_one` | pass | pass | — | — |
| SSE bodies are well-formed event-stream grammar | MUST | Implemented | `streamable_http.rs:2407` | `streaming_e2e_2026.rs::sse_body_matches_the_event_stream_grammar` | pass | pass | — | — |
| SSE responses declare an unbuffered stream (no Content-Length, `no-cache`) | SHOULD | Implemented | `streamable_http.rs:2502-2511` | `streaming_e2e_2026.rs::sse_response_headers_declare_an_unbuffered_stream` | pass | — | — | — |
| The result frame ends the stream | MUST | Implemented | `streamable_http.rs` dispatch | `streaming_e2e_2026.rs::the_result_frame_is_last_and_closes_the_stream` | pass | — | — | — |
| Origin absent → allowed; loopback → allowed; same-host → allowed | MUST | Implemented | `c/turul-http-mcp-server/src/origin.rs:82-146` | `origin_validation_2026.rs::origin_absent_is_allowed`, `::loopback_origin_is_allowed_by_default`, `::same_host_origin_is_allowed_by_default` | pass | — | — | — |
| Cross-origin → 403 before body parsing or auth | MUST | Implemented | `streamable_http.rs:439-451` | `origin_validation_2026.rs::cross_origin_is_rejected_with_403_by_default` | pass | — | — | — |
| OPTIONS preflight exempt; the following real request is still gated | MUST | Implemented | `streamable_http.rs:427-433` | `origin_validation_2026.rs::options_preflight_is_exempt_but_actual_request_is_gated` | pass | — | — | — |
| `.well-known/*` exempt from Origin validation | MUST | Implemented | `server.rs:581` dispatches before the transport | **NOT FOUND** | — | — | — | — |
| stdio transport | MAY | **Not implemented** | — | — | — | — | — | — |

**Defect:** `turul-mcp-client` declares Cargo features `stdio` and
`all-transports = ["http","sse","stdio"]`, but no stdio module exists
(`src/transport/` holds only `http.rs` and `sse.rs`). Enabling the feature
compiles and provides nothing. Either implement it or remove the feature.

FastMCP's `pass` on the two framing rows is notable evidence, not a formality:
it negotiated into **SSE framing for eight of nine requests** and JSON for the
first, and parsed both — see the wire capture in `scripts/interop-fastmcp.sh`.
The "client MUST support both framings" rule is confirmed by a client we did not
write.

## 4. Versioning

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| `MCP-Protocol-Version` required on every POST | MUST | Implemented | `streamable_http.rs:1309-1362` | `mcp_headers_2026.rs::missing_protocol_version_header_is_rejected` | pass | pass | — | — |
| Header disagreeing with `_meta.protocolVersion` → 400 + `-32020` | MUST | Implemented | `streamable_http.rs:1309-1362` | `discover_stateless_2026.rs::header_body_protocol_version_mismatch_is_rejected_with_32020` | pass | — | — | — |
| Unsupported version → 400 + `-32022`, naming supported versions | MUST | Implemented | `streamable_http.rs:1328-1359` | `discover_stateless_2026.rs::unsupported_protocol_version_header_is_rejected_with_32022` | pass | pass | — | — |
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
| `Cache-Control: no-store` on 401/403 challenges | SHOULD | **Unknown** | NOT FOUND | NOT FOUND | — | — | — | — |
| TLS enforced on JWKS / issuer URIs | SHOULD | **Unknown** | no scheme check in `JwtValidator::new` | NOT FOUND | — | — | — | — |
| RFC 9207 `iss` in the authorization response (SEP-2468) | MUST | **Out-of-role** | absent by design — an RS never handles the authorization response | n/a | n/a | n/a | n/a | n/a |
| OIDC `application_type` on DCR (SEP-837) | SHOULD | **Out-of-role** | `oauth/src/lib.rs:24-27` states DCR is out of scope | n/a | n/a | n/a | n/a | n/a |
| Refresh-token grant handling (SEP-2207) | SHOULD | **Out-of-role** | AS concern | n/a | n/a | n/a | n/a | n/a |
| Scope accumulation across incremental auth (SEP-2350) | SHOULD | **Out-of-role** | AS/client concern | n/a | n/a | n/a | n/a | n/a |

**Documentation gap:** ADR-021 mentions RFC 9207 but the governing 2026 ADR
(ADR-027) does not discuss any of the six auth-hardening SEPs that AGENTS.md and
CLAUDE.md headline in the Branch Lock. The *code* posture is defensible; the ADR
is silent on its own stated scope. `grep -rl 9207 crates/` returns nothing —
9207 appears only in `docs/`.

## 6. Request metadata headers (SEP-2243)

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| `Mcp-Method` required on every request and notification, must match the body | MUST | Implemented | `streamable_http.rs:1364-1372` | `mcp_headers_2026.rs::missing_mcp_method_header_is_rejected`, `::mismatched_mcp_method_header_is_rejected`, `::notifications_also_require_mcp_method` | pass | pass | — | — |
| `Mcp-Name` required where the method names a target, must match | MUST | Implemented | `streamable_http.rs:1374-1418` | `mcp_headers_2026.rs::missing_mcp_name_on_tools_call_is_rejected`, `::resources_read_requires_uri_as_mcp_name`, `::methods_without_name_need_no_mcp_name` | pass | pass | — | — |
| Base64 sentinel decoding for non-ASCII `Mcp-Name` | MUST | Implemented | `headers.rs:305-357` | `mcp_headers_2026.rs::base64_encoded_mcp_name_decodes_and_matches`, `::base64_encoded_mcp_name_mismatch_is_rejected` | pass | — | — | — |
| `x-mcp-header` annotations discovered and enforced as `Mcp-Param-*` | MUST | Implemented | `headers.rs:109-180`, `server.rs:1899-1949` | `mcp_param_2026.rs::matching_param_header_passes_validation`, `::omitted_param_header_with_body_value_is_rejected`, `::mismatched_param_header_is_rejected`, `::integer_params_compare_numerically` | pass | — | — | — |
| Misplaced `x-mcp-header` excluded from `tools/list` | MUST | **Partial** | `headers.rs:195-303` detects and warns, but does not exclude the tool | detector tested (`headers.rs::annotation_under_items_is_flagged`); exclusion **NOT FOUND** | — | — | — | — |

## 7. `_meta` general fields

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| `protocolVersion` required in request `_meta` | MUST | Implemented | `meta.rs:403-409` | `wire_edges_2026.rs::incomplete_meta_branches_are_rejected` | pass | pass | — | — |
| `clientCapabilities` required in request `_meta` | MUST | Implemented | `meta.rs:416-422` | `wire_edges_2026.rs::incomplete_meta_branches_are_rejected` | pass | pass | — | — |
| `clientInfo` is OPTIONAL | MUST | Implemented | `meta.rs:411-414` | `discover_stateless_2026.rs::request_without_client_info_is_served`, `wire_edges_2026.rs::absent_client_info_is_not_an_incomplete_meta` | pass | — | — | — |
| A present-but-malformed `clientInfo` is rejected | MUST | Implemented | `handlers/mod.rs` | `discover_stateless_2026.rs::malformed_client_info_is_rejected` | pass | — | — | — |
| `serverInfo` stamped on results, in `_meta`, never top-level | SHOULD | Implemented | `meta.rs:272-281`, stamped at `server.rs:1371` | `meta.rs::valid_server_info_round_trips_through_the_wire`, `progress_2026.rs::sse_result_frame_carries_server_info_meta` | pass | pass | — | — |
| Reserved `io.modelcontextprotocol/*` keys cannot be shadowed by `extra` | MUST | Implemented | `meta.rs:499-545` | `meta.rs::request_meta_object_extra_cannot_shadow_protocol_version` | pass | — | — | — |
| `ProgressToken` is string-or-number and preserves its JSON type | MUST | Implemented | `meta.rs` `ProgressToken` | `progress_2026.rs::progress_preserves_a_numeric_token`, `meta::tests::test_progress_token_integer_round_trips` | pass | — | — | — |
| `NotificationMetaObject` bound as a distinct typed carrier | SHOULD | **Partial** | `notifications.rs:18-27` still uses the loose `MetaObject` | NOT FOUND | — | — | — | — |

`crates/turul-mcp-protocol-2026-07-28/COMPLIANCE.md` still lists the reserved-key
guard under "Known gaps". It is implemented and tested; that document is stale on
this point.

## 8. Utilities

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| `ping` removed from core | MUST | Implemented | `builder.rs:144` gated to the 2025 lane | `error_mapping_2026.rs::methods_absent_from_the_2026_schema_get_404` | pass | — | — | — |
| `logging/setLevel` removed; replaced by `_meta.logLevel` | MUST | Implemented | `builder.rs:166` | `log_gating_2026.rs::message_notifications_require_a_declared_log_level` | pass | — | — | — |
| `notifications/cancelled` is client→server only, `requestId` required | MUST | Implemented | `notifications.rs` | `notifications.rs::test_cancelled_notification_always_emits_request_id`, `::test_cancelled_notification_rejects_missing_request_id` | pass | — | — | — |
| Closing the stream is treated as cancellation | MUST | Implemented | `streamable_http.rs` | `cancellation_2026.rs::client_disconnect_cancels_the_in_flight_request` | pass | — | — | — |
| Inbound client `notifications/progress` no longer dispatched | MUST | Implemented | `builder.rs:198` gated to the 2025 lane | `notifications_2026.rs::inbound_progress_notification_still_gets_202_with_no_dispatch_entry` | pass | — | — | — |
| Server progress only for a request that declared a token, in order, stopping at completion | SHOULD | Implemented | `session.rs:370-420` | `progress_2026.rs::progress_echoes_the_request_string_token`, `::progress_stops_after_completion`, `streaming_e2e_2026.rs::progress_frames_carry_increasing_values_before_the_result` | pass | — | — | — |
| Cursor pagination on list results | MUST | **Partial** | `tools.rs`/`resources.rs`/`prompts.rs` | `wire_edges_2026.rs::tools_list_is_deterministic_paginated_and_cacheable` — **tools only** | partial | — | — | — |

Pagination is asserted end-to-end for `tools/list` alone. `resources/list`,
`resources/templates/list` and `prompts/list` have type-level coverage but no
cursor walk and no invalid-cursor rejection test.

## 9. `resultType` discriminator

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| Open union `"complete" \| "input_required" \| string`; unknown values preserved | MUST | Implemented | `result_type.rs:26-77` | `result_type.rs::accepts_unknown_discriminator_as_other`, `::other_round_trips_verbatim` | pass | — | — | — |
| Absent on deserialize defaults to `"complete"` | MUST | Implemented | `result_type.rs:38-43` | `result_type.rs::complete_is_default_per_backward_compat_rule` | pass | — | — | — |
| Every result carries it on the wire | MUST | Implemented | field on all 12 result structs | `scripts/interop-fastmcp.sh` asserts it on all 9 responses a foreign client received | pass | **pass** | — | — |

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
| Present on the wire for all six | MUST | Implemented | dispatch | `wire_edges_2026.rs::tools_list_is_deterministic_paginated_and_cacheable`, `discover_stateless_2026.rs::resources_list_dispatches_statelessly_with_cacheable_result`, `::prompts_list_dispatches_statelessly_with_cacheable_result`, `::resources_templates_list_answers_empty_rather_than_method_not_found`; all six asserted by `scripts/interop-fastmcp.sh` | pass | **pass** | — | — |
| HTTP `Cache-Control`/`ETag` derived from `ttlMs`/`cacheScope` | MAY | Not implemented | — | — | — | — | — | — |

## 11. Error codes

The schema partitions the JSON-RPC server-error range: `-32000..-32019` is
implementation-defined and never assigned by the spec; `-32020..-32099` is
spec-reserved.

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| `-32020` header mismatch | MUST | Implemented | `headers.rs:58` | `mcp_headers_2026.rs::*` (16 wire sites) | pass | pass | — | — |
| `-32021` missing required client capability | MUST | Implemented | `lib.rs:480-486` | `mrtr_2026.rs::undeclared_capability_is_rejected_with_32021` | pass | — | — | — |
| `-32022` unsupported protocol version | MUST | Implemented | `lib.rs:487-497` | `discover_stateless_2026.rs::unsupported_protocol_version_header_is_rejected_with_32022` | pass | pass | — | — |
| Missing resource is `-32602`, no longer `-32002` | MUST | Implemented | `lib.rs:464-475` | `wire_edges_2026.rs::nonexistent_resource_is_invalid_params_on_the_wire` | pass | pass | — | — |
| Unknown method → HTTP 404 + `-32601` | MUST | Implemented | dispatch | `error_mapping_2026.rs::unknown_method_gets_http_404_with_method_not_found`, `::methods_absent_from_the_2026_schema_get_404` | pass | pass | — | — |
| Framework-internal codes stay out of the spec-reserved range | MUST | Implemented | `lib.rs:529-570` | `lib.rs::framework_internal_errors_stay_out_of_spec_reserved_range`, `::no_two_framework_internal_errors_share_a_code` | pass | — | — | — |

**HTTP status is layered, and the split is a contract:** a request the transport
rejects before dispatch (bad or missing headers, unsupported version) answers
4xx; an unknown method answers 404; a well-formed request that fails inside a
handler answers **200** with the error in the JSON-RPC body. Both halves are
pinned by `scripts/interop-fastmcp.sh` J5. No Rust test asserts the 200 case —
the existing `nonexistent_resource_is_invalid_params_on_the_wire` discards the
status.

## 12. Security

| Requirement | Level | Status | Implementation | Verified by | turul | py | ts | go |
|---|---|---|---|---|---|---|---|---|
| `inputSchema` validated at build time (dialect, size, depth) — SEP-2106 | MUST | Implemented | `c/turul-mcp-schema-validation/src/lib.rs:78`, wired at `builder.rs:1658` | `lib.rs::unsupported_dialect_is_rejected`, `::oversized_schema_names_the_limit_in_its_message` | pass | — | — | — |
| No hardcoded credentials | SHOULD | Implemented | config passed to `JwtValidator::new` | `jwt.rs::test_audience_always_validated` | pass | — | — | — |
| Rate limiting | SHOULD | **Partial** | example only (`examples/middleware-rate-limit-server`) | NOT FOUND | — | — | — | — |

---

## Gap register

Ordered by consequence, not by area.

1. **`stdio` is an advertised Cargo feature that does nothing** — `stdio` and
   `all-transports` exist in `turul-mcp-client/Cargo.toml` with no stdio module.
2. **Misplaced `x-mcp-header` annotations are detected but not excluded** from
   `tools/list`, so a malformed schema ships to clients with validation silently
   skipped.
3. **Pagination is only end-to-end tested for `tools/list`** — three other
   paginated list methods have no cursor walk.
4. **Batch-array rejection is structural, not tested.**
5. **`.well-known/*` Origin exemption is untested.**
6. **`Cache-Control: no-store` on auth challenges and TLS enforcement on JWKS
   URIs are both Unknown** — claimed by ADR-021, not located in code.
7. **ADR-027 does not document the six auth-hardening SEPs** the Branch Lock
   headlines.
8. **`NotificationMetaObject` is still the loose `MetaObject`** (documented
   deviation, wire-equivalent).
9. **Dead `notifications/initialized` literals** compiled into the 2026 binary
   behind a runtime rather than compile-time check.
10. **`crates/turul-mcp-protocol-2026-07-28/COMPLIANCE.md` is stale** on the
    reserved-key guard, listing as open something that is implemented and tested.
