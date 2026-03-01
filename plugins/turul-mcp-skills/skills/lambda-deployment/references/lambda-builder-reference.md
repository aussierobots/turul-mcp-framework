# LambdaMcpServerBuilder API Reference

Full API reference for `turul_mcp_aws_lambda::LambdaMcpServerBuilder`.

## Construction

```rust
let builder = LambdaMcpServerBuilder::new();
// or
let builder = LambdaMcpServerBuilder::default();
```

## Builder Methods

### Metadata

| Method | Signature | Notes |
|---|---|---|
| `.name()` | `fn name(self, name: impl Into<String>) -> Self` | **Required.** Server name. |
| `.version()` | `fn version(self, version: impl Into<String>) -> Self` | **Required.** Server version. |
| `.title()` | `fn title(self, title: impl Into<String>) -> Self` | Optional display title. |
| `.instructions()` | `fn instructions(self, instructions: impl Into<String>) -> Self` | Client instructions. |

### Tools, Resources, Prompts

| Method | Signature | Notes |
|---|---|---|
| `.tool()` | `fn tool<T: McpTool + 'static>(self, tool: T) -> Self` | Register a single tool. |
| `.tool_fn()` | `fn tool_fn<F, T>(self, func: F) -> Self` | Register a function macro tool. |
| `.tools()` | `fn tools<T, I>(self, tools: I) -> Self` | Register multiple tools. |
| `.resource()` | `fn resource<R: McpResource + 'static>(self, resource: R) -> Self` | Auto-detects template URIs. |
| `.resources()` | `fn resources<R, I>(self, resources: I) -> Self` | Register multiple resources. |
| `.prompt()` | `fn prompt<P: McpPrompt + 'static>(self, prompt: P) -> Self` | Register a prompt. |
| `.prompts()` | `fn prompts<P, I>(self, prompts: I) -> Self` | Register multiple prompts. |

### Capabilities

| Method | Notes |
|---|---|
| `.with_completion()` | Enable completion support. |
| `.with_prompts()` | Enable prompts capability flag. |
| `.with_resources()` | Enable resources with list/read/templates handlers. |
| `.with_logging()` | Enable logging capability. |
| `.with_roots()` | Enable roots support. |
| `.with_sampling()` | Enable sampling support. |
| `.with_elicitation()` | Enable elicitation with default mock provider. |
| `.with_elicitation_provider(p)` | Enable elicitation with custom provider. |
| `.with_notifications()` | Enable notifications support. |

### Session Storage

| Method | Signature | Notes |
|---|---|---|
| `.storage()` | `fn storage(self, storage: Arc<BoxedSessionStorage>) -> Self` | Set session storage backend. |
| `.dynamodb_storage()` | `async fn dynamodb_storage(self) -> Result<Self>` | DynamoDB from env. Requires `dynamodb` feature. |
| `.session_timeout_minutes()` | `fn session_timeout_minutes(self, minutes: u64) -> Self` | Default: 30. |
| `.session_cleanup_interval_seconds()` | `fn session_cleanup_interval_seconds(self, seconds: u64) -> Self` | Default: 60. |
| `.with_long_sessions()` | `fn with_long_sessions(self) -> Self` | 2hr timeout, 5min cleanup. |
| `.with_short_sessions()` | `fn with_short_sessions(self) -> Self` | 5min timeout, 30s cleanup. |

### SSE and Streaming

| Method | Signature | Notes |
|---|---|---|
| `.sse()` | `fn sse(self, enable: bool) -> Self` | Enable/disable SSE. Default: `cfg!(feature = "sse")` (true). |
| `.server_config()` | `fn server_config(self, config: ServerConfig) -> Self` | Advanced server config. |
| `.stream_config()` | `fn stream_config(self, config: StreamConfig) -> Self` | SSE stream tuning. |

### CORS (requires `cors` feature)

