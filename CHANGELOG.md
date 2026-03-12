# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.12] - 2026-03-12

### Fixed

- **CORS: expose `Mcp-Session-Id` header for browser MCP clients** (`turul-http-mcp-server`): Browser-based MCP clients couldn't read the `Mcp-Session-Id` response header because CORS didn't expose it. Added `Access-Control-Expose-Headers: Mcp-Session-Id`, added `Mcp-Session-Id` to `Access-Control-Allow-Headers`, and added `DELETE` to `Access-Control-Allow-Methods` for session teardown. Applies to both wildcard and origin-specific CORS configurations.

## [0.3.11] - 2026-03-09

### Added

- **`run_streaming_with()` custom dispatch** (`turul-mcp-aws-lambda`): Accepts a custom `Fn(Request) -> Future<Response>` closure for Lambda streaming, with the same completion-invocation handling as `run_streaming()`. Use this when you need pre-dispatch logic (e.g., `.well-known` routing) that runs before the MCP handler. Fixes completion-invocation ERROR logs for custom dispatch patterns; does not claim to resolve all Lambda streaming timeout behavior.
- **Prelude re-exports**: `run_streaming` and `run_streaming_with` are now available via `turul_mcp_aws_lambda::prelude::*`.

### Changed

- **`lambda-mcp-server-streaming` example**: Refactored from raw `lambda_http::run_with_streaming_response(service_fn(...))` to `turul_mcp_aws_lambda::run_streaming()`, demonstrating the framework's recommended streaming entry point.

## [0.3.10] - 2026-03-07

### Changed

- **`JwtValidator::new()` now requires audience** (`turul-mcp-oauth`): `JwtValidator::new(jwks_uri, audience)` — audience is a mandatory parameter per MCP spec requirement that servers MUST validate token audience. The optional `with_audience()` method has been removed.
- **`ProtectedResourceMetadata::new()` is now fallible** (`turul-mcp-oauth`): Returns `Result<Self, OAuthError>`. Validates `resource` and `authorization_servers` URIs using `url::Url` — requires http/https scheme, authority present, no fragment. Empty AS list rejected.
- **`oauth_resource_server()` is now fallible** (`turul-mcp-oauth`): Returns `Result<..., OAuthError>`. Enforces exactly one authorization server in metadata (no silent `[0]` fallback). Auto-wires audience from `metadata.resource` and issuer from single AS.

### Added

- **Scope in `WWW-Authenticate`** (`turul-mcp-oauth`): When `scopes_supported` is configured on metadata, challenge responses include `scope="scope1 scope2"` per RFC 6750 §3.
- **`Cache-Control: no-store`** (`turul-http-mcp-server`): All 401/403 challenge responses include `Cache-Control: no-store` per OAuth 2.1 §5.3. Applied in both Streamable HTTP and legacy transports.
- **Canonical URI validation** (`turul-mcp-oauth`): `ProtectedResourceMetadata::new()` validates resource and AS URIs — absolute URI with http/https scheme, authority required, no fragment allowed. New error variants: `OAuthError::InvalidResourceUri`, `OAuthError::InvalidConfiguration`.
- **Single-AS issuer enforcement** (`turul-mcp-oauth`): `oauth_resource_server()` rejects metadata with multiple authorization servers, preventing misconfigured deployments.

## [0.3.9] - 2026-03-06

### Added

- **Lambda streaming event classification** (`turul-mcp-aws-lambda`): Three-way classification of raw Lambda runtime payloads via `classify_runtime_event()` — distinguishes API Gateway events, streaming completion invocations, and unrecognized payloads. Prevents ERROR logs and CloudWatch Lambda Error metrics from completion invocations.
- **`run_streaming()` public API**: Replaces `lambda_http::run_with_streaming_response()` for MCP Lambda servers. Gracefully acknowledges completion invocations (200 + `debug` log) and unrecognized payloads (200 + `warn` log) instead of failing deserialization.
- **Testable surfacing contract**: `handle_runtime_payload()` returns typed `HandleResult { response, event_type }` for observability; `event_log_level()` maps event types to tracing levels — both independently testable without log capture.
- **OAuth resource server foundation** (`turul-http-mcp-server`): Bearer token middleware, route registry, request-scoped extensions on `SessionContext` for auth claims propagation.
- 25 classification/action-path/contract tests with `include_str!` fixture files for API Gateway v1/v2, streaming completion variants, and precedence edge cases.

