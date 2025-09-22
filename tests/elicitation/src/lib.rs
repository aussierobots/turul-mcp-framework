//! # MCP Elicitation Protocol E2E Tests
//!
//! Comprehensive end-to-end tests for MCP elicitation protocol implementation.
//! Tests elicitation tools and structured data collection workflows.

pub mod test_utils;

// Re-export common dependencies for tests
pub use mcp_e2e_shared::{McpTestClient, TestFixtures, TestServerManager};
pub use serde_json::{json, Value};
pub use tracing::{debug, info, warn};
