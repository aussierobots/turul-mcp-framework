//! Wire-level acceptance for MRTR (SEP-2322) production on the 2026 path.
//!
//! Multi Round-Trip Requests: a server needing client input during
//! `tools/call` does NOT send a server-initiated request — it returns an
//! `InputRequiredResult` (`resultType: "input_required"`, `inputRequests`,
//! opaque `requestState`), and the client retries the ORIGINAL request with
//! `inputResponses` + the echoed `requestState` under a new JSON-RPC id.
//! Servers MUST NOT request capabilities the client did not declare in the
//! request's `_meta` `clientCapabilities` (→ `-32003`, HTTP 400).
//!
//! Built only under the 2026 feature; compiles to nothing under 2025-11-25.
#![cfg(feature = "protocol-2026-07-28")]

use std::collections::HashMap;

use turul_mcp_derive::McpTool;
use turul_mcp_protocol::elicitation::{ElicitRequest, ElicitationSchema};
use turul_mcp_protocol::input_required::{InputRequest, InputRequests};
use turul_mcp_server::prelude::*;

/// Echoes only after the client answers an elicitation: first call returns
/// input-required; the retry (carrying `inputResponses`) completes.
#[derive(McpTool, Clone, Default)]
#[tool(name = "gated_echo", description = "Echo, but ask first", output = String)]
struct GatedEchoTool {}

impl GatedEchoTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
        let session = session.ok_or_else(|| McpError::tool_execution("session required"))?;

        // Retry leg: the tools/call handler surfaces the retry's inputResponses.
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
            assert_eq!(
                session.mrtr_request_state().as_deref(),
                Some("state-1"),
                "requestState must be echoed verbatim"
            );
            return Ok(format!("answered: {answer}"));
        }

        // First leg: demand input via MRTR.
        let schema = ElicitationSchema::new().with_property(
            "answer".to_string(),
            turul_mcp_protocol::elicitation::PrimitiveSchemaDefinition::string(),
        );
        let mut requests = InputRequests::new();
        requests.insert(
            "q1".to_string(),
            InputRequest::Elicit(ElicitRequest::new_form("What is the answer?", schema)),
        );
        Err(McpError::InputRequired {
            input_requests: Some(requests),
            request_state: Some("state-1".to_string()),
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
        .name("mrtr-2026-test")
        .version("0.4.0")
        .tool(GatedEchoTool::default())
        .bind_address(format!("127.0.0.1:{port}").parse().unwrap())
        .build()
        .expect("build 2026 server");

    tokio::spawn(async move {
        server.run().await.ok();
    });

    let url = format!("http://127.0.0.1:{port}/mcp");
    let client = reqwest::Client::new();
    for _ in 0..50 {
        if client.get(&url).send().await.is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    url
}

/// Per-request `_meta`; `capabilities` lets tests model what the client declared.
fn meta_with_capabilities(capabilities: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": { "name": "test-client", "version": "1.0.0" },
        "io.modelcontextprotocol/clientCapabilities": capabilities
    })
}

async fn call_gated_echo(
    url: &str,
    capabilities: serde_json::Value,
    extra_params: serde_json::Value,
) -> (reqwest::StatusCode, serde_json::Value) {
    let mut params = serde_json::json!({
        "name": "gated_echo",
        "arguments": {},
        "_meta": meta_with_capabilities(capabilities)
    });
    if let (Some(p), Some(e)) = (params.as_object_mut(), extra_params.as_object()) {
        for (k, v) in e {
            p.insert(k.clone(), v.clone());
        }
    }
    let client = reqwest::Client::new();
    let resp = client
        .post(url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "gated_echo")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": params
        }))
        .send()
        .await
        .expect("tools/call POST");
    let status = resp.status();
    let body: serde_json::Value = resp.json().await.unwrap_or_default();
    (status, body)
}

#[tokio::test]
async fn mrtr_round_trip_completes_the_original_call() {
    let url = start_server().await;
    let elicitation_caps = serde_json::json!({ "elicitation": {} });

    // Leg 1: no inputResponses → input_required with the elicit request.
    let (status, body) =
        call_gated_echo(&url, elicitation_caps.clone(), serde_json::json!({})).await;
    assert_eq!(
        status, 200,
        "input_required is a RESULT, not an error: {body}"
    );
    let result = &body["result"];
    assert_eq!(
        result["resultType"], "input_required",
        "first leg must return InputRequiredResult: {body}"
    );
    assert_eq!(
        result["inputRequests"]["q1"]["method"], "elicitation/create",
        "the elicit request must ride inputRequests: {body}"
    );
    assert_eq!(result["requestState"], "state-1");

    // Leg 2: retry the ORIGINAL request with inputResponses + echoed state.
    let (status, body) = call_gated_echo(
        &url,
        elicitation_caps,
        serde_json::json!({
            "inputResponses": {
                "q1": { "action": "accept", "content": { "answer": "42" } }
            },
            "requestState": "state-1"
        }),
    )
    .await;
    assert_eq!(status, 200);
    let result = &body["result"];
    assert_ne!(
        result["resultType"], "input_required",
        "the retry must complete, not loop: {body}"
    );
    let text = result["content"][0]["text"].as_str().unwrap_or_default();
    assert!(
        text.contains("answered: 42"),
        "the tool must see the elicit response on the retry: {body}"
    );
}

