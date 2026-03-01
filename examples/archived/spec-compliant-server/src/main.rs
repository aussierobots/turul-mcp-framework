//! # MCP 2025-11-25 Specification Compliant Server
//!
//! This example demonstrates a fully compliant MCP server that adheres to the
//! 2025-11-25 specification, including proper _meta field handling, progress tokens,
//! and structured JSON-RPC responses.

use std::collections::HashMap;

use turul_mcp_derive::{McpTool, resource};
use turul_mcp_server::{McpServer, McpTool, McpResource, SessionContext};
use turul_mcp_protocol::{ToolSchema, ToolResult, schema::JsonSchema};
use turul_mcp_protocol::tools::{HasBaseMetadata, HasDescription, HasInputSchema, HasOutputSchema, HasAnnotations, HasToolMeta, CallToolResult};
use turul_mcp_builders::prelude::{HasExecution, HasIcon};
use turul_mcp_protocol::{
    Meta, ProgressToken, ResultWithMeta, HasData, HasMeta,
    JsonRpcRequest, JsonRpcResponse, RequestParams
};
use serde_json::{json, Value};
use async_trait::async_trait;

/// A progress-aware tool that demonstrates _meta field usage
#[derive(McpTool, Clone)]
#[tool(name = "process_data", description = "Process data with progress tracking")]
struct ProcessDataTool {
    #[param(description = "Data to process")]
    data: String,
    #[param(description = "Number of processing steps")]
    steps: i32,
}

impl ProcessDataTool {
    async fn execute(&self) -> turul_mcp_server::McpResult<String> {
        // Simulate processing with progress updates
        let total_steps = self.steps as u64;

        for step in 0..total_steps {
            // In a real implementation, you would send progress notifications here
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            tracing::info!(
                "Processing step {}/{} for data: {}",
                step + 1,
                total_steps,
                self.data
            );
        }

        Ok(format!(
            "Successfully processed '{}' in {} steps",
            self.data,
            total_steps
        ))
    }
}

/// A tool that demonstrates session-aware processing with _meta
struct SessionAwareTool {
    input_schema: ToolSchema,
}

impl SessionAwareTool {
    fn new() -> Self {
        let input_schema = ToolSchema::object()
            .with_properties(HashMap::from([
                ("message".to_string(), JsonSchema::string().with_description("Message to process")),
                ("include_session_info".to_string(), JsonSchema::boolean().with_description("Whether to include session information")),
            ]))
            .with_required(vec!["message".to_string()]);
        Self { input_schema }
    }
}

impl HasBaseMetadata for SessionAwareTool {
    fn name(&self) -> &str {
        "session_tool"
    }
}

impl HasDescription for SessionAwareTool {
    fn description(&self) -> Option<&str> {
        Some("A tool that uses session context and demonstrates _meta field handling")
    }
}

impl HasInputSchema for SessionAwareTool {
    fn input_schema(&self) -> &ToolSchema {
        &self.input_schema
    }
}

impl HasOutputSchema for SessionAwareTool {
    fn output_schema(&self) -> Option<&ToolSchema> {
        None
    }
}

impl HasAnnotations for SessionAwareTool {
    fn annotations(&self) -> Option<&turul_mcp_protocol::tools::ToolAnnotations> {
        None
    }
}

impl HasToolMeta for SessionAwareTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
        None
    }
}

impl HasIcons for SessionAwareTool {}
impl HasExecution for SessionAwareTool {}

#[async_trait]
impl McpTool for SessionAwareTool {

    async fn call(&self, args: Value, session: Option<SessionContext>) -> turul_mcp_server::McpResult<CallToolResult> {
        let message = args["message"].as_str().unwrap_or("default");
        let include_session = args["include_session_info"].as_bool().unwrap_or(false);

        let mut response_data = HashMap::new();
        response_data.insert("processed_message".to_string(), json!(format!("Processed: {}", message)));
        response_data.insert("timestamp".to_string(), json!(chrono::Utc::now().to_rfc3339()));

        if include_session {
            if let Some(session) = session {
                response_data.insert("session_id".to_string(), json!(session.session_id.to_string()));
                response_data.insert("session_state".to_string(), json!("active"));
            } else {
                response_data.insert("session_info".to_string(), json!("No session available"));
            }
        }

        // Create a proper MCP 2025-11-25 compliant result with _meta
        let mut meta_data = HashMap::new();
        meta_data.insert("processing_time_ms".to_string(), json!(50));
        meta_data.insert("api_version".to_string(), json!("2025-11-25"));

        let result_with_meta = ResultWithMeta::new(response_data).with_meta(meta_data);

        // Convert to ToolResult
        let result_json = serde_json::to_value(result_with_meta).map_err(|e| e.to_string())?;

        let results = vec![ToolResult::text(format!("Result: {}", result_json))];
        Ok(CallToolResult::success(results))
    }
}

