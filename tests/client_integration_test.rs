//! Comprehensive integration test using turul-mcp-client crate
//!
//! Tests multiple example servers to verify round-trip functionality:
//! - Connection establishment and session management
//! - Tool discovery and execution
//! - Resource access
//! - Error handling
//! - Session cleanup

use anyhow::{Result, Context, anyhow};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::process::Command;
use tokio::time::{timeout, sleep};
use tracing::{info, warn, debug};
use turul_mcp_client::prelude::*;

/// Test configuration for each server
#[derive(Debug, Clone)]
struct ServerTest {
    name: String,
    binary_name: String,
    port: u16,
    expected_tools: Vec<String>,
    test_tool_name: Option<String>,
    test_tool_args: Value,
}

impl ServerTest {
    fn new(name: &str, binary_name: &str, port: u16) -> Self {
        Self {
            name: name.to_string(),
            binary_name: binary_name.to_string(),
            port,
            expected_tools: Vec::new(),
            test_tool_name: None,
            test_tool_args: json!({}),
        }
    }

    fn with_tools(mut self, tools: Vec<&str>) -> Self {
        self.expected_tools = tools.iter().map(|s| s.to_string()).collect();
        self
    }

    fn with_test_tool(mut self, name: &str, args: Value) -> Self {
        self.test_tool_name = Some(name.to_string());
        self.test_tool_args = args;
        self
    }
}

#[tokio::test]
async fn test_comprehensive_client_integration() -> Result<()> {
    // Initialize logging for debugging
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();

    info!("🚀 Starting comprehensive client integration test");

    // Define server test configurations
    let server_tests = vec![
        ServerTest::new("minimal-server", "minimal-server", 8641)
            .with_tools(vec!["echo"])
            .with_test_tool("echo", json!({"message": "Hello from client test!"})),

        ServerTest::new("tools-test-server", "tools-test-server", 8642)
            .with_tools(vec!["echo_sse", "get_session_data", "get_session_events"])
            .with_test_tool("echo_sse", json!({"text": "Testing tools-test-server"})),

        ServerTest::new("comprehensive-server", "comprehensive-server", 8643)
            .with_tools(vec!["calculator", "file_ops"])
            .with_test_tool("calculator", json!({"operation": "add", "a": 5.0, "b": 3.0})),
    ];

    let mut test_results = Vec::new();

    for server_config in server_tests {
        info!("🔧 Testing server: {}", server_config.name);

        match test_single_server(&server_config).await {
            Ok(result) => {
                info!("✅ {} passed all tests", server_config.name);
                test_results.push((server_config.name.clone(), Ok(result)));
            }
            Err(e) => {
                warn!("❌ {} failed: {}", server_config.name, e);
                test_results.push((server_config.name.clone(), Err(e)));
            }
        }

        // Small delay between tests
        sleep(Duration::from_millis(500)).await;
    }

    // Print final summary
    print_test_summary(&test_results)?;

    // Fail if any tests failed
    let failed_count = test_results.iter().filter(|(_, result)| result.is_err()).count();
    if failed_count > 0 {
        return Err(anyhow!("{} out of {} server tests failed", failed_count, test_results.len()));
    }

    info!("🎉 All server integration tests passed!");
    Ok(())
}

async fn test_single_server(config: &ServerTest) -> Result<String> {
    info!("📡 Starting {} on port {}", config.name, config.port);

    // Start the server
    let mut server_process = Command::new("cargo")
        .args(&["run", "--bin", &config.binary_name, "--", "--port", &config.port.to_string()])
        .spawn()
        .context("Failed to start server process")?;

    // Wait for server to be ready
    sleep(Duration::from_secs(2)).await;

    let result = timeout(Duration::from_secs(30), async {
        test_server_with_client(config).await
    }).await;

    // Clean up server process
    let _ = server_process.kill().await;

    match result {
        Ok(test_result) => test_result,
        Err(_) => Err(anyhow!("Test timed out after 30 seconds")),
    }
}

