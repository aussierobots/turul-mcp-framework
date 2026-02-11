//! Consolidated tools integration test suite.

#[path = "e2e_integration.rs"]
mod e2e_integration;

#[path = "large_message_handling.rs"]
mod large_message_handling;

#[path = "mcp_error_code_coverage.rs"]
mod mcp_error_code_coverage;