### Fixed

- **Benchmark compilation**: `SessionContext` struct initializers in `performance-testing` benchmarks updated for new `extensions` field.

## [0.3.8] - 2026-03-05

### Fixed

- **Client streaming response forwarding** (P1): Server-initiated requests (`sampling/createMessage`, `elicitation/create`) now receive JSON-RPC responses back from the client callback. Previously responses were logged and discarded, causing servers to hang indefinitely. Architecture: `StreamHandler` → response channel → consumer task → `transport.send_notification()`. See [ADR-020](docs/adr/020-client-response-forwarding-architecture.md).
- **HTTP transport event classification**: Server-initiated requests (with both `method` and `id`) were misclassified as notifications. Fixed classification order: `method+id` → Request, `method` only → Notification, `id` only → Response.
- **`json_schema_derive.rs` `Option<T>` type-schema**: `generate_field_schema()` now uses `segments.last()` instead of `get_ident()` to handle generic types. `Option<u32>` correctly generates `integer` schema (was falling through to `string`). `is_option_type()` fixed to use `segments.last()` for qualified path support (`std::option::Option<T>`).

### Added

- **Resource `title` attribute**: All three macro paths (`#[derive(McpResource)]`, `#[mcp_resource]`, `resource!{}`) now support `title = "..."` attribute. `HasResourceMetadata::title()` returns the configured value.
- `ServerEvent::Response` variant for distinguishing id-only SSE frames (responses to client-originated requests) from server-initiated requests. `StreamHandler` ignores these — they are handled by the normal request/response matching path.
- Null/missing `id` guard: Server requests without a valid `id` invoke the callback but do not emit a response (per JSON-RPC 2.0 spec).
- 11 new tests covering client response forwarding pipeline (unit + integration + mock transport).

## [0.3.7] - 2026-03-04

### Added

- **Tool annotations macro support**: `#[derive(McpTool)]`, `#[mcp_tool]`, and `tool!{}` now support `read_only`, `destructive`, `idempotent`, `open_world`, `title`, and `annotation_title` attributes — generates `ToolAnnotations` with camelCase JSON keys (`readOnlyHint`, `destructiveHint`, `idempotentHint`, `openWorldHint`) per MCP 2025-11-25
- `title` attribute on all three macro paths sets `Tool.title` (via `HasBaseMetadata`); `annotation_title` sets `ToolAnnotations.title` independently
- Boolean annotation type validation: `#[mcp_tool]` rejects wrong types (e.g., `read_only = "true"`) with a compile error

### Fixed

- Terminated sessions (after `DELETE /mcp`) now correctly reject subsequent POST and GET requests in both Streamable HTTP and legacy JSON transports

## [0.3.6] - 2026-03-03

### Fixed

- `#[mcp_tool]` and `#[derive(McpTool)]`: `Option<bool>`, `Option<u32>`, `Option<f64>`, `Vec<T>`, and `Option<Vec<T>>` parameters now generate correct JSON Schema types in `tools/list` input schemas (was incorrectly advertising `"type": "string"` for all generic-arg types)
- Fully-qualified paths (`std::option::Option<T>`, `std::vec::Vec<T>`) now correctly detected across all `is_option_type` checks

## [0.3.5] - 2026-03-03

### Added

- `McpClient::list_resource_templates()` and `list_resource_templates_paginated()` for `resources/templates/list` discovery

### Fixed

- `HttpTransport`: downgraded spurious session ID warning on `initialize` request from `warn!` to `debug!`

## [0.3.4] - 2026-03-03

### Fixed

- `HttpTransport::connect()` and `SseTransport::connect()` no longer send OPTIONS/HEAD pre-flight requests that fail with 405 (direct servers) or 502 (Lambda streaming servers) — connectivity failures now surface at `initialize` time instead of preflight time, matching MCP Inspector behavior
- `#[mcp_tool]` function-attribute macro: `Option<T>` parameters are now correctly excluded from the `required` array in the generated JSON schema (was incorrectly marking them as required unless `#[param(optional)]` was explicitly set)

