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
turul-mcp-json-rpc-server = "0.2.0"
tokio = { version = "1.0", features = ["macros"] }
serde_json = "1.0"
```

### Basic JSON-RPC Handler

```rust
use turul_mcp_json_rpc_server::{
    JsonRpcDispatcher, JsonRpcHandler, SessionContext, RequestParams,
    r#async::JsonRpcResult, dispatch::parse_json_rpc_message
};
use serde_json::{Value, json};
use async_trait::async_trait;

struct CalculatorHandler;

#[async_trait]
impl JsonRpcHandler for CalculatorHandler {
    async fn handle(
        &self, 
        method: &str, 
        params: Option<RequestParams>, 
        _session: Option<SessionContext>
    ) -> JsonRpcResult<Value> {
        match method {
            "add" => {
                let params = params.ok_or_else(|| {
                    turul_mcp_json_rpc_server::error::JsonRpcProcessingError::HandlerError(
                        "Missing parameters for add operation".to_string()
                    )
                })?;
                
                let map = params.to_map();
                let a = map.get("a").and_then(|v| v.as_f64()).ok_or_else(|| {
                    turul_mcp_json_rpc_server::error::JsonRpcProcessingError::HandlerError(
                        "Parameter 'a' is required and must be a number".to_string()
                    )
                })?;
                let b = map.get("b").and_then(|v| v.as_f64()).ok_or_else(|| {
                    turul_mcp_json_rpc_server::error::JsonRpcProcessingError::HandlerError(
                        "Parameter 'b' is required and must be a number".to_string()
                    )
                })?;
                
                Ok(json!({"result": a + b}))
            }
            "subtract" => {
                let params = params.ok_or_else(|| {
                    turul_mcp_json_rpc_server::error::JsonRpcProcessingError::HandlerError(
                        "Missing parameters for subtract operation".to_string()
                    )
                })?;
                
                let map = params.to_map();
                let a = map.get("a").and_then(|v| v.as_f64()).ok_or_else(|| {
                    turul_mcp_json_rpc_server::error::JsonRpcProcessingError::HandlerError(
                        "Parameter 'a' is required and must be a number".to_string()
                    )
                })?;
                let b = map.get("b").and_then(|v| v.as_f64()).ok_or_else(|| {
                    turul_mcp_json_rpc_server::error::JsonRpcProcessingError::HandlerError(
                        "Parameter 'b' is required and must be a number".to_string()
                    )
                })?;
                
                Ok(json!({"result": a - b}))
            }
            _ => Err(turul_mcp_json_rpc_server::error::JsonRpcProcessingError::HandlerError(
                format!("Unknown method: {}. Supported methods: add, subtract", method)
            ))
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
    dispatcher.register_methods(
        vec!["add".to_string(), "subtract".to_string()],
        CalculatorHandler,
    );

    // Parse and handle a JSON-RPC request
    let request_json = r#"{"jsonrpc": "2.0", "method": "add", "params": {"a": 5, "b": 3}, "id": 1}"#;
    
    match parse_json_rpc_message(request_json) {
        Ok(turul_mcp_json_rpc_server::dispatch::JsonRpcMessage::Request(request)) => {
            let response = dispatcher.handle_request(request).await;
            let response_json = serde_json::to_string(&response)?;
            println!("Response: {}", response_json);
            // Output: {"jsonrpc":"2.0","id":1,"result":{"result":8}}
        }
        Err(e) => println!("Parse error: {}", e),
        _ => println!("Received notification (no response needed)"),
    }

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
dispatcher.register_method("math.add".to_string(), MathHandler);
dispatcher.register_method("string.uppercase".to_string(), StringHandler);
dispatcher.register_method("file.read".to_string(), FileHandler);

// Handle requests with method routing
let response = dispatcher.handle_request_with_context(request, session_context).await;
```

### JsonRpcHandler Trait

Implement this trait to handle JSON-RPC method calls:

```rust
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

struct MyHandler {
    state: Arc<Mutex<MyState>>,
}

#[async_trait]
impl JsonRpcHandler for MyHandler {
    async fn handle(
        &self,
        method: &str,
        params: Option<RequestParams>,
        session: Option<SessionContext>
    ) -> JsonRpcResult<Value> {
        match method {
            "get_status" => {
                Ok(json!({
                    "status": "active",
                    "timestamp": chrono::Utc::now().timestamp(),
                    "session_id": session.as_ref().map(|s| &s.session_id)
                }))
            }
            "process_data" => {
                let params = params.ok_or_else(|| {
                    turul_mcp_json_rpc_server::error::JsonRpcProcessingError::HandlerError(
                        "Missing data parameter".to_string()
                    )
                })?;
                
                let map = params.to_map();
                let data = map.get("data").ok_or_else(|| {
                    turul_mcp_json_rpc_server::error::JsonRpcProcessingError::HandlerError(
                        "Missing 'data' field in parameters".to_string()
                    )
                })?;
                
                let result = self.process(data).await?;
                Ok(json!({"processed": result}))
            }
            _ => Err(turul_mcp_json_rpc_server::error::JsonRpcProcessingError::HandlerError(
                format!("Unknown method: {}", method)
            ))
        }
    }

    async fn handle_notification(
        &self,
        method: &str,
        params: Option<RequestParams>,
        _session: Option<SessionContext>
    ) -> JsonRpcResult<()> {
        match method {
            "log" => {
                if let Some(params) = params {
                    println!("Log: {:?}", params.to_map());
                }
                Ok(())
            }
            "ping" => {
                println!("Received ping notification");
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
        params: Option<RequestParams>,
        session: Option<SessionContext>
    ) -> JsonRpcResult<Value> {
        let session = session.ok_or("Session required")?;

        match method {
            "get_session_info" => {
                Ok(json!({
                    "session_id": session.session_id,
                    "timestamp": session.timestamp
                }))
            }
            "increment_counter" => {
                // if let Some(p) = params { /* use p.to_map() */ }
                Ok(json!({"message": "Counter increment processed"}))
            }
            _ => Err(JsonRpcError::method_not_found(method).into())
        }
    }
}
```

## Error Handling

### Standard JSON-RPC Error Codes

```rust
use turul_mcp_json_rpc_server::error::{JsonRpcErrorObject, JsonRpcProcessingError};

// Standard error codes using JsonRpcErrorObject
let parse_error = JsonRpcErrorObject::parse_error();

let invalid_request = JsonRpcErrorObject::invalid_request();

let method_not_found = JsonRpcErrorObject::method_not_found();

let invalid_params = JsonRpcErrorObject::invalid_params("Required parameter missing");

let internal_error = JsonRpcErrorObject::internal_error(Some("Database connection failed".to_string()));

// For handler errors, use JsonRpcProcessingError
let handler_error = JsonRpcProcessingError::HandlerError(
    "Custom application error".to_string()
);
```

### Custom Error Handling

```rust
struct DatabaseHandler {
    // In practice, you'd have a database connection here
}

#[async_trait]
impl JsonRpcHandler for DatabaseHandler {
    async fn handle(
        &self,
        method: &str,
        params: Option<RequestParams>,
        _session: Option<SessionContext>
    ) -> JsonRpcResult<Value> {
        match method {
            "query_users" => {
                // Simulate database operation
                match self.simulate_database_query().await {
                    Ok(users) => {
                        Ok(json!({"users": users}))
                    }
                    Err(e) => {
                        Err(turul_mcp_json_rpc_server::error::JsonRpcProcessingError::HandlerError(
                            format!("Database query failed: {}", e)
                        ))
                    }
                }
            }
            _ => Err(turul_mcp_json_rpc_server::error::JsonRpcProcessingError::HandlerError(
                format!("Unknown method: {}", method)
            ))
        }
    }
}

impl DatabaseHandler {
    async fn simulate_database_query(&self) -> Result<Vec<Value>, &'static str> {
        // Simulate potential database errors
        Ok(vec![
            json!({"id": 1, "name": "Alice"}),
            json!({"id": 2, "name": "Bob"})
        ])
    }
}
```

## Batch Requests

Handle multiple requests in a single call:

```rust
use turul_mcp_json_rpc_server::dispatch::parse_json_rpc_message;
use serde_json::json;

// Note: This crate handles individual JSON-RPC messages
// Batch processing would be handled by the transport layer
let batch_request_json = r#"[
    {"jsonrpc": "2.0", "id": 1, "method": "add", "params": {"a": 1, "b": 2}},
    {"jsonrpc": "2.0", "id": 2, "method": "subtract", "params": {"a": 5, "b": 3}},
    {"jsonrpc": "2.0", "method": "log", "params": {"message": "Batch processed"}}
]"#;

// Parse each message in the batch
let batch: Vec<serde_json::Value> = serde_json::from_str(batch_request_json)?;
let mut responses = Vec::new();

for message_json in batch {
    let message_str = serde_json::to_string(&message_json)?;
    match parse_json_rpc_message(&message_str) {
        Ok(turul_mcp_json_rpc_server::dispatch::JsonRpcMessage::Request(request)) => {
            let response = dispatcher.handle_request(request).await;
            responses.push(serde_json::to_value(response)?);
        }
        Ok(turul_mcp_json_rpc_server::dispatch::JsonRpcMessage::Notification(notification)) => {
            let _ = dispatcher.handle_notification(notification).await;
            // Notifications don't generate responses
        }
        Err(_) => {
            // Handle parse errors as needed
        }
    }
}

// Return array of responses (notifications don't generate responses)
let batch_response = json!(responses);
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
        params: Option<RequestParams>,
        session: Option<SessionContext>
    ) -> JsonRpcResult<()> {
        match method {
            "user_activity" => {
                if let Some(params) = params {
                    let map = params.to_map();
                    println!("User activity: {:?}", map);
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
        _params: Option<RequestParams>,
        _session: Option<SessionContext>
    ) -> JsonRpcResult<Value> {
        Err(turul_mcp_json_rpc_server::error::JsonRpcProcessingError::HandlerError(
            format!("Method not supported for requests: {}", method)
        ))
    }
}
```

## Integration with Transport Layers

### HTTP Integration Example

```rust
use hyper::{Body, Request, Response, StatusCode};
use turul_mcp_json_rpc_server::{
    JsonRpcDispatcher, dispatch::parse_json_rpc_message, 
    types::JsonRpcResponse, error::JsonRpcErrorObject
};
use std::sync::Arc;
use serde_json::Value;

async fn handle_http_request(
    req: Request<Body>,
    dispatcher: Arc<JsonRpcDispatcher>
) -> Result<Response<Body>, hyper::Error> {
    let body_bytes = hyper::body::to_bytes(req.into_body()).await?;
    
    let request_str = match std::str::from_utf8(&body_bytes) {
        Ok(s) => s,
        Err(_) => {
            let error_response = JsonRpcResponse::error(
                None,
                JsonRpcErrorObject::parse_error()
            );
            let response_json = serde_json::to_string(&error_response).unwrap();
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from(response_json))
                .unwrap());
        }
    };

    match parse_json_rpc_message(request_str) {
        Ok(turul_mcp_json_rpc_server::dispatch::JsonRpcMessage::Request(request)) => {
            let response = dispatcher.handle_request(request).await;
            let response_json = serde_json::to_string(&response).unwrap();
            
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/json")
                .body(Body::from(response_json))
                .unwrap())
        }
        Ok(turul_mcp_json_rpc_server::dispatch::JsonRpcMessage::Notification(notification)) => {
            let _ = dispatcher.handle_notification(notification).await;
            // Notifications don't generate responses
            Ok(Response::builder()
                .status(StatusCode::NO_CONTENT)
                .body(Body::empty())
                .unwrap())
        }
        Err(_) => {
            let error_response = JsonRpcResponse::error(
                None,
                JsonRpcErrorObject::parse_error()
            );
            let response_json = serde_json::to_string(&error_response).unwrap();
            Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
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
use turul_mcp_json_rpc_server::{
    JsonRpcDispatcher, dispatch::parse_json_rpc_message,
    types::JsonRpcResponse, error::JsonRpcErrorObject
};
use std::sync::Arc;

async fn handle_websocket_connection(
    ws_stream: WebSocketStream<tokio::net::TcpStream>,
    dispatcher: Arc<JsonRpcDispatcher>
) {
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    while let Some(msg) = ws_receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                match parse_json_rpc_message(&text) {
                    Ok(turul_mcp_json_rpc_server::dispatch::JsonRpcMessage::Request(request)) => {
                        let response = dispatcher.handle_request(request).await;
                        let response_json = serde_json::to_string(&response).unwrap();
                        let _ = ws_sender.send(Message::Text(response_json)).await;
                    }
                    Ok(turul_mcp_json_rpc_server::dispatch::JsonRpcMessage::Notification(notification)) => {
                        let _ = dispatcher.handle_notification(notification).await;
                        // Notifications don't generate responses
                    }
                    Err(_) => {
                        let error = JsonRpcResponse::error(
                            None,
                            JsonRpcErrorObject::parse_error()
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
        
        // Create parameters using RequestParams
        let params = turul_mcp_json_rpc_server::RequestParams::from_map(
            vec![
                ("a".to_string(), json!(5)),
                ("b".to_string(), json!(3))
            ].into_iter().collect()
        );
        
        // Test add method
        let result = handler.handle(
            "add",
            Some(params),
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
        dispatcher.register_methods(
            vec!["add".to_string(), "subtract".to_string()],
            CalculatorHandler,
        );

        // Create a proper JsonRpcRequest
        let request = turul_mcp_json_rpc_server::types::JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "add".to_string(),
            params: Some(turul_mcp_json_rpc_server::RequestParams::from_map(
                vec![
                    ("a".to_string(), json!(10)),
                    ("b".to_string(), json!(20))
                ].into_iter().collect()
            )),
            id: Some(turul_mcp_json_rpc_server::types::RequestId::Number(1)),
        };

        let response = dispatcher.handle_request(request).await;
        
        // Check response structure
        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(turul_mcp_json_rpc_server::types::RequestId::Number(1)));
        
        if let Some(result) = response.result {
            assert_eq!(result["result"], json!(30));
        } else {
            panic!("Expected result, got error: {:?}", response.error);
        }
    }
}
```

### Integration Testing

```rust
use tokio_test;

#[tokio::test]
async fn test_full_json_rpc_flow() {
    let mut dispatcher = JsonRpcDispatcher::new();
    dispatcher.register_method("echo".to_string(), TestHandler);

    // Test request
    let request_json = r#"{"jsonrpc": "2.0", "id": "test-123", "method": "echo", "params": {"message": "Hello, JSON-RPC!"}}"#;
    
    match turul_mcp_json_rpc_server::dispatch::parse_json_rpc_message(request_json) {
        Ok(turul_mcp_json_rpc_server::dispatch::JsonRpcMessage::Request(request)) => {
            let response = dispatcher.handle_request(request).await;
            
            assert_eq!(response.jsonrpc, "2.0");
            assert_eq!(response.id, Some(turul_mcp_json_rpc_server::types::RequestId::String("test-123".to_string())));
            
            if let Some(result) = response.result {
                assert_eq!(result["echo"], "Hello, JSON-RPC!");
            } else {
                panic!("Expected result, got error: {:?}", response.error);
            }
        }
        _ => panic!("Failed to parse request"),
    }

    // Test notification (no response expected)
    let notification_json = r#"{"jsonrpc": "2.0", "method": "log", "params": {"level": "info", "message": "Test log"}}"#;
    
    match turul_mcp_json_rpc_server::dispatch::parse_json_rpc_message(notification_json) {
        Ok(turul_mcp_json_rpc_server::dispatch::JsonRpcMessage::Notification(notification)) => {
            let result = dispatcher.handle_notification(notification).await;
            assert!(result.is_ok()); // Notifications should be handled without error
        }
        _ => panic!("Failed to parse notification"),
    }
}
```

## Feature Flags

```toml
[dependencies]
turul-mcp-json-rpc-server = { version = "0.2.0", features = ["async"] }
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