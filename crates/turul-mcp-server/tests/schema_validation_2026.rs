//! Server-side trust boundary: a server MUST NOT advertise an invalid
//! tool `inputSchema`. `McpServerBuilder::build()` rejects registration of a
//! tool whose `inputSchema` fails JSON Schema 2020-12 dialect validation.
//!
//! Built only under the 2026 feature.
#![cfg(feature = "protocol-2026-07-28")]

use async_trait::async_trait;
use serde_json::{Value, json};
use std::collections::HashMap;
use turul_mcp_protocol::tools::{CallToolResult, ToolAnnotations, ToolSchema};
use turul_mcp_server::prelude::*;
use turul_mcp_server::{McpResult, McpServer, McpTool, SessionContext};

#[derive(Clone, Default)]
struct BadSchemaTool;

impl HasBaseMetadata for BadSchemaTool {
    fn name(&self) -> &str {
        "bad_schema_tool"
    }
}

impl HasDescription for BadSchemaTool {
    fn description(&self) -> Option<&str> {
        Some("Tool with an invalid inputSchema")
    }
}

impl HasInputSchema for BadSchemaTool {
    fn input_schema(&self) -> &ToolSchema {
        // `"type": 123` fails JSON Schema 2020-12 meta-validation (the
        // `type` keyword must be a string or array of strings).
        static SCHEMA: std::sync::OnceLock<ToolSchema> = std::sync::OnceLock::new();
        SCHEMA.get_or_init(|| ToolSchema {
            schema_type: "object".to_string(),
            properties: Some(HashMap::from([("bad".to_string(), json!({"type": 123}))])),
            required: None,
            additional: HashMap::new(),
        })
    }
}

impl HasOutputSchema for BadSchemaTool {
    fn output_schema(&self) -> Option<&ToolSchema> {
        None
    }
}

impl HasAnnotations for BadSchemaTool {
    fn annotations(&self) -> Option<&ToolAnnotations> {
        None
    }
}

impl HasToolMeta for BadSchemaTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
        None
    }
}

impl HasIcons for BadSchemaTool {}

#[async_trait]
impl McpTool for BadSchemaTool {
    async fn call(
        &self,
        _args: Value,
        _session: Option<SessionContext>,
    ) -> McpResult<CallToolResult> {
        Ok(CallToolResult::success(vec![]))
    }
}

#[derive(Clone, Default)]
struct GoodSchemaTool;

impl HasBaseMetadata for GoodSchemaTool {
    fn name(&self) -> &str {
        "good_schema_tool"
    }
}

impl HasDescription for GoodSchemaTool {
    fn description(&self) -> Option<&str> {
        Some("Tool with a well-formed inputSchema")
    }
}

impl HasInputSchema for GoodSchemaTool {
    fn input_schema(&self) -> &ToolSchema {
        static SCHEMA: std::sync::OnceLock<ToolSchema> = std::sync::OnceLock::new();
        SCHEMA.get_or_init(|| ToolSchema {
            schema_type: "object".to_string(),
            properties: Some(HashMap::from([(
                "name".to_string(),
                json!({"type": "string"}),
            )])),
            required: None,
            additional: HashMap::new(),
        })
    }
}

impl HasOutputSchema for GoodSchemaTool {
    fn output_schema(&self) -> Option<&ToolSchema> {
        None
    }
}

impl HasAnnotations for GoodSchemaTool {
    fn annotations(&self) -> Option<&ToolAnnotations> {
        None
    }
}

impl HasToolMeta for GoodSchemaTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
        None
    }
}

impl HasIcons for GoodSchemaTool {}

#[async_trait]
impl McpTool for GoodSchemaTool {
    async fn call(
        &self,
        _args: Value,
        _session: Option<SessionContext>,
    ) -> McpResult<CallToolResult> {
        Ok(CallToolResult::success(vec![]))
    }
}

#[test]
fn server_rejects_tool_with_invalid_input_schema_at_build() {
    let result = McpServer::builder()
        .name("bad-schema-server")
        .version("0.4.0")
        .tool(BadSchemaTool)
        .build();
    assert!(
        result.is_err(),
        "build() must reject a server whose advertised inputSchema is invalid"
    );
    let err = result.err().unwrap().to_string();
    assert!(
        err.contains("bad_schema_tool"),
        "error must name the offending tool: {err}"
    );
}

#[test]
fn server_accepts_tool_with_valid_input_schema() {
    let result = McpServer::builder()
        .name("good-schema-server")
        .version("0.4.0")
        .tool(GoodSchemaTool)
        .build();
    assert!(
        result.is_ok(),
        "build() must not reject a well-formed inputSchema: {:?}",
        result.err()
    );
}
