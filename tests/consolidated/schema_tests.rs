//! Consolidated schema test suite.
//!
//! Groups: tool schema sync, vec result schemas, explicit vec output,
//! badly named tools, output field consistency, acronym output, custom output,
//! schemars detailed/optional, schemars derive

#[path = "../mcp_tool_schema_runtime_sync_test.rs"]
mod mcp_tool_schema_runtime_sync_test;

#[path = "../mcp_vec_result_schema_test.rs"]
mod mcp_vec_result_schema_test;

#[path = "../mcp_vec_result_runtime_schema_test.rs"]
mod mcp_vec_result_runtime_schema_test;

#[path = "../mcp_explicit_vec_output_test.rs"]
mod mcp_explicit_vec_output_test;

#[path = "../mcp_vec_badly_named_tool_test.rs"]
mod mcp_vec_badly_named_tool_test;

#[path = "../output_field_consistency_test.rs"]
mod output_field_consistency_test;

#[path = "../acronym_output_field_test.rs"]
mod acronym_output_field_test;

#[path = "../custom_output_field_test.rs"]
mod custom_output_field_test;

#[path = "../schemars_detailed_schema_test.rs"]
mod schemars_detailed_schema_test;

#[path = "../schemars_optional_fields_test.rs"]
mod schemars_optional_fields_test;

#[path = "../test_schemars_derive.rs"]
mod test_schemars_derive;
