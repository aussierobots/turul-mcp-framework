# TODO Tracker

**Last Updated**: 2026-02-26
**Version**: v0.3.0 (branch: `0.3.0`)
**Tests**: 1,560+ passing, 43 test binaries, zero warnings

---

## v0.3.0 Release

- [x] **CHANGELOG.md** — Document MCP 2025-11-25 protocol support, task storage (4 backends), task executor/runtime, test optimization
- [ ] **CHANGELOG.md** — Expand [0.3.0] section with full 2025-11-25 feature list (protocol phases, task stack, durable backends)
- [ ] **crates.io publish preparation** — Version bumps, dependency audit, dry-run publish

---

## Future Work (Post v0.3.0)

### P1: Spec Features

- [ ] **resources/subscribe** — Real-time resource update notifications (SSE-based subscription, change detection, unsubscribe)
- [ ] **roots/list enhancements** — Advanced filtering, permissions, dynamic root generation

### P2: Framework Enhancements

- [x] **Session-aware resources** — Add `SessionContext` parameter to `McpResource::read()` for personalized content *(done in v0.2.0)*
- [x] **README testing** — Validate code examples in published crate READMEs compile via skeptic *(done in v0.3.0)*

### P2: Examples

- [ ] **Durable task storage examples** — SQLite, PostgreSQL, DynamoDB backends for task storage (backends implemented, no examples yet)
- [ ] **Lambda + tasks example** — `LambdaMcpServerBuilder` with `.with_task_storage()` (API exists, no example)
- [ ] **Task progress notifications example** — SSE-based task status updates during long-running operations
- [x] **Run full example verification** — All 58 examples verified 2026-02-26 (50 functional, 2 skipped external deps, 2 blocked by client library bug, 4 compile-only)

### P3: Documentation

- [x] **README.md accuracy review** — Fixed builder methods, counts, test commands, transport section *(2026-02-26)*
- [x] **CLAUDE.md accuracy review** — Fixed schemars docs, test commands, added missing crate *(2026-02-26)*
- [x] **DOCUMENTATION_TESTING.md update** — Updated stale status section with current doctest counts *(2026-02-26)*
- [x] **EXAMPLES.md accuracy review** — Fixed count (57→58), added lambda-authorizer, de-duplicated entries *(2026-02-26)*
- [x] **EXAMPLE_VERIFICATION_LOG.md update** — Full 2026-02-26 verification run: 50 functional, 2 skipped, 2 client bug, 4 compile-only *(2026-02-26)*
- [x] **WORKING_MEMORY.md update** — Updated test counts, example counts, consolidated remaining work to point at TODO_TRACKER.md *(2026-02-26)*
- [x] **HISTORY.md review** — Verified accurate, no changes needed *(2026-02-26)*
- [x] **CHANGELOG.md review** — Content correct; expansion tracked as v0.3.0 release item above *(2026-02-26)*

---

### P1: Bug Fixes

- [ ] **`HttpTransport::connect()` sends OPTIONS** — Server returns 405. Affects `streamable-http-client`, `tasks-e2e-inmemory-client`, `client-task-lifecycle`. Fix: change OPTIONS to POST or skip preflight check. (`crates/turul-mcp-client/src/transport/http.rs:390`)
- [ ] **Port inconsistencies in EXAMPLES.md** — Several examples have hardcoded ports that differ from documented values (partially fixed 2026-02-26, remaining: minimal-server ignores `--port` flag)
- [ ] **Phase 6 verification script** — Uses `cargo run` instead of pre-built binaries, causing timeouts. Update to match phases 1-5 pattern.

## Known Issues

- `tasks/result` error path wraps original error code in `McpError::ToolExecutionError` — loses original JSON-RPC error code
- `HttpTransport::connect()` uses OPTIONS method — not supported by server (returns 405)

---

## Reference

- `WORKING_MEMORY.md` — Current status and architecture
- `CHANGELOG.md` — User-facing changes
- `CLAUDE.md` — Project guidelines and conventions
- `docs/adr/` — Architectural decisions
