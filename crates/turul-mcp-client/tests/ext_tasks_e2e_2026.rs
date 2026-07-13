//! Real-server e2e for the client's Tasks-extension surface (SEP-2663):
//! bilingual client with `declared_capabilities.ext_tasks` against an
//! in-process `turul-mcp-server` built with the `ext-tasks` feature.
//!
//! Run with: `cargo test -p turul-mcp-client --features ext-tasks --test ext_tasks_e2e_2026`
#![cfg(all(feature = "ext-tasks", feature = "client-bilingual", feature = "http"))]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::{Value, json};
use turul_mcp_client::transport::HttpTransport;
use turul_mcp_client::{ClientConfig, McpClient, ToolCallOutcome};
use turul_mcp_ext_tasks::{InMemoryTaskStore, TaskStatus};
use turul_mcp_protocol::ToolSchema;
use turul_mcp_protocol::tools::{CallToolResult, ToolResult};
use turul_mcp_server::prelude::*;
use turul_mcp_server::{McpServer, McpTool, SessionContext};

/// Slow doubling tool, registered for task election.
struct SlowDoubleTool {
    input_schema: ToolSchema,
}

impl SlowDoubleTool {
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

impl HasBaseMetadata for SlowDoubleTool {
    fn name(&self) -> &str {
        "slow_double"
    }
}
impl HasDescription for SlowDoubleTool {
    fn description(&self) -> Option<&str> {
        Some("Double a number, slowly")
    }
}
impl HasInputSchema for SlowDoubleTool {
    fn input_schema(&self) -> &ToolSchema {
        &self.input_schema
    }
}
impl HasOutputSchema for SlowDoubleTool {}
impl HasAnnotations for SlowDoubleTool {}
impl HasToolMeta for SlowDoubleTool {}
impl HasIcons for SlowDoubleTool {}

#[async_trait]
impl McpTool for SlowDoubleTool {
    async fn call(&self, args: Value, _s: Option<SessionContext>) -> McpResult<CallToolResult> {
        let n = args.get("n").and_then(|v| v.as_f64()).unwrap_or(0.0);
        tokio::time::sleep(Duration::from_millis(120)).await;
        Ok(CallToolResult::success(vec![ToolResult::text(format!(
            "{}",
            n * 2.0
        ))]))
    }
}

/// Tool that demands an elicited approval before completing.
struct ApprovalTool {
    input_schema: ToolSchema,
}
impl ApprovalTool {
    fn new() -> Self {
        Self {
            input_schema: ToolSchema::object(),
        }
    }
}
impl HasBaseMetadata for ApprovalTool {
    fn name(&self) -> &str {
        "needs_approval"
    }
}
impl HasDescription for ApprovalTool {
    fn description(&self) -> Option<&str> {
        Some("Completes only after an elicited approval")
    }
}
impl HasInputSchema for ApprovalTool {
    fn input_schema(&self) -> &ToolSchema {
        &self.input_schema
    }
}
impl HasOutputSchema for ApprovalTool {}
impl HasAnnotations for ApprovalTool {}
impl HasToolMeta for ApprovalTool {}
impl HasIcons for ApprovalTool {}

#[async_trait]
impl McpTool for ApprovalTool {
    async fn call(&self, _a: Value, session: Option<SessionContext>) -> McpResult<CallToolResult> {
        use turul_mcp_protocol::elicitation::{ElicitRequest, ElicitationSchema};
        use turul_mcp_protocol::input_required::{InputRequest, InputRequests, InputResponse};

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
                if approved { "approved" } else { "denied" },
            )]));
        }
        let schema = ElicitationSchema::new().with_property(
            "approved".to_string(),
            turul_mcp_protocol::elicitation::PrimitiveSchemaDefinition::boolean(),
        );
        let mut requests = InputRequests::new();
        requests.insert(
            "approval".to_string(),
            InputRequest::Elicit(ElicitRequest::new_form("Approve?", schema)),
        );
        Err(McpError::InputRequired {
            input_requests: Some(requests),
            request_state: None,
        })
    }
}

