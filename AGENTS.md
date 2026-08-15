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
- `turul-mcp-protocol-2026-07-28`: MCP spec types for the 2026-07-28 schema (independently versioned at 0.4.x; current active spec line).
- `turul-mcp-protocol-2025-11-25`: **FROZEN** at 0.3.x — historical snapshot, do not modify.
- `turul-mcp-protocol-2025-06-18`: **FROZEN** at 0.3.x — historical snapshot, do not modify.

## Frozen Protocol Crates

`turul-mcp-protocol-2025-06-18` and `turul-mcp-protocol-2025-11-25` are **frozen at `0.3.47`**. Do not edit them — no code changes, no version bumps, no doc updates, no dependency changes. They are immutable historical spec snapshots. All new MCP spec work belongs in `turul-mcp-protocol-2026-07-28`. The only permitted touch is workspace `Cargo.toml` metadata if a workspace-wide rename forces it.

## Crate Versioning Policy

**Per-crate independent versioning.** Each crate's `Cargo.toml` carries its own literal `version = "X.Y.Z"`. No crate uses `version.workspace = true`.

- The **0.4.0 release** is the first under this policy. Every non-frozen crate ships at `0.4.0` together as a coordinated initial cut.
- After 0.4.0, crates may be patched and published independently — bump only the crate(s) that changed, not the whole workspace.
- Frozen crates (`2025-06-18`, `2025-11-25`) stay at `0.3.47` regardless of what the rest of the workspace does.
- `[workspace.package].version` exists for tooling compatibility but is not authoritative — per-crate literal versions are the source of truth.
- When bumping a crate, update both the crate's `Cargo.toml` AND the matching `[workspace.dependencies]` pin in the root `Cargo.toml`.

External dependencies (`serde`, `tokio`, etc.) continue to use `workspace = true` references — only internal crate versions are independent.

## Branch-Conditional Spec Guidance

This file documents the framework under two simultaneous spec targets:

- **`main` (and any branch derived from `main`)** — MCP 2025-11-25 (stateful core: `initialize`/`notifications/initialized` handshake, `Mcp-Session-Id` header, capability negotiation at handshake time).
- **`feat/turul-mcp-protocol-2026-07-28`** (the 0.4 release in preparation) — MCP 2026-07-28, the released current specification (stateless core: handshake and session header removed, capabilities travel in `_meta` on every request, new `server/discover` method). See §"Branch Lock" below for the full diff.

Rules in §"MCP Specification Compliance", §"Notifications Compliance", §"Testing Guidelines", §"Agent-Specific Instructions", §"Critic Review Mode", and §"Reviewer Focus Areas" are written against the `main` 2025-11-25 baseline and are explicitly tagged `(2025-11-25 baseline)` where the draft branch supersedes them. On the draft branch, defer to:

- `docs/adr/027-targeting-mcp-draft-2026-v1.md` — spec target + cutover plan
- `crates/turul-mcp-protocol-2026-07-28/COMPLIANCE.md` — current compliance state
- §"Branch Lock" below — headline diffs vs 2025-11-25

If a `(2025-11-25 baseline)` rule conflicts with a draft-branch spec requirement, the draft-branch artifact wins on the draft branch; do not back-port the draft-branch shape to `main` without explicit instruction.
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
- Invoke `client.connect().await?` to perform the JSON-RPC handshake; the client automatically sends `initialize` and the required `notifications/initialized` follow-up. (2025-11-25 baseline; the 2026-07-28 branch is stateless and removes both `initialize` and `notifications/initialized`.)
- Interact through the high-level APIs (`list_tools`, `call_tool`, `list_resources`, `read_resource`, `list_prompts`, `get_prompt`, etc.) which all return `McpClientResult<T>` with rich `McpClientError` variants.
- For streaming notifications, subscribe through the transport-specific stream helpers and always handle progress tokens echoed by tools.
- When embedding in other applications, propagate errors using the typed enums rather than string matching and surface meaningful diagnostics (e.g., include `McpClientError::Lifecycle` messaging when initialization fails).

## Build, Test, and Development Commands

