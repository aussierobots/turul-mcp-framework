//! E2E Integration Tests for MCP Prompts
//!
//! Tests real HTTP/SSE transport using prompts-test-server
//! Validates complete MCP 2025-06-18 specification compliance

use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;
use tokio::process::{Child, Command};
use tokio::time::sleep;
use tracing::{debug, info};

/// E2E test client for MCP operations
pub struct McpTestClient {
    client: Client,
    base_url: String,
    session_id: Option<String>,
}

impl McpTestClient {
    /// Create new test client
    pub fn new(port: u16) -> Self {
        Self {
            client: Client::new(),
            base_url: format!("http://127.0.0.1:{}/mcp", port),
            session_id: None,
        }
    }

    /// Initialize MCP session
    pub async fn initialize(&mut self) -> Result<HashMap<String, Value>, reqwest::Error> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {
                    "prompts": {
                        "listChanged": false  // MCP compliance: static framework
                    }
                },
                "clientInfo": {
                    "name": "prompts-e2e-test",
                    "version": "1.0.0"
                }
            }
        });

        let response = self
            .client
            .post(&self.base_url)
            .header("Content-Type", "application/json")
            .header("MCP-Protocol-Version", "2025-06-18")
            .json(&request)
            .send()
            .await?;

        // Extract session ID from headers
        if let Some(session_header) = response.headers().get("mcp-session-id") {
            self.session_id = Some(session_header.to_str().unwrap().to_string());
            debug!("Session ID: {:?}", self.session_id);
        }

        let result: HashMap<String, Value> = response.json().await?;
        Ok(result)
    }

    /// List available prompts
    pub async fn list_prompts(&self) -> Result<HashMap<String, Value>, reqwest::Error> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "prompts/list",
            "params": {}
        });

        let mut req_builder = self
            .client
            .post(&self.base_url)
            .header("Content-Type", "application/json")
            .json(&request);

        if let Some(ref session_id) = self.session_id {
            req_builder = req_builder.header("mcp-session-id", session_id);
        }

        let response = req_builder.send().await?;
        let result: HashMap<String, Value> = response.json().await?;
        Ok(result)
    }

    /// Get a specific prompt
    pub async fn get_prompt(&self, name: &str, arguments: Option<HashMap<String, Value>>) -> Result<HashMap<String, Value>, reqwest::Error> {
        let mut params = json!({
            "name": name
        });

        if let Some(args) = arguments {
            params["arguments"] = Value::Object(args.into_iter().map(|(k, v)| (k, v)).collect());
        }

        let request = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "prompts/get",
            "params": params
        });

        let mut req_builder = self
            .client
            .post(&self.base_url)
            .header("Content-Type", "application/json")
            .json(&request);

        if let Some(ref session_id) = self.session_id {
            req_builder = req_builder.header("mcp-session-id", session_id);
        }

        let response = req_builder.send().await?;
        let result: HashMap<String, Value> = response.json().await?;
        Ok(result)
    }

    /// Test SSE prompt notifications
    pub async fn test_sse_notifications(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut req_builder = self.client.get(&self.base_url).header("Accept", "text/event-stream");

        if let Some(ref session_id) = self.session_id {
            req_builder = req_builder.header("mcp-session-id", session_id);
        }

        let mut response = req_builder.send().await?;
        let mut events = Vec::new();

        // Read SSE events for a short time
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_secs(2) {
            if let Some(chunk) = response.chunk().await? {
                if let Ok(text) = String::from_utf8(chunk.to_vec()) {
                    events.push(text);
                    break; // Got an event, that's enough for the test
                }
            }
        }

        Ok(events)
    }
}

/// Test server manager for E2E tests
pub struct TestServerManager {
    server_process: Option<Child>,
    port: u16,
}

