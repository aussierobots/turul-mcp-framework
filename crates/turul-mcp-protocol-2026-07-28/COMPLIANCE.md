# MCP 2026-07-28 Compliance Report

`turul-mcp-protocol-2026-07-28` against the vendored MCP schema (`schema/draft/`
upstream path; finalized wire string `"2026-07-28"`).

## Pin

- **Schema source**: `modelcontextprotocol/modelcontextprotocol` @ `schema/draft/schema.ts`
- **Vendored copy**: `crates/turul-mcp-protocol-2026-07-28/schema/draft-schema.ts`
- **Fixture pin (commit SHA)**: `60dc69e9a9723a7bab535ade6c5c5b9695d97dfc` — see `schema/EXAMPLES_PIN.md`
- **Captured**: 2026-07-02 (re-vendored; error-code renumbering, cancellation `requestId` required, `ElicitationCompleteNotification` removal, subscription-stream metadata contracts. Prior cuts: 2026-06-10, 2026-06-07, 2026-05-24)
- **Schema content sha256**: `6e4cba2d17f7156877357762b6b4b63cd790d8973f61ec35ab73cd61ad67017d` (was `1bf94a601817ab07fc04058a9ff2e031227f9b9384e198ea7f187e75eb4b9ec6`)
- **Schema surface**: 126 `export interface` + 27 `export type` + 10 `export const` = 163 declarations (the 2026-07-02 re-vendor removed `ElicitationCompleteNotification`/`ElicitationCompleteNotificationParams` (2 interfaces) and added `HeaderMismatchError`, `NotificationMetaObject`, `SubscriptionsListenResultMeta`, `SubscriptionsListenResult` (4 interfaces) plus the `HEADER_MISMATCH` const; see `docs/adr/027-targeting-mcp-draft-2026-v1.md` revision log for the full diff)
- **Upstream MCP version string**: `"2026-07-28"` (finalized; was `"DRAFT-2026-v1"` — accepted on deserialize for back-compat. See `docs/adr/027`)

## Known gaps (2026-07-02 re-pin)

