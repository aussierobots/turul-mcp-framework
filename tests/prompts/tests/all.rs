//! Consolidated prompts integration test suite.

#[path = "e2e_integration.rs"]
mod e2e_integration;

#[path = "e2e_shared_integration.rs"]
mod e2e_shared_integration;

#[path = "mcp_prompts_protocol_coverage.rs"]
mod mcp_prompts_protocol_coverage;

#[path = "mcp_prompts_specification.rs"]
mod mcp_prompts_specification;

#[path = "prompts_arguments_validation.rs"]
mod prompts_arguments_validation;

#[path = "prompts_endpoints_integration.rs"]
mod prompts_endpoints_integration;

#[path = "prompts_notifications.rs"]
mod prompts_notifications;

#[path = "sse_notifications_test.rs"]
mod sse_notifications_test;