**`--workspace` does not work on this branch.** Cargo unifies features across a
single invocation, so building every member at once forces the mutually exclusive
`protocol-2025-11-25` and `protocol-2026-07-28` features onto the shared
`turul-mcp-protocol` dependency and trips its own guard
(`features ... are mutually exclusive` + `E0659: MCP_VERSION is ambiguous`).
Build the curated default-members set, then any opt-in member individually.

That mutex is not only a build constraint — it fixes the framework's **server era
posture**, so do not describe turul servers as supporting both specs at once. The
2026-07-28 spec defines **dual-era** (one implementation serving modern and legacy
clients, optionally on the same endpoint) and makes it a **MAY**. turul does not
implement it: a server binary speaks 2026-07-28 *or* 2025-11-25, never both. Serving
both means two instances. The **client** is the exception — it links both protocol
crates and negotiates per connection (ADR-030). Consequence worth stating whenever
lanes come up: per the spec's own compatibility matrix a legacy client → modern server
**fails**, so a 2026-default turul server is unreachable to 2025-era clients. Full
write-up with the wire evidence: `docs/compliance/base-protocol.md` §4.

- Build: `cargo build` (default-members = the 2026-07-28 lane)
- Test: `cargo test`
- Opt-in 2025-11-25 member: `cargo build -p <crate> --no-default-features --features protocol-2025-11-25`
- Single non-default example: `cargo check -p <example> --all-targets`
- Compliance tests: `cargo test -p turul-mcp-protocol-2026-07-28 --features compliance`
- Schema pin integrity: `./scripts/check-schema-pin.sh`
- Lint: `cargo clippy --all-targets -- -D warnings`
- All release gates: `./scripts/ci-gates.sh`. `.github/workflows/ci.yml` matches it command-for-command on the fmt, default-2026, opt-in-2025, mutex and docs gates, but `lambda` (needs the cargo-lambda + Zig cross-compilation toolchain) and `examples` (boots real servers for minutes; one leg wants DynamoDB) have no hosted equivalent. So the script remains a superset: a green CI run is necessary but not sufficient — run it before tagging.
- Format: `cargo fmt --all -- --check`  •  Fix: `cargo fmt --all`  •  Gated as `./scripts/ci-gates.sh fmt` (also first in `all`)
- Run example: `cd examples/minimal-server && cargo run` (adjust folder as needed)
- Middleware smoke tests: `bash scripts/test_middleware_live.sh` (HTTP) and `cargo lambda watch --package middleware-auth-lambda` (Lambda) for interactive validation.
- Schema/notification regressions. The integration crate sets `autotests = false`,
  so these files are **not** their own `--test` targets — each is pulled in with
  `#[path]` by an aggregate target, and naming the file directly matches nothing:
  - `cargo test -p turul-mcp-framework-integration-tests --test feature_tests`
    (contains `notification_payload_correctness`)
  - `cargo test -p turul-mcp-framework-integration-tests --test schema_tests`
    (contains `mcp_vec_result_schema_test`)
  - `cargo test -p turul-mcp-framework-integration-tests derive_schemars_integration_test`
    (a filter, not a target; it lives in this crate, not in `turul-mcp-derive`)

## MCP Specification Compliance (2025-11-25 baseline)

_Rules below apply to `main` / the 2025-11-25 spec target. On the 2026-07-28 branch, see `crates/turul-mcp-protocol-2026-07-28/COMPLIANCE.md` and §"Branch Lock" — stateless core, `_meta`-routed capability negotiation, error code `-32002 → -32602`._

- Target spec: https://modelcontextprotocol.io/specification/2025-11-25
- Requirements: correct JSON-RPC usage, `_meta` fields, version negotiation, pagination/cursors, progress, and session isolation/TTL.
- Validate: run `cargo test -p turul-mcp-framework-integration-tests --test compliance`
  (the `mcp_compliance_tests` file is `#[path]`-included by that target, not a target
  itself); for end‑to‑end session compliance, see README “MCP Session Management Compliance Testing”.