### Changed

**DynamoDB Storage: camelCase Attribute Names (One-Way Migration):**
- New DynamoDB tables created by `turul-mcp-session-storage` and `turul-mcp-task-storage` now use camelCase attribute names (`sessionId`, `taskId`, `createdAt`, `lastActivity`, etc.) — aligning with DynamoDB convention
- Existing snake_case tables (`session_id`, `task_id`, `created_at`, etc.) are auto-detected via `describe_table()` key schema inspection and continue to work without any changes
- Per-table detection: session and events tables are detected independently, supporting mixed-convention deployments
- Read tolerance: non-key attributes written with either convention are readable via fallback lookup

**Rollback Contract (Breaking Storage Format):**
- This is a **one-way storage format change**. Once new tables are created with camelCase key schemas, pre-v0.3.4 code cannot read them (it has hardcoded snake_case key names)
- New code reads legacy snake_case tables: **Yes** (auto-detected)
- New code creates fresh tables with camelCase: **Yes**
- Old code reads legacy snake_case tables: **Yes** (unchanged)
- **Old code reads new camelCase tables: No — will fail**
- Rolling back to pre-v0.3.4 code after creating camelCase tables will break. Plan accordingly.

## [0.3.3] - 2026-03-01

### Fixed

- PostgreSQL task storage: `tasks.session_id` column type changed from `TEXT` to `VARCHAR(36)` to match `sessions.session_id` and `events.session_id`

## [0.3.2] - 2026-02-28

### Added

- `HasExecution` trait for per-tool task support declaration (follows `HasIcons` supertrait pattern)
- `task_support` attribute on `#[derive(McpTool)]` and `#[mcp_tool]` (`"optional"` | `"required"` | `"forbidden"`)
- `.execution()` builder method on `ToolBuilder`
- Build-time coherence guard rejects `taskSupport=required` without task runtime configured
- `tools/list` strips `execution` field when server has no tasks capability (truthful capability advertisement)
- `tools/call` with `params.task` returns `InvalidParameters` when server has no task runtime (was silent sync fallback)

### Changed

- **Breaking**: `HasExecution` added to `ToolDefinition` supertrait — manual tool impls must add `impl HasExecution for MyTool {}`

### Fixed

- `ToolDefinition::to_tool()` now populates `execution` field from trait (was hardcoded `None`)
- `tools/call` rejects task-augmented requests to tools that don't declare `task_support` (was silently accepted)

## [0.3.1] - 2026-02-28

### Fixed

- `ToolSchemaExt::from_schemars()` now handles schemars v1 nullable type arrays (`"type": ["string", "null"]`) and `anyOf`/null patterns for `Option<T>` fields
- `from_schemars()` enforces `type: "object"` root schema validation per MCP protocol requirements
- `from_schemars()` resolves `$ref` references through both `$defs` and `definitions` maps (merged, not first-hit)

## [0.3.0] - 2026-02-26

### Added

**MCP 2025-11-25 Protocol Support:**
- `turul-mcp-protocol-2025-11-25` crate with full spec compliance (127+ protocol tests)
- `turul-mcp-protocol` alias now re-exports 2025-11-25 types (ADR-015)
- `Icon` struct (`src`, `mime_type`, `sizes`, `theme`) on tools, resources, prompts, resource templates, and implementations
- `Task` struct with `task_id`, `TaskStatus` (`Working`/`InputRequired`/`Completed`/`Failed`/`Cancelled`), `created_at`/`last_updated_at`, `ttl`, `poll_interval`
- `ToolUse` and `ToolResult` content block variants
- `ToolExecution`, `ToolChoice`, `ToolChoiceMode` (`Auto`/`None`/`Required`)
- `TaskStatusNotification` and `ElicitationCompleteNotification`
- URL elicitation mode (`ElicitRequestURLParams`) alongside existing form mode
- `$schema` field on `ElicitationSchema`
- `tools` field on `CreateMessageParams` for sampling with tools
- `ModelHint { name }` struct (replaces closed enum)
- `Implementation` gains `description` and `website_url` fields
- Structured `TasksCapabilities` with `list`, `cancel`, `requests` sub-fields

