# Migration Diff: 2025-11-25 → DRAFT-2026-v1

Derived from `crates/turul-mcp-protocol-2026-07-28/schema/draft-schema.ts` (vendored 2026-05-24, ETag `8bdd4ae5...`). This document maps every TS symbol to its Rust counterpart and flags what's NEW, REMOVED, or CHANGED. Compliance work was driven from this diff via the (since executed and deleted) phase plan; current tracking is `docs/plans/2026-07-28-spec-compliance.md`.

**Convention:** schema line refs are `[L<n>]`. Existing Rust files refer to `crates/turul-mcp-protocol-2026-07-28/src/`.

---

## Method strings — exhaustive enumeration

22 methods total in DRAFT-2026-v1 (vs 25+ in 2025-11-25):

| Method | Direction | Status | Schema line |
|--------|-----------|--------|-------------|
| `server/discover` | C→S request | **NEW** | L569 |
| `notifications/cancelled` | bidi notification | retained | L552 |
| `notifications/progress` | bidi notification | retained | L930 |
| `resources/list` | C→S request | retained | L1009 |
| `resources/templates/list` | C→S request | retained | L1045 |
| `resources/read` | C→S request | retained | L1104 |
| `notifications/resources/list_changed` | S→C notification | retained | L1144 |
| `subscriptions/listen` | C→S request | **NEW** (replaces `resources/subscribe` + HTTP GET) | L1202 |
| `notifications/subscriptions/acknowledged` | S→C notification | **NEW** | L1233 |
| `notifications/resources/updated` | S→C notification | retained | L1263 |
| `prompts/list` | C→S request | retained | L1401 |
| `prompts/get` | C→S request | retained | L1456 |
| `notifications/prompts/list_changed` | S→C notification | retained | L1588 |
| `tools/list` | C→S request | retained | L1602 |
| `tools/call` | C→S request | retained | L1717 |
| `notifications/tools/list_changed` | S→C notification | retained | L1730 |
| `notifications/message` | S→C notification | retained (opt-in changed) | L1880 |
| `sampling/createMessage` | S→C request | retained (soft-deprecated) | L1987 |
| `completion/complete` | C→S request | retained | L2406 |
| `roots/list` | S→C request | retained (soft-deprecated) | L2492 |
| `elicitation/create` | S→C request | retained (form+url modes) | L2627 |
| `notifications/elicitation/complete` | S→C notification | retained (URL mode) | L2928 |

### Removed (must reject; tests must assert removal)
- `ping` — gone entirely. No replacement.
- `initialize` / `notifications/initialized` — handshake removed (stateless core).
- `logging/setLevel` — replaced by `_meta.io.modelcontextprotocol/logLevel` per request.
- `resources/subscribe` / `resources/unsubscribe` — replaced by `SubscriptionFilter.resourceSubscriptions` inside `subscriptions/listen`.
- `notifications/roots/list_changed` — roots is soft-deprecated; no listChanged notification.
- All `tasks/*` — Tasks moved to extension repo (SEP-2663), out of core schema.

---

## Foundational types

### `lib.rs`
| Symbol | Status | Notes |
|--------|--------|-------|
| `MCP_VERSION` constant | **CHANGED** | `"2026-07-28"` → `"DRAFT-2026-v1"` (already landed) |
| `McpError::ResourceNotFound` | **CHANGED** | Wire code `-32002` → `-32602` (SEP-2164) |
| `McpError::MissingRequiredClientCapability` | **NEW** | Code `-32003`, carries `requiredCapabilities: ClientCapabilities` in `data` |
| `McpError::UnsupportedProtocolVersion` | **NEW** | Code `-32004`, carries `{supported: [], requested: ""}` in `data` |

### `json_rpc.rs` [L26–258]
| TS symbol | Rust counterpart | Status |
|-----------|-------------------|--------|
| `JSONRPCMessage` (union) | `JsonRpcMessage` | confirm union variant order matches |
| `JSONRPCRequest` | `JsonRpcRequest` | verify `jsonrpc: "2.0"` literal |
| `JSONRPCNotification` | `JsonRpcNotification` | same |
| `JSONRPCResultResponse` | `JsonRpcResponse::Success`? | check tagged-union shape |
| `JSONRPCErrorResponse` | `JsonRpcResponse::Error`? | `id?` is optional — must match |
| `JSONRPCResponse` | `JsonRpcResponse` | verify untagged union |
| `Error` interface | `JsonRpcError` | `code/message/data?` |
| `Request` (loose) | n/a | TS allows `params: {[key: string]: any}`; Rust uses typed `RequestParams` |
| `Notification` (loose) | n/a | same |

