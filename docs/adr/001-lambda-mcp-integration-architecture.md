# ADR-001: Lambda MCP Integration Architecture

**Date**: 2025-08-31  
**Status**: Accepted  
**Context**: AWS Lambda integration for turul-mcp-framework  

## Context and Problem Statement

The turul-mcp-framework was designed with a traditional HTTP server architecture that runs its own TCP listener. However, AWS Lambda provides a different execution model where the HTTP runtime is managed by the Lambda service itself. This creates an architectural mismatch that needs to be resolved to enable serverless MCP server deployment.

During the lambda-mcp-server development, we discovered fundamental incompatibilities between the framework's architecture and Lambda's execution model, leading to the need for a comprehensive integration solution.

## Framework Architecture Discovery

### 3-Layer Architecture

Through analysis, we discovered the framework has a 3-layer architecture:

1. **Layer 1: McpServer** (`turul-mcp-server`)
   - High-level builder and handler management
   - Tool registration and capability configuration
   - Internal handlers: `SessionAwareToolHandler`, `SessionAwareInitializeHandler`
   - **Problem**: Handlers are internal and not exposed for external use

2. **Layer 2: HttpMcpServer** (`turul-http-mcp-server`)  
   - TCP server using hyper with `TcpListener::bind()`
   - HTTP routing and middleware
   - **Problem**: Lambda provides the HTTP runtime - we can't run our own TCP server

3. **Layer 3: SessionMcpHandler** (`turul-http-mcp-server`)
   - Request handler implementing `handle_mcp_request(req: hyper::Request<Incoming>)`
   - Session management and storage integration
   - **Solution**: This is what Lambda integration actually needs

### Type System Analysis

All components use hyper as the foundational HTTP library:
- **McpServer** → creates **HttpMcpServer** → creates **SessionMcpHandler**
- **SessionMcpHandler** expects `hyper::Request<hyper::body::Incoming>`  
- **Lambda** provides `lambda_http::Request` (based on `http` crate)
- **AWS SDK** also uses hyper internally

**Key Insight**: The common hyper foundation enables integration through type conversion.

## Integration Challenges

### 1. Type Incompatibility
```rust
// Lambda provides:
lambda_http::Request → lambda_http::Response

// Framework expects:
hyper::Request<hyper::body::Incoming> → hyper::Response<UnifiedMcpBody>
```

### 2. Handler Registration Gap
- `McpServer` builds handlers internally but doesn't expose them
- `JsonRpcDispatcher` needs handlers registered
- No bridge exists between `McpServer` and `JsonRpcDispatcher`

### 3. Middleware Differences
- Framework uses Tower middleware in HTTP server
- Lambda needs CORS headers injected directly into responses
- SSE streaming requires different handling through Lambda's streaming response

### 4. Session Management
- Framework's `SessionMcpHandler` handles session creation/management
- Lambda needs session persistence across invocations (DynamoDB)
- Cold starts require efficient session restoration

## Decision

Create a dedicated **`turul-mcp-aws-lambda`** crate that provides Lambda-specific integration components.

### Solution Architecture

#### 1. Type Conversion Layer (`adapter.rs`)
```rust
pub async fn lambda_to_hyper(req: lambda_http::Request) -> Result<hyper::Request<Incoming>>
pub async fn hyper_to_lambda(resp: hyper::Response<UnifiedMcpBody>) -> Result<lambda_http::Response>
```

#### 2. Lambda MCP Handler (`handler.rs`)
```rust
pub struct LambdaMcpHandler {
    session_handler: Arc<SessionMcpHandler>,
    dispatcher: Arc<JsonRpcDispatcher>,
    cors_config: CorsConfig,
}

impl LambdaMcpHandler {
    pub fn register_tool(&mut self, tool: Arc<dyn McpTool>)
    pub async fn handle(&self, req: lambda_http::Request) -> Result<lambda_http::Response>
}
```

#### 3. CORS Middleware (`cors.rs`)
```rust
pub fn inject_cors_headers(response: &mut lambda_http::Response, config: &CorsConfig)
```