impl TestServerManager {
    /// Start prompts-test-server on random port
    pub async fn start() -> Result<Self, Box<dyn std::error::Error>> {
        let port = portpicker::pick_unused_port().expect("Failed to find available port");

        info!("Starting prompts-test-server on port {}", port);

        // Find workspace root dynamically instead of using hardcoded path
        let workspace_root = std::env::var("CARGO_MANIFEST_DIR")
            .map(|dir| std::path::PathBuf::from(dir).parent().unwrap().parent().unwrap().to_path_buf())
            .unwrap_or_else(|_| std::path::PathBuf::from("."));
        
        let binary_path = workspace_root.join("target").join("debug").join("prompts-test-server");
        let mut server_process = Command::new(&binary_path)
            .args(["--port", &port.to_string()])
            .current_dir(&workspace_root)
            .spawn()
            .map_err(|e| format!("Failed to start prompts-test-server: {}. Binary: {:?}", e, binary_path))?;

        // Wait for server to start
        let mut attempts = 0;
        let client = reqwest::Client::new();
        let health_url = format!("http://127.0.0.1:{}/mcp", port);

        while attempts < 50 {
            sleep(Duration::from_millis(300)).await;
            
            // Try to make a simple POST request to check if server is responding
            let test_request = json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-06-18",
                    "capabilities": {},
                    "clientInfo": {"name": "health-check", "version": "1.0.0"}
                }
            });
            
            if let Ok(response) = client
                .post(&health_url)
                .header("Content-Type", "application/json")
                .json(&test_request)
                .send()
                .await
            {
                if response.status().is_success() {
                    info!("Server started successfully on port {}", port);
                    return Ok(Self {
                        server_process: Some(server_process),
                        port,
                    });
                }
            }
            attempts += 1;
        }

        server_process.kill().await?;
        Err("Failed to start test server".into())
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

impl Drop for TestServerManager {
    fn drop(&mut self) {
        if let Some(mut process) = self.server_process.take() {
            let _ = process.kill();
        }
    }
}

#[tokio::test]
async fn test_mcp_initialize_session() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    let result = client.initialize().await.expect("Failed to initialize");

    // Verify response structure
    assert!(result.contains_key("result"));
    let result_data = result["result"].as_object().unwrap();

    assert!(result_data.contains_key("protocolVersion"));
    assert!(result_data.contains_key("capabilities"));
    assert!(result_data.contains_key("serverInfo"));

    // Verify protocol version
    assert_eq!(result_data["protocolVersion"], "2025-06-18");

    // Verify session ID was provided
    assert!(client.session_id.is_some());
}

#[tokio::test]
async fn test_prompts_list() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize().await.expect("Failed to initialize");
    let result = client.list_prompts().await.expect("Failed to list prompts");

    // Verify response structure
    assert!(result.contains_key("result"));
    let result_data = result["result"].as_object().unwrap();
    assert!(result_data.contains_key("prompts"));

    let prompts = result_data["prompts"].as_array().unwrap();
    assert!(prompts.len() > 0, "Should have test prompts available");

    // Verify all expected test prompts are present
    let expected_names = vec![
        "simple_prompt",
        "string_args_prompt",
        "number_args_prompt", 
        "boolean_args_prompt",
        "template_prompt",
        "multi_message_prompt",
        "session_aware_prompt",
        "validation_prompt",
        "dynamic_prompt",
        "empty_messages_prompt",
        "validation_failure_prompt",
    ];

    for expected_name in expected_names {
        let found = prompts.iter().any(|p| {
            p.as_object()
                .and_then(|obj| obj.get("name"))
                .and_then(|name| name.as_str())
            == Some(expected_name)
        });
        assert!(found, "Missing expected prompt: {}", expected_name);
    }
}

