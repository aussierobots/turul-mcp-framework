# Repository Guidelines

## Project Structure & Module Organization
- `Cargo.toml` (root): Workspace manifest; shared deps and profiles.
- `crates/`: Core crates (server, client, protocol 2025-06-18, builders, derive, json-rpc, session-storage, http server).
- `examples/`: Runnable servers/clients showing patterns and real apps.
- `tests/`: Integration tests (Tokio async): compliance, session, framework integration.
- `docs/`: Architecture/spec notes (see README for overview).

## Architecture Overview (Key Crates)
- `turul-mcp-server`: High-level server builder and areas (tools/resources/prompts/etc.).
- `turul-mcp-client`: HTTP client library.
- `turul-mcp-protocol-2025-06-18`: MCP spec types and contracts.
- `turul-mcp-json-rpc-server`: JSON-RPC 2.0 foundation.
- `turul-http-mcp-server`: HTTP/SSE transport.
- `turul-mcp-derive` / `turul-mcp-builders`: Macros and builders for ergonomics.

## Build, Test, and Development Commands
- Build: `cargo build --workspace`
- Test (all): `cargo test --workspace`
- Compliance tests: `cargo test --test mcp_compliance_tests`
- Lint: `cargo clippy --workspace --all-targets -- -D warnings`
- Format: `cargo fmt --all -- --check`  •  Fix: `cargo fmt --all`
- Run example: `cd examples/minimal-server && cargo run` (adjust folder as needed)

## MCP Specification Compliance
- Target spec: https://modelcontextprotocol.io/specification/2025-06-18
- Requirements: correct JSON-RPC usage, `_meta` fields, version negotiation, pagination/cursors, progress, and session isolation/TTL.
- Validate: run `cargo test --test mcp_compliance_tests`; for end‑to‑end session compliance, see README “MCP Session Management Compliance Testing”.

### TypeScript Schema Alignment
- Shapes must match the latest TS schema in `turul-mcp-protocol-2025-06-18` (camelCase, optional `_meta` on params/results where spec allows).
- Prompts: `GetPromptParams.arguments` is `map<string,string>` at the boundary. Handlers may convert internally to `Value` for rendering.
- Tools: `ToolSchema` type is `object`; `properties`/`required` present when needed; `annotations` are optional hints.
- Resources: `Resource`, `ResourceTemplate`, and results (`List*Result`, `ReadResourceResult`) follow TS names, including `nextCursor` and `_meta`.
- Extension: `CallToolResult.structuredContent` is an optional extension. Keep optional, document it, and ensure clients/tests do not depend on it for correctness.

## Resources Compliance
- Capabilities: advertise `resources.subscribe` and `resources.listChanged` when supported (only set `listChanged` when wired).
- Listing: implement `resources/list` and `resources/templates/list` with stable, absolute URIs; paginate via cursor (`nextCursor`). Do not enumerate dynamic template instances in `resources/list`; publish templates only via `resources/templates/list`.
- Reading: `resources/read` returns `contents[]` with `uri`, `mimeType`, and Text/Blob/URI reference; avoid `unwrap()`.
- Dynamic templates: publish via `ResourceTemplate` (e.g., `file:///user-{user_id}.json`, `file:///user-profile-{user_id}.{image_format}`); resolve at read-time with strict validation.
- Security: enforce roots and access controls (allow/block patterns, MIME allowlist, size caps) for `file://` and user input.
- Updates: send `notifications/resources/updated` and `notifications/resources/listChanged` appropriately.
- `_meta`: round-trip optional `_meta` for list/template operations (params → result meta) to match MCP behavior.
- Invalid URIs: do not publish invalid URIs in `resources/list`; test invalid cases via `resources/read` error scenarios. URIs must be absolute; encode spaces if demonstrated.
- Example:
  - List: `curl -s http://127.0.0.1:52950/mcp -H 'Content-Type: application/json' -d '{"method":"resources/list"}'`
  - Read: `curl -s http://127.0.0.1:52950/mcp -H 'Content-Type: application/json' -d '{"method":"resources/read","params":{"uri":"config://app.json"}}'`

## Prompts Compliance
- Capabilities: advertise `prompts.listChanged` when prompts are exposed.
- Listing: implement `prompts/list` with stable prompt names; include descriptions.
- Retrieval: `prompts/get` returns `messages[]` with roles and text content; define `arguments[]` with `required` flags and descriptions.
- Meta: support optional `_meta` on requests/results; emit `notifications/prompts/listChanged` when the set changes.
- Example:
  - List: `curl -s http://127.0.0.1:52950/mcp -H 'Content-Type: application/json' -d '{"method":"prompts/list"}'`
  - Get: `curl -s http://127.0.0.1:52950/mcp -H 'Content-Type: application/json' -d '{"method":"prompts/get","params":{"name":"code_review","arguments":{"language":"rust"}}}'`

## Tools Compliance
- Listing: implement `tools/list` with stable ordering (sort by name) and support pagination (`nextCursor`) when applicable.
- `_meta`: round-trip optional `_meta` for list operations.
- Calling: `tools/call` returns `content[]` and may include `isError`; `_meta` optional. If `structuredContent` is included, treat as optional extension.