- **`SubscriptionsListenResult` server-side emission is unwired.** The type
  (`subscriptions::SubscriptionsListenResult` / `SubscriptionsListenResultMeta`)
  is bound and wire-tested (`subscriptions::tests::listen_result_*`), but
  `turul-http-mcp-server`'s `handle_subscriptions_listen` has no graceful
  server-shutdown hook to emit it from — the schema explicitly permits this
  (the result is sent "only when the server tears the subscription down…an
  abrupt transport close carries no response"), so the gap is spec-legal but
  should be closed when a shutdown-signal path is added to the SSE stream
  handler. Tracked here rather than silently claimed as wired. Confirmed no
  shutdown-signal infrastructure (no `CancellationToken`, no `tokio::signal`
  wiring) exists anywhere in `turul-http-mcp-server` or `turul-mcp-server` to
  hook into — this is unbuilt transport infrastructure, not a missed call site.
- **`RequestMetaObject.extra` has the same reserved-key-collision risk fixed
  on `SubscriptionsListenResultMeta` below** (public `#[serde(flatten)]` map
  alongside named typed fields — a caller can insert
  `io.modelcontextprotocol/protocolVersion` etc. into `extra` and collide with
  the typed field, producing a duplicate wire key). Pre-existing, not
  introduced by the 2026-07-02 slice; out of scope here — flagged for a
  dedicated pass across all `extra`/flatten-map fields in the crate.
- **Server still dispatches an inbound `notifications/progress` from the
  client.** The schema narrowed `ClientNotification` from
  `CancelledNotification | ProgressNotification` to `CancelledNotification`
  only — clients may no longer send progress notifications to the server.
  `turul-mcp-server`'s dispatch table (`builder.rs` `handlers.insert("notifications/progress", ...)`)
  routes it to a no-op logger (`NotificationsHandler::handle`) on BOTH lanes,
  unconditionally. The handler also accepts `notifications/message`, which
  was never a `ClientNotification` member on any pin — this looks like
  deliberate lenient any-notification handling, not a strict binding to the
  schema union, so a partial fix (gating only `notifications/progress`)
  would be inconsistent with the handler's evident design intent. No wire
  emission is affected (accept-side leniency only); flagged for a dedicated
  slice that decides whether the dispatch table should enforce
  `ClientNotification` at all.

## Test gate

| Surface | Count | Status |
|---|---|---|
| Lib unit tests (`--features compliance`) | 185 | ✅ pass |
| Lib unit tests (default features) | 178 | ✅ pass |
| `tests/compliance.rs` integration | 187 | ✅ pass |
| `tests/upstream_fixtures.rs` harness | 3 | ✅ pass |
| Doctests | 1 (+ 2 ignored) | ✅ pass |
| **Total (`--features compliance`)** | **420** | ✅ all green, `clippy -D warnings` clean |
| Total (default features) | 409 | ✅ — `compliance` feature adds the 7 lib + 3 fixture wire-gate tests |
| `mcp-compliance-2026-07-28` binary | 22/22 fixtures | ✅ all pass |
| Modeled fixtures | 10 of 87 (11.5%) | ⚠ partial — see §Coverage below |

Verified on `turul-rpc 0.2.2` (with `turul-rpc-jsonrpc 0.2.2` for the `frame` module fix).

## Wire envelope conformance (JSON-RPC §5)

Wire types re-exported from `turul-rpc` 0.2.2:

| Schema interface | Rust re-export | Status |
|---|---|---|
| `JSONRPCRequest` | `turul_rpc::JsonRpcRequest` | ✅ |
| `JSONRPCNotification` | `turul_rpc::JsonRpcNotification` | ✅ |
| `JSONRPCResultResponse` | `turul_rpc::JsonRpcSuccessResponse` | ✅ |
| `JSONRPCErrorResponse` (`id?: RequestId`) | `turul_rpc::JsonRpcError` | ✅ id is `Option<RequestId>` |
| `JSONRPCResponse = Success \| Error` union | `turul_rpc::JsonRpcResponse` (untagged enum) | ✅ |
| `JSONRPCMessage = Request \| Notification \| Response` | `turul_rpc::JsonRpcWireMessage` (new in 0.2.2) | ✅ |
| `Error { code, message, data? }` | `turul_rpc::error::JsonRpcErrorObject` | ✅ |
| `RequestId = string \| number` | `turul_rpc::RequestId` (typed enum) | ✅ |
| `JSONRPC_VERSION = "2.0"` | `turul_rpc::JsonRpcVersion` (typed) + `JSONRPC_VERSION` const | ✅ |

## `_meta` carriers

| Schema | Rust | Status |
|---|---|---|
| `MetaObject = Record<string, unknown>` | `meta::MetaObject = HashMap<String, Value>` | ✅ |
| `RequestMetaObject extends MetaObject` (5 named fields, 3 required namespaced) | `meta::RequestMetaObject` typed struct + `extra: HashMap` flatten | ✅ |
| `RequestParams._meta: RequestMetaObject` (REQUIRED) | `json_rpc::RequestParams.meta: RequestMetaObject` (not `Option`) | ✅ |
| `extends RequestParams` (per-RPC) — every extender carries the same typed required `_meta` | `CallToolRequestParams`, `PaginatedRequestParams`, `ReadResourceRequestParams`, `GetPromptRequestParams`, `CompleteRequestParams`, `SubscriptionsListenRequestParams` — all `meta: RequestMetaObject` | ✅ |
| `NotificationParams._meta?: NotificationMetaObject` | `notifications::NotificationParams.meta: Option<MetaObject>` via `HasNotificationMeta`; the `subscriptionId` key is stamped at the transport | ⚠ structural-only — see §Intentional deviations |
| `Result._meta?: ResultMetaObject` | per-result struct: `meta: Option<ResultMetaObject>` (all 12 result types); `HasMeta::meta()` returns `Option<&ResultMetaObject>` | ✅ |
| Required keys: `io.modelcontextprotocol/protocolVersion`, `clientCapabilities` (`clientInfo` is OPTIONAL) | typed named fields with `#[serde(rename = "io.modelcontextprotocol/…")]`; `client_info: Option<Implementation>` | ✅ |
| Optional keys: `progressToken?`, `io.modelcontextprotocol/logLevel?` | typed `Option<ProgressToken>` / `Option<LoggingLevel>` | ✅ |

## Method-string conformance (22 schema methods)

All 22 schema-declared method strings are present in the crate at their canonical wire spelling:

```
completion/complete       prompts/get               server/discover
elicitation/create        prompts/list              subscriptions/listen
resources/list            resources/read            tools/call
resources/templates/list  roots/list                tools/list
sampling/createMessage

notifications/cancelled                notifications/progress
notifications/elicitation/complete     notifications/prompts/list_changed
notifications/message                  notifications/resources/list_changed
notifications/resources/updated        notifications/subscriptions/acknowledged
notifications/tools/list_changed
```

**Spec-correct underscores in `list_changed` / `list_updated` forms** (DRAFT-2026-v1 uses underscores, not camelCase `listChanged`).

No method strings outside the canonical 22 are declared anywhere in the crate. Earlier compat traits for the removed `initialize` handshake and `notifications/roots/list_changed` have been deleted in keeping with Protocol Crate Purity (schema-only API). The crate also has no surviving `pub const *_METHOD: &str` entries for removed methods (`initialize`, `ping`, `logging/setLevel`, `notifications/roots/list_changed`); only the canonical method strings are declared (see `grep -rn 'pub const.*METHOD' src/`).

## Symbol coverage matrix

123 + 27 = 150 schema interface/type symbols. Sampled coverage by category (full per-symbol table available via the `mcp-compliance-2026-07-28` binary's CASES table):

| Category | Symbols | Bound in Rust | Wire-tested via fixtures |
|---|---|---|---|
| Wire envelopes (Request/Notification/Response/Error/Message) | 7 | 7 (via `turul-rpc`) | 0 (no upstream fixture) |
| `*Params` request shapes | 9 | 9 | 1 (CallToolRequestParams) |
| Result interfaces | 14 | 14 | 3 (CallToolResult, ListToolsResult, ListRootsResult, ElicitResult) |
| `*Response` envelope unions (e.g. `CallToolResultResponse`) | 9 | 1 (DiscoverResultResponse) | 0 |
| Notification interfaces | 9 | 9 | 0 |
| Notification params | 5 | 5 | 0 |
| Content blocks (Text/Image/Audio/ToolUse/ToolResult/ResourceLink/EmbeddedResource) | 7 | 7 (enum variants in `ContentBlock`) | 0 |
| Errors (Parse/InvalidRequest/MethodNotFound/InvalidParams/Internal/MissingRequired/UnsupportedProtocolVersion) | 7 | 1 (via `JsonRpcErrorObject` factory methods) | 0 |
| Cacheable/Paginated mixins | 2 | 2 | 0 |
| Elicitation schema variants (Untitled/Titled, Single/Multi-select) | 4 | 4 | 0 |
| Primitive JSON Schema (Boolean/Number/String/Enum) | 4 | 4 | 0 |
| Constants (`JSONRPC_VERSION`, `LATEST_PROTOCOL_VERSION`, error codes ×7) | 9 | 9 | n/a |
| Schema-author types (`Request`, `Notification`, `Result`, `ClientRequest`, `ServerRequest`, etc. — TS-only unions) | ~10 | not bound (Rust traits cover this) | n/a |
| **Total** | ~150 | ~150 | 22 file-level wire tests (10 modeled cases) |

## Wire-field name conformance (camelCase via serde)

Spot-checked high-risk fields — all serde renames match schema exactly:

| Wire name | Rust field | Source |
|---|---|---|
| `inputSchema` | `tools::Tool.input_schema` | ✅ |
| `outputSchema` | `tools::Tool.output_schema` | ✅ |
| `mimeType` | `mime_type` on Resource/Content/etc. | ✅ |
| `nextCursor` | `next_cursor` on `*Result` | ✅ |
| `ttlMs` / `cacheScope` | `caching::CacheableResult.{ttl_ms, cache_scope}` | ✅ |
| `resultType` | `result_type::ResultType` typed enum | ✅ |
| `progressToken` (`= string \| number`) | `meta::ProgressToken` untagged enum `String(String) \| Number(serde_json::Number)` — `serde_json::Number` losslessly preserves any JSON number (int, float, large) at both `RequestMetaObject.progressToken?` and `ProgressNotificationParams.progressToken` | ✅ |
| `structuredContent` | `tools::CallToolResult.structured_content` | ✅ |
| `toolUseId` | `content::ContentBlock::ToolResult { tool_use_id }` | ✅ |
| `isError` | `tools::CallToolResult.is_error` | ✅ |
| `readOnlyHint` / `destructiveHint` / `idempotentHint` / `openWorldHint` | `tools::ToolAnnotations.{...}` | ✅ |
| `costPriority` / `speedPriority` / `intelligencePriority` | `sampling::ModelPreferences.{...}` | ✅ |
| `requestState` | `input_required::InputRequiredResult.request_state` | ✅ |
| `inputRequests` / `inputResponses` | typed maps in `input_required` | ✅ |
| `io.modelcontextprotocol/protocolVersion` (etc.) | `meta::RequestMetaObject.protocol_version` (rename) | ✅ |
| `io.modelcontextprotocol/subscriptionId` | `meta::META_KEY_SUBSCRIPTION_ID` on `NotificationMetaObject` / `SubscriptionsListenResultMeta` (2026-07-02 re-pin — now schema-declared, was previously a documented convention) | ✅ |

## Spec `@see` anchor coverage

8 `@see` block-tags in `schema/draft-schema.ts`:

| # | Schema anchor | Rust binding | Status |
|---|---|---|---|
| 1 | `[General fields: _meta](/specification/draft/basic/index#meta)` | `meta::MetaObject` | ✅ mirrored |
| 2 | `[General fields: _meta]` (same) | `meta::RequestMetaObject` | ✅ mirrored |
| 3 | TypeDoc `{@link MetaObject}` cross-ref | `meta::RequestMetaObject` | ✅ uses `[`MetaObject`]` intra-doc link |
| 4 | `[JSON-RPC 2.0 Error Object](https://www.jsonrpc.org/specification#error_object)` (ParseError) | `json_rpc::JsonRpcError` parent doc | ✅ mirrored |
| 5–8 | Same JSON-RPC anchor on `InvalidRequestError`, `MethodNotFoundError`, `InvalidParamsError`, `InternalError` | All factory methods → `JsonRpcError` | ✅ collapsed onto parent struct doc |

Anchors are URL fragments (section IDs) — they survive re-pins. Schema line numbers do not, and are not used as comment anchors anywhere in this crate's `src/` or `tests/` directories.

## Compliance harness

Bidirectional wire-format gate against the upstream's canonical example JSON fixtures (`schema/draft/examples/`, 86 directories, 124 fixture files):

- **Build-time** — `cargo test -p turul-mcp-protocol-2026-07-28 --features compliance --test upstream_fixtures` drives every modeled `Case` against every `.json` file in its directory; asserts semantic-diff equality after parse → re-serialize.
- **Runtime** — `cargo run -p turul-mcp-protocol-2026-07-28 --features compliance --bin mcp-compliance-2026-07-28` calls the same `compliance::roundtrip::run_all` path. Green tests ⇒ green binary on the same pin.
- **Floor** — `tests/upstream_fixtures.rs::COVERAGE_FLOOR = 8`. Modeled cases:

  | Case | Files | Status |
  |---|---|---|
  | `Tool` | 6 | ✅ |
  | `CallToolRequestParams` | 2 | ✅ |
  | `CallToolResult` | 3 | ✅ |
  | `ListToolsResult` | 1 | ✅ |
  | `Resource` | 1 | ✅ |
  | `Root` | 1 | ✅ |
  | `ListRootsResult` | 2 | ✅ |
  | `ElicitResult` | 3 | ✅ |
  | **Total** | **20/20** | **✅** |

- **77 remaining cases** marked `Kind::NotModeled` — wave-by-wave migration to be raised by deliberate PR. A green run means only "the modeled subset matches": `DiscoverResult`/`DiscoverResultResponse` were unmodeled when upstream #3002 changed them, so the harness reported `failed=0` across the whole 12-day drift window. Read the `modeled=N` line on every run.

## Schema-fidelity corrections (Slice A' follow-up, 2026-05-31)

Defects surfaced by an internal review and verified against the pinned schema. Each shipped with a regression test that asserts the corrected wire shape.

| Defect | Resolution | Test |
|---|---|---|
| `meta::ProgressToken` was a `String` newtype while the schema declares `ProgressToken = string \| number`. `notifications::ProgressTokenValue` had the correct enum on the notification side — two non-interoperable bindings of one schema type. A spec-valid `_meta.progressToken: 42` failed to deserialize through `RequestMetaObject`. Initial fix used `Number(i64)` which still rejected JSON floats like `1.5` (codex P1 follow-up review, 2026-05-31). | Final form: untagged enum `ProgressToken { String(String), Number(serde_json::Number) }`; `notifications::ProgressTokenValue` aliases it; both carriers use the unified type. `From<i64>` / `From<u64>` / `From<i32>` / `From<f64>` impls + `as_i64` / `as_f64` / `as_number` / `as_str` accessors. `From<f64>` panics on NaN/±Inf (no JSON representation). | `meta::tests::test_progress_token_integer_round_trips`, `meta::tests::test_progress_token_float_round_trips`, `meta::tests::test_progress_token_negative_and_large_round_trip`, `meta::tests::test_progress_token_deserializes_from_both_shapes`, `meta::tests::test_progress_token_from_nan_panics`, `notifications::tests::test_request_meta_accepts_numeric_progress_token` |
| `SubscriptionsListenRequestParams._meta: Option<HashMap>` while the schema says `extends RequestParams` ⇒ required typed `RequestMetaObject`. The lone outlier among `RequestParams` extenders. | Changed to required `meta: RequestMetaObject`; `new(filter, meta)` and `SubscriptionsListenRequest::new(filter, meta)` signatures updated. The companion `SubscriptionsAcknowledgedNotificationParams.meta` correctly stays `Option<HashMap>` (extends `NotificationParams`). | `subscriptions::tests::listen_params_meta_required_on_wire`, `subscriptions::tests::listen_request_rejects_missing_meta` |
| `CancelledNotificationParams.request_id: RequestId` (required) while the schema declared it optional (`requestId?`) as of the 2026-06-10 pin. **Superseded by the 2026-07-02 re-pin**: the schema tightened `requestId?` back to required `requestId` and restricted the notification to client→server only (a stdio-only server-sent exception closes `subscriptions/listen` streams). The crate reverted to the pre-2026-05-31 required-field shape; `without_id()` was removed. | `request_id: RequestId` (non-`Option`, no `skip_serializing_if`); `HasCancelledParams::request_id()` returns `&RequestId`. | `notifications::tests::test_cancelled_notification_always_emits_request_id`, `notifications::tests::test_cancelled_notification_rejects_missing_request_id` |
| `ContentBlock::ResourceLink` embeds `ResourceReference` which was missing `size` and `icons` — the schema says `ResourceLink extends Resource` (which has `size?: number` and `icons?` via `extends Icons`). Wire `size: 1234` round-tripped to silent drop. | Added `size: Option<u64>` and `icons: Option<Vec<Icon>>` (plus `with_size`/`with_icons` builders) to `ResourceReference`. Note: `ResourceReference` and `resources::Resource` remain parallel structs for now — collapsing onto one is queued for the trait-surface slice. | `content::tests::test_resource_link_round_trips_size_and_icons` |
| **[2026-07-02, HIGH]** `ContentBlock::ResourceLink` carried its own `annotations`/`_meta` variant fields ALONGSIDE the flattened `ResourceReference`, which also declares `annotations`/`meta`. Schema: `ResourceLink extends Resource` — exactly ONE of each. Consequence: serializing a block with both populated emitted **duplicate wire keys**, and deserialization always routed the value to the variant level, leaving `resource.annotations`/`resource.meta` permanently `None`. Flagged as an outbound wire defect (not accept-side leniency) by external review. | Removed the variant-level `annotations`/`meta` fields entirely — `ResourceReference` (flattened) is now the single source of truth. `ContentBlock::resource_link()`, `with_annotations()`, `with_meta()` updated to route through `resource.annotations`/`resource.meta`. | `content::tests::test_resource_reference_serialization_with_annotations_and_meta` (asserts exactly one `annotations`/`_meta` key **on the raw serialized text** — see correction note below — then round-trips through the single `ResourceReference` copy) |
| **[2026-07-02, correction]** The regression test above initially asserted duplicate-key absence against `serde_json::to_value()`. `to_value()` builds a `Map`, which cannot represent duplicate keys — a second `serialize_entry` for the same key silently overwrites the first in-memory — so the test could never have caught the defect it claimed to guard, regardless of whether the underlying `Serialize` impl emitted the key once or twice. Confirmed by isolating the exact pre-fix struct shape: `to_value()` → 1 key, `to_string()` → the literal duplicate. Caught by external re-review the same day. | Test rewritten to assert on the raw serialized text (`json_str.matches("\"annotations\":").count()`), and verified against the actual pre-fix shape to confirm it fails there and passes against the fix. | `content::tests::test_resource_reference_serialization_with_annotations_and_meta` |
| **[2026-07-02, MEDIUM]** `SubscriptionsListenResultMeta.extra` (public, caller-writable, `#[serde(flatten)]`) could be populated with the reserved `io.modelcontextprotocol/subscriptionId` key, colliding with the typed `subscription_id` field — the identical duplicate-key defect class as the ResourceLink row above, freshly introduced by the same slice that added the type. `RequestMetaObject.extra` carries the identical structural risk (typed field + flatten map, both able to claim the same key) but predates this slice and is **not** fixed here — out of scope, flagged for a dedicated pass. | Hand-written `Serialize` for `SubscriptionsListenResultMeta`: always emits the typed field; any `extra` entry matching the reserved key name is silently dropped rather than emitted, so the typed field always wins and the wire never repeats the key. `Deserialize` is unaffected (still derived). | `subscriptions::tests::listen_result_meta_extra_cannot_shadow_subscription_id` (populates `extra` with the reserved key, asserts exactly one occurrence on the raw wire, asserts the typed value — not the `extra` value — wins) |
| **[2026-07-02, MEDIUM]** `PrimitiveSchemaDefinition` (`StringSchema \| NumberSchema \| BooleanSchema \| EnumSchema`) was `#[serde(untagged)]` with no discriminator enforcement — each interface fixes a literal `type` (`"string"` / `"number"\|"integer"` / `"boolean"`), but `schema_type: String` accepts any string, and untagged dispatch tries `StringSchema` first. A bare `{"type":"integer"}` or `{"type":"boolean","title":"x"}` (no numeric/boolean-only field to force a mismatch) silently parsed as `StringSchema{schema_type:"integer"/"boolean"}`. Wire-lossless (the type string round-trips), but misclassifies for any Rust consumer matching the union variant, and permits constructing invalid states (`{"type":"banana"}`). | Hand-written `Deserialize` peeks `type` (and `enum`/`oneOf` presence, to route enum-shaped payloads to the untagged `EnumSchema` sub-union) before ever trying a variant; unrecognized/missing `type` is rejected outright instead of falling through to `StringSchema`. | `elicitation::enum_union_fidelity_tests::{bare_integer_discriminator_selects_number_not_string, bare_boolean_discriminator_selects_boolean_not_string, unknown_type_discriminator_is_rejected}` |
| `ElicitationSchema` was missing the `$schema?: string` field. JSON Schema 2020-12 (which DRAFT-2026-v1 adopts) carries this optional dialect declaration. Wire `{"$schema": "https://json-schema.org/draft/2020-12/schema"}` round-tripped to silent drop. | Added `schema_dialect: Option<String>` with `#[serde(rename = "$schema")]` plus `with_schema_dialect` builder. | `elicitation::tests::test_elicitation_schema_dialect_round_trips`, `elicitation::tests::test_elicitation_schema_omits_dialect_when_none` |
| `ListRootsRequest.params` was a bespoke `Option<ListRootsParams { meta: Option<HashMap> }>` shape instead of the schema's `params?: RequestParams`, as of the 2026-06-10 pin. **Superseded by the 2026-07-02 re-pin**: the schema retyped `params?` to a bespoke inline `{ _meta?: MetaObject }` (NOT the full `RequestParams` extension) — reverting the direction of the 2026-05-31 fix. | Reverted to a bespoke `ListRootsRequestParams { meta: Option<HashMap<String, Value>> }`; `ListRootsRequest::with_meta(HashMap<String, Value>)` takes a loose map again (not `RequestMetaObject`). `ListRootsRequest::new()` (paramsless) unchanged. | `roots::tests::test_list_roots_request_matches_typescript_spec`, `roots::tests::test_optional_params_serialization` |
| Seven notification traits in `traits.rs` (`*ListChangedNotification`, `ProgressNotification`, etc.) were bound on `JsonRpcNotificationTrait`, requiring `HasJsonRpcVersion` — a field the inner notification structs intentionally omit (the JSON-RPC envelope is added by wrapping in `JsonRpcNotification` at transport time per the wire-format discipline). The traits were unimplementable. | Rebound on `RpcNotification` (`HasMethod + HasParams` only); renamed with a `*Trait` suffix to avoid struct-name collisions; concrete impls added in `notifications.rs`. Trait abstraction now matches the Rust struct split (inner payload only), not the schema interface (which is wire-complete) — documented in §Intentional deviations as the notification-wire-format split. | `notifications::tests::tool_list_changed_satisfies_rpc_notification_and_trait`, `notifications::tests::cancelled_params_field_getters_via_trait`, `notifications::tests::progress_params_field_getters_via_trait`, `notifications::tests::resource_updated_uri_via_trait` |
| Missing trait coverage for new DRAFT-2026 RPCs — `server/discover`, `subscriptions/listen`, `InputRequiredResult` (SEP-2322). | Added `DiscoverRequestTrait`, `SubscriptionsListenRequestTrait` + field-getter `HasSubscriptionsListenParams`, `HasInputRequiredResult`. All bound on `RpcRequest` / `HasResultType` to match the Rust struct split. Impls added in each module. | `discover::tests::discover_request_satisfies_rpc_trait`, `subscriptions::tests::listen_request_satisfies_new_rpc_trait`, `input_required::tests::input_required_result_field_getters_via_trait` |

## SEP-2577 deprecation annotations (Slice A'' follow-up, 2026-05-31)

DRAFT-2026-v1 deprecates **Roots**, **Sampling**, and **Logging** per SEP-2577 (annotation-only this revision; earliest removal in the first release on or after **2027-07-28**). The crate carries inline `#[deprecated]` attributes at every relevant type definition site so downstream consumers get compile-time migration warnings.

| Feature | Deprecated types | Migration path |
|---|---|---|
| **Roots** | `Root`, `ListRootsRequest`, `ListRootsResult`, `RootsCapabilities`, `traits::ListRootsResult` | Pass directories / files via tool parameters, resource URIs, or server configuration. `notifications/roots/list_changed` has no binding in this crate's pinned `schema.ts` (the request/response surface is the deprecated-but-present remainder). |
| **Sampling** | `SamplingMessage`, `SamplingMessageContent`, `SamplingMessageContentBlock`, `CreateMessageRequest`, `CreateMessageRequestParams`, `CreateMessageResult`, `SamplingCapabilities`, `ModelHint`, `ModelPreferences`, `ToolChoice`, `ContentBlock::ToolUse`/`ToolResult` (variants + constructors), `traits::HasCreateMessageRequestParams`, `traits::CreateMessageRequest`, `traits::CreateMessageResult` | Integrate directly with LLM provider APIs. The soft-deprecated `include_context` values `"thisServer"`/`"allServers"` (per SEP-2596) are documented on the field. |
| **Logging** | `LoggingMessageNotification`, `LoggingMessageNotificationParams`, `LoggingLevel` (+ `LogLevel` alias), `META_KEY_LOG_LEVEL`, `RequestMetaObject.log_level` / `with_log_level`, `ServerCapabilities.logging`, `traits::LoggingMessageNotificationTrait`, `traits::HasLevelParam` | Log to `stderr` for stdio transports or use [OpenTelemetry](https://opentelemetry.io/). The per-request log-level opt-in (`RequestMetaObject.log_level`) replaces the `logging/setLevel` RPC, which has no binding in this crate's pinned `schema.ts` — and the schema deprecates the WHOLE Logging surface including that opt-in and `LoggingLevel`. The opt-in remains normative and implemented through the migration window. |

**Internal cross-references** use `#[allow(deprecated)]` where the SEP-2322 multi-round-trip flow legitimately references the deprecated types during the migration window (e.g., `InputRequest::CreateMessage(CreateMessageRequest)`, `InputRequest::ListRoots(ListRootsRequest)`, `InputResponse::CreateMessage(CreateMessageResult)`, `InputResponse::ListRoots(ListRootsResult)`).

**Pre-existing bug also fixed**: `LoggingMessageNotification` was previously defined twice — once in `src/logging.rs` (with `LoggingMessageParams`) and once in `src/notifications.rs` (with `LoggingMessageNotificationParams`). The two had the same wire shape but were distinct Rust types. The `logging.rs` duplicate was removed; only the spec-aligned `notifications.rs` version remains. The `HasLevelParam` / `HasLoggerParam` / `HasMetaParam` field-getter trait impls moved to `notifications::LoggingMessageNotificationParams`.

## Supported JSON Schema dialects

The 2026-07-28 line adopts **JSON Schema 2020-12** (SEP-2106) as the default
dialect for tool input/output schemas and elicitation form schemas. This
workspace's posture:

- **Emission**: schemas produced by the derive/builders pipeline target
  2020-12 (`$defs`/`$ref` preserved on the 2026 lane; see
  `schema_fidelity_2026.rs`). `ElicitationSchema` carries the optional
  `$schema` dialect declaration (`with_schema_dialect`).
- **Processing**: the framework does **not** run a validating JSON Schema
  processor. Schemas are transported faithfully and surfaced to applications;
  instance validation (tool arguments against `inputSchema`,
  `structuredContent` against `outputSchema`, elicitation responses against
  the requested form schema) is delegated to the application layer, which can
  pick a 2020-12 validator suited to its needs. Consequently the framework
  neither validates against a declared non-default dialect nor rejects
  unsupported dialects — a `$schema` value rides through verbatim.

This section is the dialect-support statement the spec's Basic-Protocol
SHOULD asks implementations to publish.

## Known follow-ups (not blockers for current state)

- **Elicitation enum schemas (2026-06-10)** — `EnumSchema` now binds the schema's union (`SingleSelectEnumSchema | MultiSelectEnumSchema | LegacyTitledEnumSchema`); the legacy `enumNames` shape is `LegacyTitledEnumSchema` (with its `default` field). `StringSchema`/`NumberSchema`/`BooleanSchema` and the untitled select shapes carry `deny_unknown_fields` so the untagged `PrimitiveSchemaDefinition` lands every payload on its precise variant instead of silently dropping `enum`/numeric constraints. Round-trip fidelity tests cover the union incl. through `ElicitationSchema.properties`.
- **`ContentBlock` union split** — the schema declares two distinct unions: `ContentBlock = Text | Image | Audio | ResourceLink | EmbeddedResource` (5) and `SamplingMessageContentBlock = Text | Image | Audio | ToolUse | ToolResult` (5). The crate models them as a single 7-variant enum, allowing wire-impossible shapes (a `CallToolResult.content` carrying `ToolUseContent`, etc.). Wire-equivalent for the 3-variant overlap; documented in §Intentional deviations.
- **`ResourceReference` ↔ `Resource` duplication** — A4 closed the wire-shape gap (size/icons now present on `ResourceReference`), but the two structs remain parallel. Collapsing onto one canonical `Resource` (per schema's `ResourceLink extends Resource`) is the cleaner end-state and queued for a follow-up.
- **`turul-mcp-ext-tasks` extension crate** (spec-neutral name per ADR-028's 2026-06-07 amendment) — SEP-2663 moved tasks out of the core protocol; the extension crate is scaffolded (2026-06-12: `v2026_07_28` types/lifecycle/capability + wire-shape compliance tests against the vendored `modelcontextprotocol/ext-tasks` schema). Tasks types remain correctly absent from the protocol crate; server dispatch wiring is a separate slice.

## Intentional deviations from strict schema

1. **`JsonRpcRequest.id: Value`** (`turul-rpc` permissive) — schema declares `RequestId = string | number`. Permissive shape is upstream choice for backward compatibility.
2. **`ContentBlock` modeled as an `enum` with inline struct-variants** — schema models the same union as separate `TextContent | ImageContent | …` interfaces. Wire-equivalent (same `type` tag discrimination); structural-only deviation. Slated for extraction to standalone structs in a separate slice.
3. **`*ResultResponse` envelope unions** (e.g. `CallToolResultResponse.result: CallToolResult | InputRequiredResult`) — only `DiscoverResultResponse` is bound. Others handled via `JsonRpcSuccessResponse.result: Value` + caller-side discrimination on `resultType`. Functional but untyped at the dispatcher layer.
4. **Pagination `cursor` lives on `PaginatedRequestParams`, not on a separate `PaginatedRequest` extender struct** — wire shape identical; Rust uses field composition instead of interface extension.
5. **`NotificationParams._meta` typed as the loose `MetaObject`, not `NotificationMetaObject`** — the schema retypes it to `NotificationMetaObject` (`MetaObject` plus optional `io.modelcontextprotocol/subscriptionId`). The result side of this deviation was closed on 2026-07-28: all 12 result types now carry `Option<ResultMetaObject>` and `HasMeta` was split from the notification carrier (`HasNotificationMeta`), because one trait returning one type cannot model two different schema carriers. The notification side is the same shape of work — add the type, retype the field, move the transport's `subscriptionId` stamp onto it — and is a named follow-up, not a permanent exemption. Wire-equivalent meanwhile: `NotificationMetaObject` extends `MetaObject`, and a loose map round-trips the namespaced key verbatim.

## Verifying the report

```bash
# Build + test
cargo test -p turul-mcp-protocol-2026-07-28 --features compliance

# Compliance binary (network + git required first run; cached thereafter)
cargo run -p turul-mcp-protocol-2026-07-28 --features compliance --bin mcp-compliance-2026-07-28

# Refresh upstream pin
cargo run -p turul-mcp-protocol-2026-07-28 --features compliance \
    --bin mcp-compliance-2026-07-28 -- refresh        # dry-run
cargo run -p turul-mcp-protocol-2026-07-28 --features compliance \
    --bin mcp-compliance-2026-07-28 -- refresh --write  # bumps PIN atomically
```

## Refresh contract

When upstream `schema/draft/examples` changes:

1. `refresh` resolves `main` HEAD via `git ls-remote`.
2. Re-fetches into a side cache (does not pollute primary).
3. Re-runs the full harness against the candidate pin.
4. Exits non-zero if any modeled case would regress.
5. With `--write`, rewrites both `schema/EXAMPLES_PIN.md` and the `PIN` constant in `src/compliance/fetch.rs` atomically (with rollback on partial failure).

The PIN constant is the **single source of truth** for what version the harness checks against.
