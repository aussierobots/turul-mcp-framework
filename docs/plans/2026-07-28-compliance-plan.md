# Plan: `turul-mcp-protocol-2026-07-28` Compliance with `DRAFT-2026-v1` schema.ts

**Branch:** sub-branches off `2026-07-28-MCP-Specification` (parent branch is locked — see `CLAUDE.md`/`AGENTS.md` "Branch Lock"). Current scaffolding sub-branch: `feat/turul-mcp-protocol-2026-07-28`.

**Goal:** Rust types in `crates/turul-mcp-protocol-2026-07-28/src/` serialize to JSON that matches `crates/turul-mcp-protocol-2026-07-28/schema/draft-schema.ts` exactly. Tests in `src/compliance_test.rs` prove it the same way `turul-mcp-protocol-2025-11-25/src/compliance_test.rs` proves 2025-11-25 compliance.

**Verification gate:** `cargo test -p turul-mcp-protocol-2026-07-28` passes, with compliance tests covering every request/result/notification type, every method string, every error code, every required-vs-optional field, every camelCase field, every enum value.

**Spec target:** `DRAFT-2026-v1` per the vendored schema's `LATEST_PROTOCOL_VERSION` constant. Will flip to the final wire string when the upstream final ships — see `docs/adr/027-targeting-mcp-draft-2026-v1.md`.

---

## Sequencing principles

1. **Foundation before features.** Wire primitives (JSON-RPC envelope, meta, errors, result discrimination) must be aligned first because everything else extends them.
2. **One area per slice.** Each `src/*.rs` rewrite is its own sub-branch back to `2026-07-28-MCP-Specification`. Reviewable diffs beat one giant rewrite.
3. **Compliance test grows with each slice.** Don't accumulate all tests at the end — every slice lands its type changes AND the tests proving the new shape matches the vendored TS, in the same commit.
4. **Revert-and-fail check** per `CLAUDE.md` §"Test Coverage Discipline" #4: every behaviour-changing slice's new tests must fail when the fix is reverted. Otherwise they're not regression-net, they're code-shape-net.
5. **Don't touch the alias.** `turul-mcp-protocol` keeps pointing at 2025-11-25 throughout this work. Flipping the alias is its own slice, gated on this plan reaching full green.

---

## Phase 0 — Foundation (this sub-branch: `feat/turul-mcp-protocol-2026-07-28`)

- [x] **0.1 Crate scaffold.** Fork from 2025-11-25, rename, independent `version = "0.4.0"`, workspace wiring, doctests pass. *(landed)*
- [x] **0.2 Vendor draft schema.ts** with provenance README and ETag. *(landed)*
- [x] **0.3 ADR-027** — wire string and regeneration trigger. *(landed)*
- [x] **0.4 Wire-string fix.** `MCP_VERSION = "DRAFT-2026-v1"`, `McpVersion::V2026_07_28` serde-renames to `"DRAFT-2026-v1"`. *(landed)*
- [x] **0.5 This plan document.** *(landed)*
- [x] **0.6 Migration diff table.** *(landed: `docs/plans/2026-07-28-migration-diff.md`)*
- [x] **0.7 Update version.rs capability flags** *(landed)* — `supports_tasks` flipped to `false` since tasks moved to extension in DRAFT-2026-v1 (SEP-2663). Other flags audited against schema and unchanged.

## Phase 1 — Wire primitives

Spec sections: lines 1–500 of `schema/draft-schema.ts` (JSON types, JSON-RPC envelope, Meta, ResultType, errors, InputRequest/Response).

