//! # MCP Server Framework
//!
//! A production-ready Rust framework for building Model Context Protocol (MCP) servers.
//! Provides zero-configuration setup, comprehensive MCP 2025-11-25 specification support,
//! and multiple deployment targets including HTTP, AWS Lambda, and local development.
//!
//! [![Crates.io](https://img.shields.io/crates/v/turul-mcp-server.svg)](https://crates.io/crates/turul-mcp-server)
//! [![Documentation](https://docs.rs/turul-mcp-server/badge.svg)](https://docs.rs/turul-mcp-server)
//! [![License](https://img.shields.io/crates/l/turul-mcp-server.svg)](https://github.com/aussierobots/turul-mcp-framework/blob/main/LICENSE)
//!
//! ## Features
//!
//! - **Zero Configuration**: Framework auto-determines method strings from types
//! - **Type-Safe Error Handling**: Clean domain/protocol separation
//! - **4 Tool Creation Levels**: Function macros → derive macros → builders → manual
//! - **Multiple Transports**: HTTP, Server-Sent Events (SSE), AWS Lambda
//! - **Pluggable Storage**: InMemory, SQLite, PostgreSQL, DynamoDB
//! - **Real-time Streaming**: SSE notifications for progress and logging
//! - **Production Ready**: Comprehensive testing, monitoring, and deployment support
//!
//! ## Installation
//!
//! Add to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! turul-mcp-server = "0.2"
//! turul-mcp-derive = "0.2"  # For macros
//! tokio = { version = "1.0", features = ["full"] }
//! ```
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! # use turul_mcp_server::prelude::*;
//! #
//! # async fn example() -> McpResult<()> {
//! // Create a basic MCP server with detailed configuration
//! let server = McpServer::builder()
//!     .name("calculator-server")
//!     .version("1.0.0")
//!     .title("Advanced Calculator Server")
//!     .instructions("A production-grade calculator server supporting basic arithmetic operations including addition, subtraction, multiplication, and division. Use the available tools to perform calculations. The server maintains session state for calculation history, supports real-time notifications for long-running computations, and provides detailed error reporting for invalid operations.")
//!     .build()?;
//!
//! server.run().await
//! # }
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
//!
//! ## Examples
//!
//! **Complete working examples available at:**
//! [github.com/aussierobots/turul-mcp-framework/tree/main/examples](https://github.com/aussierobots/turul-mcp-framework/tree/main/examples)
//!
//! - **Minimal Server** - Basic tool setup
//! - **Calculator** - Math operations with error handling
//! - **HTTP Server** - Production HTTP deployment
//! - **AWS Lambda** - Serverless deployment
//! - **Real-time Streaming** - SSE notifications
//! - **Database Integration** - SQLite/PostgreSQL/DynamoDB
//!
//! ## Deployment Options
//!
//! ### Local Development
//! ```bash
//! cargo run --example minimal-server
//! # Server runs on http://localhost:8080/mcp
//! ```
//!
//! ### AWS Lambda
//! ```bash
//! cargo lambda build --release
//! cargo lambda deploy --iam-role arn:aws:iam::...
//! ```
//!
//! ### Docker
//! ```dockerfile
//! FROM rust:1.70 as builder
//! COPY . .
//! RUN cargo build --release
//!
//! FROM debian:bookworm-slim
//! COPY --from=builder /target/release/my-mcp-server /usr/local/bin/
//! EXPOSE 8080
//! CMD ["my-mcp-server"]
//! ```
//!
//! ## Configuration
//!
//! The framework supports extensive configuration through the builder pattern:
//!
//! ```rust,no_run
//! # use turul_mcp_server::prelude::*;
//! # use turul_mcp_session_storage::SqliteSessionStorage;
//! # use std::time::Duration;
//! #
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let server = McpServer::builder()
//!     .name("production-server")
//!     .version("1.0.0")
//!     .title("Production MCP Server")
//!     .instructions("This production server provides tools for database operations, file management, and API integrations. Use the 'database/query' tool for SQL operations, 'files/read' for file access, and 'api/call' for external service integration. Session management is enabled with SQLite persistence for reliability.")
//!     .with_session_storage(std::sync::Arc::new(SqliteSessionStorage::new().await?))
//!     .build()?;
//! # Ok(())
//! # }
//! ```

pub mod builder;
pub mod cancellation;
pub mod completion;
pub mod elicitation;
pub mod handlers;
pub mod logging;
pub mod middleware;
pub mod notifications;
pub mod prompt;
pub mod resource;
pub mod roots;
pub mod sampling;
pub mod server;
pub mod session;
pub mod task;
pub mod tool;
// Re-export session storage from separate crate (breaks circular dependency)
pub use turul_mcp_session_storage as session_storage;
// Re-export task storage from separate crate
pub use turul_mcp_task_storage as task_storage;
pub mod dispatch;
pub mod prelude;
pub mod security;
pub mod uri_template;

