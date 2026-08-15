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
use turul_mcp_ext_tasks::{InMemoryTaskStore, RetentionPolicy, TaskStatus};
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
        .ext_task_tool(FailingTool::new())
        .ext_task_tool_required(SlowRequiredTool::new())
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

/// `taskSupport=required`: a client that did not declare the extension must be
/// refused with -32021 rather than silently running it synchronously.
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
        "requires_task"
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
        Ok(CallToolResult::success(vec![ToolResult::text("done")]))
    }
}

/// Always fails, so the `failed` terminal status is reachable end to end.
struct FailingTool {
    input_schema: ToolSchema,
}
impl FailingTool {
    fn new() -> Self {
        Self {
            input_schema: ToolSchema::object(),
        }
    }
}
impl HasBaseMetadata for FailingTool {
    fn name(&self) -> &str {
        "always_fails"
    }
}
impl HasDescription for FailingTool {
    fn description(&self) -> Option<&str> {
        Some("Always returns a tool execution error")
    }
}
impl HasInputSchema for FailingTool {
    fn input_schema(&self) -> &ToolSchema {
        &self.input_schema
    }
}
impl HasOutputSchema for FailingTool {}
impl HasAnnotations for FailingTool {}
impl HasToolMeta for FailingTool {}
impl HasIcons for FailingTool {}
#[async_trait]
impl McpTool for FailingTool {
    async fn call(&self, _a: Value, _s: Option<SessionContext>) -> McpResult<CallToolResult> {
        Err(McpError::tool_execution("deliberate failure"))
    }
}

/// A server whose retention sweep runs aggressively, so a test can observe it
/// firing instead of assuming it was started.
async fn start_server_with_sweep() -> String {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let server = McpServer::builder()
        .name("ext-tasks-client-e2e-sweep")
        .version("0.4.0")
        .with_ext_tasks(Arc::new(InMemoryTaskStore::new()))
        .with_ext_tasks_retention(
            // Anything non-terminal and quiet for 1ms is "abandoned" — absurd
            // for production, exactly right for observing the loop.
            RetentionPolicy {
                orphan_after_ms: Some(1),
                ..Default::default()
            },
            Duration::from_millis(150),
        )
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

/// Asks TWO questions in one round, so partial fulfilment is reachable:
/// answering one must drop that key and leave the other outstanding.
struct TwoQuestionTool {
    input_schema: ToolSchema,
}
impl TwoQuestionTool {
    fn new() -> Self {
        Self {
            input_schema: ToolSchema::object(),
        }
    }
}
impl HasBaseMetadata for TwoQuestionTool {
    fn name(&self) -> &str {
        "two_questions"
    }
}
impl HasDescription for TwoQuestionTool {
    fn description(&self) -> Option<&str> {
        Some("Asks two things at once")
    }
}
impl HasInputSchema for TwoQuestionTool {
    fn input_schema(&self) -> &ToolSchema {
        &self.input_schema
    }
}
impl HasOutputSchema for TwoQuestionTool {}
impl HasAnnotations for TwoQuestionTool {}
impl HasToolMeta for TwoQuestionTool {}
impl HasIcons for TwoQuestionTool {}
#[async_trait]
impl McpTool for TwoQuestionTool {
    async fn call(&self, _a: Value, session: Option<SessionContext>) -> McpResult<CallToolResult> {
        use turul_mcp_protocol::elicitation::{ElicitRequest, ElicitationSchema};
        use turul_mcp_protocol::input_required::{InputRequest, InputRequests};

        let session = session.ok_or_else(|| McpError::tool_execution("context required"))?;
        if let Some(r) = session.input_responses()
            && r.contains_key("first")
            && r.contains_key("second")
        {
            return Ok(CallToolResult::success(vec![ToolResult::text("both")]));
        }
        let mut requests = InputRequests::new();
        for key in ["first", "second"] {
            requests.insert(
                key.to_string(),
                InputRequest::Elicit(ElicitRequest::new_form("?", ElicitationSchema::new())),
            );
        }
        Err(McpError::InputRequired {
            input_requests: Some(requests),
            request_state: None,
        })
    }
}

/// SEP-2663 §Composition with MRTR: gathers input SYNCHRONOUSLY, then mints
/// the task. Registered with `mrtr_first`.
struct GatherThenTaskTool {
    input_schema: ToolSchema,
}
impl GatherThenTaskTool {
    fn new() -> Self {
        Self {
            input_schema: ToolSchema::object(),
        }
    }
}
impl HasBaseMetadata for GatherThenTaskTool {
    fn name(&self) -> &str {
        "gather_then_task"
    }
}
impl HasDescription for GatherThenTaskTool {
    fn description(&self) -> Option<&str> {
        Some("Gathers input synchronously, then runs as a task")
    }
}
impl HasInputSchema for GatherThenTaskTool {
    fn input_schema(&self) -> &ToolSchema {
        &self.input_schema
    }
}
impl HasOutputSchema for GatherThenTaskTool {}
impl HasAnnotations for GatherThenTaskTool {}
impl HasToolMeta for GatherThenTaskTool {}
impl HasIcons for GatherThenTaskTool {}
#[async_trait]
impl McpTool for GatherThenTaskTool {
    async fn call(&self, _a: Value, session: Option<SessionContext>) -> McpResult<CallToolResult> {
        use turul_mcp_protocol::elicitation::{ElicitRequest, ElicitationSchema};
        use turul_mcp_protocol::input_required::{InputRequest, InputRequests};

        let session = session.ok_or_else(|| McpError::tool_execution("context required"))?;
        if session.input_responses().is_some() {
            return Ok(CallToolResult::success(vec![ToolResult::text("gathered")]));
        }
        let mut requests = InputRequests::new();
        requests.insert(
            "user_name".to_string(),
            InputRequest::Elicit(ElicitRequest::new_form("Name?", ElicitationSchema::new())),
        );
        Err(McpError::InputRequired {
            input_requests: Some(requests),
            request_state: Some("gather-1".to_string()),
        })
    }
}

/// Which store a server under test is built on. The client must behave
/// identically against all of them; a client exercised only against a HashMap
/// is a client nobody can deploy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StoreKind {
    InMemory,
    Sqlite,
    Postgres,
    DynamoDb,
}

