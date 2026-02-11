# Turul JSON-RPC Server

A generic JSON-RPC 2.0 server implementation that provides the foundation for the MCP protocol transport layer.

## Features

- **JSON-RPC 2.0 Compliant**: Full specification support with proper error handling
- **Type-Safe Error Handling**: Domain errors with automatic protocol conversion
- **Session Support**: Optional session context for stateful operations
- **Generic Handler Interface**: Flexible handler registration system
- **Unified Error Architecture**: Clean domain/protocol separation

## Key Architecture

The framework uses a **clean domain/protocol separation** where:

1. **Handlers return domain errors only** (`Result<Value, DomainError>`)
2. **Dispatcher owns protocol conversion** (domain → JSON-RPC errors)
3. **No double-wrapping** or protocol structures in business logic

## Quick Start

```rust
use turul_mcp_json_rpc_server::{
    JsonRpcHandler, JsonRpcDispatcher, RequestParams,
    r#async::{SessionContext, ToJsonRpcError},
    dispatch::parse_json_rpc_message,
    error::JsonRpcErrorObject,
};
use serde_json::{Value, json};
use async_trait::async_trait;

/// Calculator error type - handlers return domain errors only
#[derive(thiserror::Error, Debug)]
enum CalculatorError {
    #[error("Missing parameters: {0}")]
    MissingParameters(String),
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
    #[error("Unknown method: {0}")]
    UnknownMethod(String),
}

impl ToJsonRpcError for CalculatorError {
    fn to_error_object(&self) -> JsonRpcErrorObject {
        match self {
            CalculatorError::MissingParameters(msg) => JsonRpcErrorObject::invalid_params(msg),
            CalculatorError::InvalidParameter(msg) => JsonRpcErrorObject::invalid_params(msg),
            CalculatorError::UnknownMethod(method) => JsonRpcErrorObject::method_not_found(method),
        }
    }
}

struct CalculatorHandler;

#[async_trait]
impl JsonRpcHandler for CalculatorHandler {
    type Error = CalculatorError;  // Handlers specify their error type

    async fn handle(
        &self,
        method: &str,
        params: Option<RequestParams>,
        _session: Option<SessionContext>
    ) -> Result<Value, Self::Error> {  // Return domain errors only
        match method {
            "add" => {
                let params = params.ok_or_else(|| {
                    CalculatorError::MissingParameters("Missing parameters for add operation".to_string())
                })?;

                let map = params.to_map();
                let a = map.get("a").and_then(|v| v.as_f64()).ok_or_else(|| {
                    CalculatorError::InvalidParameter("Parameter 'a' is required and must be a number".to_string())
                })?;
                let b = map.get("b").and_then(|v| v.as_f64()).ok_or_else(|| {
                    CalculatorError::InvalidParameter("Parameter 'b' is required and must be a number".to_string())
                })?;

                Ok(json!({"result": a + b}))
            }
            "subtract" => {
                let params = params.ok_or_else(|| {
                    CalculatorError::MissingParameters("Missing parameters for subtract operation".to_string())
                })?;

                let map = params.to_map();
                let a = map.get("a").and_then(|v| v.as_f64()).ok_or_else(|| {
                    CalculatorError::InvalidParameter("Parameter 'a' is required and must be a number".to_string())
                })?;
                let b = map.get("b").and_then(|v| v.as_f64()).ok_or_else(|| {
                    CalculatorError::InvalidParameter("Parameter 'b' is required and must be a number".to_string())
                })?;

                Ok(json!({"result": a - b}))
            }
            _ => Err(CalculatorError::UnknownMethod(
                format!("{}. Supported methods: add, subtract", method)
            ))
        }
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["add".to_string(), "subtract".to_string()]
    }
}

#[tokio::main]
async fn main() {
    // Create type-safe dispatcher
    let mut dispatcher: JsonRpcDispatcher<CalculatorError> = JsonRpcDispatcher::new();
    dispatcher.register_methods(
        vec!["add".to_string(), "subtract".to_string()],
        CalculatorHandler,
    );

    // Example request processing
    let request_json = r#"{"jsonrpc": "2.0", "method": "add", "params": {"a": 5, "b": 3}, "id": 1}"#;

    match parse_json_rpc_message(request_json) {
        Ok(turul_mcp_json_rpc_server::dispatch::JsonRpcMessage::Request(request)) => {
            let response = dispatcher.handle_request(request).await;
            println!("Response: {}", serde_json::to_string_pretty(&response).unwrap());
        }
        _ => println!("Invalid request"),
    }
}
```

