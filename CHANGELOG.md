# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.1...HEAD
[0.3.1]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/aussierobots/turul-mcp-framework/releases/tag/v0.1.0