#[tokio::test]
async fn undeclared_capability_is_rejected_with_32003() {
    let url = start_server().await;

    // The client declares NO capabilities — the server must not emit an
    // elicitation input request, per the MRTR rules: -32003 + HTTP 400.
    let (status, body) = call_gated_echo(&url, serde_json::json!({}), serde_json::json!({})).await;
    assert_eq!(
        status, 400,
        "MissingRequiredClientCapabilityError must be HTTP 400: {body}"
    );
    assert_eq!(
        body["error"]["code"], -32003,
        "must be -32003 MissingRequiredClientCapability: {body}"
    );
}

/// Keep the unused import warnings honest: HashMap is used by the tool body.
#[allow(dead_code)]
fn _t(_: HashMap<String, String>) {}

// ---- Sub-capability gating (elicitation modes; sampling.tools) ----

/// Demands a URL-mode elicitation — needs `elicitation.url` declared.
#[derive(McpTool, Clone, Default)]
#[tool(name = "url_gated", description = "Needs a URL elicitation", output = String)]
struct UrlGatedTool {}

impl UrlGatedTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
        if let Some(session) = &session
            && session.input_responses().is_some()
        {
            return Ok("done".to_string());
        }
        let mut requests = InputRequests::new();
        requests.insert(
            "auth".to_string(),
            InputRequest::Elicit(ElicitRequest::new_url(
                "Authorize via browser",
                "el-1",
                "https://example.test/authorize",
            )),
        );
        Err(McpError::InputRequired {
            input_requests: Some(requests),
            request_state: None,
        })
    }
}

/// Demands tool-enabled sampling — needs `sampling.tools` declared.
#[derive(McpTool, Clone, Default)]
#[tool(name = "sampler", description = "Needs tool-enabled sampling", output = String)]
struct SamplingGatedTool {}

impl SamplingGatedTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
        if let Some(session) = &session
            && session.input_responses().is_some()
        {
            return Ok("done".to_string());
        }
        #[allow(deprecated)]
        let request = {
            use turul_mcp_protocol::sampling::{
                CreateMessageRequest, CreateMessageRequestParams, SamplingMessage,
            };
            let mut params =
                CreateMessageRequestParams::new(vec![SamplingMessage::user_text("hi")], 64);
            params.tools = Some(vec![turul_mcp_protocol::tools::Tool::new(
                "lookup",
                turul_mcp_protocol::ToolSchema::object(),
            )]);
            CreateMessageRequest::new(vec![], 64).with_params(params)
        };
        let mut requests = InputRequests::new();
        requests.insert("s1".to_string(), InputRequest::CreateMessage(request));
        Err(McpError::InputRequired {
            input_requests: Some(requests),
            request_state: None,
        })
    }
}

async fn start_subcap_server() -> String {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let server = McpServer::builder()
        .name("mrtr-2026-subcap")
        .version("0.4.0")
        .tool(UrlGatedTool::default())
        .tool(SamplingGatedTool::default())
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

async fn call_tool_with_caps(
    url: &str,
    tool: &str,
    capabilities: serde_json::Value,
) -> (reqwest::StatusCode, serde_json::Value) {
    let client = reqwest::Client::new();
    let resp = client
        .post(url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", tool)
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {
                "name": tool,
                "arguments": {},
                "_meta": meta_with_capabilities(capabilities)
            }
        }))
        .send()
        .await
        .expect("tools/call POST");
    let status = resp.status();
    let body: serde_json::Value = resp.json().await.unwrap_or_default();
    (status, body)
}

