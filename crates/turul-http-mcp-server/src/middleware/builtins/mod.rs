//! Built-in middleware implementations
//!
//! This module provides production-ready middleware for common use cases:
//!
//! - **Logging**: Request/response logging with duration tracking
//! - **Authentication**: API key and token validation
//! - **Rate Limiting**: Token bucket rate limiting per session
//!
//! # Examples
//!
//! ```rust,ignore
//! use turul_mcp_server::prelude::*;
//! use turul_mcp_server::middleware::builtins::LoggingMiddleware;
//!
//! let server = McpServer::builder()
//!     .name("my-server")
//!     .middleware(LoggingMiddleware::new())
//!     .build()?;
//! ```

// Placeholder module: no built-in middleware ships yet.
// For now, this module serves as a placeholder
