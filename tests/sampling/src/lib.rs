//! # MCP Sampling Protocol E2E Tests
//!
//! Comprehensive end-to-end tests for MCP sampling protocol implementation.
//! Tests the actual sampling/createMessage endpoint with real sampling handlers.

pub mod test_utils;

// Re-export common dependencies for tests
pub use mcp_e2e_shared::{McpTestClient, TestServerManager, TestFixtures};
pub use serde_json::{json, Value};
pub use tracing::{debug, info, warn};