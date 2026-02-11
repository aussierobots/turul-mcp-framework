# TODO Tracker

**Last Updated**: 2026-02-12
**Version**: v0.3.0 (branch: `0.3.0`)
**Tests**: ~1,500+ passing, 43 test binaries, zero warnings

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

---

## Known Issues

- `tasks/result` error path wraps original error code in `McpError::ToolExecutionError` — loses original JSON-RPC error code

---

## Reference

- `WORKING_MEMORY.md` — Current status and architecture
- `CHANGELOG.md` — User-facing changes
- `CLAUDE.md` — Project guidelines and conventions
- `docs/adr/` — Architectural decisions
