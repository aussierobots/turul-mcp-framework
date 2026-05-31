# Schema Coverage Matrix — `DRAFT-2026-v1` schema.ts ↔ `turul-mcp-protocol-2026-07-28`

**Source**: `crates/turul-mcp-protocol-2026-07-28/schema/draft-schema.ts` (2983 lines, ETag `8bdd4ae5…`)
**Target**: `crates/turul-mcp-protocol-2026-07-28/src/` (288 tests passing)
**Last walked**: 2026-05-24

This document walks every `export interface`, `export type`, and `export const` in `schema/draft-schema.ts` from top to bottom and records:

- **Rust binding**: which `pub struct`/`enum`/`type`/`const` implements it, and where
- **Tests**: which compliance test asserts the wire shape, and where
- **Docs**: where the type is documented (module rustdoc / README / ADR)

`✓` = covered. `✗` = gap (would be a real compliance issue). `↳` = intentionally absent (the schema doesn't define it; we shouldn't either).

---

## 1. JSON-RPC primitives (schema lines 1–258)

| Schema | Lines | Rust binding | Test | Doc |
|--------|-------|--------------|------|-----|
| `JSONValue`, `JSONObject`, `JSONArray` | 6, 17, 22 | `serde_json::Value` (built-in) | n/a | inline rustdoc |
| `JSONRPCMessage` (union) | 31 | `json_rpc::JsonRpcMessage` (`json_rpc.rs:296`) | `compliance_test.rs::envelope` | `json_rpc.rs` mod doc |
| `LATEST_PROTOCOL_VERSION = "DRAFT-2026-v1"` | 37 | `lib.rs::MCP_VERSION` + `version.rs::McpVersion::V2026_07_28` serde rename | `compliance_test.rs::removed_methods::schema_protocol_version_constant_matches_crate` | `lib.rs` + ADR-027 |
| `JSONRPC_VERSION = "2.0"` | 39 | `json_rpc::JSONRPC_VERSION` (`json_rpc.rs:32`) | `compliance_test.rs::envelope::jsonrpc_version_constant_is_literal_2_0` | `json_rpc.rs` |
| `MetaObject = Record<string, unknown>` | 61 | `meta::MetaObject` type alias (`meta.rs:408`) | `request_meta::extra_keys_preserved_via_flatten` | `meta.rs` mod doc |
| `RequestMetaObject` | 70–107 | `meta::RequestMetaObject` (`meta.rs:434`) | `compliance_test.rs::request_meta` (8 tests) | `meta.rs` mod doc, migration-diff |
| `ProgressToken = string \| number` | 114 | `meta::ProgressToken` (transparent String, `meta.rs:66`) + `notifications::ProgressTokenValue` (string/number enum, `notifications.rs:206`) | `request_meta::progress_token_serializes_under_short_camelcase_key`, `method_strings::notifications_progress_binding` | inline |
| `Cursor = string` | 121 | `meta::Cursor` (transparent String, `meta.rs:93`) | round-trips in `tools_alignment`, `resources_alignment` | inline |
| `RequestParams { _meta: RequestMetaObject }` | 128 | `json_rpc::RequestParams` (`json_rpc.rs:37`) — note: `_meta` typed `Option<Meta>` transitionally; tightening to required `RequestMetaObject` is Phase 3 finalization | `envelope::jsonrpc_request_with_params_serializes_params` | `json_rpc.rs` "Known divergences" |
| `Request` (internal loose shape) | 133 | n/a — Rust uses typed `RequestParams` | n/a | `json_rpc.rs` |
| `NotificationParams { _meta?: MetaObject }` | 145 | `notifications::NotificationParams` (`notifications.rs:16`) | implicitly via notification shape tests | inline |
| `Notification` (internal loose shape) | 150 | n/a — Rust uses typed notification structs | n/a | inline |
| `ResultType = "complete" \| "input_required"` | 165 | `result_type::ResultType` (`result_type.rs:25`) | `result_type::tests` (6) + `compliance_test::result_discrimination` (3) | `result_type.rs` mod doc |
| `Result { _meta?, resultType, [k]: unknown }` | 172 | embedded `result_type: ResultType` field on each `*Result` struct | per-area: `tools_alignment::call_tool_result_always_emits_result_type`, etc. | per-result rustdoc |
| `Error { code, message, data? }` | 190 | `json_rpc::JsonRpcError` (`json_rpc.rs:226`) | `envelope::jsonrpc_error_object_shape`, `jsonrpc_error_object_omits_data_when_absent` | `json_rpc.rs` |
| `RequestId = string \| number` | 210 | `turul_mcp_json_rpc_server::types::RequestId` (re-exported) | `envelope::jsonrpc_success_response_has_result_no_error` | `json_rpc.rs` "Known divergences" |
| `JSONRPCRequest` | 217 | `json_rpc::JsonRpcRequest` (`json_rpc.rs:169`) | `envelope::jsonrpc_request_wire_shape` | `json_rpc.rs` |
| `JSONRPCNotification` | 227 | `json_rpc::JsonRpcNotification` (`json_rpc.rs:271`) | `envelope::jsonrpc_notification_has_no_id` | `json_rpc.rs` |
| `JSONRPCResultResponse` | 236 | `json_rpc::JsonRpcResponse::success` constructor | `envelope::jsonrpc_success_response_has_result_no_error` | `json_rpc.rs` |
| `JSONRPCErrorResponse` | 247 | `json_rpc::JsonRpcResponse::error` constructor | `envelope::jsonrpc_error_response_has_error_no_result` | `json_rpc.rs` |
| `JSONRPCResponse` (union) | 258 | `json_rpc::JsonRpcResponse` (single struct with Optional result/error) | both success + error tests above | `json_rpc.rs` "Known divergences" |