/// Elicitation §Capabilities: "Servers MUST NOT send elicitation requests
/// with modes that are not supported by the client" and "an empty
/// capabilities object is equivalent to declaring support for form mode
/// only" — a URL-mode request against `elicitation: {}` must be -32003.
#[tokio::test]
async fn url_mode_elicitation_requires_the_url_subcapability() {
    let url = start_subcap_server().await;

    // Form-only declaration (empty object): URL mode rejected.
    let (status, body) =
        call_tool_with_caps(&url, "url_gated", serde_json::json!({ "elicitation": {} })).await;
    assert_eq!(status, 400, "url mode vs form-only client: {body}");
    assert_eq!(body["error"]["code"], -32003, "{body}");

    // URL declared: passes the gate, input_required comes back.
    let (status, body) = call_tool_with_caps(
        &url,
        "url_gated",
        serde_json::json!({ "elicitation": { "url": {} } }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["result"]["resultType"], "input_required", "{body}");
}

/// Elicitation §Capabilities: form mode rides both the empty object and an
/// explicit form declaration.
#[tokio::test]
async fn form_mode_elicitation_passes_with_empty_capability_object() {
    let url = start_server().await;
    let (status, body) = call_gated_echo(
        &url,
        serde_json::json!({ "elicitation": {} }),
        serde_json::json!({}),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["result"]["resultType"], "input_required", "{body}");
}

/// Sampling §Tools in Sampling: "Servers MUST NOT send tool-enabled sampling
/// requests to Clients that have not declared support for tool use via the
/// sampling.tools capability."
#[tokio::test]
async fn tool_enabled_sampling_requires_the_tools_subcapability() {
    let url = start_subcap_server().await;

    // Bare sampling declaration: tool-enabled request rejected.
    let (status, body) =
        call_tool_with_caps(&url, "sampler", serde_json::json!({ "sampling": {} })).await;
    assert_eq!(
        status, 400,
        "tool-enabled sampling vs bare sampling: {body}"
    );
    assert_eq!(body["error"]["code"], -32003, "{body}");

    // sampling.tools declared: passes the gate.
    let (status, body) = call_tool_with_caps(
        &url,
        "sampler",
        serde_json::json!({ "sampling": { "tools": {} } }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["result"]["resultType"], "input_required", "{body}");
}

// ---- MRTR on resources/read and prompts/get ----

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
        session: Option<&SessionContext>,
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
        let schema = ElicitationSchema::new().with_property(
            "answer".to_string(),
            turul_mcp_protocol::elicitation::PrimitiveSchemaDefinition::string(),
        );
        let mut requests = InputRequests::new();
        requests.insert(
            "q1".to_string(),
            InputRequest::Elicit(ElicitRequest::new_form("Which answer?", schema)),
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
        let schema = ElicitationSchema::new().with_property(
            "answer".to_string(),
            turul_mcp_protocol::elicitation::PrimitiveSchemaDefinition::string(),
        );
        let mut requests = InputRequests::new();
        requests.insert(
            "q1".to_string(),
            InputRequest::Elicit(ElicitRequest::new_form("Which answer?", schema)),
        );
        Err(McpError::InputRequired {
            input_requests: Some(requests),
            request_state: Some("prompt-state".to_string()),
        })
    }
}

async fn start_full_server() -> String {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let server = McpServer::builder()
        .name("mrtr-2026-full")
        .version("0.4.0")
        .tool(GatedEchoTool::default())
        .resource(GatedResource)
        .prompt(GatedPrompt)
        .bind_address(format!("127.0.0.1:{port}").parse().unwrap())
        .build()
        .expect("build 2026 server");
    tokio::spawn(async move {
        server.run().await.ok();
    });
    let url = format!("http://127.0.0.1:{port}/mcp");
    let client = reqwest::Client::new();
    for _ in 0..50 {
        if client.get(&url).send().await.is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    url
}

async fn post_mrtr(
    url: &str,
    rpc_method: &str,
    name_header: &str,
    params: serde_json::Value,
) -> (reqwest::StatusCode, serde_json::Value) {
    let client = reqwest::Client::new();
    let resp = client
        .post(url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", rpc_method)
        .header("Mcp-Name", name_header)
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": rpc_method, "params": params
        }))
        .send()
        .await
        .expect("POST");
    let status = resp.status();
    let body: serde_json::Value = resp.json().await.unwrap_or_default();
    (status, body)
}