### `meta.rs` [L41–185]
| TS symbol | Rust counterpart | Status |
|-----------|-------------------|--------|
| `MetaObject = Record<string, unknown>` | `Meta`? `HashMap<String, Value>`? | check shape |
| `RequestMetaObject extends MetaObject` | **NEW Rust type** | REQUIRED fields: `io.modelcontextprotocol/protocolVersion`, `io.modelcontextprotocol/clientInfo: Implementation`, `io.modelcontextprotocol/clientCapabilities: ClientCapabilities`. Optional: `progressToken?`, `io.modelcontextprotocol/logLevel?: LoggingLevel` |
| `ProgressToken = string \| number` | `ProgressTokenValue` enum | already matches |
| `Cursor = string` | `Cursor` | matches |
| `RequestParams { _meta: RequestMetaObject }` | **CHANGED** | `_meta` was optional; now REQUIRED |
| `NotificationParams { _meta?: MetaObject }` | matches | already optional |
| `Result { _meta?, resultType: ResultType, [k]: unknown }` | **CHANGED** | `resultType` REQUIRED |
| `ResultType = "complete" \| "input_required"` | **NEW Rust type** | enum |

### Result discrimination
| TS symbol | Rust counterpart | Status |
|-----------|-------------------|--------|
| `EmptyResult = Result` | `EmptyResult` | must still serialize `resultType` |
| `InputRequiredResult extends Result` | **NEW Rust type** | `inputRequests?: InputRequests`, `requestState?: string`; at least one must be present |
| `InputRequest` (union) | **NEW Rust type** | `CreateMessageRequest \| ListRootsRequest \| ElicitRequest` |
| `InputResponse` (union) | **NEW Rust type** | `CreateMessageResult \| ListRootsResult \| ElicitResult` |
| `InputRequests { [k]: InputRequest }` | **NEW Rust type** | server-assigned IDs |
| `InputResponses { [k]: InputResponse }` | **NEW Rust type** | client responses keyed by same IDs |
| `InputResponseRequestParams extends RequestParams` | **NEW Rust type** | `inputResponses?`, `requestState?` — mixed into `CallToolRequestParams`, `ReadResourceRequestParams`, `GetPromptRequestParams` |

### Pagination + caching mixins
| TS symbol | Rust counterpart | Status |
|-----------|-------------------|--------|
| `PaginatedRequestParams { cursor?: Cursor }` | already exists as `WithCursor` | verify extends `RequestParams` (with required `_meta`) |
| `PaginatedRequest` | n/a (mixin) | |
| `PaginatedResult { nextCursor?: Cursor }` | `PaginatedResponse` | verify still extends `Result` |
| `CacheableResult { ttlMs: number; cacheScope: "public" \| "private" }` | **NEW Rust type** | REQUIRED fields. Extended by `ListResourcesResult`, `ListResourceTemplatesResult`, `ReadResourceResult`, `ListPromptsResult`, `ListToolsResult` |

---

## `initialize.rs` — DELETE

The entire file's purpose (handshake) is gone. Migrate the surviving capability types to `discover.rs`:
- `InitializeRequest` — **DELETE**
- `InitializeResult` — **DELETE**
- `ClientCapabilities` — **MIGRATE** to `discover.rs` (shape changed; see below)
- `ServerCapabilities` — **MIGRATE** to `discover.rs` (shape changed; see below)
- `Implementation` — **MIGRATE** to shared location (used by both `DiscoverResult.serverInfo` and `RequestMetaObject.clientInfo`)
- `TasksCapabilities`, `TasksCancelCapabilities`, `TasksListCapabilities`, `TasksRequestCapabilities`, `TasksToolCallCapabilities`, `TasksToolCapabilities` — **DELETE** (tasks out of core)

## `discover.rs` — NEW [L556–772]
| TS symbol | Rust shape |
|-----------|------------|
| `DiscoverRequest` | method `"server/discover"`, params `RequestParams` |
| `DiscoverResult extends Result` | `supportedVersions: Vec<String>`, `capabilities: ServerCapabilities`, `serverInfo: Implementation`, `instructions?: String` |
| `DiscoverResultResponse extends JSONRPCResultResponse` | `result: DiscoverResult` |
| `ClientCapabilities` | `experimental?`, `roots?: {}` (empty obj), `sampling?: {context?, tools?}`, `elicitation?: {form?, url?}`, `extensions?: HashMap<String, JsonObject>` |
| `ServerCapabilities` | `experimental?`, `logging?`, `completions?`, `prompts?: {listChanged?}`, `resources?: {subscribe?, listChanged?}`, `tools?: {listChanged?}`, `extensions?` |
| `Implementation extends BaseMetadata, Icons` | `name`, `title?`, `icons?`, `version`, `description?`, `websiteUrl?` |
| `BaseMetadata` | `name: String`, `title?: String` |
| `Icons` interface (mixin) | `icons?: Vec<Icon>` |
| `Icon` | `src`, `mimeType?`, `sizes?`, `theme?` (light/dark) — matches 2025-11-25 |

