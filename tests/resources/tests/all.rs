//! Consolidated resources integration test suite.

#[path = "e2e_integration.rs"]
mod e2e_integration;

#[path = "e2e_shared_integration.rs"]
mod e2e_shared_integration;

#[path = "mcp_resources_protocol_coverage.rs"]
mod mcp_resources_protocol_coverage;

#[path = "mcp_resources_specification.rs"]
mod mcp_resources_specification;

#[path = "resource_templates_e2e.rs"]
mod resource_templates_e2e;

#[path = "sse_notifications_test.rs"]
mod sse_notifications_test;