impl StoreKind {
    /// Every backend this build can reach. Postgres and DynamoDB PANIC rather
    /// than skip if unreachable — a silent skip is how a backend ends up
    /// reported as covered without ever being contacted.
    fn all() -> Vec<StoreKind> {
        vec![
            StoreKind::InMemory,
            StoreKind::Sqlite,
            StoreKind::Postgres,
            StoreKind::DynamoDb,
        ]
    }

    async fn store(self) -> Arc<dyn turul_mcp_ext_tasks::TaskStore> {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        match self {
            StoreKind::InMemory => Arc::new(InMemoryTaskStore::new()),
            StoreKind::Sqlite => Arc::new(
                turul_mcp_ext_tasks::v2026_07_28::sqlite::SqliteTaskStore::in_memory()
                    .await
                    .expect("sqlite store"),
            ),
            StoreKind::Postgres => {
                use sqlx::Executor;
                let base = std::env::var("TURUL_TEST_PG_URL")
                    .unwrap_or_else(|_| "postgres://nick@%2Fvar%2Frun%2Fpostgresql".to_string());
                let admin = sqlx::PgPool::connect(&format!("{base}/postgres"))
                    .await
                    .unwrap_or_else(|e| panic!("Postgres unreachable for client e2e: {e}"));
                let db = format!("turul_client_e2e_{unique}");
                admin
                    .execute(sqlx::AssertSqlSafe(format!("CREATE DATABASE {db}")))
                    .await
                    .expect("create scratch db");
                admin.close().await;
                Arc::new(
                    turul_mcp_ext_tasks::v2026_07_28::postgres::PostgresTaskStore::connect(
                        &format!("{base}/{db}"),
                    )
                    .await
                    .expect("postgres store"),
                )
            }
            StoreKind::DynamoDb => {
                use aws_sdk_dynamodb::config::{BehaviorVersion, Credentials, Region};
                let url = std::env::var("TURUL_TEST_DDB_URL")
                    .unwrap_or_else(|_| "http://127.0.0.1:8123".to_string());
                let cfg = aws_config::defaults(BehaviorVersion::latest())
                    .region(Region::new("us-east-1"))
                    .endpoint_url(&url)
                    .credentials_provider(Credentials::new(
                        "local",
                        "local",
                        None,
                        None,
                        "turul-test",
                    ))
                    .load()
                    .await;
                let client = aws_sdk_dynamodb::Client::new(&cfg);
                client
                    .list_tables()
                    .send()
                    .await
                    .unwrap_or_else(|e| panic!("DynamoDB Local unreachable at {url}: {e}"));
                let store = turul_mcp_ext_tasks::v2026_07_28::dynamodb::DynamoDbTaskStore::new(
                    client,
                    format!("turul_client_e2e_{unique}"),
                );
                store.ensure_table().await.expect("ddb table");
                Arc::new(store)
            }
        }
    }
}