---

## `tools.rs` [L1592–1844]
| TS symbol | Status |
|-----------|--------|
| `ListToolsRequest extends PaginatedRequest` | retained |
| `ListToolsResult extends PaginatedResult, CacheableResult` | **CHANGED** — now requires `ttlMs`, `cacheScope` |
| `ListToolsResultResponse` | retained |
| `CallToolRequest` | retained, `params: CallToolRequestParams` |
| `CallToolRequestParams extends InputResponseRequestParams` | **CHANGED** — adds `inputResponses?`, `requestState?` (from mixin) |
| `CallToolResult extends Result` | `content: ContentBlock[]`, `structuredContent?: unknown`, `isError?: boolean` — **CHANGED** `structuredContent` was object-only |
| `CallToolResultResponse.result` | **CHANGED** — `CallToolResult \| InputRequiredResult` union |
| `ToolListChangedNotification` | retained |
| `Tool extends BaseMetadata, Icons` | description?, **inputSchema: {$schema?, type:"object", [k]: unknown}**, **outputSchema?: {$schema?, [k]: unknown}**, annotations?, _meta? |
| `ToolAnnotations` | `title?, readOnlyHint?, destructiveHint?, idempotentHint?, openWorldHint?` — matches 2025-11-25 |
| **REMOVED**: `ToolExecution`, `TaskSupport`, `task` field on `CallToolRequestParams` | tasks moved to extension |

## `resources.rs` [L999–1390]
| TS symbol | Status |
|-----------|--------|
| `ListResourcesRequest/Result/Response` | **CHANGED** — Result extends `CacheableResult` |
| `ListResourceTemplatesRequest/Result/Response` | **CHANGED** — Result extends `CacheableResult` |
| `ResourceRequestParams { uri: string }` | mixin |
| `ReadResourceRequestParams extends ResourceRequestParams, InputResponseRequestParams` | **CHANGED** — multi-inherit |
| `ReadResourceRequest` | retained |
| `ReadResourceResult extends CacheableResult` | **CHANGED** — caching mixin |
| `ReadResourceResultResponse.result` | **CHANGED** — `ReadResourceResult \| InputRequiredResult` |
| `ResourceListChangedNotification` | retained |
| `SubscriptionFilter` | **NEW** — toolsListChanged?, promptsListChanged?, resourcesListChanged?, resourceSubscriptions? |
| `SubscriptionsListenRequestParams` | **NEW** — `{notifications: SubscriptionFilter}` |
| `SubscriptionsListenRequest` | **NEW** — method `"subscriptions/listen"` |
| `SubscriptionsAcknowledgedNotificationParams` | **NEW** |
| `SubscriptionsAcknowledgedNotification` | **NEW** — first message on subscription stream |
| `ResourceUpdatedNotificationParams/Notification` | retained |
| `Resource extends BaseMetadata, Icons` | `uri, description?, mimeType?, annotations?, size?, _meta?` — matches |
| `ResourceTemplate extends BaseMetadata, Icons` | `uriTemplate, description?, mimeType?, annotations?, _meta?` — matches |
| `ResourceContents`, `TextResourceContents`, `BlobResourceContents` | retained |
| **REMOVED**: `SubscribeRequest`, `UnsubscribeRequest`, `ResourceSubscription` | replaced by subscriptions/listen filter |

## `prompts.rs` [L1391–1590]
| TS symbol | Status |
|-----------|--------|
| `ListPromptsRequest/Result/Response` | **CHANGED** — Result extends `CacheableResult` |
| `GetPromptRequestParams extends InputResponseRequestParams` | **CHANGED** — `name, arguments?: {[k]: string}` plus inputResponses/requestState |
| `GetPromptRequest` | retained |
| `GetPromptResult extends Result` | `description?, messages: PromptMessage[]` |
| `GetPromptResultResponse.result` | **CHANGED** — `GetPromptResult \| InputRequiredResult` |
| `PromptListChangedNotification` | retained |
| `Prompt extends BaseMetadata, Icons` | `description?, arguments?: PromptArgument[], _meta?` — matches |
| `PromptArgument extends BaseMetadata` | `description?, required?` |

