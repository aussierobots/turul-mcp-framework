//! Wire-level acceptance for the Tasks extension
//! (`io.modelcontextprotocol/tasks`, SEP-2663) on the 2026-07-28 lane.
//!
//! Run with: `cargo test -p turul-mcp-server --features ext-tasks --test ext_tasks_2026`
#![cfg(all(feature = "ext-tasks", feature = "protocol-2026-07-28"))]

mod common;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::{Value, json};
use turul_mcp_ext_tasks::InMemoryTaskStore;
use turul_mcp_protocol::ToolSchema;
use turul_mcp_protocol::tools::{CallToolResult, ToolResult};
use turul_mcp_server::middleware::{
    McpMiddleware, MiddlewareError, RequestContext, SessionInjection,
};
use turul_mcp_server::prelude::*;
use turul_mcp_server::{McpServer, McpTool, SessionContext, SessionView};

/// Slow tool — long enough that the create response races ahead of completion.
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
        tokio::time::sleep(Duration::from_millis(150)).await;
        Ok(CallToolResult::success(vec![ToolResult::text(format!(
            "{}",
            n * 2.0
        ))]))
    }
}

/// Tool that demands an elicited confirmation before completing — exercises
/// the input_required ⇄ tasks/update bridge.
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
    async fn call(
        &self,
        _args: Value,
        session: Option<SessionContext>,
    ) -> McpResult<CallToolResult> {
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
            assert_eq!(
                session.mrtr_request_state().as_deref(),
                Some("approval-round-1"),
                "requestState must replay into the resumed worker"
            );
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
            request_state: Some("approval-round-1".to_string()),
        })
    }
}

async fn start_server() -> String {
    let reserved = common::reserve_port().await;
    let port = reserved.port;

    let server = McpServer::builder()
        .name("ext-tasks-2026-test")
        .version("0.4.0")
        .with_ext_tasks(Arc::new(InMemoryTaskStore::new()))
        .ext_task_tool(SlowDoubleTool::new())
        .ext_task_tool(ApprovalTool::new())
        .ext_task_tool_required(SlowRequiredTool::new())
        .bind_address(format!("127.0.0.1:{port}").parse().unwrap())
        .build()
        .expect("build server");

    tokio::spawn(async move {
        server.run().await.ok();
    });

    let url = format!("http://127.0.0.1:{port}/mcp");
    let client = reqwest::Client::new();
    for _ in 0..50 {
        if client.get(&url).send().await.is_ok() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    url
}

/// Stand-in for `OAuthResourceMiddleware`: reads a plain `x-test-principal`
/// header instead of verifying a Bearer JWT, but writes the exact same
/// `__turul_internal.auth_claims` extension key a real validated token's
/// `sub` claim would land in. This drives the real ext-tasks owner-binding
/// code path over real HTTP without standing up a JWKS/IdP in the test.
struct TestPrincipalMiddleware;

#[async_trait]
impl McpMiddleware for TestPrincipalMiddleware {
    fn runs_before_session(&self) -> bool {
        true
    }

    async fn before_dispatch(
        &self,
        ctx: &mut RequestContext<'_>,
        _session: Option<&dyn SessionView>,
        _injection: &mut SessionInjection,
    ) -> Result<(), MiddlewareError> {
        if let Some(sub) = ctx
            .metadata()
            .get("x-test-principal")
            .and_then(|v| v.as_str())
        {
            ctx.set_extension("__turul_internal.auth_claims", json!({ "sub": sub }));
        }
        Ok(())
    }
}

/// Same server as [`start_server`] but with [`TestPrincipalMiddleware`]
/// installed, so `tasks/*` requests carry a caller principal.
async fn start_server_with_principals() -> String {
    let reserved = common::reserve_port().await;
    let port = reserved.port;

    let server = McpServer::builder()
        .name("ext-tasks-2026-owner-test")
        .version("0.4.0")
        .with_ext_tasks(Arc::new(InMemoryTaskStore::new()))
        .ext_task_tool(SlowDoubleTool::new())
        .ext_task_tool(ApprovalTool::new())
        .middleware(Arc::new(TestPrincipalMiddleware))
        .bind_address(format!("127.0.0.1:{port}").parse().unwrap())
        .build()
        .expect("build server");

    tokio::spawn(async move {
        server.run().await.ok();
    });

    let url = format!("http://127.0.0.1:{port}/mcp");
    let client = reqwest::Client::new();
    for _ in 0..50 {
        if client.get(&url).send().await.is_ok() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    url
}

/// Same as SlowDoubleTool but registered task-REQUIRED.
struct SlowRequiredTool {
    input_schema: ToolSchema,
}
impl SlowRequiredTool {
    fn new() -> Self {
        Self {
            input_schema: ToolSchema::object(),
        }
    }
}
impl HasBaseMetadata for SlowRequiredTool {
    fn name(&self) -> &str {
        "must_be_task"
    }
}
impl HasDescription for SlowRequiredTool {
    fn description(&self) -> Option<&str> {
        Some("Only runs as a task")
    }
}
impl HasInputSchema for SlowRequiredTool {
    fn input_schema(&self) -> &ToolSchema {
        &self.input_schema
    }
}
impl HasOutputSchema for SlowRequiredTool {}
impl HasAnnotations for SlowRequiredTool {}
impl HasToolMeta for SlowRequiredTool {}
impl HasIcons for SlowRequiredTool {}
#[async_trait]
impl McpTool for SlowRequiredTool {
    async fn call(&self, _a: Value, _s: Option<SessionContext>) -> McpResult<CallToolResult> {
        Ok(CallToolResult::success(vec![ToolResult::text("ran")]))
    }
}

const EXT: &str = "io.modelcontextprotocol/tasks";

fn meta(declare_ext: bool) -> Value {
    let caps = if declare_ext {
        json!({ "extensions": { EXT: {} } })
    } else {
        json!({})
    };
    json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": { "name": "ext-tasks-test", "version": "1.0" },
        "io.modelcontextprotocol/clientCapabilities": caps
    })
}

