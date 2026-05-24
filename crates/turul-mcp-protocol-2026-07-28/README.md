# turul-mcp-protocol-2026-07-28

[![Crates.io](https://img.shields.io/crates/v/turul-mcp-protocol-2026-07-28.svg)](https://crates.io/crates/turul-mcp-protocol-2026-07-28)
[![Documentation](https://docs.rs/turul-mcp-protocol-2026-07-28/badge.svg)](https://docs.rs/turul-mcp-protocol-2026-07-28)

Model Context Protocol (MCP) specification implementation for the **DRAFT-2026-v1** schema.

- Wire-version string: **`DRAFT-2026-v1`** (the `LATEST_PROTOCOL_VERSION` value in upstream `schema.ts`).
- Vendored upstream schema: [`schema/draft-schema.ts`](schema/draft-schema.ts) (ETag-pinned; see [`schema/README.md`](schema/README.md)).
- Draft spec on the web: <https://modelcontextprotocol.io/specification/draft>.

## What's in this crate

A faithful 1:1 mapping of `schema/draft-schema.ts`. Every TS interface, type, and method-string in the schema has a Rust binding; every binding has a compliance test asserting wire shape.

**Core protocol surface**:

- **Stateless RPC** — per-request capability negotiation via [`meta::RequestMetaObject`] (`io.modelcontextprotocol/protocolVersion`, `clientInfo`, `clientCapabilities`).
- **`server/discover`** — [`discover::DiscoverRequest`] / [`discover::DiscoverResult`] for server capability advertisement.
- **Multi round-trip requests** — [`input_required`] module: `InputRequest`/`InputResponse` pairs, `InputRequiredResult` with `requestState` opaque echo, `InputResponseRequestParams` mixin embedded in `tools/call`, `resources/read`, `prompts/get` params.
- **Unified subscription stream** — [`subscriptions::SubscriptionsListenRequest`] with opt-in `SubscriptionFilter` (toolsListChanged/promptsListChanged/resourcesListChanged/resourceSubscriptions) and `SubscriptionsAcknowledgedNotification` for the server ack.
- **Caching mixin** — [`caching::CacheableResult`] (`ttlMs` + `cacheScope`) required on every list/read result.
- **Tool schemas** — [`tools::ToolSchema`] (JSON Schema 2020-12 inputSchema, root `type: "object"`) and [`tools::ToolOutputSchema`] (unrestricted output schema).
- **Result discrimination** — [`result_type::ResultType`] (`"complete"` | `"input_required"`) required on every Result.
- **All other areas** — resources, prompts, tools, completion, elicitation (incl. all 4 enum-schema variants), sampling, roots, logging, content blocks, icons, notifications.

**Convention constants** for `_meta` keys and HTTP headers:

- [`META_KEY_PROTOCOL_VERSION`], [`META_KEY_CLIENT_INFO`], [`META_KEY_CLIENT_CAPABILITIES`], [`META_KEY_LOG_LEVEL`] — schema-declared.
- [`META_KEY_TRACEPARENT`], [`META_KEY_TRACESTATE`], [`META_KEY_BAGGAGE`] — W3C Trace Context (SEP-414).
- [`META_KEY_SUBSCRIPTION_ID`] — `subscriptions/listen` notification tagging.
- [`HTTP_HEADER_PROTOCOL_VERSION`], [`HTTP_HEADER_METHOD`], [`HTTP_HEADER_NAME`], [`HTTP_HEADER_CUSTOM_PREFIX`] — Streamable HTTP per SEP-2243.

**Extensions** — this crate hosts the `extensions: HashMap<String, Value>` capability map on `ClientCapabilities`/`ServerCapabilities`. Extension *types* live in separate `turul-mcp-ext-*` crates per [`docs/adr/028-extensions-strategy.md`](../../docs/adr/028-extensions-strategy.md).

## Status

**322 tests passing** (139 lib + 179 compliance integration + 3 upstream-fixture + 1 doctest). The schema-drift detector in `tests/compliance.rs::removed_methods` enforces absence of methods the schema does not declare. The method-string count-pin in `method_strings::schema_method_count_matches_canonical_list` catches new schema methods that don't have Rust bindings.

⚠ **DRAFT spec.** The upstream `schema.ts` is still in draft. When the final 2026-07-28 specification publishes, the wire-version string is expected to change from `"DRAFT-2026-v1"` to a stable date form. Regeneration trigger and process: [`docs/adr/027-targeting-mcp-draft-2026-v1.md`](../../docs/adr/027-targeting-mcp-draft-2026-v1.md).

## Crate layout

```
src/
  json_rpc.rs        — JSON-RPC 2.0 envelopes (Request/Notification/Response/Error/Message)
  meta.rs            — MetaObject, RequestMetaObject, Annotations, ProgressToken, Cursor,
                       META_KEY_* constants
  headers.rs         — HTTP_HEADER_* constants for Streamable HTTP transport
  result_type.rs     — ResultType discriminator ("complete" | "input_required")
  input_required.rs  — InputRequest/InputResponse/InputRequests/InputResponses,
                       InputRequiredResult, InputResponseRequestParams (SEP-2322)
  discover.rs        — server/discover: DiscoverRequest, DiscoverResult, DiscoverResultResponse
  caching.rs         — CacheableResult mixin (ttlMs, cacheScope) per SEP-2549
  subscriptions.rs   — subscriptions/listen + SubscriptionFilter +
                       SubscriptionsAcknowledgedNotification
  initialize.rs      — ClientCapabilities, ServerCapabilities, Implementation
  notifications.rs   — All notification types per schema
  tools.rs           — Tool, ToolSchema (input), ToolOutputSchema (output),
                       CallToolRequest/Result, ListToolsRequest/Result
  resources.rs       — Resource, ResourceTemplate, ListResources/Read/Templates types
  prompts.rs         — Prompt, PromptArgument, PromptMessage, ListPrompts, GetPrompt
  elicitation.rs     — ElicitRequest, all enum schema variants (Single/Multi/Titled/Untitled),
                       PrimitiveSchemaDefinition, ElicitResult
  completion.rs      — completion/complete: CompleteRequest/Result
  sampling.rs        — sampling/createMessage: CreateMessageRequest/Result, ToolChoice,
                       ModelPreferences/Hint, SamplingMessage
  roots.rs           — roots/list: ListRootsRequest/Result, Root
  logging.rs         — LoggingMessageNotification, LoggingLevel
  content.rs         — ContentBlock variants (Text/Image/Audio/ResourceLink/EmbeddedResource)
  icons.rs           — Icon, IconTheme
  ping.rs            — EmptyResult, EmptyParams
  schema.rs          — JsonSchema enum (utility for tool schema construction)
  traits.rs          — Per-type trait contracts (Params, HasMeta, HasMethod, RpcResult, etc.)
  version.rs         — McpVersion enum incl. V2026_07_28
  prelude.rs         — Convenience re-exports
  param_extraction.rs — Generic parameter extraction
tests/
  compliance.rs      — 19+ named test modules (integration tests via public API)
  upstream_fixtures.rs — Pinned upstream-example roundtrip harness
schema/
  draft-schema.ts    — Vendored upstream schema (ETag-pinned)
  README.md          — Provenance + regeneration instructions
```

## Documentation map

| Doc | What |
|-----|------|
| [`docs/plans/2026-07-28-schema-coverage-matrix.md`](../../docs/plans/2026-07-28-schema-coverage-matrix.md) | Every TS symbol → Rust binding → test → doc, plus the upstream-changelog coverage table |
| [`docs/plans/2026-07-28-compliance-plan.md`](../../docs/plans/2026-07-28-compliance-plan.md) | Live status of each spec area + remaining items |
| [`docs/adr/027-targeting-mcp-draft-2026-v1.md`](../../docs/adr/027-targeting-mcp-draft-2026-v1.md) | Wire-string target + regeneration trigger |
| [`docs/adr/028-extensions-strategy.md`](../../docs/adr/028-extensions-strategy.md) | How extensions are hosted (separate `turul-mcp-ext-*` crates) |
| Per-module rustdoc | Schema-line refs threaded through every `pub` type |

## See also

- `turul-mcp-protocol` — consumer-facing alias crate (a single-line re-export of the active protocol-version crate).
- `turul-mcp-server`, `turul-mcp-client` — high-level framework + client built on top.
