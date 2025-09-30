//! Shared E2E Test Utilities
//!
//! Common utilities for MCP E2E testing across resources and prompts

use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use tokio::process::{Child, Command};
use tokio::time::sleep;
use tracing::{debug, info};

/// Find the workspace root directory by looking for Cargo.toml with [workspace]
fn find_workspace_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    // Start from current directory and walk up
    let mut current = std::env::current_dir()?;

    loop {
        let cargo_toml = current.join("Cargo.toml");
        if cargo_toml.exists() {
            // Read Cargo.toml to check if it's a workspace
            let cargo_content = std::fs::read_to_string(&cargo_toml)?;
            if cargo_content.contains("[workspace]") {
                return Ok(current);
            }
        }

        // Move up one directory
        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => break,
        }
    }

    // Fallback: use CARGO_MANIFEST_DIR if available (works in tests)
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let manifest_path = PathBuf::from(manifest_dir);
        // Walk up to find workspace root
        let mut current = manifest_path;
        while let Some(parent) = current.parent() {
            let cargo_toml = parent.join("Cargo.toml");
            if cargo_toml.exists() {
                let cargo_content = std::fs::read_to_string(&cargo_toml).unwrap_or_default();
                if cargo_content.contains("[workspace]") {
                    return Ok(parent.to_path_buf());
                }
            }
            current = parent.to_path_buf();
        }
    }

    Err("Could not find workspace root with [workspace] in Cargo.toml".into())
}

/// Generic E2E test client for MCP operations
#[derive(Clone)]
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

    /// Initialize MCP session with custom capabilities
    pub async fn initialize_with_capabilities(
        &mut self,
        capabilities: Value,
    ) -> Result<HashMap<String, Value>, reqwest::Error> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": capabilities,
                "clientInfo": {
                    "name": "mcp-e2e-test",
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

    /// Initialize MCP session with standard capabilities
    pub async fn initialize(&mut self) -> Result<HashMap<String, Value>, reqwest::Error> {
        self.initialize_with_capabilities(json!({})).await
    }

    /// Send notifications/initialized to complete session handshake (required for strict lifecycle mode)
    pub async fn send_initialized_notification(&self) -> Result<HashMap<String, Value>, reqwest::Error> {
        let notification = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });

        self.send_notification(notification).await
    }

    /// Make a generic MCP request
    pub async fn make_request(
        &self,
        method: &str,
        params: Value,
        id: u64,
    ) -> Result<HashMap<String, Value>, reqwest::Error> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
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

    /// List available resources
    pub async fn list_resources(&self) -> Result<HashMap<String, Value>, reqwest::Error> {
        self.make_request("resources/list", json!({}), 2).await
    }

    /// Read a specific resource
    pub async fn read_resource(&self, uri: &str) -> Result<HashMap<String, Value>, reqwest::Error> {
        self.make_request("resources/read", json!({"uri": uri}), 3)
            .await
    }

    /// Subscribe to resource changes
    pub async fn subscribe_resource(
        &self,
        uri: &str,
    ) -> Result<HashMap<String, Value>, reqwest::Error> {
        self.make_request("resources/subscribe", json!({"uri": uri}), 4)
            .await
    }

    /// List available prompts
    pub async fn list_prompts(&self) -> Result<HashMap<String, Value>, reqwest::Error> {
        self.make_request("prompts/list", json!({}), 5).await
    }

    /// Get a specific prompt
    pub async fn get_prompt(
        &self,
        name: &str,
        arguments: Option<HashMap<String, Value>>,
    ) -> Result<HashMap<String, Value>, reqwest::Error> {
        let mut params = json!({"name": name});

        if let Some(args) = arguments {
            params["arguments"] = Value::Object(args.into_iter().collect());
        }

        self.make_request("prompts/get", params, 6).await
    }

    /// Test SSE notifications
    pub async fn test_sse_notifications(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut req_builder = self
            .client
            .get(&self.base_url)
            .header("Accept", "text/event-stream");

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

    /// List available tools
    pub async fn list_tools(&self) -> Result<HashMap<String, Value>, reqwest::Error> {
        self.make_request("tools/list", json!({}), 7).await
    }

    /// Call a specific tool
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: Value,
    ) -> Result<HashMap<String, Value>, reqwest::Error> {
        self.make_request(
            "tools/call",
            json!({"name": name, "arguments": arguments}),
            8,
        )
        .await
    }

    /// Send a notification (no response expected)
    pub async fn send_notification(
        &self,
        notification: Value,
    ) -> Result<HashMap<String, Value>, reqwest::Error> {
        let mut req_builder = self
            .client
            .post(&self.base_url)
            .header("Content-Type", "application/json")
            .json(&notification);

        if let Some(ref session_id) = self.session_id {
            req_builder = req_builder.header("mcp-session-id", session_id);
        }

        let response = req_builder.send().await?;

        // For notifications, we might get an empty response or a simple acknowledgment
        // Try to parse as JSON, but handle empty responses gracefully
        let text = response.text().await?;
        if text.trim().is_empty() {
            Ok(HashMap::new())
        } else {
            match serde_json::from_str(&text) {
                Ok(json_val) => Ok(json_val),
                Err(_) => {
                    // If not valid JSON, return the text in a wrapper
                    let mut result = HashMap::new();
                    result.insert("response".to_string(), Value::String(text));
                    Ok(result)
                }
            }
        }
    }

    /// Connect to SSE stream for real-time events
    pub async fn connect_sse(&self) -> Result<reqwest::Response, reqwest::Error> {
        let mut req_builder = self
            .client
            .get(&self.base_url)
            .header("Accept", "text/event-stream");

        if let Some(ref session_id) = self.session_id {
            req_builder = req_builder.header("mcp-session-id", session_id);
        }

        req_builder.send().await
    }

    /// Get session ID
    pub fn session_id(&self) -> Option<&String> {
        self.session_id.as_ref()
    }
}

