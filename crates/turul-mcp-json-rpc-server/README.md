# turul-mcp-json-rpc-server

[![Crates.io](https://img.shields.io/crates/v/turul-mcp-json-rpc-server.svg)](https://crates.io/crates/turul-mcp-json-rpc-server)
[![Documentation](https://docs.rs/turul-mcp-json-rpc-server/badge.svg)](https://docs.rs/turul-mcp-json-rpc-server)

Transport-agnostic JSON-RPC 2.0 server implementation with MCP-aware session management and comprehensive error handling.

## Overview

`turul-mcp-json-rpc-server` provides a pure JSON-RPC 2.0 server implementation that follows the specification exactly. It's completely transport-agnostic, making it suitable for HTTP, WebSocket, TCP, stdio, or any other transport layer.

## Features

- ✅ **JSON-RPC 2.0 Specification Compliance** - Full adherence to the JSON-RPC 2.0 standard
- ✅ **Transport Agnostic** - Works with any transport layer (HTTP, WebSocket, TCP, stdio)
- ✅ **Async/Await Support** - Built for modern Rust async/await patterns
- ✅ **Session Context** - MCP-aware session management for stateful operations
- ✅ **Comprehensive Error Handling** - Standard error codes with detailed context
- ✅ **Notification Support** - Handle both requests and notifications
- ✅ **Method Registration** - Dynamic method registration with type safety

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
turul-mcp-json-rpc-server = "0.1.1"
tokio = { version = "1.0", features = ["macros"] }
serde_json = "1.0"
```

### Basic JSON-RPC Handler

```rust
use turul_mcp_json_rpc_server::{
    JsonRpcDispatcher, JsonRpcHandler, SessionContext, JsonRpcResult
};
use serde_json::{Value, json};
use async_trait::async_trait;

struct CalculatorHandler;

#[async_trait]
impl JsonRpcHandler for CalculatorHandler {
    async fn handle(
        &self, 
        method: &str, 
        params: Option<serde_json::Value>, 
        _session: Option<SessionContext>
    ) -> JsonRpcResult<Value> {
        match method {
            "add" => {
                let params = params.ok_or("Missing parameters")?;
                let a = params["a"].as_f64().ok_or("Missing 'a' parameter")?;
                let b = params["b"].as_f64().ok_or("Missing 'b' parameter")?;
                Ok(json!({"result": a + b}))
            }
            "subtract" => {
                let params = params.ok_or("Missing parameters")?;
                let a = params["a"].as_f64().ok_or("Missing 'a' parameter")?;
                let b = params["b"].as_f64().ok_or("Missing 'b' parameter")?;
                Ok(json!({"result": a - b}))
            }
            _ => Err(format!("Unknown method: {}", method).into())
        }
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["add".to_string(), "subtract".to_string()]
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create dispatcher and register handler
    let mut dispatcher = JsonRpcDispatcher::new();
    dispatcher.register_handler("calculator", Box::new(CalculatorHandler));

    // Handle a JSON-RPC request
    let request_json = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "calculator.add",
        "params": {"a": 5, "b": 3}
    });

    let response = dispatcher.dispatch(request_json, None).await?;
    println!("Response: {}", response);
    // Output: {"jsonrpc":"2.0","id":1,"result":{"result":8}}

    Ok(())
}
```

## Core Components

### JsonRpcDispatcher

The main dispatcher routes JSON-RPC requests to appropriate handlers:

```rust
use turul_mcp_json_rpc_server::JsonRpcDispatcher;

let mut dispatcher = JsonRpcDispatcher::new();

// Register handlers
dispatcher.register_handler("math", Box::new(MathHandler));
dispatcher.register_handler("string", Box::new(StringHandler));
dispatcher.register_handler("file", Box::new(FileHandler));

// Handle requests with method routing: "math.add", "string.uppercase", etc.
let response = dispatcher.dispatch(request_json, session_context).await?;
```

### JsonRpcHandler Trait

Implement this trait to handle JSON-RPC method calls:

```rust
use async_trait::async_trait;

struct MyHandler {
    state: Arc<Mutex<MyState>>,
}

#[async_trait]
impl JsonRpcHandler for MyHandler {
    async fn handle(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
        session: Option<SessionContext>
    ) -> JsonRpcResult<Value> {
        match method {
            "get_status" => {
                Ok(json!({
                    "status": "active",
                    "timestamp": chrono::Utc::now(),
                    "session_id": session.as_ref().map(|s| &s.session_id)
                }))
            }
            "process_data" => {
                let data = params.ok_or("Missing data parameter")?;
                let result = self.process(data).await?;
                Ok(json!({"processed": result}))
            }
            _ => Err(JsonRpcError::method_not_found(method).into())
        }
    }

    async fn handle_notification(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
        session: Option<SessionContext>
    ) -> JsonRpcResult<()> {
        match method {
            "log" => {
                if let Some(params) = params {
                    println!("Log: {:?}", params);
                }
                Ok(())
            }
            "ping" => {
                // Handle ping notification
                Ok(())
            }
            _ => Ok(()) // Ignore unknown notifications
        }
    }

    fn supported_methods(&self) -> Vec<String> {
        vec![
            "get_status".to_string(),
            "process_data".to_string(),
            "log".to_string(),
            "ping".to_string(),
        ]
    }
}
```

## Session Management

### SessionContext

The `SessionContext` provides session-aware request handling:

```rust
use turul_mcp_json_rpc_server::SessionContext;
use std::collections::HashMap;

struct SessionAwareHandler;

#[async_trait]
impl JsonRpcHandler for SessionAwareHandler {
    async fn handle(
        &self,
        method: &str,
        params: Option<Value>,
        session: Option<SessionContext>
    ) -> JsonRpcResult<Value> {
        let session = session.ok_or("Session required")?;
        
        match method {
            "get_session_info" => {
                Ok(json!({
                    "session_id": session.session_id,
                    "timestamp": session.timestamp,
                    "metadata": session.metadata
                }))
            }
            "increment_counter" => {
                // Access session metadata
                let current = session.metadata
                    .get("counter")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                
                let new_count = current + 1;
                
                // In a real implementation, you'd update the session storage
                Ok(json!({"counter": new_count}))
            }
            _ => Err(JsonRpcError::method_not_found(method).into())
        }
    }
}
```

## Error Handling

### Standard JSON-RPC Error Codes

```rust
use turul_mcp_json_rpc_server::{JsonRpcError, JsonRpcErrorCode};

// Standard error codes
let parse_error = JsonRpcError::new(
    JsonRpcErrorCode::ParseError,
    "Invalid JSON was received"
);

let invalid_request = JsonRpcError::new(
    JsonRpcErrorCode::InvalidRequest,
    "The JSON sent is not a valid Request object"
);

let method_not_found = JsonRpcError::method_not_found("unknown_method");

let invalid_params = JsonRpcError::invalid_params("Required parameter missing");

let internal_error = JsonRpcError::internal_error("Database connection failed");
```

### Custom Error Handling

```rust
struct DatabaseHandler {
    pool: sqlx::PgPool,
}

#[async_trait]
impl JsonRpcHandler for DatabaseHandler {
    async fn handle(
        &self,
        method: &str,
        params: Option<Value>,
        _session: Option<SessionContext>
    ) -> JsonRpcResult<Value> {
        match method {
            "query_users" => {
                match sqlx::query!("SELECT * FROM users")
                    .fetch_all(&self.pool)
                    .await
                {
                    Ok(rows) => {
                        let users: Vec<Value> = rows.into_iter()
                            .map(|row| json!({"id": row.id, "name": row.name}))
                            .collect();
                        Ok(json!({"users": users}))
                    }
                    Err(sqlx::Error::Database(db_err)) => {
                        Err(JsonRpcError::new(
                            JsonRpcErrorCode::Custom(-32001),
                            &format!("Database error: {}", db_err)
                        ).into())
                    }
                    Err(e) => {
                        Err(JsonRpcError::internal_error(&format!("Query failed: {}", e)).into())
                    }
                }
            }
            _ => Err(JsonRpcError::method_not_found(method).into())
        }
    }
}
```

## Batch Requests

Handle multiple requests in a single call:

```rust
use serde_json::json;

let batch_request = json!([
    {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "math.add",
        "params": {"a": 1, "b": 2}
    },
    {
        "jsonrpc": "2.0",
        "id": 2,
        "method": "math.multiply",
        "params": {"a": 3, "b": 4}
    },
    {
        "jsonrpc": "2.0",
        "method": "log.info",  // notification (no id)
        "params": {"message": "Batch processed"}
    }
]);

let response = dispatcher.dispatch(batch_request, session).await?;
// Returns array of responses (notifications don't generate responses)
```

## Notifications vs Requests

### Handling Notifications

Notifications don't expect a response and don't have an `id` field:

```rust
struct NotificationHandler;

#[async_trait]
impl JsonRpcHandler for NotificationHandler {
    async fn handle_notification(
        &self,
        method: &str,
        params: Option<Value>,
        session: Option<SessionContext>
    ) -> JsonRpcResult<()> {
        match method {
            "user_activity" => {
                if let Some(params) = params {
                    println!("User activity: {:?}", params);
                    // Log to database, send to analytics, etc.
                }
                Ok(())
            }
            "heartbeat" => {
                println!("Received heartbeat from session: {:?}", 
                         session.as_ref().map(|s| &s.session_id));
                Ok(())
            }
            _ => Ok(()) // Ignore unknown notifications
        }
    }

    async fn handle(
        &self,
        method: &str,
        _params: Option<Value>,
        _session: Option<SessionContext>
    ) -> JsonRpcResult<Value> {
        Err(JsonRpcError::method_not_found(method).into())
    }
}
```

## Integration with Transport Layers

### HTTP Integration Example

```rust
use hyper::{Body, Request, Response, StatusCode};
use turul_mcp_json_rpc_server::JsonRpcDispatcher;

async fn handle_http_request(
    req: Request<Body>,
    dispatcher: Arc<JsonRpcDispatcher>
) -> Result<Response<Body>, hyper::Error> {
    let body_bytes = hyper::body::to_bytes(req.into_body()).await?;
    
    let request_json: Value = match serde_json::from_slice(&body_bytes) {
        Ok(json) => json,
        Err(_) => {
            let error_response = JsonRpcResponse::error(
                None,
                JsonRpcError::parse_error()
            );
            let response_json = serde_json::to_string(&error_response).unwrap();
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from(response_json))
                .unwrap());
        }
    };

    // Create session context from HTTP headers
    let session_id = extract_session_id(&req);
    let session_context = session_id.map(|id| SessionContext {
        session_id: id,
        metadata: HashMap::new(),
        broadcaster: None,
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
    });

    match dispatcher.dispatch(request_json, session_context).await {
        Ok(response_json) => {
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/json")
                .body(Body::from(response_json))
                .unwrap())
        }
        Err(e) => {
            let error_response = JsonRpcResponse::error(
                None,
                JsonRpcError::internal_error(&e.to_string())
            );
            let response_json = serde_json::to_string(&error_response).unwrap();
            Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(response_json))
                .unwrap())
        }
    }
}
```

### WebSocket Integration Example

```rust
use tokio_tungstenite::{WebSocketStream, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};

