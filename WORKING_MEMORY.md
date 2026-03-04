# Working Memory

**Last Updated**: 2026-03-05
**Version**: v0.3.8 (branch: `main`)
**Tests**: 1,590+ workspace tests, 43 test binaries, zero warnings
**Examples**: 58 active, 25 archived

---

## Current Status

All major v0.3.0 work is complete. The framework fully supports MCP 2025-11-25.

### Published Releases

| Version | Date | Highlights |
|---------|------|-----------|
| v0.3.8 | 2026-03-05 | Client streaming response forwarding (P1 fix); JsonSchema `Option<T>` fix; resource `title` macro support |
| v0.3.7 | 2026-03-04 | ToolAnnotations macro support (`read_only`, `destructive`, `idempotent`, `open_world`, `title`, `annotation_title`) across all 3 macro paths; session termination fix |
| v0.3.6 | 2026-03-03 | Fix `Option<T>`/`Vec<T>` JSON Schema types in derive macros; qualified-path `is_option_type` fix |
| v0.3.5 | 2026-03-03 | `McpClient::list_resource_templates()` + `list_resource_templates_paginated()`; HTTP transport session ID warning fix |
| v0.3.4 | 2026-03-03 | HTTP/SSE preflight removal; `#[mcp_tool]` optional params fix; DynamoDB camelCase migration |
| v0.3.3 | 2026-03-01 | PostgreSQL task storage column type fix |
| v0.3.2 | 2026-02-28 | Per-tool task support (`HasExecution` trait, `task_support` attribute) |
| v0.3.1 | 2026-02-27 | Lambda task parity, task runtime builder API |
| v0.3.0 | 2026-02-26 | Full MCP 2025-11-25 support, task storage, durable backends, test optimization |

### Completed (v0.3.0+)

| Area | Status | Details |
|------|--------|---------|
| MCP 2025-11-25 Protocol | Done | Icons, tasks, elicitation, sampling tools, tool execution, notifications |
| Framework Integration | Done | Protocol re-export, builder traits, derive macros, all examples updated |
| Task Storage (Phase A-C) | Done | `TaskStorage` trait, `InMemoryTaskStorage`, server handlers, capability auto-advertisement |
| Durable Backends (Phase D) | Done | SQLite, PostgreSQL, DynamoDB + 11-function parity test suite |
| Task Executor (Phase E) | Done | `TaskExecutor` trait, `TokioTaskExecutor`, `TaskRuntime`, `CancellationHandle` |
| Test Optimization (Phase F) | Done | 155 → 43 test binaries, ~7:41 workspace test time |
| Governance Review | Done | 63 files dual-reviewed (spec-auditor + arch-reviewer), all PASS |
| Tool Annotations Macros | Done | `read_only`, `destructive`, `idempotent`, `open_world`, `title`, `annotation_title` on derive/function/declarative macros |
| Client Response Forwarding | Done | Channel-based response forwarding for server-initiated requests (ADR-020) |
| crates.io publish | Done | All 12 crates published through v0.3.8 |

### Key Architecture

- `turul-mcp-protocol` re-exports `turul-mcp-protocol-2025-11-25` (see ADR 015)
- Task storage: 4 backends (InMemory, SQLite, PostgreSQL, DynamoDB) with shared parity tests
- Task runtime: storage (trait) / executor (`turul-mcp-server`) / runtime (`TaskRuntime`) three-layer split
- Test binaries consolidated via `autotests = false` + `[[test]]` entries (Phase F)

---

## Remaining Work

See `TODO_TRACKER.md` for the full tracked list. Key items:

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

**Key ADRs**: 001 (Session Storage), 009 (Handler Routing), 012 (Middleware), 013 (Lambda Auth), 014 (Schemars), 015 (Protocol Crate Strategy), 016 (Task Storage), 017 (Runtime-Executor Boundary), 018 (Pagination Cursor), 020 (Client Response Forwarding)
