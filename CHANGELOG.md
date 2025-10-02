# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.1] - 2025-10-03

### Fixed
- **CRITICAL: MCP Resumability with SSE Keepalives**: Fixed resumable SSE path to use comment-style keepalives (`": keepalive\n\n"`) instead of events with `id: 0`. Previously, keepalives included `id: 0` which reset clients' Last-Event-ID, causing full event replay on reconnection (violating MCP resumability spec). Now keepalives preserve Last-Event-ID for proper event resumption. Affected files: `stream_manager.rs`, `traits.rs` (SseEventInfo::format), test expectations updated.
- **MCP Inspector SSE Compatibility**: Changed all SSE events to use standard `event: message` type instead of custom event types. JavaScript EventSource API only processes `event: message` or omitted event lines. This fix ensures all notifications (including `notifications/progress`) are visible in MCP Inspector.
- **Lambda DynamoDB Notification Timing**: Added `.consistent_read(true)` to DynamoDB queries in `get_recent_events()` and `get_events_after()`. Fixes race condition where notifications worked on reconnect but not initial Lambda invocation due to eventual consistency.
- **POST SSE Response Timing**: Removed unnecessary 50ms sleep in `create_post_sse_stream()` that was a workaround, not a proper fix. Tool execution is fully awaited, so notifications are immediately available with consistent reads.
- **Output Field Schema/Runtime Consistency**: Fixed bug where `tools/list` schema and `tools/call` structuredContent used different field names when `output = Type` specified without explicit `output_field`. Schema generation and runtime wrapping now consistently use the same field name derived from the type.
- **Acronym CamelCase Conversion**: Fixed awkward camelCase conversion for all-caps acronyms. `LLH` now converts to `llh` (not `lLH`), `GPS` to `gps` (not `gPS`). Leading acronyms in mixed names also handled correctly: `HTTPServer` → `httpServer`.
- **Lambda Compilation**: Fixed `LambdaError::Config` → `LambdaError::Configuration` in builder.rs

### Changed
- **SSE Event Formatting**: Keepalive events now use SSE comment syntax (`: keepalive`) instead of `event: ping` for better client compatibility.
- **DynamoDB Consistency**: Event queries now use strongly consistent reads (2x RCU cost) to guarantee notification visibility.
- **Code Quality**: Fixed all 156 clippy warnings across workspace (156 → 0, 100% clean)
  - Replaced `get().is_none()` with idiomatic `!contains_key()` (2 instances)
  - Collapsed nested if statements using let-chain syntax (56 instances)
  - Used `std::io::Error::other()` for concise error creation (1 instance)
  - Replaced `let _ = future` with explicit `drop()` for async clarity (1 instance)
  - Removed redundant closure wrappers (3 instances)
  - Fixed borrowed expression warnings - removed unnecessary `&` (10 instances)
  - Fixed length comparison warnings - use `.is_empty()` instead of `.len() > 0` (6 instances)
  - Removed redundant imports (6 instances)
  - Replaced `unwrap_or_else(T::default)` with `unwrap_or_default()` (1 instance)
  - Fixed method comparison to use dereference instead of reference (1 instance)
  - Replaced match patterns with dereferenced value for cleaner code (3 instances)
  - Used `is_err()` instead of `if let Err(_)` pattern matching (1 instance)
  - Replaced `.min().max()` with `.clamp()` for range bounds (3 instances)
  - Used `Option::map` instead of manual if-let-Some mapping (1 instance)
  - Removed useless `format!()` macro for static strings (1 instance)
  - Replaced `vec![]` with array `[]` for static data (1 instance)
  - Added `#[allow(clippy::too_many_arguments)]` for Lambda handler constructors (2 instances)
  - Fixed empty line after outer attribute in test documentation (1 instance)
  - Replaced manual prefix stripping with `strip_prefix()` (1 instance)
  - Added `#[allow(dead_code)]` for test-only struct fields (4 structs, 7 fields)
  - Prefixed unused test variables with underscore (2 instances)
  - Added `#[allow(clippy::upper_case_acronyms)]` for domain-specific types (2 instances)

### Documentation
- **Doctests**: All doctests now passing in core crates
  - turul-mcp-derive: 25/25 doctests passing ✅
  - turul-mcp-protocol-2025-06-18: 7/7 doctests passing (7 intentionally ignored) ✅

### Tests
- **Phase 7: Integration Tests Validation**: Fixed 9 test failures across 4 test files, all 161 integration tests now passing
  - `tests/mcp_specification_compliance.rs`: Fixed URI validation for empty scheme paths and capabilities JSON path (3 failures → 9 tests passing)
  - `tests/session_context_macro_tests.rs`: Fixed zero-config tool output structure assertions (3 failures → 8 tests passing)
  - `tests/mcp_tool_compliance.rs`: Fixed test to use CompliantCountTool instead of NonCompliantCountTool (1 failure → 8 tests passing)
  - `tests/streamable_http_e2e.rs`: Fixed missing session ID status code and progress_tracker parameters (2 failures → 17 tests passing)
- Updated all SSE-related tests to expect `event: message` format
- Added keepalive-specific test cases for comment syntax
- All 161 integration tests passing across 20 core test suites

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