- [x] **1.1 `json_rpc.rs` — minimum-viable** *(landed)*: doc strings updated to reference DRAFT-2026-v1; compliance tests in `src/compliance_test.rs::envelope` (9 tests covering: jsonrpc literal "2.0", request shape, request with params, notification has no id, success response has result/no error, error response has error/no result, error object shape, data omission when None, standard JSON-RPC error code constants). Tracked refinements deferred to **Phase 1.1b** (post-Phase 1.2/1.3): typed `RequestId = String|Number` enum instead of `serde_json::Value`; split `JsonRpcResponse` into `JSONRPCResultResponse`/`JSONRPCErrorResponse` matching schema's union; drop unused `Error` variant from `JsonRpcMessage` union.
- [x] **1.2 `meta.rs` — RequestMetaObject** *(landed)* — `RequestMetaObject` added with required `io.modelcontextprotocol/protocolVersion`, `clientInfo`, `clientCapabilities` fields plus optional `progressToken`, `io.modelcontextprotocol/logLevel`, and flatten-extras for tracing/custom keys. `MetaObject` type alias added. Compliance tests in `src/compliance_test.rs::request_meta` (8 tests including rejection of missing required fields). Existing `Meta` carrier kept for legacy 2025-11-25 paths until Phase 3 cascade finalization.
- [x] **1.3 Result discrimination** *(landed)* — `ResultType` enum (`Complete` / `InputRequired`) in `src/result_type.rs` with snake_case serde rename and `Complete` default (per schema backward-compat clause). Added `result_type: ResultType` field with `#[serde(default)]` to CallToolResult, ListToolsResult, ListResourcesResult, ListResourceTemplatesResult, ReadResourceResult, ListPromptsResult, GetPromptResult, DiscoverResult. Compliance tests in `src/compliance_test.rs::result_discrimination` + `src/result_type::tests` (6 unit + 3 integration tests).
- [x] **1.4 Error codes** — *(landed)* Added `McpError::MissingRequiredClientCapability { required: serde_json::Value }` (→ `-32003`) and `UnsupportedProtocolVersion { supported, requested }` (→ `-32004`); remapped `ToolNotFound`/`ResourceNotFound`/`PromptNotFound` from custom `-32001/-32002/-32003` to standard `-32602`. Compliance tests in `src/compliance_test.rs::error_codes` (7 tests including a drift-detector). Pre-existing bug noted: `JsonRpcError { code: standard_code }` passthrough panics via `server_error` range check — orthogonal to fix later. Note: `required` field stays `serde_json::Value` until Phase 2.3 introduces typed `ClientCapabilities`.
- [x] **1.5 InputRequest types** *(landed)* — New `src/input_required.rs` with `InputRequest` (untagged union of CreateMessageRequest/ListRootsRequest/ElicitRequest), `InputResponse` (union of their results), `InputRequests`/`InputResponses` (HashMap aliases), `InputRequiredResult` (with at-least-one-of invariant + dedicated constructors), `InputResponseRequestParams` mixin. Compliance tests in `src/compliance_test.rs::multi_round_trip` + `src/input_required::tests` (9 unit + 3 flow tests). Mixin fields wired into `CallToolRequestParams`, `ReadResourceRequestParams`, `GetPromptRequestParams`.

## Phase 2 — Stateless core

Spec sections: `DiscoverRequest`, `DiscoverResult` (lines ~568–620), `ClientCapabilities`/`ServerCapabilities` (~623–900).

- [x] **2.1 Stateless-core handshake removal** *(landed)* — `initialize.rs` now hosts only the surviving capability and `Implementation` types. Capability negotiation lives in `RequestMetaObject` (per-request); server info lives in `DiscoverResult`. See migration diff for the symbol-level mapping.
- [x] **2.2 `discover.rs`** *(landed)* — `DiscoverRequest`, `DiscoverResult`, `DiscoverResultResponse` per schema. Wire method `server/discover`. 9 compliance tests in module + Phase 8 method-string binding.
- [x] **2.3 `ClientCapabilities`/`ServerCapabilities` reshape** *(landed)* — `SamplingCapabilities.context?/tools?` and `ElicitationCapabilities.form?/url?` added; `extensions: Option<HashMap>` added to both. `tasks` field marked `#[deprecated]`. Compliance tests in `src/compliance_test.rs::capabilities_shape` (7 tests).
- [x] **2.4 Schema-drift detectors for removed methods** *(landed)* — `src/compliance_test.rs::removed_methods` (10 tests) asserts the schema does not declare `initialize`, `notifications/initialized`, `ping`, `logging/setLevel`, `resources/subscribe`, `resources/unsubscribe`, `notifications/roots/list_changed`, or any `tasks/*` method. Includes a positive control and a protocol-version constant pin so re-vendoring is caught.

## Phase 3 — Per-area type alignment

One sub-branch per area. Each lands type changes + compliance tests + revert-and-fail check.