#[cfg(feature = "http")]
pub mod http;

#[cfg(test)]
mod tests;

// Re-export main types
/// Builder for creating MCP servers with fluent API
pub use builder::McpServerBuilder;
/// Cancellation handle for cooperative task cancellation
pub use cancellation::CancellationHandle;
/// Completion provider for text generation requests
pub use completion::McpCompletion;
/// Request dispatching and middleware support for MCP operations
pub use dispatch::{DispatchContext, DispatchMiddleware, McpDispatcher};
/// Elicitation handler for interactive form-based data collection
pub use elicitation::McpElicitation;
/// Collection of built-in MCP request handlers
pub use handlers::*;
/// Logging provider for structured application logs
pub use logging::McpLogger;
/// Notification system for real-time client updates via SSE
pub use notifications::McpNotification;
/// Prompt provider for generating conversation templates
pub use prompt::McpPrompt;
/// Resource provider for serving file-like content with URI templates
pub use resource::McpResource;
/// Root provider for workspace and project context
pub use roots::McpRoot;
/// Sampling configuration for LLM inference parameters
pub use sampling::McpSampling;
/// Security middleware and access control components
pub use security::{
    AccessLevel, InputValidator, RateLimitConfig, ResourceAccessControl, SecurityMiddleware,
};
/// Core MCP server and session-aware handlers
pub use server::{
    ListToolsHandler, McpServer, SessionAwareInitializeHandler, SessionAwareMcpHandlerBridge,
    SessionAwareToolHandler,
};
/// Session management and context for stateful operations
pub use session::{SessionContext, SessionEvent, SessionManager};
/// Task executor abstraction for pluggable execution backends
pub use task::executor::{TaskExecutor, TaskHandle};
/// Task handlers for tasks/get, tasks/list, tasks/cancel, tasks/result
pub use task::handlers::{
    TasksCancelHandler, TasksGetHandler, TasksListHandler, TasksResultHandler,
};
/// Task runtime for managing long-running operations
pub use task::runtime::TaskRuntime;
/// Default Tokio-based task executor
pub use task::tokio_executor::TokioTaskExecutor;
/// Tool trait for executable MCP functions
pub use tool::McpTool;
/// SessionView trait for middleware - re-exported from turul-mcp-session-storage
pub use turul_mcp_session_storage::SessionView;

// Re-export foundational types
/// JSON-RPC 2.0 request dispatcher and handler trait for protocol operations
pub use turul_mcp_json_rpc_server::{JsonRpcDispatcher, JsonRpcHandler};
/// Core MCP protocol types, errors, and specification compliance
pub use turul_mcp_protocol::*;

// Re-export builder pattern for Level 3 tool creation
/// Dynamic tool creation with runtime configuration and type-safe builders
pub use turul_mcp_builders::tool::{DynamicTool, DynamicToolFn, ToolBuilder};

// Explicitly re-export error types for convenience
/// Domain error type for MCP operations with protocol conversion support
pub use turul_mcp_protocol::{McpError, McpResult as ProtocolMcpResult};

#[cfg(feature = "http")]
/// HTTP transport layer with SSE streaming and session management
pub use turul_http_mcp_server;

/// Result type for MCP server operations with domain-specific error handling
///
/// This alias provides structured error types that automatically convert to JSON-RPC 2.0
/// error responses when crossing the protocol boundary. Use this for all tool and handler
/// implementations to ensure consistent error reporting to MCP clients.
pub type McpResult<T> = turul_mcp_protocol::McpResult<T>;

/// Convenience alias for McpResult
pub type Result<T> = McpResult<T>;

/// Implements McpTool for DynamicTool to enable Level 3 builder pattern tool creation
///
/// This implementation bridges DynamicTool's builder pattern with the framework's
/// session-aware execution model, enabling runtime tool construction with type safety.
#[async_trait::async_trait]
impl McpTool for DynamicTool {
    async fn call(
        &self,
        args: serde_json::Value,
        _session: Option<SessionContext>,
    ) -> McpResult<turul_mcp_protocol::tools::CallToolResult> {
        use turul_mcp_builders::prelude::HasOutputSchema;
        use turul_mcp_protocol::tools::CallToolResult;

        match self.execute(args).await {
            Ok(result) => {
                // Use smart response builder with automatic structured content
                CallToolResult::from_result_with_schema(&result, self.output_schema())
            }
            Err(e) => Err(McpError::tool_execution(&e)),
        }
    }
}