### TypeScript Schema Alignment (2025-11-25 baseline)
- Shapes must match the latest TS schema in `turul-mcp-protocol-2025-11-25` (camelCase, optional `_meta` on params/results where spec allows). _On the 2026-07-28 branch, the equivalent reference is `crates/turul-mcp-protocol-2026-07-28/schema/schema.ts`._
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

## Notifications Compliance (2025-11-25 baseline)

_On the 2026-07-28 branch the `notifications/initialized` rule does not apply — the handshake is gone. Per-request capability negotiation rides in `_meta`._

- `notifications/initialized`: in strict lifecycle mode, reject operations until client sends `notifications/initialized`; add E2E to verify gating and acceptance after. (2025-11-25 baseline only.)
- `notifications/progress`: progress updates must include `progressToken`. Add at least one strict E2E that asserts ≥1 progress event and token match with tool response. Satisfied on both lanes: `progress_token_match_2025_11_25.rs` (2025-11-25) and `progress_2026.rs` / `streaming_e2e_2026.rs` (2026-07-28). The `SessionContext` progress API is deliberately lane-neutral — `notify_progress(arbitrary_string, ..)` does not satisfy this rule, because a token the tool invented is not correlation.
- `list_changed` notifications (e.g., `notifications/tools/list_changed`) for tools/prompts/resources must only be advertised/emitted when dynamic change sources exist; keep capability key `listChanged=false` for static servers.

## Capabilities Truthfulness
- On every initialize E2E, assert capability truthfulness for the static framework: `resources.subscribe=false`, `tools.listChanged=false`, `prompts.listChanged=false` (and others only when actually wired).

## Server & Client Testing
- Start a session‑enabled server (choose backend):
  - SQLite (dev): `cargo run -p client-initialise-server -- --port 52950 --storage-backend sqlite --create-tables`
  - DynamoDB (prod): `cargo run -p client-initialise-server -- --port 52950 --storage-backend dynamodb --create-tables`
  - PostgreSQL (enterprise): `cargo run -p client-initialise-server -- --port 52950 --storage-backend postgres`
  - InMemory (fast, no persistence): `cargo run -p client-initialise-server -- --port 52950 --storage-backend inmemory`
- Run the compliance client against it:
  - `RUST_LOG=info cargo run -p session-management-compliance-test -- http://127.0.0.1:52950/mcp`
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
- **Spec-version naming: ALWAYS the full date (`YYYY-MM-DD` / `YYYY_MM_DD`), NEVER a bare year.** A bare year is ambiguous — 2025 shipped two specs (`2025-06-18` and `2025-11-25`). Forbidden: `v2026`, `client-2026-only`, `protocol-2025`. Required: `v2026_07_28`, `McpVersion::V2026_07_28`, `feature = "client-2026-07-28-only"`, `feature = "protocol-2025-11-25"`. Applies to modules, identifiers, cargo features, types, and prose. The only dateless spec tokens are deliberately spec-neutral names (e.g. the single `turul-mcp-ext-tasks` crate, ADR-028). Full rule: CLAUDE.md §"Spec-Version Naming".
- **Comments describe the code as-is and what is non-obvious to a human — nothing else.** Keep them clean and minimal; default to no comment when the code already reads clearly. Forbidden in source (`.rs` and `Cargo.toml`/manifests): internal phase/slice/batch tags **and internal requirement/gap-register/audit identifiers** (`Phase 3.4`, `Slice 1`, `BP-3`, `GAP-CF-9`, `VER-1`, `TX/GAP-7`, or any tracking ID from the compliance matrix / gap register — state the spec MUST itself, or cite the `SEP-####` anchor, never the internal ID), decision-record (ADR) citations (`per ADR-025`, `see ADR-029`), CLAUDE.md/AGENTS.md self-references, tombstones/dev-log narratives (`was removed in vX`, `formerly known as`), unverified comparative claims, and speculation about author intent. Decision history belongs in the ADR/CHANGELOG/commit, not in source. **External MCP spec anchors (`SEP-####`, schema `@see` links) remain allowed** — they name the wire contract the code implements. Full rule: CLAUDE.md §Comments.

## Testing Guidelines
- Use `#[tokio::test]` for async. Key suites: `session_context_macro_tests`, `framework_integration_tests`, `mcp_compliance_tests`.
- Add unit tests under `#[cfg(test)]` per crate; keep deterministic and isolated.

