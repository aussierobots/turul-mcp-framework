# MCP Framework - Working Memory

**Last Updated**: 2026-02-08
**Framework Version**: v0.2.2
**Purpose**: Track current work, active context, and immediate priorities

---

## Current Status: v0.2.2 Development

**Status**: IN PROGRESS
**Branch**: 0.2.2 (development)
**Test Status**: All tests passing, zero warnings
**Focus**: MCP 2025-11-25 protocol crate and framework integration

### MCP 2025-11-25 Migration (2026-02-07)

A new protocol crate `turul-mcp-protocol-2025-11-25` has been created as a separate crate
(not feature-flagged) to cleanly support the new spec version alongside 2025-06-18.
See ADR 015 for the rationale behind the crate splitting strategy.

**Phase 2 - Icons: COMPLETE**
- `Icon` struct in `icons.rs` with `src`, `mime_type`, `sizes`, `theme` fields
- `icons: Option<Vec<Icon>>` field on Tool, Resource, Prompt, ResourceTemplate, Implementation
- Serializes as JSON object array (not transparent string)
- Tests: icon creation, serialization, round-trip, omission when None

**Phase 3 - URL Elicitation: COMPLETE**
- StringFormat::Uri variant for string schema format constraints
- StringSchema::url() constructor for URL-formatted strings
- PrimitiveSchemaDefinition::url() and url_with_description() convenience constructors
- ElicitationBuilder::url_input() for one-call URL elicitation requests
- Tests: serialization, builder patterns, spec compliance

**Phase 4 - Sampling Tools: COMPLETE**
- `tools` field on CreateMessageParams (Optional<Vec<Tool>>)
- CreateMessageParams::with_tools() builder method
- CreateMessageRequest::with_tools() forwarding method
- Tests: serialization with tools field

**Phase 5 - Tasks: COMPLETE**
- TaskStatus enum (Working, InputRequired, Completed, Failed, Cancelled) with lowercase serialization
- `Task` struct with `task_id`, `status`, `status_message`, `created_at`, `last_updated_at`, `ttl`, `poll_interval`, `_meta`
- GetTask, CancelTask, ListTasks (no CreateTask — tasks created via task-augmented request params)
- `TaskMetadata { ttl }` on CallToolParams, CreateMessageParams, ElicitCreateParams
- ListTasksResult with pagination (nextCursor)
- All trait impls: HasMethod, HasParams, HasData, HasMeta, RpcResult, Params, HasMetaParam
- 20+ dedicated tests covering serialization, round-trips, camelCase compliance

**Phase 6 - Framework Integration: COMPLETE**
- `turul-mcp-protocol` re-export alias now points to `turul-mcp-protocol-2025-11-25`
- Builder traits updated: `HasIcons`, `HasSamplingTools`, derive macros, declarative macros
- All cascade fixes applied across server, builders, derive, examples, tests
- 1,319 workspace tests passing (verified 2026-02-08)

**Protocol Crate Stats:**
- 127+ unit tests passing in protocol crate
- 1,319 workspace tests passing (verified 2026-02-08)
- Zero compiler warnings
- McpVersion::V2025_11_25 with feature detection methods
- Complete version.rs with supports_tasks(), supports_icons(), etc.

### Previous Completions (2025-10-10)

**Schemars Integration & Testing:**
- Mandatory JsonSchema requirement for tool output types
- Vec<T> output schema documentation (requires `output` attribute)
- Removed unused schema_provider module
- All schema tests passing (16 tests)
- CLAUDE.md updated with output type requirements

**Middleware System (Completed in v0.2.0):**
- Phase 1-4 complete (Core, HTTP, Lambda, Docs)
- All 17 middleware tests passing
- Lambda handler caching pattern documented

**Test Results:**
- 1,319 workspace tests passing (verified 2026-02-08)
- 127+ tests in turul-mcp-protocol-2025-11-25
- 16 schema integration tests passing
- 17 middleware tests passing
- 27 derive doctests passing
- Zero compiler warnings
- Clean workspace build

---

## Known Issues & Remaining Work

### Phase 6 - Framework Integration: COMPLETE (2026-02-08)

