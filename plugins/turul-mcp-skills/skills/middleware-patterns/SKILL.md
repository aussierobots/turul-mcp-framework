---
name: middleware-patterns
description: >
  This skill should be used when the user asks about "middleware",
  "McpMiddleware", "before_dispatch", "after_dispatch", "RequestContext",
  "SessionInjection", "MiddlewareError", "rate limiting middleware",
  "auth middleware", "logging middleware", "middleware stack",
  "middleware execution order", "middleware error handling",
  "lambda auth middleware", or "DispatcherResult".
  Covers creating HTTP middleware for auth, rate limiting, logging,
  and Lambda authorizer extraction in the Turul MCP Framework (Rust).
---

# Middleware Patterns — Turul MCP Framework

Middleware intercepts MCP requests before/after dispatch for cross-cutting concerns: authentication, rate limiting, logging, and auditing. Middleware is transport-agnostic — the same `McpMiddleware` trait works across HTTP and Lambda.

## When to Use Middleware

```
Where does this logic belong?
├─ Cross-cutting concern (auth, rate-limit, logging, audit) ──→ Middleware
└─ Business logic (tool/resource/prompt behavior) ────────────→ Handler
```

**Middleware is for concerns that apply to ALL or MOST requests**, not for per-tool logic.

## The McpMiddleware Trait

```rust
// turul-mcp-server v0.3
use turul_http_mcp_server::middleware::{
    McpMiddleware, RequestContext, SessionInjection, MiddlewareError, DispatcherResult,
};
use turul_mcp_session_storage::SessionView;
use async_trait::async_trait;

struct MyMiddleware;

#[async_trait]
impl McpMiddleware for MyMiddleware {
    // REQUIRED — runs before the MCP handler
    async fn before_dispatch(
        &self,
        ctx: &mut RequestContext<'_>,     // method, params, metadata
        session: Option<&dyn SessionView>, // None for `initialize`
        injection: &mut SessionInjection,  // write-only session state injection
    ) -> Result<(), MiddlewareError> {
        Ok(())
    }

    // OPTIONAL — runs after the MCP handler (default: no-op)
    async fn after_dispatch(
        &self,
        ctx: &RequestContext<'_>,
        result: &mut DispatcherResult,
    ) -> Result<(), MiddlewareError> {
        Ok(())
    }
}
```

**Key types:**
- `RequestContext<'a>` — method name (`ctx.method()`), params (`ctx.params()`), transport metadata (`ctx.metadata()`)
- `SessionInjection` — write-only: `injection.set_state(key, value)`, `injection.set_metadata(key, value)`
- `SessionView` — read-only session access (None during `initialize`)
- `DispatcherResult` — `Success(Value)` or `Error(String)`

## Pattern 1: Auth Middleware

Validate an API key from transport metadata, skip `initialize`/`ping`, inject authenticated user state.

```rust
// turul-mcp-server v0.3
use turul_http_mcp_server::middleware::*;
use turul_mcp_session_storage::SessionView;
use async_trait::async_trait;

struct ApiKeyAuth {
    valid_key: String,
}

#[async_trait]
impl McpMiddleware for ApiKeyAuth {
    async fn before_dispatch(
        &self,
        ctx: &mut RequestContext<'_>,
        _session: Option<&dyn SessionView>,
        injection: &mut SessionInjection,
    ) -> Result<(), MiddlewareError> {
        // Skip auth for initialize and ping (session doesn't exist yet)
        if ctx.method() == "initialize" || ctx.method() == "ping" {
            return Ok(());
        }

        let key = ctx.metadata()
            .get("x-api-key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MiddlewareError::unauthenticated("Missing x-api-key header"))?;

        if key != self.valid_key {
            return Err(MiddlewareError::unauthorized("Invalid API key"));
        }

        // Inject authenticated state — tools can read via session.get_typed_state("user_id")
        injection.set_state("user_id", serde_json::json!("authenticated-user"));
        Ok(())
    }
}
```

**See:** `examples/auth-middleware.rs` for a complete example.

## Pattern 2: Rate Limiting

Per-session request counters with configurable limits and `retry_after`.

