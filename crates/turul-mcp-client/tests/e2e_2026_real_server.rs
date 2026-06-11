//! End-to-end: the bilingual client against a REAL 2026-07-28 stateless server
//! (in-process `turul-mcp-server` on an ephemeral port — no mocks).
//!
//! This is the production path a 2026 deployment exercises: negotiation probes
//! `server/discover` with the full 2026 request-metadata headers
//! (`MCP-Protocol-Version: 2026-07-28` + `Mcp-Method`), the server's
//! §Server Validation enforces them, and every subsequent operation carries
//! `Mcp-Method`/`Mcp-Name` plus the per-request `_meta`.
#![cfg(feature = "client-bilingual")]

use serde_json::json;
use turul_mcp_client::config::ClientConfig;
use turul_mcp_client::transport::http::HttpTransport;
use turul_mcp_client::{McpClient, McpVersion};
use turul_mcp_derive::McpTool;
use turul_mcp_server::prelude::*;

#[derive(McpTool, Clone, Default)]
#[tool(name = "echo", description = "Echo back the provided message", output = String)]
struct EchoTool {
    #[param(description = "Message to echo back")]
    message: String,
}

impl EchoTool {
    async fn execute(
        &self,
        _session: Option<turul_mcp_server::SessionContext>,
    ) -> McpResult<String> {
        Ok(format!("Echo: {}", self.message))
    }
}

