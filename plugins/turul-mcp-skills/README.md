# turul-mcp-skills

Skills and tools for building MCP servers and clients with the [Turul MCP Framework](https://github.com/aussierobots/turul-mcp-framework) (Rust).

## What's Included (v0.3.1)

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
| `/new-mcp-server` | Command | Scaffold a Turul MCP server project with storage backend selection and dual validation |
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

Triggers on: "lambda", "LambdaMcpServerBuilder", "Lambda deployment", "lambda MCP server", "AWS Lambda MCP", "LambdaMcpHandler", "lambda cold start", "OnceCell handler", "lambda SSE", "run_with_streaming_response", "handle_streaming", "lambda CORS", "cors_allow_all_origins", "production_config", "development_config"

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

## Command

### /new-mcp-server

Scaffolds a new Turul MCP server project with:
- Storage backend selection (`--storage inmemory|sqlite|postgres|dynamodb`)
- Cargo.toml with correct dependencies and feature flags for the chosen backend
- A starter tool using the function macro pattern
- `.env.example` with connection string template (non-inmemory backends)
- Dual validation: full release gates in monorepo, local checks for external projects

## Version Compatibility

This plugin targets **turul-mcp-server v0.3** (MCP 2025-11-25).

## Changelog

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
