//! Middleware system for MCP servers
//!
//! Re-exports middleware types from the HTTP transport layer.
//! Middleware is implemented in `turul-http-mcp-server` where it's actually used.
//!
//! # Examples
//!
//! ```rust,no_run
//! use turul_mcp_server::prelude::*;
//! use turul_mcp_server::middleware::{McpMiddleware, RequestContext, SessionInjection, MiddlewareError};
//! use turul_mcp_session_storage::SessionView;
//! use async_trait::async_trait;
//!
//! // Define custom middleware
//! struct LoggingMiddleware;
//!
//! #[async_trait]
//! impl McpMiddleware for LoggingMiddleware {
//!     async fn before_dispatch(
//!         &self,
//!         ctx: &mut RequestContext<'_>,
//!         _session: Option<&dyn SessionView>,
//!         _injection: &mut SessionInjection,
//!     ) -> Result<(), MiddlewareError> {
//!         println!("Request: {}", ctx.method());
//!         Ok(())
//!     }
//! }
//! ```

// Re-export middleware types when HTTP feature is enabled
// When HTTP is disabled (e.g., Lambda-only builds), this module simply doesn't export anything
#[cfg(feature = "http")]
pub use turul_http_mcp_server::middleware::*;
