//! Middleware system for MCP servers
//!
//! This module provides a trait-based middleware architecture that allows intercepting
//! and modifying MCP requests and responses. Middleware can be used for authentication,
//! logging, rate limiting, and custom business logic.
//!
//! # Overview
//!
//! The middleware system consists of:
//! - [`McpMiddleware`] - Core trait for implementing middleware
//! - [`RequestContext`] - Normalized request context across transports
//! - [`SessionInjection`] - Write-only mechanism for populating session state
//! - [`MiddlewareStack`] - Ordered execution of multiple middleware layers
//!
//! # Examples
//!
//! ```rust,no_run
//! use turul_http_mcp_server::middleware::{McpMiddleware, RequestContext, SessionInjection, MiddlewareError};
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

pub mod traits;
pub mod context;
pub mod error;
pub mod stack;
pub mod builtins;
pub mod session_view_adapter;

pub use traits::McpMiddleware;
pub use context::{RequestContext, SessionInjection, DispatcherResult};
pub use error::{MiddlewareError, error_codes};
pub use stack::MiddlewareStack;
pub use session_view_adapter::StorageBackedSessionView;
