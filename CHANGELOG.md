# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.1] - 2025-10-03

### Fixed
- **MCP Inspector SSE Compatibility**: Changed all SSE events to use standard `event: message` type instead of custom event types. JavaScript EventSource API only processes `event: message` or omitted event lines. This fix ensures all notifications (including `notifications/progress`) are visible in MCP Inspector.
- **Lambda DynamoDB Notification Timing**: Added `.consistent_read(true)` to DynamoDB queries in `get_recent_events()` and `get_events_after()`. Fixes race condition where notifications worked on reconnect but not initial Lambda invocation due to eventual consistency.
- **POST SSE Response Timing**: Removed unnecessary 50ms sleep in `create_post_sse_stream()` that was a workaround, not a proper fix. Tool execution is fully awaited, so notifications are immediately available with consistent reads.
- **Output Field Schema/Runtime Consistency**: Fixed bug where `tools/list` schema and `tools/call` structuredContent used different field names when `output = Type` specified without explicit `output_field`. Schema generation and runtime wrapping now consistently use the same field name derived from the type.
- **Acronym CamelCase Conversion**: Fixed awkward camelCase conversion for all-caps acronyms. `LLH` now converts to `llh` (not `lLH`), `GPS` to `gps` (not `gPS`). Leading acronyms in mixed names also handled correctly: `HTTPServer` → `httpServer`.

### Changed
- **SSE Event Formatting**: Keepalive events now use SSE comment syntax (`: keepalive`) instead of `event: ping` for better client compatibility.
- **DynamoDB Consistency**: Event queries now use strongly consistent reads (2x RCU cost) to guarantee notification visibility.

### Tests
- Updated all SSE-related tests to expect `event: message` format
- Added keepalive-specific test cases for comment syntax
- All 440+ tests passing

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
