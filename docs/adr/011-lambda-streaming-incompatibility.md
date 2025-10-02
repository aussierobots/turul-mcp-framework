**Status**: Active  
**Date**: 2025-10-01 (Updated: 2025-10-02)  
**Authors**: Claude Code via turul-mcp-framework development

## Status Update (2025-10-02)

**This ADR documents the Lambda streaming limitation and investigation approach.** The Lambda environment detection approach was implemented and then abandoned after it broke working tool call functionality.

**Current State**: Simple protocol-version-based routing. Tool calls work correctly via StreamableHttpHandler. Server-initiated notifications are a documented limitation in Lambda environments.

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

---

## Lambda POST Streamable HTTP Investigation

### ⚠️ Current Status: UNVERIFIED

**Important**: No empirical testing has been conducted with `cargo lambda watch` to verify whether POST Streamable HTTP actually works or not in Lambda environments. The concerns below are THEORETICAL based on code inspection.

### Theoretical Analysis

**Concern 1: Tests Are Compilation Checks Only**

File: `tests/lambda_examples.rs` (lines 103-302)

Current test pattern does not execute handlers:
```rust
#[test]
fn test_lambda_streaming_feature_e2e() {
    async fn example_lambda_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let _server = LambdaMcpServerBuilder::new()
            .tool(StreamTestTool::default())
            .sse(true)
            .build()
            .await?;
        Ok(())
    }
    
    let _ = example_lambda_server; // Just assigns to _ - NEVER RUNS!
}
```

**What's Missing**:
- No actual HTTP POST request to Lambda handler
- No verification that `handle_streaming()` is called
- No check that chunked response frames are delivered
- No validation that tool progress notifications reach clients

**Concern 2: Background Task Lifecycle**

Current implementation spawns background tasks:
```rust
// StreamableHttpHandler spawns task to forward notifications
tokio::spawn(async move {
    while let Ok(notification) = progress_rx.recv().await {
        // Forward notification to SSE stream
    }
});
```

**Theoretical Issue**: Lambda may tear down the invocation immediately after handler returns, potentially killing spawned tasks before notifications can be sent.

**Why Tool Calls Work**: Tool execution completes synchronously within the Lambda handler's lifetime, so the response is fully assembled before Lambda tears down.

**Concern 3: Lambda Streaming Pattern Usage**

Lambda's `lambda_http` crate provides `Body::from_stream()` pattern:
```rust
// Lambda recommended pattern
async fn handler(event: Request) -> Result<Response<Body>, Error> {
    let chunks = vec![
        Bytes::from("data: event1\n\n"),
        Bytes::from("data: event2\n\n"),
    ];
    let stream = stream::iter(chunks).map(Ok);
    Ok(Response::builder().body(Body::from_stream(stream))?)
}
```

Current implementation returns hyper types converted to lambda_http types, which may not align with the recommended pattern.

### Lambda Stream Lifecycle: How Streams Survive

**Critical Understanding**: When a Lambda handler returns a `lambda_http::Response<Body>` built from `Body::from_stream`, the Lambda runtime **keeps the invocation alive** until the stream completes or errors.

**What Works**:
- ✅ Pre-computed chunks: `stream::iter(chunks).map(Ok)`
- ✅ Generator streams (synchronous data generation)

**What Doesn't Work**:
- ❌ Background task dependencies (`tokio::spawn` killed on return)
- ❌ External channel dependencies (receiver streams without data source)

### Required Verification Work

**Phase 1: Empirical Testing (Week 1)**
- [ ] Complete test stubs in `tests/lambda_streaming_real.rs`
- [ ] Deploy to local Lambda with `cargo lambda watch`
- [ ] Send real POST requests with `Accept: text/event-stream`
- [ ] Capture actual chunked responses
- [ ] Measure notification delivery
- [ ] Document empirical results with logs

**Decision Gate**: If streaming works → document success and update tests. If broken → proceed to Phase 2.

**Phase 2: Implementation (If Needed) (Week 2)**
- ONLY proceed if Phase 1 shows empirical evidence of failure
- [ ] Implement `Body::from_stream()` pattern
- [ ] Buffer notifications synchronously before stream construction
- [ ] Test with `cargo lambda watch`
- [ ] Verify fix resolves measured problem

**Phase 3: Production Validation (If Needed) (Week 3)**
- ONLY proceed if Phase 2 implemented fixes
- [ ] Deploy to production Lambda
- [ ] Test with real MCP clients
- [ ] Measure performance metrics
- [ ] Document production behavior

### Reference Implementation (If Fixes Needed)

If empirical testing shows the current implementation needs fixing:

```rust
pub async fn handle_streaming(&self, req: LambdaRequest)
    -> Result<lambda_http::Response<lambda_http::Body>, Box<dyn Error>>
{
    // 1. Parse MCP request
    let mcp_request = parse_mcp_request(req).await?;
    
    // 2. Execute tool and collect ALL notifications synchronously
    let mut notifications = Vec::new();
    let result = self.execute_with_notifications(mcp_request, &mut notifications).await?;
    
    // 3. Convert notifications to SSE format chunks
    let chunks: Vec<Bytes> = notifications.into_iter()
        .map(|n| Bytes::from(format!("data: {}\n\n", serde_json::to_string(&n)?)))
        .chain(std::iter::once(Bytes::from(format!("data: {}\n\n", serde_json::to_string(&result)?))))
        .collect();
    
    // 4. Build self-contained stream from pre-collected chunks
    let stream = stream::iter(chunks).map(Ok);
    
    // 5. Return Lambda response with streaming body
    Ok(Response::builder()
        .status(200)
        .header("content-type", "text/event-stream")
        .body(Body::from_stream(stream))?)
}
```

**Trade-offs**:
- Memory pressure: All notifications buffered in Lambda memory
- Latency: First byte delayed until tool completes
- Timeout risk: Long-running tools must complete within timeout

### Architectural Divergence

**Long-Running Servers** (turul-http-mcp-server):
- ✅ Asynchronous notification forwarding
- ✅ Low latency (events sent as generated)
- ✅ Low memory (streaming)
- ❌ Requires persistent process

**Lambda** (turul-mcp-aws-lambda):
- ✅ Synchronous notification collection (if needed)
- ✅ Guaranteed delivery (all events buffered)
- ❌ Higher latency (buffered until completion)
- ❌ Memory constraints (buffer size limited)

### When to Use Each Approach

**Lambda Streaming** (if it works):
- ✅ Tools with moderate notification volume (<1000 events)
- ✅ Notifications generated quickly (<5 minutes)
- ✅ Memory footprint under Lambda limits

**Traditional Server**:
- ✅ High-volume notifications (>1000 events)
- ✅ Long-running tools (>5 minutes)
- ✅ Real-time streaming requirements (low latency)

**Alternative Platforms** (for notification requirements):
- AWS Fargate: Container-based with long-running processes
- ECS: Full container orchestration
- EC2: Traditional server deployment
- Cloud Run (GCP): Container platform with streaming support

## Lessons Learned

1. **Don't break working functionality to fix non-working functionality**
2. **Environment-specific routing adds complexity and fragility**
3. **Accept limitations when fixes break more than they fix**
4. **Simple solutions are often better than complex "fixes"**
5. **Empirical testing is required before architectural changes**

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
- [lambda_http Documentation](https://docs.rs/lambda_http/latest/lambda_http/)
- Commit `cb4ad8943b21b6a638892bb6bc67eea3ee9c5af5` - Protocol version routing introduction