- [x] **3.0 `caching.rs`** *(landed)* — New module with `CacheScope` enum and `CacheableResult` mixin struct (`ttl_ms: u64`, `cache_scope: CacheScope`). 9 tests including flatten-pattern integration test.
- [x] **3.1 `tools.rs`** *(landed)* — `CallToolResult` and `ListToolsResult` got `resultType: ResultType` field. `ListToolsResult` got transitional `ttl_ms`/`cache_scope` Optional fields + `with_cache()` constructor. `CallToolRequestParams` got `input_responses`/`request_state` mixin from InputResponseRequestParams. `task` field deprecated. 12 compliance tests in `tools_alignment`.
- [x] **3.2 `resources.rs`** *(landed)* — `ListResourcesResult`, `ListResourceTemplatesResult`, `ReadResourceResult` carry resultType + CacheableResult mixin fields with `with_cache()` constructors. `ReadResourceRequestParams` carries the `InputResponseRequestParams` mixin (`input_responses`/`request_state`). Subscription model uses the unified `subscriptions/listen` stream + `SubscriptionFilter.resource_subscriptions` (see Phase 3.8). 11 compliance tests in `resources_alignment`.
- [x] **3.3 `prompts.rs`** *(landed)* — Same pattern: `ListPromptsResult`/`GetPromptResult` got resultType + transitional cache fields. `GetPromptRequestParams` got InputResponseRequestParams mixin. 8 compliance tests in `prompts_alignment`.
- [x] **3.4 `elicitation.rs` enum schemas** *(landed)* — Added DRAFT-2026-v1 types: `UntitledSingleSelectEnumSchema`, `TitledSingleSelectEnumSchema`, `UntitledMultiSelectEnumSchema`, `TitledMultiSelectEnumSchema`, `TitledEnumOption`, plus `SingleSelectEnumSchema`/`MultiSelectEnumSchema` untagged unions. Existing `EnumSchema` (single struct) kept as `LegacyTitledEnumSchema` equivalent. 8 compliance tests in `elicitation_enum_schemas`. **NOT YET DONE**: full InputRequiredResult rewrite for elicitation flow (Phase 1.5 foundation built; cascade to elicitation lifecycle pending).
- [x] **3.5 `completion.rs`** *(landed)* — `CompleteResult` got `result_type: ResultType`. 6 compliance tests in `completion_alignment`.
- [x] **3.6 `ping.rs`** *(landed)* — Module now hosts `EmptyResult` (extends `Result` per schema line 435; carries `result_type: ResultType`) and `EmptyParams` as a no-params utility. 3 compliance tests in `empty_result_alignment`.
- [x] **3.7 `content.rs`** *(landed)* — 7 compliance tests in `content_alignment` verifying TextContent/ImageContent/AudioContent type discriminators, untagged-union round-trip, ResourceLink and EmbeddedResource parsing, Annotations passthrough.
- [x] **3.8 `notifications.rs` + `subscriptions.rs`** *(landed)* — Module hosts exactly the notifications the schema declares: `CancelledNotification`, `ProgressNotification`, `ResourceListChangedNotification`, `ResourceUpdatedNotification`, `PromptListChangedNotification`, `ToolListChangedNotification`, `LoggingMessageNotification`, `ElicitationCompleteNotification`. New `src/subscriptions.rs` module adds `SubscriptionsListenRequest`, `SubscriptionFilter`, `SubscriptionsAcknowledgedNotification`. 10 compliance tests in `subscriptions::tests`.
- [ ] **3.9 `icons.rs`** — Pending verify (looks aligned, no changes seen).

## Phase 4 — Deprecated areas (SEP-2577)

Roots/Sampling/Logging are deprecated in 2026-07-28 (12-month annotation-only window). Types may still exist in schema; mark them `#[deprecated]` in Rust if so.

- [x] **4.1 `roots.rs`** *(landed)* — `ListRootsRequest`/`Root`/`ListRootsResult` per schema. No deprecation markers — soft-deprecation per SEP-2577 is spec-level only; consumers shouldn't get warnings until the 12-month window closes.
- [x] **4.2 `sampling.rs`** *(landed)* — Types match the schema verbatim. Same SEP-2577 treatment.
- [x] **4.3 `logging.rs`** *(landed)* — `LoggingMessageNotification` + `LoggingLevel` per schema. Log-level opt-in flows through `_meta.io.modelcontextprotocol/logLevel` on every request (modeled in `meta::RequestMetaObject::log_level`).

