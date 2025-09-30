//! Shared E2E Testing Utilities for MCP Framework
//!
//! This crate provides common utilities, fixtures, and helpers for E2E testing
//! across different MCP components (resources, prompts, tools, etc.)

pub mod e2e_utils;

// Re-export the main types for convenience
pub use e2e_utils::{McpTestClient, SessionTestUtils, TestFixtures, TestServerManager};
