//! Build [Model Context Protocol](https://modelcontextprotocol.io) servers in Rust
//! with a few lines of code.
//!
//! Write a tool as an ordinary `async fn`, register it, and run. The framework
//! derives the JSON schema, the method names and the wire format from your types.
//! This crate is the entry point: it re-exports the macros and everything a
//! server needs, so most projects depend on it and `tokio` alone.
//!
//! # Example
//!
//! ```toml
//! [dependencies]
//! turul-mcp-server = "0.4"
//! tokio = { version = "1", features = ["full"] }
//! ```
//!
//! ```rust,no_run
//! use turul_mcp_server::prelude::*;
//!
//! #[mcp_tool(name = "add", description = "Add two numbers")]
//! async fn add(a: f64, b: f64) -> McpResult<f64> {
//!     Ok(a + b)
//! }
//!
//! #[tokio::main]
//! async fn main() -> McpResult<()> {
//!     McpServer::builder()
//!         .name("calculator")
//!         .tool_fn(add)
//!         .build()?
//!         .run()
//!         .await
//! }
//! ```
//!
//! The server listens on `http://127.0.0.1:8000/mcp`. Change it with
//! [`McpServerBuilder::bind_address`] and [`McpServerBuilder::mcp_path`].
//!
//! For a tool that carries state or configuration, derive it on a struct:
//!
//! ```rust,no_run
//! use turul_mcp_server::prelude::*;
//!
//! #[derive(McpTool, Default)]
//! #[tool(name = "greet", description = "Greet someone")]
//! struct Greet {
//!     #[param(description = "Who to greet")]
//!     name: String,
//! }
//!
//! impl Greet {
//!     async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
//!         Ok(format!("Hello, {}!", self.name))
//!     }
//! }
//!
//! # async fn run() -> McpResult<()> {
//! McpServer::builder().name("greeter").tool(Greet::default()).build()?.run().await
//! # }
//! ```
//!
//! # Ways to define a tool
//!
//! | Approach | Use when |
//! |---|---|
//! | `#[mcp_tool]` on an `async fn` | Most tools. Least code. |
//! | `#[derive(McpTool)]` on a struct | The tool holds state or config. |
//! | [`ToolBuilder`] | Tools defined at runtime. |
//! | Implement [`McpTool`] by hand | Full control. |
//!
//! Resources and prompts follow the same pattern with `#[mcp_resource]`,
//! `#[derive(McpResource)]` and `#[derive(McpPrompt)]`.
//!
//! # Features
//!
//! | Feature | Default | Enables |
//! |---|---|---|
//! | `http` | yes | Streamable HTTP transport |
//! | `sse` | yes | Server-sent event streams |
//! | `protocol-2026-07-28` | yes | MCP 2026-07-28 (stateless core) |
//! | `protocol-2025-11-25` | no | MCP 2025-11-25 sessions; use `--no-default-features --features http,sse,protocol-2025-11-25` |
//! | `dynamic-tools` | no | Add and remove tools while running |
//! | `sqlite`, `postgres`, `dynamodb` | no | Storage backends for `dynamic-tools` and 2025-11-25 sessions |
//! | `ext-tasks` | no | The `io.modelcontextprotocol/tasks` extension |
//!
//! # Where next
//!
//! - [`McpServerBuilder`] for every server option
//! - [`turul-mcp-aws-lambda`](https://docs.rs/turul-mcp-aws-lambda) to run on AWS Lambda
//! - [`turul-mcp-client`](https://docs.rs/turul-mcp-client) to call MCP servers
//! - [Examples](https://github.com/aussierobots/turul-mcp-framework/tree/main/examples)

extern crate self as turul_mcp_server;

pub mod builder;
pub mod cancellation;
pub mod completion;
#[cfg(feature = "protocol-2025-11-25")]
pub mod elicitation;
pub mod handlers;
#[cfg(feature = "protocol-2025-11-25")]
pub mod logging;
pub mod middleware;
pub mod notifications;
pub mod prompt;
pub mod resource;
pub mod roots;
#[cfg(feature = "protocol-2025-11-25")]
pub mod sampling;
pub mod server;
pub mod session;
#[cfg(feature = "protocol-2025-11-25")]
pub mod task;
pub mod tool;
#[cfg(feature = "dynamic-tools")]
pub mod tool_registry;
// Re-export session storage from separate crate (breaks circular dependency)
pub use turul_mcp_session_storage as session_storage;
// Re-export task storage from separate crate
#[cfg(feature = "protocol-2025-11-25")]
pub use turul_mcp_task_storage as task_storage;
pub mod dispatch;
#[cfg(feature = "ext-tasks")]
pub mod ext_tasks;
pub mod prelude;
pub mod security;
pub mod uri_template;

