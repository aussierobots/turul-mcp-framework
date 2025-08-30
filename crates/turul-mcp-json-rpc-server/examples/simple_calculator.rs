//! Simple Calculator JSON-RPC Example
//!
//! This example demonstrates a basic JSON-RPC server that implements
//! calculator operations (add, subtract) with proper error handling
//! and session context support.

use turul_mcp_json_rpc_server::{
    JsonRpcHandler, JsonRpcDispatcher, RequestParams, SessionContext,
    r#async::JsonRpcResult, dispatch::parse_json_rpc_message,
};
use async_trait::async_trait;
use serde_json::{json, Value};

/// Calculator handler that implements basic arithmetic operations
struct CalculatorHandler;

#[async_trait]
impl JsonRpcHandler for CalculatorHandler {
    async fn handle(&self, method: &str, params: Option<RequestParams>, session_context: Option<SessionContext>) -> JsonRpcResult<Value> {
        // Log session info if available
        if let Some(session) = session_context {
            println!("Processing {} with session: {}", method, session.session_id);
        }
        
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
                
                let result = a + b;
                println!("Addition: {} + {} = {}", a, b, result);
                Ok(json!({"result": result}))
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
                
                let result = a - b;
                println!("Subtraction: {} - {} = {}", a, b, result);
                Ok(json!({"result": result}))
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
async fn main() {
    println!("🧮 Simple Calculator JSON-RPC Server Example");
    println!("=============================================");
    
    // Create dispatcher and register handler
    let mut dispatcher = JsonRpcDispatcher::new();
    dispatcher.register_methods(
        vec!["add".to_string(), "subtract".to_string()],
        CalculatorHandler,
    );

    // Test multiple requests
    let test_requests = vec![
        r#"{"jsonrpc": "2.0", "method": "add", "params": {"a": 5, "b": 3}, "id": 1}"#,
        r#"{"jsonrpc": "2.0", "method": "subtract", "params": {"a": 10, "b": 4}, "id": 2}"#,
        r#"{"jsonrpc": "2.0", "method": "multiply", "params": {"a": 2, "b": 3}, "id": 3}"#, // Will fail
        r#"{"jsonrpc": "2.0", "method": "add", "params": {"a": "invalid", "b": 5}, "id": 4}"#, // Will fail
    ];
    
    for (i, request_json) in test_requests.iter().enumerate() {
        println!("\n--- Test {} ---", i + 1);
        println!("Request: {}", request_json);
        
        // Parse and handle message
        match parse_json_rpc_message(request_json) {
            Ok(turul_mcp_json_rpc_server::dispatch::JsonRpcMessage::Request(request)) => {
                println!("✓ Parsed: method='{}', id={:?}", request.method, request.id);
                
                // Handle request
                let response = dispatcher.handle_request(request).await;
                
                // Serialize and display response
                match serde_json::to_string_pretty(&response) {
                    Ok(response_json) => {
                        println!("Response:\n{}", response_json);
                    }
                    Err(e) => println!("❌ Failed to serialize response: {}", e),
                }
            }
            Ok(turul_mcp_json_rpc_server::dispatch::JsonRpcMessage::Notification(_)) => {
                println!("📢 Received notification (no response needed)");
            }
            Err(e) => {
                println!("❌ Parse error: {}", e);
            }
        }
    }
    
    println!("\n🎉 Calculator example completed!");
    println!("This demonstrates:");
    println!("  • JSON-RPC 2.0 request/response handling");
    println!("  • Session context support (when available)");
    println!("  • Proper parameter validation");
    println!("  • Error handling for invalid methods and parameters");
}