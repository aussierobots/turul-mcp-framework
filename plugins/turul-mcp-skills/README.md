# turul-mcp-skills

Skills and tools for building MCP servers and clients with the [Turul MCP Framework](https://github.com/aussierobots/turul-mcp-framework) (Rust).

## What's Included (v0.6.2)

| Component | Type | Purpose |
|---|---|---|
| `tool-creation-patterns` | Skill | Decision tree: function macro vs derive vs builder |
| `resource-prompt-patterns` | Skill | Resource creation (4 patterns) and prompt creation (3 patterns) with decision flowcharts |
| `output-schemas` | Skill | The `output = Type` requirement, schemars, Vec\<T\>, structuredContent |
| `mcp-client-patterns` | Skill | Transport selection, connection lifecycle, tool invocation, task workflows, error handling |
| `middleware-patterns` | Skill | McpMiddleware trait, auth/rate-limit/logging/Lambda middleware, SessionInjection, error handling |
| `error-handling-patterns` | Skill | McpError decision tree, 22 variants, error codes, From conversions, common mistakes |
| `task-patterns` | Skill | Task state machine, TaskRuntime, TaskStorage backends, task\_support attribute, cancellation |
| `lambda-deployment` | Skill | LambdaMcpServerBuilder, cold-start caching, streaming vs snapshot, DynamoDB, CORS, middleware, tasks |
| `testing-patterns` | Skill | Unit tests, E2E tests, compliance tests, McpTestClient, TestServerManager, doctest strategy |
| `elicitation-workflows` | Skill | ElicitationBuilder, schema primitives, multi-step workflows, ElicitationProvider, validation |
| `session-storage-backends` | Skill | SessionStorage trait, backend decision tree, event management, SSE resumability, error types |
| `auth-patterns` | Skill | OAuth 2.1 RS, JWT validation, API key middleware, Lambda authorizer, RFC 9728 metadata |
| `authorization-server-patterns` | Skill | Demo OAuth 2.1 AS: PKCE flow, JWKS, token issuance, DCR, MCP interop (demo-grade) |
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
- **Rate limiting** — Per-session counters with configurable windows and `retry_after`
- **Logging/timing** — Request duration tracking with `before_dispatch`/`after_dispatch`
- **Error handling** — 6 `MiddlewareError` variants with JSON-RPC code mapping
- Execution order, session injection lifecycle, common mistakes

### error-handling-patterns

Triggers on: "error handling", "McpError", "McpResult", "tool_execution", "missing_param", "invalid_param_type", "param_out_of_range", "JsonRpcError", "error code", "error conversion"

Covers the 3-layer error architecture and all 22 `McpError` variants:
- **Decision tree** — Choose the right error variant based on what went wrong
- **Parameter errors** — `missing_param`, `invalid_param_type`, `param_out_of_range` (all -32602)
- **Execution errors** — `tool_execution` (-32010), `resource_execution` (-32012), `prompt_execution` (-32013)
- **String conversion** — `From<String>` and `From<&str>` → `ToolExecutionError` (with warnings)
- **`?` operator** — Which types have `From` impls, which need `.map_err()`
- Full JSON-RPC error code table, common mistakes

### task-patterns

Triggers on: "task support", "TaskRuntime", "TaskStorage", "task_support attribute", "long-running tool", "CancellationHandle", "tasks/get", "tasks/list", "tasks/cancel", "tasks/result", "InMemoryTaskStorage"

Covers MCP task support for long-running tools:
- **State machine** — Working/InputRequired/Completed/Failed/Cancelled with valid transitions
- **Server setup** — `.with_task_storage()`, `.with_task_runtime()`, `TaskRuntime::in_memory()`
- **Tool declaration** — `task_support = "optional"/"required"/"forbidden"` on all three macro patterns
- **Storage backends** — InMemory, SQLite, PostgreSQL, DynamoDB (brief table, hand-off to storage-backend-matrix)
- **Cancellation** — How TokioTaskExecutor handles cancellation transparently
- **Capability truthfulness** — Server strips `execution` when no runtime; `required` without runtime errors at build time

### lambda-deployment

Triggers on: "lambda", "LambdaMcpServerBuilder", "Lambda deployment", "lambda MCP server", "AWS Lambda MCP", "LambdaMcpHandler", "lambda cold start", "OnceCell handler", "lambda SSE", "run_streaming", "run_streaming_with", "handle_streaming", "lambda CORS", "cors_allow_all_origins", "production_config", "development_config"

Guides you through deploying MCP servers on AWS Lambda:
- **Builder** — `LambdaMcpServerBuilder` with all builder methods, feature flags, convenience presets
- **Cold-start caching** — `OnceCell<LambdaMcpHandler>` pattern for handler reuse
- **Streaming vs snapshot** — 4 handler/runtime combinations, when to use each
- **DynamoDB storage** — Session and task persistence across Lambda invocations
- **CORS** — `cors_allow_all_origins()`, `cors_from_env()`, `cors_allow_origins()`
- **Middleware, tasks, logging** — Same traits as HTTP servers, CloudWatch-optimized
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

Covers building MCP client applications with `turul-mcp-client`:
- Transport selection (auto-detect, HttpTransport, SseTransport)
- Connection lifecycle (connect, disconnect, session states)
- Tool/resource/prompt invocation from the client side
- Task workflows (call_tool_with_task, polling, all TaskStatus variants)
- Error handling (McpClientError variants, retryability, backoff)
- Configuration (ClientConfig, timeouts, retries, connection settings)

### testing-patterns

Triggers on: "testing", "test patterns", "write tests", "unit test", "e2e test", "integration test", "McpTestClient", "TestServerManager", "compliance test", "test server", "test fixture", "doctest", "cargo test"

Covers three testing layers for MCP servers:
- **Unit testing** — `tool.call()` with framework-native API, `#[tokio::test]`
- **E2E testing** — `TestServerManager::start()` + `McpTestClient` for full HTTP round-trips
- **Compliance testing** — 4 compliance modules (JSON-RPC format, capabilities, behavior, tools)
- **SSE testing** — `call_tool_with_sse()`, event parsing, `Last-Event-ID` replay
- Test organization (consolidated binaries, `autotests = false`), doctest strategy, common mistakes

### elicitation-workflows

Triggers on: "elicitation", "ElicitationBuilder", "elicit", "ElicitResult", "ElicitAction", "ElicitationProvider", "PrimitiveSchemaDefinition", "ElicitationSchema", "with_elicitation"

Covers MCP elicitation for collecting structured user input:
- **Schema primitives** — StringSchema, NumberSchema, BooleanSchema, EnumSchema (no nesting)
- **ElicitationBuilder** — Field methods, convenience constructors (`text_input`, `confirm`, `choice`)
- **Response handling** — `ElicitAction::Accept`/`Decline`/`Cancel`, `ElicitResultBuilder`
- **Server setup** — `.with_elicitation()` (mock) vs `.with_elicitation_provider(custom)`
- **Multi-step workflows** — Sequential elicitations with session state accumulation
- Validation via `DynamicElicitation`, common mistakes

### session-storage-backends

Triggers on: "session storage", "SessionStorage trait", "SqliteSessionStorage", "PostgresSessionStorage", "DynamoDbSessionStorage", "InMemorySessionStorage", "session backend", "session persistence", "SSE reconnection storage"

Covers the SessionStorage trait and backend architecture:
- **Backend decision tree** — InMemory → SQLite → PostgreSQL → DynamoDB based on persistence/scaling needs
- **SessionStorage trait** — Session lifecycle, state management, event management for SSE resumability
- **Event management** — `store_event()`, `get_events_after()`, `Last-Event-ID` replay
- **Backend-specific gotchas** — DynamoDB 5-min TTL, SQLite `:memory:` pool isolation, PostgreSQL optimistic locking
- Error types (`SessionStorageError`), background cleanup patterns, common mistakes

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
- **Client models** — pre-registered, DCR, CIMD (MCP 2025-11-25 supported)
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

This plugin targets **turul-mcp-server v0.3** (MCP 2025-11-25).

## Changelog

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
