//! Consolidated compliance test suite.
//!
//! Groups: mcp_compliance_tests, mcp_specification_compliance,
//! mcp_behavioral_compliance, mcp_tool_compliance

#[path = "../mcp_compliance_tests.rs"]
mod mcp_compliance_tests;

#[path = "../mcp_specification_compliance.rs"]
mod mcp_specification_compliance;

#[path = "../mcp_behavioral_compliance.rs"]
mod mcp_behavioral_compliance;

#[path = "../mcp_tool_compliance.rs"]
mod mcp_tool_compliance;