**Task Storage (`turul-mcp-task-storage` crate):**
- `TaskStorage` trait with zero-Tokio public API
- `InMemoryTaskStorage` with state machine enforcement
- SQLite backend (`SqliteTaskStorage`) — optimistic locking, `julianday()` TTL, background cleanup
- PostgreSQL backend (`PostgresTaskStorage`) — `version` column optimistic locking, JSONB, partial index for stuck tasks
- DynamoDB backend (`DynamoDbTaskStorage`) — conditional writes, GSIs, native TTL, base64 cursors
- 11-function parity test suite shared across all backends
- Feature flags: `sqlite`, `postgres`, `dynamodb` (each opt-in with Tokio)

**Task Runtime & Executor:**
- `TaskExecutor` trait and `TokioTaskExecutor` in `turul-mcp-server`
- `CancellationHandle` for cooperative task cancellation
- `TaskRuntime` with `::new(storage, executor)`, `::with_default_executor(storage)`, `::in_memory()` constructors
- Server handlers for `tasks/get`, `tasks/list`, `tasks/cancel`, `tasks/result` (blocks until terminal per spec)
- Auto-capability advertisement via `McpServer::builder().with_task_runtime()`

**Task Examples:**
- `tasks-e2e-inmemory-server` — task-enabled MCP server with `slow_add` tool
- `tasks-e2e-inmemory-client` — full task lifecycle client (create, poll, cancel, result)
- `client-task-lifecycle` — task API demonstration
- `task-types-showcase` — print-only demo of Task, TaskStatus, TaskMetadata, CRUD types

**Lambda Examples:**
- `lambda-authorizer` — API Gateway REQUEST authorizer with wildcard methodArn for MCP Streamable HTTP

**README Testing Infrastructure:**
- `skeptic` crate for automated markdown code block testing
- README.md files validated as part of `cargo test` suite

### Changed

**Protocol Types (Breaking):**
- `CreateMessageResult` flattened — `role` and `content` at top level (no `message` wrapper)
- `Role` enum: only `User` and `Assistant` (removed `System` variant; system prompts use `systemPrompt` field)
- `ProgressNotificationParams.progress`: `f64` (was `u64`)
- `icon` fields renamed to `icons: Option<Vec<Icon>>` (singular string → plural object array)
- `HasIcon` trait renamed to `HasIcons`; `HasSamplingTools` trait added
- Notification method strings use underscores (`notifications/tools/list_changed`) per spec; JSON capability keys remain camelCase (`listChanged`)
- Default protocol version is 2025-11-25 everywhere; backward-compat 2025-06-18 paths annotated with `// Intentional`

**Test Infrastructure:**
- 1,560+ workspace tests passing, 98 doctests, zero warnings
- Test binaries reduced from 155 to 43 via consolidation (Phase F)
- Root integration tests: 39 → 8 binaries (5 consolidated in `tests/consolidated/` + 3 standalone)
- Sub-crate integration tests: 24 → 7 binaries (`tests/*/tests/all.rs` with `#[path]` imports)
- Derive crate integration tests moved to workspace root (2 binaries eliminated)

**Examples:**
- 58 active examples (up from 42+ in v0.1.0), 25 archived
- 12 core crates in workspace

**Documentation:**
- README narrative updated to reflect spec-pure protocol crate design
- All 20+ protocol crate README code examples tested and verified
- Documentation accuracy fixes across READMEs, ADRs, and compliance reports (repo URL, config field names, notification method strings, version references, port numbers)
- CHANGELOG duplicate `[0.2.0]` sections merged
- ADR-009 updated with `V2025_03_26` and `V2025_11_25` protocol versions
- ADR-004 status updated from CRITICAL to Accepted (Implemented)
- Stale MIGRATION_0.2.1.md references removed workspace-wide

### Fixed

- Sampling server README: removed `System` role, fixed `ModelHint` to object form, corrected snake_case JSON fields to camelCase
- Session storage README: corrected config field names (`session_timeout_minutes`, `database_url`, `PostgresConfig`)
- Compliance reports marked as historical with accurate resolution status
- Client README compatibility list now includes 2025-11-25
- Protocol alias ADR updated from 2025-06-18 to 2025-11-25
- Notification method strings in ADR-005 and E2E test plan corrected to `list_changed`

## [0.2.1] - 2025-10-08