async fn handle_websocket_connection(
    ws_stream: WebSocketStream<tokio::net::TcpStream>,
    dispatcher: Arc<JsonRpcDispatcher>
) {
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    while let Some(msg) = ws_receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                let request_json: Value = match serde_json::from_str(&text) {
                    Ok(json) => json,
                    Err(_) => {
                        let error = JsonRpcResponse::error(
                            None,
                            JsonRpcError::parse_error()
                        );
                        let _ = ws_sender.send(Message::Text(
                            serde_json::to_string(&error).unwrap()
                        )).await;
                        continue;
                    }
                };

                let response = dispatcher.dispatch(request_json, None).await;
                match response {
                    Ok(response_json) => {
                        let _ = ws_sender.send(Message::Text(response_json)).await;
                    }
                    Err(e) => {
                        let error = JsonRpcResponse::error(
                            None,
                            JsonRpcError::internal_error(&e.to_string())
                        );
                        let _ = ws_sender.send(Message::Text(
                            serde_json::to_string(&error).unwrap()
                        )).await;
                    }
                }
            }
            Ok(Message::Close(_)) => break,
            _ => continue,
        }
    }
}
```

## Testing

### Unit Testing Handlers

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_calculator_handler() {
        let handler = CalculatorHandler;
        
        // Test add method
        let result = handler.handle(
            "add",
            Some(json!({"a": 5, "b": 3})),
            None
        ).await.unwrap();
        
        assert_eq!(result, json!({"result": 8}));
        
        // Test error handling
        let error = handler.handle("unknown", None, None).await;
        assert!(error.is_err());
    }

    #[tokio::test]
    async fn test_dispatcher() {
        let mut dispatcher = JsonRpcDispatcher::new();
        dispatcher.register_handler("calc", Box::new(CalculatorHandler));

        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "calc.add",
            "params": {"a": 10, "b": 20}
        });

        let response = dispatcher.dispatch(request, None).await.unwrap();
        let response_json: Value = serde_json::from_str(&response).unwrap();
        
        assert_eq!(response_json["result"]["result"], json!(30));
        assert_eq!(response_json["id"], json!(1));
        assert_eq!(response_json["jsonrpc"], json!("2.0"));
    }
}
```

