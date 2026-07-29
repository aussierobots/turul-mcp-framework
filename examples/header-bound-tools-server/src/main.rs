//! # Header-Bound Tools Server (SEP-2243 `x-mcp-header` / `Mcp-Param-*`)
//!
//! A tool can designate input parameters for HTTP-header mirroring by
//! annotating the property with `x-mcp-header` in its `inputSchema`. Clients
//! then mirror the argument into an `Mcp-Param-<Name>` request header, so
//! infrastructure (API gateways, load balancers) can route on tool arguments
//! — region pinning, tenant sharding — **without parsing JSON bodies**.
//!
//! The server validates that mirrored headers match the body arguments:
//! - value in body but header missing → HTTP 400 + JSON-RPC `-32020`
//! - spurious header (no matching body argument) → 400 + `-32020`
//! - decoded mismatch → 400 + `-32020`
//! - non-tchar values ride a Base64 sentinel (`:b64:` prefix)
//!
//! The `route_query` tool below pins a `region` parameter to the
//! `Mcp-Param-Region` header. Run it and try the printed curls.

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::{Value, json};
use turul_mcp_protocol::ToolSchema;
use turul_mcp_protocol::tools::{CallToolResult, ToolResult};
use turul_mcp_server::prelude::*;
use turul_mcp_server::{McpServer, McpTool, SessionContext};

/// Query tool whose `region` argument is mirrored into `Mcp-Param-Region`.
struct RouteQueryTool {
    input_schema: ToolSchema,
}

impl RouteQueryTool {
    fn new() -> Self {
        let mut properties = HashMap::new();
        properties.insert(
            "region".to_string(),
            json!({
                "type": "string",
                "description": "Deployment region — mirrored to Mcp-Param-Region for LB routing",
                "x-mcp-header": "Region"
            }),
        );
        properties.insert(
            "query".to_string(),
            json!({ "type": "string", "description": "Query to run in that region" }),
        );
        Self {
            input_schema: ToolSchema::object()
                .with_properties(properties)
                .with_required(vec!["region".to_string(), "query".to_string()]),
        }
    }
}

impl HasBaseMetadata for RouteQueryTool {
    fn name(&self) -> &str {
        "route_query"
    }
}
impl HasDescription for RouteQueryTool {
    fn description(&self) -> Option<&str> {
        Some("Run a query in a region (region is header-mirrored per SEP-2243)")
    }
}
impl HasInputSchema for RouteQueryTool {
    fn input_schema(&self) -> &ToolSchema {
        &self.input_schema
    }
}
impl HasOutputSchema for RouteQueryTool {}
impl HasAnnotations for RouteQueryTool {}
impl HasToolMeta for RouteQueryTool {}
impl HasIcons for RouteQueryTool {}

#[async_trait]
impl McpTool for RouteQueryTool {
    async fn call(
        &self,
        args: Value,
        _session: Option<SessionContext>,
    ) -> McpResult<CallToolResult> {
        let region = args
            .get("region")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
        Ok(CallToolResult::success(vec![ToolResult::text(format!(
            "ran {query:?} in {region}"
        ))]))
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
        .name("header-bound-tools-server")
        .version("0.4.0")
        .title("SEP-2243 Header-Bound Tools Example")
        .instructions(
            "route_query's region argument is x-mcp-header-annotated: mirror \
             it into the Mcp-Param-Region request header.",
        )
        .tool(RouteQueryTool::new())
        .bind_address("127.0.0.1:8644".parse()?)
        .build()?;

    let meta = r#""_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"curl","version":"1.0"},"io.modelcontextprotocol/clientCapabilities":{}}"#;
    tracing::info!("Header-bound tools server running at http://127.0.0.1:8644/mcp");
    tracing::info!("# Matching header → 200:");
    tracing::info!(
        "curl -s -X POST http://127.0.0.1:8644/mcp -H 'Content-Type: application/json' -H 'MCP-Protocol-Version: 2026-07-28' -H 'Mcp-Method: tools/call' -H 'Mcp-Name: route_query' -H 'Mcp-Param-Region: ap-southeast-2' -d '{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{{{meta},\"name\":\"route_query\",\"arguments\":{{\"region\":\"ap-southeast-2\",\"query\":\"SELECT 1\"}}}}}}'"
    );
    tracing::info!("# Omit Mcp-Param-Region (value still in body) → 400 + -32020");
    tracing::info!("# Mismatched header value → 400 + -32020");

    server.run().await?;
    Ok(())
}
