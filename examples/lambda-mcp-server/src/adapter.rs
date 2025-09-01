//! Simple adapter for Lambda MCP Server
//!
//! Instead of converting between lambda_http and hyper types,
//! we'll implement a simplified handler that works directly with lambda_http

use lambda_http::{Request as LambdaRequest, Response as LambdaResponse, Body as LambdaBody, Error};
use std::sync::Arc;
use tracing::{debug, info};
use serde_json::json;

use turul_http_mcp_server::SessionMcpHandler;

/// Simplified adapter that handles MCP requests directly
pub async fn lambda_adapter(
    lambda_req: LambdaRequest,
    _handler: Arc<SessionMcpHandler>,
) -> Result<LambdaResponse<LambdaBody>, Error> {
    let method = lambda_req.method().clone();
    let uri = lambda_req.uri().clone();
    
    info!("🌐 Lambda MCP request: {} {}", method, uri);

    // For now, return a simple MCP initialization response
    match (method.as_str(), uri.path()) {
        ("POST", "/mcp") => {
            debug!("📞 Handling MCP POST request");
            
            // Return a basic MCP initialize response
            let response_body = json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": {
                    "protocolVersion": "2025-06-18",
                    "capabilities": {
                        "tools": {
                            "dynamodb_query": {},
                            "sns_publish": {},
                            "sqs_send_message": {},
                            "cloudwatch_metrics": {}
                        }
                    },
                    "serverInfo": {
                        "name": "lambda-mcp-server",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                }
            });
            
            let response = LambdaResponse::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .header("Mcp-Session-Id", uuid::Uuid::new_v4().to_string())
                .body(LambdaBody::Text(response_body.to_string()))?;
                
            debug!("✅ Returning MCP initialize response");
            Ok(response)
        },
        ("OPTIONS", _) => {
            debug!("📞 Handling CORS preflight");
            
            let response = LambdaResponse::builder()
                .status(200)
                .header("Access-Control-Allow-Origin", "*")
                .header("Access-Control-Allow-Methods", "GET, POST, DELETE, OPTIONS")
                .header("Access-Control-Allow-Headers", "Content-Type, Accept, Mcp-Session-Id")
                .body(LambdaBody::Empty)?;
                
            Ok(response)
        },
        _ => {
            debug!("📞 Unsupported method/path: {} {}", method, uri.path());
            
            let error_body = json!({
                "jsonrpc": "2.0", 
                "id": null,
                "error": {
                    "code": -32601,
                    "message": "Method not found",
                    "data": format!("{} {} is not supported", method, uri.path())
                }
            });
            
            let response = LambdaResponse::builder()
                .status(404)
                .header("Content-Type", "application/json")
                .body(LambdaBody::Text(error_body.to_string()))?;
                
            Ok(response)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::Method;
    
    #[tokio::test]
    async fn test_options_request() {
        let lambda_req = LambdaRequest::builder()
            .method(Method::OPTIONS)
            .uri("https://example.com/mcp")
            .body(LambdaBody::Empty)
            .unwrap();
            
        // Create a dummy handler for testing  
        let session_handler = Arc::new(turul_http_mcp_server::SessionMcpHandler::new(
            turul_http_mcp_server::ServerConfig::default(),
            Arc::new(turul_mcp_json_rpc_server::JsonRpcDispatcher::new())
        ));
        
        let response = lambda_adapter(lambda_req, session_handler).await.unwrap();
        
        assert_eq!(response.status(), 200);
    }
}