//! # Tasks Extension Server (`io.modelcontextprotocol/tasks`, SEP-2663)
//!
//! Durable poll handles instead of blocking on long-running tool calls: a
//! client that declares the Tasks extension in its per-request `_meta`
//! `clientCapabilities.extensions` gets a `CreateTaskResult`
//! (`resultType: "task"`) back immediately and polls `tasks/get` until the
//! task is terminal. Clients that don't declare it get the ordinary
//! synchronous result — same tool, progressive enhancement.
//!
//! Two task-electing tools:
//! - `crunch` — sleeps ~2s then answers (watch it poll through `working`)
//! - `deploy` — demands an elicited approval mid-task (`input_required` →
//!   `tasks/update` → `completed`), reusing the exact MRTR tool contract
//!
//! Pair with the client leg:
//!
//! ```bash
//! cargo run -p ext-tasks-server
//! cargo run -p ext-tasks-server --bin ext-tasks-client
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::{Value, json};
use turul_mcp_ext_tasks::InMemoryTaskStore;
use turul_mcp_protocol::ToolSchema;
use turul_mcp_protocol::tools::{CallToolResult, ToolResult};
use turul_mcp_server::prelude::*;
use turul_mcp_server::{McpServer, McpTool, SessionContext};

/// Long-running number cruncher.
struct CrunchTool {
    input_schema: ToolSchema,
}

impl CrunchTool {
    fn new() -> Self {
        let mut properties = HashMap::new();
        properties.insert("n".to_string(), json!({ "type": "number" }));
        Self {
            input_schema: ToolSchema::object()
                .with_properties(properties)
                .with_required(vec!["n".to_string()]),
        }
    }
}

impl HasBaseMetadata for CrunchTool {
    fn name(&self) -> &str {
        "crunch"
    }
}
impl HasDescription for CrunchTool {
    fn description(&self) -> Option<&str> {
        Some("Square a number after ~2s of pretend work")
    }
}
impl HasInputSchema for CrunchTool {
    fn input_schema(&self) -> &ToolSchema {
        &self.input_schema
    }
}
impl HasOutputSchema for CrunchTool {}
impl HasAnnotations for CrunchTool {}
impl HasToolMeta for CrunchTool {}
impl HasIcons for CrunchTool {}

#[async_trait]
impl McpTool for CrunchTool {
    async fn call(&self, args: Value, _s: Option<SessionContext>) -> McpResult<CallToolResult> {
        let n = args.get("n").and_then(|v| v.as_f64()).unwrap_or(0.0);
        tokio::time::sleep(Duration::from_secs(2)).await;
        Ok(CallToolResult::success(vec![ToolResult::text(format!(
            "{}",
            n * n
        ))]))
    }
}

/// Deploy that pauses for an elicited approval — identical MRTR tool code to
/// the synchronous retry pattern; under task election the runtime parks the
/// task instead of returning input_required to the caller.
struct DeployTool {
    input_schema: ToolSchema,
}

impl DeployTool {
    fn new() -> Self {
        let mut properties = HashMap::new();
        properties.insert("service".to_string(), json!({ "type": "string" }));
        Self {
            input_schema: ToolSchema::object()
                .with_properties(properties)
                .with_required(vec!["service".to_string()]),
        }
    }
}

impl HasBaseMetadata for DeployTool {
    fn name(&self) -> &str {
        "deploy"
    }
}
impl HasDescription for DeployTool {
    fn description(&self) -> Option<&str> {
        Some("Deploy a service after an elicited approval")
    }
}
impl HasInputSchema for DeployTool {
    fn input_schema(&self) -> &ToolSchema {
        &self.input_schema
    }
}
impl HasOutputSchema for DeployTool {}
impl HasAnnotations for DeployTool {}
impl HasToolMeta for DeployTool {}
impl HasIcons for DeployTool {}

#[async_trait]
impl McpTool for DeployTool {
    async fn call(
        &self,
        args: Value,
        session: Option<SessionContext>,
    ) -> McpResult<CallToolResult> {
        use turul_mcp_protocol::elicitation::{
            ElicitRequest, ElicitationSchema, PrimitiveSchemaDefinition,
        };
        use turul_mcp_protocol::input_required::{InputRequest, InputRequests, InputResponse};

        let service = args
            .get("service")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let session = session.ok_or_else(|| McpError::tool_execution("context required"))?;

        if let Some(responses) = session.input_responses() {
            let approved = responses
                .get("approval")
                .and_then(|r| match r {
                    InputResponse::Elicit(e) => e
                        .content
                        .as_ref()
                        .and_then(|c| c.get("approved"))
                        .and_then(|v| v.as_bool()),
                    _ => None,
                })
                .unwrap_or(false);
            return Ok(CallToolResult::success(vec![ToolResult::text(
                if approved {
                    format!("deployed {service} ✅")
                } else {
                    format!("deploy of {service} rejected")
                },
            )]));
        }

        let schema = ElicitationSchema::new()
            .with_property("approved".to_string(), PrimitiveSchemaDefinition::boolean());
        let mut requests = InputRequests::new();
        requests.insert(
            "approval".to_string(),
            InputRequest::Elicit(ElicitRequest::new_form(
                format!("Approve deployment of {service}?"),
                schema,
            )),
        );
        Err(McpError::InputRequired {
            input_requests: Some(requests),
            request_state: None,
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let server = McpServer::builder()
        .name("ext-tasks-server")
        .version(env!("CARGO_PKG_VERSION"))
        .title("Tasks Extension Example")
        .instructions(
            "crunch and deploy are task-electing: declare \
             io.modelcontextprotocol/tasks in clientCapabilities.extensions \
             to receive durable task handles; omit it for synchronous results.",
        )
        .with_ext_tasks(Arc::new(InMemoryTaskStore::new()))
        .ext_task_tool(CrunchTool::new())
        .ext_task_tool(DeployTool::new())
        .bind_address("127.0.0.1:8645".parse()?)
        .build()?;

    tracing::info!("Tasks-extension server at http://127.0.0.1:8645/mcp");
    tracing::info!("Walk the lifecycle with the paired client:");
    tracing::info!("  cargo run -p ext-tasks-server --bin ext-tasks-client");

    server.run().await?;
    Ok(())
}