## Phase 5 — Extensions framework (SEP-2133)

The schema may have an `extensions` capability map. Confirm exact shape before designing.

- [ ] **5.1 New `extensions.rs`** — Extension descriptor types, reverse-DNS ID validation, capability negotiation map. **Tests**: ID validation, capability-map serialization.
- [ ] **5.2 Tasks-as-extension migration (`tasks.rs` → `extensions/tasks.rs`?)** — SEP-2663. Migrate from 2025-11-25 core lifecycle to 2026 extension lifecycle: server-directed task creation, `tasks/get`/`update`/`cancel` only, no `tasks/list`. **Tests**: new lifecycle methods, removed `tasks/list` is rejected.
- [ ] **5.3 MCP Apps extension (`extensions/apps.rs`)** — SEP-1865. UI templates, JSON-RPC over postMessage (or whatever the schema models). Confirm whether this is in `draft-schema.ts` or in a separate `ext-*` file before scope-locking.

## Phase 6 — JSON Schema 2020-12 (SEP-2106)

- [x] **6.1 `schema.rs` — minimum-viable** *(landed)*: `ToolSchema.additional: HashMap<String, Value>` flatten pattern already passes through arbitrary 2020-12 keywords at the schema root. 9 compliance tests in `json_schema_2020_12` proving round-trip of `oneOf`, `anyOf`, `allOf`, `$ref`, `$defs`, conditional (`if`/`then`/`else`), and the `$schema` dialect marker. **Documented gap**: `ToolSchema.properties: Option<HashMap<String, JsonSchema>>` is over-restrictive (forces our structured `JsonSchema` enum instead of accepting `[k]: unknown`). Test `known_gap_properties_field_too_strict_for_2020_12_unknown_values` pins this gap and self-disables when fixed. **Documented gap**: `outputSchema` per schema may be non-object-rooted; current `ToolSchema` hardcodes `type:"object"`. Both gaps are Phase 6 finalization items.

## Phase 7 — Routing, caching, tracing (SEP-2243, SEP-2549, SEP-414)

These are partly transport-layer (HTTP headers) and partly protocol-layer (`_meta` fields). Decide which belong in `turul-mcp-protocol-2026-07-28` vs `turul-http-mcp-server`.

- [ ] **7.1 Caching mixin (`CacheableResult` per schema)** — `ttlMs`, `cacheScope` fields. Already mixed into list/read results in Phase 3; capture the mixin formally here. **Tests**: round-trip with cache fields, omit when absent.
- [ ] **7.2 Tracing in `RequestMetaObject`** — W3C `traceparent`, `tracestate`, `baggage` fields per schema. **Tests**: round-trip, field naming, optional.
- [ ] **7.3 Header types** — `MCP-Protocol-Version`, `Mcp-Method`, `Mcp-Name` constants. These live more naturally in the http-server crate but the *constants* (header names) can live here. Defer until http-server slice.

## Phase 8 — Compliance test suite consolidation

By the time Phases 1–7 land, each area has its own compliance tests in `src/compliance_test.rs` (or per-file `#[cfg(test)]` blocks). Phase 8 audits the coverage.

- [ ] **8.1 Coverage audit.** Pending — multi-area sweep still has gaps (content blocks, completion, sampling).
- [x] **8.2 Method-string exhaustive test** *(landed)* — `src/compliance_test.rs::method_strings` enumerates 22 method strings in a canonical const slice, asserts each appears in `schema/draft-schema.ts` (positive cross-check), asserts schema's method count equals the list length (count-pin drift detector), and provides explicit binding tests for 16 of 22 methods via `method_of()` helper. Remaining 6 are checked via their owning module's unit tests (constructors that need complex parameters).
- [x] **8.3 Error-code exhaustive test** *(landed in Phase 1.4)* — `src/compliance_test.rs::error_codes::no_unauthorised_error_codes_emitted` enumerates every `McpError` variant via Rust exhaustiveness check and asserts each emits a code in the schema-allowed set.
- [x] **8.4 Negative tests** *(partial)* — `error_codes`, `request_meta`, `result_discrimination`, `caching` modules all include negative-path assertions (missing required fields, unknown discriminator values, wrong types). More coverage needed in per-area modules.
- [x] **8.5 Schema drift canaries** *(landed in Phase 2.4)* — `removed_methods::schema_protocol_version_constant_matches_crate` pins `LATEST_PROTOCOL_VERSION = "DRAFT-2026-v1"` in schema.ts. `server_discover_method_is_present_positive_control` proves the scanner matches the expected pattern.
- [ ] **8.6 Revert-and-fail audit.** Pending — to be done at slice-commit time, not at the in-flight working tree.