#[tokio::test]
async fn mrtr_round_trip_on_resources_read() {
    let url = start_full_server().await;
    let caps = serde_json::json!({ "elicitation": {} });

    // Leg 1: input_required with the elicit request + state.
    let (status, body) = post_mrtr(
        &url,
        "resources/read",
        "file:///gated.txt",
        serde_json::json!({
            "uri": "file:///gated.txt",
            "_meta": meta_with_capabilities(caps.clone())
        }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["result"]["resultType"], "input_required", "{body}");
    assert_eq!(body["result"]["requestState"], "res-state", "{body}");

    // Leg 2: retry with inputResponses → content.
    let (status, body) = post_mrtr(
        &url,
        "resources/read",
        "file:///gated.txt",
        serde_json::json!({
            "uri": "file:///gated.txt",
            "inputResponses": { "q1": { "action": "accept", "content": { "answer": "42" } } },
            "requestState": "res-state",
            "_meta": meta_with_capabilities(caps)
        }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let text = body["result"]["contents"][0]["text"]
        .as_str()
        .unwrap_or_default();
    assert!(text.contains("content for 42"), "{body}");
}

#[tokio::test]
async fn mrtr_round_trip_on_prompts_get() {
    let url = start_full_server().await;
    let caps = serde_json::json!({ "elicitation": {} });

    let (status, body) = post_mrtr(
        &url,
        "prompts/get",
        "gated_prompt",
        serde_json::json!({
            "name": "gated_prompt",
            "_meta": meta_with_capabilities(caps.clone())
        }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["result"]["resultType"], "input_required", "{body}");
    assert_eq!(body["result"]["requestState"], "prompt-state", "{body}");

    let (status, body) = post_mrtr(
        &url,
        "prompts/get",
        "gated_prompt",
        serde_json::json!({
            "name": "gated_prompt",
            "inputResponses": { "q1": { "action": "accept", "content": { "answer": "42" } } },
            "requestState": "prompt-state",
            "_meta": meta_with_capabilities(caps)
        }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let text = body["result"]["messages"][0]["content"]["text"]
        .as_str()
        .unwrap_or_default();
    assert!(text.contains("prompt for 42"), "{body}");
}

#[tokio::test]
async fn resources_read_capability_gate_applies() {
    let url = start_full_server().await;
    // No elicitation capability declared → -32003 at HTTP 400.
    let (status, body) = post_mrtr(
        &url,
        "resources/read",
        "file:///gated.txt",
        serde_json::json!({
            "uri": "file:///gated.txt",
            "_meta": meta_with_capabilities(serde_json::json!({}))
        }),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    assert_eq!(body["error"]["code"], -32003, "{body}");
}

// ---- MRTR negative paths (PAT/G7) ----

/// Errs InputRequired with NEITHER inputRequests nor requestState — the
/// schema invariant ("at least one of") makes this a server error, not an
/// input_required result.
#[derive(McpTool, Clone, Default)]
#[tool(name = "broken_mrtr", description = "Violates the MRTR invariant", output = String)]
struct BrokenMrtrTool {}

impl BrokenMrtrTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
        Err(McpError::InputRequired {
            input_requests: None,
            request_state: None,
        })
    }
}

/// "Servers MUST include at least one of inputRequests or requestState in
/// every InputRequiredResult" — the neither-field case surfaces as a JSON-RPC
/// error, never as a resultType:"input_required" result.
#[tokio::test]
async fn input_required_with_neither_field_is_a_server_error() {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let server = McpServer::builder()
        .name("mrtr-neg-2026")
        .version("0.4.0")
        .tool(BrokenMrtrTool::default())
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

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "tools/call")
        .header("Mcp-Name", "broken_mrtr")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": { "name": "broken_mrtr", "arguments": {},
                        "_meta": meta_with_capabilities(serde_json::json!({"elicitation": {}})) }
        }))
        .send()
        .await
        .expect("POST");
    let body: serde_json::Value = resp.json().await.expect("json");
    assert!(
        body.get("error").is_some(),
        "neither-field InputRequired must be an error: {body}"
    );
    assert!(
        body["result"].get("resultType").is_none(),
        "must NOT surface as input_required: {body}"
    );
}

/// "Servers MUST NOT send InputRequiredResult responses on any other client
/// requests" — an InputRequired escaping a non-MRTR method (completion/complete)
/// surfaces as an internal error, never as an input_required result.
struct EscapingCompleter;

impl turul_mcp_server::prelude::HasCompletionMetadata for EscapingCompleter {
    fn method(&self) -> &str {
        "completion/complete"
    }
    fn reference(&self) -> &turul_mcp_protocol::completion::CompletionReference {
        use std::sync::OnceLock;
        use turul_mcp_protocol::completion::{CompletionReference, PromptReference};
        static R: OnceLock<CompletionReference> = OnceLock::new();
        R.get_or_init(|| CompletionReference::Prompt(PromptReference::new("escape")))
    }
}
impl turul_mcp_server::prelude::HasCompletionContext for EscapingCompleter {
    fn argument(&self) -> &turul_mcp_protocol::completion::CompleteArgument {
        use std::sync::OnceLock;
        use turul_mcp_protocol::completion::CompleteArgument;
        static A: OnceLock<CompleteArgument> = OnceLock::new();
        A.get_or_init(|| CompleteArgument::new("arg", ""))
    }
}
impl turul_mcp_server::prelude::HasCompletionHandling for EscapingCompleter {}