async fn post(
    url: &str,
    method: &str,
    name_header: Option<&str>,
    body: Value,
) -> (reqwest::StatusCode, Value) {
    let client = reqwest::Client::new();
    let mut req = client
        .post(url)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", method);
    if let Some(n) = name_header {
        req = req.header("Mcp-Name", n);
    }
    let resp = req.json(&body).send().await.expect("POST");
    let status = resp.status();
    let body: Value = resp.json().await.unwrap_or(Value::Null);
    (status, body)
}

/// Like [`post`] but attaches `x-test-principal`, which
/// [`TestPrincipalMiddleware`] turns into the caller's bound owner.
async fn post_as(
    url: &str,
    method: &str,
    name_header: Option<&str>,
    principal: &str,
    body: Value,
) -> (reqwest::StatusCode, Value) {
    let client = reqwest::Client::new();
    let mut req = client
        .post(url)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", method)
        .header("x-test-principal", principal);
    if let Some(n) = name_header {
        req = req.header("Mcp-Name", n);
    }
    let resp = req.json(&body).send().await.expect("POST");
    let status = resp.status();
    let body: Value = resp.json().await.unwrap_or(Value::Null);
    (status, body)
}

async fn call_tool_as(
    url: &str,
    tool: &str,
    args: Value,
    principal: &str,
) -> (reqwest::StatusCode, Value) {
    post_as(
        url,
        "tools/call",
        Some(tool),
        principal,
        json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": { "_meta": meta(true), "name": tool, "arguments": args }
        }),
    )
    .await
}

async fn tasks_get_as(url: &str, task_id: &str, principal: &str) -> (reqwest::StatusCode, Value) {
    post_as(
        url,
        "tasks/get",
        None,
        principal,
        json!({
            "jsonrpc": "2.0", "id": 2, "method": "tasks/get",
            "params": { "_meta": meta(true), "taskId": task_id }
        }),
    )
    .await
}

async fn tasks_update_as(
    url: &str,
    task_id: &str,
    principal: &str,
    input_responses: Value,
) -> (reqwest::StatusCode, Value) {
    post_as(
        url,
        "tasks/update",
        None,
        principal,
        json!({
            "jsonrpc": "2.0", "id": 3, "method": "tasks/update",
            "params": { "_meta": meta(true), "taskId": task_id, "inputResponses": input_responses }
        }),
    )
    .await
}

async fn tasks_cancel_as(
    url: &str,
    task_id: &str,
    principal: &str,
) -> (reqwest::StatusCode, Value) {
    post_as(
        url,
        "tasks/cancel",
        None,
        principal,
        json!({
            "jsonrpc": "2.0", "id": 4, "method": "tasks/cancel",
            "params": { "_meta": meta(true), "taskId": task_id }
        }),
    )
    .await
}