#[cfg(feature = "http")]
pub mod http;

/// Crates that macro-generated code names; lets users depend on this crate alone.
/// Not a public API.
#[doc(hidden)]
pub mod __private {
    pub use ::async_trait;
    pub use ::serde;
    pub use ::serde_json;
    pub use ::turul_mcp_builders;
    pub use ::turul_mcp_derive;
    pub use ::turul_mcp_protocol;
}

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
#[cfg(feature = "protocol-2025-11-25")]
pub use elicitation::McpElicitation;
/// Collection of built-in MCP request handlers
pub use handlers::*;
/// Logging provider for structured application logs
#[cfg(feature = "protocol-2025-11-25")]
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
#[cfg(feature = "protocol-2025-11-25")]
pub use sampling::McpSampling;
/// Security middleware and access control components
pub use security::{
    AccessLevel, InputValidator, RateLimitConfig, ResourceAccessControl, SecurityMiddleware,
};
/// Handler for the 2026-07-28 `server/discover` method
#[cfg(feature = "protocol-2026-07-28")]
pub use server::DiscoverHandler;
/// Core MCP server and session-aware handlers
#[cfg(feature = "protocol-2025-11-25")]
pub use server::SessionAwareInitializeHandler;
/// Core MCP server and session-aware handlers
pub use server::{
    ListToolsHandler, McpServer, SessionAwareMcpHandlerBridge, SessionAwareToolHandler,
};
/// Session management and context for stateful operations
pub use session::{SessionContext, SessionEvent, SessionEventDispatcher, SessionManager};
/// Task executor abstraction for pluggable execution backends
#[cfg(feature = "protocol-2025-11-25")]
pub use task::executor::{TaskExecutor, TaskHandle};
/// Task handlers for tasks/get, tasks/list, tasks/cancel, tasks/result
#[cfg(feature = "protocol-2025-11-25")]
pub use task::handlers::{
    TasksCancelHandler, TasksGetHandler, TasksListHandler, TasksResultHandler,
};
/// Task runtime for managing long-running operations
#[cfg(feature = "protocol-2025-11-25")]
pub use task::runtime::TaskRuntime;
/// Default Tokio-based task executor
#[cfg(feature = "protocol-2025-11-25")]
pub use task::tokio_executor::TokioTaskExecutor;
/// Tool trait for executable MCP functions
pub use tool::McpTool;
/// Stable fingerprint of the registered tool set for session versioning
pub use tool::compute_tool_fingerprint;
/// Runtime tool activation mode — controls fingerprint vs live notification behavior
#[cfg(feature = "dynamic-tools")]
pub use tool_registry::ToolRegistry;

/// Configuration for how tool changes are detected and communicated.
///
/// `Static` (default): no change detection, no fingerprint, no notifications.
/// `Dynamic`: fingerprint tracking + live notifications (`listChanged=true`).
/// Fingerprint check only runs when `listChanged=true`.
///
/// When `Dynamic` is used with `.server_state_storage()`, cross-instance
/// coordination is enabled (polling on EC2, request-time check on Lambda).
/// Without explicit storage, an in-memory backend is used automatically.
#[derive(Debug, Clone, Default)]
pub enum ToolChangeMode {
    /// Static tools. `listChanged=false`. No fingerprint check. No notifications.
    /// Tools are fixed at build time. This is the default.
    #[default]
    Static,
    /// Dynamic tool registry. `listChanged=true`.
    /// Precompiled tools can be activated/deactivated at runtime.
    /// Fingerprint tracks tool-version per session.
    /// Connected clients receive `notifications/tools/list_changed`.
    ///
    /// When paired with `.server_state_storage()`, enables multi-instance
    /// coordination via shared `ServerStateStorage` (DynamoDB/PostgreSQL/SQLite).
    #[cfg(feature = "dynamic-tools")]
    Dynamic,
}

/// SessionView trait for middleware - re-exported from turul-mcp-session-storage
pub use turul_mcp_session_storage::SessionView;

// Re-export foundational types
/// Core MCP protocol types, errors, and specification compliance
pub use turul_mcp_protocol::*;
/// JSON-RPC 2.0 request dispatcher and handler trait for protocol operations
pub use turul_rpc::{JsonRpcDispatcher, JsonRpcHandler};

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
