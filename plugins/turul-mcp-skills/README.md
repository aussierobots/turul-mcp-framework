# turul-mcp-skills

Skills and tools for building MCP servers and clients with the [Turul MCP Framework](https://github.com/aussierobots/turul-mcp-framework) (Rust).

## What's Included (v0.7.0)

Skills target **MCP 2026-07-28** (current default) unless noted; several also document the frozen 2025-11-25 opt-in lane (`--no-default-features --features protocol-2025-11-25`) where the two diverge.

| Component | Type | Purpose |
|---|---|---|
| `tool-creation-patterns` | Skill | Decision tree: function macro vs derive vs builder |
| `resource-prompt-patterns` | Skill | Resource creation (4 patterns) and prompt creation (3 patterns) with decision flowcharts |
| `output-schemas` | Skill | The `output = Type` requirement, schemars, Vec\<T\>, structuredContent, JSON Schema 2020-12 |
| `mcp-client-patterns` | Skill | Bilingual client negotiation (`server/discover` vs `initialize` fallback), transport selection, tool invocation, error handling |
| `middleware-patterns` | Skill | McpMiddleware trait, auth/rate-limit/logging/Lambda middleware, SessionInjection, per-request ephemeral sessions |
| `error-handling-patterns` | Skill | McpError decision tree, error codes (renumbered for 2026-07-28), From conversions, common mistakes |
| `task-patterns` | Skill | 2026-07-28 Tasks extension (SEP-2663: TaskStore, ext_task_tool, poll-based lifecycle) + frozen 2025-11-25 in-core tasks |
| `lambda-deployment` | Skill | LambdaMcpServerBuilder, cold-start caching, streaming vs snapshot, DynamoDB, CORS, middleware |
| `testing-patterns` | Skill | Unit tests (spec-agnostic), 2025-11-25 E2E harness, 2026-07-28 E2E via turul-mcp-client, compliance tests |
| `elicitation-workflows` | Skill | MRTR (InputRequiredResult/inputResponses) + frozen 2025-11-25 synchronous ElicitationProvider |
| `auth-patterns` | Skill | OAuth 2.1 RS, JWT validation, API key middleware, Lambda authorizer, RFC 9728 metadata |
| `authorization-server-patterns` | Skill | Demo OAuth 2.1 AS: PKCE flow, JWKS, token issuance, DCR (deprecated 2026-07-28), CIMD, MCP interop (demo-grade) |
| `/new-mcp-server` | Command | Scaffold a Turul MCP server project with storage backend selection and dual validation |
| `/validate-mcp-server` | Command | Validate an existing Turul MCP server for correctness, compliance, and best practices |
| `server-patterns-index` | Reference | Pointer index to CLAUDE.md/AGENTS.md authoritative sections |
| `storage-backend-matrix` | Reference | Feature flags, Cargo.toml patterns, and config for InMemory/SQLite/PostgreSQL/DynamoDB |

## Installation

**From plugin registry:**
```bash
claude plugin install turul-mcp-skills
```

**From repository:**
```bash
claude plugin install --url https://github.com/aussierobots/turul-mcp-framework --path plugins/turul-mcp-skills
```

**Local development:**
```bash
claude --plugin-dir plugins/turul-mcp-skills
```

## Skills

### middleware-patterns

Triggers on: "middleware", "McpMiddleware", "before_dispatch", "after_dispatch", "RequestContext", "SessionInjection", "MiddlewareError", "rate limiting middleware", "auth middleware", "logging middleware"

Guides you through creating HTTP middleware for cross-cutting concerns:
- **Auth middleware** — API key validation, session injection, Lambda authorizer extraction
- **Rate limiting** — Per-caller counters keyed on API key/bearer subject (not `session_id`, which is a fresh throwaway per request on 2026-07-28)
- **Logging/timing** — Request duration tracking with `before_dispatch`/`after_dispatch`
- **Error handling** — `MiddlewareError` variants with JSON-RPC code mapping
- Execution order, ephemeral-session semantics on 2026-07-28, common mistakes

### error-handling-patterns

Triggers on: "error handling", "McpError", "McpResult", "tool_execution", "missing_param", "invalid_param_type", "param_out_of_range", "JsonRpcError", "error code", "error conversion"

Covers the 3-layer error architecture and `McpError` variants:
- **Decision tree** — Choose the right error variant based on what went wrong
- **Parameter and not-found errors** — all map to the JSON-RPC standard `-32602` on 2026-07-28 (not-found errors moved off their own codes — breaking change from 2025-11-25's `-32002`)
- **Execution/validation/config codes** — renumbered into `-32000..-32019` to leave the spec's `-32020..-32099` reserved range alone
- **Spec-assigned codes (2026-07-28)** — `MissingRequiredClientCapability` (-32021), `UnsupportedProtocolVersion` (-32022)
- **String conversion** — `From<String>` and `From<&str>` → `ToolExecutionError` (with warnings)
- **`?` operator** — Which types have `From` impls, which need `.map_err()`
- Full JSON-RPC error code table (verified against `turul-mcp-protocol-2026-07-28`), common mistakes

### task-patterns

Triggers on: "task support", "TaskStore", "InMemoryTaskStore", "with_ext_tasks", "ext_task_tool", "Tasks extension", "SEP-2663", "tasks/get", "tasks/update", "tasks/cancel", "long-running tool"

Covers the 2026-07-28 Tasks extension (SEP-2663) for long-running tools, plus the frozen 2025-11-25 in-core system:
- **2026-07-28 lifecycle** — poll-based (`tasks/get`), no `tasks/list`, no blocking `tasks/result`; `tasks/update` delivers input responses
- **Server setup** — `.with_ext_tasks(store)` + `.ext_task_tool()` / `.ext_task_tool_required()` (progressive enhancement, opt-in `ext-tasks` feature)
- **TaskStore trait** — `InMemoryTaskStore` is the only backend shipped today (no SQLite/Postgres/DynamoDB yet)
- **2025-11-25 in-core tasks (frozen)** — `task_support` attribute, `TaskRuntime`, `.with_task_storage()`, `tasks/list`/`tasks/result`, state machine, cancellation, SQLite/Postgres/DynamoDB backends

### lambda-deployment

Triggers on: "lambda", "LambdaMcpServerBuilder", "Lambda deployment", "lambda MCP server", "AWS Lambda MCP", "LambdaMcpHandler", "lambda cold start", "OnceCell handler", "lambda SSE", "run_streaming", "run_streaming_with", "handle_streaming", "lambda CORS", "cors_allow_all_origins", "production_config", "development_config"

Guides you through deploying MCP servers on AWS Lambda:
- **Builder** — `LambdaMcpServerBuilder` with all builder methods, feature flags, convenience presets
- **Cold-start caching** — `OnceCell<LambdaMcpHandler>` pattern for handler reuse
- **Streaming vs snapshot** — 4 handler/runtime combinations; GET /mcp is unconditionally 405 on 2026-07-28 regardless of mode (the standalone GET SSE listener is removed)
- **Session storage** — ephemeral per-request on 2026-07-28 (DynamoDB not needed for session continuity); real cross-invocation sessions only on a 2025-11-25 build
- **Task support** — no Tasks-extension wiring yet for 2026-07-28; `.with_task_storage()`/DynamoDB task persistence is 2025-11-25-only
- **CORS** — `cors_allow_all_origins()`, `cors_from_env()`, `cors_allow_origins()`
- **Middleware, logging** — Same traits as HTTP servers, CloudWatch-optimized
- Common mistakes, environment variables, API Gateway authorizer integration

### resource-prompt-patterns

Triggers on: "create a resource", "MCP resource", "McpResource", "mcp_resource", "resource!", "ResourceBuilder", "create a prompt", "MCP prompt", "McpPrompt", "prompt!", "PromptBuilder"

Guides you through choosing the right resource or prompt creation approach:
- **Resources** — 4 patterns: Function Macro (`#[mcp_resource]`), Derive (`#[derive(McpResource)]`), Declarative (`resource!{}`), Builder (`ResourceBuilder`)
- **Prompts** — 3 patterns: Derive (`#[derive(McpPrompt)]`), Declarative (`prompt!{}`), Builder (`PromptBuilder`)
- Decision flowcharts, comparison table, common mistakes, cross-references

### tool-creation-patterns

Triggers on: "create a tool", "mcp_tool macro", "derive McpTool", "ToolBuilder", "which tool pattern"

Guides you through choosing the right tool creation approach:
- **Level 1 — Function Macro** (`#[mcp_tool]`): Quick-start for simple, stateless tools
- **Level 2 — Derive Macro** (`#[derive(McpTool)]`): Complex tools needing session access, custom output types
- **Level 3 — Builder** (`ToolBuilder`): Dynamic/runtime tool construction

### output-schemas

Triggers on: "output schema", "structuredContent", "schemars", "output_field", "Vec output"

Covers the most common gotchas with MCP tool output:
- Why `output = Type` is mandatory on derive macros
- Automatic schemars detection
- Vec\<T\> output patterns
- `output_field` customization
- structuredContent auto-generation

### mcp-client-patterns

Triggers on: "MCP client", "McpClient", "McpClientBuilder", "connect to MCP server", "HttpTransport", "SseTransport", "client session", "ToolCallResponse"

Covers building MCP client applications with `turul-mcp-client` (bilingual by default):
- Transport selection (auto-detect, HttpTransport, SseTransport) and per-connection wire-spec negotiation (`server/discover` probe, `initialize` fallback)
- Connection lifecycle (connect, disconnect, connection states) — no server-side session on a negotiated 2026-07-28 connection
- Tool/resource/prompt invocation from the client side
- Error handling (McpClientError variants, retryability, backoff; `is_resource_not_found(version)` accepts legacy -32002 only on the 2025-11-25 lane)
- Configuration (ClientConfig, timeouts, retries, connection settings)

### testing-patterns

Triggers on: "testing", "test patterns", "write tests", "unit test", "e2e test", "integration test", "McpTestClient", "TestServerManager", "compliance test", "test server", "test fixture", "doctest", "cargo test"

Covers three testing layers for MCP servers:
- **Unit testing** — `tool.call()` with framework-native API, `#[tokio::test]` (spec-agnostic)
- **2025-11-25 E2E testing** — `TestServerManager::start()` + `McpTestClient` for full HTTP round-trips (session handshake-based)
- **2026-07-28 E2E testing** — drive `turul-mcp-client` (bilingual) against a `TestServerManager`-started server; no equivalent of `McpTestClient` exists yet for the stateless core
- **Compliance testing** — 4 compliance modules (JSON-RPC format, capabilities, behavior, tools)
- **SSE testing (2025-11-25 only)** — `call_tool_with_sse()`, event parsing, `Last-Event-ID` replay; the GET SSE listener this targets is unconditionally 405 on 2026-07-28
- Test organization (consolidated binaries, `autotests = false`), doctest strategy, common mistakes

### elicitation-workflows

Triggers on: "elicitation", "ElicitationBuilder", "elicit", "ElicitResult", "ElicitAction", "ElicitationProvider", "PrimitiveSchemaDefinition", "ElicitationSchema", "with_elicitation"

Covers MCP elicitation for collecting structured user input:
- **Schema primitives** — StringSchema, NumberSchema, BooleanSchema, EnumSchema (no nesting) — unchanged across spec lanes
- **ElicitationBuilder** — Field methods, convenience constructors (`text_input`, `confirm`, `choice`)
- **MRTR (2026-07-28)** — `McpError::InputRequired`, `session.input_responses()`/`mrtr_request_state()`, `ElicitRequest::new_form()` — no server builder opt-in needed
- **Synchronous elicitation (frozen 2025-11-25)** — `.with_elicitation()` (mock) vs `.with_elicitation_provider(custom)`, session-state multi-step workflows
- Validation via `DynamicElicitation`, common mistakes

### auth-patterns

Triggers on: "OAuth", "authentication", "authorization", "JWT", "Bearer", "JwtValidator", "oauth_resource_server", "ProtectedResourceMetadata", "turul-mcp-oauth", "API key auth", "auth middleware", "token validation", "WWW-Authenticate", "audience validation", "OAuthResourceMiddleware", "TokenClaims", "JWKS", "RFC 9728"

Covers authentication and authorization patterns for MCP servers:
- **Decision tree** — OAuth 2.1 RS vs API key middleware vs Lambda authorizer
- **OAuth 2.1 RS** — `ProtectedResourceMetadata`, `oauth_resource_server()`, `JwtValidator`, RFC 9728 metadata
- **Audience validation** — Why it's mandatory, how `required_audience` works
- **Token claims** — Reading `TokenClaims` in tools via `get_typed_extension()`
- **JWKS caching** — Key rotation handling, rate-limited refresh, Lambda cold-start behavior
- **API key middleware** — Simple alternative using `McpMiddleware`
- **Lambda + OAuth** — `.route()` for `.well-known` endpoints, `run_streaming()` for standard streaming
- Common mistakes, OAuthError variants, WWW-Authenticate header format

### authorization-server-patterns

Triggers on: "authorization server", "OAuth AS", "token issuer", "PKCE", "authorization code flow", "oauth-authorization-server", "DCR", "dynamic client registration", "CIMD", "client metadata", "demo auth server", "token endpoint", "authorize endpoint", "JWKS signing key", "access token issuance", "refresh token"

Demo-grade patterns for building a standalone OAuth 2.1 Authorization Server:
- **AS vs RS role separation** — what each side does, how they connect
- **Required endpoints** — AS metadata, JWKS, /authorize, /token
- **Client models** — pre-registered, DCR (deprecated 2026-07-28, 12-month window), CIMD (standards-preferred direction for 2026-07-28, adds `.well-known` suffix + issuer-binding requirements)
- **PKCE flow** — authorization code + S256 challenge/verifier
- **Token issuance** — JWT access tokens, opaque refresh tokens, audience/scope validation
- **Signing key management** — static demo key vs ephemeral (restart consequences)
- **MCP interoperability** — client discovery chain, connecting demo AS to Turul RS
- Common mistakes, redirect URI allowlisting, security boundaries

## Commands

### /new-mcp-server

Scaffolds a new Turul MCP server project with:
- Storage backend selection (`--storage inmemory|sqlite|postgres|dynamodb`)
- Cargo.toml with correct dependencies and feature flags for the chosen backend
- A starter tool using the function macro pattern
- `.env.example` with connection string template (non-inmemory backends)
- Dual validation: full release gates in monorepo, local checks for external projects

### /validate-mcp-server

Validates an existing Turul MCP server project:
- Auto-detects monorepo vs external project
- **Monorepo mode** — Runs all 7 release gate tests (compliance, lifecycle, capability truthfulness, E2E)
- **External mode** — `cargo check` + `cargo clippy` + `cargo test`
- **Additional checks** — Turul dependency presence, MCP component registration, forbidden direct protocol imports, derive macro `output` attributes, builder `.name()`/`.version()`, `JsonRpcError` usage in handlers
- Report with pass/fail/warn per check and actionable fix suggestions

## Version Compatibility

This plugin targets **turul-mcp-server v0.4**, current default spec **MCP 2026-07-28**. The frozen **MCP 2025-11-25** lane remains available as an opt-in build (`--no-default-features --features protocol-2025-11-25`) and is documented where skills diverge between the two.

## Changelog

### v0.7.0
- **Deleted**: `session-storage-backends` skill. Its entire subject — session persistence and SSE-reconnection resumability keyed by `Mcp-Session-Id`/`Last-Event-ID` — is removed by 2026-07-28's stateless core.
- **Rewritten for 2026-07-28**: `output-schemas` (JSON Schema 2020-12; Vec\<T\> wrapper-struct requirement re-verified as still necessary — the framework's derive/function-macro output-schema path is still object-root-constrained even though the wire-level `outputSchema` is now unrestricted), `mcp-client-patterns` (bilingual negotiation via `server/discover`, no `initialize` on 2026-07-28), `middleware-patterns` (dropped the `initialize`/`ping` skip — both removed methods; documented that `session` is a fresh per-request ephemeral session, not `None`, on 2026-07-28), `error-handling-patterns` (full error-code table renumbering verified against `turul-mcp-protocol-2026-07-28`), `task-patterns` (2026-07-28 Tasks extension, SEP-2663, is a different API from the frozen 2025-11-25 in-core system — both documented), `lambda-deployment` (GET /mcp unconditionally 405 on 2026-07-28; ephemeral sessions; no Tasks-extension wiring yet), `elicitation-workflows` (MRTR replaces synchronous `ElicitationProvider`), `auth-patterns` (added a "Not Yet Implemented" section for 2026-07-28 auth-hardening SEPs), `authorization-server-patterns` (DCR deprecation banner, CIMD 2026-07-28 additions).
- **Lane-scoped**: every surviving `SKILL.md` now states its spec lane up front. `tool-creation-patterns` and `resource-prompt-patterns` kept as-is (pure compile-time authoring mechanics, verified to not reference `initialize`/session/SSE/tasks/elicitation).
- **Version bump**: `turul-mcp-server v0.3` → `v0.4` and `turul-mcp-client v0.3` → `v0.4` across SKILL.md, examples, and references.

### v0.6.3
- **Fixed**: `task-patterns` dead constructors — `SqliteTaskStorage::new("…")`, `PostgresTaskStorage::new("…")`, `DynamoDbTaskStorage::new("…")` did not compile against the current API. All three now use `with_config(<BackendConfig>{...})` form, matching `lambda-deployment`. Also corrected the stale "auto-creates tables on connect" note in `references/task-storage-guide.md` (tables are migrated only when `verify_tables = true`).
- **Fixed**: `x-authorizer-principalid` examples in `auth-patterns`, `middleware-patterns`, `lambda-deployment` (SKILL.md + reference + 2 example .rs files). The Lambda adapter intentionally skips API Gateway internals (`principalId`, `integrationLatency`, `usageIdentifierKey`); examples now use concrete custom-context fields (`x-authorizer-user_id`, `x-authorizer-account_id`, `x-authorizer-scope`) with an explicit "not forwarded" note.
- **Added**: `mcp-client-patterns` — new "Bearer Lifecycle & Rotation" section covering `set_bearer()` (v0.3.44), idempotent `disconnect()`/`Drop` (v0.3.43), SSE GET 4xx terminal behavior (v0.3.38), concurrent `call_tool` via `&self` transport (v0.3.33), and the MCP requirement that bearer auth is present on every HTTP request including GET listeners and DELETE cleanup.
- **Added**: `auth-patterns` — new "Authorization Server Discovery Chain (RFC 8414)" section spelling out the RS PRM → `authorization_servers` → AS metadata fetch flow, with explicit "the RS does NOT serve `/.well-known/oauth-authorization-server`" guidance and quoted MCP 2025-11-25 normative MUSTs.
- **Added**: `auth-patterns` — new "Resource Parameter (RFC 8707)" section covering the client-side MCP MUST that `resource` is included in both `/authorize` and `/token` requests, and the corresponding RS implementer responsibilities (canonical URI alignment between `ProtectedResourceMetadata::new`, `JwtValidator::new`, and what clients send).
- **Added**: `auth-patterns` — two new "Common Mistakes" entries: resource/audience URI mismatch (the most common interop failure) and the RS-vs-AS discovery boundary.
- **Added**: 7 frontmatter `Do NOT use for ...` disambiguation clauses to reduce trigger collisions:
  - `auth-patterns` ↔ `authorization-server-patterns` (validate-vs-issue boundary)
  - `task-patterns` ↔ `session-storage-backends` (reciprocal — TaskStorage ≠ SessionStorage)
  - `testing-patterns` ↔ `mcp-client-patterns` (test harness vs production client)
  - `middleware-patterns` ↔ `auth-patterns` (plumbing vs OAuth specifics)
  - `mcp-client-patterns` (client-side vs server-side scope)
  - `authorization-server-patterns` (tightened to "demo examples are not production identity infrastructure" per spec)
- **Verified**: smoke-tested 5 skill patterns in a throwaway crate (`/tmp/turul-skills-smoke`) — `echo_text`, `add_numbers`, `server_time_stub` (function macro), `WordCountTool` (derive + `output = Type` + schemars), `slow_add` (function macro + `task_support = "optional"`), plus `InMemoryTaskStorage::new()` — all compile cleanly against current crate APIs. No production examples touched.
- **Scope**: skills-only release. Framework crates were not modified; status-code teaching unchanged.

### v0.6.2
- Added server identity (icons) section to `tool-creation-patterns` skill with `.icons()` builder method, `Icon::data_uri()`, and trigger phrases
- Added session 404 status code table to `mcp-client-patterns` error handling section (MCP 2025-11-25 compliance)
- Added session expiry behavior section to `session-storage-backends` skill (terminated/expired → 404)
- Added server icons row to `server-patterns-index` reference
- Fixed `plugin.json` version mismatch (was 0.5.0, now 0.6.2)

### v0.6.1
- Added `authorization-server-patterns` skill: demo OAuth 2.1 AS with PKCE, JWKS, pre-registered clients, DCR, CIMD patterns, MCP interop notes, 1 reference file, 2 example files
- Added Authorization Server patterns row to `server-patterns-index`

### v0.6.0
- Added `auth-patterns` skill: OAuth 2.1 RS, JWT validation with JWKS caching, API key middleware, Lambda authorizer integration, RFC 9728 metadata, 1 reference file, 3 example files
- Added Auth patterns row to `server-patterns-index`
- Updated `lambda-deployment` stale `run_with_streaming_response` references to `run_streaming` / `run_streaming_with`
- Updated streaming example, builder reference, and streaming modes guide for v0.3 API

### v0.5.0
- Added `session-storage-backends` skill: SessionStorage trait, backend decision tree, event management for SSE resumability, error types, background cleanup, 3 example files
- Added `/validate-mcp-server` command: monorepo/external detection, 7 release gates, 6 additional static checks, pass/fail/warn report
- Added Session storage backends row to `server-patterns-index`
- Updated `lambda-deployment` "Beyond This Skill" with session-storage-backends hand-off

### v0.4.0
- Added `testing-patterns` skill: unit testing, E2E testing (McpTestClient + TestServerManager), compliance tests, SSE testing, test organization, doctest strategy, 1 reference file, 3 example files
- Added `elicitation-workflows` skill: ElicitationBuilder, schema primitives, multi-step workflows, ElicitationProvider trait, DynamicElicitation validation, 1 reference file, 3 example files
- Added Testing patterns and Elicitation workflows rows to `server-patterns-index`
- Updated `tool-creation-patterns` "Beyond This Skill" with testing-patterns hand-off

### v0.3.1
- Added `lambda-deployment` skill: LambdaMcpServerBuilder, cold-start caching, streaming vs snapshot modes, DynamoDB session/task storage, CORS, middleware, API Gateway authorizer integration, 2 reference files, 4 example files
- Updated Lambda deployment row in `server-patterns-index` to point to skill
- Updated `middleware-patterns` "Beyond This Skill" with lambda-deployment hand-off

### v0.3.0
- Added `middleware-patterns` skill: McpMiddleware trait, auth/rate-limit/logging/Lambda middleware, SessionInjection, MiddlewareError variants, execution order, 4 example files
- Added `error-handling-patterns` skill: McpError decision tree, all 22 variants with JSON-RPC error codes, From conversions, parameter/execution/custom error examples
- Added `task-patterns` skill: task state machine, TaskRuntime/TaskStorage/TaskExecutor, task_support declaration, 4 storage backends, cancellation, capability truthfulness
- Added Middleware, Error handling, Task patterns rows to `server-patterns-index`
- Updated `tool-creation-patterns` and `resource-prompt-patterns` "Beyond This Skill" with middleware/error/task hand-offs

### v0.2.0
- Added `resource-prompt-patterns` skill: 4 resource patterns (function macro, derive, declarative, builder) + 3 prompt patterns (derive, declarative, builder) with decision flowcharts, comparison table, and 7 example files
- Added Resource creation and Prompt creation rows to `server-patterns-index`
- Updated `tool-creation-patterns` "Beyond This Skill" with resource/prompt hand-off

### v0.1.2
- Added `mcp-client-patterns` skill: transport selection, connection lifecycle, tool invocation, task workflows, error handling for `turul-mcp-client`
- Added MCP Client row to `server-patterns-index`

### v0.1.1
- Added `storage-backend-matrix` reference: decision matrix, feature flags, Cargo.toml patterns, config structs for all storage backends
- Updated `/new-mcp-server` scaffold with `--storage` flag (inmemory/sqlite/postgres/dynamodb) and `.env.example` generation
- Added storage backend row to `server-patterns-index`

### v0.1.0
- Initial release: tool-creation-patterns skill, output-schemas skill, /new-mcp-server command, server-patterns-index reference