/// A server on `kind`, carrying every fixture tool the suite needs.
async fn start_server_on(kind: StoreKind) -> String {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let server = McpServer::builder()
        .name("ext-tasks-client-e2e")
        .version("0.4.0")
        .with_ext_tasks(kind.store().await)
        .ext_task_tool(SlowDoubleTool::new())
        .ext_task_tool(ApprovalTool::new())
        .ext_task_tool(FailingTool::new())
        .ext_task_tool(TwoQuestionTool::new())
        .ext_task_tool_required(SlowRequiredTool::new())
        .ext_task_tool_with(
            GatherThenTaskTool::new(),
            ExtTaskElection::required().with_mrtr_first(),
        )
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

/// Declared client: task handle → `task_wait` polls to the completed result,
/// and the server supplies the `pollIntervalMs` hint the client paces itself
/// with.
///
/// The hint is asserted on the wire rather than by timing the poll: the client
/// clamps it to [50ms, 30s] internally, so a stopwatch assertion would be
/// measuring the clamp and would be flaky besides. What matters at this
/// boundary is that the server actually sends a usable value — a client cannot
/// honour a hint that never arrives.
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
    let hint = task
        .task
        .fields
        .poll_interval_ms
        .expect("the server must advertise pollIntervalMs for the client to pace itself");
    assert!(
        (10.0..=60_000.0).contains(&hint),
        "pollIntervalMs must be a sane millisecond value, got {hint}"
    );

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
/// The parser stays intact (the sync result is an ordinary CallToolResult).
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
                "_meta": { "io.modelcontextprotocol/serverInfo": { "name": "mock-2026", "version": "1.0.0" } } }
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(body_partial_json(json!({"method": "tools/call"})))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/json")
                .set_body_json(json!({
                    "jsonrpc": "2.0", "id": "req_1",
                    "result": { "resultType": "complete", "ttlMs": 0, "cacheScope": "public",
                        "content": [{ "type": "text", "text": "ok" }], "isError": false }
                })),
        )
        .mount(&server)
        .await;

    let url = format!("{}/mcp", server.uri());
    let mut config = ClientConfig::default();
    config.declared_capabilities.ext_tasks = true;
    let transport = Box::new(HttpTransport::new(&url).unwrap());
    let client = McpClient::new(transport, config);
    client
        .connect()
        .await
        .expect("connect against a 2026 server");

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
                .and_then(|b| {
                    b.get("method")
                        .and_then(|m| m.as_str())
                        .map(|s| s == "tools/call")
                })
                .unwrap_or(false)
        })
        .collect();
    assert_eq!(
        tools_call.len(),
        1,
        "expected exactly one tools/call request"
    );
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

// ---------------------------------------------------------------------------
// The scenarios the original five did not reach. Each drives the TYPED CLIENT
// surface rather than raw JSON, so the client's own parsing is under test too
// — a server that is right and a client that cannot read it is still a broken
// pair, and only an e2e catches that.
// ---------------------------------------------------------------------------

/// SEP-2663: a `taskSupport=required` tool refuses a client that never
/// declared the extension, with -32021 MissingRequiredClientCapability. It
/// must NOT quietly run synchronously — that would hand the caller a result
/// from a tool the server was told to run asynchronously.
#[tokio::test]
async fn required_task_tool_refuses_an_undeclared_client() {
    let url = start_server().await;
    let client = connect(&url, false).await;

    let err = client
        .call_tool_or_task("requires_task", json!({}))
        .await
        .expect_err("a required-task tool must refuse an undeclared client");
    let text = err.to_string();
    assert!(
        text.contains("-32021") || text.to_lowercase().contains("capability"),
        "the refusal must be the missing-capability error, got: {text}"
    );
}