### Breaking Changes

**Schemars Integration (Detailed Schema Generation):**
- **BREAKING**: Tool output types MUST now derive `schemars::JsonSchema`
- **Impact**: Tools with custom output types generate detailed schemas with full property information
- **Migration**: Add `#[derive(JsonSchema)]` to all tool output types:
  ```rust
  use schemars::JsonSchema;

  #[derive(Serialize, Deserialize, JsonSchema)]  // Added JsonSchema
  struct MyOutput {
      result: f64,
      message: String,
  }
  ```
- **Benefit**: All tools now provide detailed schemas in `tools/list` with property names, types, and descriptions
- **Note**: `schemars` is already a workspace dependency - no Cargo.toml changes needed

**Framework Trait Reorganization (Protocol Crate Purity):**
- **BREAKING**: All framework traits moved from `turul-mcp-protocol` to `turul-mcp-builders::traits`
- **BREAKING**: `HasNotificationPayload::payload()` now returns `Option<Value>` (owned) instead of `Option<&Value>` (reference)
- **Impact**: Protocol crate is now 100% MCP spec-pure (no framework-specific code)
- **Migration**: Update imports to use preludes:
  ```rust
  // Before
  use turul_mcp_protocol::{ToolDefinition, ResourceDefinition};

  // After
  use turul_mcp_builders::prelude::*;  // or turul_mcp_server::prelude::*
  ```
- **Migration Guide**: See the breaking changes listed above for step-by-step migration instructions

### Fixed

**Critical Notification Payload Regression:**
- Fixed all notification types returning `None` for payloads (data loss bug)
- Base Notification now properly serializes `params.other` and `_meta`
- ProgressNotification now preserves progressToken, progress, total, message, _meta
- ResourceUpdatedNotification now preserves uri, _meta
- CancelledNotification now preserves requestId, reason, _meta
- All list-changed notifications now preserve _meta fields
- Added 18 comprehensive tests validating notification payload correctness

### Changed

**Framework Trait Locations:**
- Moved 10 trait hierarchies (~1200 LOC) from protocol to builders crate
- All protocol type implementations now in `turul-mcp-builders/src/protocol_impls.rs`
- Derive macros updated to generate correct trait signatures
- All examples and tests updated to use new import paths

## [0.2.0] - 2025-10-05

### Added

**MCP 2025-06-18 Specification:**
- Full compliance with MCP 2025-06-18 spec
- Session-Aware Resources: All resources now support `session: Option<&SessionContext>` parameter
- Sampling Validation Framework: `ProvidedSamplingHandler` for request validation
- SSE Streaming: Chunked transfer encoding with real-time notifications
- CLI Support: All test servers now support `--port` argument with dynamic binding
- Path Normalization: Traversal attack detection in roots validation
- Strict Lifecycle Mode: Optional strict session initialization enforcement

**Middleware System:**
- Complete middleware architecture for HTTP and Lambda transports
- `.middleware()` builder method on `McpServer` and `LambdaMcpServerBuilder`
- Transport-agnostic middleware execution (FIFO before dispatch, LIFO after)
- Session-aware middleware with `StorageBackedSessionView` and `SessionInjection`
- Error short-circuiting with semantic JSON-RPC error codes

**Middleware Examples:**
- `middleware-auth-server` - API key authentication (HTTP)
- `middleware-auth-lambda` - API key authentication (AWS Lambda)
- `middleware-logging-server` - Request timing and tracing
- `middleware-rate-limit-server` - Per-session rate limiting

**Testing Infrastructure:**
- Shared verification utilities (`tests/shared/bin/wait_for_server.sh`)
- Test server bin targets in all test packages (tools, prompts, resources, sampling, roots, elicitation)
- Comprehensive example verification suite (5 phases, 31 servers)
- Session lifecycle compliance: `notifications/initialized` in all e2e tests

### Changed