## Phase 5 — Extensions framework (SEP-2133)

The `extensions: HashMap<String, Value>` map field is wired into both `ClientCapabilities` and `ServerCapabilities` (Phase 2.3, landed). What remains is the **strategy decision** for how the framework hosts the actual extension types:

- [x] **5.1 ADR-028 — Extensions strategy** *(landed)* — `docs/adr/028-extensions-strategy.md`. Decision: separate `turul-mcp-ext-<name>-<schema-version>` crates mirroring upstream `ext-*` repos, with independent semver and Cargo-dependency-as-opt-in. Reverse-DNS validation deferred to runtime negotiation boundary. SEP-2133 + SEP-2663 content verified via `gh api repos/modelcontextprotocol/modelcontextprotocol/contents/seps/...`.
- [ ] **5.2 Tasks extension binding** — Implement once ADR-028 decides where it lives. Migrate the deleted `turul-mcp-protocol-2025-11-25` tasks types into the new home with DRAFT-2026-v1's stateless lifecycle per SEP-2663 (server-directed creation, `tasks/get`/`update`/`cancel` only, no `tasks/list`).
- [ ] **5.3 MCP Apps extension binding** (SEP-1865) — UI templates + sandboxed-iframe rendering. Same scaffolding pattern as 5.2 once ADR-028 decides location.

## Phase 9 — Documentation & cutover prep

- [ ] **9.1 lib.rs/README update.** Final implemented status, drop the "scaffold status" disclaimer, list every spec area now covered.
- [ ] **9.2 version.rs flags audit.** Reconcile `McpVersion::V2026_07_28.supports_*()` returns against actual implementation truth. Currently inherited from 2025-11-25.
- [ ] **9.3 ADR-027 revision log.** Add entries for each major phase completed.
- [ ] **9.4 Alias-flip plan.** Separate document or ADR: how `turul-mcp-protocol` switches from `2025-11-25` → `2026-07-28`, what cascades through `turul-mcp-server`, `turul-mcp-builders`, every example, every test. **This is its own branch, not part of this plan.** Listed here only as the natural sequel.

---

## Cross-cutting rules

- **Plan describes forward state.** Each phase entry says what was built (types + tests). Historical removal mappings live in `docs/plans/2026-07-28-migration-diff.md`; the plan doesn't double-bookkeep deletions.
- **Schema is source of truth.** If a test disagrees with the schema, change the test. If the schema disagrees with the blog post or memory, schema wins.
- **camelCase on the wire.** Every Rust snake_case field must `#[serde(rename = "...")]` to its camelCase JSON name.
- **Optional fields use `#[serde(skip_serializing_if = "Option::is_none")]`.** TS `field?: T` → Rust `Option<T>` with skip-when-None.
- **No framework features in the protocol crate.** Per `CLAUDE.md` §"Protocol Crate Purity," this crate is spec-pure. No helpers, no builders beyond basic constructors, no middleware hooks.
- **Frozen crates untouched.** `turul-mcp-protocol-2025-11-25` and `turul-mcp-protocol-2025-06-18` are not edited.
- **No publish without permission.** Per the user-feedback memory.
- **Branch-lock unchanged.** No merges to `main` without express maintainer authority.

## Status tracking

This document is the canonical task list. As phases complete, update the checkboxes in the same commit that lands the work. Sub-branch naming: `feat/2026-07-28-phase-<N>.<M>-<short-desc>` (e.g. `feat/2026-07-28-phase-1.1-json-rpc-envelope`).

When all checkboxes are green and `cargo test -p turul-mcp-protocol-2026-07-28` is fully green, the crate is compliance-ready for the alias-flip slice.
