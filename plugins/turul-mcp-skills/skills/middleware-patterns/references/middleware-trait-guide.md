# McpMiddleware Trait Reference

Comprehensive reference for the `McpMiddleware` trait, `RequestContext`, `SessionInjection`, and `DispatcherResult`.

## Import

```rust
use turul_http_mcp_server::middleware::{
    McpMiddleware, RequestContext, SessionInjection, MiddlewareError, DispatcherResult,
};
use turul_mcp_session_storage::SessionView;
use async_trait::async_trait;
```

## McpMiddleware Trait

```rust
#[async_trait]
pub trait McpMiddleware: Send + Sync {
    /// REQUIRED — called before the MCP method handler
    async fn before_dispatch(
        &self,
        ctx: &mut RequestContext<'_>,
        session: Option<&dyn SessionView>,
        injection: &mut SessionInjection,
    ) -> Result<(), MiddlewareError>;

    /// OPTIONAL — called after the MCP method handler (default: no-op)
    async fn after_dispatch(
        &self,
        ctx: &RequestContext<'_>,
        result: &mut DispatcherResult,
    ) -> Result<(), MiddlewareError> {
        Ok(())
    }
}
```

### `before_dispatch` Parameters

| Parameter | Type | Description |
|---|---|---|
| `ctx` | `&mut RequestContext<'_>` | Mutable request context: method, params, metadata |
| `session` | `Option<&dyn SessionView>` | Read-only session view. `None` for `initialize` (session not created yet). `Some` for all other methods. |
| `injection` | `&mut SessionInjection` | Write-only mechanism to inject state/metadata into the session |

### `after_dispatch` Parameters

| Parameter | Type | Description |
|---|---|---|
| `ctx` | `&RequestContext<'_>` | Read-only request context (note: not mutable) |
| `result` | `&mut DispatcherResult` | Mutable dispatcher result — can inspect/modify |

## RequestContext

Normalized request context across all transports (HTTP, Lambda).

```rust
pub struct RequestContext<'a> {
    method: &'a str,
    params: Option<Value>,
    metadata: Map<String, Value>,
}
```

### Methods

| Method | Return | Description |
|---|---|---|
| `method()` | `&str` | MCP method name (e.g., `"tools/call"`, `"initialize"`) |
| `params()` | `Option<&Value>` | Request parameters (JSON-RPC params field) |
| `params_mut()` | `Option<&mut Value>` | Mutable access to request parameters |
| `metadata()` | `&Map<String, Value>` | Transport metadata (HTTP headers, Lambda event fields) |
| `add_metadata(key, value)` | `()` | Add a metadata entry |

### Common metadata keys

For HTTP transport:
- `"x-api-key"` — API key header
- `"user-agent"` — User-Agent header
- `"x-forwarded-for"` — Client IP (behind proxy)

For Lambda transport:
- `"x-authorizer-principalid"` — API Gateway authorizer principal
- `"x-authorizer-*"` — Authorizer context fields
- `"source-ip"` — API Gateway source IP

## SessionInjection

Write-only mechanism for middleware to populate session state.

```rust
pub struct SessionInjection {
    state: HashMap<String, Value>,
    metadata: HashMap<String, Value>,
}
```

### Methods

| Method | Description |
|---|---|
| `set_state(key, value)` | Inject state (tools read via `session.get_typed_state(key).await`) |
| `set_metadata(key, value)` | Inject metadata |
| `is_empty()` | Check if any injections exist |

### Lifecycle

1. Each middleware gets its own fresh `SessionInjection`
2. The stack **accumulates** injections from all middleware
3. Later middleware **overrides** earlier middleware for the same key
4. Injections are **applied to the session** only after ALL `before_dispatch` middleware succeeds
5. If any middleware returns `Err`, no injections are applied

## DispatcherResult

Result from the MCP dispatcher, available in `after_dispatch`.

```rust
pub enum DispatcherResult {
    Success(Value),
    Error(String),
}
```

### Methods

| Method | Return | Description |
|---|---|---|
| `is_success()` | `bool` | Check if result is successful |
| `is_error()` | `bool` | Check if result is an error |
| `success()` | `Option<&Value>` | Get success value (if any) |
| `success_mut()` | `Option<&mut Value>` | Get mutable success value |
| `error()` | `Option<&str>` | Get error message (if any) |

## SessionView

Read-only session access available in `before_dispatch` (when `Some`).

Provided by `turul_mcp_session_storage::SessionView`. Key methods:

| Method | Return | Description |
|---|---|---|
| `session_id()` | `Option<&str>` | The session's unique ID |

## Registration

```rust
use std::sync::Arc;

let server = McpServer::builder()
    .middleware(Arc::new(MyMiddleware))  // Takes Arc<dyn McpMiddleware>
    .build()?;
```

Multiple middleware are executed in registration order (before) / reverse order (after).

## Execution Flow

```
Request arrives
    │
    ▼
before_dispatch [middleware 1]  →  before_dispatch [middleware 2]  →  ...
    │                                     │
    │ (error? → stop chain)               │ (error? → stop chain)
    │                                     │
    ▼                                     ▼
       Apply accumulated SessionInjection to session
    │
    ▼
       MCP Dispatcher (handler execution)
    │
    ▼
after_dispatch [middleware N]  →  ...  →  after_dispatch [middleware 1]
    │                                     │
    ▼                                     ▼
       Return response to client
```