async fn start_2026_server() -> String {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();

    let server = McpServer::builder()
        .name("e2e-2026-real")
        .version("0.4.0")
        .tool(EchoTool::default())
        .bind_address(format!("127.0.0.1:{port}").parse().unwrap())
        .build()
        .expect("build 2026 server");

    tokio::spawn(async move {
        server.run().await.ok();
    });

    let url = format!("http://127.0.0.1:{port}/mcp");
    // 405 on GET = the accept loop is live (2026 endpoint is POST-only).
    let probe = reqwest::Client::new();
    for _ in 0..50 {
        if probe.get(&url).send().await.is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    url
}

#[tokio::test]
async fn bilingual_client_negotiates_and_calls_tools_on_a_real_2026_server() {
    let url = start_2026_server().await;

    let transport = Box::new(HttpTransport::new(&url).unwrap());
    let client = McpClient::new(transport, ClientConfig::default());
    client
        .connect()
        .await
        .expect("negotiation against the real 2026 server must succeed");

    assert_eq!(
        client.negotiated_version().await,
        Some(McpVersion::V2026_07_28),
        "the real 2026 server must be detected via server/discover"
    );

    // tools/list — requires MCP-Protocol-Version + Mcp-Method on the wire.
    let tools = client.list_tools().await.expect("list_tools");
    assert!(
        tools.iter().any(|t| t.name == "echo"),
        "echo tool must be advertised"
    );

    // tools/call — additionally requires Mcp-Name matching params.name.
    let result = client
        .call_tool("echo", json!({ "message": "round-trip" }))
        .await
        .expect("call_tool");
    let text = serde_json::to_string(&result).unwrap_or_default();
    assert!(
        text.contains("Echo: round-trip"),
        "tool result must round-trip through the real server: {text}"
    );
}

/// Echoes only after the client answers an elicitation (MRTR round trip).
#[derive(McpTool, Clone, Default)]
#[tool(name = "gated_echo", description = "Echo, but ask first", output = String)]
struct GatedEchoTool {}

impl GatedEchoTool {
    async fn execute(
        &self,
        session: Option<turul_mcp_server::SessionContext>,
    ) -> McpResult<String> {
        let session = session.ok_or_else(|| McpError::tool_execution("session required"))?;
        if let Some(responses) = session.input_responses() {
            let answer = responses
                .get("q1")
                .and_then(|r| match r {
                    turul_mcp_protocol::input_required::InputResponse::Elicit(e) => e
                        .content
                        .as_ref()
                        .and_then(|c| c.get("answer"))
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    _ => None,
                })
                .ok_or_else(|| McpError::tool_execution("q1 elicit response missing"))?;
            return Ok(format!("answered: {answer}"));
        }
        let schema = turul_mcp_protocol::elicitation::ElicitationSchema::new().with_property(
            "answer".to_string(),
            turul_mcp_protocol::elicitation::PrimitiveSchemaDefinition::string(),
        );
        let mut requests = turul_mcp_protocol::input_required::InputRequests::new();
        requests.insert(
            "q1".to_string(),
            turul_mcp_protocol::input_required::InputRequest::Elicit(
                turul_mcp_protocol::elicitation::ElicitRequest::new_form(
                    "What is the answer?",
                    schema,
                ),
            ),
        );
        Err(McpError::InputRequired {
            input_requests: Some(requests),
            request_state: Some("st-9".to_string()),
        })
    }
}

#[tokio::test]
async fn mrtr_round_trip_through_the_bilingual_client() {
    // Server with the gated tool.
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let server = McpServer::builder()
        .name("e2e-2026-mrtr")
        .version("0.4.0")
        .tool(GatedEchoTool::default())
        .bind_address(format!("127.0.0.1:{port}").parse().unwrap())
        .build()
        .expect("build 2026 server");
    tokio::spawn(async move {
        server.run().await.ok();
    });
    let url = format!("http://127.0.0.1:{port}/mcp");
    let probe = reqwest::Client::new();
    for _ in 0..50 {
        if probe.get(&url).send().await.is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }

    // Client that DECLARES the elicitation capability (servers reject MRTR
    // elicit requests against clients that did not declare it).
    let mut config = ClientConfig::default();
    config.declared_capabilities.elicitation = true;
    let transport = Box::new(HttpTransport::new(&url).unwrap());
    let client = McpClient::new(transport, config);
    client.connect().await.expect("connect");

    // Leg 1: the call surfaces InputRequired with the elicit request.
    let outcome = client.call_tool("gated_echo", json!({})).await;
    let (input_requests, request_state) = match outcome {
        Err(turul_mcp_client::McpClientError::InputRequired {
            input_requests,
            request_state,
        }) => (input_requests, request_state),
        other => panic!("expected InputRequired, got: {other:?}"),
    };
    let requests = input_requests.expect("inputRequests present");
    assert_eq!(
        requests["q1"]["method"], "elicitation/create",
        "the elicit request must surface to the application"
    );
    assert_eq!(request_state.as_deref(), Some("st-9"));

    // Leg 2: retry the original call with the gathered response + echoed state.
    let result = client
        .call_tool_with_input_responses(
            "gated_echo",
            json!({}),
            json!({ "q1": { "action": "accept", "content": { "answer": "42" } } }),
            request_state,
        )
        .await
        .expect("MRTR retry must complete");
    let text = serde_json::to_string(&result).unwrap_or_default();
    assert!(
        text.contains("answered: 42"),
        "the retry must complete the original call: {text}"
    );
}

/// Manual tool with an `x-mcp-header`-annotated `region` parameter (SEP-2243).
struct ExecuteSqlTool {
    input_schema: turul_mcp_protocol::ToolSchema,
}

impl ExecuteSqlTool {
    fn new() -> Self {
        let mut properties = std::collections::HashMap::new();
        properties.insert(
            "region".to_string(),
            json!({ "type": "string", "x-mcp-header": "Region" }),
        );
        properties.insert("query".to_string(), json!({ "type": "string" }));
        Self {
            input_schema: turul_mcp_protocol::ToolSchema::object()
                .with_properties(properties)
                .with_required(vec!["region".to_string(), "query".to_string()]),
        }
    }
}

impl turul_mcp_server::prelude::HasBaseMetadata for ExecuteSqlTool {
    fn name(&self) -> &str {
        "execute_sql"
    }
}
impl turul_mcp_server::prelude::HasDescription for ExecuteSqlTool {
    fn description(&self) -> Option<&str> {
        Some("Execute SQL in a region")
    }
}
impl turul_mcp_server::prelude::HasInputSchema for ExecuteSqlTool {
    fn input_schema(&self) -> &turul_mcp_protocol::ToolSchema {
        &self.input_schema
    }
}
impl turul_mcp_server::prelude::HasOutputSchema for ExecuteSqlTool {}
impl turul_mcp_server::prelude::HasAnnotations for ExecuteSqlTool {}
impl turul_mcp_server::prelude::HasToolMeta for ExecuteSqlTool {}
impl turul_mcp_server::prelude::HasIcons for ExecuteSqlTool {}

#[async_trait::async_trait]
impl turul_mcp_server::McpTool for ExecuteSqlTool {
    async fn call(
        &self,
        args: serde_json::Value,
        _session: Option<turul_mcp_server::SessionContext>,
    ) -> McpResult<turul_mcp_protocol::CallToolResult> {
        use turul_mcp_protocol::tools::ToolResult;
        let region = args
            .get("region")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        Ok(turul_mcp_protocol::CallToolResult::success(vec![
            ToolResult::text(format!("ran in {region}")),
        ]))
    }
}

#[tokio::test]
async fn client_mirrors_mcp_param_headers_from_x_mcp_header_annotations() {
    // The server VALIDATES Mcp-Param-* (an annotated argument without its
    // header is rejected -32001), so a green call below proves the client
    // actually emitted the mirror header.
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let server = McpServer::builder()
        .name("e2e-2026-param")
        .version("0.4.0")
        .tool(ExecuteSqlTool::new())
        .bind_address(format!("127.0.0.1:{port}").parse().unwrap())
        .build()
        .expect("build 2026 server");
    tokio::spawn(async move {
        server.run().await.ok();
    });
    let url = format!("http://127.0.0.1:{port}/mcp");
    let probe = reqwest::Client::new();
    for _ in 0..50 {
        if probe.get(&url).send().await.is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }

    let transport = Box::new(HttpTransport::new(&url).unwrap());
    let client = McpClient::new(transport, ClientConfig::default());
    client.connect().await.expect("connect");

    // tools/list populates the client's x-mcp-header binding cache.
    let tools = client.list_tools().await.expect("list_tools");
    assert!(tools.iter().any(|t| t.name == "execute_sql"));

    // Plain ASCII value.
    let result = client
        .call_tool(
            "execute_sql",
            json!({ "region": "us-west1", "query": "SELECT 1" }),
        )
        .await
        .expect("annotated tools/call must succeed — requires the Mcp-Param-Region mirror");
    let text = serde_json::to_string(&result).unwrap_or_default();
    assert!(text.contains("ran in us-west1"), "{text}");

    // Value requiring the Base64 sentinel (leading/trailing whitespace).
    let result = client
        .call_tool(
            "execute_sql",
            json!({ "region": " padded ", "query": "SELECT 1" }),
        )
        .await
        .expect("Base64-sentinel values must round-trip");
    let text = serde_json::to_string(&result).unwrap_or_default();
    assert!(text.contains("ran in  padded "), "{text}");
}

#[tokio::test]
async fn client_subscriptions_listen_receives_filtered_notifications() {
    // Real server with the broadcast-triggering tool; the client opens a
    // listen stream, a second request triggers server-wide broadcasts, and
    // only the opted-in type arrives — stamped with the subscription id.
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let server = McpServer::builder()
        .name("e2e-2026-listen")
        .version("0.4.0")
        .tool(EmitOnceTool::default())
        .with_resources()
        .with_prompts()
        .bind_address(format!("127.0.0.1:{port}").parse().unwrap())
        .build()
        .expect("build 2026 server");
    tokio::spawn(async move {
        server.run().await.ok();
    });
    let url = format!("http://127.0.0.1:{port}/mcp");
    let probe = reqwest::Client::new();
    for _ in 0..50 {
        if probe.get(&url).send().await.is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }

    let transport = Box::new(HttpTransport::new(&url).unwrap());
    let client = McpClient::new(transport, ClientConfig::default());
    client.connect().await.expect("connect");

    let mut stream = client
        .subscriptions_listen(json!({ "resourcesListChanged": true }))
        .await
        .expect("listen stream must open with an acknowledgement first");
    assert_eq!(
        stream.honored["resourcesListChanged"], true,
        "honored filter must echo the requested type"
    );
    let sub_id = stream.subscription_id.clone().expect("subscription id");

    // Trigger broadcasts (one requested type, one not).
    client
        .call_tool("emit_once", json!({}))
        .await
        .expect("emit tool");

    let event = tokio::time::timeout(std::time::Duration::from_secs(5), stream.next())
        .await
        .expect("a notification must arrive")
        .expect("stream open");
    assert_eq!(event["method"], "notifications/resources/list_changed");
    assert_eq!(
        event["params"]["_meta"]["io.modelcontextprotocol/subscriptionId"],
        serde_json::Value::String(sub_id),
        "stream notifications carry the subscription id: {event}"
    );
}

/// Broadcasts one requested and one unrequested notification type.
#[derive(McpTool, Clone, Default)]
#[tool(name = "emit_once", description = "Broadcast change notifications", output = String)]
struct EmitOnceTool {}

impl EmitOnceTool {
    async fn execute(
        &self,
        session: Option<turul_mcp_server::SessionContext>,
    ) -> McpResult<String> {
        use turul_mcp_server::turul_http_mcp_server::notification_bridge::SharedNotificationBroadcaster;
        let session = session.ok_or_else(|| McpError::tool_execution("session required"))?;
        let broadcaster = session
            .broadcaster
            .as_ref()
            .and_then(|a| a.downcast_ref::<SharedNotificationBroadcaster>())
            .ok_or_else(|| McpError::tool_execution("broadcaster required"))?
            .clone();
        let _ = broadcaster
            .broadcast_to_all_sessions(turul_rpc::JsonRpcNotification::new_no_params(
                "notifications/resources/list_changed".to_string(),
            ))
            .await;
        let _ = broadcaster
            .broadcast_to_all_sessions(turul_rpc::JsonRpcNotification::new_no_params(
                "notifications/prompts/list_changed".to_string(),
            ))
            .await;
        Ok("emitted".to_string())
    }
}

// ---- MRTR on resources/read and prompts/get through the client ----

/// Resource that demands an elicitation before serving content.
struct GatedResource;

impl turul_mcp_server::prelude::HasResourceMetadata for GatedResource {
    fn name(&self) -> &str {
        "gated"
    }
}
impl turul_mcp_server::prelude::HasResourceDescription for GatedResource {
    fn description(&self) -> Option<&str> {
        Some("Needs an answer first")
    }
}
impl turul_mcp_server::prelude::HasResourceUri for GatedResource {
    fn uri(&self) -> &str {
        "file:///gated.txt"
    }
}
impl turul_mcp_server::prelude::HasResourceMimeType for GatedResource {}
impl turul_mcp_server::prelude::HasResourceSize for GatedResource {}
impl turul_mcp_server::prelude::HasResourceAnnotations for GatedResource {}
impl turul_mcp_server::prelude::HasResourceMeta for GatedResource {}
impl turul_mcp_server::prelude::HasIcons for GatedResource {}

#[async_trait::async_trait]
impl turul_mcp_server::McpResource for GatedResource {
    async fn read(
        &self,
        _params: Option<serde_json::Value>,
        session: Option<&turul_mcp_server::SessionContext>,
    ) -> McpResult<Vec<turul_mcp_protocol::resources::ResourceContent>> {
        if let Some(responses) = session.and_then(|s| s.input_responses()) {
            let answer = responses
                .get("q1")
                .and_then(|r| match r {
                    turul_mcp_protocol::input_required::InputResponse::Elicit(e) => e
                        .content
                        .as_ref()
                        .and_then(|c| c.get("answer"))
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    _ => None,
                })
                .ok_or_else(|| McpError::tool_execution("q1 response missing"))?;
            return Ok(vec![turul_mcp_protocol::resources::ResourceContent::text(
                "file:///gated.txt",
                format!("content for {answer}"),
            )]);
        }
        let schema = turul_mcp_protocol::elicitation::ElicitationSchema::new().with_property(
            "answer".to_string(),
            turul_mcp_protocol::elicitation::PrimitiveSchemaDefinition::string(),
        );
        let mut requests = turul_mcp_protocol::input_required::InputRequests::new();
        requests.insert(
            "q1".to_string(),
            turul_mcp_protocol::input_required::InputRequest::Elicit(
                turul_mcp_protocol::elicitation::ElicitRequest::new_form("Which answer?", schema),
            ),
        );
        Err(McpError::InputRequired {
            input_requests: Some(requests),
            request_state: Some("res-state".to_string()),
        })
    }
}

/// Prompt that demands an elicitation; the retry's responses arrive in the
/// render args under the reserved io.modelcontextprotocol/* keys.
struct GatedPrompt;

impl turul_mcp_server::prelude::HasPromptMetadata for GatedPrompt {
    fn name(&self) -> &str {
        "gated_prompt"
    }
}
impl turul_mcp_server::prelude::HasPromptDescription for GatedPrompt {}
impl turul_mcp_server::prelude::HasPromptArguments for GatedPrompt {}
impl turul_mcp_server::prelude::HasPromptAnnotations for GatedPrompt {}
impl turul_mcp_server::prelude::HasPromptMeta for GatedPrompt {}
impl turul_mcp_server::prelude::HasIcons for GatedPrompt {}

#[async_trait::async_trait]
impl turul_mcp_server::McpPrompt for GatedPrompt {
    async fn render(
        &self,
        args: Option<std::collections::HashMap<String, serde_json::Value>>,
    ) -> McpResult<Vec<turul_mcp_protocol::prompts::PromptMessage>> {
        let responses = args
            .as_ref()
            .and_then(|a| a.get("io.modelcontextprotocol/inputResponses"));
        if let Some(responses) = responses {
            let answer = responses
                .pointer("/q1/content/answer")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            return Ok(vec![turul_mcp_protocol::prompts::PromptMessage::user_text(
                format!("prompt for {answer}"),
            )]);
        }
        let schema = turul_mcp_protocol::elicitation::ElicitationSchema::new().with_property(
            "answer".to_string(),
            turul_mcp_protocol::elicitation::PrimitiveSchemaDefinition::string(),
        );
        let mut requests = turul_mcp_protocol::input_required::InputRequests::new();
        requests.insert(
            "q1".to_string(),
            turul_mcp_protocol::input_required::InputRequest::Elicit(
                turul_mcp_protocol::elicitation::ElicitRequest::new_form("Which answer?", schema),
            ),
        );
        Err(McpError::InputRequired {
            input_requests: Some(requests),
            request_state: Some("prompt-state".to_string()),
        })
    }
}

async fn start_gated_server() -> String {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let server = McpServer::builder()
        .name("e2e-2026-mrtr-full")
        .version("0.4.0")
        .resource(GatedResource)
        .prompt(GatedPrompt)
        .bind_address(format!("127.0.0.1:{port}").parse().unwrap())
        .build()
        .expect("build 2026 server");
    tokio::spawn(async move {
        server.run().await.ok();
    });
    let url = format!("http://127.0.0.1:{port}/mcp");
    let probe = reqwest::Client::new();
    for _ in 0..50 {
        if probe.get(&url).send().await.is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    url
}

async fn connect_elicit_client(url: &str) -> McpClient {
    let mut config = ClientConfig::default();
    config.declared_capabilities.elicitation = true;
    let transport = Box::new(HttpTransport::new(url).unwrap());
    let client = McpClient::new(transport, config);
    client.connect().await.expect("connect");
    client
}

#[tokio::test]
async fn mrtr_round_trip_on_resources_read_through_the_client() {
    let url = start_gated_server().await;
    let client = connect_elicit_client(&url).await;

    // Leg 1: read_resource surfaces InputRequired (not a serde error).
    let outcome = client.read_resource("file:///gated.txt").await;
    let (input_requests, request_state) = match outcome {
        Err(turul_mcp_client::McpClientError::InputRequired {
            input_requests,
            request_state,
        }) => (input_requests, request_state),
        other => panic!("expected InputRequired, got: {other:?}"),
    };
    assert_eq!(
        input_requests.expect("inputRequests present")["q1"]["method"],
        "elicitation/create"
    );
    assert_eq!(request_state.as_deref(), Some("res-state"));

    // Leg 2: retry the original read with the gathered response + echoed state.
    let contents = client
        .read_resource_with_input_responses(
            "file:///gated.txt",
            json!({ "q1": { "action": "accept", "content": { "answer": "42" } } }),
            request_state,
        )
        .await
        .expect("MRTR retry must complete");
    let text = serde_json::to_string(&contents).unwrap_or_default();
    assert!(
        text.contains("content for 42"),
        "the retry must complete the original read: {text}"
    );
}

#[tokio::test]
async fn mrtr_round_trip_on_prompts_get_through_the_client() {
    let url = start_gated_server().await;
    let client = connect_elicit_client(&url).await;

    // Leg 1: get_prompt surfaces InputRequired (not a serde error).
    let outcome = client.get_prompt("gated_prompt", None).await;
    let (input_requests, request_state) = match outcome {
        Err(turul_mcp_client::McpClientError::InputRequired {
            input_requests,
            request_state,
        }) => (input_requests, request_state),
        other => panic!("expected InputRequired, got: {other:?}"),
    };
    assert_eq!(
        input_requests.expect("inputRequests present")["q1"]["method"],
        "elicitation/create"
    );
    assert_eq!(request_state.as_deref(), Some("prompt-state"));

    // Leg 2: retry with the gathered response + echoed state.
    let result = client
        .get_prompt_with_input_responses(
            "gated_prompt",
            None,
            json!({ "q1": { "action": "accept", "content": { "answer": "42" } } }),
            request_state,
        )
        .await
        .expect("MRTR retry must complete");
    let text = serde_json::to_string(&result).unwrap_or_default();
    assert!(
        text.contains("prompt for 42"),
        "the retry must complete the original get: {text}"
    );
}

/// Emits one request-scoped progress notification, then completes.
#[derive(McpTool, Clone, Default)]
#[tool(name = "progress_worker", description = "Works with progress", output = String)]
struct ProgressWorkerTool {}

impl ProgressWorkerTool {
    async fn execute(
        &self,
        session: Option<turul_mcp_server::SessionContext>,
    ) -> McpResult<String> {
        if let Some(session) = session {
            session.notify_request_progress(0.5, Some(1.0)).await;
        }
        Ok("worked".to_string())
    }
}

/// The client's per-request progress feed: `_meta.progressToken` goes out,
/// the progress notification comes back on the request stream before the
/// final result; the discover accessors expose the server's declared
/// capabilities and instructions.
#[tokio::test]
async fn progress_feed_and_discovered_server_accessors() {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let server = McpServer::builder()
        .name("e2e-2026-progress")
        .version("0.4.0")
        .instructions("Use progress_worker for long jobs.")
        .tool(ProgressWorkerTool::default())
        .bind_address(format!("127.0.0.1:{port}").parse().unwrap())
        .build()
        .expect("build 2026 server");
    tokio::spawn(async move {
        server.run().await.ok();
    });
    let url = format!("http://127.0.0.1:{port}/mcp");
    let probe = reqwest::Client::new();
    for _ in 0..50 {
        if probe.get(&url).send().await.is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }

    let transport = Box::new(HttpTransport::new(&url).unwrap());
    let client = McpClient::new(transport, ClientConfig::default());
    client.connect().await.expect("connect");

    // GAP-ARCH-1/DISC-1: the discover body is retained and exposed.
    let discovered = client
        .discovered_server()
        .await
        .expect("discover body retained on a 2026 connection");
    assert!(
        discovered
            .supported_versions
            .iter()
            .any(|v| v == "2026-07-28"),
        "{discovered:?}"
    );
    assert_eq!(
        client.server_instructions().await.as_deref(),
        Some("Use progress_worker for long jobs.")
    );
    let caps = client
        .server_capabilities()
        .await
        .expect("capabilities retained");
    assert!(
        caps.get("tools").is_some(),
        "a server with tools must declare the tools capability: {caps}"
    );

    // PAT/G4: progress events reach the application before the result.
    let progress_events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = progress_events.clone();
    let result = client
        .call_tool_with_progress(
            "progress_worker",
            json!({}),
            json!("tok-e2e"),
            move |params| sink.lock().unwrap().push(params),
        )
        .await
        .expect("tool call with progress");
    let text = serde_json::to_string(&result).unwrap_or_default();
    assert!(text.contains("worked"), "{text}");
    let events = progress_events.lock().unwrap();
    assert_eq!(events.len(), 1, "exactly one progress event: {events:?}");
    assert_eq!(events[0]["progressToken"], json!("tok-e2e"), "{events:?}");
    assert_eq!(events[0]["progress"], json!(0.5));
}