All protocol types for MCP 2025-11-25 are integrated into the framework stack:
1. **turul-mcp-protocol** - Re-export alias now points to 2025-11-25 crate
2. **turul-mcp-server** - Handler dispatch updated for all 2025-11-25 types
3. **turul-mcp-builders** - `HasIcons`, `HasSamplingTools`, all `Has*` trait cascade complete
4. **turul-mcp-derive** - Derive macros generate 2025-11-25-compliant output
5. **Examples** - Icon showcase, task types showcase, sampling-with-tools showcase created

### Documentation Gaps

1. **DOCUMENTATION_TESTING.md** - Can be merged into TESTING_GUIDE.md
2. **EXAMPLE_VERIFICATION_LOG.md** - KEEP as operational documentation
3. **TODO_TRACKER.md** - Middleware status needs updating to COMPLETE

---

## Next Steps

### v0.2.2 Release Priorities

1. **README testing** - Skeptic-based markdown code block testing
2. **CHANGELOG.md** - Update with 2025-11-25 protocol support details
3. **Publish** - crates.io release preparation

### Future Work (v0.3.0+)

- resources/subscribe - Real-time resource updates
- Advanced pagination - Cursor-based navigation
- roots/list enhancements - Advanced filtering

---

## Framework Metrics

**Test Coverage:**
- **Total Tests**: 1,319 (verified 2026-02-08)
- **Unit Tests**: ~200+ (builders, protocol, server, derive)
- **Protocol 2025-11-25 Tests**: 121+ (tools, resources, prompts, tasks, elicitation, sampling, compliance)
- **Integration Tests**: ~170 (HTTP, Lambda, SSE, streaming)
- **E2E Tests**: ~90 (behavioral compliance, specification)
- **Doc Tests**: 27 (trait documentation examples)
- **Middleware Tests**: 17 (HTTP integration, Lambda parity)
- **Schema Tests**: 16 (schemars, output fields, Vec types)

**Code Quality:**
- **Compiler Warnings**: 0
- **Clippy Warnings**: 0
- **Doc Coverage**: 100% of public APIs documented
- **Example Coverage**: 40+ examples, all verified working

---

## Reference Documentation

**Key Files:**
- `HISTORY.md` - Archived completed work (notification fix, protocol purity, middleware)
- `TODO_TRACKER.md` - Detailed task breakdown and progress tracking
- `CHANGELOG.md` - User-facing changes and migration guides
- `CLAUDE.md` - Project guidelines and conventions for AI assistants
- `TESTING_GUIDE.md` - Testing strategy and commands

**ADRs (Architecture Decision Records):**
- ADR 001: Session Storage Architecture
- ADR 009: Protocol-Based Handler Routing
- ADR 012: Middleware Architecture
- ADR 013: Lambda Authorizer Integration
- ADR 014: Schemars Integration for Detailed Schemas
- ADR 015: MCP 2025-11-25 Protocol Crate Strategy

**Protocol Crates:**
- `turul-mcp-protocol-2025-11-25` - MCP 2025-11-25 spec types (aliased as `turul-mcp-protocol`)
- `turul-mcp-protocol-2025-06-18` - MCP 2025-06-18 spec types (previous version, retained for reference)

**Examples:**
- 40+ working examples covering all MCP areas
- 4 middleware examples (auth HTTP/Lambda, logging, rate-limiting)
- All verified via automated scripts

---

## Development Workflow

**Key Commands:**
```bash
# Build and test
cargo build --workspace
cargo test --workspace
cargo check --workspace
cargo clippy --workspace

# Test the new protocol crate specifically
cargo test --package turul-mcp-protocol-2025-11-25

# Run examples
cargo run --example minimal-server
cargo run --example middleware-auth-server

# MCP testing
cargo run --example client-initialise-server -- --port 52935
cargo run --example client-initialise-report -- --url http://127.0.0.1:52935/mcp
```

**Git Workflow:**
- Branch: `0.2.2` (development)
- Main branch: `main`
- Commits: Succinct messages

---

## Notes for Next Session

**Immediate Actions:**
1. Update CHANGELOG.md with 2025-11-25 protocol support
2. README testing with skeptic
3. crates.io publish preparation

**Architecture Decisions:**
- See ADR 015 for protocol crate splitting rationale
- Version negotiation happens at the HTTP transport layer (extends ADR 009)
- MCP 2025-11-25 migration completed across all 7 phases (2026-02-08)
