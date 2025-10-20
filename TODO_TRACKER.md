# TODO Tracker

**Last Updated**: 2025-10-21
**Purpose**: Track active tasks and priorities for turul-mcp-framework development

**For completed work history, see `HISTORY.md`**

---

## Current Status

**Framework Version**: v0.2.2
**Branch**: 0.2.2
**Test Status**: ✅ 450+ tests passing, zero warnings
**Build Status**: ✅ All 40+ crates compile cleanly

**Recent Completions**:
- ✅ Schemars Integration & Testing (2025-10-09) - 11 tests with regression prevention
- ✅ Protocol Crate Purity (2025-10-07) - All framework traits moved to builders crate
- ✅ Notification Payload Fix (2025-10-08) - All notifications properly serialize data
- ✅ Middleware Phase 1 & 3 (2025-10-05) - Core infrastructure and Lambda integration

---

## ✅ P0: RESOLVED - Vec<T> Schema Documentation

**Status**: ✅ RESOLVED (2025-10-09) - Test documentation clarified, not a bug
**Resolution**: Users must specify `output = Vec<T>` attribute for Vec return types

**Finding**: The "bug" was actually a missing `output` attribute in the test. The framework correctly requires users to explicitly declare their output type when it's not `Self`.

**Correct Usage** (now documented in tests):
```rust
#[derive(McpTool)]
#[tool(
    name = "search_items",
    description = "Search for items",
    output = Vec<SearchResult>  // ← REQUIRED for Vec return types
)]
struct SearchTool {
    query: String,
    limit: usize,
}
```

**Why This Is Correct Design**:
1. **Type Safety**: Macros can't inspect `execute` method return types at compile time
2. **Explicit > Implicit**: Clear declaration of what the tool returns
3. **Consistent Pattern**: All custom output types require the `output` attribute

**Tests Updated**:
- ✅ `tests/mcp_vec_result_schema_test.rs` - Now has `output = Vec<SearchResult>`
- ✅ All 3 tests passing (schema generation, actual return value, validation)
- ✅ Documentation clarified in test file header