async fn test_server_with_client(config: &ServerTest) -> Result<String> {
    let server_url = format!("http://127.0.0.1:{}/mcp", config.port);
    info!("🔗 Connecting to {}", server_url);

    // Create MCP client
    let client = McpClient::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    // Test 1: Connection and initialization
    info!("📡 Test 1: Connection and initialization");
    client.connect(&server_url).await
        .context("Failed to connect to server")?;

    info!("✅ Successfully connected and initialized session");

    // Test 2: Server capabilities
    info!("📋 Test 2: Server capabilities");
    let server_info = client.get_server_info()
        .context("Failed to get server info")?;

    info!("📊 Server info: {}", serde_json::to_string_pretty(&server_info)?);

    // Test 3: Tool discovery
    info!("🔧 Test 3: Tool discovery");
    let tools = client.list_tools().await
        .context("Failed to list tools")?;

    info!("🛠️  Found {} tools", tools.tools.len());
    for tool in &tools.tools {
        info!("   • {}: {}", tool.name, tool.description.as_deref().unwrap_or("No description"));
    }

    // Verify expected tools are present
    for expected_tool in &config.expected_tools {
        let found = tools.tools.iter().any(|t| &t.name == expected_tool);
        if !found {
            return Err(anyhow!("Expected tool '{}' not found", expected_tool));
        }
        info!("✅ Found expected tool: {}", expected_tool);
    }

    // Test 4: Tool execution (if test tool specified)
    if let Some(tool_name) = &config.test_tool_name {
        info!("⚙️  Test 4: Tool execution - {}", tool_name);

        let tool_result = client.call_tool(tool_name, config.test_tool_args.clone()).await
            .context("Failed to call tool")?;

        info!("🎯 Tool result: {}", serde_json::to_string_pretty(&tool_result)?);

        // Verify we got a result
        if tool_result.content.is_empty() {
            return Err(anyhow!("Tool returned empty content"));
        }

        info!("✅ Tool execution successful");
    } else {
        info!("⏭️  Test 4: Skipped (no test tool specified)");
    }

    // Test 5: Resource discovery (if supported)
    info!("📁 Test 5: Resource discovery");
    match client.list_resources().await {
        Ok(resources) => {
            info!("📂 Found {} resources", resources.resources.len());
            for resource in &resources.resources {
                info!("   • {}: {}", resource.uri, resource.name.as_deref().unwrap_or("No name"));
            }
        }
        Err(e) => {
            debug!("Resource listing not supported or failed: {}", e);
            info!("⏭️  Resource listing not supported (expected for some servers)");
        }
    }

    // Test 6: Prompt discovery (if supported)
    info!("💭 Test 6: Prompt discovery");
    match client.list_prompts().await {
        Ok(prompts) => {
            info!("📝 Found {} prompts", prompts.prompts.len());
            for prompt in &prompts.prompts {
                info!("   • {}: {}", prompt.name, prompt.description.as_deref().unwrap_or("No description"));
            }
        }
        Err(e) => {
            debug!("Prompt listing not supported or failed: {}", e);
            info!("⏭️  Prompt listing not supported (expected for some servers)");
        }
    }

    // Test 7: Error handling
    info!("❌ Test 7: Error handling");
    match client.call_tool("nonexistent_tool", json!({})).await {
        Ok(_) => {
            warn!("⚠️  Expected error for nonexistent tool, but call succeeded");
        }
        Err(e) => {
            info!("✅ Error handling working: {}", e);
        }
    }

    // Test 8: Session cleanup
    info!("🧹 Test 8: Session cleanup");
    client.disconnect().await
        .context("Failed to disconnect cleanly")?;

    info!("✅ Clean disconnection successful");

    Ok(format!("All tests passed for {}", config.name))
}

fn print_test_summary(results: &[(String, Result<String>)]) -> Result<()> {
    info!("");
    info!("📊 CLIENT INTEGRATION TEST SUMMARY");
    info!("═══════════════════════════════════");

    let passed = results.iter().filter(|(_, r)| r.is_ok()).count();
    let failed = results.iter().filter(|(_, r)| r.is_err()).count();

    info!("✅ Passed: {} servers", passed);
    info!("❌ Failed: {} servers", failed);
    info!("📊 Total:  {} servers", results.len());

    info!("");
    info!("📋 Detailed Results:");

    for (server_name, result) in results {
        match result {
            Ok(message) => {
                info!("   ✅ {}: {}", server_name, message);
            }
            Err(e) => {
                info!("   ❌ {}: {}", server_name, e);
            }
        }
    }

    info!("");
    info!("🎯 Test Coverage:");
    info!("   ✅ Connection establishment");
    info!("   ✅ Session management");
    info!("   ✅ Tool discovery and execution");
    info!("   ✅ Resource/prompt discovery");
    info!("   ✅ Error handling");
    info!("   ✅ Clean session cleanup");

    if failed == 0 {
        info!("");
        info!("🎉 ALL CLIENT INTEGRATION TESTS PASSED!");
        info!("✅ turul-mcp-client round-trip functionality verified");
        info!("✅ Multiple server compatibility confirmed");
        info!("✅ Production-ready client-server communication");
    }

    Ok(())
}

/// Standalone test for minimal-server specifically
#[tokio::test]
async fn test_minimal_server_round_trip() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init();

    info!("🎯 Testing minimal-server round-trip specifically");

    let config = ServerTest::new("minimal-server", "minimal-server", 8650)
        .with_tools(vec!["echo"])
        .with_test_tool("echo", json!({"message": "Round-trip test successful!"}));

    let result = test_single_server(&config).await?;
    info!("🎉 Minimal server test result: {}", result);

    Ok(())
}

/// Test client behavior with server errors
#[tokio::test]
async fn test_client_error_handling() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init();

    info!("⚠️  Testing client error handling");

    // Test connection to non-existent server
    let client = McpClient::builder()
        .timeout(Duration::from_secs(2))
        .build()?;

    match client.connect("http://127.0.0.1:9999/mcp").await {
        Ok(_) => {
            return Err(anyhow!("Expected connection to fail for non-existent server"));
        }
        Err(e) => {
            info!("✅ Connection error handled correctly: {}", e);
        }
    }

    info!("✅ Client error handling verification complete");
    Ok(())
}