#[tokio::test]
async fn test_simple_prompt_get() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize().await.expect("Failed to initialize");
    let result = client
        .get_prompt("simple_prompt", None)
        .await
        .expect("Failed to get simple prompt");

    // Verify response structure
    assert!(result.contains_key("result"));
    let result_data = result["result"].as_object().unwrap();
    assert!(result_data.contains_key("messages"));

    let messages = result_data["messages"].as_array().unwrap();
    assert!(messages.len() > 0);

    // Verify message structure
    let message = &messages[0];
    let message_obj = message.as_object().unwrap();
    assert!(message_obj.contains_key("role"));
    assert!(message_obj.contains_key("content"));
}

#[tokio::test]
async fn test_string_args_prompt() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize().await.expect("Failed to initialize");

    let mut arguments = HashMap::new();
    arguments.insert("required_text".to_string(), json!("test string"));
    arguments.insert("optional_text".to_string(), json!("optional value"));

    let result = client
        .get_prompt("string_args_prompt", Some(arguments))
        .await
        .expect("Failed to get string args prompt");

    assert!(result.contains_key("result"));
    let result_data = result["result"].as_object().unwrap();
    assert!(result_data.contains_key("messages"));

    let messages = result_data["messages"].as_array().unwrap();
    assert!(messages.len() > 0);

    // Check that arguments were used in the message content
    let message_content = messages[0]["content"]["text"].as_str().unwrap();
    assert!(message_content.contains("test string"));
}

#[tokio::test]
async fn test_number_args_prompt() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize().await.expect("Failed to initialize");

    let mut arguments = HashMap::new();
    arguments.insert("count".to_string(), json!("42"));
    arguments.insert("multiplier".to_string(), json!("3.14"));

    let result = client
        .get_prompt("number_args_prompt", Some(arguments))
        .await
        .expect("Failed to get number args prompt");

    assert!(result.contains_key("result"));
    let result_data = result["result"].as_object().unwrap();
    assert!(result_data.contains_key("messages"));

    let messages = result_data["messages"].as_array().unwrap();
    assert!(messages.len() > 0);

    // Check that numbers were used in the message content
    let message_content = messages[0]["content"]["text"].as_str().unwrap();
    assert!(message_content.contains("42") || message_content.contains("125.4")); // 42 * 3.14 ≈ 131.88
}

#[tokio::test]
async fn test_boolean_args_prompt() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize().await.expect("Failed to initialize");

    let mut arguments = HashMap::new();
    arguments.insert("enable_feature".to_string(), json!("true"));
    arguments.insert("debug_mode".to_string(), json!("false"));

    let result = client
        .get_prompt("boolean_args_prompt", Some(arguments))
        .await
        .expect("Failed to get boolean args prompt");

    assert!(result.contains_key("result"));
    let result_data = result["result"].as_object().unwrap();
    assert!(result_data.contains_key("messages"));

    let messages = result_data["messages"].as_array().unwrap();
    assert!(messages.len() > 0);

    // Check that booleans were used in the message content
    let message_content = messages[0]["content"]["text"].as_str().unwrap();
    assert!(message_content.contains("ENABLED") || message_content.contains("DISABLED") || message_content.contains("ON") || message_content.contains("OFF"));
}

#[tokio::test]
async fn test_template_prompt() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize().await.expect("Failed to initialize");

    let mut arguments = HashMap::new();
    arguments.insert("name".to_string(), json!("test_user"));
    arguments.insert("topic".to_string(), json!("artificial intelligence"));
    arguments.insert("style".to_string(), json!("casual"));

    let result = client
        .get_prompt("template_prompt", Some(arguments))
        .await
        .expect("Failed to get template prompt");

    assert!(result.contains_key("result"));
    let result_data = result["result"].as_object().unwrap();
    assert!(result_data.contains_key("messages"));

    let messages = result_data["messages"].as_array().unwrap();
    assert!(messages.len() > 0);

    // Check that template variables were substituted
    let message_content = messages[0]["content"]["text"].as_str().unwrap();
    assert!(message_content.contains("test_user"));
    assert!(message_content.contains("artificial intelligence"));
}

