# ADR 012: Middleware Architecture

**Status:** Accepted
**Date:** 2025-01-05
**Authors:** Architecture Team
**Context:** MCP server framework needs extensible request/response interception

---

## Context

The turul-mcp-framework requires a mechanism for cross-cutting concerns like authentication, logging, rate limiting, and session state management. These concerns need to:

1. Execute before and after request dispatch
2. Read and write session state
3. Short-circuit requests with errors
4. Work identically across all transports (HTTP, Lambda, etc.)
5. Compose multiple middleware components

## Decision

We implement a **trait-based middleware architecture** with:

### Core Components

1. **`McpMiddleware` Trait** - Core abstraction for middleware
   ```rust
   #[async_trait]
   pub trait McpMiddleware: Send + Sync {
       async fn before_dispatch(
           &self,
           ctx: &mut RequestContext<'_>,
           session: Option<&dyn SessionView>,
           injection: &mut SessionInjection,
       ) -> Result<(), MiddlewareError>;

       async fn after_dispatch(
           &self,
           ctx: &RequestContext<'_>,
           result: &mut DispatcherResult,
       ) -> Result<(), MiddlewareError> {
           Ok(())
       }
   }
   ```

2. **`RequestContext`** - Normalized request context across transports
   - Provides `method()`, `params()`, and request ID
   - Transport-agnostic abstraction

3. **`SessionInjection`** - Write-only mechanism for populating session data
   - `set_state(key, value)` - Write to session state
   - `set_metadata(key, value)` - Write to session metadata
   - Applied **immediately** after `before_dispatch()` returns
   - Prevents middleware from reading its own writes (enforces layering)

4. **`SessionView`** - Read-only view of session data for middleware
   - `get_state(key)` - Read session state
   - `get_metadata(key)` - Read session metadata
   - Implemented by `StorageBackedSessionView` adapter

5. **`MiddlewareStack`** - Ordered execution of middleware layers
   - Middleware execute in FIFO order for `before_dispatch()`
   - Middleware execute in LIFO order for `after_dispatch()` (reverse)
   - Short-circuits on first error

### Session State Conventions

**Metadata Prefix:** `__meta__:`

Metadata is stored with a `__meta__:` prefix in the underlying session storage to avoid collisions with user state:

```rust
// Middleware writes:
injection.set_metadata("user_id", json!("user123"));

// Stored in session.metadata as:
"__meta__:user_id" → "user123"

// Middleware reads:
session.get_metadata("user_id") // Returns Some(json!("user123"))
```

**Why?**
- Prevents namespace collisions between middleware metadata and user state
- Follows SessionContext pattern from turul-mcp-server
- Keeps session storage implementation simple (flat HashMap)

### Transport Parity

Both HTTP and Lambda transports use the **same middleware infrastructure**:

```rust
// HTTP: StreamableHttpHandler and SessionMcpHandler
fn run_middleware_and_dispatch(...) -> (JsonRpcResponse, bool) {
    let session_view = StorageBackedSessionView::new(session_id, storage);

    // Execute middleware
    let injection = middleware_stack.execute_before(&mut ctx, Some(&session_view)).await?;

    // Apply injection immediately
    for (key, value) in injection.state() {
        session_view.set_state(key, value).await?;
    }

    // Dispatch request
    let result = dispatcher.dispatch(...).await;

    // Execute after middleware
    middleware_stack.execute_after(&ctx, &mut result).await?;
}

// Lambda: LambdaMcpHandler delegates to same helpers
// Uses with_middleware() constructor to inject middleware stack
```

**Key Invariant:** Middleware behavior is **identical** across all transports.

### Lambda Handler Lifecycle Requirements

Lambda requires handler caching to maintain transport parity:

```rust
use tokio::sync::OnceCell;

// Global handler instance - created once per Lambda container
static HANDLER: OnceCell<LambdaMcpHandler> = OnceCell::const_new();

async fn lambda_handler(request: Request) -> Result<Response<Body>, Error> {
    let handler = HANDLER
        .get_or_try_init(|| async { create_lambda_mcp_handler().await })
        .await?;
    handler.handle(request).await
}
```

**Critical Requirement:** Lambda containers may handle multiple sequential requests. Without handler caching:
- ❌ Each request creates new DynamoDB client (connection state lost)
- ❌ Each request creates new StreamManager (in-memory SSE tracking lost)
- ❌ Each request creates new middleware instances (middleware state lost)
- ❌ Middleware can't access session data stored in DynamoDB (client recreated)

