//! # MCP Server Framework
//!
//! A high-level framework for building Model Context Protocol (MCP) servers in Rust.
//! This framework provides a simple, builder-pattern API for creating MCP servers
//! with HTTP transport and comprehensive tool support.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use mcp_server::{McpServer, McpTool, SessionContext};
//! use mcp_protocol::{ToolSchema, ToolResult, schema::JsonSchema};
//! use serde_json::json;
//! use async_trait::async_trait;
//!
//! struct EchoTool;
//!
//! #[async_trait]
//! impl McpTool for EchoTool {
//!     fn name(&self) -> &str { "echo" }
//!     fn description(&self) -> &str { "Echo back the input text" }
//!     
//!     fn input_schema(&self) -> ToolSchema {
//!         ToolSchema::object()
//!             .with_properties(std::collections::HashMap::from([
//!                 ("text".to_string(), JsonSchema::string())
//!             ]))
//!             .with_required(vec!["text".to_string()])
//!     }
//!     
//!     async fn call(&self, args: serde_json::Value, _session: Option<SessionContext>) -> crate::McpResult<Vec<ToolResult>> {
//!         let text = args.get("text")
//!             .and_then(|v| v.as_str())
//!             .unwrap_or("No text provided");
//!         
//!         Ok(vec![ToolResult::text(format!("Echo: {}", text))])
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
//!     let server = McpServer::builder()
//!         .name("echo-server")
//!         .version("1.0.0")
//!         .tool(EchoTool)
//!         .build()?;
//!     
//!     // For HTTP transport, use:
//!     // server.run_http("127.0.0.1:8000".parse()?).await?;
//!     
//!     server.run().await?;
//!     Ok(())
//! }
//! ```

pub mod builder;
pub mod tool;
pub mod server;
pub mod handlers;
pub mod session;
pub mod dispatch;

#[cfg(feature = "http")]
pub mod http;

#[cfg(test)]
mod tests;

// Re-export main types
pub use builder::McpServerBuilder;
pub use tool::McpTool;
pub use server::McpServer;
pub use handlers::*;
pub use session::{SessionContext, SessionManager, SessionEvent};
pub use dispatch::{McpDispatcher, DispatchMiddleware, DispatchContext};

// Re-export foundational types
pub use json_rpc_server::{JsonRpcHandler, JsonRpcDispatcher};
pub use mcp_protocol::*;

// Explicitly re-export error types for convenience
pub use mcp_protocol::{McpError, McpResult as ProtocolMcpResult};

#[cfg(feature = "http")]
pub use http_mcp_server;

/// Result type for framework operations
pub type Result<T> = std::result::Result<T, McpFrameworkError>;

/// Result type for tool operations - uses structured MCP errors
pub type McpResult<T> = mcp_protocol::McpResult<T>;

/// Framework-level errors
#[derive(Debug, thiserror::Error)]
pub enum McpFrameworkError {
    #[error("JSON-RPC error: {0}")]
    JsonRpc(#[from] json_rpc_server::JsonRpcError),
    
    #[error("MCP protocol error: {0}")]
    Mcp(#[from] mcp_protocol::McpError),
    
    #[cfg(feature = "http")]
    #[error("HTTP transport error: {0}")]
    Http(#[from] http_mcp_server::HttpMcpError),
    
    #[error("Tool error: {0}")]
    Tool(String),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
