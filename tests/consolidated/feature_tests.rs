//! Consolidated feature test suite.
//!
//! Groups: MCP 2025-11-25 features, notification payloads, derive macro bugs,
//! runtime capabilities, framework integration, calculator levels

#[path = "../mcp_2025_11_25_features.rs"]
mod mcp_2025_11_25_features;

#[path = "../notification_payload_correctness.rs"]
mod notification_payload_correctness;

#[path = "../mcp_derive_macro_bug_detection.rs"]
mod mcp_derive_macro_bug_detection;

#[path = "../mcp_runtime_capability_validation.rs"]
mod mcp_runtime_capability_validation;

#[path = "../framework_integration_tests.rs"]
mod framework_integration_tests;

#[path = "../calculator_levels_integration.rs"]
mod calculator_levels_integration;

#[path = "../session_id_compliance.rs"]
mod session_id_compliance;
