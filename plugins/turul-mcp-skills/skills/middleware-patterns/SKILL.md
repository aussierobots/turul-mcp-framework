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
  For OAuth/JWT-specific middleware (oauth_resource_server,
  JwtValidator, ProtectedResourceMetadata) see auth-patterns —
  this skill covers the McpMiddleware trait plumbing only.
---

**Spec lane: MCP 2026-07-28 (current default).** The `McpMiddleware` trait, `RequestContext`, `SessionInjection`, and `MiddlewareError` are framework plumbing and apply on both spec lanes. 2026-07-28's stateless core removes `initialize` and `ping` as protocol methods — there is no bootstrapping method left to special-case, so middleware runs uniformly on every request. On a 2025-11-25 build (`--no-default-features --features protocol-2025-11-25`), reinstate an `initialize`/`ping` skip if your middleware requires a session, since `session` is `None` during that handshake.

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
// turul-mcp-server v0.4
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
        session: Option<&dyn SessionView>, // see note below on 2026-07-28 semantics
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
- `SessionView` — read-only session access
- `DispatcherResult` — `Success(Value)` or `Error(String)`

**`session` on 2026-07-28 is not `None` — it's a fresh, throwaway session, every request.** The stateless core has no `Mcp-Session-Id` handshake, so `streamable_http.rs` mints an ephemeral per-request session internally to keep the dispatch pipeline (which still carries a `SessionContext`) unchanged; that id is never read from the client and never echoed back. Practical effect: `SessionInjection` state written in `before_dispatch` is visible to the tool handler *within that same request* (still useful for e.g. attaching auth identity for the handler to read), but nothing persists to a second request — there is no cross-request session identity to key on. Rate limiting or any pattern that needs to correlate requests from the same caller must key on something else (API key, bearer subject, client IP), not `session.session_id()`. See [Pattern 2](#pattern-2-rate-limiting) below.

## Pattern 1: Auth Middleware

Validate an API key from transport metadata, inject authenticated user state.

```rust
// turul-mcp-server v0.4
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

Per-caller request counters with configurable limits and `retry_after`. **Key on a stable caller identity from `ctx.metadata()` (API key, bearer subject), not `session.session_id()`** — on 2026-07-28, `session` is a fresh throwaway per request, so a session-keyed counter never accumulates past 1.

```rust
// turul-mcp-server v0.4
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
        _session: Option<&dyn SessionView>,
        _injection: &mut SessionInjection,
    ) -> Result<(), MiddlewareError> {
        let caller_id = ctx.metadata()
            .get("x-api-key")
            .and_then(|v| v.as_str())
            .unwrap_or("anonymous")
            .to_string();

        let mut counters = self.counters.lock().unwrap(); // OK: no .await while held
        let now = std::time::Instant::now();

        let (count, window_start) = counters
            .entry(caller_id)
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
// turul-mcp-server v0.4
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
// turul-mcp-server v0.4
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
        // API Gateway authorizer's custom context fields land as x-authorizer-*
        // headers. Use the field name your authorizer Lambda returns under
        // `context: {...}` (e.g. user_id, sub, account_id). `principalId` is
        // intentionally NOT forwarded — return your own identity field instead.
        let user_id = ctx.metadata()
            .get("x-authorizer-user_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MiddlewareError::unauthenticated(
                "Missing authorizer user_id — is the API Gateway authorizer returning it in context?"
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
// turul-mcp-server v0.4
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
| `Unauthorized(msg)` | -32005 | Credentials provided but insufficient |
| `RateLimitExceeded { message, retry_after }` | -32003 | Rate limit exceeded |
| `InvalidRequest(msg)` | -32600 | Malformed request — **panics today** |
| `Internal(msg)` | -32603 | Internal error — **panics today** |
| `Custom { code, message }` | -32603 | Application-specific — **panics today**; `code` is discarded |

`Unauthorized` is `-32005`: MCP 2026-07-28 reassigns `-32002` to
resource-not-found and forbids this version's implementations from emitting it.

The bottom three do not reach the wire. Middleware errors are built through
`JsonRpcErrorObject::server_error`, which asserts `-32099..=-32000`, so a
`-32600`/`-32603` code aborts the request rather than answering it. Use one of
the top three until that is fixed.

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

1. **Keying cross-request state (rate limits, caches) on `session.session_id()`** — on 2026-07-28 every request gets a fresh throwaway session, so a session-keyed counter never accumulates. Key on API key, bearer subject, or another caller-stable identifier from `ctx.metadata()` instead. (On a 2025-11-25 build, `session` is `None` during `initialize` — skip early for `ctx.method() == "initialize"` if your middleware requires a session.)

2. **Creating `JsonRpcError` directly** — Always return `MiddlewareError` variants. The framework handles conversion. See: [CLAUDE.md — Critical Error Handling Rules](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#critical-error-handling-rules)

3. **Confusing `Unauthenticated` vs `Unauthorized`** — `Unauthenticated` = no credentials at all (-32001). `Unauthorized` = credentials present but insufficient permissions (-32005).

4. **Holding `Mutex` across `.await`** — `std::sync::Mutex` is fine for quick in-memory operations (no `.await` while held). For async-heavy workloads, use `tokio::sync::Mutex` instead.

5. **Expecting `after_dispatch` to see injection state** — `SessionInjection` is write-only and applied after `before_dispatch`. In `after_dispatch`, use the `session` parameter (passed via `ctx`) or read the `DispatcherResult` directly.

6. **Forgetting `Arc::new()` when registering** — `.middleware()` takes `Arc<dyn McpMiddleware>`, not a bare instance.

## Beyond This Skill

**Error handling in tool/resource handlers?** → See the `error-handling-patterns` skill for `McpError` variants, decision tree, and error code mapping.

**Deploying middleware on Lambda?** → See the `lambda-deployment` skill for `LambdaMcpServerBuilder`, cold-start caching, CORS, and API Gateway authorizer integration.

**Creating tools, resources, or prompts?** → See the `tool-creation-patterns` or `resource-prompt-patterns` skill.

**OAuth / JWT authentication?** → See the `auth-patterns` skill for OAuth 2.1 RS, `JwtValidator`, audience validation, and RFC 9728 metadata.

**Client-side workflows?** → See the `mcp-client-patterns` skill.
