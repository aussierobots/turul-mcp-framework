//! AWS Lambda integration for turul-mcp-framework
//!
//! This crate provides seamless integration between the turul-mcp-framework and AWS Lambda,
//! enabling serverless deployment of MCP servers with proper session management, CORS handling,
//! and SSE streaming support.
//!
//! ## Architecture
//!
//! The crate bridges the gap between Lambda's HTTP execution model and the framework's
//! hyper-based architecture through:
//!
//! - **Type Conversion**: Clean conversion between `lambda_http` and `hyper` types
//! - **Handler Registration**: Direct tool registration with `JsonRpcDispatcher`
//! - **Session Management**: DynamoDB-backed session persistence across invocations
//! - **CORS Support**: Proper CORS header injection for browser clients
//! - **SSE Streaming**: Server-Sent Events adaptation through Lambda's streaming response
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use turul_mcp_aws_lambda::LambdaMcpServerBuilder;
//! use turul_mcp_derive::McpTool;
//! use turul_mcp_server::{McpResult, SessionContext};
//! use lambda_http::{run_with_streaming_response, service_fn, Error};
//!
//! #[derive(McpTool, Clone, Default)]
//! #[tool(name = "example", description = "Example tool")]
//! struct ExampleTool {
//!     #[param(description = "Example parameter")]
//!     value: String,
//! }
//!
//! impl ExampleTool {
//!     async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
//!         Ok(format!("Got: {}", self.value))
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Error> {
//!     let server = LambdaMcpServerBuilder::new()
//!         .tool(ExampleTool::default())
//!         .cors_allow_all_origins()
//!         .build()
//!         .await?;
//!
//!     let handler = server.handler().await?;
//!
//!     run_with_streaming_response(service_fn(move |req| {
//!         let handler = handler.clone();
//!         async move {
//!             handler.handle(req).await.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
//!         }
//!     })).await
//! }
//! ```

pub mod adapter;
pub mod builder;
pub mod error;
pub mod handler;
pub mod server;

#[cfg(feature = "cors")]
pub mod cors;

#[cfg(feature = "sse")]
pub mod streaming;


// Re-exports for convenience
/// Builder for creating Lambda MCP servers with fluent configuration API
pub use builder::LambdaMcpServerBuilder;
/// Lambda-specific error types and result aliases
pub use error::{LambdaError, Result};
/// Lambda request handler with session management and protocol conversion
pub use handler::LambdaMcpHandler;
/// Core Lambda MCP server implementation with DynamoDB integration
pub use server::LambdaMcpServer;

#[cfg(feature = "cors")]
pub use cors::CorsConfig;

