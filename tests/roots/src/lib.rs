//! # MCP Roots Protocol E2E Tests
//!
//! Comprehensive end-to-end tests for MCP roots protocol implementation.
//! Tests the roots/list endpoint and file system access control.

pub mod test_utils;

// Re-export common dependencies for tests
pub use mcp_e2e_shared::{McpTestClient, TestFixtures, TestServerManager};
pub use serde_json::{Value, json};
pub use tracing::{debug, info, warn};