## Reviewer Checklist: Resources & Prompts
- Capabilities: `resources.subscribe`, `resources.listChanged`, `prompts.listChanged` match actual support.
- Endpoints: `resources/list`, `resources/read`, `resources/templates/list`, `prompts/list`, `prompts/get` implemented and registered (separate handlers).
- Types: request params and results follow protocol (cursor in params; `nextCursor` and optional `_meta` in results).
- Prompts: `GetPromptParams.arguments` is a map of string→string; handler converts safely from inputs.
- Messages: `PromptMessage` roles and content blocks conform; no ad‑hoc shapes.
- Resources: `ResourceContent` variants include `uri` and `mimeType` correctly; URIs are absolute and stable.
- Notifications: listChanged/updated methods use spec‑accurate names consistently; SSE bridge emits them.
- Pagination: respects `cursor` and returns `nextCursor` when more items exist.
- Tests: add/keep coverage for all of the above.

## Notifications Compliance
- `notifications/initialized`: in strict lifecycle mode, reject operations until client sends `notifications/initialized`; add E2E to verify gating and acceptance after.
- `notifications/progress`: progress updates must include `progressToken`. Add at least one strict E2E that asserts ≥1 progress event and token match with tool response.
- `listChanged` notifications for tools/prompts/resources must only be advertised/emitted when dynamic change sources exist; keep `listChanged=false` for static servers.

## Capabilities Truthfulness
- On every initialize E2E, assert capability truthfulness for the static framework: `resources.subscribe=false`, `tools.listChanged=false`, `prompts.listChanged=false` (and others only when actually wired).

## Server & Client Testing
- Start a session‑enabled server (choose backend):
  - SQLite (dev): `cargo run --example client-initialise-server -- --port 52950 --storage-backend sqlite --create-tables`
  - DynamoDB (prod): `cargo run --example client-initialise-server -- --port 52950 --storage-backend dynamodb --create-tables`
  - PostgreSQL (enterprise): `cargo run --example client-initialise-server -- --port 52950 --storage-backend postgres`
  - InMemory (fast, no persistence): `cargo run --example client-initialise-server -- --port 52950 --storage-backend inmemory`
- Run the compliance client against it:
  - `RUST_LOG=info cargo run --example session-management-compliance-test -- http://127.0.0.1:52950/mcp`
- Explore additional servers/clients for manual testing:
  - Servers: `examples/minimal-server`, `examples/comprehensive-server`, `examples/notification-server`
  - Clients: `examples/logging-test-client`, `examples/lambda-mcp-client`
  - Pattern: `cd examples/<name> && cargo run`

## Troubleshooting
- Port busy: change `--port` or stop the existing process.
- DynamoDB: ensure AWS credentials are configured; include `--create-tables` on first run.
- PostgreSQL/SQLite: defaults are auto-configured; if custom DSNs/paths are needed, set via environment variables supported by storage crates.
- Verbose diagnostics: set `RUST_LOG=debug` and re-run the command.

## Coding Style & Naming
- Rust 2024; `rustfmt` defaults; deny warnings in CI.
- Naming: `snake_case` (items), `CamelCase` (types/traits), `SCREAMING_SNAKE_CASE` (consts).
- Errors via `thiserror`; avoid `unwrap()` outside tests.
- Logging with `tracing`; prefer structured fields and UUID v7 correlation.

## Testing Guidelines
- Use `#[tokio::test]` for async. Key suites: `session_context_macro_tests`, `framework_integration_tests`, `mcp_compliance_tests`.
- Add unit tests under `#[cfg(test)]` per crate; keep deterministic and isolated.

### E2E Test Authoring & Portability
- Use `tests/shared` server manager; do not hardcode `current_dir` paths. Discover workspace root dynamically.
- Add E2E for `resources/templates/list` (pagination, stable ordering, `_meta` round‑trip).
- Add a strict SSE progress test validating at least one progress event and `progressToken` match.
- Add strict lifecycle E2E gating with `notifications/initialized`.
- Assert initialize capability snapshot in each E2E suite.

## Commit & Pull Request Guidelines
- Commits: imperative subject (≤72 chars), meaningful body; reference issues (`Fixes #123`).
- Pre‑PR: `cargo fmt`, `cargo clippy -D warnings`, `cargo test --workspace`; update README/examples/docs when APIs change.
- PRs: clear description, linked issues, testing notes (commands/output), risk/rollback.

## Security & Configuration Tips
- Never commit secrets. AWS examples require valid credentials; prefer env vars/roles.
- Keep debug logs off by default; gate experimental features behind flags.

## Agent‑Specific Instructions
- Scope: this file applies to the entire repository.
- Role: act as a strict critic for MCP 2025‑06‑18 compliance within the Turul MCP Framework; flag deviations and propose compliant fixes.
- Do not relax security, logging, or API contracts to “make tests pass”; fix root causes while preserving spec compliance.
- Boundaries: do not modify core framework areas unless explicitly requested. The ~9 areas are Tools, Resources, Prompts, Sampling, Completion, Logging, Roots, Elicitation, and Notifications.
 - Extensions: if introducing non-standard fields (e.g., `structuredContent`), document them clearly, keep optional, and ensure baseline compliance without them.
