# Repository Guidelines

## Project Structure & Module Organization
- `Cargo.toml` (root): Workspace manifest; shared deps and profiles.
- `crates/`: Core crates (server, client, protocol alias + 2025-11-25 spec, builders, derive, json-rpc, session-storage, http server, AWS Lambda transport).
- `examples/`: Runnable servers/clients showing patterns and real apps.
- `tests/`: Integration tests (Tokio async): compliance, session, framework integration.
- `docs/`: Architecture/spec notes (see README for overview).

## Architecture Overview (Key Crates)
- `turul-mcp-server`: High-level server builder and areas (tools/resources/prompts/etc.).
- `turul-mcp-client`: HTTP client library.
- `turul-mcp-protocol`: Current-spec alias that re-exports the active protocol crate for downstreams.
- `turul-mcp-protocol-2025-11-25`: MCP spec types and contracts for the 2025-11-25 schema.
- `turul-mcp-session-storage`: Pluggable session backends (in-memory, SQLite, Postgres, DynamoDB).
- `turul-mcp-json-rpc-server`: JSON-RPC 2.0 foundation.
- `turul-http-mcp-server`: HTTP/SSE transport.
- `turul-mcp-aws-lambda`: AWS Lambda entrypoint integration for serverless deployments.
- `turul-mcp-derive` / `turul-mcp-builders`: Macros and builders for ergonomics.
- `examples/middleware-*/`: Reference middleware servers (HTTP + Lambda auth/logging/rate limiting).

## Building MCP Services (Servers)
- Prefer `turul_mcp_server::McpServer::builder()` for integrated HTTP transport; choose function macros, derive macros, builders, or manual traits depending on ergonomics.
- Custom transports (Hyper/AWS Lambda/etc.) should construct an `McpServer` configuration and pass it to `turul-http-mcp-server` or `turul-mcp-aws-lambda`.
- Handlers must return domain errors: derive `thiserror::Error` for new error types and implement `turul_mcp_json_rpc_server::r#async::ToJsonRpcError`; avoid creating `JsonRpcError` directly.
- Register additional JSON-RPC methods via `JsonRpcDispatcher<McpError>` (or your custom error type) to guarantee type-safe conversion to protocol errors.
- Always advertise only the capabilities actually wired (e.g., leave `resources.listChanged=false` when notifications are not emitted) and back responses with cursor-aware pagination helpers from `turul_mcp_protocol`.
- Middleware:
  - Attach request/response middleware via `.middleware(Arc<dyn McpMiddleware>)` on both `McpServer::builder()` and `LambdaMcpServerBuilder`.
  - Middleware executes FIFO before dispatch and reverse order after dispatch.
  - Use `StorageBackedSessionView` + `SessionInjection` to read/write session state safely.
  - See `examples/middleware-auth-server`, `middleware-logging-server`, and `middleware-auth-lambda` for working patterns (API-key auth, logging, rate limiting).

## Building MCP Clients
- Use `turul_mcp_client::McpClientBuilder` with an appropriate transport (`HttpTransport`, `SseTransport`, etc.); the builder owns connection retries and timeouts.
- Invoke `client.connect().await?` to perform the JSON-RPC handshake; the client automatically sends `initialize` and the required `notifications/initialized` follow-up.
- Interact through the high-level APIs (`list_tools`, `call_tool`, `list_resources`, `read_resource`, `list_prompts`, `get_prompt`, etc.) which all return `McpClientResult<T>` with rich `McpClientError` variants.
- For streaming notifications, subscribe through the transport-specific stream helpers and always handle progress tokens echoed by tools.
- When embedding in other applications, propagate errors using the typed enums rather than string matching and surface meaningful diagnostics (e.g., include `McpClientError::Lifecycle` messaging when initialization fails).

## Build, Test, and Development Commands
- Build: `cargo build --workspace`
- Test (all): `cargo test --workspace`
- Compliance tests: `cargo test --test mcp_compliance_tests`
- Lint: `cargo clippy --workspace --all-targets -- -D warnings`
- Format: `cargo fmt --all -- --check`  •  Fix: `cargo fmt --all`
- Run example: `cd examples/minimal-server && cargo run` (adjust folder as needed)
- Middleware smoke tests: `bash scripts/test_middleware_live.sh` (HTTP) and `cargo lambda watch --package middleware-auth-lambda` (Lambda) for interactive validation.
- Schema/notification regressions:
  - `cargo test --test notification_payload_correctness`
  - `cargo test --test mcp_vec_result_schema_test`
  - `cargo test -p turul-mcp-derive schemars_integration_test`

## MCP Specification Compliance
- Target spec: https://modelcontextprotocol.io/specification/2025-11-25
- Requirements: correct JSON-RPC usage, `_meta` fields, version negotiation, pagination/cursors, progress, and session isolation/TTL.
- Validate: run `cargo test --test mcp_compliance_tests`; for end‑to‑end session compliance, see README “MCP Session Management Compliance Testing”.