#### 4. SSE Stream Utilities (`streaming.rs`)
```rust
// Note: adapt_sse_stream was removed in 0.2.0 - use handle_streaming() for real streaming
pub fn format_sse_event(data: &str, event_type: Option<&str>, event_id: Option<&str>) -> String
pub fn create_sse_stream<T>(...) -> impl Stream<Item = Result<Bytes>>
```

#### 5. Builder API (`builder.rs`)
```rust
pub struct LambdaMcpServerBuilder {
    // Provides fluent API similar to McpServer::builder()
}
```

## Rationale

### Why a New Crate?

1. **Clean Separation**: Lambda-specific concerns isolated from core framework
2. **Reusability**: Other Lambda MCP servers can use this crate  
3. **Framework Integrity**: Core framework remains cloud-agnostic
4. **Type Safety**: Proper conversion with comprehensive error handling
5. **Best Practices**: Lambda-specific optimizations (cold starts, memory usage)

### Why Not Modify the Core Framework?

1. **Single Responsibility**: Core framework shouldn't know about Lambda specifics
2. **Platform Agnostic**: Framework should work with any HTTP transport
3. **Complexity**: Adding Lambda logic to core would increase complexity
4. **Future Extensibility**: Enables other cloud provider integrations

## Implementation Plan

### Phase 1: Core Components
1. Create crate structure with proper dependencies
2. Implement type conversion functions
3. Build LambdaMcpHandler with handler registration
4. Add CORS and SSE adaptation

### Phase 2: Builder API  
1. Create fluent builder similar to McpServer
2. Enable tool registration directly with dispatcher
3. Support storage backend configuration

### Phase 3: Example Updates
1. Update lambda-mcp-server to use new crate
2. Simplify main.rs to ~20 lines of code
3. Remove custom adapter implementation

### Phase 4: Testing & Documentation
1. Unit tests for type conversions
2. Integration tests with Lambda runtime
3. Comprehensive documentation and examples

## Consequences

### Positive
- **Clean Lambda Integration**: First-class Lambda support with minimal code
- **Type Safety**: Proper error handling for all conversions  
- **Performance**: Optimized for Lambda execution model
- **Reusability**: Standard pattern for Lambda MCP servers
- **Maintainability**: Clear separation of concerns

### Negative
- **Additional Dependency**: Users need to import Lambda-specific crate
- **Duplication**: Some functionality duplicated from core framework
- **Complexity**: More crates to understand and maintain

### Neutral
- **Framework Usage**: Core framework patterns remain unchanged
- **Migration Path**: Existing servers can adopt gradually

## Related Decisions

- **ADR-JsonSchema-Standardization**: Ensures consistent schema handling across Lambda integration
- **ADR-SessionContext-Macro-Support**: Enables full MCP features in Lambda-deployed tools

## Notes

This ADR resolves the circular development issues we encountered during lambda-mcp-server development. The architectural discovery documented here prevents future confusion about framework integration patterns.

The solution enables Lambda deployment while maintaining the framework's design principles and zero-configuration approach.
## DRAFT-2026-v1: stateless mode

**Status: Added 2026-05-31. Relevant when the Lambda binary is built with default (DRAFT-2026-v1) protocol per ADR-027.**

### What stays the same

- **Type conversion layer** (`lambda_http::Request` ↔ `hyper::Request<Incoming>`) is unchanged. Lambda's HTTP envelope is independent of the MCP protocol version.
- **CORS middleware** (`inject_cors_headers`) is unchanged. CORS lives at the HTTP boundary, not the MCP boundary.
- **Streaming response wrapper** (`into_lambda_stream_response`, `EnsureOneFrame` per ADR-026) is unchanged. The streaming envelope is the AWS Runtime API contract, not an MCP concept.
- **Builder API** (`LambdaMcpServerBuilder`) is unchanged in shape; tools/resources/prompts registration is identical.

### What changes