```rust
// turul-mcp-server v0.3
use turul_http_mcp_server::middleware::*;
use turul_mcp_session_storage::SessionView;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Mutex;

struct RateLimitMiddleware {
    max_requests: u64,
    window_seconds: u64,
    counters: Mutex<HashMap<String, (u64, std::time::Instant)>>,
}

#[async_trait]
impl McpMiddleware for RateLimitMiddleware {
    async fn before_dispatch(
        &self,
        ctx: &mut RequestContext<'_>,
        session: Option<&dyn SessionView>,
        _injection: &mut SessionInjection,
    ) -> Result<(), MiddlewareError> {
        if ctx.method() == "initialize" {
            return Ok(());
        }

        let session_id = session
            .and_then(|s| s.session_id())
            .unwrap_or("anonymous")
            .to_string();

        let mut counters = self.counters.lock().unwrap(); // OK: no .await while held
        let now = std::time::Instant::now();

        let (count, window_start) = counters
            .entry(session_id)
            .or_insert((0, now));

        if now.duration_since(*window_start).as_secs() >= self.window_seconds {
            *count = 0;
            *window_start = now;
        }

        *count += 1;
        if *count > self.max_requests {
            return Err(MiddlewareError::rate_limit(
                "Too many requests",
                Some(self.window_seconds),
            ));
        }

        Ok(())
    }
}
```

**See:** `examples/rate-limit-middleware.rs` for a complete example.

## Pattern 3: Logging / Timing

Record request timing using `before_dispatch` and `after_dispatch`.

```rust
// turul-mcp-server v0.3
use turul_http_mcp_server::middleware::*;
use turul_mcp_session_storage::SessionView;
use async_trait::async_trait;
use std::sync::Mutex;

struct TimingMiddleware {
    start_times: Mutex<std::collections::HashMap<String, std::time::Instant>>,
}

#[async_trait]
impl McpMiddleware for TimingMiddleware {
    async fn before_dispatch(
        &self,
        ctx: &mut RequestContext<'_>,
        _session: Option<&dyn SessionView>,
        _injection: &mut SessionInjection,
    ) -> Result<(), MiddlewareError> {
        tracing::info!(method = %ctx.method(), "Request started");
        // Store start time keyed by method (simplified — real impl uses request ID)
        self.start_times.lock().unwrap()
            .insert(ctx.method().to_string(), std::time::Instant::now());
        Ok(())
    }

    async fn after_dispatch(
        &self,
        ctx: &RequestContext<'_>,
        result: &mut DispatcherResult,
    ) -> Result<(), MiddlewareError> {
        let elapsed = self.start_times.lock().unwrap()
            .remove(ctx.method())
            .map(|start| start.elapsed());

        tracing::info!(
            method = %ctx.method(),
            duration_ms = ?elapsed.map(|d| d.as_millis()),
            success = %result.is_success(),
            "Request completed"
        );
        Ok(())
    }
}
```

**See:** `examples/logging-middleware.rs` for a complete example.

## Pattern 4: Lambda Auth (API Gateway Authorizer)

Extract pre-validated identity from API Gateway authorizer headers.

```rust
// turul-mcp-server v0.3
use turul_http_mcp_server::middleware::*;
use turul_mcp_session_storage::SessionView;
use async_trait::async_trait;

struct LambdaAuthMiddleware;

#[async_trait]
impl McpMiddleware for LambdaAuthMiddleware {
    async fn before_dispatch(
        &self,
        ctx: &mut RequestContext<'_>,
        _session: Option<&dyn SessionView>,
        injection: &mut SessionInjection,
    ) -> Result<(), MiddlewareError> {
        if ctx.method() == "initialize" {
            return Ok(());
        }

        // API Gateway authorizer populates x-authorizer-* headers
        let user_id = ctx.metadata()
            .get("x-authorizer-principalid")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MiddlewareError::unauthenticated(
                "Missing authorizer principal — is the API Gateway authorizer configured?"
            ))?;

        injection.set_state("user_id", serde_json::json!(user_id));
        Ok(())
    }
}
```

**See:** `examples/lambda-auth-middleware.rs` for a complete example.

## Session Injection

`SessionInjection` is a **write-only** mechanism. Middleware writes state that tools read later.