### TypeScript Schema Alignment
- Shapes must match the latest TS schema in `turul-mcp-protocol-2025-11-25` (camelCase, optional `_meta` on params/results where spec allows).
- Prompts: `GetPromptParams.arguments` is `map<string,string>` at the boundary. Handlers may convert internally to `Value` for rendering.
- Tools: `ToolSchema` type is `object`; `properties`/`required` present when needed; `annotations` are optional hints.
- Resources: `Resource`, `ResourceTemplate`, and results (`List*Result`, `ReadResourceResult`) follow TS names, including `nextCursor` and `_meta`.
- `CallToolResult.structuredContent` is an optional field in the MCP 2025-11-25 schema. Keep it optional and ensure clients/tests handle its absence correctly.
- Tool output schemas:
  - External output structs **must** derive `schemars::JsonSchema` so the derive macros can emit detailed schemas via `schema_for!(T)`. Missing derives now produce compile-time errors (see CHANGELOG.md v0.2.1 breaking changes).
  - Zero-config (`output` omitted) heuristics still target `Self`; use `#[tool(output = Type)]` for accurate schemas on complex responses.
  - Array outputs (`Vec<T>`) are validated by `mcp_vec_result_schema_test` to ensure `tools/list` advertises `"type": "array"` and the runtime result matches.

## Resources Compliance
- Capabilities: advertise `resources.subscribe` and `resources.listChanged` when supported (only set `listChanged` when wired).
- Listing: implement `resources/list` and `resources/templates/list` with stable, absolute URIs; paginate via cursor (`nextCursor`). Do not enumerate dynamic template instances in `resources/list`; publish templates only via `resources/templates/list`.
- Reading: `resources/read` returns `contents[]` with `uri`, `mimeType`, and Text/Blob/URI reference; avoid `unwrap()`.
- Dynamic templates: publish via `ResourceTemplate` (e.g., `file:///user-{user_id}.json`, `file:///user-profile-{user_id}.{image_format}`); resolve at read-time with strict validation.
- Security: enforce roots and access controls (allow/block patterns, MIME allowlist, size caps) for `file://` and user input.
- Updates: send `notifications/resources/updated` and `notifications/resources/list_changed` appropriately.
- `_meta`: round-trip optional `_meta` for list/template operations (params → result meta) to match MCP behavior.
- Invalid URIs: do not publish invalid URIs in `resources/list`; test invalid cases via `resources/read` error scenarios. URIs must be absolute; encode spaces if demonstrated.
- Example:
  - List: `curl -s http://127.0.0.1:52950/mcp -H 'Content-Type: application/json' -d '{"method":"resources/list"}'`
  - Read: `curl -s http://127.0.0.1:52950/mcp -H 'Content-Type: application/json' -d '{"method":"resources/read","params":{"uri":"config://app.json"}}'`

## Prompts Compliance
- Capabilities: advertise `prompts.listChanged` when prompts are exposed.
- Listing: implement `prompts/list` with stable prompt names; include descriptions.
- Retrieval: `prompts/get` returns `messages[]` with roles and text content; define `arguments[]` with `required` flags and descriptions.
- Meta: support optional `_meta` on requests/results; emit `notifications/prompts/list_changed` when the set changes.
- Example:
  - List: `curl -s http://127.0.0.1:52950/mcp -H 'Content-Type: application/json' -d '{"method":"prompts/list"}'`
  - Get: `curl -s http://127.0.0.1:52950/mcp -H 'Content-Type: application/json' -d '{"method":"prompts/get","params":{"name":"code_review","arguments":{"language":"rust"}}}'`

## Tools Compliance
- Listing: implement `tools/list` with stable ordering (sort by name) and support pagination (`nextCursor`) when applicable.
- `_meta`: round-trip optional `_meta` for list operations.
- Calling: `tools/call` returns `content[]` and may include `isError`; `_meta` optional. `structuredContent` is an optional schema field and must remain optional in handling.

## Reviewer Checklist: Resources & Prompts
- Capabilities: `resources.subscribe`, `resources.listChanged`, `prompts.listChanged` match actual support.
- Endpoints: `resources/list`, `resources/read`, `resources/templates/list`, `prompts/list`, `prompts/get` implemented and registered (separate handlers).
- Types: request params and results follow protocol (cursor in params; `nextCursor` and optional `_meta` in results).
- Prompts: `GetPromptParams.arguments` is a map of string→string; handler converts safely from inputs.
- Messages: `PromptMessage` roles and content blocks conform; no ad‑hoc shapes.
- Resources: `ResourceContent` variants include `uri` and `mimeType` correctly; URIs are absolute and stable.
- Notifications: method names use spec strings (e.g., `notifications/resources/list_changed`, `notifications/prompts/list_changed`, `notifications/tools/list_changed`), while capability keys remain camelCase (e.g., `listChanged`).
- Pagination: respects `cursor` and returns `nextCursor` when more items exist.
- Tests: add/keep coverage for all of the above.