### E2E Test Authoring & Portability
- Use `tests/shared` server manager; do not hardcode `current_dir` paths. Discover workspace root dynamically.
- Add E2E for `resources/templates/list` (pagination, stable ordering, `_meta` round‑trip).
- Add a strict SSE progress test validating at least one progress event and `progressToken` match.
- Add strict lifecycle E2E gating with `notifications/initialized`. (2025-11-25 baseline only — 2026-07-28 has no lifecycle to gate.)
- Assert initialize capability snapshot in each E2E suite.

## Commit & Pull Request Guidelines
- Commits: imperative subject (≤72 chars), meaningful body; reference issues (`Fixes #123`).
- Pre‑PR: `./scripts/ci-gates.sh` covers fmt, clippy and test across both lanes; run it rather than the individual commands. Update README/examples/docs when APIs change.
- PRs: clear description, linked issues, testing notes (commands/output), risk/rollback.

## Security & Configuration Tips
- Never commit secrets. AWS examples require valid credentials; prefer env vars/roles.
- Keep debug logs off by default; gate experimental features behind flags.

## Branch Lock: `feat/turul-mcp-protocol-2026-07-28`

**This branch is the 0.4 release in preparation**, adopting MCP 2026-07-28 — now the released current specification (see https://modelcontextprotocol.io/specification/2026-07-28 and its [changelog](https://modelcontextprotocol.io/specification/2026-07-28/changelog)).

**0.4 becomes the current release only when the maintainer opens the PR and merges it.** Until then the branch is pre-release: it carries the 0.4 line (every non-frozen crate is already `0.4.0`), `main` carries 0.3 / 2025-11-25, and neither fact authorises a merge. Describe this branch's contents as 0.4; the frozen crates, since-markers, changelog history and external crate pins that legitimately stay 0.3 are enumerated in CLAUDE.md §Version References.

Headline changes the framework must absorb:

- **Stateless protocol core**: `initialize`/`notifications/initialized` handshake removed; `Mcp-Session-Id` header removed; protocol version, client info, and capabilities travel in `_meta` on every request; new `server/discover` method.
- **Server-to-client requests**: SSE no longer held open for elicitation — server returns `InputRequiredResult` with `inputRequests` + `requestState`; client re-issues original call with `inputResponses`.
- **New required headers**: `MCP-Protocol-Version: 2026-07-28`, `Mcp-Method`, `Mcp-Name`; servers reject header/body disagreement.
- **Caching headers**: `ttlMs`, `cacheScope` on list/read results.
- **Distributed tracing**: W3C `traceparent`/`tracestate`/`baggage` in `_meta`.
- **Extensions first-class** (SEP-2133): reverse-DNS IDs, `extensions` capability map, separate `ext-*` repos.
- **Tasks → official extension** (SEP-2663): new stateless lifecycle (`tasks/get`/`update`/`cancel`, no `tasks/list`); 2025-11-25 experimental Tasks API is **incompatible** and requires migration.
- **MCP Apps extension** (SEP-1865): sandboxed-iframe HTML UI templates.
- **JSON Schema 2020-12** (SEP-2106): `oneOf`/`anyOf`/`allOf`/`$ref`/`$defs` on `inputSchema`; `outputSchema` unrestricted; `structuredContent` may be any JSON value.
- **Auth hardening**: RFC 9207 `iss` validation (SEP-2468), OIDC `application_type` on DCR (SEP-837), issuer binding (SEP-2352), refresh token requests (SEP-2207), scope accumulation (SEP-2350), `.well-known` discovery suffix (SEP-2351).
- **Deprecations** (12-month window, annotation-only this version): Roots, Sampling, Logging.
- **Breaking error code**: missing resource `-32002` → JSON-RPC standard `-32602`.

**Branch governance — MANDATORY:**
- **DO NOT merge `feat/turul-mcp-protocol-2026-07-28` into `main` without the maintainer's express authority.** This applies to merge commits, fast-forward merges, rebase-onto-main, squash-merges, and merge PRs alike.
- **DO NOT mark this branch "done," delete it, force-push it, or open a release/merge PR** without explicit maintainer authorization in the current session.
- "All SEPs implemented," "all tests pass," and "conformance suite green" are necessary but **not sufficient** — disposition is the maintainer's call.
- `main` remains on MCP 2025-11-25 — now the *previous* spec, not the current one — throughout this work. Do not back-port 2026-07-28-only changes to `main` without explicit instruction.
- Side-branches off `feat/turul-mcp-protocol-2026-07-28` may merge back into it freely; only the branch → `main` direction is locked.

**Schema pin governance — MANDATORY:**
- **2026-07-28 has finalized.** The released schema lives at the immutable upstream path `schema/2026-07-28/schema.ts` (tag `2026-07-28`); upstream `schema/draft/` is now the *next* spec cycle's floating pointer and is no longer what this crate tracks. Any pin, fetch, or drift check still resolving against `schema/draft/` or against `main` will silently walk onto next-cycle content while claiming to implement 2026-07-28. **Verify the pin still names the released artifact at the START of every 2026-07-28 slice** — before writing code, and before trusting a green suite.
- **Two artifacts are pinned and MUST name the same immutable upstream commit:** `PIN` in `crates/turul-mcp-protocol-2026-07-28/src/compliance/fetch.rs` (the example fixtures) and the provenance block in `schema/README.md` (the vendored `schema.ts`). Re-vendor **by commit SHA, never `main`** — a `main`-sourced copy cannot be reproduced later. Leaving the two on different commits is a provenance defect, not a cosmetic mismatch.
- **A green compliance run is only as strong as its `modeled=N` count.** Most upstream example directories are `Kind::NotModeled`, so the harness reports `failed=0` for changes it never looked at. Read the modeled count on every run. If a slice changes a type whose fixture directory is unmodeled, model it **in the same slice** — otherwise the fix ships with no fixture-level proof.
- **Pin parity is a claim about a moment, not a standing property.** Any document asserting "no re-pin trigger exists" is dated the instant it is written; re-verify rather than citing it.
- **Reconcile `docs/compliance/` in the same slice as the re-pin.** Those four area records name, per requirement, the test that asserts it and which independent implementation has exercised it. A re-pin can add a requirement, retire one, or move a test — and a row whose "Verified by" cell names a test that no longer exists is a defect, not a stale doc. The same rule applies to any behaviour change to a governed requirement and to any interop probe run that changes an interop cell.
- Operator commands for the drift check: see CLAUDE.md §"Check the schema pins BEFORE any 2026-07-28 work".

## Agent-Specific Instructions
- Scope: this file applies to the entire repository.
- Role: act as a strict critic for the **active branch's spec target** — MCP 2025-11-25 on `main`, MCP 2026-07-28 on the `feat/turul-mcp-protocol-2026-07-28` branch (see §"Branch-Conditional Spec Guidance") — within the Turul MCP Framework; flag deviations and propose compliant fixes.
- Do not relax security, logging, or API contracts to “make tests pass”; fix root causes while preserving spec compliance.
- Boundaries: do not modify core framework areas unless explicitly requested. The ~9 areas are Tools, Resources, Prompts, Sampling, Completion, Logging, Roots, Elicitation, and Notifications.
 - Extensions: if introducing truly non-standard fields, document them clearly, keep optional, and ensure baseline compliance without them.

### Complexity Control
- Default to the smallest design that satisfies the current requirement.
- Do not introduce new modes, traits, crates, storage abstractions, polling loops, caches, or transport-specific branches unless the current task demonstrably requires them.
- Prefer one authoritative path for a behavior over parallel or fallback paths that can drift or duplicate work.
- Separate clearly:
  - current implemented behavior,
  - intended architecture,
  - and deferred future work.
  Do not describe future architecture as if it is already implemented.
- When a requirement can be satisfied by narrowing an existing path, prefer that over adding a second path.

### Proof Before Expansion
- Green tests do **not** prove there are no implementation/behavior gaps; they only prove the covered scenarios.
- If a production symptom is reported, require a targeted regression test for that exact scenario before claiming the issue is closed.
- Do not treat broad test counts or “all tests pass” as proof of architectural correctness.
- When behavior depends on async ordering, detached tasks, or storage replay, require one explicit test for the timing boundary being discussed.
- If a docs or ADR update states a behavior, ensure there is either:
  - an automated test proving it, or
  - an explicit note that it is intended behavior with an implementation gap still open.

### Eventing & Notification Architecture
- Treat `SessionManager` as the central session-event bus unless the user explicitly requests a different architecture.
- Do not add emitter-specific persistence or delivery paths when the same behavior should flow through the shared session event architecture.
- Best-effort detached tasks are not acceptable for mandatory persistence guarantees.
- If persistence before request completion is required, the persistence step must be on the awaited request path.
- Avoid duplicate notification paths. If a new authoritative persistence/delivery path is introduced, explicitly redefine older paths as observer-only or remove them.
- For notification changes, verify all three concerns separately:
  - storage persistence,
  - live delivery,
  - duplicate suppression / single authoritative path.

### Planning Discipline
- Plans must distinguish:
  - mandatory now,
  - optional later,
  - and deferred future work.
- Do not merge immediate bug fixes, observability improvements, and larger architectural redesigns into one undifferentiated plan.
- For architecture changes, state explicitly:
  - what the current code already does,
  - what exact invariant is missing,
  - where the current boundary is broken,
  - and why smaller fixes are insufficient.
- If a plan proposes changing a core event path, include the exact files and the specific authoritative flow before coding.
- Stay inside the approved plan and stated requirement. Do not broaden scope by changing adjacent contracts, tests, or semantics unless the change is directly required by the approved fix.
- If implementation starts forcing unrelated API behavior changes, altered test expectations, or new architectural branches, stop and reassess instead of improvising.
- If there is real ambiguity about scope, architecture, or whether a change is still on-plan, stop and ask for clarification rather than guessing.

### Critic Review Mode (Architecture + Best Practices + MCP Compliance)
- Default stance for review-only requests: **no code changes** unless the user explicitly asks for a patch.
- Review output should prioritize findings over summaries:
  - Lead with concrete issues (severity-ordered) and file references.
  - Separate architecture risks, spec compliance risks, and documentation/process drift.
  - Call out missing tests/coverage when behavior claims change.
- Treat docs/examples/agent-instruction changes as potentially compliance-impacting:
  - Flag docs that advertise unsupported capabilities or incorrect defaults.
  - Flag examples that imply `listChanged`/subscription/progress/lifecycle support without matching implementation/tests.
  - Flag spec-version drift relative to the active branch's spec target (MCP 2025-11-25 on `main`; MCP 2026-07-28 on the `feat/turul-mcp-protocol-2026-07-28` branch). On `main`, do not back-port 2026-07-28 shapes; on the 2026-07-28 branch, do not preserve removed 2025-11-25 contracts (`initialize`/`notifications/initialized`/`Mcp-Session-Id`).
- When reviewing client/server API guidance, verify it preserves typed error propagation and truthful capability advertisement.

### Workspace State Triage (Required Before Review Conclusions)
- Start with `git status --short --branch` to identify whether changes are code, docs, tests, or agent/process files.
- If changes are primarily docs/agent guidance (e.g., `README.md`, `CLAUDE.md`, `.claude/agents/*`):
  - Perform a consistency audit across all agent guidance files and this `AGENTS.md`.
  - Check that MCP terminology, method names, capability keys, and spec date are consistent.
  - Check that testing commands and compliance expectations match the current framework guidance in this file.
- If no code changed but behavior claims changed, treat that as a review finding unless the claims are demonstrably accurate.

### Reviewer Focus Areas (Do Not Skip)
- Architecture boundaries: examples and docs should not encourage bypassing crate layering (`protocol` vs `server` vs transport crates).
- Capability truthfulness: docs/tests must not imply dynamic capabilities when the framework is static by default.
- Lifecycle strictness: on the 2025-11-25 baseline, guidance must preserve `notifications/initialized` gating and correct error mapping semantics. On the 2026-07-28 branch this rule is inapplicable (stateless core).
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