## `content.rs` [L1523–2269]
| TS symbol | Status |
|-----------|--------|
| `Role = "user" \| "assistant"` | matches |
| `PromptMessage { role, content: ContentBlock }` | matches |
| `ResourceLink extends Resource { type: "resource_link" }` | check existence |
| `EmbeddedResource { type: "resource", resource, annotations?, _meta? }` | matches |
| `ContentBlock` union | `TextContent \| ImageContent \| AudioContent \| ResourceLink \| EmbeddedResource` |
| `TextContent { type: "text", text, annotations?, _meta? }` | matches |
| `ImageContent { type: "image", data, mimeType, annotations?, _meta? }` | matches |
| `AudioContent { type: "audio", data, mimeType, annotations?, _meta? }` | matches |
| `ToolUseContent { type: "tool_use", id, name, input, _meta? }` | sampling-only |
| `ToolResultContent { type: "tool_result", toolUseId, content, structuredContent?, isError?, _meta? }` | sampling-only |
| `SamplingMessageContentBlock` union | `TextContent \| ImageContent \| AudioContent \| ToolUseContent \| ToolResultContent` — distinct from `ContentBlock`! |
| `Annotations { audience?, priority?, lastModified? }` | matches |

## `notifications.rs` [reorganize]
Trim to exactly the methods in the schema. **Remove**: `InitializedNotification`, `RootsListChangedNotification` (if present), all task notifications.
**Retain/align**:
- `CancelledNotification` [L551]
- `ProgressNotification` [L929]
- `ResourceListChangedNotification` [L1143]
- `ResourceUpdatedNotification` [L1262]
- `PromptListChangedNotification` [L1587]
- `ToolListChangedNotification` [L1729]
- `LoggingMessageNotification` [L1879]
- `ElicitationCompleteNotification` [L2927]
**Add new**:
- `SubscriptionsAcknowledgedNotification` [L1232]

## `sampling.rs` [L1902–2356]
**Status: retained, soft-deprecated** (annotation-only per SEP-2577 12-month window).
| TS symbol | Status |
|-----------|--------|
| `CreateMessageRequest` | retained, no `extends JSONRPCRequest` (server-initiated, model under InputRequest union) |
| `CreateMessageRequestParams` | `messages, modelPreferences?, systemPrompt?, includeContext? (soft-deprecated "thisServer"/"allServers"), temperature?, maxTokens, stopSequences?, metadata?, tools? (gated by sampling.tools cap), toolChoice?` |
| `ToolChoice { mode?: "auto"\|"required"\|"none" }` | **CHANGED** — only `"any"` legacy alias gone? confirm in implementation. |
| `CreateMessageResult extends SamplingMessage` | `model, stopReason?` |
| `SamplingMessage { role, content: SamplingMessageContentBlock \| SamplingMessageContentBlock[], _meta? }` | **CHANGED** — content can be array now |
| `ModelPreferences { hints?, costPriority?, speedPriority?, intelligencePriority? }` | matches |
| `ModelHint { name? }` | matches |

Add `#[deprecated]` markers on the public types.

## `completion.rs` [L2358–2474]
| TS symbol | Status |
|-----------|--------|
| `CompleteRequestParams extends RequestParams` | **CHANGED** — `_meta` required; `ref, argument {name, value}, context? {arguments?}` |
| `CompleteRequest` | method `"completion/complete"` |
| `CompleteResult extends Result` | `completion: {values: string[] (max 100), total?, hasMore?}` |
| `CompleteResultResponse.result` | `CompleteResult` (no InputRequired variant) |
| `ResourceTemplateReference { type: "ref/resource", uri }` | matches |
| `PromptReference extends BaseMetadata { type: "ref/prompt" }` | matches |

## `roots.rs` [L2476–2538]
**Status: retained, soft-deprecated.**
| TS symbol | Status |
|-----------|--------|
| `ListRootsRequest` | method `"roots/list"`, params optional |
| `ListRootsResult { roots: Root[] }` | matches (no `extends Result`?? confirm — schema line 2509 has no `extends Result`!) |
| `Root { uri, name?, _meta? }` | matches |

