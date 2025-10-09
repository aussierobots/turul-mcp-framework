# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
- **Migration Guide**: See [MIGRATION_0.2.1.md](MIGRATION_0.2.1.md) for complete step-by-step migration instructions

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

### Changed

- SSE keepalives use comment syntax for better client compatibility
- DynamoDB queries use strongly consistent reads
- Lambda `LambdaMcpHandler` now cached globally (preserves DynamoDB client, StreamManager, middleware instances)
- Test packages updated to Rust edition 2024 and tokio version "1"
- Middleware stack execution order documented (FIFO/LIFO)

### Documentation

- README middleware section with examples and testing commands
- AGENTS.md middleware guidance with ADR 012 reference
- Doctests passing: turul-mcp-derive (25/25), turul-mcp-protocol (7/7)
- Complete verification run documented with bug fixes and runbook
- Middleware testing scripts: `test_middleware_live.sh` and Lambda examples

### Tests

- Fixed 9 integration test failures
- All 161 integration tests passing across 20 test suites
- 30/31 examples verified (Phases 1-5: 100% passing)
- Middleware parity tests verify HTTP/Lambda consistency

## [0.2.0] - 2025-10-01

### Added
- **MCP 2025-06-18 Specification**: Full compliance with latest MCP spec
- **Session-Aware Resources**: All resources now support `session: Option<&SessionContext>` parameter
- **Sampling Validation Framework**: `ProvidedSamplingHandler` for request validation
- **SSE Streaming**: Chunked transfer encoding with real-time notifications
- **CLI Support**: All test servers now support `--port` argument with dynamic binding
- **Path Normalization**: Traversal attack detection in roots validation
- **Strict Lifecycle Mode**: Optional strict session initialization enforcement

### Changed
- **Resource Trait**: Updated `read()` signature to include session parameter
- **Tool Output**: Tools with `outputSchema` automatically include `structuredContent`
- **Error Handling**: Session lifecycle violations use `SessionError` type
- **Pagination**: Reject `limit=0` to prevent stalls
- **HTTP Transport**: Protocol-based routing (≥2025-03-26 uses streaming, ≤2024-11-05 uses buffered)

### Fixed
- **TestServerManager**: Blocking wait for process termination, prevents zombie processes
- **Session Tests**: Correct response structure (`output` vs `value`)
- **Prompt Arguments**: Fix argument name mismatches in test expectations
- **MCP Inspector**: Enable compatibility with MCP Inspector and FastMCP clients
- **Zero-Config**: Correct output field expectations for derived tools
- **Borrow Checker**: Resolve errors in `roots_derive` macro

### Examples
- Restored `roots-server` with clap CLI (108 lines, down from 512)
- Updated `elicitation-server` with multi-path data loading
- Updated `sampling-server` with dynamic port binding
- Updated `pagination-server` with proper SQLite URI (`?mode=rwc`)
- All 31 core examples verified and working

### Tests
- 440+ unit tests passing
- 31/35 examples verified (89%)
- Phase 1-4, 6, 8 verification: 100% passing
- All critical functionality validated

### Documentation
- Updated CLAUDE.md with session-aware patterns
- Updated EXAMPLES.md with validation results
- Added curl and jq to auto-approved commands
- Comprehensive test coverage documentation

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

[0.2.1]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/aussierobots/turul-mcp-framework/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/aussierobots/turul-mcp-framework/releases/tag/v0.1.0