```
Middleware:  injection.set_state("user_id", json!("alice"))
                       ↓  (deferred apply after all middleware succeed)
Tool:        session.get_typed_state::<String>("user_id").await  →  Some("alice")
```

- **Deferred**: Injections are applied to the session AFTER all before_dispatch middleware succeeds
- **Write-only**: Middleware cannot read from `SessionInjection` — use `session: Option<&dyn SessionView>` to read existing session state
- **Accumulative**: Multiple middleware can inject different keys; later middleware overrides earlier for the same key

## Registration and Execution Order

```rust
// turul-mcp-server v0.3
use std::sync::Arc;

let server = McpServer::builder()
    .name("my-server")
    .middleware(Arc::new(LoggingMiddleware))   // 1st before, 3rd after
    .middleware(Arc::new(AuthMiddleware))      // 2nd before, 2nd after
    .middleware(Arc::new(RateLimitMiddleware)) // 3rd before, 1st after
    .build()?;
```

- **Before dispatch**: Forward registration order (Logging → Auth → RateLimit)
- **After dispatch**: Reverse registration order (RateLimit → Auth → Logging)
- **Error short-circuits**: First error in `before_dispatch` stops the chain; remaining middleware do not execute

## Error Handling

Middleware returns `MiddlewareError` — the framework converts it through the standard chain:

```
MiddlewareError → McpError → JsonRpcError → HTTP/Lambda response
```

| Variant | JSON-RPC Code | When to Use |
|---|---|---|
| `Unauthenticated(msg)` | -32001 | No credentials provided |
| `Unauthorized(msg)` | -32002 | Credentials provided but insufficient |
| `RateLimitExceeded { message, retry_after }` | -32003 | Rate limit exceeded |
| `InvalidRequest(msg)` | -32600 | Malformed request |
| `Internal(msg)` | -32603 | Internal error (do not expose details to client) |
| `Custom { code, message }` | custom | Application-specific errors |

**Constructors:**
```rust
MiddlewareError::unauthenticated("Missing token")
MiddlewareError::unauthorized("Insufficient permissions")
MiddlewareError::rate_limit("Too many requests", Some(60))  // retry_after in seconds
MiddlewareError::invalid_request("Malformed params")
MiddlewareError::internal("Database connection lost")
MiddlewareError::custom("CUSTOM_ERR", "Something specific")
```

**See:** `references/middleware-error-guide.md` for the full error reference.

## Common Mistakes

1. **Forgetting to skip `initialize`** — Session is `None` during `initialize`. If your middleware requires a session, return `Ok(())` early for `ctx.method() == "initialize"`.

2. **Creating `JsonRpcError` directly** — Always return `MiddlewareError` variants. The framework handles conversion. See: [CLAUDE.md — Critical Error Handling Rules](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#critical-error-handling-rules)

3. **Confusing `Unauthenticated` vs `Unauthorized`** — `Unauthenticated` = no credentials at all (-32001). `Unauthorized` = credentials present but insufficient permissions (-32002).

4. **Holding `Mutex` across `.await`** — `std::sync::Mutex` is fine for quick in-memory operations (no `.await` while held). For async-heavy workloads, use `tokio::sync::Mutex` instead.

5. **Expecting `after_dispatch` to see injection state** — `SessionInjection` is write-only and applied after `before_dispatch`. In `after_dispatch`, use the `session` parameter (passed via `ctx`) or read the `DispatcherResult` directly.

6. **Forgetting `Arc::new()` when registering** — `.middleware()` takes `Arc<dyn McpMiddleware>`, not a bare instance.

## Beyond This Skill

**Error handling in tool/resource handlers?** → See the `error-handling-patterns` skill for `McpError` variants, decision tree, and error code mapping.

**Deploying middleware on Lambda?** → See the `lambda-deployment` skill for `LambdaMcpServerBuilder`, cold-start caching, CORS, and API Gateway authorizer integration.

**Creating tools, resources, or prompts?** → See the `tool-creation-patterns` or `resource-prompt-patterns` skill.

**OAuth / JWT authentication?** → See the `auth-patterns` skill for OAuth 2.1 RS, `JwtValidator`, audience validation, and RFC 9728 metadata.

**Client-side workflows?** → See the `mcp-client-patterns` skill.