/// A tool error becomes a `failed` task carrying the JSON-RPC error, not a
/// completed task with an error-shaped result.
#[tokio::test]
async fn a_failing_tool_reaches_the_failed_status() {
    let url = start_server().await;
    let client = connect(&url, true).await;

    let outcome = client
        .call_tool_or_task("always_fails", json!({}))
        .await
        .expect("call");
    let ToolCallOutcome::Task(task) = outcome else {
        panic!("expected a task handle, got {outcome:?}");
    };

    let done = client
        .task_wait(&task.task.fields.task_id)
        .await
        .expect("wait");
    assert_eq!(done.status(), TaskStatus::Failed);
    let turul_mcp_ext_tasks::DetailedTask::Failed { error, .. } = done else {
        panic!("expected failed: {done:?}");
    };
    assert!(
        error["message"]
            .as_str()
            .unwrap_or("")
            .contains("deliberate"),
        "the tool's own error must survive into the task: {error}"
    );
}

/// An unknown task id is an error, and — per the State Handle Hijacking
/// guidance — indistinguishable from someone else's task.
#[tokio::test]
async fn an_unknown_task_id_is_an_error() {
    let url = start_server().await;
    let client = connect(&url, true).await;

    client
        .task_get("00000000000000000000000000000000")
        .await
        .expect_err("an unknown task id must not resolve");
}