## Error Handling Architecture

### 🚨 Critical: Use Domain Errors Only

**✅ CORRECT Pattern:**
```rust
#[derive(thiserror::Error, Debug)]
enum MyError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

impl ToJsonRpcError for MyError {
    fn to_error_object(&self) -> JsonRpcErrorObject {
        match self {
            MyError::InvalidInput(msg) => JsonRpcErrorObject::invalid_params(msg),
        }
    }
}

impl JsonRpcHandler for MyHandler {
    type Error = MyError;

    async fn handle(&self, ...) -> Result<Value, MyError> {
        Err(MyError::InvalidInput("Bad data".to_string()))  // Domain error only
    }
}
```

**❌ WRONG Pattern:**
```rust
// DON'T DO THIS - JsonRpcProcessingError no longer exists
use turul_mcp_json_rpc_server::error::JsonRpcProcessingError;  // Removed!

// DON'T DO THIS - Never create protocol errors in handlers
impl JsonRpcHandler for MyHandler {
    async fn handle(&self, ...) -> Result<Value, JsonRpcError> {  // NO!
        Err(JsonRpcError::new(...))  // Wrong layer!
    }
}
```

### JSON-RPC Error Conversion

The dispatcher automatically converts domain errors to proper JSON-RPC errors:

- **Domain Error**: `InvalidInput("missing field")`
- **JSON-RPC Error**: `{"code": -32602, "message": "missing field"}`

### Response Format

**Success Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {"calculation": 42}
}
```

**Error Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32602,
    "message": "Invalid parameter: missing required field"
  }
}
```

## Advanced Usage

### Session Context

```rust
#[async_trait]
impl JsonRpcHandler for StatefulHandler {
    type Error = MyError;

    async fn handle(
        &self,
        method: &str,
        params: Option<RequestParams>,
        session: Option<SessionContext>
    ) -> Result<Value, Self::Error> {
        if let Some(ctx) = session {
            // Use async session operations
            let state = (ctx.get_state)("user_id").await;
            (ctx.set_state)("last_method", json!(method)).await;
        }
        Ok(json!({"status": "success"}))
    }
}
```

### Multiple Handlers

```rust
let mut dispatcher: JsonRpcDispatcher<MyError> = JsonRpcDispatcher::new();

// Register specific methods
dispatcher.register_method("calculate".to_string(), CalculatorHandler);
dispatcher.register_method("validate".to_string(), ValidatorHandler);

// Register default handler for unmatched methods
dispatcher.set_default_handler(FallbackHandler);
```

## Migration Guide

When upgrading from earlier versions:

1. **Remove JsonRpcProcessingError imports** - no longer exists
2. **Add associated Error type** to JsonRpcHandler implementations
3. **Implement ToJsonRpcError** for your error types
4. **Return domain errors directly** - no protocol layer creation
5. **Use JsonRpcDispatcher<YourError>** with explicit type parameter

## Features

- `serde`: JSON serialization/deserialization support
- `async`: Async handler support with futures
- `session`: Session context support for stateful operations

## Dependencies

This crate depends on:
- `serde` and `serde_json` for JSON handling
- `async-trait` for async trait support
- `thiserror` for domain error definitions
- `tokio` for async runtime support

For MCP protocol support, use this with `turul-mcp-server` which provides the high-level MCP server framework.