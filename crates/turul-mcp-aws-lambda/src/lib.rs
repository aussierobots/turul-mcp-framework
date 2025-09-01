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
//! use turul_mcp_server::McpTool;
//! use lambda_http::{run_with_streaming_response, service_fn, Error};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Error> {
//!     let handler = LambdaMcpServerBuilder::new()
//!         .tool(MyTool::default())
//!         .cors_allow_all_origins()
//!         .build()
//!         .await?;
//!     
//!     run_with_streaming_response(service_fn(move |req| {
//!         handler.clone().handle(req)
//!     })).await
//! }
//! ```

pub mod adapter;
pub mod handler;
pub mod builder;
pub mod server;
pub mod error;

#[cfg(feature = "cors")]
pub mod cors;

#[cfg(feature = "sse")]
pub mod streaming;

// Re-exports for convenience
pub use builder::LambdaMcpServerBuilder;
pub use handler::LambdaMcpHandler;
pub use server::LambdaMcpServer;
pub use error::{LambdaError, Result};

#[cfg(feature = "cors")]
pub use cors::CorsConfig;

#[cfg(feature = "sse")]
pub use streaming::adapt_sse_stream;