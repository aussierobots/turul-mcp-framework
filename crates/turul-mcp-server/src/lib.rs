//! # MCP Server Framework
//!
//! A high-level framework for building Model Context Protocol (MCP) servers in Rust.
//! This framework provides a simple, builder-pattern API for creating MCP servers
//! with HTTP transport and comprehensive tool support.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use turul_mcp_server::{McpServer, McpTool, SessionContext};
//! use turul_mcp_protocol::{CallToolResult, tools::*};
//! use serde_json::json;
//! use async_trait::async_trait;
//! use std::collections::HashMap;
//!
//! struct EchoTool {
//!     input_schema: ToolSchema,
//! }
//!
//! impl EchoTool {
//!     fn new() -> Self {
//!         let input_schema = ToolSchema::object()
//!             .with_properties(HashMap::from([
//!                 ("text".to_string(), turul_mcp_protocol::schema::JsonSchema::string())
//!             ]))
//!             .with_required(vec!["text".to_string()]);
//!         Self { input_schema }
//!     }
//! }
//!
//! // Implement fine-grained traits for complete ToolDefinition
//! impl HasBaseMetadata for EchoTool {
//!     fn name(&self) -> &str { "echo" }
//! }
//!
//! impl HasDescription for EchoTool {
//!     fn description(&self) -> Option<&str> { Some("Echo back the input text") }
//! }
//!
//! impl HasInputSchema for EchoTool {
//!     fn input_schema(&self) -> &ToolSchema { &self.input_schema }
//! }
//!
//! impl HasOutputSchema for EchoTool {
//!     fn output_schema(&self) -> Option<&ToolSchema> { None }
//! }
//!
//! impl HasAnnotations for EchoTool {
//!     fn annotations(&self) -> Option<&ToolAnnotations> { None }
//! }
//!
//! impl HasToolMeta for EchoTool {
//!     fn tool_meta(&self) -> Option<&HashMap<String, serde_json::Value>> { None }
//! }
//!
//! // ToolDefinition automatically implemented via trait composition!
//!
//! #[async_trait]
//! impl McpTool for EchoTool {
//!     async fn call(&self, args: serde_json::Value, _session: Option<SessionContext>) -> crate::McpResult<CallToolResult> {
//!         let text = args.get("text")
//!             .and_then(|v| v.as_str())
//!             .unwrap_or("No text provided");
//!         
//!         Ok(CallToolResult::from_text(format!("Echo: {}", text)))
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
//!     let server = McpServer::builder()
//!         .name("echo-server")
//!         .version("1.0.0")
//!         .tool(EchoTool::new())
//!         .build()?;
//!     
//!     server.run().await?;
//!     Ok(())
//! }
//! ```

pub mod builder;
pub mod tool;
pub mod resource;
pub mod elicitation;
pub mod prompt;
pub mod sampling;
pub mod completion;
pub mod logging;
pub mod roots;
pub mod notifications;
pub mod server;
pub mod handlers;
pub mod session;
// Re-export session storage from separate crate (breaks circular dependency)
pub use turul_mcp_session_storage as session_storage;
pub mod dispatch;

#[cfg(feature = "http")]
pub mod http;

#[cfg(test)]
mod tests;

// Re-export main types
pub use builder::McpServerBuilder;
pub use tool::McpTool;
pub use resource::McpResource;
pub use elicitation::McpElicitation;
pub use prompt::McpPrompt;
pub use sampling::McpSampling;
pub use completion::McpCompletion;
pub use logging::McpLogger;
pub use roots::McpRoot;
pub use notifications::McpNotification;
pub use server::{McpServer, SessionAwareInitializeHandler, SessionAwareToolHandler, ListToolsHandler, SessionAwareMcpHandlerBridge};
pub use handlers::*;
pub use session::{SessionContext, SessionManager, SessionEvent};
pub use dispatch::{McpDispatcher, DispatchMiddleware, DispatchContext};

// Re-export foundational types
pub use turul_mcp_json_rpc_server::{JsonRpcHandler, JsonRpcDispatcher};
pub use turul_mcp_protocol::*;

// Re-export builder pattern for Level 3 tool creation
pub use turul_mcp_protocol::tools::builder::{ToolBuilder, DynamicTool};

// Explicitly re-export error types for convenience
pub use turul_mcp_protocol::{McpError, McpResult as ProtocolMcpResult};

#[cfg(feature = "http")]
pub use turul_http_mcp_server;

/// Result type for framework operations
pub type Result<T> = std::result::Result<T, McpFrameworkError>;

/// Result type for tool operations - uses structured MCP errors
pub type McpResult<T> = turul_mcp_protocol::McpResult<T>;

/// Framework-level errors
#[derive(Debug, thiserror::Error)]
pub enum McpFrameworkError {
    #[error("JSON-RPC error: {0}")]
    JsonRpc(#[from] turul_mcp_json_rpc_server::JsonRpcError),
    
    #[error("MCP protocol error: {0}")]
    Mcp(#[from] turul_mcp_protocol::McpError),
    
    #[cfg(feature = "http")]
    #[error("HTTP transport error: {0}")]
    Http(#[from] turul_http_mcp_server::HttpMcpError),
    
    #[error("Tool error: {0}")]
    Tool(String),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// Implement McpTool for DynamicTool (Level 3 builder pattern)
#[async_trait::async_trait]
impl McpTool for DynamicTool {
    async fn call(&self, args: serde_json::Value, _session: Option<SessionContext>) -> McpResult<turul_mcp_protocol::tools::CallToolResult> {
        use turul_mcp_protocol::tools::{HasOutputSchema, CallToolResult};

        match self.execute(args).await {
            Ok(result) => {
                // Use smart response builder with automatic structured content
                CallToolResult::from_result_with_schema(&result, self.output_schema())
            }
            Err(e) => Err(McpError::tool_execution(&e))
        }
    }
}