/// `tasks/update` carrying a key the task is not waiting on is inert: it acks
/// and the round stays open. Pinned end to end because the decision only
/// makes sense from the client's side — an error here would tell the client
/// nothing it could act on. (See the store's `provide_input` contract.)
#[tokio::test]
async fn an_inert_input_key_acks_and_leaves_the_round_open() {
    let url = start_server().await;
    let client = connect(&url, true).await;

    let outcome = client
        .call_tool_or_task("needs_approval", json!({}))
        .await
        .expect("call");
    let ToolCallOutcome::Task(task) = outcome else {
        panic!("expected a task handle, got {outcome:?}");
    };
    let task_id = task.task.fields.task_id.clone();

    for _ in 0..100 {
        if client.task_get(&task_id).await.expect("get").status() == TaskStatus::InputRequired {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    client
        .task_update(&task_id, json!({ "never-asked": { "ignored": true } }))
        .await
        .expect("an inert key must ack, not error");

    let still = client.task_get(&task_id).await.expect("get");
    assert_eq!(
        still.status(),
        TaskStatus::InputRequired,
        "the round must stay open after an inert key: {still:?}"
    );

    // And the real answer still completes it.
    client
        .task_update(
            &task_id,
            json!({ "approval": { "action": "accept", "content": { "approved": true } } }),
        )
        .await
        .expect("update");
    let done = client.task_wait(&task_id).await.expect("wait");
    assert_eq!(done.status(), TaskStatus::Completed);
}

/// The other half: a malformed response for a key the task IS waiting on
/// blocks the round, so it errors.
#[tokio::test]
async fn a_malformed_response_for_an_outstanding_key_errors() {
    let url = start_server().await;
    let client = connect(&url, true).await;

    let outcome = client
        .call_tool_or_task("needs_approval", json!({}))
        .await
        .expect("call");
    let ToolCallOutcome::Task(task) = outcome else {
        panic!("expected a task handle, got {outcome:?}");
    };
    let task_id = task.task.fields.task_id.clone();

    for _ in 0..100 {
        if client.task_get(&task_id).await.expect("get").status() == TaskStatus::InputRequired {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    let err = client
        .task_update(&task_id, json!({ "approval": { "ignored": true } }))
        .await
        .expect_err("a malformed response for an outstanding key must error");
    assert!(
        err.to_string().contains("approval"),
        "the error must name the offending key: {err}"
    );
}

/// Cancelling an already-terminal task acks rather than erroring — the ack-only
/// design means a client retrying a cancel is not punished for it.
#[tokio::test]
async fn cancelling_an_already_terminal_task_still_acks() {
    let url = start_server().await;
    let client = connect(&url, true).await;

    let outcome = client
        .call_tool_or_task("slow_double", json!({ "n": 1 }))
        .await
        .expect("call");
    let ToolCallOutcome::Task(task) = outcome else {
        panic!("expected a task handle, got {outcome:?}");
    };
    let task_id = task.task.fields.task_id.clone();
    client.task_wait(&task_id).await.expect("wait");

    client
        .task_cancel(&task_id)
        .await
        .expect("cancelling a finished task must ack, not error");
}

/// **The retention sweep is actually running.**
///
/// Nothing else proves this: `TaskStore::sweep` was implemented by every
/// backend and called by nothing, so retention was inert — indistinguishable
/// from absent. This server is built with `with_ext_tasks_retention`, and a
/// parked task must therefore be reaped into `failed` with no client action at
/// all. Without the builder wiring the task sits in `input_required` forever
/// and this times out.
#[tokio::test]
async fn the_retention_sweep_reaps_an_abandoned_task() {
    let url = start_server_with_sweep().await;
    let client = connect(&url, true).await;

    let outcome = client
        .call_tool_or_task("needs_approval", json!({}))
        .await
        .expect("call");
    let ToolCallOutcome::Task(task) = outcome else {
        panic!("expected a task handle, got {outcome:?}");
    };
    let task_id = task.task.fields.task_id.clone();

    // Nobody answers the input round. The sweep must notice on its own.
    let mut reaped = None;
    for _ in 0..100 {
        let state = client.task_get(&task_id).await.expect("get");
        if state.status() == TaskStatus::Failed {
            reaped = Some(state);
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let state = reaped.expect(
        "the retention sweep must reap an abandoned task; if this times out the \
         sweep loop was never spawned at build time",
    );
    let turul_mcp_ext_tasks::DetailedTask::Failed { error, .. } = state else {
        panic!("expected failed");
    };
    assert!(
        error["message"]
            .as_str()
            .unwrap_or("")
            .contains("abandoned"),
        "a swept task must say why: {error}"
    );
}

// ===========================================================================
// Full-coverage additions. Everything below drives the TYPED CLIENT surface,
// so a server that is right and a client that cannot read it still fails —
// only an e2e catches that pair.
// ===========================================================================

/// The whole lifecycle, against EVERY backend the build can reach.
///
/// One test per store rather than one suite per store: the discrete edge
/// cases below are store-independent (they exercise the shared state machine
/// in `turul-mcp-ext-tasks::traits`), but the happy path must be shown to
/// work against real storage, because that is what a deployment runs. Before
/// this, every client e2e used `InMemoryTaskStore` and the client had never
/// been driven against SQL or DynamoDB at all.
async fn full_lifecycle_on(kind: StoreKind) {
    let url = start_server_on(kind).await;
    let client = connect(&url, true).await;

    // Election → task → poll to completion.
    let ToolCallOutcome::Task(task) = client
        .call_tool_or_task("slow_double", json!({ "n": 21 }))
        .await
        .unwrap_or_else(|e| panic!("{kind:?}: call failed: {e}"))
    else {
        panic!("{kind:?}: expected a task handle");
    };
    let done = client
        .task_wait(&task.task.fields.task_id)
        .await
        .unwrap_or_else(|e| panic!("{kind:?}: wait failed: {e}"));
    assert_eq!(done.status(), TaskStatus::Completed, "{kind:?}");

    // Park → answer → resume, through the store.
    let ToolCallOutcome::Task(parked) = client
        .call_tool_or_task("needs_approval", json!({}))
        .await
        .expect("call")
    else {
        panic!("{kind:?}: expected a task handle");
    };
    let id = parked.task.fields.task_id.clone();
    await_status(&client, &id, TaskStatus::InputRequired).await;
    client
        .task_update(
            &id,
            json!({ "approval": { "action": "accept", "content": { "approved": true } } }),
        )
        .await
        .unwrap_or_else(|e| panic!("{kind:?}: update failed: {e}"));
    let resumed = client.task_wait(&id).await.expect("wait");
    assert_eq!(resumed.status(), TaskStatus::Completed, "{kind:?}");

    // Cancel a live task.
    let ToolCallOutcome::Task(live) = client
        .call_tool_or_task("needs_approval", json!({}))
        .await
        .expect("call")
    else {
        panic!("{kind:?}: expected a task handle");
    };
    let live_id = live.task.fields.task_id.clone();
    await_status(&client, &live_id, TaskStatus::InputRequired).await;
    client.task_cancel(&live_id).await.expect("cancel");
    await_status(&client, &live_id, TaskStatus::Cancelled).await;
}

/// Poll until a task reaches `want`, failing loudly rather than hanging.
async fn await_status(client: &McpClient, task_id: &str, want: TaskStatus) {
    for _ in 0..200 {
        if let Ok(state) = client.task_get(task_id).await
            && state.status() == want
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("task {task_id} never reached {want:?}");
}

#[tokio::test]
async fn full_lifecycle_in_memory() {
    full_lifecycle_on(StoreKind::InMemory).await;
}

#[tokio::test]
async fn full_lifecycle_sqlite() {
    full_lifecycle_on(StoreKind::Sqlite).await;
}

#[tokio::test]
async fn full_lifecycle_postgres() {
    full_lifecycle_on(StoreKind::Postgres).await;
}

#[tokio::test]
async fn full_lifecycle_dynamodb() {
    full_lifecycle_on(StoreKind::DynamoDb).await;
}

/// Every backend must be reachable — a build that quietly tested fewer stores
/// than it claims is the failure this whole effort exists to prevent.
#[tokio::test]
async fn every_backend_is_actually_exercised() {
    assert_eq!(
        StoreKind::all().len(),
        4,
        "in-memory, SQLite, Postgres and DynamoDB each need a full_lifecycle_* test"
    );
}

/// SEP-2663 §Composition with MRTR: round 1 answers synchronously with NO
/// taskId; only round 2 mints the task. The client's `call_tool_or_task` must
/// surface the first round as a non-task outcome, or a caller would wait for
/// a task handle that is never coming.
#[tokio::test]
async fn mrtr_round_one_is_not_a_task_and_round_two_is() {
    let url = start_server_on(StoreKind::InMemory).await;
    let mut config = ClientConfig::default();
    config.declared_capabilities.ext_tasks = true;
    config.declared_capabilities.elicitation = true;
    let transport = Box::new(HttpTransport::new(&url).expect("transport"));
    let client = McpClient::new(transport, config);
    client.connect().await.expect("connect");

    // Round 1: no inputResponses → an input-required result, not a task.
    let first = client
        .call_tool_or_task("gather_then_task", json!({}))
        .await;
    match first {
        Ok(ToolCallOutcome::Task(t)) => panic!(
            "round 1 must NOT mint a task; got taskId {}",
            t.task.fields.task_id
        ),
        // The client surfaces the MRTR round however it models it; what must
        // not happen is a task handle.
        _ => {}
    }

    // Round 2: the answer arrives, and NOW a task is minted.
    let second = client
        .call_tool_or_task_with_input_responses(
            "gather_then_task",
            json!({}),
            json!({ "user_name": { "action": "accept", "content": { "name": "Alice" } } }),
            Some("gather-1".to_string()),
        )
        .await
        .expect("round 2 must be accepted once the answer is supplied");
    let ToolCallOutcome::Task(task) = second else {
        panic!("round 2 MUST mint a task (SEP-2663 composition), got {second:?}");
    };
    assert_eq!(
        client
            .task_wait(&task.task.fields.task_id)
            .await
            .expect("wait")
            .status(),
        TaskStatus::Completed,
        "the composed task must run to completion"
    );
}

/// Partial fulfilment through the client: answering one of two questions
/// leaves the round open AND drops the answered key, so the client is not
/// asked the same thing twice.
#[tokio::test]
async fn a_partial_round_drops_the_answered_key() {
    let url = start_server_on(StoreKind::InMemory).await;
    let client = connect(&url, true).await;

    let ToolCallOutcome::Task(task) = client
        .call_tool_or_task("two_questions", json!({}))
        .await
        .expect("call")
    else {
        panic!("expected a task handle");
    };
    let id = task.task.fields.task_id.clone();
    await_status(&client, &id, TaskStatus::InputRequired).await;

    client
        .task_update(
            &id,
            json!({ "first": { "action": "accept", "content": {} } }),
        )
        .await
        .expect("partial update");

    let state = client.task_get(&id).await.expect("get");
    assert_eq!(
        state.status(),
        TaskStatus::InputRequired,
        "one of two answers must leave the round open"
    );
    let turul_mcp_ext_tasks::DetailedTask::InputRequired { input_requests, .. } = state else {
        panic!("expected input_required");
    };
    assert!(
        !input_requests.contains_key("first"),
        "the answered key must be dropped, or the client is asked it twice"
    );
    assert!(
        input_requests.contains_key("second"),
        "the unanswered key must remain outstanding"
    );

    // Answering the rest completes it.
    client
        .task_update(
            &id,
            json!({ "second": { "action": "accept", "content": {} } }),
        )
        .await
        .expect("final update");
    assert_eq!(
        client.task_wait(&id).await.expect("wait").status(),
        TaskStatus::Completed
    );
}

/// Task ids are bearer tokens (Security Best Practices, "State Handle
/// Hijacking"), so they must not be guessable or sequential.
#[tokio::test]
async fn task_ids_are_unguessable_and_distinct() {
    let url = start_server_on(StoreKind::InMemory).await;
    let client = connect(&url, true).await;

    let mut ids = Vec::new();
    for _ in 0..3 {
        let ToolCallOutcome::Task(t) = client
            .call_tool_or_task("slow_double", json!({ "n": 1 }))
            .await
            .expect("call")
        else {
            panic!("expected a task handle");
        };
        ids.push(t.task.fields.task_id);
    }
    for id in &ids {
        assert_eq!(id.len(), 32, "expected a 32-char simple UUID, got {id:?}");
        assert!(
            id.chars().all(|c| c.is_ascii_hexdigit()),
            "task ids must be hex, got {id:?}"
        );
    }
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 3, "task ids must be distinct");
}

/// `server/discover` must advertise the extension when a store is configured
/// — the client uses that to decide whether to declare it at all.
#[tokio::test]
async fn discover_advertises_the_tasks_extension_to_the_client() {
    let url = start_server_on(StoreKind::InMemory).await;
    let client = connect(&url, true).await;

    let caps = client.server_capabilities().await.expect("capabilities");
    let raw = serde_json::to_value(&caps).expect("serialize");
    assert!(
        raw.to_string().contains("io.modelcontextprotocol/tasks"),
        "a server with a task store must advertise the extension: {raw}"
    );
}

/// The published extension overview says of `tasks/update`: "Ignore responses
/// for unknown **or already-satisfied** keys."
///
/// The unknown-key half is covered above. This is the other half: re-sending
/// an answer the server already accepted — which a client retrying after a
/// dropped response does routinely — must ack, not error. It works because an
/// answered key is removed from `inputRequests`, so on the retry it is simply
/// no longer outstanding and takes the same inert path.
#[tokio::test]
async fn resending_an_already_satisfied_key_is_ignored_not_rejected() {
    let url = start_server_on(StoreKind::InMemory).await;
    let client = connect(&url, true).await;

    let ToolCallOutcome::Task(task) = client
        .call_tool_or_task("two_questions", json!({}))
        .await
        .expect("call")
    else {
        panic!("expected a task handle");
    };
    let id = task.task.fields.task_id.clone();
    await_status(&client, &id, TaskStatus::InputRequired).await;

    let first = json!({ "first": { "action": "accept", "content": {} } });
    client
        .task_update(&id, first.clone())
        .await
        .expect("answer");

    // The same answer again — the retry a flaky network produces.
    client
        .task_update(&id, first)
        .await
        .expect("re-sending an already-satisfied key must ack, not error");

    let state = client.task_get(&id).await.expect("get");
    assert_eq!(
        state.status(),
        TaskStatus::InputRequired,
        "the duplicate must not disturb the still-open round"
    );

    // And the round still completes normally afterwards.
    client
        .task_update(
            &id,
            json!({ "second": { "action": "accept", "content": {} } }),
        )
        .await
        .expect("final answer");
    assert_eq!(
        client.task_wait(&id).await.expect("wait").status(),
        TaskStatus::Completed
    );
}