- **No session restore on cold start.** The 2025-11-25 cold-start path reads any in-flight session record from DynamoDB to attach the incoming request to its persisted state. DRAFT-2026-v1 has no session record to restore — every invocation is independent. Cold starts become cheaper: no `get_session` DynamoDB read on the request path, no `mcp:tool_fingerprint` read-before-write, no `notifications/initialized` 202 short-circuit.
- **No session storage backend required.** The `with_session_storage(DynamoDbSessionStorage::new(...))` builder call is a 2025-only configuration. In default 0.4.0 (DRAFT-2026-v1), session storage is unused; the builder method either compile-gates behind `legacy-2025-11-25` or accepts the configuration silently as a no-op.
- **Task storage is decoupled from session storage.** Tasks in DRAFT-2026-v1 are an SEP-2663 extension (see ADR-028). `turul-mcp-ext-tasks-2026-07-28` (when scaffolded) carries its own storage abstraction; the existing `turul-mcp-task-storage` crate continues to provide the durable backend implementations but its API surface integrates with the extension crate rather than with the session lifecycle.
- **`server/discover` instead of `initialize`.** The dispatcher routes `server/discover` to a per-request capability synthesis instead of a session-creating handler. No state is mutated; the response is computed from the current `LambdaMcpServerBuilder` registration.
- **No `notifications/initialized` 202 response.** The dispatcher rejects this notification with `-32601 Method Not Found` (it's not a 2026 method).
- **No GET SSE handler.** Per ADR-006 amendment, GET SSE is 2025-only. In 2026 mode, GET `/mcp` returns 405. The existing Lambda streaming limitation for server-initiated notifications (background `tokio::spawn` killed at invocation completion) is moot — those notifications don't exist in the 2026 wire model.

### Cold-start sequencing (DRAFT-2026-v1)

With session restore removed, the cold-start request path is:

1. **`lambda_runtime::handler` entry** — type conversion: `lambda_http::Request` → `hyper::Request<Incoming>`.
2. **CORS check** — middleware as before.
3. **OAuth check** (if configured) — middleware as before.
4. **Dispatcher** — method dispatch directly into the registered tool/resource/prompt handlers. No session lookup, no fingerprint check, no `initialize` short-circuit.
5. **Tool change detection** (if `Dynamic` mode + `ServerStateStorage` configured) — per ADR-023 amendment, request-time `check_for_changes()` with TTL gating against the server-global fingerprint in `ServerStateStorage`. Notification, if any, is included in the response `_meta` rather than persisted to a per-session event log.
6. **Response** — buffered (200/204) or streamed (tool call with progress).

This is simpler than the 2025-11-25 sequence (which carries `validate_session_exists()`, fingerprint compare against per-session state, `notifications/tools/list_changed` persistence) and faster on cold start (one fewer DynamoDB read per invocation).

### What downstream consumers need to change

- Drop the `DynamoDbSessionStorage` configuration step from the builder if running in default 0.4.0 (DRAFT-2026-v1). It is unused. Keep it if running under `--features legacy-2025-11-25`.
- Drop any reliance on persisted session state across invocations. If session-equivalent state is needed (e.g., conversational memory, per-user history), it must be re-architected as either (a) an extension under SEP-2133, (b) explicit per-request `_meta` carriage, or (c) external state outside the MCP envelope.
- Plan for the eventual `turul-mcp-ext-tasks-2026-07-28` extension crate (per ADR-028) for any tasks-based workflow. The current `turul-mcp-task-storage` backends (SQLite, PostgreSQL, DynamoDB) remain — they will be the storage layer behind the extension crate's API.

### References

- ADR-006 §"DRAFT-2026-v1: Stateless variant; GET SSE is 2025-only" — transport behavior.
- ADR-023 §"DRAFT-2026-v1: per-request fingerprint persistence" — tool change detection.
- ADR-027 §"Status update (2026-05-31)" — feature flag mechanics, publication gate.
- ADR-028 — tasks/apps as separate extension crates.