| Method | Signature | Notes |
|---|---|---|
| `.cors()` | `fn cors(self, config: CorsConfig) -> Self` | Custom CORS config. |
| `.cors_allow_all_origins()` | `fn cors_allow_all_origins(self) -> Self` | Development: allow all. |
| `.cors_allow_origins()` | `fn cors_allow_origins(self, origins: Vec<String>) -> Self` | Production: specific origins. |
| `.cors_from_env()` | `fn cors_from_env(self) -> Self` | Read `MCP_CORS_*` env vars. |
| `.cors_disabled()` | `fn cors_disabled(self) -> Self` | No CORS headers. |

### Middleware

| Method | Signature | Notes |
|---|---|---|
| `.middleware()` | `fn middleware(self, mw: Arc<dyn McpMiddleware>) -> Self` | Add middleware. FIFO before, LIFO after. |

### Task Support

| Method | Signature | Notes |
|---|---|---|
| `.with_task_storage()` | `fn with_task_storage(self, storage: Arc<dyn TaskStorage>) -> Self` | Enable tasks with default executor. |
| `.with_task_runtime()` | `fn with_task_runtime(self, runtime: Arc<TaskRuntime>) -> Self` | Enable tasks with custom runtime. |
| `.task_recovery_timeout_ms()` | `fn task_recovery_timeout_ms(self, timeout_ms: u64) -> Self` | Stuck task recovery. Default: 300,000 (5 min). |

### Lifecycle

| Method | Signature | Notes |
|---|---|---|
| `.strict_lifecycle()` | `fn strict_lifecycle(self, strict: bool) -> Self` | Enforce MCP lifecycle. |
| `.with_strict_lifecycle()` | `fn with_strict_lifecycle(self) -> Self` | Convenience: `strict_lifecycle(true)`. |

### Convenience Presets

| Method | Requires | What it does |
|---|---|---|
| `.production_config().await?` | `dynamodb` + `cors` | `dynamodb_storage()` + `cors_from_env()` |
| `.development_config()` | `cors` | `InMemorySessionStorage` + `cors_allow_all_origins()` |

### Build

```rust
let server: LambdaMcpServer = builder.build().await?;
```

**Validation rules:**
- `name` must not be empty
- `version` must not be empty
- Capabilities are auto-detected from registered components (tools, resources, prompts, etc.)
- Task handlers auto-registered when task runtime is configured
- Defaults to `InMemorySessionStorage` if no storage provided

## LambdaMcpServer

Created by `builder.build().await?`.

| Method | Signature | Notes |
|---|---|---|
| `.handler()` | `async fn handler(&self) -> Result<LambdaMcpHandler>` | Create handler for Lambda runtime. |
| `.capabilities()` | `fn capabilities(&self) -> &ServerCapabilities` | Inspect server capabilities. |

## LambdaMcpHandler

Created by `server.handler().await?`. Implements `Clone` — safe to share across invocations.

| Method | Return Type | Notes |
|---|---|---|
| `.handle(req)` | `Result<LambdaResponse<LambdaBody>>` | Snapshot mode. Error type: `LambdaError`. Use with `run()`. |
| `.handle_streaming(req)` | `Result<Response<UnsyncBoxBody<Bytes, hyper::Error>>, Box<dyn Error + Send + Sync>>` | Streaming mode. Error type differs from `handle()`. Use with `run_with_streaming_response()`. |
| `.get_stream_manager()` | `&Arc<StreamManager>` | Access SSE stream manager. |

**Note:** `handle()` and `handle_streaming()` have **different error types**. Copy the exact return type from the examples when writing your `lambda_handler` function signature.

## Feature Flags

| Feature | Default | Enables |
|---|---|---|
| `cors` | Yes | CORS header injection. |
| `sse` | Yes | SSE stream adaptation. |
| `streaming` | No | Real SSE streaming (`run_with_streaming_response` + `handle_streaming`). Implies `sse`. |
| `dynamodb` | No | DynamoDB session storage. Enables `turul-mcp-session-storage/dynamodb`. |