async fn poll_until_input_required_as(url: &str, task_id: &str, principal: &str) -> Value {
    for _ in 0..100 {
        let (status, body) = tasks_get_as(url, task_id, principal).await;
        assert_eq!(status, 200, "tasks/get: {body}");
        if body["result"]["status"] == "input_required" {
            return body;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("task {task_id} never reached input_required");
}

async fn call_tool(
    url: &str,
    tool: &str,
    args: Value,
    declare_ext: bool,
) -> (reqwest::StatusCode, Value) {
    post(
        url,
        "tools/call",
        Some(tool),
        json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": { "_meta": meta(declare_ext), "name": tool, "arguments": args }
        }),
    )
    .await
}

async fn tasks_get(url: &str, task_id: &str) -> Value {
    let (status, body) = post(
        url,
        "tasks/get",
        None,
        json!({
            "jsonrpc": "2.0", "id": 2, "method": "tasks/get",
            "params": { "_meta": meta(true), "taskId": task_id }
        }),
    )
    .await;
    assert_eq!(status, 200, "tasks/get: {body}");
    body
}

async fn poll_until_terminal(url: &str, task_id: &str) -> Value {
    for _ in 0..100 {
        let body = tasks_get(url, task_id).await;
        let status = body["result"]["status"].as_str().unwrap_or("").to_string();
        if ["completed", "failed", "cancelled"].contains(&status.as_str()) {
            return body;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("task {task_id} never reached a terminal status");
}

/// server/discover advertises the extension when a store is configured.
#[tokio::test]
async fn discover_advertises_the_tasks_extension() {
    let url = start_server().await;
    let (status, body) = post(
        &url,
        "server/discover",
        None,
        json!({
            "jsonrpc": "2.0", "id": 1, "method": "server/discover",
            "params": { "_meta": meta(false) }
        }),
    )
    .await;
    assert_eq!(status, 200);
    assert!(
        body["result"]["capabilities"]["extensions"][EXT].is_object(),
        "extensions map must carry the tasks identifier: {body}"
    );
}

/// Declared extension → CreateTaskResult (resultType "task", durable before
/// response) → poll to completed; the result is what the sync call would
/// have returned.
#[tokio::test]
async fn declared_call_returns_task_and_polls_to_completion() {
    let url = start_server().await;
    let (status, body) = call_tool(&url, "slow_double", json!({ "n": 21 }), true).await;
    assert_eq!(status, 200, "{body}");
    let result = &body["result"];
    assert_eq!(result["resultType"], "task", "{body}");
    assert_eq!(result["status"], "working");
    assert!(result["pollIntervalMs"].is_number());
    assert!(result.get("ttlMs").is_some(), "ttlMs is required: {body}");
    let task_id = result["taskId"].as_str().expect("taskId").to_string();

    // Durable before response: an immediate get must find it.
    let got = tasks_get(&url, &task_id).await;
    assert!(got["result"]["status"].is_string(), "{got}");

    let done = poll_until_terminal(&url, &task_id).await;
    assert_eq!(done["result"]["status"], "completed", "{done}");
    assert_eq!(
        done["result"]["result"]["content"][0]["text"], "42",
        "completed result carries the tool's CallToolResult: {done}"
    );
}

/// Progressive enhancement: same tool, extension NOT declared → ordinary
/// synchronous CallToolResult.
#[tokio::test]
async fn undeclared_call_runs_synchronously() {
    let url = start_server().await;
    let (status, body) = call_tool(&url, "slow_double", json!({ "n": 4 }), false).await;
    assert_eq!(status, 200, "{body}");
    assert!(
        body["result"].get("resultType").is_none() || body["result"]["resultType"] == "complete",
        "must not be a task: {body}"
    );
    assert_eq!(body["result"]["content"][0]["text"], "8", "{body}");
}

/// A task-REQUIRED tool without the declared extension → -32021 with the
/// upstream overview's exact data shape.
#[tokio::test]
async fn required_tool_without_extension_is_32021() {
    let url = start_server().await;
    let (status, body) = call_tool(&url, "must_be_task", json!({}), false).await;
    assert_eq!(status, 400, "{body}");
    assert_eq!(body["error"]["code"], -32021, "{body}");
    assert!(
        body["error"]["data"]["requiredCapabilities"]["extensions"][EXT].is_object(),
        "data.requiredCapabilities.extensions must name the extension: {body}"
    );
}

/// input_required ⇄ tasks/update round trip: the worker parks on the tool's
/// MRTR demand and resumes with the delivered responses.
#[tokio::test]
async fn input_required_round_trip_via_tasks_update() {
    let url = start_server().await;
    let (status, body) = call_tool(&url, "needs_approval", json!({}), true).await;
    assert_eq!(status, 200, "{body}");
    let task_id = body["result"]["taskId"]
        .as_str()
        .expect("taskId")
        .to_string();

    // Poll until the worker parks in input_required with the elicit request.
    let mut input_seen = Value::Null;
    for _ in 0..100 {
        let got = tasks_get(&url, &task_id).await;
        if got["result"]["status"] == "input_required" {
            input_seen = got;
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(
        input_seen["result"]["status"], "input_required",
        "{input_seen}"
    );
    assert_eq!(
        input_seen["result"]["inputRequests"]["approval"]["method"], "elicitation/create",
        "{input_seen}"
    );

    // Deliver the approval.
    let (status, ack) = post(
        &url,
        "tasks/update",
        None,
        json!({
            "jsonrpc": "2.0", "id": 3, "method": "tasks/update",
            "params": {
                "_meta": meta(true),
                "taskId": task_id,
                "inputResponses": {
                    "approval": { "action": "accept", "content": { "approved": true } }
                }
            }
        }),
    )
    .await;
    assert_eq!(status, 200, "{ack}");
    assert_eq!(ack["result"]["resultType"], "complete", "{ack}");

    let done = poll_until_terminal(&url, &task_id).await;
    assert_eq!(done["result"]["status"], "completed", "{done}");
    assert_eq!(
        done["result"]["result"]["content"][0]["text"], "approved",
        "{done}"
    );
}

/// tasks/cancel mid-working → cancelled (cooperative); cancelling a terminal
/// task still acks.
#[tokio::test]
async fn cancel_flips_working_to_cancelled_and_acks_terminal() {
    let url = start_server().await;
    let (_, body) = call_tool(&url, "slow_double", json!({ "n": 1 }), true).await;
    let task_id = body["result"]["taskId"]
        .as_str()
        .expect("taskId")
        .to_string();

    let (status, ack) = post(
        &url,
        "tasks/cancel",
        None,
        json!({
            "jsonrpc": "2.0", "id": 4, "method": "tasks/cancel",
            "params": { "_meta": meta(true), "taskId": task_id }
        }),
    )
    .await;
    assert_eq!(status, 200, "{ack}");

    let got = tasks_get(&url, &task_id).await;
    assert_eq!(got["result"]["status"], "cancelled", "{got}");

    // Second cancel on the now-terminal task still acks (eventually consistent).
    let (status, ack2) = post(
        &url,
        "tasks/cancel",
        None,
        json!({
            "jsonrpc": "2.0", "id": 5, "method": "tasks/cancel",
            "params": { "_meta": meta(true), "taskId": got["result"]["taskId"] }
        }),
    )
    .await;
    assert_eq!(status, 200, "{ack2}");
}

/// Unknown taskId → -32602.
#[tokio::test]
async fn unknown_task_id_is_invalid_params() {
    let url = start_server().await;
    let (status, body) = post(
        &url,
        "tasks/get",
        None,
        json!({
            "jsonrpc": "2.0", "id": 6, "method": "tasks/get",
            "params": { "_meta": meta(true), "taskId": "nope" }
        }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["error"]["code"], -32602, "{body}");
}

/// Task IDs are bearer-token-grade: 32 hex chars of UUIDv4, not enumerable.
#[tokio::test]
async fn task_ids_are_unguessable_uuids() {
    let url = start_server().await;
    let (_, a) = call_tool(&url, "slow_double", json!({ "n": 1 }), true).await;
    let (_, b) = call_tool(&url, "slow_double", json!({ "n": 2 }), true).await;
    let ida = a["result"]["taskId"].as_str().unwrap();
    let idb = b["result"]["taskId"].as_str().unwrap();
    assert_eq!(ida.len(), 32);
    assert_ne!(ida, idb);
    // v4 marker: the 13th hex digit is the version nibble.
    assert_eq!(&ida[12..13], "4", "task ids must be UUIDv4: {ida}");
}

/// `notifications/tasks` rides `subscriptions/listen` filtered by `taskIds`:
/// the matching task's terminal notification arrives; the ack echoes the
/// honored taskIds.
#[tokio::test]
async fn task_notifications_ride_listen_filtered_by_task_id() {
    let url = start_server().await;

    // Create the task first (its id goes into the filter).
    let (_, body) = call_tool(&url, "slow_double", json!({ "n": 5 }), true).await;
    let task_id = body["result"]["taskId"]
        .as_str()
        .expect("taskId")
        .to_string();

    // Open the listen stream with that taskId before the worker finishes
    // (slow_double sleeps 150ms).
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Accept", "text/event-stream")
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "subscriptions/listen")
        .json(&json!({
            "jsonrpc": "2.0", "id": 9, "method": "subscriptions/listen",
            "params": {
                "_meta": meta(true),
                "notifications": { "taskIds": [task_id] }
            }
        }))
        .send()
        .await
        .expect("listen POST");
    assert_eq!(resp.status(), 200);

    // Drain SSE frames: first the ack (echoing taskIds), then the terminal
    // notifications/tasks for our task.
    let mut stream = resp.bytes_stream();
    use futures::StreamExt;
    let mut buf = String::new();
    let mut saw_ack_task_ids = false;
    let mut saw_task_notification = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while tokio::time::Instant::now() < deadline {
        let chunk = match tokio::time::timeout(Duration::from_millis(500), stream.next()).await {
            Ok(Some(Ok(c))) => c,
            _ => break,
        };
        buf.push_str(&String::from_utf8_lossy(&chunk));
        for frame in buf.split("\n\n") {
            for line in frame.lines() {
                if let Some(data) = line.strip_prefix("data: ")
                    && let Ok(v) = serde_json::from_str::<Value>(data)
                {
                    match v["method"].as_str() {
                        Some("notifications/subscriptions/acknowledged") => {
                            if v["params"]["notifications"]["taskIds"][0] == json!(task_id) {
                                saw_ack_task_ids = true;
                            }
                        }
                        Some("notifications/tasks")
                            if v["params"]["taskId"] == json!(task_id)
                                && v["params"]["status"] == "completed" =>
                        {
                            saw_task_notification = true;
                        }
                        _ => {}
                    }
                }
            }
        }
        if saw_ack_task_ids && saw_task_notification {
            break;
        }
    }
    assert!(saw_ack_task_ids, "ack must echo honored taskIds: {buf}");
    assert!(
        saw_task_notification,
        "terminal notifications/tasks must arrive on the filtered stream: {buf}"
    );
}

/// State Handle Hijacking (Security Best Practices): a task id is a state
/// handle bound to the principal that created it. A task created by `alice`
/// MUST NOT be readable, updatable, or cancellable by `bob` — each attempt
/// must fail exactly like an unknown task id (masking existence), and none
/// of bob's attempts may actually mutate alice's task.
#[tokio::test]
async fn task_bound_to_owner_rejects_other_principal() {
    let url = start_server_with_principals().await;

    let (status, body) = call_tool_as(&url, "needs_approval", json!({}), "alice").await;
    assert_eq!(status, 200, "{body}");
    let task_id = body["result"]["taskId"]
        .as_str()
        .expect("taskId")
        .to_string();

    // Wait for the worker to park in input_required, as alice.
    poll_until_input_required_as(&url, &task_id, "alice").await;

    // bob cannot read it.
    let (status, body) = tasks_get_as(&url, &task_id, "bob").await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body["error"]["code"], -32602,
        "bob's tasks/get on alice's task must look exactly like an unknown task id: {body}"
    );

    // bob cannot cancel it.
    let (status, body) = tasks_cancel_as(&url, &task_id, "bob").await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body["error"]["code"], -32602,
        "bob's tasks/cancel on alice's task must look exactly like an unknown task id: {body}"
    );

    // bob cannot deliver input responses to it.
    let (status, body) = tasks_update_as(
        &url,
        &task_id,
        "bob",
        json!({ "approval": { "action": "accept", "content": { "approved": true } } }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body["error"]["code"], -32602,
        "bob's tasks/update on alice's task must look exactly like an unknown task id: {body}"
    );

    // None of bob's attempts moved the task: alice still sees input_required.
    let (status, still_pending) = tasks_get_as(&url, &task_id, "alice").await;
    assert_eq!(status, 200, "{still_pending}");
    assert_eq!(
        still_pending["result"]["status"], "input_required",
        "bob's rejected calls must not have mutated alice's task: {still_pending}"
    );

    // alice still owns it end to end: she can deliver input and the task completes.
    let (status, ack) = tasks_update_as(
        &url,
        &task_id,
        "alice",
        json!({ "approval": { "action": "accept", "content": { "approved": true } } }),
    )
    .await;
    assert_eq!(status, 200, "{ack}");
    assert_eq!(ack["result"]["resultType"], "complete", "{ack}");

    let mut done = Value::Null;
    for _ in 0..100 {
        let (status, body) = tasks_get_as(&url, &task_id, "alice").await;
        assert_eq!(status, 200, "{body}");
        if ["completed", "failed", "cancelled"]
            .contains(&body["result"]["status"].as_str().unwrap_or(""))
        {
            done = body;
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(done["result"]["status"], "completed", "{done}");
    assert_eq!(
        done["result"]["result"]["content"][0]["text"], "approved",
        "alice's own delivery must resume the worker: {done}"
    );
}