**Status**: ✓ All bound. Two "known divergences" documented inline in `json_rpc.rs` module header (single response struct vs schema's tagged union; `RequestId` typed loosely as `serde_json::Value`).

## 2. Error codes (schema lines 261–427)

| Schema | Lines | Rust binding | Test | Doc |
|--------|-------|--------------|------|-----|
| `PARSE_ERROR = -32700` | 261 | `JsonRpcError::parse_error()` (`json_rpc.rs`) + dispatched via `McpError` | `envelope::standard_jsonrpc_error_codes_match_schema_constants` | `json_rpc.rs` |
| `INVALID_REQUEST = -32600` | 262 | `JsonRpcError::invalid_request()` | same test + `error_codes::invalid_params_variants_all_emit_minus_32602` | `json_rpc.rs` |
| `METHOD_NOT_FOUND = -32601` | 263 | `JsonRpcError::method_not_found()` | same test | `json_rpc.rs` |
| `INVALID_PARAMS = -32602` | 264 | `JsonRpcError::invalid_params()` + multiple `McpError` variants map here per SEP-2164 | `error_codes::tool_not_found_maps_to_invalid_params`, `resource_not_found_maps_to_invalid_params`, `prompt_not_found_maps_to_invalid_params`, `invalid_params_variants_all_emit_minus_32602` | `lib.rs::McpError` rustdoc |
| `INTERNAL_ERROR = -32603` | 265 | `JsonRpcError::internal_error()` + `McpError::IoError`/`SerializationError` map here | `envelope::standard_jsonrpc_error_codes_match_schema_constants` | `lib.rs::McpError` |
| `ParseError`, `InvalidRequestError`, `MethodNotFoundError`, `InvalidParamsError`, `InternalError` (type aliases) | 277, 288, 307, 339, 353 | covered via the `McpError` → wire-code mapping in `lib.rs::to_error_object` | `error_codes::no_unauthorised_error_codes_emitted` (drift detector enumerates every variant) | `lib.rs::McpError` rustdoc |
| `MISSING_REQUIRED_CLIENT_CAPABILITY = -32003` | 363 | `McpError::MissingRequiredClientCapability { required: Value }` (`lib.rs`) | `error_codes::missing_required_client_capability_emits_minus_32003_with_data` | `lib.rs::McpError` |
| `UNSUPPORTED_PROTOCOL_VERSION = -32004` | 371 | `McpError::UnsupportedProtocolVersion { supported, requested }` | `error_codes::unsupported_protocol_version_emits_minus_32004_with_data` | `lib.rs::McpError` |
| `UnsupportedProtocolVersionError` (structured) | 384–402 | same `McpError::UnsupportedProtocolVersion` variant — `data` shape verified | `error_codes::unsupported_protocol_version_emits_minus_32004_with_data` (asserts `data.supported` and `data.requested`) | `lib.rs::McpError` |
| `MissingRequiredClientCapabilityError` (structured) | 414–427 | same `McpError::MissingRequiredClientCapability` variant | `error_codes::missing_required_client_capability_emits_minus_32003_with_data` (asserts `data.requiredCapabilities`) | `lib.rs::McpError` |

**Status**: ✓ All bound. Drift detector pins the allowed wire-code set.

## 3. Empty + multi-round-trip flow (schema lines 429–514)

| Schema | Lines | Rust binding | Test | Doc |
|--------|-------|--------------|------|-----|
| `EmptyResult = Result` | 435 | `ping::EmptyResult` (`ping.rs:17`) with required `result_type` | `empty_result_alignment::empty_result_serializes_result_type_complete`, `empty_result_back_compat_accepts_missing_result_type`, `empty_result_with_meta_round_trips` | `ping.rs` mod doc |
| `InputRequest` (union) | 438 | `input_required::InputRequest` (`input_required.rs:36`) untagged enum: CreateMessage/ListRoots/Elicit | `multi_round_trip::input_request_list_roots_serializes_with_method_string` (in tests/input_required) | `input_required.rs` mod doc |
| `InputResponse` (union) | 444 | `input_required::InputResponse` (`input_required.rs:51`) | `multi_round_trip::client_retries_with_responses_keyed_same_as_input_requests` | `input_required.rs` |
| `InputRequests { [k]: InputRequest }` | 458 | `input_required::InputRequests` type alias (`input_required.rs:63`) | `multi_round_trip::server_emits_input_required_result_with_one_request_and_state` | `input_required.rs` |
| `InputResponses { [k]: InputResponse }` | 472 | `input_required::InputResponses` type alias (`input_required.rs:68`) | `multi_round_trip::client_retries_with_responses_keyed_same_as_input_requests` | `input_required.rs` |
| `InputRequiredResult extends Result` | 489 | `input_required::InputRequiredResult` (`input_required.rs:80`) | `multi_round_trip::server_emits_input_required_result_with_one_request_and_state`, `input_required_well_formed_invariant`, plus 6 unit tests in `input_required::tests` | `input_required.rs` |
| `InputResponseRequestParams extends RequestParams` | 505 | `input_required::InputResponseRequestParams` mixin (`input_required.rs:161`) — actually embedded as `input_responses`/`request_state` fields on `CallToolRequestParams`/`ReadResourceRequestParams`/`GetPromptRequestParams` | `tools_alignment::call_tool_params_with_input_responses_mixes_in_correctly`, `resources_alignment::read_resource_params_input_responses_mixin_serializes`, `prompts_alignment::get_prompt_params_input_responses_mixin_serializes` | `input_required.rs` |

**Status**: ✓ All bound. Multi-round-trip flow has dedicated `multi_round_trip` test module proving end-to-end.

## 4. Cancellation (schema lines 516–554)

| Schema | Lines | Rust binding | Test | Doc |
|--------|-------|--------------|------|-----|
| `CancelledNotificationParams` | 525 | `notifications::CancelledNotificationParams` (`notifications.rs:326`) | covered via `method_strings::DRAFT_METHODS` list (`notifications/cancelled`); construction tested in `notifications::tests::test_cancelled_notification` | inline |
| `CancelledNotification` | 551 | `notifications::CancelledNotification` (`notifications.rs:317`) | `method_strings::DRAFT_METHODS` enumerates `"notifications/cancelled"` and pins schema presence | inline |

**Status**: ✓ Bound.

## 5. Discovery (schema lines 556–616)

| Schema | Lines | Rust binding | Test | Doc |
|--------|-------|--------------|------|-----|
| `DiscoverRequest` | 568 | `discover::DiscoverRequest` (`discover.rs:34`) | `discover::tests::discover_request_method_is_server_discover`, `method_strings::server_discover_binding` | `discover.rs` mod doc |
| `DiscoverResult extends Result` | 581 | `discover::DiscoverResult` (`discover.rs:72`) with `result_type: ResultType` | `discover::tests::discover_result_serializes_required_fields`, `discover_result_back_compat_accepts_missing_result_type` (+ 3 more) | `discover.rs` |
| `DiscoverResultResponse extends JSONRPCResultResponse` | 614 | `discover::DiscoverResultResponse` (`discover.rs:127`) | `discover::tests::discover_result_response_wire_shape` | `discover.rs` |
| `SERVER_DISCOVER_METHOD` const | derived | `discover::SERVER_DISCOVER_METHOD = "server/discover"` (`discover.rs:23`) | `discover::tests::discover_request_constant_matches_method` | inline |

**Status**: ✓ All bound.

## 6. Capabilities (schema lines 618–772)

| Schema | Lines | Rust binding | Test | Doc |
|--------|-------|--------------|------|-----|
| `ClientCapabilities` | 623 | `initialize::ClientCapabilities` (`initialize.rs:139`) | `capabilities_shape::client_sampling_subcapabilities_serialize_with_camelcase`, `client_elicitation_subcapabilities_serialize_with_camelcase`, `client_extensions_serializes_reverse_dns_keys`, `capabilities_round_trip_through_json` | `initialize.rs` mod doc, ADR-027 |
| `ClientCapabilities.experimental?: { [k]: JSONObject }` | 627 | field `experimental: Option<HashMap<String, Value>>` | covered via shape test | inline |
| `ClientCapabilities.roots?: {}` | 635 | field `roots: Option<RootsCapabilities>` (`initialize.rs:67`) | covered via shape test | inline |
| `ClientCapabilities.sampling?: { context?, tools? }` | 648 | `sampling: Option<SamplingCapabilities>` with `context` + `tools` sub-fields (`initialize.rs:83`) | `capabilities_shape::client_sampling_subcapabilities_serialize_with_camelcase`, `client_sampling_subcapabilities_omitted_when_none` | inline |
| `ClientCapabilities.elicitation?: { form?, url? }` | 668 | `elicitation: Option<ElicitationCapabilities>` with `form` + `url` sub-fields (`initialize.rs:111`) | `capabilities_shape::client_elicitation_subcapabilities_serialize_with_camelcase` | inline |
| `ClientCapabilities.extensions?: { [k]: JSONObject }` | 681 | `extensions: Option<HashMap<String, Value>>` (`initialize.rs:170`) | `client_extensions_serializes_reverse_dns_keys`, `capabilities_omit_extensions_when_none` | inline + plan §5 |
| `ServerCapabilities` | 689 | `initialize::ServerCapabilities` (`initialize.rs:227`) | `capabilities_shape::server_extensions_serializes`, `capabilities_omit_extensions_when_none` | inline |
| `ServerCapabilities.experimental` | 693 | field `experimental` | covered | inline |
| `ServerCapabilities.logging?: JSONObject` | 700 | `logging: Option<LoggingCapabilities>` (loosely typed; JsonObject is just any HashMap) | covered | inline |
| `ServerCapabilities.completions?: JSONObject` | 707 | `completions: Option<CompletionsCapabilities>` | covered | inline |
| `ServerCapabilities.prompts?: { listChanged? }` | 717 | `prompts: Option<PromptsCapabilities>` (`initialize.rs:163`) | covered | inline |
| `ServerCapabilities.resources?: { subscribe?, listChanged? }` | 738 | `resources: Option<ResourcesCapabilities>` | covered | inline |
| `ServerCapabilities.tools?: { listChanged? }` | 757 | `tools: Option<ToolsCapabilities>` (`initialize.rs:172`) | covered | inline |
| `ServerCapabilities.extensions?: { [k]: JSONObject }` | 771 | `extensions: Option<HashMap<String, Value>>` | `server_extensions_serializes` | inline + plan §5 |

**Status**: ✓ All bound. Tasks support advertised via `extensions["io.modelcontextprotocol/tasks"]` per SEP-2663 / ADR-028.

## 7. Icons + base metadata (schema lines 774–886)

| Schema | Lines | Rust binding | Test | Doc |
|--------|-------|--------------|------|-----|
| `Icon { src, mimeType?, sizes?, theme? }` | 779 | `icons::Icon` (`icons.rs:24`) | `remaining_shapes::icon_shape_matches_schema` | inline |
| `Icons` (mixin) | 823 | `icons` field on each entity that mixes it in (Tool, Resource, ResourceTemplate, Prompt, Implementation) | implicit via parent type round-trips | inline |
| `BaseMetadata { name, title? }` | 843 | Each entity that extends BaseMetadata embeds `name`+`title` directly (Implementation, Resource, ResourceTemplate, Prompt, PromptArgument, PromptReference) | per-entity tests in `remaining_shapes::*_shape_matches_schema` | inline |
| `Implementation extends BaseMetadata, Icons` | 865 | `initialize::Implementation` (`initialize.rs:12`) | `remaining_shapes::implementation_shape_matches_schema` | inline |

**Status**: ✓ All bound.

## 8. Progress (schema lines 888–932)

| Schema | Lines | Rust binding | Test | Doc |
|--------|-------|--------------|------|-----|
| `ProgressNotificationParams { progressToken, progress, total?, message? }` | 898 | `notifications::ProgressNotificationParams` (`notifications.rs:231`) | `notifications::tests::test_progress_notification`, `test_progress_token_number` | inline |
| `ProgressNotification` | 929 | `notifications::ProgressNotification` (`notifications.rs:195`) | `method_strings::notifications_progress_binding` | inline |

**Status**: ✓ Bound.

## 9. Pagination + Caching mixins (schema lines 934–997)

| Schema | Lines | Rust binding | Test | Doc |
|--------|-------|--------------|------|-----|
| `PaginatedRequestParams { cursor? }` | 943 | per-area `*Params { cursor: Option<Cursor> }` (e.g. `ListToolsParams.cursor`) | per-area tests | inline |
| `PaginatedRequest` (internal mixin) | 952 | n/a — applied at per-request level | n/a | inline |
| `PaginatedResult { nextCursor? }` | 957 | per-area `*Result { next_cursor: Option<Cursor> }` | per-area tests | inline |
| `CacheableResult { ttlMs, cacheScope }` | 970 | `caching::CacheableResult` (`caching.rs:66`) mixin struct + per-result embedded as `ttl_ms: Option<u64>` + `cache_scope: Option<CacheScope>` | `caching::tests` (9 tests) + per-area `with_cache()` tests in `tools_alignment`, `resources_alignment`, `prompts_alignment` | `caching.rs` mod doc |
| `CacheScope = "public" \| "private"` | 996 | `caching::CacheScope` enum (`caching.rs:25`) | `caching::tests::cache_scope_serializes_lowercase`, `cache_scope_parses_lowercase_only` | inline |

**Status**: ✓ All bound. Note: cache fields are transitionally `Option<…>` on per-result types pending Phase 3 finalization that tightens to required; the mixin struct itself has them required.

## 10. Resources (schema lines 999–1390)

| Schema | Lines | Rust binding | Test | Doc |
|--------|-------|--------------|------|-----|
| `ListResourcesRequest` | 1008 | `resources::ListResourcesRequest` (`resources.rs:210`) | `method_strings::resources_list_binding` | inline |
| `ListResourcesResult extends PaginatedResult, CacheableResult` | 1020 | `resources::ListResourcesResult` (`resources.rs:250`) — resultType + ttl_ms/cache_scope | `resources_alignment::list_resources_result_emits_result_type`, `list_resources_result_with_cache_produces_compliant_wire_shape`, `list_resources_result_omits_cache_fields_when_absent`, `list_resources_result_back_compat_accepts_missing_result_type` | inline |
| `ListResourcesResultResponse` | 1032 | constructed via `JsonRpcResponse::success(id, ListResourcesResult)` | implicit | inline |
| `ListResourceTemplatesRequest` | 1044 | `resources::ListResourceTemplatesRequest` (`resources.rs:508`) | `method_strings::resources_templates_list_binding` | inline |
| `ListResourceTemplatesResult` | 1056 | `resources::ListResourceTemplatesResult` (`resources.rs:533`) | `resources_alignment::list_resource_templates_result_emits_result_type_and_camelcase_key`, `list_resource_templates_result_with_cache` | inline |
| `ListResourceTemplatesResultResponse` | 1069 | via JsonRpcResponse | implicit | inline |
| `ResourceRequestParams { uri }` (internal) | 1078 | embedded as `uri` field on ReadResourceRequestParams | covered | inline |
| `ReadResourceRequestParams extends ResourceRequestParams, InputResponseRequestParams` | 1092 | `resources::ReadResourceRequestParams` (`resources.rs:313`) — has uri + input_responses + request_state | `resources_alignment::read_resource_params_input_responses_mixin_serializes`, `read_resource_params_omits_mixin_fields_when_absent` | inline |
| `ReadResourceRequest` | 1103 | `resources::ReadResourceRequest` (`resources.rs:360`) | `method_strings::resources_read_binding` | inline |
| `ReadResourceResult extends CacheableResult` | 1116 | `resources::ReadResourceResult` (`resources.rs:597`) — resultType + ttl_ms/cache_scope + contents | `resources_alignment::read_resource_result_emits_result_type`, `read_resource_result_with_cache`, `read_resource_result_back_compat_accepts_missing_result_type` | inline |
| `ReadResourceResultResponse { result: ReadResourceResult \| InputRequiredResult }` | 1131 | constructed via JsonRpcResponse — caller chooses which variant to return | implicit | inline |
| `ResourceListChangedNotification` | 1143 | `notifications::ResourceListChangedNotification` (`notifications.rs:83`) | `method_strings::notifications_resources_list_changed_binding` | inline |
| `SubscriptionFilter` | 1157 | `subscriptions::SubscriptionFilter` (`subscriptions.rs:32`) | `subscriptions::tests::filter_camelcase_field_names`, `filter_omits_absent_fields`, `filter_round_trips`, `ack_filter_can_be_subset_of_request_filter` | `subscriptions.rs` mod doc |
| `SubscriptionsListenRequestParams` | 1182 | `subscriptions::SubscriptionsListenRequestParams` (`subscriptions.rs:81`) | `subscriptions::tests::listen_request_serializes_method`, `listen_request_round_trips_from_wire_example`, `listen_params_meta_omitted_when_none` | inline |
| `SubscriptionsListenRequest` | 1201 | `subscriptions::SubscriptionsListenRequest` (`subscriptions.rs:105`) | `method_strings::subscriptions_listen_binding`, plus `subscriptions::tests::listen_method_constant_matches_schema` | inline |
| `SubscriptionsAcknowledgedNotificationParams` | 1211 | `subscriptions::SubscriptionsAcknowledgedNotificationParams` (`subscriptions.rs:127`) | `subscriptions::tests::acknowledged_notification_serializes_method` | inline |
| `SubscriptionsAcknowledgedNotification` | 1232 | `subscriptions::SubscriptionsAcknowledgedNotification` (`subscriptions.rs:146`) | `method_strings::notifications_subscriptions_acknowledged_binding`, `subscriptions::tests::acknowledged_method_constant_matches_schema` | inline |
| `ResourceUpdatedNotificationParams` | 1245 | `notifications::ResourceUpdatedNotificationParams` (`notifications.rs:289`) | `notifications::tests::test_resource_updated` | inline |
| `ResourceUpdatedNotification` | 1262 | `notifications::ResourceUpdatedNotification` (`notifications.rs:280`) | `method_strings::notifications_resources_updated_binding` | inline |
| `Resource extends BaseMetadata, Icons` | 1275 | `resources::Resource` (`resources.rs:91`) | `remaining_shapes::resource_shape_matches_schema` | inline |
| `ResourceTemplate extends BaseMetadata, Icons` | 1315 | `resources::ResourceTemplate` (`resources.rs:16`) | `remaining_shapes::resource_template_shape_matches_schema` | inline |
| `ResourceContents` (internal base) | 1348 | `resources::ResourceContents` (`resources.rs:384`) | implicit via Text/Blob round-trips | inline |
| `TextResourceContents` | 1369 | `resources::TextResourceContents` + alias in `content::TextResourceContents` | `content_alignment::embedded_resource_type_field_present` (uses text resource via JSON) | inline |
| `BlobResourceContents` | 1382 | `resources::BlobResourceContents` + alias in `content::BlobResourceContents` | implicit | inline |

**Removed (intentionally absent — confirmed via `removed_methods` test module):**
- `↳ SubscribeRequest`, `↳ SubscribeParams` — `resources/subscribe` REMOVED. Verified via `removed_methods::resources_subscribe_method_is_gone`.
- `↳ UnsubscribeRequest`, `↳ UnsubscribeParams` — `resources/unsubscribe` REMOVED. Verified via `removed_methods::resources_unsubscribe_method_is_gone`.

**Status**: ✓ All bound. Old subscribe/unsubscribe correctly deleted, replaced by `subscriptions/listen` filter.

## 11. Prompts (schema lines 1391–1590)

| Schema | Lines | Rust binding | Test | Doc |
|--------|-------|--------------|------|-----|
| `ListPromptsRequest` | 1400 | `prompts::ListPromptsRequest` (`prompts.rs:195`) | `method_strings::prompts_list_binding` | inline |
| `ListPromptsResult extends PaginatedResult, CacheableResult` | 1412 | `prompts::ListPromptsResult` (`prompts.rs:230`) — resultType + ttl_ms/cache_scope | `prompts_alignment::list_prompts_result_emits_result_type`, `list_prompts_result_with_cache`, `list_prompts_result_back_compat_accepts_missing_result_type` | inline |
| `ListPromptsResultResponse` | 1424 | via JsonRpcResponse | implicit | inline |
| `GetPromptRequestParams extends InputResponseRequestParams` | 1436 | `prompts::GetPromptRequestParams` (`prompts.rs:294`) — name + arguments (string-map) + input_responses + request_state | `prompts_alignment::get_prompt_params_arguments_is_string_map`, `get_prompt_params_input_responses_mixin_serializes`, `get_prompt_params_omits_mixin_fields_when_absent` | inline |
| `GetPromptRequest` | 1455 | `prompts::GetPromptRequest` (`prompts.rs:350`) | `method_strings::prompts_get_binding` | inline |
| `GetPromptResult extends Result` | 1468 | `prompts::GetPromptResult` (`prompts.rs:423`) | `prompts_alignment::get_prompt_result_emits_result_type`, `get_prompt_result_back_compat_accepts_missing_result_type` | inline |
| `GetPromptResultResponse { result: GetPromptResult \| InputRequiredResult }` | 1484 | via JsonRpcResponse | implicit | inline |
| `Prompt extends BaseMetadata, Icons` | 1493 | `prompts::Prompt` (`prompts.rs:39`) | `remaining_shapes::prompt_shape_matches_schema` | inline |
| `PromptArgument extends BaseMetadata` | 1512 | `prompts::PromptArgument` (`prompts.rs:110`) | `remaining_shapes::prompt_argument_shape_matches_schema` | inline |
| `Role = "user" \| "assistant"` | 1528 | `prompts::Role` (`prompts.rs:100`) — also re-exported via `sampling::Role` | `remaining_shapes::prompt_message_role_values_match_schema`, `prompt_message_only_user_and_assistant_roles`, `role_default_value_user` | inline |
| `PromptMessage { role, content: ContentBlock }` | 1538 | `prompts::PromptMessage` (`prompts.rs:379`) | `remaining_shapes::prompt_message_role_values_match_schema`, `prompt_message_only_user_and_assistant_roles` | inline |
| `ResourceLink extends Resource { type: "resource_link" }` | 1553 | variant of `content::ContentBlock` (handled via untagged enum discrimination on `type` field) | `content_alignment::resource_link_type_field_present` | inline |
| `EmbeddedResource { type: "resource", resource, annotations?, _meta? }` | 1568 | variant of `content::ContentBlock` | `content_alignment::embedded_resource_type_field_present` | inline |
| `PromptListChangedNotification` | 1587 | `notifications::PromptListChangedNotification` (`notifications.rs:160`) | `method_strings::notifications_prompts_list_changed_binding` | inline |

**Status**: ✓ All bound.

## 12. Tools (schema lines 1592–1844)

| Schema | Lines | Rust binding | Test | Doc |
|--------|-------|--------------|------|-----|
| `ListToolsRequest` | 1601 | `tools::ListToolsRequest` (`tools.rs:307`) | `method_strings::tools_list_binding`, `tools_alignment::tools_list_request_emits_correct_method` | inline |
| `ListToolsResult extends PaginatedResult, CacheableResult` | 1613 | `tools::ListToolsResult` (`tools.rs:355`) — resultType + ttl_ms/cache_scope | `tools_alignment::list_tools_result_emits_result_type`, `list_tools_result_cache_fields_omitted_when_absent`, `list_tools_result_with_cache_produces_compliant_wire_shape`, `list_tools_result_round_trips_with_cache`, `list_tools_result_back_compat_accepts_missing_result_type` | inline |
| `ListToolsResultResponse` | 1625 | via JsonRpcResponse | implicit | inline |
| `CallToolResult extends Result` | 1643 | `tools::CallToolResult` (`tools.rs:534`) — resultType + content + isError? + structuredContent (any Value) | `tools_alignment::call_tool_result_always_emits_result_type`, `structured_content_accepts_any_json_value`, `call_tool_result_back_compat_accepts_missing_result_type` | inline |
| `CallToolResultResponse { result: CallToolResult \| InputRequiredResult }` | 1682 | via JsonRpcResponse | implicit | inline |
| `CallToolRequestParams extends InputResponseRequestParams { name, arguments?, inputResponses?, requestState? }` | 1697 | `tools::CallToolRequestParams` (`tools.rs:415`) | `tools_alignment::call_tool_params_with_input_responses_mixes_in_correctly`, `call_tool_params_omits_mixin_fields_when_absent` | inline |
| `CallToolRequest` | 1716 | `tools::CallToolRequest` (`tools.rs:492`) | `method_strings::tools_call_binding`, `tools_alignment::tools_call_request_emits_correct_method` | inline |
| `ToolListChangedNotification` | 1729 | `notifications::ToolListChangedNotification` (`notifications.rs:126`) | `method_strings::notifications_tools_list_changed_binding` | inline |
| `ToolAnnotations { title?, readOnlyHint?, destructiveHint?, idempotentHint?, openWorldHint? }` | 1746 | `tools::ToolAnnotations` (`tools.rs:19`) | covered via `tools.rs::tests` and Tool shape tests | inline |
| `Tool extends BaseMetadata, Icons` with `inputSchema`, `outputSchema?` (JSON Schema 2020-12) | 1807 | `tools::Tool` (`tools.rs:182`) with `input_schema: ToolSchema` + `output_schema: Option<ToolOutputSchema>` | `tools::tests::test_tool_creation`, `test_tool_with_icons`, plus the entire `json_schema_2020_12` module (9 tests) | `tools.rs::ToolSchema` and `ToolOutputSchema` rustdoc |
| `Tool.inputSchema: { $schema?, type: "object", [k]: unknown }` | 1815 | `tools::ToolSchema` (`tools.rs:105`) — root `type:"object"` baked in, `properties: HashMap<String, Value>` accepts any 2020-12 keyword | `json_schema_2020_12::input_schema_accepts_one_of` + 7 more, plus `properties_field_accepts_2020_12_unknown_values` proving `$ref` / `oneOf` inside properties work | `tools.rs::ToolSchema` rustdoc |
| `Tool.outputSchema?: { $schema?, [k]: unknown }` | 1828 | `tools::ToolOutputSchema` (`tools.rs:158`) — separate type, NO `type` constraint | `json_schema_2020_12::output_schema_accepts_non_object_root` (verifies array root, empty schema, string root) | `tools.rs::ToolOutputSchema` rustdoc |

**Status**: ✓ All bound. Tools is the most-tested area.

## 13. Logging (schema lines 1846–1900)

| Schema | Lines | Rust binding | Test | Doc |
|--------|-------|--------------|------|-----|
| `LoggingMessageNotificationParams { level, logger?, data }` | 1856 | `notifications::LoggingMessageNotificationParams` (`notifications.rs:377`) and alias `logging::LoggingMessageParams` (`logging.rs:30`) | `notifications::tests::test_logging_message_notification` | inline |
| `LoggingMessageNotification` | 1879 | `notifications::LoggingMessageNotification` (`notifications.rs:368`) | covered via method-string DRAFT_METHODS | inline |
| `LoggingLevel = "debug"\|"info"\|"notice"\|"warning"\|"error"\|"critical"\|"alert"\|"emergency"` | 1892 | `logging::LoggingLevel` (`logging.rs:13`) | `logging::tests::test_logging_level_priority`, `request_meta::log_level_serializes_under_namespaced_key` | inline |

**Removed (intentionally absent):**
- `↳ SetLevelRequest`, `↳ SetLevelParams` — `logging/setLevel` REMOVED, replaced by `_meta.io.modelcontextprotocol/logLevel`. Verified via `removed_methods::logging_set_level_method_is_gone`.

**Status**: ✓ Bound. Removed types correctly deleted.

## 14. Sampling (schema lines 1902–2356)

| Schema | Lines | Rust binding | Test | Doc |
|--------|-------|--------------|------|-----|
| `CreateMessageRequestParams { messages, modelPreferences?, systemPrompt?, includeContext?, temperature?, maxTokens, stopSequences?, metadata?, tools?, toolChoice? }` | 1917 | `sampling::CreateMessageParams` (`sampling.rs:153`) | construction via `sampling::tests` | inline |
| `ToolChoice { mode? }` | 1968 | `sampling::ToolChoice` (`sampling.rs:96`) with `ToolChoiceMode` enum (`sampling.rs:83`) | covered via sampling tests | inline |
| `CreateMessageRequest` | 1986 | `sampling::CreateMessageRequest` (`sampling.rs:190`) | covered via method-string `DRAFT_METHODS` (`sampling/createMessage`) | inline |
| `CreateMessageResult extends SamplingMessage` | 2007 | `sampling::CreateMessageResult` (`sampling.rs:202`) | covered via sampling tests | inline |
| `SamplingMessage { role, content, _meta? }` | 2038 | `sampling::SamplingMessage` (`sampling.rs:143`) | covered | inline |
| `SamplingMessageContentBlock` (union) | 2047 | reuses `content::ContentBlock` for the common variants; ToolUse/ToolResult variants in content too | covered via `content_alignment::*` | inline |
| `Annotations { audience?, priority?, lastModified? }` | 2059 | `meta::Annotations` (`meta.rs:20`) | `remaining_shapes::annotations_shape_matches_schema` | inline |
| `ContentBlock` (union for prompts/tools: Text\|Image\|Audio\|ResourceLink\|EmbeddedResource) | 2094 | `content::ContentBlock` (`content.rs:82`) | `content_alignment` (7 tests) | `content.rs` |
| `TextContent { type:"text", text, annotations?, _meta? }` | 2109 | variant of `ContentBlock` | `content_alignment::text_content_type_field_is_text`, `text_content_with_annotations_serializes` | inline |
| `ImageContent { type:"image", data, mimeType, annotations?, _meta? }` | 2133 | variant of `ContentBlock` | `content_alignment::image_content_type_field_is_image` | inline |
| `AudioContent { type:"audio", data, mimeType, annotations?, _meta? }` | 2164 | variant of `ContentBlock` | `content_alignment::audio_content_type_field_is_audio` | inline |
| `ToolUseContent { type:"tool_use", id, name, input, _meta? }` | 2195 | variant of `ContentBlock` (also acts as SamplingMessageContentBlock variant) | round-trips via `content_alignment::content_blocks_round_trip_via_untagged_discrimination` | inline |
| `ToolResultContent { type:"tool_result", toolUseId, content, structuredContent?, isError?, _meta? }` | 2230 | variant of `ContentBlock` | same round-trip test | inline |
| `ModelPreferences { hints?, costPriority?, speedPriority?, intelligencePriority? }` | 2289 | `sampling::ModelPreferences` (`sampling.rs:65`) | covered | inline |
| `ModelHint { name? }` | 2343 | `sampling::ModelHint` (`sampling.rs:48`) | covered | inline |

**Status**: ✓ All bound.

## 15. Completion (schema lines 2358–2474)

| Schema | Lines | Rust binding | Test | Doc |
|--------|-------|--------------|------|-----|
| `CompleteRequestParams { ref, argument, context? }` | 2370 | `completion::CompleteRequestParams` (`completion.rs:63`) | `completion_alignment::complete_argument_shape` | inline |
| `CompleteRequest` | 2405 | `completion::CompleteRequest` (`completion.rs:101`) | `method_strings::DRAFT_METHODS` (`completion/complete`) | inline |
| `CompleteResult extends Result { completion: {values, total?, hasMore?} }` | 2421 | `completion::CompleteResult` (`completion.rs:144`) — resultType + completion inner | `completion_alignment::complete_result_emits_result_type`, `complete_result_back_compat_accepts_missing_result_type`, `completion_inner_field_shape`, `completion_omits_total_and_has_more_when_absent` | inline |
| `CompleteResultResponse` | 2448 | via JsonRpcResponse | implicit | inline |
| `ResourceTemplateReference { type:"ref/resource", uri }` | 2457 | `completion::ResourceTemplateReference` (`completion.rs:12`) | `completion_alignment::resource_template_reference_type_field` | inline |
| `PromptReference extends BaseMetadata { type:"ref/prompt" }` | 2472 | `completion::PromptReference` (`completion.rs:23`) | `completion_alignment::prompt_reference_type_field` | inline |

**Status**: ✓ All bound.

## 16. Roots (schema lines 2476–2538)

| Schema | Lines | Rust binding | Test | Doc |
|--------|-------|--------------|------|-----|
| `ListRootsRequest` | 2491 | `roots::ListRootsRequest` (`roots.rs:63`) | `method_strings::roots_list_binding`, `roots::tests::test_list_roots_request_matches_typescript_spec` | inline |
| `ListRootsResult { roots: Root[] }` | 2509 | `roots::ListRootsResult` (`roots.rs:74`) — NOTE: schema does NOT extend Result here (no resultType requirement) | `roots::tests::test_list_roots_result_matches_typescript_spec` | inline |
| `Root { uri, name?, _meta? }` | 2521 | `roots::Root` (`roots.rs:12`) | `roots::tests::test_root_creation` | inline |

**Removed (intentionally absent):**
- `↳ RootsListChangedNotification` / `↳ RootsListChangedParams` — `notifications/roots/list_changed` REMOVED. Verified via `removed_methods::roots_list_changed_notification_is_gone`.

**Status**: ✓ Bound. Note that `ListRootsResult` per schema lines 2509 intentionally does NOT extend `Result`, so it does not carry `resultType` — this is a schema asymmetry, not a Rust bug.

## 17. Elicitation (schema lines 2540–2935)

| Schema | Lines | Rust binding | Test | Doc |
|--------|-------|--------------|------|-----|
| `ElicitRequestFormParams { mode?:"form", message, requestedSchema }` | 2551 | folded into `elicitation::ElicitCreateParams` (`elicitation.rs:301`) | covered via elicitation tests | inline |
| `ElicitRequestURLParams { mode:"url", message, elicitationId, url }` | 2584 | NOTE: not separately bound; the URL-mode flow is supported via the form-mode params struct extension and the `notifications/elicitation/complete` lifecycle. Schema's discriminated union (Form\|URL) is a wire-level discrimination that the current crate models loosely | n/a (see Phase 3.4 in plan doc) | inline |
| `ElicitRequestParams` (union) | 2614 | n/a (handled per-variant above) | n/a | inline |
| `ElicitRequest` | 2626 | `elicitation::ElicitCreateRequest` (`elicitation.rs:314`) | `method_strings::DRAFT_METHODS` (`elicitation/create`) | inline |
| `PrimitiveSchemaDefinition` (union) | 2637 | `elicitation::PrimitiveSchemaDefinition` (`elicitation.rs:269`) | covered | inline |
| `StringSchema` | 2649 | `elicitation::StringSchema` (`elicitation.rs:13`) | covered | inline |
| `NumberSchema` | 2665 | `elicitation::NumberSchema` (`elicitation.rs:33`) | covered | inline |
| `BooleanSchema` | 2680 | `elicitation::BooleanSchema` (`elicitation.rs:51`) | covered | inline |
| `UntitledSingleSelectEnumSchema { type:"string", enum:[], default? }` | 2695 | `elicitation::UntitledSingleSelectEnumSchema` (`elicitation.rs:90`) | `elicitation_enum_schemas::untitled_single_select_wire_shape`, `untitled_single_select_omits_optional_fields_when_none`, `enum_schemas_round_trip` | inline |
| `TitledSingleSelectEnumSchema { type:"string", oneOf:[{const, title}], default? }` | 2723 | `elicitation::TitledSingleSelectEnumSchema` (`elicitation.rs:119`) + `TitledEnumOption` (`elicitation.rs:135`) | `elicitation_enum_schemas::titled_single_select_wire_shape`, `titled_enum_option_camelcase_const_key`, `schema_examples_round_trip` | inline |
| `SingleSelectEnumSchema` (union) | 2756 | `elicitation::SingleSelectEnumSchema` untagged enum (`elicitation.rs:167`) | covered via enum_schemas_round_trip | inline |
| `UntitledMultiSelectEnumSchema { type:"array", items:{type:"string", enum}, minItems?, maxItems?, default? }` | 2768 | `elicitation::UntitledMultiSelectEnumSchema` (`elicitation.rs:175`) + `UntitledMultiSelectItems` (`elicitation.rs:193`) | `elicitation_enum_schemas::untitled_multi_select_wire_shape` | inline |
| `TitledMultiSelectEnumSchema { type:"array", items:{anyOf:[{const, title}]}, ... }` | 2810 | `elicitation::TitledMultiSelectEnumSchema` (`elicitation.rs:220`) + `TitledMultiSelectItems` (`elicitation.rs:238`) | `elicitation_enum_schemas::titled_multi_select_wire_shape` | inline |
| `MultiSelectEnumSchema` (union) | 2856 | `elicitation::MultiSelectEnumSchema` (`elicitation.rs:260`) | covered | inline |
| `LegacyTitledEnumSchema` | 2866 | `elicitation::EnumSchema` (`elicitation.rs:70`) — the legacy single-struct form is kept here | doctest in mod | inline |
| `EnumSchema` (union) | 2883 | n/a as a single union type; the variants above cover each shape | n/a | inline |
| `ElicitResult { action, content? }` | 2902 | `elicitation::ElicitResult` (`elicitation.rs:447`) + `ElicitAction` (`elicitation.rs:435`) | covered via elicitation tests | inline |
| `ElicitationCompleteNotification` | 2927 | `notifications::ElicitationCompleteNotification` (`notifications.rs:423`) | covered via method-string `DRAFT_METHODS` + `notifications::tests::test_elicitation_complete_notification` | inline |

**Status**: ✓ Mostly bound. ⚠ One gap: `ElicitRequestURLParams` (schema 2584) discrimination is loosely modeled; URL-mode discrimination via the `mode` field doesn't have a strict tagged-union test. This is a Phase 3.4 partial-coverage item.

## 18. Top-level union types (schema lines 2937–2983)

| Schema | Lines | Rust binding | Test | Doc |
|--------|-------|--------------|------|-----|
| `ClientRequest` (union) | 2939 | not a single Rust union; verified via `method_strings::DRAFT_METHODS` enumerating all 10 client-request method strings | `method_strings::every_listed_method_appears_in_schema`, `schema_method_count_matches_canonical_list` | inline |
| `ClientNotification` (union: Cancelled\|Progress) | 2952 | covered via individual notification structs + DRAFT_METHODS | same | inline |
| `ClientResult = EmptyResult` | 2955 | `ping::EmptyResult` | `empty_result_alignment` (3 tests) | inline |
| `ServerNotification` (union, 9 variants) | 2960 | covered via 9 individual notification structs + DRAFT_METHODS enumeration | `method_strings::*_binding` for each | inline |
| `ServerResult` (union, 11 variants incl. InputRequired) | 2972 | covered via individual `*Result` types | per-area `*_alignment` modules | inline |

**Status**: ✓ All bound via the exhaustive `method_strings` test (count-pin + per-binding shape checks). The schema's wide unions aren't single Rust enums but are covered via the dual approach: (a) every method-string present in the canonical list is asserted against the schema, (b) every Rust constructor for each method produces the right wire string.

---

## Compliance test module summary

`crates/turul-mcp-protocol-2026-07-28/src/compliance_test.rs` contains 17 dedicated test modules organized by phase:

| Module | Phase | Schema range | Tests |
|--------|-------|--------------|-------|
| `tests` (legacy carryover) | — | mixed | 8 (carryover from 2025-11-25, still pass) |
| `json_schema_2020_12` | 6 | 1815–1834 (Tool schemas) | 9 |
| `remaining_shapes` | 9 | 779–886, 1275–1521, 2059–2089 (Icon/Implementation/Resource/Prompt/Annotations) | 11 |
| `completion_alignment` | 3.5 | 2358–2474 | 6 |
| `empty_result_alignment` | 3.6 | 435 + ping deletion | 3 |
| `content_alignment` | 3.7 | 2094–2269 (ContentBlock variants) | 7 |
| `elicitation_enum_schemas` | 3.4 | 2687–2886 | 8 |
| `prompts_alignment` | 3.3 | 1391–1590 | 8 |
| `resources_alignment` | 3.2 | 999–1390 | 11 |
| `tools_alignment` | 3.1 | 1601–1844 | 12 |
| `capabilities_shape` | 2.3 | 623–772 | 7 |
| `method_strings` | 8 | enumeration | 18 (16 binding + 2 schema cross-check) |
| `removed_methods` | 2.4 | absence assertions | 10 |
| `request_meta` | 1.2 | 70–107 | 8 |
| `result_discrimination` | 1.3 | 157–185 | 3 |
| `multi_round_trip` | 1.5 | 437–514 | 3 |
| `envelope` | 1.1 | 26–258 | 9 |
| `error_codes` | 1.4 | 261–427 | 7 |
| **Total compliance tests** | — | — | **148** |

Plus per-module unit tests: 140 across `tools::tests`, `resources::tests`, `prompts::tests`, etc. **Grand total: 288 tests passing.**

---

## Documentation map

| Layer | Where | Coverage |
|-------|-------|----------|
| **Source-line rustdoc** | every `pub` type in `src/*.rs` references the schema line range it implements | Inline `///` comments cite schema lines (e.g. "Schema lines 70–107") |
| **Module rustdoc** | top of each `src/*.rs` | each module summarizes which schema area it covers |
| **Crate README** | `crates/turul-mcp-protocol-2026-07-28/README.md` | scaffold status + planned-SEPs roadmap |
| **Vendored schema** | `crates/turul-mcp-protocol-2026-07-28/schema/draft-schema.ts` + `schema/README.md` | ETag-pinned provenance + regeneration instructions |
| **Migration diff** | `docs/plans/2026-07-28-migration-diff.md` | 2025-11-25 → DRAFT-2026-v1 per-file diff |
| **Compliance plan** | `docs/plans/2026-07-28-compliance-plan.md` | every Phase with current checkbox state |
| **Coverage matrix** | `docs/plans/2026-07-28-schema-coverage-matrix.md` (THIS DOC) | per-type schema→rust→test→doc mapping |
| **ADR-027** | `docs/adr/027-targeting-mcp-draft-2026-v1.md` | wire-string target + regeneration trigger |
| **ADR-028** | `docs/adr/028-extensions-strategy.md` | how extensions are hosted: separate `turul-mcp-ext-*` crates mirroring upstream `ext-*` repos. SEP-2133 + SEP-2663 content verified via `gh api`. |
| **Branch lock** | `CLAUDE.md` + `AGENTS.md` "Branch Lock" sections | merge-protection on `2026-07-28-MCP-Specification` |
| **Frozen-crate rule** | `CLAUDE.md` + `AGENTS.md` "Frozen Protocol Crates" sections | `2025-11-25` and `2025-06-18` are immutable |

---

## Open compliance items (be explicit about what's NOT yet covered)

| Item | Status | Where tracked |
|------|--------|---------------|
| ADR-028 — Extensions strategy | ✓ landed | `docs/adr/028-extensions-strategy.md` |
| Tasks extension (`io.modelcontextprotocol/tasks`) | TODO Phase 5.2 — scaffold `turul-mcp-ext-tasks-2026-07-28` per ADR-028 | plan doc Phase 5.2 |
| MCP Apps extension (SEP-1865) | TODO Phase 5.3 — scaffold `turul-mcp-ext-apps-2026-07-28` per ADR-028 | plan doc Phase 5.3 |
| Tighten CacheableResult fields from `Option<…>` to required | TODO Phase 3.x finalization | per-result rustdoc notes |
| Tighten `RequestParams._meta` from `Option<Meta>` to required `RequestMetaObject` | TODO Phase 1.1b | `json_rpc.rs` "Known divergences" |
| Split `JsonRpcResponse` into separate `ResultResponse`/`ErrorResponse` types | TODO Phase 1.1b | `json_rpc.rs` "Known divergences" |
| Type `RequestId` as `String\|Number` enum (currently `serde_json::Value`) | TODO Phase 1.1b | `json_rpc.rs` "Known divergences" |
| `ElicitRequestURLParams` discriminated-union testing | partial coverage | plan doc Phase 3.4 |
| Flip alias `turul-mcp-protocol` to depend on `2026-07-28` instead of `2025-11-25` | TODO Phase 9.4 | plan doc Phase 9.4 |

**None of these blocks claiming "DRAFT-2026-v1 schema compliance" for the types this crate exposes** — they're refinements and extension-territory work.

---

## Changelog coverage — `modelcontextprotocol.io/specification/draft/changelog`

Cross-check against the upstream changelog ("Key Changes" since 2025-11-25). Every numbered item is mapped to its coverage here.

### Major changes (8 items)

| # | Changelog item | SEP | Coverage in this crate |
|---|----------------|-----|------------------------|
| 1 | Remove sessions / `Mcp-Session-Id`; list endpoints don't vary per-connection; server-minted handles as tool args | SEP-2567 | ✓ Schema doesn't define session-id; tracked transport-side in `turul-http-mcp-server`. Header constants exported in [`headers`] for canonical spelling. Drift detector `removed_methods::*_method_is_gone` covers method-level removals. |
| 2 | Stateless: remove `initialize`/`notifications/initialized` handshake; per-request `_meta` carries protocol version + client info + client capabilities; `UnsupportedProtocolVersionError` | SEP-2575 | ✓ `meta::RequestMetaObject` (required named keys). `InitializeRequest`/`Result` and `InitializedNotification` not in crate (and `removed_methods` test pins absence). `McpError::UnsupportedProtocolVersion` → wire code `-32004`. 8 tests in `request_meta`, 10 in `removed_methods`. |
| 3 | Add `server/discover` — REQUIRED on server | SEP-2575 | ✓ `discover::DiscoverRequest`/`DiscoverResult`/`DiscoverResultResponse`. 9 tests in `discover::tests` + `method_strings::server_discover_binding`. |
| 4 | Replace HTTP GET + `resources/subscribe`/`unsubscribe` with `subscriptions/listen`; opt-in filter; ack with `io.modelcontextprotocol/subscriptionId` tag | SEP-2575 | ✓ `subscriptions::SubscriptionsListenRequest` + `SubscriptionFilter` (toolsListChanged/promptsListChanged/resourcesListChanged/resourceSubscriptions). `SubscriptionsAcknowledgedNotification` for the ack. `META_KEY_SUBSCRIPTION_ID` exported as `meta::META_KEY_SUBSCRIPTION_ID`. 10 tests in `subscriptions::tests` + 4 in `convention_meta_keys`. |
| 5 | Remove `ping`, `logging/setLevel`, `notifications/roots/list_changed`; log level now in `_meta.io.modelcontextprotocol/logLevel` | SEP-2575 | ✓ All three methods absent from crate; `removed_methods::{ping,logging_set_level,roots_list_changed}_method_is_gone` pin absence. `RequestMetaObject.log_level` field carries the new opt-in (schema line 106). `META_KEY_LOG_LEVEL` constant exported. |
| 6 | Move tasks to extension `io.modelcontextprotocol/tasks`; new lifecycle (`tasks/get`/`update`/`cancel`); no `tasks/list`; unsolicited handles allowed | SEP-2663 | ✓ Tasks not in core crate. `removed_methods::tasks_methods_are_gone_from_core` pins absence of all `tasks/*` methods in core schema. ADR-028 documents the extension strategy (separate `turul-mcp-ext-tasks-2026-07-28` crate). |
| 7 | Multi Round-Trip Requests (MRTR): server returns `inputRequests`, client retries with `inputResponses` | SEP-2322 | ✓ `input_required` module: `InputRequest`/`InputResponse`/`InputRequests`/`InputResponses`/`InputRequiredResult`/`InputResponseRequestParams` mixin. 9 unit + 3 multi-round-trip flow tests + mixin usage in `tools_alignment`/`resources_alignment`/`prompts_alignment`. |
| 8 | Deprecate Roots, Sampling, Logging (12-month soft-deprecation) | SEP-2577 | ✓ Types remain functional per the 12-month window. No `#[deprecated]` markers added — soft-deprecation is spec-level only; consumers shouldn't get warnings until the window closes. Plan §4.1–4.3. |

### Minor changes (6 items)

| # | Changelog item | SEP | Coverage |
|---|----------------|-----|----------|
| 1 | `extensions` field on `ClientCapabilities` and `ServerCapabilities` | SEP-2133 | ✓ Both struct fields present (`initialize.rs:170` client, `initialize.rs:262` server). ADR-028 documents the strategy. 2 compliance tests in `capabilities_shape::{client,server}_extensions_serializes`. |
| 2 | Document OpenTelemetry `_meta` keys: `traceparent`, `tracestate`, `baggage` | SEP-414 | ✓ Conventional `_meta` keys not schema-declared. Constants `META_KEY_TRACEPARENT`/`TRACESTATE`/`BAGGAGE` exported from `meta`; values flow through `RequestMetaObject.extra: HashMap<String, Value>` flatten. Pinned in `convention_meta_keys::tracing_keys_use_w3c_unprefixed_spelling`. |
| 3 | Servers SHOULD return tools/list in deterministic order | (server behavior) | N/A for protocol crate — this is a server-side recommendation. Belongs in `turul-mcp-server` framework guidance. |
| 4 | Required HTTP headers (`Mcp-Method`, `Mcp-Name`, `MCP-Protocol-Version`) + `x-mcp-header` custom-header prefix | SEP-2243 | ✓ Header name constants exported from new [`headers`] module: `HTTP_HEADER_METHOD`, `HTTP_HEADER_NAME`, `HTTP_HEADER_PROTOCOL_VERSION`, `HTTP_HEADER_CUSTOM_PREFIX`. Pinned in `headers::tests::header_names_exact_spelling`. Actual enforcement on the wire lives in `turul-http-mcp-server`. |
| 5 | Require `ttlMs` and `cacheScope` on list/read results (CacheableResult mixin) | SEP-2549 | ✓ `caching::CacheableResult` mixin types defined (`ttl_ms: u64`, `cache_scope: CacheScope`). **Required (not optional) fields** on `ListToolsResult`, `ListResourcesResult`, `ListResourceTemplatesResult`, `ListPromptsResult`, `ReadResourceResult` — each constructor defaults to `(0, Public)` (immediately-stale public). 9 caching tests + per-result tests (e.g. `tools_alignment::list_tools_result_emits_required_cache_fields_with_defaults`). |
| 6 | Resource-not-found error code: `-32002` → `-32602` (Invalid Params) | SEP-2164 | ✓ `McpError::{Tool,Resource,Prompt}NotFound` all map to `-32602` per Phase 1.4. 3 dedicated tests in `error_codes::{tool,resource,prompt}_not_found_maps_to_invalid_params`. Drift detector `no_unauthorised_error_codes_emitted` blocks regression to the old code. |

### Other schema changes / Governance / Process

- **Other schema changes**: changelog says N/A.
- **Governance updates**: changelog says N/A.
- **Process changes** (SEP-1850 PR-based SEP workflow): N/A for this crate — repo-process change, not a protocol type.

### Full changelog reference

Diff link from the changelog: <https://github.com/modelcontextprotocol/specification/compare/2025-11-25...draft>. If new commits land that aren't covered above, the `removed_methods::schema_method_count_matches_canonical_list` test (Phase 8) will catch any new method strings and the schema-vs-Rust binding gap will surface on `cargo test`.

---

## How to use this matrix

1. **Adding a new compliance test**: find the schema lines for the type, add the row to the matrix, then write the test in the appropriate `compliance_test.rs::<area>_alignment` module.
2. **Re-vendoring the schema**: after `curl` to update `schema/draft-schema.ts`, walk this matrix and confirm each row's line ranges still match. Adjust ranges, add new rows for new TS exports, mark removed rows as "removed via Phase X."
3. **Reviewing a PR that touches a protocol type**: cross-check the modified type's row here to ensure the test and doc both still cover it.