**What Gets Lost:** Handler infrastructure (DynamoDB client, StreamManager, middleware instances)
**What Persists:** Session data in DynamoDB (stored correctly, but handler can't access efficiently)

**With Caching:**
- ✅ Same DynamoDB client across container lifetime
- ✅ Same StreamManager tracks SSE connections in memory
- ✅ Same middleware instances maintain any internal state
- ✅ Handler can efficiently access session data in DynamoDB
- ✅ Transport parity with long-running HTTP servers

**Pattern:** All Lambda middleware examples MUST use `tokio::sync::OnceCell` or equivalent for handler caching.

**Example:** See `examples/middleware-auth-lambda/src/main.rs` for reference implementation.

### Error Mapping

Middleware errors are mapped to semantic JSON-RPC error codes:

```rust
// crates/turul-http-mcp-server/src/middleware/error.rs — the sole owner.
pub fn map_middleware_error_to_jsonrpc(
    err: MiddlewareError,
    request_id: RequestId,
) -> JsonRpcResponse {
    let error_obj = match err {
        // Server-range codes: server_error's -32099..=-32000 assert admits these.
        MiddlewareError::Unauthenticated(msg) =>
            JsonRpcErrorObject::server_error(error_codes::UNAUTHENTICATED, &msg, None),
        MiddlewareError::Unauthorized(msg) =>
            JsonRpcErrorObject::server_error(error_codes::UNAUTHORIZED, &msg, None),
        MiddlewareError::RateLimitExceeded { message, retry_after } =>
            JsonRpcErrorObject::server_error(
                error_codes::RATE_LIMIT_EXCEEDED,
                &message,
                retry_after.map(|s| json!({ "retryAfter": s })),
            ),
        // Standard JSON-RPC codes: their own constructors, not server_error.
        MiddlewareError::InvalidRequest(msg) =>
            JsonRpcErrorObject::invalid_request(Some(json!({ "reason": msg }))),
        MiddlewareError::Internal(msg) => JsonRpcErrorObject::internal_error(Some(msg)),
        MiddlewareError::Custom { message, .. } => JsonRpcErrorObject::internal_error(Some(message)),
        // Answered as a raw HTTP challenge before JSON-RPC dispatch is reached,
        // so this arm is unreachable rather than mapped.
        MiddlewareError::HttpChallenge { .. } => unreachable!(),
    };
    JsonRpcResponse::Error(JsonRpcError::new(Some(request_id), error_obj))
}
```

**Semantic Error Codes:**
- `-32001` - Unauthenticated (missing credentials)
- `-32005` - Unauthorized (invalid permissions)
- `-32003` - RateLimitExceeded (with `retryAfter` in data)
- `-32600` - InvalidRequest (malformed request)
- `-32603` - InternalError (session storage failure, `Internal`, `Custom`)

`HttpChallenge` has no JSON-RPC code: OAuth 2.1 requires a `401` with a
`WWW-Authenticate` header, so it short-circuits to a raw HTTP response before
dispatch. `Unauthorized` keeps the JSON-RPC form because a request that is
well-formed but fails during processing is answered `200` with the error in the
body — the two are different layers, not an inconsistency.

**The constructor is chosen by code class.** `JsonRpcErrorObject::server_error`
asserts the code lies in the implementation-defined `-32099..=-32000`, so it
serves the three server-range codes only. `-32600` and `-32603` are standard
JSON-RPC codes with their own constructors (`invalid_request`, `internal_error`)
and use those. Routing all six through `server_error` tripped the assert, which
meant `InvalidRequest`, `Internal` and `Custom` aborted the request rather than
answering it; every variant now produces a response.

**One owner.** The mapping lives in `middleware/error.rs` beside the enum and the
codes it maps to. Both transports call it, so the code a client receives cannot
depend on which handler served the request — it was previously duplicated
verbatim in `session_handler.rs` and `streamable_http.rs`.

### Builder Integration

Middleware is registered via the builder's `.middleware()` method:

```rust
let server = McpServer::builder()
    .name("my-server")
    .middleware(Arc::new(AuthMiddleware))      // 1st before, 3rd after
    .middleware(Arc::new(LoggingMiddleware))   // 2nd before, 2nd after
    .middleware(Arc::new(RateLimitMiddleware)) // 3rd before, 1st after
    .build()?;
```

**Key Properties:**
- **Additive** - Each `.middleware()` call adds to the stack (no reset)
- **FIFO before, LIFO after** - Execution order is documented
- **Transport-agnostic** - Works identically in HTTP and Lambda
- **No interaction with `.test_mode()`** - Middleware always executes

## Alternatives Considered

### 1. Axum-style Layer Architecture
**Why not:** Axum's `Layer` trait is tightly coupled to HTTP tower services. We need transport-agnostic middleware that works in Lambda, HTTP, and future transports.

### 2. SessionContext as Middleware Parameter
**Why not:** This would require middleware to have direct write access to SessionContext, breaking the clean separation between middleware and session storage. The `SessionInjection` write-only pattern enforces better layering.

### 3. Middleware Reads Own Writes
**Why not:** Allowing middleware to read its own writes creates ordering dependencies and makes testing harder. The injection pattern enforces that middleware can only read existing session state.

### 4. Per-Transport Middleware
**Why not:** Would create maintenance burden and confuse users about transport parity. All transports delegate to the same `run_middleware_and_dispatch` helper.

## Consequences

### Positive

✅ **Transport Parity** - Same middleware works in HTTP, Lambda, and future transports
✅ **Clean Separation** - Middleware doesn't know about storage implementation
✅ **Easy Testing** - Middleware can be tested in isolation via `execute_before()`
✅ **Composable** - Multiple middleware layers combine naturally
✅ **Type-Safe** - Rust's type system prevents common errors
✅ **Semantic Errors** - Clear error codes for client handling

### Negative

⚠️ **Learning Curve** - Users need to understand `SessionView` vs `SessionInjection`
⚠️ **Immediate Application** - Injection is applied immediately (no deferred writes)
⚠️ **No Async After** - `after_dispatch()` can't perform async writes (by design)
⚠️ **Lambda Handler Caching Required** - Lambda implementations MUST cache the handler globally (via `OnceCell` or equivalent) to preserve handler infrastructure (DynamoDB client, StreamManager, middleware instances). Failure to do so breaks middleware's ability to access session data and transport parity.

### Migration

**From Previous Approach:** None - this is the initial middleware implementation.

**Future:** When adding new transports, they MUST use `run_middleware_and_dispatch` pattern to ensure parity.

## Testing Strategy

1. **Unit Tests** - `MiddlewareStack` execution order
2. **Integration Tests** - Middleware can read/write session state
3. **Handler Tests** - Both HTTP handlers use `run_middleware_and_dispatch` pattern
4. **Lambda Parity Tests** - Lambda transport has same middleware behavior
5. **Error Mapping Tests** - Middleware errors map to correct JSON-RPC codes

## Examples

See framework examples:
- `middleware-auth-server` - API key authentication (HTTP)
- `middleware-logging-server` - Request timing and tracing (HTTP)
- `middleware-rate-limit-server` - Rate limiting with `retryAfter` (HTTP)
- `middleware-auth-lambda` - Lambda authentication with handler caching (proves transport parity)

**Critical Pattern:** The `middleware-auth-lambda` example demonstrates the required `OnceCell` handler caching pattern. This is MANDATORY for all Lambda middleware implementations to maintain session state across requests.

## References

- MCP Specification 2025-06-18
- [ADR 001: Session Storage Architecture](001-session-storage-architecture.md)
- [ADR 009: Protocol-Based Handler Routing](009-protocol-based-handler-routing.md)
- `crates/turul-http-mcp-server/src/middleware/` - Implementation
- `crates/turul-http-mcp-server/src/tests/middleware_tests.rs` - Tests

## Revision Log

| Date | Change |
|---|---|
| 2026-07-30 | `Unauthorized` remapped `-32002` → `-32005`. 2026-07-28 states implementations of that version MUST NOT emit `-32002`, which it reassigns to resource-not-found. Recorded the `HttpChallenge` variant, which predated this ADR's error table, and corrected `SessionError` to the actual `Internal`/`Custom` variants. |
| 2026-07-30 | Recorded that `InvalidRequest`, `Internal` and `Custom` do not reach the wire: their `-32600`/`-32603` codes fall outside the `-32099..=-32000` range `JsonRpcErrorObject::server_error` asserts, so the mapping panics rather than returning. The error table had documented all six arms as if they were served. Verified by calling `server_error` with each of the five codes the mapping produces. Debt named in place with an owner and removal trigger rather than corrected here — the fix is a code change in this crate or in the sibling `turul-rpc`, and this revision is documentation-only. |
| 2026-07-30 | **Debt above discharged, no `turul-rpc` change needed.** `server_error` was simply the wrong constructor for standard JSON-RPC codes: `turul-rpc` already exposes `invalid_request` and `internal_error`, which carry no range assert. The three affected arms now use them, so every variant a middleware can return produces a response. `Custom` reports `-32603` and its application-level `code` string does not reach the wire — the error table said it did. The mapping also moved out of the two transports, which held verbatim copies, into `middleware/error.rs` beside the enum and the codes, giving it one owner. Guarded by `every_returnable_variant_maps_to_a_response_without_panicking` (all six variants, code asserted per variant) and `rate_limit_carries_retry_after_but_only_when_given`. Revert-and-fail: restoring `server_error` for `InvalidRequest` panics that test on `turul-rpc-core-0.2.3/src/error.rs:96`. |
