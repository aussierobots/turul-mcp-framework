# Error Handling Guide

Deep-dive reference for MCP client error types, retryability, and recovery patterns.

## McpClientError Variant Catalog

### Transport Errors

`McpClientError::Transport(TransportError)` — problems with the underlying connection.

| Sub-variant | Meaning | Retryable? |
|---|---|---|
| `Http(String)` | HTTP transport-specific error | No |
| `Sse(String)` | SSE transport-specific error | No |
| `Stdio(String)` | Stdio transport error | No |
| `Unsupported(String)` | Unknown transport type | No |
| `ConnectionFailed(String)` | Could not establish connection | **Yes** |
| `Closed` | Transport closed unexpectedly | **Yes** |

### Protocol Errors

`McpClientError::Protocol(ProtocolError)` — MCP protocol violations. Never retryable.

| Sub-variant | Meaning |
|---|---|
| `InvalidRequest(String)` | Malformed JSON-RPC request |
| `InvalidResponse(String)` | Malformed JSON-RPC response |
| `UnsupportedVersion(String)` | Server protocol version incompatible |
| `MethodNotFound(String)` | Server doesn't support the method |
| `InvalidParams(String)` | Wrong parameters for method |
| `NegotiationFailed(String)` | Initialize handshake failed |
| `CapabilityMismatch(String)` | Server doesn't support required capability |

### Session Errors

`McpClientError::Session(SessionError)` — session lifecycle issues. Never retryable.

| Sub-variant | Meaning |
|---|---|
| `NotInitialized` | Operation before `connect()` |
| `AlreadyInitialized` | Double `connect()` call |
| `Expired` | Session timed out on server |
| `Terminated` | Session explicitly terminated |
| `InvalidState { expected, actual }` | Unexpected state transition |
| `RecoveryFailed(String)` | Reconnection attempt failed |

### Other Variants

| Variant | Meaning | Retryable? |
|---|---|---|
| `Connection(reqwest::Error)` | Network-level errors | **Yes** |
| `Timeout` | Operation exceeded deadline | **Yes** |
| `ServerError { code, message, data }` | JSON-RPC error from server | Codes -32099..-32000 **yes** |
| `Auth(String)` | Authentication failure | No |
| `Json(serde_json::Error)` | JSON parse error | No |
| `Config(String)` | Configuration invalid | No |
| `Generic { message }` | Catch-all | No |

## Built-in Helpers

```rust
// Check retryability
if error.is_retryable() {
    // Safe to retry: ConnectionFailed, Closed, Connection, Timeout,
    // ServerError with code in -32099..-32000
}

// Classify the error
error.is_protocol_error()  // true for Protocol(_) variants
error.is_session_error()   // true for Session(_) variants
error.error_code()         // Some(i32) for ServerError, None otherwise
```

## Retry with Backoff

`RetryConfig` provides built-in delay calculation:

```rust
// turul-mcp-client v0.3
use turul_mcp_client::config::RetryConfig;

let retry = RetryConfig::default(); // max_attempts: 3, initial_delay: 100ms, backoff: 2.0x

for attempt in 0..retry.max_attempts {
    match client.call_tool("my_tool", args.clone()).await {
        Ok(result) => return Ok(result),
        Err(e) if e.is_retryable() && retry.should_retry(attempt) => {
            let delay = retry.delay_for_attempt(attempt);
            // delay_for_attempt applies: initial_delay * backoff_multiplier^attempt + jitter
            tokio::time::sleep(delay).await;
        }
        Err(e) => return Err(e),
    }
}
```

**`RetryConfig` defaults:**
| Field | Default | Purpose |
|---|---|---|
| `max_attempts` | 3 | Total attempts before giving up |
| `initial_delay` | 100ms | Base delay before first retry |
| `max_delay` | 10s | Ceiling on delay growth |
| `backoff_multiplier` | 2.0 | Exponential growth factor |
| `jitter` | 0.1 | Random variance (10%) to avoid thundering herd |
| `exponential_backoff` | true | Enable exponential growth vs fixed delay |

## Wrapping in Application Errors

```rust
// turul-mcp-client v0.3
use turul_mcp_client::McpClientError;

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("MCP client error: {0}")]
    Mcp(#[from] McpClientError),

    #[error("Application error: {0}")]
    App(String),
}

// McpClientError converts automatically via From
fn do_work(client: &McpClient) -> Result<(), AppError> {
    let tools = client.list_tools().await?; // McpClientError → AppError::Mcp
    Ok(())
}
```

## The client_error! Macro

For creating `McpClientError::Generic` variants with formatting:

```rust
use turul_mcp_client::client_error;

let err = client_error!("unexpected state: {}", state);
// Equivalent to: McpClientError::generic(format!("unexpected state: {}", state))
```