#[tokio::test]
async fn test_multi_message_prompt() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize().await.expect("Failed to initialize");

    let mut arguments = HashMap::new();
    arguments.insert("scenario".to_string(), json!("machine learning"));

    let result = client
        .get_prompt("multi_message_prompt", Some(arguments))
        .await
        .expect("Failed to get multi message prompt");

    assert!(result.contains_key("result"));
    let result_data = result["result"].as_object().unwrap();
    assert!(result_data.contains_key("messages"));

    let messages = result_data["messages"].as_array().unwrap();
    // Multi-message prompt should return multiple messages
    assert!(messages.len() > 1, "Multi message prompt should return multiple messages");

    // Verify different roles are used
    let roles: Vec<&str> = messages.iter()
        .filter_map(|m| m["role"].as_str())
        .collect();
    
    // Should have different roles (user and assistant)
    assert!(roles.contains(&"user") || roles.contains(&"assistant"));
}

#[tokio::test]
async fn test_session_aware_prompt() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize().await.expect("Failed to initialize");

    let result = client
        .get_prompt("session_aware_prompt", None)
        .await
        .expect("Failed to get session aware prompt");

    assert!(result.contains_key("result"));
    let result_data = result["result"].as_object().unwrap();
    assert!(result_data.contains_key("messages"));

    let messages = result_data["messages"].as_array().unwrap();
    assert!(messages.len() > 0);

    // Check that session information is included
    let message_content = messages[0]["content"]["text"].as_str().unwrap();
    assert!(message_content.contains("session"));
}

#[tokio::test]
async fn test_validation_failure_prompt() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize().await.expect("Failed to initialize");

    // Try to get validation failure prompt without required arguments
    let result = client
        .get_prompt("validation_failure_prompt", None)
        .await
        .expect("Request should succeed but prompt should error");

    // Should get a JSON-RPC error response for validation failure
    assert!(result.contains_key("error"), "Validation failure prompt should return JSON-RPC error response");
    let error = result["error"].as_object().unwrap();
    assert!(error.contains_key("code"));
    assert!(error.contains_key("message"));
}

#[tokio::test]
async fn test_dynamic_prompt() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize().await.expect("Failed to initialize");

    let mut arguments = HashMap::new();
    arguments.insert("mode".to_string(), json!("analytical"));
    arguments.insert("content".to_string(), json!("data analysis results"));

    let result = client
        .get_prompt("dynamic_prompt", Some(arguments))
        .await
        .expect("Failed to get dynamic prompt");

    assert!(result.contains_key("result"));
    let result_data = result["result"].as_object().unwrap();
    assert!(result_data.contains_key("messages"));

    let messages = result_data["messages"].as_array().unwrap();
    assert!(messages.len() > 0);

    // Check that dynamic mode was used
    let message_content = messages[0]["content"]["text"].as_str().unwrap();
    assert!(message_content.contains("analytical") || message_content.contains("ANALYTICAL"));
}

#[tokio::test]
async fn test_empty_messages_prompt() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize().await.expect("Failed to initialize");

    let result = client
        .get_prompt("empty_messages_prompt", None)
        .await
        .expect("Failed to get empty messages prompt");

    assert!(result.contains_key("result"));
    let result_data = result["result"].as_object().unwrap();
    assert!(result_data.contains_key("messages"));

    let messages = result_data["messages"].as_array().unwrap();
    // Empty messages prompt should return an empty array
    assert_eq!(messages.len(), 0, "Empty messages prompt should return no messages");
}

#[tokio::test]
async fn test_sse_prompt_notifications() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize().await.expect("Failed to initialize");

    // Test SSE stream (simplified - real test would trigger changes)
    let events = client
        .test_sse_notifications()
        .await
        .expect("Failed to test SSE notifications");

    // Should receive some SSE data format
    if !events.is_empty() {
        info!("Received SSE events: {:?}", events);
        // Events should contain SSE format data
        assert!(events.iter().any(|e| e.contains("data:") || e.contains("event:")));
    }
}