## Notifications Compliance
- `notifications/initialized`: in strict lifecycle mode, reject operations until client sends `notifications/initialized`; add E2E to verify gating and acceptance after.
- `notifications/progress`: progress updates must include `progressToken`. Add at least one strict E2E that asserts ≥1 progress event and token match with tool response.
- `list_changed` notifications (e.g., `notifications/tools/list_changed`) for tools/prompts/resources must only be advertised/emitted when dynamic change sources exist; keep capability key `listChanged=false` for static servers.

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

## Agent-Specific Instructions
- Scope: this file applies to the entire repository.
- Role: act as a strict critic for MCP 2025-11-25 compliance within the Turul MCP Framework; flag deviations and propose compliant fixes.
- Do not relax security, logging, or API contracts to “make tests pass”; fix root causes while preserving spec compliance.
- Boundaries: do not modify core framework areas unless explicitly requested. The ~9 areas are Tools, Resources, Prompts, Sampling, Completion, Logging, Roots, Elicitation, and Notifications.
 - Extensions: if introducing truly non-standard fields, document them clearly, keep optional, and ensure baseline compliance without them.

### Critic Review Mode (Architecture + Best Practices + MCP Compliance)
- Default stance for review-only requests: **no code changes** unless the user explicitly asks for a patch.
- Review output should prioritize findings over summaries:
  - Lead with concrete issues (severity-ordered) and file references.
  - Separate architecture risks, spec compliance risks, and documentation/process drift.
  - Call out missing tests/coverage when behavior claims change.
- Treat docs/examples/agent-instruction changes as potentially compliance-impacting:
  - Flag docs that advertise unsupported capabilities or incorrect defaults.
  - Flag examples that imply `listChanged`/subscription/progress/lifecycle support without matching implementation/tests.
  - Flag spec-version drift (must remain aligned to MCP `2025-11-25` unless intentionally upgraded everywhere).
- When reviewing client/server API guidance, verify it preserves typed error propagation and truthful capability advertisement.

### Workspace State Triage (Required Before Review Conclusions)
- Start with `git status --short --branch` to identify whether changes are code, docs, tests, or agent/process files.
- If changes are primarily docs/agent guidance (e.g., `README.md`, `CLAUDE.md`, `GEMINI.md`, `.claude/agents/*`):
  - Perform a consistency audit across all agent guidance files and this `AGENTS.md`.
  - Check that MCP terminology, method names, capability keys, and spec date are consistent.
  - Check that testing commands and compliance expectations match the current framework guidance in this file.
- If no code changed but behavior claims changed, treat that as a review finding unless the claims are demonstrably accurate.

### Reviewer Focus Areas (Do Not Skip)
- Architecture boundaries: examples and docs should not encourage bypassing crate layering (`protocol` vs `server` vs transport crates).
- Capability truthfulness: docs/tests must not imply dynamic capabilities when the framework is static by default.
- Lifecycle strictness: guidance must preserve `notifications/initialized` gating and correct error mapping semantics.
- Pagination/meta/schema accuracy: docs/examples must use `cursor`, `nextCursor`, and optional `_meta` consistently with the protocol crate.
- Notifications naming: spec method names use snake_case path segments; capability keys remain camelCase.
- Tool error semantics: do not normalize transport/framework errors into fake successful tool payloads.

### Current Workspace Risk Pattern (Doc + Agent Expansion)
- When multiple agent instruction files are added/modified alongside `README.md`, treat it as a **coordination risk**:
  - Watch for conflicting role definitions (critic vs implementer vs docs writer).
  - Watch for duplicated but diverging command guidance.
  - Prefer this `AGENTS.md` as the compliance authority when conflicts exist, and flag drift explicitly.

## Release Readiness Notes (2025-10-01)
- **Pagination Compliance**: `prompts/list`, `resources/list`, and `resources/templates/list` now honor caller-supplied `limit` values, clamp to the DoS ceiling, and reject `limit=0`. Preserve this behaviour in future patches and cover regression paths in the relevant handler tests.
- **Lifecycle Errors**: Strict lifecycle flows must continue returning `McpError::SessionError` for pre-initialization access. Any refactor that touches `SessionAware*` handlers needs to preserve the error mapping to `-32031`.
- **Tool Error Propagation**: Keep propagating `McpTool::call` failures as direct `McpError` results. Never re-wrap them as successful `CallToolResult::error` payloads.
- **Test Coverage**: Maintain the behavioural suites that assert pagination limits, lifecycle enforcement, and error propagation; add cases whenever new branches are introduced.
- **Server Teardown Discipline**: Use `TestServerManager` (with its `drop`-based shutdown) for integration/E2E suites. Avoid manual `kill` sequences that can leave ports occupied and cascade failures into later tests.
- **Tool Output Schemas**: External output types must derive `schemars::JsonSchema`; run `schemars_integration_test` and `mcp_vec_result_schema_test` before tagging a release to ensure detailed schemas (including arrays) are emitted.
- **Notification Payloads**: `notification_payload_correctness.rs` must stay green—any custom notification should round-trip `_meta` and payload fields exactly.
