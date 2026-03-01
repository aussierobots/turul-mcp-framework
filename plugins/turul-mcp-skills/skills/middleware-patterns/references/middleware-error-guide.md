# MiddlewareError Reference

Complete reference for middleware error variants, constructors, and JSON-RPC error code mapping.

## Import

```rust
use turul_http_mcp_server::middleware::MiddlewareError;
```

## Error Variants

### Unauthenticated

No credentials provided.

```rust
MiddlewareError::Unauthenticated(String)
MiddlewareError::unauthenticated("Missing API key")
```

- **JSON-RPC code:** -32001
- **Use when:** No authentication token/key is present in the request
- **Wire message:** `"Authentication required: Missing API key"`

### Unauthorized

Credentials provided but insufficient permissions.

```rust
MiddlewareError::Unauthorized(String)
MiddlewareError::unauthorized("Insufficient permissions for admin endpoint")
```

- **JSON-RPC code:** -32002
- **Use when:** A valid credential is present but lacks the required scope/role
- **Wire message:** `"Unauthorized: Insufficient permissions for admin endpoint"`

### RateLimitExceeded

Request rate limit exceeded.

```rust
MiddlewareError::RateLimitExceeded { message: String, retry_after: Option<u64> }
MiddlewareError::rate_limit("Too many requests", Some(60))
MiddlewareError::rate_limit("Rate limit exceeded", None)
```

- **JSON-RPC code:** -32003
- **Use when:** Per-session or global rate limits are exceeded
- **`retry_after`:** Seconds until the limit resets (optional)
- **Wire message:** `"Too many requests (retry after 60 seconds)"` or `"Rate limit exceeded"`

### InvalidRequest

Request validation failed.

```rust
MiddlewareError::InvalidRequest(String)
MiddlewareError::invalid_request("Malformed request body")
```

- **JSON-RPC code:** -32600 (standard JSON-RPC Invalid Request)
- **Use when:** The request structure is malformed before it reaches the handler
- **Wire message:** `"Invalid request: Malformed request body"`

### Internal

Internal middleware error (details should not be exposed to client).

```rust
MiddlewareError::Internal(String)
MiddlewareError::internal("Database connection pool exhausted")
```

- **JSON-RPC code:** -32603 (standard JSON-RPC Internal error)
- **Use when:** An internal system error occurs (database down, config missing)
- **Wire message:** `"Internal middleware error: Database connection pool exhausted"`
- **Note:** Consider logging the full error and returning a generic message to avoid leaking internals

### Custom

Application-specific error with a custom code.

```rust
MiddlewareError::Custom { code: String, message: String }
MiddlewareError::custom("MAINTENANCE", "Server is in maintenance mode")
```

- **JSON-RPC code:** Custom (application-defined)
- **Use when:** None of the built-in variants fits your use case
- **Wire message:** `"MAINTENANCE: Server is in maintenance mode"`

## Conversion Chain

```
MiddlewareError
    │
    ▼  (framework conversion)
McpError
    │
    ▼  McpError::to_error_object()
JsonRpcErrorObject
    │
    ▼  (transport serialization)
HTTP/Lambda response
```

Middleware NEVER creates `McpError` or `JsonRpcError` directly. Return `MiddlewareError` and the framework handles conversion.

## Error Code Summary

| Variant | Constructor | JSON-RPC Code |
|---|---|---|
| `Unauthenticated` | `unauthenticated(msg)` | -32001 |
| `Unauthorized` | `unauthorized(msg)` | -32002 |
| `RateLimitExceeded` | `rate_limit(msg, retry_after)` | -32003 |
| `InvalidRequest` | `invalid_request(msg)` | -32600 |
| `Internal` | `internal(msg)` | -32603 |
| `Custom` | `custom(code, msg)` | custom |

## Unauthenticated vs Unauthorized

| Scenario | Error |
|---|---|
| No `x-api-key` header in request | `Unauthenticated` |
| Invalid/expired API key | `Unauthorized` |
| Valid key but no access to this method | `Unauthorized` |
| No Lambda authorizer principal | `Unauthenticated` |
| Authorizer principal lacks required role | `Unauthorized` |
