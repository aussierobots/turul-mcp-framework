use mcp_json_rpc_server::{
    JsonRpcHandler, JsonRpcDispatcher, RequestParams,
    r#async::JsonRpcResult, dispatch::parse_json_rpc_message,
};
use async_trait::async_trait;
use serde_json::{json, Value};

struct CalculatorHandler;

#[async_trait]
impl JsonRpcHandler for CalculatorHandler {
    async fn handle(&self, method: &str, params: Option<RequestParams>) -> JsonRpcResult<Value> {
        match method {
            "add" => {
                let params = params.ok_or_else(|| {
                    mcp_json_rpc_server::error::JsonRpcProcessingError::HandlerError(
                        "Missing parameters".to_string()
                    )
                })?;
                
                let map = params.to_map();
                let a = map.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let b = map.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
                
                Ok(json!({"result": a + b}))
            }
            "subtract" => {
                let params = params.ok_or_else(|| {
                    mcp_json_rpc_server::error::JsonRpcProcessingError::HandlerError(
                        "Missing parameters".to_string()
                    )
                })?;
                
                let map = params.to_map();
                let a = map.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let b = map.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
                
                Ok(json!({"result": a - b}))
            }
            _ => Err(mcp_json_rpc_server::error::JsonRpcProcessingError::HandlerError(
                format!("Unknown method: {}", method)
            ))
        }
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["add".to_string(), "subtract".to_string()]
    }
}

#[tokio::main]
async fn main() {
    println!("Testing JSON-RPC Server Framework");
    
    // Create dispatcher and register handler
    let mut dispatcher = JsonRpcDispatcher::new();
    dispatcher.register_methods(
        vec!["add".to_string(), "subtract".to_string()],
        CalculatorHandler,
    );

    // Test request JSON
    let request_json = r#"{"jsonrpc": "2.0", "method": "add", "params": {"a": 5, "b": 3}, "id": 1}"#;
    
    println!("Processing request: {}", request_json);
    
    // Parse message
    match parse_json_rpc_message(request_json) {
        Ok(mcp_json_rpc_server::dispatch::JsonRpcMessage::Request(request)) => {
            println!("Parsed request: method={}, id={:?}", request.method, request.id);
            
            // Handle request
            let response = dispatcher.handle_request(request).await;
            
            // Serialize response
            match serde_json::to_string_pretty(&response) {
                Ok(response_json) => {
                    println!("Response:");
                    println!("{}", response_json);
                }
                Err(e) => println!("Failed to serialize response: {}", e),
            }
        }
        Ok(mcp_json_rpc_server::dispatch::JsonRpcMessage::Notification(_)) => {
            println!("Received notification (no response needed)");
        }
        Err(e) => {
            println!("Parse error: {}", e);
        }
    }
}