/// Test server manager for E2E tests
pub struct TestServerManager {
    server_process: Option<Child>,
    port: u16,
    _server_name: String,
}

impl TestServerManager {
    /// Find an available port using OS ephemeral port allocation only
    /// This eliminates the port thrashing that caused 60s+ delays in sandbox environments
    fn find_available_port() -> Option<u16> {
        // Use OS ephemeral port allocation (bind to 0) - this is the most reliable approach
        // The OS will assign an available port from the ephemeral range
        for attempt in 1..=5 {
            if let Ok(listener) = std::net::TcpListener::bind("127.0.0.1:0") {
                if let Ok(addr) = listener.local_addr() {
                    let port = addr.port();
                    drop(listener); // Release the port immediately
                    debug!("OS assigned ephemeral port {} (attempt {})", port, attempt);
                    return Some(port);
                }
            }
            // Small delay between attempts to avoid tight loops
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        // No fallback to portpicker - it fails in sandbox environments with "Operation not permitted"
        // If OS ephemeral port allocation fails, this indicates network binding is restricted
        debug!("Failed to allocate port via OS ephemeral binding - likely sandboxed environment");
        None
    }

    /// Start a test server by name on random port with robust port allocation
    pub async fn start(server_name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let port = Self::find_available_port()
            .ok_or("Failed to find available port after exhaustive search - network binding may be restricted in this environment")?;

        info!("Starting {} on port {}", server_name, port);

        // Find workspace root dynamically instead of using hardcoded path
        let workspace_root =
            find_workspace_root().map_err(|e| format!("Failed to find workspace root: {}", e))?;

        let binary_path = workspace_root
            .join("target")
            .join("debug")
            .join(server_name);
        let mut server_process = Command::new(&binary_path)
            .args(["--port", &port.to_string()])
            .current_dir(&workspace_root)
            .spawn()
            .map_err(|e| {
                format!(
                    "Failed to start server {}: {}. Binary path: {:?}, Workspace: {:?}",
                    server_name, e, binary_path, workspace_root
                )
            })?;

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
                    info!(
                        "Server {} started successfully on port {}",
                        server_name, port
                    );
                    return Ok(Self {
                        server_process: Some(server_process),
                        port,
                        _server_name: server_name.to_string(),
                    });
                }
            }
            attempts += 1;
        }

        server_process.kill().await?;
        Err(format!("Failed to start test server {}", server_name).into())
    }

    /// Start resource test server
    pub async fn start_resource_server() -> Result<Self, Box<dyn std::error::Error>> {
        Self::start("resource-test-server").await
    }

    /// Start prompts test server
    pub async fn start_prompts_server() -> Result<Self, Box<dyn std::error::Error>> {
        Self::start("prompts-test-server").await
    }

    /// Start tools test server
    pub async fn start_tools_server() -> Result<Self, Box<dyn std::error::Error>> {
        Self::start("tools-test-server").await
    }

    /// Start sampling test server
    pub async fn start_sampling_server() -> Result<Self, Box<dyn std::error::Error>> {
        Self::start("sampling-server").await
    }

    /// Start roots test server
    pub async fn start_roots_server() -> Result<Self, Box<dyn std::error::Error>> {
        Self::start("roots-server").await
    }

    /// Start elicitation test server
    pub async fn start_elicitation_server() -> Result<Self, Box<dyn std::error::Error>> {
        Self::start("elicitation-server").await
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

impl Drop for TestServerManager {
    fn drop(&mut self) {
        if let Some(mut process) = self.server_process.take() {
            std::mem::drop(process.kill());
        }
    }
}

/// Common test fixtures and helpers
pub struct TestFixtures;

impl TestFixtures {
    /// Standard resource capabilities for initialization (MCP 2025-06-18 compliant)
    pub fn resource_capabilities() -> Value {
        json!({
            "resources": {
                "subscribe": true,
                "listChanged": false  // Static framework: no dynamic changes = listChanged false
            }
        })
    }

    /// Standard prompts capabilities for initialization (MCP 2025-06-18 compliant)
    pub fn prompts_capabilities() -> Value {
        json!({
            "prompts": {
                "listChanged": false  // Static framework: no dynamic changes = listChanged false
            }
        })
    }

    /// Standard tools capabilities for initialization (MCP 2025-06-18 compliant)
    pub fn tools_capabilities() -> Value {
        json!({
            "tools": {
                "listChanged": false  // Static framework: no dynamic changes = listChanged false
            }
        })
    }

    /// Verify standard MCP initialization response structure
    pub fn verify_initialization_response(result: &HashMap<String, Value>) {
        assert!(result.contains_key("result"));
        let result_data = result["result"].as_object().unwrap();

        assert!(result_data.contains_key("protocolVersion"));
        assert!(result_data.contains_key("capabilities"));
        assert!(result_data.contains_key("serverInfo"));

        // Verify protocol version
        assert_eq!(result_data["protocolVersion"], "2025-06-18");
    }

    /// Verify MCP error response structure
    pub fn verify_error_response(result: &HashMap<String, Value>) {
        assert!(result.contains_key("error"));
        let error = result["error"].as_object().unwrap();
        assert!(error.contains_key("code"));
        assert!(error.contains_key("message"));
    }

    /// Create test arguments for prompts
    pub fn create_string_args() -> HashMap<String, Value> {
        let mut args = HashMap::new();
        args.insert("required_text".to_string(), json!("test string"));
        args.insert("optional_text".to_string(), json!("optional value"));
        args
    }

    /// Create test number arguments for prompts - MCP spec requires string arguments
    pub fn create_number_args() -> HashMap<String, Value> {
        let mut args = HashMap::new();
        args.insert("count".to_string(), json!("42"));  // number_args_prompt expects "count" as string
        args.insert("multiplier".to_string(), json!("3.14"));  // optional multiplier as string
        args
    }

    /// Create test boolean arguments for prompts - MCP spec requires string arguments
    pub fn create_boolean_args() -> HashMap<String, Value> {
        let mut args = HashMap::new();
        args.insert("enable_feature".to_string(), json!("true"));  // boolean_args_prompt expects "enable_feature" as string
        args.insert("debug_mode".to_string(), json!("false"));  // optional debug_mode as string
        args
    }

    /// Create test template arguments for prompts
    pub fn create_template_args() -> HashMap<String, Value> {
        let mut args = HashMap::new();
        args.insert("name".to_string(), json!("Alice"));  // template_prompt expects "name"
        args.insert("topic".to_string(), json!("machine learning"));  // template_prompt expects "topic"
        args.insert("style".to_string(), json!("casual"));  // optional style
        args
    }

    /// Verify prompt response structure
    pub fn verify_prompt_response(result: &HashMap<String, Value>) {
        assert!(result.contains_key("result"));
        let result_data = result["result"].as_object().unwrap();
        assert!(result_data.contains_key("messages"));

        let messages = result_data["messages"].as_array().unwrap();
        if !messages.is_empty() {
            // Verify message structure for non-empty responses
            let message = &messages[0];
            let message_obj = message.as_object().unwrap();
            assert!(message_obj.contains_key("role"));
            assert!(message_obj.contains_key("content"));
        }
    }

    /// Extract and parse the first tool result object from structured content
    pub fn extract_tool_result_object(result: &HashMap<String, Value>) -> Option<Value> {
        if let Some(structured) = Self::extract_tool_structured_content(result) {
            // Get the first key-value pair from structured content (the actual tool result)
            if let Some(obj) = structured.as_object() {
                if let Some((_, value)) = obj.iter().next() {
                    return Some(value.clone());
                }
            }
        }
        None
    }

    /// Extract text content from tool result response
    pub fn extract_tool_content_text(result: &HashMap<String, Value>) -> String {
        result["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or("{}")
            .to_string()
    }

    /// Extract structured content from tool result for direct access
    pub fn extract_tool_structured_content(result: &HashMap<String, Value>) -> Option<&Value> {
        result.get("result")?.get("structuredContent")
    }

    /// Verify resource list response structure
    pub fn verify_resource_list_response(result: &HashMap<String, Value>) {
        assert!(result.contains_key("result"));
        let result_data = result["result"].as_object().unwrap();
        assert!(result_data.contains_key("resources"));

        let resources = result_data["resources"].as_array().unwrap();
        for resource in resources {
            let resource_obj = resource.as_object().unwrap();
            assert!(resource_obj.contains_key("uri"));
            assert!(resource_obj.contains_key("name"));
            // Optional fields: description, mimeType
        }
    }

    /// Verify prompts list response structure
    pub fn verify_prompts_list_response(result: &HashMap<String, Value>) {
        assert!(result.contains_key("result"));
        let result_data = result["result"].as_object().unwrap();
        assert!(result_data.contains_key("prompts"));

        let prompts = result_data["prompts"].as_array().unwrap();
        for prompt in prompts {
            let prompt_obj = prompt.as_object().unwrap();
            assert!(prompt_obj.contains_key("name"));
            // Optional fields: description, arguments
        }
    }

    /// Verify resource content response structure
    pub fn verify_resource_content_response(result: &HashMap<String, Value>) {
        assert!(result.contains_key("result"));
        let result_data = result["result"].as_object().unwrap();
        assert!(result_data.contains_key("contents"));

        let contents = result_data["contents"].as_array().unwrap();
        for content in contents {
            let content_obj = content.as_object().unwrap();
            assert!(content_obj.contains_key("uri"));
            // Must have either "text" or "blob"
            assert!(content_obj.contains_key("text") || content_obj.contains_key("blob"));
        }
    }

    /// Extract tools list from tools/list response
    pub fn extract_tools_list(result: &HashMap<String, Value>) -> Option<Vec<Value>> {
        result
            .get("result")?
            .as_object()?
            .get("tools")?
            .as_array()
            .cloned()
    }
}

/// Session context testing utilities
pub struct SessionTestUtils;

impl SessionTestUtils {
    /// Verify session context is maintained across requests
    pub async fn verify_session_consistency(
        client: &McpTestClient,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let _session_id = client.session_id().expect("Session ID should be available");

        // Make multiple requests and verify session consistency
        let result1 = client.list_resources().await?;
        let result2 = client.list_prompts().await?;

        // Both requests should succeed (no session errors)
        assert!(result1.contains_key("result") || result1.contains_key("error"));
        assert!(result2.contains_key("result") || result2.contains_key("error"));

        // If there are errors, they shouldn't be session-related
        if let Some(error) = result1.get("error") {
            let error_message = error["message"].as_str().unwrap_or("");
            assert!(!error_message.to_lowercase().contains("session"));
        }

        if let Some(error) = result2.get("error") {
            let error_message = error["message"].as_str().unwrap_or("");
            assert!(!error_message.to_lowercase().contains("session"));
        }

        Ok(())
    }

    /// Test session-aware resource behavior
    pub async fn test_session_aware_resource(
        client: &McpTestClient,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let result = client.read_resource("file:///session/info.json").await?;

        if result.contains_key("result") {
            let result_data = result["result"].as_object().unwrap();
            let contents = result_data["contents"].as_array().unwrap();

            if !contents.is_empty() {
                let content = &contents[0];
                let text = content.as_object().unwrap()["text"].as_str().unwrap();
                assert!(
                    text.contains("session"),
                    "Session-aware resource should include session info"
                );
            }
        }

        Ok(())
    }

    /// Test session-aware prompt behavior
    pub async fn test_session_aware_prompt(
        client: &McpTestClient,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let result = client.get_prompt("session_aware_prompt", None).await?;

        if result.contains_key("result") {
            let result_data = result["result"].as_object().unwrap();
            let messages = result_data["messages"].as_array().unwrap();

            if !messages.is_empty() {
                let message_content = messages[0]["content"]["text"].as_str().unwrap();
                assert!(
                    message_content.contains("session"),
                    "Session-aware prompt should include session info"
                );
            }
        }

        Ok(())
    }
}