async fn start_server() -> String {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let server = McpServer::builder()
        .name("ext-tasks-client-e2e")
        .version("0.4.0")
        .with_ext_tasks(Arc::new(InMemoryTaskStore::new()))
        .ext_task_tool(SlowDoubleTool::new())
        .ext_task_tool(ApprovalTool::new())
        .bind_address(format!("127.0.0.1:{port}").parse().unwrap())
        .build()
        .expect("build server");
    tokio::spawn(async move {
        server.run().await.ok();
    });
    let url = format!("http://127.0.0.1:{port}/mcp");
    let probe = reqwest::Client::new();
    for _ in 0..50 {
        if probe.get(&url).send().await.is_ok() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    url
}

async fn connect(url: &str, declare: bool) -> McpClient {
    let mut config = ClientConfig::default();
    config.declared_capabilities.ext_tasks = declare;
    let transport = Box::new(HttpTransport::new(url).expect("transport"));
    let client = McpClient::new(transport, config);
    client.connect().await.expect("connect");
    client
}

/// Declared client: task handle → task_wait polls (honoring pollIntervalMs)
/// to the completed result.
#[tokio::test]
async fn task_outcome_polls_to_completion() {
    let url = start_server().await;
    let client = connect(&url, true).await;

    let outcome = client
        .call_tool_or_task("slow_double", json!({ "n": 21 }))
        .await
        .expect("call");
    let ToolCallOutcome::Task(task) = outcome else {
        panic!("expected a task handle, got {outcome:?}");
    };
    assert_eq!(task.task.status, TaskStatus::Working);
    assert!(task.has_task_discriminator());

    let done = client
        .task_wait(&task.task.fields.task_id)
        .await
        .expect("wait");
    assert_eq!(done.status(), TaskStatus::Completed);
    let turul_mcp_ext_tasks::DetailedTask::Completed { result, .. } = done else {
        panic!("expected completed: {done:?}");
    };
    assert_eq!(result["content"][0]["text"], "42");
}

/// Undeclared client: the SAME tool completes synchronously — and the strict
/// BP-1 parser stays intact (the sync result is an ordinary CallToolResult).
#[tokio::test]
async fn undeclared_client_gets_synchronous_outcome() {
    let url = start_server().await;
    let client = connect(&url, false).await;

    let outcome = client
        .call_tool_or_task("slow_double", json!({ "n": 4 }))
        .await
        .expect("call");
    let ToolCallOutcome::Completed(result) = outcome else {
        panic!("expected sync completion, got {outcome:?}");
    };
    let text = serde_json::to_value(&result).unwrap()["content"][0]["text"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(text, "8");
}

/// input_required → task_update → completion, all through the typed client
/// surface.
#[tokio::test]
async fn task_update_resumes_an_input_required_task() {
    let url = start_server().await;
    let client = connect(&url, true).await;

    let outcome = client
        .call_tool_or_task("needs_approval", json!({}))
        .await
        .expect("call");
    let ToolCallOutcome::Task(task) = outcome else {
        panic!("expected a task handle");
    };
    let task_id = task.task.fields.task_id.clone();

    // Poll until the worker parks.
    let mut parked = None;
    for _ in 0..100 {
        let t = client.task_get(&task_id).await.expect("get");
        if t.status() == TaskStatus::InputRequired {
            parked = Some(t);
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let parked = parked.expect("task never parked in input_required");
    let turul_mcp_ext_tasks::DetailedTask::InputRequired { input_requests, .. } = &parked else {
        panic!("wrong variant");
    };
    assert!(input_requests.contains_key("approval"));

    client
        .task_update(
            &task_id,
            json!({ "approval": { "action": "accept", "content": { "approved": true } } }),
        )
        .await
        .expect("update");

    let done = client.task_wait(&task_id).await.expect("wait");
    let turul_mcp_ext_tasks::DetailedTask::Completed { result, .. } = done else {
        panic!("expected completed: {done:?}");
    };
    assert_eq!(result["content"][0]["text"], "approved");
}

/// task_cancel flips a working task to cancelled.
#[tokio::test]
async fn task_cancel_reaches_cancelled() {
    let url = start_server().await;
    let client = connect(&url, true).await;

    let outcome = client
        .call_tool_or_task("slow_double", json!({ "n": 1 }))
        .await
        .expect("call");
    let ToolCallOutcome::Task(task) = outcome else {
        panic!("expected a task handle");
    };
    client
        .task_cancel(&task.task.fields.task_id)
        .await
        .expect("cancel");
    let done = client
        .task_wait(&task.task.fields.task_id)
        .await
        .expect("wait");
    assert_eq!(done.status(), TaskStatus::Cancelled);
}

/// `call_tool_or_task` must NOT pass an explicit raw `Mcp-Name` header — the
/// transport already derives and Base64-sentinel-encodes it from `params.name`.
/// A raw extra-header would produce a SECOND, unencoded `Mcp-Name` on the wire
/// (reqwest appends). This inspects the wire directly (a real server reads
/// `Mcp-Name` via `.get()` and would silently take the first, hiding the
/// duplicate).
#[tokio::test]
async fn call_tool_or_task_emits_exactly_one_encoded_mcp_name_header() {
    use wiremock::matchers::{body_partial_json, method};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(body_partial_json(json!({"method": "server/discover"})))
        .respond_with(ResponseTemplate::new(200).insert_header("Content-Type", "application/json").set_body_json(json!({
            "jsonrpc": "2.0", "id": "req_0",
            "result": { "resultType": "complete", "ttlMs": 0, "cacheScope": "public",
                "supportedVersions": ["2026-07-28"], "capabilities": {},
                "serverInfo": { "name": "mock-2026", "version": "1.0.0" } }
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(body_partial_json(json!({"method": "tools/call"})))
        .respond_with(ResponseTemplate::new(200).insert_header("Content-Type", "application/json").set_body_json(json!({
            "jsonrpc": "2.0", "id": "req_1",
            "result": { "resultType": "complete", "ttlMs": 0, "cacheScope": "public",
                "content": [{ "type": "text", "text": "ok" }], "isError": false }
        })))
        .mount(&server)
        .await;

    let url = format!("{}/mcp", server.uri());
    let mut config = ClientConfig::default();
    config.declared_capabilities.ext_tasks = true;
    let transport = Box::new(HttpTransport::new(&url).unwrap());
    let client = McpClient::new(transport, config);
    client.connect().await.expect("connect against a 2026 server");

    // A padded name: raw " padded " vs encoded "=?base64?IHBhZGRlZCA=?=" differ,
    // so a duplicate is detectable on the wire.
    client
        .call_tool_or_task(" padded ", json!({}))
        .await
        .expect("call_tool_or_task");

    let reqs = server
        .received_requests()
        .await
        .expect("wiremock records requests");
    let tools_call: Vec<_> = reqs
        .iter()
        .filter(|r| {
            serde_json::from_slice::<Value>(&r.body)
                .ok()
                .and_then(|b| b.get("method").and_then(|m| m.as_str()).map(|s| s == "tools/call"))
                .unwrap_or(false)
        })
        .collect();
    assert_eq!(tools_call.len(), 1, "expected exactly one tools/call request");
    let names: Vec<_> = tools_call[0]
        .headers
        .get_all("mcp-name")
        .iter()
        .map(|v| v.to_str().unwrap().to_string())
        .collect();
    assert_eq!(
        names.len(),
        1,
        "exactly one Mcp-Name header (no raw+encoded duplicate), got {names:?}"
    );
    assert_eq!(
        names[0], "=?base64?IHBhZGRlZCA=?=",
        "the single Mcp-Name must be the Base64-sentinel-encoded form"
    );
}
