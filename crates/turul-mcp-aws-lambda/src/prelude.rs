//! # AWS Lambda MCP Server Prelude
//!
//! This module provides convenient re-exports of the most commonly used types
//! from the AWS Lambda MCP server library.
//!
//! ```rust
//! use turul_mcp_aws_lambda::prelude::*;
//! ```

// Core Lambda types
pub use crate::builder::LambdaMcpServerBuilder;
pub use crate::error::{LambdaError, Result};
pub use crate::handler::LambdaMcpHandler;
pub use crate::server::LambdaMcpServer;

#[cfg(feature = "cors")]
pub use crate::cors::CorsConfig;

// Re-export MCP protocol types
pub use turul_mcp_protocol::prelude::*;

// Lambda runtime types commonly used
pub use lambda_http::{
    Error as LambdaHttpError, Request as LambdaRequest, Response as LambdaResponse,
};