**Framework Behavior** (working as designed):
- With `output = Vec<T>`: Generates array schema with detailed item type
- Without `output`: Falls back to Self schema (tool's input parameters)

---

## 🎯 P0: Schemars Coverage Gaps

**Status**: ✅ COMPLETE (2025-10-09)
**Outcome**: 11 comprehensive tests with regression prevention

### What Was Completed

✅ **Test Registration**: All 3 workspace schemars tests now registered in `tests/Cargo.toml`
✅ **Regression Prevention**: Added comprehensive schema assertions to prevent regression to generic objects
✅ **Coverage**: 11 tests total across workspace and derive crate

**Test Coverage**:
- `test_schemars_derive.rs` (2 tests) - Tool execution + detailed schema assertions
- `schemars_detailed_schema_test.rs` (2 tests) - Nested structures (6 top + 6 nested + 5 array fields)
- `schemars_optional_fields_test.rs` (2 tests) - Optional fields with anyOf resolution
- `schemars_integration_test.rs` (5 tests) - Basic compilation, nested, optional, serialization

**Key Improvements**:
- Tests now **fail if schema becomes generic** (panic on missing properties)
- Assert specific field names present (e.g., `value`, `message`)
- Verify field types are detailed (`Number`, `String`, not generic `Object`)
- All tests run during `cargo test --workspace`

### Remaining Known Limitations (Documented in ADR-014)

These edge cases are **documented but not blocking**:

1. **HashMap/BTreeMap fields** - May show as generic `{type: "object"}` without value type details
2. **Complex $refs** - Unresolvable references fall back to generic object schema

**Decision**: These are acceptable limitations. The converter prioritizes safety (never crashes) over perfect coverage. Users can still use these types; schemas just won't show full detail.

---

## ✅ P1: Middleware System - COMPLETE

**Status**: ✅ **COMPLETE** (2025-10-10)
**Released**: v0.2.0
**Documentation**: ADR 012, README.md, 4 working examples

### Completed Phases

- ✅ **Phase 1: Core Infrastructure** - Complete (McpMiddleware trait, MiddlewareStack, 17 tests)
- ✅ **Phase 2: HTTP Integration** - Complete (both handlers use run_middleware_and_dispatch pattern)
- ✅ **Phase 3: Lambda Integration** - Complete (transport parity verified with tests)
- ✅ **Phase 4: Examples & Documentation** - Complete (4 examples, ADR 012, README section)

### Implementation Details (Historical Record)

**Note:** Phase 1.5 was NOT needed - SessionView already existed in turul-mcp-session-storage crate, avoiding circular dependencies.

**Actual Implementation:**
- Middleware implemented directly in turul-http-mcp-server crate
- Re-exported via turul-mcp-server for user convenience
- SessionView trait from turul-mcp-session-storage used for middleware
- StorageBackedSessionView adapter bridges storage to SessionView
- Both HTTP handlers (StreamableHttpHandler, SessionMcpHandler) use run_middleware_and_dispatch pattern
- Lambda handler uses same middleware infrastructure (transport parity)

**Test Coverage:**
- 17 middleware tests (unit + integration)
- 1 Lambda parity test
- Error code mapping verified (-32001, -32002, -32003)
- Session injection verified working
- Handler integration tests confirm both handlers use middleware

**Examples:**
- middleware-auth-server (HTTP with X-API-Key)
- middleware-auth-lambda (Lambda with OnceCell handler caching)
- middleware-logging-server (Request timing/tracing)
- middleware-rate-limit-server (Per-session limits with retry_after)

**Documentation:**
- ADR 012: Middleware Architecture (comprehensive)
- README.md: Middleware section with examples
- CHANGELOG.md: v0.2.0 feature announcement
- All 4 examples have detailed docstrings

---

## 🔧 P1: README Testing with Skeptic (v0.2.2)

**Status**: 🚧 IN PROGRESS
**Branch**: 0.2.2
**Impact**: Documentation quality and crates.io reliability
**Estimated**: 1-2 days

**Context**: During v0.2.1 release preparation, discovered that README.md files are not tested by `cargo test --doc`. The turul-mcp-protocol-2025-06-18 README contains outdated version numbers and architectural misrepresentations that could mislead users.

**Goals**:
1. Add skeptic crate for markdown code block testing
2. Configure skeptic for all published crate READMEs
3. Fix turul-mcp-protocol-2025-06-18/README.md accuracy issues
4. Prevent future documentation drift

**Tasks**:
- [ ] Add skeptic to workspace dependencies
- [ ] Configure skeptic build.rs for protocol crate
- [ ] Add skeptic test runner to protocol crate tests
- [ ] Fix README.md version numbers (0.2.0 → 0.2.1, lines 28 and 619)
- [ ] Rewrite README.md architectural narrative:
  - [ ] Remove "trait-based architecture" marketing (lines 6, 10, 15, 69-92, 629-635)
  - [ ] Emphasize "spec-pure concrete types" as main value
  - [ ] Clarify that framework traits are in turul-mcp-builders
  - [ ] Update "Why Choose This Crate" section to reflect actual design
- [ ] Verify all 20+ README code examples compile with skeptic
- [ ] Run full test suite to confirm no regressions
- [ ] Update CHANGELOG.md with documentation fixes
- [ ] Create PR for v0.2.2

**Success Criteria**:
- ✅ `cargo test` includes README.md code block validation
- ✅ All README examples compile and run
- ✅ Version numbers accurate across all documentation
- ✅ Architectural descriptions match actual implementation
- ✅ Zero warnings from skeptic

---

## 📋 P2: Future Work (Not Blocking v0.2.2)

### Session-Aware Resources

**Status**: 📝 DESIGN PHASE
**Impact**: Breaking change to McpResource trait
**Estimated**: 2-3 days

**Goal**: Add SessionContext parameter to `McpResource::read()` for personalized content

**Open Questions**:
- Backwards compatibility strategy?
- Derive macro updates required?
- Migration path for 30+ existing resource implementations?
- Performance implications of session lookups?

**Tasks** (when prioritized):
- [ ] Design backwards-compatible trait evolution
- [ ] Update McpResource trait signature
- [ ] Update #[derive(McpResource)] macro
- [ ] Update #[mcp_resource] attribute macro
- [ ] Update all resource examples
- [ ] Create migration guide
- [ ] Write session-aware resource patterns guide

### Advanced MCP 2025-06-18 Features

**Status**: 📝 SPEC TRACKING

Features defined in spec but not yet implemented:

- [ ] **resources/subscribe** - Real-time resource update notifications
  - SSE-based subscription mechanism
  - Resource change detection
  - Unsubscribe handling
- [ ] **Advanced Pagination** - Cursor-based navigation beyond basic impl
  - Cursor generation and validation
  - Efficient dataset traversal
  - Edge case handling (empty results, invalid cursors)
- [ ] **roots/list Enhancements** - Advanced filtering and permissions
  - Permission-based filtering
  - Complex root hierarchies
  - Dynamic root generation

---

## 📊 Metrics & Health

### Test Coverage

- **Total Tests**: 440+
- **Unit Tests**: ~200 (builders, protocol, server, derive)
- **Integration Tests**: ~150 (HTTP, Lambda, SSE, streaming)
- **E2E Tests**: ~90 (behavioral compliance, specification)
- **Doc Tests**: 11 (trait documentation examples)

### Build Performance

- **Protocol Crate**: 1.5s (spec-pure, minimal dependencies)
- **Builders Crate**: 2.5s (framework traits, builders)
- **Server Crate**: 4s (server infrastructure)
- **Full Workspace**: ~35s (40+ crates)

### Code Quality

- **Compiler Warnings**: 0
- **Clippy Warnings**: 0 (all 156 fixed in v0.2.0)
- **Doc Coverage**: 100% of public APIs documented
- **Example Coverage**: 30/31 verified working (96.8%)

---

## 🎯 Next Priorities

1. ✅ Schemars Integration & Testing (P0 - **COMPLETE** 2025-10-09)
2. ✅ Middleware Completion (P1 - **COMPLETE** 2025-10-10)
3. 🚧 README Testing with Skeptic (P1 - **IN PROGRESS** 2025-10-21)
4. 📝 Session-Aware Resources (P2 - design phase, breaking change)
5. 📝 Advanced MCP 2025-06-18 Features (P2 - spec tracking)

---

## 📚 Reference

**Documentation**:
- `WORKING_MEMORY.md` - Current work, active context, implementation plans
- `HISTORY.md` - Archived completed work (notification fix, protocol purity, middleware phase 1)
- `CHANGELOG.md` - User-facing changes and migration guides
- `MIGRATION_0.2.1.md` - v0.2.0 → v0.2.1 upgrade instructions
- `CLAUDE.md` - Project guidelines, conventions, auto-approved commands

**Repositories**:
- Main Repo: https://github.com/anthropics/turul-mcp-framework
- Issues: https://github.com/anthropics/turul-mcp-framework/issues
- MCP Spec: https://spec.modelcontextprotocol.io/

**Key Commands**:
```bash
# Development
cargo build --workspace
cargo test --workspace
cargo check --workspace
cargo clippy --workspace

# Specific tests
cargo test --test mcp_compliance_tests
cargo test --test notification_payload_correctness
cargo test --test tool_output_schema_methods

# Examples
cargo run --example minimal-server
cargo run --example tool-output-schemas
```
