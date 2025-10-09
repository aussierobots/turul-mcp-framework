# TODO Tracker

**Last Updated**: 2025-10-09
**Purpose**: Track active tasks and priorities for turul-mcp-framework development

**For completed work history, see `HISTORY.md`**

---

## Current Status

**Framework Version**: v0.2.1
**Branch**: 0.2.1 (stable)
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

## 🚧 P1: Middleware Completion (Blocked)

**Status**: ⏸️ BLOCKED on Phase 1.5 (SessionView abstraction)
**Estimated**: 3-4 days
**Owner**: TBD

### Current State

- ✅ **Phase 1: Core Infrastructure** - Complete (McpMiddleware trait, stack executor, unit tests)
- ✅ **Phase 3: Lambda Integration** - Complete (middleware parity tests)
- ⏸️ **Phase 1.5: SessionView Abstraction** - BLOCKED (need to start)
- ⏸️ **Phase 2: HTTP Integration** - BLOCKED (depends on 1.5)
- ⏸️ **Phase 4: Examples & Docs** - BLOCKED (depends on 2)

### Phase 1.5: SessionView Abstraction (1 day)

**Problem**: HTTP/Lambda handlers can't access SessionContext due to circular dependencies

**Solution**: Extract middleware to separate crate with SessionView trait abstraction

**Tasks**:
- [ ] Step 1: Define SessionView trait in turul-mcp-protocol (15 min)
  - Create `turul-mcp-protocol/src/session_view.rs`
  - Define trait methods: `session_id()`, `get_state()`, `set_state()`, `get_metadata()`
  - Export from lib.rs
- [ ] Step 2: Create turul-mcp-middleware crate (30 min)
  - Create `crates/turul-mcp-middleware/` directory
  - Move middleware files from turul-mcp-server
  - Change signatures to use `Option<&dyn SessionView>`
  - Add to workspace Cargo.toml
  - Depend on turul-mcp-protocol only
- [ ] Step 3: Implement SessionView in turul-mcp-server (15 min)
  - `impl SessionView for SessionContext`
  - Delegate to existing closure-based implementation
  - Add turul-mcp-middleware dependency
  - Re-export: `pub use turul_mcp_middleware as middleware`
- [ ] Step 4: Update transport dependencies (30 min)
  - turul-http-mcp-server → add turul-mcp-middleware dependency
  - turul-mcp-aws-lambda → add turul-mcp-middleware dependency
  - Update import paths
- [ ] Step 5: Verify no circular dependencies (15 min)
  - Run `cargo check --workspace`
  - Run middleware unit tests: `cargo test --package turul-mcp-middleware --lib`
  - Verify 8 middleware tests still pass

**Acceptance**:
- Middleware extracted to separate crate
- No circular dependencies
- All existing tests pass
- SessionView trait cleanly abstracts session access

### Phase 2: HTTP Integration (2 days)

**Critical Requirements**:
- ⚠️ MUST integrate into BOTH HTTP handlers:
  - `StreamableHttpHandler` (protocol ≥ 2025-03-26)
  - `SessionMcpHandler` (protocol ≤ 2024-11-05)
- Parse JSON-RPC body ONCE to extract method before middleware (avoid double-parse)
- `initialize` method gets `session = None`, all other methods get `session = Some(...)`
- Only persist SessionInjection if session exists (not for initialize)

**Tasks**:
- [ ] Add middleware_stack field to StreamableHttpHandler
- [ ] Add middleware_stack field to SessionMcpHandler
- [ ] Parse method from request body (lightweight single-field parse)
- [ ] Create RequestContext with method + HTTP headers as metadata
- [ ] Determine session: None if method == "initialize", Some(session) otherwise
- [ ] Hook `execute_before(&mut ctx, session_opt)` in both handlers
- [ ] Persist session injection (only if session exists)
- [ ] Hook `execute_after(&ctx, &mut result)` in both handlers
- [ ] Error conversion: MiddlewareError → McpError → JsonRpcError
- [ ] Pass middleware_stack from builder to both handlers
- [ ] Integration test: middleware runs for protocol ≥ 2025-03-26
- [ ] Integration test: middleware runs for protocol ≤ 2024-11-05
- [ ] Integration test: initialize method with session = None
- [ ] Integration test: error codes (-32001, -32002, -32003) match spec
- [ ] Integration test: session injection persists across requests

**Acceptance**:
- Middleware runs in both HTTP handlers
- initialize method works without session
- Session injection persists correctly
- All integration tests pass
- No regressions in existing HTTP tests

### Phase 4: Examples & Documentation (1-2 days)

**Examples**:
- [ ] `middleware-auth-server` - API key authentication
- [ ] `middleware-logging-server` - Request timing and tracing
- [ ] `middleware-rate-limit-server` - Per-session rate limiting

**Documentation**:
- [ ] ADR-XXX: Middleware Architecture
  - Why traits over function hooks
  - Why before/after pattern
  - Error handling strategy
- [ ] CLAUDE.md: Middleware section
  - Quick start guide
  - Example middleware implementation
  - Error code reference
- [ ] CHANGELOG.md: Middleware feature announcement
- [ ] README.md: Middleware quick start

**Acceptance**:
- All 3 examples compile and run
- Documentation covers common use cases
- ADR explains architectural decisions

---

## 📋 P2: Future Work (Not Blocking v0.2.1)

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
2. ⏸️ Middleware Completion (P1 - blocked on Phase 1.5 SessionView abstraction)
3. 📝 Session-Aware Resources (P2 - design phase, breaking change)
4. 📝 Advanced MCP 2025-06-18 Features (P2 - spec tracking)

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
