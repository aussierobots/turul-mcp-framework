//! # MCP Server Framework
//!
//! A high-level framework for building Model Context Protocol (MCP) servers in Rust.
//! This framework provides a zero-configuration, builder-pattern API for creating MCP servers
//! with HTTP transport, session management, and comprehensive MCP 2025-06-18 specification support.
//!
//! ## Features
//!
//! - **Zero Configuration**: Framework auto-determines all method strings from types
//! - **Unified Error Handling**: Clean domain/protocol separation with thiserror
//! - **4 Tool Creation Levels**: Function macros, derive macros, builders, manual
//! - **Session Management**: Async UUID v7 sessions with pluggable storage
//! - **Real-time Notifications**: SSE streaming for progress and logging
//! - **Production Ready**: Type-safe, compliant, thoroughly tested
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use turul_mcp_server::prelude::*;
//! use turul_mcp_derive::McpTool;
//!
//! // Level 1: Function Tool (simplest)
//! #[mcp_tool(name = "add", description = "Add two numbers")]
//! async fn add(
//!     #[param(description = "First number")] a: f64,
//!     #[param(description = "Second number")] b: f64,
//! ) -> McpResult<f64> {
//!     Ok(a + b)
//! }
//!
//! // Level 2: Derive Tool (most common)
//! #[derive(McpTool)]
//! #[mcp(name = "echo", description = "Echo back the input text")]
//! struct EchoTool {
//!     #[param(description = "Text to echo")]
//!     text: String,
//! }
//!
//! impl EchoTool {
//!     async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
//!         Ok(format!("Echo: {}", self.text))
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() -> McpResult<()> {
//!     let server = McpServer::builder()
//!         .name("echo-server")
//!         .version("1.0.0")
//!         .tool(add)                    // Function tool
//!         .tool(EchoTool::default())    // Derive tool
//!         .build()?;
//!
//!     server.run().await
//! }
//! ```
//!
//! ## Architecture
//!
//! The framework uses **clean domain/protocol separation**:
//!
//! - **Domain Layer**: Tools return `McpResult<T>` with domain errors
//! - **Protocol Layer**: Framework converts to JSON-RPC 2.0 automatically
//! - **Transport Layer**: HTTP/SSE with session-aware error handling
//! - **Storage Layer**: Pluggable backends (InMemory, SQLite, PostgreSQL, DynamoDB)

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
pub mod prelude;
pub mod uri_template;
pub mod security;

#[cfg(feature = "http")]
pub mod http;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod uri_template_tests;

#[cfg(test)]
mod security_integration_tests;

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
pub use security::{SecurityMiddleware, RateLimitConfig, ResourceAccessControl, AccessLevel, InputValidator};

// Re-export foundational types
pub use turul_mcp_json_rpc_server::{JsonRpcHandler, JsonRpcDispatcher};
pub use turul_mcp_protocol::*;


// Re-export builder pattern for Level 3 tool creation
pub use turul_mcp_protocol::tools::builder::{ToolBuilder, DynamicTool};

// Explicitly re-export error types for convenience
pub use turul_mcp_protocol::{McpError, McpResult as ProtocolMcpResult};

#[cfg(feature = "http")]
pub use turul_http_mcp_server;

/// Result type for all MCP operations - uses structured MCP errors
pub type McpResult<T> = turul_mcp_protocol::McpResult<T>;

/// Convenience alias for McpResult
pub type Result<T> = McpResult<T>;

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
