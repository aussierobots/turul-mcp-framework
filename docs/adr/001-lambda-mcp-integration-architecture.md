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