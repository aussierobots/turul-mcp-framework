# Working Memory

**Last Updated**: 2026-02-26
**Version**: v0.3.0 (branch: `0.3.0`)
**Tests**: 1,560+ workspace tests, 43 test binaries, zero warnings
**Examples**: 58 active, 25 archived

---

## Current Status

All major v0.3.0 work is complete. The framework fully supports MCP 2025-11-25.

### Completed (v0.3.0)

| Area | Status | Details |
|------|--------|---------|
| MCP 2025-11-25 Protocol | Done | Icons, tasks, elicitation, sampling tools, tool execution, notifications |
| Framework Integration | Done | Protocol re-export, builder traits, derive macros, all examples updated |
| Task Storage (Phase A-C) | Done | `TaskStorage` trait, `InMemoryTaskStorage`, server handlers, capability auto-advertisement |
| Durable Backends (Phase D) | Done | SQLite, PostgreSQL, DynamoDB + 11-function parity test suite |
| Task Executor (Phase E) | Done | `TaskExecutor` trait, `TokioTaskExecutor`, `TaskRuntime`, `CancellationHandle` |
| Test Optimization (Phase F) | Done | 155 → 43 test binaries, ~7:41 workspace test time |
| Governance Review | Done | 63 files dual-reviewed (spec-auditor + arch-reviewer), all PASS |

### Key Architecture

- `turul-mcp-protocol` re-exports `turul-mcp-protocol-2025-11-25` (see ADR 015)
- Task storage: 4 backends (InMemory, SQLite, PostgreSQL, DynamoDB) with shared parity tests
- Task runtime: storage (trait) / executor (`turul-mcp-server`) / runtime (`TaskRuntime`) three-layer split
- Test binaries consolidated via `autotests = false` + `[[test]]` entries (Phase F)

---

## Remaining Work

See `TODO_TRACKER.md` for the full tracked list. Key items:

- **CHANGELOG.md** — Expand [0.3.0] with full feature list
- **crates.io publish preparation** — Version bumps, dependency audit
- **Durable task storage examples** — SQLite, PostgreSQL, DynamoDB (backends done, no examples)
- **Verification phases 6-8** — Scripts exist, never executed

### Known Issues

- `tasks/result` error path wraps original error code in `McpError::ToolExecutionError` (loses original JSON-RPC error code)

---

## Quick Reference

```bash
cargo test --workspace                                    # Full suite (1,560+ tests)
cargo test -p turul-mcp-task-storage --all-features       # Task storage (62 pass, 31 ignored)
cargo test -p turul-mcp-task-storage --features sqlite    # SQLite backend (57 tests)
cargo check -p turul-mcp-task-storage --no-default-features  # Verify zero-Tokio public API
cargo test --package turul-mcp-protocol-2025-11-25        # Protocol crate (127+ tests)
```

**Key ADRs**: 001 (Session Storage), 009 (Handler Routing), 012 (Middleware), 013 (Lambda Auth), 014 (Schemars), 015 (Protocol Crate Strategy), 016 (Task Storage), 017 (Runtime-Executor Boundary), 018 (Pagination Cursor)