### Integration Testing

```rust
use tokio_test;

#[tokio::test]
async fn test_full_json_rpc_flow() {
    let mut dispatcher = JsonRpcDispatcher::new();
    dispatcher.register_handler("test", Box::new(TestHandler));

    // Test request
    let request = json!({
        "jsonrpc": "2.0",
        "id": "test-123",
        "method": "test.echo",
        "params": {"message": "Hello, JSON-RPC!"}
    });

    let response_str = dispatcher.dispatch(request, None).await.unwrap();
    let response: Value = serde_json::from_str(&response_str).unwrap();

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], "test-123");
    assert_eq!(response["result"]["echo"], "Hello, JSON-RPC!");

    // Test notification (no response expected)
    let notification = json!({
        "jsonrpc": "2.0",
        "method": "test.log",
        "params": {"level": "info", "message": "Test log"}
    });

    let response_str = dispatcher.dispatch(notification, None).await.unwrap();
    assert_eq!(response_str, ""); // No response for notifications
}
```

## Feature Flags

```toml
[dependencies]
turul-mcp-json-rpc-server = { version = "0.1.1", features = ["async"] }
```

Available features:
- `default` = `["async"]` - Enable async support
- `async` - Provides async trait and dispatcher support

## Performance Tips

1. **Handler Registration**: Register handlers once at startup, not per request
2. **Session Context**: Keep session context lightweight - avoid large metadata objects
3. **Error Handling**: Use structured errors with proper JSON-RPC error codes
4. **Batch Requests**: Handle batch requests efficiently by processing in parallel where possible
5. **Memory Management**: Consider using `Arc<JsonRpcDispatcher>` for shared access across threads

## License

Licensed under the MIT License. See [LICENSE](../../LICENSE) for details.