⚠ `ListRootsResult` does NOT extend `Result` per the schema (L2509). That means no `resultType`. Investigate — is this intentional (deprecated, doesn't carry new invariant) or a schema oversight? Compliance tests must mirror the schema literally.

## `elicitation.rs` [L2540–2935]
| TS symbol | Status |
|-----------|--------|
| `ElicitRequestFormParams { mode?: "form", message, requestedSchema {$schema?, type:"object", properties: {[k]: PrimitiveSchemaDefinition}, required?} }` | matches form mode |
| `ElicitRequestURLParams { mode: "url", message, elicitationId, url }` | matches URL mode |
| `ElicitRequestParams` (union) | `ElicitRequestFormParams \| ElicitRequestURLParams` |
| `ElicitRequest` | method `"elicitation/create"` |
| `PrimitiveSchemaDefinition` (union) | `StringSchema \| NumberSchema \| BooleanSchema \| EnumSchema` |
| `StringSchema { type:"string", title?, description?, minLength?, maxLength?, format?, default? }` | matches |
| `NumberSchema { type:"number"\|"integer", title?, description?, minimum?, maximum?, default? }` | matches |
| `BooleanSchema { type:"boolean", title?, description?, default? }` | matches |
| `UntitledSingleSelectEnumSchema { type:"string", enum: string[], default? }` | **NEW** |
| `TitledSingleSelectEnumSchema { type:"string", oneOf: [{const, title}], default? }` | **NEW** |
| `SingleSelectEnumSchema` (union) | **NEW** |
| `UntitledMultiSelectEnumSchema { type:"array", items: {type:"string", enum}, minItems?, maxItems?, default? }` | **NEW** |
| `TitledMultiSelectEnumSchema { type:"array", items: {anyOf: [{const, title}]}, minItems?, maxItems?, default? }` | **NEW** |
| `MultiSelectEnumSchema` (union) | **NEW** |
| `LegacyTitledEnumSchema { type:"string", enum, enumNames?, default? }` | **NEW** (compat shim) |
| `EnumSchema` (union) | `SingleSelectEnumSchema \| MultiSelectEnumSchema \| LegacyTitledEnumSchema` |
| `ElicitResult { action: "accept"\|"decline"\|"cancel", content? {[k]: string\|number\|boolean\|string[]} }` | matches |
| `ElicitationCompleteNotification { method: "notifications/elicitation/complete", params: {elicitationId} }` | matches |

## `logging.rs` [L1846–1900]
| TS symbol | Status |
|-----------|--------|
| `LoggingMessageNotificationParams { level: LoggingLevel, logger?, data: unknown }` | matches |
| `LoggingMessageNotification` | method `"notifications/message"` |
| `LoggingLevel = "debug"\|"info"\|"notice"\|"warning"\|"error"\|"critical"\|"alert"\|"emergency"` | matches |
| **REMOVED**: `SetLevelRequest` (`logging/setLevel`) | replaced by `_meta.io.modelcontextprotocol/logLevel` per request |
| Add deprecation markers per SEP-2577 |

## `tasks.rs` — DELETE FROM CORE

All task types belong in an extension repo per SEP-2663. For this protocol crate: delete `tasks.rs`. Re-export removal cascades through `lib.rs`.

## `ping.rs` — DELETE

`ping` is not in the schema. Delete the file. No replacement.

## `schema.rs` — JSON Schema 2020-12

Tool inputSchema/outputSchema are now full JSON Schema 2020-12. Rewrite `schema.rs` to support: `oneOf`, `anyOf`, `allOf`, `not`, `if`/`then`/`else`, `$ref`, `$defs`, `$anchor`. Add safeguard: refuse to auto-dereference external `$ref` URIs.

## Top-level union types (compliance test fixtures)
| TS symbol | Schema line | Required Rust |
|-----------|-------------|----------------|
| `ClientRequest` | L2939 | enum: Discover\|Complete\|GetPrompt\|ListPrompts\|ListResources\|ListResourceTemplates\|ReadResource\|SubscriptionsListen\|CallTool\|ListTools |
| `ClientNotification` | L2952 | enum: Cancelled\|Progress |
| `ClientResult` | L2955 | = EmptyResult |
| `ServerNotification` | L2960 | enum: Cancelled\|Progress\|LoggingMessage\|ResourceUpdated\|ResourceListChanged\|ToolListChanged\|PromptListChanged\|ElicitationComplete\|SubscriptionsAcknowledged |
| `ServerResult` | L2972 | enum: Empty\|Discover\|Complete\|GetPrompt\|ListPrompts\|ListResourceTemplates\|ListResources\|ReadResource\|CallTool\|ListTools\|InputRequired |

These four unions drive the compliance-test exhaustive-coverage check: every variant must have a `serde_json::to_value(..)` shape test.

---

## Workflow note

This diff is read-only. Status tracking lives in `docs/plans/2026-07-28-spec-compliance.md`; the source files themselves are the eventual truth. If upstream schema changes invalidate this diff, regenerate.