- **Resource Trait**: Updated `read()` signature to include session parameter
- **Tool Output**: Tools with `outputSchema` automatically include `structuredContent`
- **Error Handling**: Session lifecycle violations use `SessionError` type
- **Pagination**: Reject `limit=0` to prevent stalls
- **HTTP Transport**: Protocol-based routing (≥2025-03-26 uses streaming, ≤2024-11-05 uses buffered)
- SSE keepalives use comment syntax for better client compatibility
- DynamoDB queries use strongly consistent reads
- Lambda `LambdaMcpHandler` now cached globally (preserves DynamoDB client, StreamManager, middleware instances)
- Test packages updated to Rust edition 2024 and tokio version "1"
- Middleware stack execution order documented (FIFO/LIFO)

### Fixed

**Examples (4 bugs fixed):**
- pagination-server: Database unique constraint error (email generation duplicates)
- comprehensive-server: Missing resources and prompts registration
- audit-trail-server: SQLite connection URL missing protocol and create mode
- All 30/31 examples now verified working (96.8% passing, 1 skipped for PostgreSQL)

**Protocol & Core:**
- SSE resumability: Keepalive events preserve Last-Event-ID for proper reconnection
- MCP Inspector compatibility: Events use standard `event: message` format
- Lambda notifications: DynamoDB consistent reads fix race condition
- Lambda handler caching: Global `OnceCell` preserves handler instance (DynamoDB client, StreamManager, middleware) across invocations
- Tool output: Schema and runtime field names now consistent
- CamelCase: Proper acronym handling (GPS → gps, HTTPServer → httpServer)
- Lambda compilation: Fixed `LambdaError::Config` reference
- **TestServerManager**: Blocking wait for process termination, prevents zombie processes
- **Session Tests**: Correct response structure (`output` vs `value`)
- **Prompt Arguments**: Fix argument name mismatches in test expectations
- **MCP Inspector**: Enable compatibility with MCP Inspector and FastMCP clients
- **Zero-Config**: Correct output field expectations for derived tools
- **Borrow Checker**: Resolve errors in `roots_derive` macro

**Code Quality:**
- Fixed 14 collapsible_if clippy warnings using Rust 2024 let-chain syntax
- Fixed unused variable warnings in test suite
- Fixed useless type conversions in Lambda tests
- All clippy warnings addressed (100% clean workspace builds with `-D warnings`)

**Verification Infrastructure:**
- Scripts use deterministic 15s polling instead of fixed sleeps
- Pre-built binaries eliminate compilation timeouts
- SKIPPED tracked separately from PASSED (no hidden failures)
- Build errors properly diagnosed with detailed logs

### Examples
- Restored `roots-server` with clap CLI (108 lines, down from 512)
- Updated `elicitation-server` with multi-path data loading
- Updated `sampling-server` with dynamic port binding
- Updated `pagination-server` with proper SQLite URI (`?mode=rwc`)
- All 31 core examples verified and working

### Documentation

- README middleware section with examples and testing commands
- AGENTS.md middleware guidance with ADR 012 reference
- Doctests passing: turul-mcp-derive (25/25), turul-mcp-protocol (7/7)
- Complete verification run documented with bug fixes and runbook
- Middleware testing scripts: `test_middleware_live.sh` and Lambda examples
- Updated CLAUDE.md with session-aware patterns
- Updated EXAMPLES.md with validation results
- Added curl and jq to auto-approved commands
- Comprehensive test coverage documentation

### Tests

- 440+ unit tests passing (161 integration tests across 20 test suites)
- 30/31 examples verified (Phases 1-5: 100% passing)
- Middleware parity tests verify HTTP/Lambda consistency
- All critical functionality validated

## [0.1.0] - Initial Release

### Added
- Core MCP server framework
- Tool creation patterns (function, derive, builder, manual)
- Resource management with templates
- Prompt generation system
- Session management with multiple storage backends
- HTTP transport layer
- Client library
- Builder patterns
- AWS Lambda support
- 42+ working examples

[Unreleased]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.12...HEAD
[0.3.12]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.11...v0.3.12
[0.3.11]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.10...v0.3.11
[0.3.10]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.9...v0.3.10
[0.3.9]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.8...v0.3.9
[0.3.8]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.7...v0.3.8
[0.3.7]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.6...v0.3.7
[0.3.6]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.5...v0.3.6
[0.3.5]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.4...v0.3.5
[0.3.4]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.3...v0.3.4
[0.3.3]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.2...v0.3.3
[0.3.2]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/aussierobots/turul-mcp-framework/releases/tag/v0.1.0
