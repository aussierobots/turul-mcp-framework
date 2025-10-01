# ADR-011: AWS Lambda Streaming Incompatibility (Superseded)

**Status**: Superseded (2025-10-02)  
**Date**: 2025-10-01  
**Authors**: Claude Code via turul-mcp-framework development

## Status Update (2025-10-02)

**This ADR is superseded.** The Lambda environment detection approach described here was implemented and then abandoned after it broke working tool call functionality.

**Current State**: See [ADR-006: Lambda Streaming Limitation](006-streamable-http-compatibility.md#lambda-streaming-limitation-updated-2025-10-02) for the current documented approach.

**Summary of What Happened**:
1. Lambda notification delivery issue identified (notifications not reaching clients)
2. Implemented environment detection to route Lambda → SessionMcpHandler
3. This broke tool calls, causing -32001 timeout errors
4. Reverted all Lambda detection code
5. Accepted notifications as known limitation, tool calls work correctly

**Current Approach**:
- Simple protocol-version-based routing (no environment detection)
- Tool calls work correctly in Lambda via StreamableHttpHandler
- Server-initiated notifications documented as known limitation in Lambda
- Framework works correctly in both Lambda and non-Lambda environments

---

## Original Context (Historical)

In version 0.2.1, the framework introduced protocol version-based routing (commit `cb4ad8943b21b6a638892bb6bc67eea3ee9c5af5`) that routes all clients with MCP protocol ≥ 2025-03-26 through `StreamableHttpHandler`. This broke notification delivery in AWS Lambda deployments, despite the same code working correctly in local/container environments.

### The Problem

**Symptom**: MCP Inspector shows "No notifications yet" despite server code calling `session.notify_progress()`. Progress notifications never reach clients in Lambda deployments.

**Initial Misdiagnosis**: Appeared to be a race condition where shutdown signal arrived before notifications could be forwarded.

**Actual Root Cause**: Fundamental architectural incompatibility between AWS Lambda's execution model and `StreamableHttpHandler`'s design.

## Lambda Execution Model

AWS Lambda's execution model:
1. Handler function is invoked with request
2. Handler processes request and returns response
3. **Lambda immediately tears down invocation context**
4. Any spawned tasks are killed
5. No opportunity for background tasks to complete

This is fundamentally different from long-running server processes where spawned tasks continue executing until explicitly stopped.

## StreamableHttpHandler Architecture

`StreamableHttpHandler` uses background tasks for notification delivery:

```rust
// StreamableHttpHandler spawns task to forward notifications
tokio::spawn(async move {
    while let Ok(notification) = progress_rx.recv().await {
        // Forward notification to SSE stream
        // ...
    }
});

// Handler returns response immediately
// Lambda kills spawned task before it can forward notifications ❌
```

**Why This Fails in Lambda**:
1. Tool execution calls `session.notify_progress()` → sends to `progress_tx`
2. Background task receives from `progress_rx` → ready to forward
3. Handler function returns with partial response
4. **Lambda terminates invocation → kills background task**
5. Notifications never reach client stream

**Why This Works in Non-Lambda**:
- Server process continues running
- Background tasks complete their work
- Notifications successfully forwarded

## SessionMcpHandler Architecture

`SessionMcpHandler` uses buffered responses without background tasks:

```rust
// SessionMcpHandler collects all notifications synchronously
let result = handler.dispatch(method, params, session).await;

// All notifications collected before response returns
// No background tasks → Lambda compatible ✅
```

## Attempted Solution (Failed)

### Detection Strategy

Attempted to detect AWS Lambda environment via standard environment variables:

```rust
fn is_lambda_environment() -> bool {
    if std::env::var("TURUL_FORCE_STREAMING").is_ok() {
        return false;
    }
    
    std::env::var("AWS_EXECUTION_ENV").is_ok()
        || std::env::var("AWS_LAMBDA_FUNCTION_NAME").is_ok()
        || std::env::var("LAMBDA_TASK_ROOT").is_ok()
}
```

### Routing Logic

```rust
let in_lambda = is_lambda_environment();
let use_streaming = protocol_version.supports_streamable_http() && !in_lambda;

let hyper_resp = if use_streaming {
    self.streamable_handler.handle_request(hyper_req).await
} else if protocol_version.supports_streamable_http() && in_lambda {
    debug!("⚠️  Lambda environment detected: using SessionMcpHandler");
    self.session_handler.handle_mcp_request(hyper_req).await?
} else {
    self.session_handler.handle_mcp_request(hyper_req).await?
};
```

### Why It Failed

**Problem**: Routing to SessionMcpHandler broke tool calls, causing -32001 timeout errors.

**Root Cause**: SessionMcpHandler apparently has issues handling tool calls correctly in Lambda, while StreamableHttpHandler works fine for synchronous operations.

**Key Insight**: Tool calls complete synchronously within Lambda's handler lifetime, so StreamableHttpHandler works correctly. Only server-initiated notifications (which require background tasks) fail.

## Final Resolution (Accepted Approach)

### Decision

**Accept notifications as a known limitation in Lambda.** Remove all environment detection code and use simple protocol-version-based routing everywhere:

```rust
// Simple routing - no environment detection
let hyper_resp = if protocol_version.supports_streamable_http() {
    self.streamable_handler.handle_request(hyper_req).await
} else {
    self.session_handler.handle_mcp_request(hyper_req).await?
};
```

### What Works

- ✅ Tool calls execute correctly via StreamableHttpHandler
- ✅ Synchronous request/response operations
- ✅ All MCP protocol operations except server-initiated notifications

### What Doesn't Work

- ❌ Server-initiated progress notifications (`session.notify_progress()`)
- ❌ Background task-based notification delivery

### Why This Is Acceptable

1. **Tool calls work** - the primary MCP functionality works correctly
2. **Simpler architecture** - no environment detection complexity
3. **Predictable behavior** - same code path everywhere
4. **Clear documentation** - users understand the limitation
5. **Alternative solutions exist** - Fargate, ECS, EC2, Cloud Run for notification needs

## Lessons Learned

1. **Don't break working functionality to fix non-working functionality**
2. **Environment-specific routing adds complexity and fragility**
3. **Accept limitations when fixes break more than they fix**
4. **Simple solutions are often better than complex "fixes"**

## Alternative Approaches

For deployments requiring server-initiated notifications:

- **AWS Fargate**: Container-based, supports long-running processes
- **ECS**: Full container orchestration with persistent connections
- **EC2**: Traditional server deployment with complete streaming support
- **Cloud Run (GCP)**: Container platform with streaming response support
- **Standard HTTP Server**: Any long-running server process (not serverless)

## Conclusion

Lambda's execution model is fundamentally incompatible with background task-based notification delivery. The attempted fix (environment detection + handler routing) broke working tool call functionality. The accepted solution is to use StreamableHttpHandler everywhere and document notifications as a known limitation in Lambda.

This maintains framework integrity with one simple code path while clearly communicating limitations to users.

## References

- [ADR-006: Streamable HTTP Compatibility](006-streamable-http-compatibility.md)
- [MCP 2025-06-18 Specification](https://spec.modelcontextprotocol.io/specification/2025-06-18/)
- [AWS Lambda Execution Model](https://docs.aws.amazon.com/lambda/latest/dg/lambda-runtime-environment.html)
- Commit `cb4ad8943b21b6a638892bb6bc67eea3ee9c5af5` - Protocol version routing introduction