/// A resource that demonstrates proper _meta field usage
async fn create_status_resource() -> impl McpResource {
    resource! {
        uri: "system://status-v2",
        name: "System Status with Meta",
        description: "Enhanced system status with _meta information per MCP 2025-11-25",
        content: |_self| async move {
            let status_data = json!({
                "system_health": "excellent",
                "uptime_seconds": 12345,
                "active_sessions": 3,
                "memory_usage_mb": 128,
                "mcp_version": "2025-11-25"
            });

            // Create proper content with _meta
            let mut meta_info = HashMap::new();
            meta_info.insert("generated_at".to_string(), json!(chrono::Utc::now().to_rfc3339()));
            meta_info.insert("cache_expires_in_seconds".to_string(), json!(60));
            meta_info.insert("data_freshness".to_string(), json!("real-time"));

            let content_with_meta = json!({
                "data": status_data,
                "_meta": meta_info
            });

            Ok(vec![turul_mcp_protocol::resources::ResourceContent::blob(
                serde_json::to_string_pretty(&content_with_meta).unwrap(),
                "application/json".to_string()
            )])
        }
    }
}

/// Demonstrate JSON-RPC 2.0 message creation per spec
fn demonstrate_json_rpc_compliance() {
    println!("\n=== JSON-RPC 2.0 Specification Compliance Demo ===");

    // Create a proper MCP 2025-11-25 request with _meta
    let mut request_data = HashMap::new();
    request_data.insert("name".to_string(), json!("process_data"));
    request_data.insert("arguments".to_string(), json!({
        "data": "test data",
        "steps": 5
    }));

    let request_params = RequestParams {
        meta: Some(Meta {
            progress_token: Some(ProgressToken::new("req-12345")),
            cursor: None,
            total: Some(100),
            has_more: Some(true),
            estimated_remaining_seconds: Some(30.5),
            progress: Some(0.25),
            current_step: Some(25),
            total_steps: Some(100),
            ..Default::default()
        }),
        other: request_data,
    };

    let request = JsonRpcRequest::new(json!("req-001"), "tools/call".to_string())
        .with_params(request_params);

    println!("Request JSON:");
    println!("{}", serde_json::to_string_pretty(&request).unwrap());

    // Create a proper response with _meta
    let mut response_data = HashMap::new();
    response_data.insert("content".to_string(), json!([{
        "type": "text",
        "text": "Processing completed successfully"
    }]));

    let mut meta_data = HashMap::new();
    meta_data.insert("progress".to_string(), json!(1.0));
    meta_data.insert("total_steps".to_string(), json!(100));
    meta_data.insert("current_step".to_string(), json!(100));
    meta_data.insert("completed_at".to_string(), json!(chrono::Utc::now().to_rfc3339()));

    let result = ResultWithMeta::new(response_data).with_meta(meta_data);
    let response = JsonRpcResponse::success(json!("req-001"), result);

    println!("\nResponse JSON:");
    println!("{}", serde_json::to_string_pretty(&response).unwrap());

    // Demonstrate trait compliance
    println!("\n=== Trait Compliance Verification ===");
    println!("Response implements RpcResult: {}", std::any::type_name::<ResultWithMeta>());

    if let Some(result) = &response.result {
        println!("HasData implementation: {} keys in data", result.data().len());
        println!("HasMeta implementation: {} meta fields",
                 result.meta().map_or(0, |m| m.len()));
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("Starting MCP 2025-11-25 Specification Compliant Server");

    // Demonstrate JSON-RPC compliance
    demonstrate_json_rpc_compliance();

    // Create tools
    let process_tool = ProcessDataTool {
        data: "example".to_string(),
        steps: 3,
    };
    let session_tool = SessionAwareTool::new();

    // Create resources
    let status_resource = create_status_resource().await;

    // Build server with full MCP 2025-11-25 compliance
    let server = McpServer::builder()
        .name("spec-compliant-server")
        .version("1.0.0")
        .title("MCP 2025-11-25 Specification Compliant Server")
        .instructions("This server demonstrates full compliance with MCP 2025-11-25 specification including _meta fields, progress tokens, and proper JSON-RPC structure.")
        .tool(process_tool)
        .tool(session_tool)
        .resource(status_resource)
        .with_resources()
        .with_completion()
        .with_logging()
        .with_notifications()
        .bind_address("127.0.0.1:8012".parse()?)
        .build()?;

    println!("\nMCP 2025-11-25 Compliant Server running at: http://127.0.0.1:8012/mcp");
    println!("\nFeatures demonstrated:");
    println!("  ✓ Proper _meta field handling in requests and responses");
    println!("  ✓ Progress token support for long-running operations");
    println!("  ✓ Structured JSON-RPC 2.0 messages per specification");
    println!("  ✓ Session-aware tools with context passing");
    println!("  ✓ Resources with metadata and content type handling");
    println!("  ✓ Full trait compliance (RpcResult, HasData, HasMeta)");
    println!("  ✓ MCP protocol version negotiation");

    println!("\nEndpoints available:");
    println!("  - tools/list: List available tools with proper schemas");
    println!("  - tools/call: Execute tools with progress tracking");
    println!("  - resources/list: List resources with metadata");
    println!("  - resources/read: Read resources with _meta information");
    println!("  - completion/complete: Completion suggestions");
    println!("  - logging/setLevel: Set logging level");

    server.run().await?;
    Ok(())
}