#[async_trait::async_trait]
impl turul_mcp_server::McpCompletion for EscapingCompleter {
    async fn complete(
        &self,
        _request: turul_mcp_protocol::completion::CompleteRequest,
    ) -> McpResult<turul_mcp_protocol::completion::CompleteResult> {
        let mut requests = InputRequests::new();
        requests.insert(
            "q1".to_string(),
            InputRequest::Elicit(ElicitRequest::new_form(
                "escape attempt",
                ElicitationSchema::new(),
            )),
        );
        Err(McpError::InputRequired {
            input_requests: Some(requests),
            request_state: None,
        })
    }
}

#[tokio::test]
async fn input_required_escaping_a_non_mrtr_method_is_an_error() {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let server = McpServer::builder()
        .name("mrtr-escape-2026")
        .version("0.4.0")
        .tool(GatedEchoTool::default())
        .completion_provider(EscapingCompleter)
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

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Accept", "application/json")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "completion/complete")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 2, "method": "completion/complete",
            "params": {
                "ref": { "type": "ref/prompt", "name": "escape" },
                "argument": { "name": "arg", "value": "x" },
                "_meta": meta_with_capabilities(serde_json::json!({"elicitation": {}}))
            }
        }))
        .send()
        .await
        .expect("POST");
    let body: serde_json::Value = resp.json().await.expect("json");
    assert!(
        body.get("error").is_some(),
        "InputRequired on a non-MRTR method must be an error: {body}"
    );
    assert_ne!(
        body["result"]["resultType"], "input_required",
        "completion/complete may NEVER return input_required: {body}"
    );
}

/// Demands a roots listing via MRTR — needs the roots capability.
#[derive(McpTool, Clone, Default)]
#[tool(name = "roots_gated", description = "Needs roots", output = String)]
struct RootsGatedTool {}

impl RootsGatedTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
        if let Some(session) = &session
            && session.input_responses().is_some()
        {
            return Ok("done".to_string());
        }
        #[allow(deprecated)]
        let request = turul_mcp_protocol::roots::ListRootsRequest::new();
        let mut requests = InputRequests::new();
        requests.insert("r1".to_string(), InputRequest::ListRoots(request));
        Err(McpError::InputRequired {
            input_requests: Some(requests),
            request_state: None,
        })
    }
}

/// Plain (tool-free) sampling request — needs only the sampling capability.
#[derive(McpTool, Clone, Default)]
#[tool(name = "plain_sampler", description = "Plain sampling", output = String)]
struct PlainSamplerTool {}

impl PlainSamplerTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
        if let Some(session) = &session
            && session.input_responses().is_some()
        {
            return Ok("done".to_string());
        }
        #[allow(deprecated)]
        let request = {
            use turul_mcp_protocol::sampling::{CreateMessageRequest, SamplingMessage};
            CreateMessageRequest::new(vec![SamplingMessage::user_text("hi")], 32)
        };
        let mut requests = InputRequests::new();
        requests.insert("s1".to_string(), InputRequest::CreateMessage(request));
        Err(McpError::InputRequired {
            input_requests: Some(requests),
            request_state: None,
        })
    }
}

async fn start_caparm_server() -> String {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let server = McpServer::builder()
        .name("mrtr-2026-caparms")
        .version("0.4.0")
        .tool(RootsGatedTool::default())
        .tool(PlainSamplerTool::default())
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

/// The roots and sampling arms of the -32003 capability gate, both
/// directions: undeclared → 400 -32003; declared → input_required.
#[tokio::test]
async fn roots_and_sampling_capability_arms_are_gated() {
    let url = start_caparm_server().await;

    for (tool, cap) in [("roots_gated", "roots"), ("plain_sampler", "sampling")] {
        // Undeclared capability → -32003 + HTTP 400.
        let (status, body) = call_tool_with_caps(&url, tool, serde_json::json!({})).await;
        assert_eq!(status, 400, "{tool} vs no caps: {body}");
        assert_eq!(body["error"]["code"], -32003, "{tool}: {body}");

        // Declared → the input request rides input_required.
        let (status, body) = call_tool_with_caps(&url, tool, serde_json::json!({ cap: {} })).await;
        assert_eq!(status, 200, "{tool} with {cap}: {body}");
        assert_eq!(body["result"]["resultType"], "input_required", "{body}");
    }
}
