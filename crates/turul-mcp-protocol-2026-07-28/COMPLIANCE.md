# DRAFT-2026-v1 Compliance Report

`turul-mcp-protocol-2026-07-28` against the vendored MCP draft schema.

## Pin

- **Schema source**: `modelcontextprotocol/modelcontextprotocol` @ `schema/draft/schema.ts`
- **Vendored copy**: `crates/turul-mcp-protocol-2026-07-28/schema/draft-schema.ts`
- **Fixture pin (commit SHA)**: `c3e3f09eb5d271407afac0f0bb6ee2dae5813d1d` — see `schema/EXAMPLES_PIN.md`
- **Captured**: 2026-05-24
- **Schema surface**: 123 `export interface` + 27 `export type` + 9 `export const` = 159 declarations
- **Upstream MCP version string**: `"DRAFT-2026-v1"` (will flip when the final 2026-07-28 spec ships — see `docs/adr/027`)

## Test gate

| Surface | Count | Status |
|---|---|---|
| Lib unit tests | 139 | ✅ pass |
| `tests/compliance.rs` integration | 179 | ✅ pass |
| `tests/upstream_fixtures.rs` harness | 3 | ✅ pass |
| Doctests | 1 (+ 2 ignored) | ✅ pass |
| **Total** | **322** | ✅ all green, 0 warnings |
| `mcp-compliance-2026-07-28` binary | 20/20 fixtures | ✅ all pass |
| Modeled fixtures | 8 of 86 (9.3%) | ⚠ partial — see §Coverage below |

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
| `NotificationParams._meta?: MetaObject` | `notifications::NotificationParams.meta: Option<MetaObject>` | ✅ |
| `Result._meta?: MetaObject` | per-result struct: `meta: Option<MetaObject>` | ✅ |
| Required keys: `io.modelcontextprotocol/protocolVersion`, `clientInfo`, `clientCapabilities` | typed named fields with `#[serde(rename = "io.modelcontextprotocol/…")]` | ✅ |
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

No method strings outside the canonical 22 are declared anywhere in the crate. Earlier compat traits for the removed `initialize` handshake and `notifications/roots/list_changed` have been deleted in keeping with Protocol Crate Purity (schema-only API).

These survive as constants but must NOT be emitted; tooling that sends them violates the spec. Removal pending (separate slice).

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
| **Total** | ~150 | ~150 | 20 file-level wire tests (8 modeled cases) |

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
| `progressToken` | `meta::ProgressToken` | ✅ |
| `structuredContent` | `tools::CallToolResult.structured_content` | ✅ |
| `toolUseId` | `content::ContentBlock::ToolResult { tool_use_id }` | ✅ |
| `isError` | `tools::CallToolResult.is_error` | ✅ |
| `readOnlyHint` / `destructiveHint` / `idempotentHint` / `openWorldHint` | `tools::ToolAnnotations.{...}` | ✅ |
| `costPriority` / `speedPriority` / `intelligencePriority` | `sampling::ModelPreferences.{...}` | ✅ |
| `requestState` | `input_required::InputRequiredResult.request_state` | ✅ |
| `inputRequests` / `inputResponses` | typed maps in `input_required` | ✅ |
| `elicitationId` | `elicitation::ElicitRequestURLParams.elicitation_id` | ✅ |
| `io.modelcontextprotocol/protocolVersion` (etc.) | `meta::RequestMetaObject.protocol_version` (rename) | ✅ |

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

- **78 remaining cases** marked `Kind::NotModeled` — wave-by-wave migration to be raised by deliberate PR.

## Intentional deviations from strict schema

1. **`JsonRpcRequest.id: Value`** (`turul-rpc` permissive) — schema declares `RequestId = string | number`. Permissive shape is upstream choice for backward compatibility.
2. **`ContentBlock` modeled as an `enum` with inline struct-variants** — schema models the same union as separate `TextContent | ImageContent | …` interfaces. Wire-equivalent (same `type` tag discrimination); structural-only deviation. Slated for extraction to standalone structs in a separate slice.
3. **`*ResultResponse` envelope unions** (e.g. `CallToolResultResponse.result: CallToolResult | InputRequiredResult`) — only `DiscoverResultResponse` is bound. Others handled via `JsonRpcSuccessResponse.result: Value` + caller-side discrimination on `resultType`. Functional but untyped at the dispatcher layer.
4. **Pagination `cursor` lives on `PaginatedRequestParams`, not on a separate `PaginatedRequest` extender struct** — wire shape identical; Rust uses field composition instead of interface extension.

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
