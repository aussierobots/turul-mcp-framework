//! INDEPENDENT VERIFICATION (agent != implementer) of R1 + R3.
//!
//! Drives the REAL server build path — `McpServer::builder()...build()` at
//! crates/turul-mcp-server/src/builder.rs:1672-1682 — with adversarial
//! inputs DIFFERENT from the implementer's `schema_validation_2026.rs`
//! (which only exercised a single `{"type":123}` malformed property).
#![cfg(feature = "protocol-2026-07-28")]

use async_trait::async_trait;
use serde_json::{Value, json};
use std::collections::HashMap;
use turul_mcp_protocol::tools::{CallToolResult, ToolAnnotations, ToolSchema};
use turul_mcp_server::prelude::*;
use turul_mcp_server::{McpResult, McpServer, McpTool, SessionContext};

/// A tool whose advertised `inputSchema` is whatever we hand it — lets us push
/// arbitrary JSON through the real registration/build path.
struct ProbeTool {
    schema: ToolSchema,
}

impl ProbeTool {
    fn new(schema: Value) -> Self {
        Self {
            schema: serde_json::from_value(schema).expect("valid ToolSchema envelope"),
        }
    }
}

impl HasBaseMetadata for ProbeTool {
    fn name(&self) -> &str {
        "probe_tool"
    }
}
impl HasDescription for ProbeTool {
    fn description(&self) -> Option<&str> {
        Some("probe")
    }
}
impl HasInputSchema for ProbeTool {
    fn input_schema(&self) -> &ToolSchema {
        &self.schema
    }
}
impl HasOutputSchema for ProbeTool {
    fn output_schema(&self) -> Option<&ToolSchema> {
        None
    }
}
impl HasAnnotations for ProbeTool {
    fn annotations(&self) -> Option<&ToolAnnotations> {
        None
    }
}
impl HasToolMeta for ProbeTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
        None
    }
}
impl HasIcons for ProbeTool {}

#[async_trait]
impl McpTool for ProbeTool {
    async fn call(&self, _args: Value, _s: Option<SessionContext>) -> McpResult<CallToolResult> {
        Ok(CallToolResult::success(vec![]))
    }
}

fn build_with(schema: Value) -> Result<McpServer, turul_mcp_protocol::McpError> {
    McpServer::builder()
        .name("verify-bp3")
        .version("0.4.0")
        .tool(ProbeTool::new(schema))
        .build()
}

// ---- R1: unsupported dialect is rejected at build() ----
#[test]
fn verify_r1_unsupported_dialect_draft07_rejected_at_build() {
    // Different from implementer input: declare a draft-07 `$schema` dialect.
    let err = build_with(json!({
        "type": "object",
        "$schema": "http://json-schema.org/draft-07/schema#",
        "properties": { "a": { "type": "string" } }
    }))
    .expect_err("draft-07 dialect must be rejected at build()");
    let msg = err.to_string();
    assert!(msg.contains("probe_tool"), "must name the tool: {msg}");
    assert!(
        msg.contains("draft-07") || msg.contains("unsupported dialect"),
        "must explain the dialect rejection: {msg}"
    );
}

// ---- R1: a DIFFERENT malformed shape (type array with a non-string member) ----
#[test]
fn verify_r1_malformed_type_array_rejected_at_build() {
    let err = build_with(json!({
        "type": "object",
        "properties": { "bad": { "type": ["string", 42] } }
    }))
    .expect_err("`type: [\"string\", 42]` is not valid 2020-12 meta-schema");
    assert!(err.to_string().contains("probe_tool"), "{err}");
}

// ---- R1: absent $schema is accepted ----
#[test]
fn verify_r1_absent_schema_dialect_accepted_at_build() {
    build_with(json!({
        "type": "object",
        "properties": { "a": { "type": "string" } }
    }))
    .expect("absent $schema (default dialect) must be accepted");
}

// ---- R3: a genuinely recursive LOCAL $ref is accepted (not falsely TooDeep) ----
#[test]
fn verify_r3_recursive_local_ref_accepted_and_terminates() {
    // node -> children/items -> $ref back to node. If the bounds walk followed
    // $ref, this would hang or overflow; it must terminate and accept.
    build_with(json!({
        "type": "object",
        "properties": { "root": { "$ref": "#/$defs/Node" } },
        "$defs": {
            "Node": {
                "type": "object",
                "properties": {
                    "value": { "type": "string" },
                    "children": { "type": "array", "items": { "$ref": "#/$defs/Node" } }
                }
            }
        }
    }))
    .expect("a recursive local $ref schema must be accepted");
}

// ---- R3: a REMOTE $ref is rejected, diagnostic names the ref + policy ----
#[test]
fn verify_r3_remote_ref_rejected_naming_ref_and_policy() {
    let err = build_with(json!({
        "type": "object",
        "properties": { "a": { "$ref": "https://attacker.example/evil.json" } }
    }))
    .expect_err("a remote $ref must be rejected");
    let msg = err.to_string();
    assert!(msg.contains("probe_tool"), "must name the tool: {msg}");
    assert!(
        msg.contains("https://attacker.example/evil.json"),
        "diagnostic must name the offending $ref: {msg}"
    );
    assert!(
        msg.contains("policy") || msg.contains("disabled"),
        "diagnostic must state the policy rejection: {msg}"
    );
}

// ---- R3 BYPASS ATTEMPT: remote $ref hidden under `prefixItems`, a 2020-12
// keyword the cheap `check_bounds` walk does NOT traverse. If the compile step
// still resolves+blocks it, defense-in-depth holds. Capture actual behavior. ----
#[test]
fn verify_r3_bypass_remote_ref_under_prefix_items() {
    let result = build_with(json!({
        "type": "object",
        "properties": {
            "pair": {
                "type": "array",
                "prefixItems": [ { "$ref": "https://attacker.example/evil.json" } ]
            }
        }
    }));
    // Assert the security invariant: a schema that pulls a remote $ref MUST NOT
    // build successfully, regardless of which keyword hides it.
    assert!(
        result.is_err(),
        "SECURITY: a remote $ref under prefixItems must not build — this is a bypass"
    );
}

// ---- R3 BYPASS ATTEMPT #2: remote $ref under `additionalProperties`. ----
#[test]
fn verify_r3_bypass_remote_ref_under_additional_properties() {
    let result = build_with(json!({
        "type": "object",
        "properties": {
            "cfg": {
                "type": "object",
                "additionalProperties": { "$ref": "https://attacker.example/evil.json" }
            }
        }
    }));
    assert!(
        result.is_err(),
        "SECURITY: a remote $ref under additionalProperties must not build — this is a bypass"
    );
}
