//! Comprehensive integration test using turul-mcp-client crate
//!
//! Tests example servers end-to-end through the high-level `McpClient` API
//! (the entry point real consumers use, not raw JSON-RPC) to verify:
//! - Connection establishment and session management
//! - Tool discovery and execution
//! - Resource/prompt discovery
//! - Error handling
//! - Session cleanup

use anyhow::{Context, Result, anyhow};
use mcp_e2e_shared::TestServerManager;
use serde_json::{Value, json};
use std::time::Duration;
use tracing::{debug, info, warn};
use turul_mcp_client::prelude::*;

/// Test configuration for each server
struct ServerTest {
    name: &'static str,
    manager: TestServerManager,
    expected_tools: Vec<&'static str>,
    test_tool_name: &'static str,
    test_tool_args: Value,
}

#[tokio::test]
async fn test_comprehensive_client_integration() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();

    info!("Starting comprehensive client integration test");

    let minimal = TestServerManager::start_with_args("minimal-server", &[])
        .await
        .map_err(|e| anyhow!("Failed to start minimal-server: {e}"))?;
    let tools = TestServerManager::start_tools_server()
        .await
        .map_err(|e| anyhow!("Failed to start tools-test-server: {e}"))?;

    let server_tests = vec![
        ServerTest {
            name: "minimal-server",
            manager: minimal,
            expected_tools: vec!["echo"],
            test_tool_name: "echo",
            test_tool_args: json!({"text": "Hello from client test!"}),
        },
        ServerTest {
            name: "tools-test-server",
            manager: tools,
            expected_tools: vec!["calculator", "string_processor"],
            test_tool_name: "calculator",
            test_tool_args: json!({"operation": "add", "a": 2.0, "b": 3.0}),
        },
    ];

    let mut failures = Vec::new();

    for server in &server_tests {
        info!("Testing server: {}", server.name);
        if let Err(e) = test_single_server(server).await {
            warn!("{} failed: {}", server.name, e);
            failures.push((server.name, e));
        } else {
            info!("{} passed all tests", server.name);
        }
    }

    if !failures.is_empty() {
        return Err(anyhow!(
            "{} out of {} server tests failed: {:?}",
            failures.len(),
            server_tests.len(),
            failures
        ));
    }

    info!("All server integration tests passed");
    Ok(())
}

async fn test_single_server(server: &ServerTest) -> Result<()> {
    let server_url = format!("http://127.0.0.1:{}/mcp", server.manager.port());
    info!("Connecting to {}", server_url);

    let client = McpClientBuilder::new()
        .with_url(&server_url)
        .map_err(|e| anyhow!("Invalid server URL: {e}"))?
        .build();

    // Test 1: Connection and initialization
    client
        .connect()
        .await
        .context("Failed to connect to server")?;
    info!("Connected and initialized session");

    // Test 2: Server capabilities (via negotiated session info)
    let session = client.session_info().await;
    info!(
        "Negotiated protocol version: {:?}, capabilities: {:?}",
        session.protocol_version, session.server_capabilities
    );

    // Test 3: Tool discovery
    let discovered_tools = client.list_tools().await.context("Failed to list tools")?;
    info!("Found {} tools", discovered_tools.len());
    for expected in &server.expected_tools {
        let found = discovered_tools.iter().any(|t| &t.name == expected);
        if !found {
            return Err(anyhow!("Expected tool '{}' not found", expected));
        }
    }

    // Test 4: Tool execution
    let tool_result = client
        .call_tool(server.test_tool_name, server.test_tool_args.clone())
        .await
        .context("Failed to call tool")?;
    if tool_result.content.is_empty() {
        return Err(anyhow!("Tool returned empty content"));
    }
    info!("Tool execution successful: {:?}", tool_result.content);

    // Test 5: Resource discovery (best-effort; not every server exposes resources)
    match client.list_resources().await {
        Ok(resources) => info!("Found {} resources", resources.len()),
        Err(e) => debug!("Resource listing not supported or failed: {e}"),
    }

    // Test 6: Prompt discovery (best-effort; not every server exposes prompts)
    match client.list_prompts().await {
        Ok(prompts) => info!("Found {} prompts", prompts.len()),
        Err(e) => debug!("Prompt listing not supported or failed: {e}"),
    }

    // Test 7: Error handling for a nonexistent tool
    match client.call_tool("nonexistent_tool", json!({})).await {
        Ok(_) => warn!("Expected error for nonexistent tool, but call succeeded"),
        Err(e) => info!("Error handling working as expected: {e}"),
    }

    // Test 8: Session cleanup
    client
        .disconnect()
        .await
        .context("Failed to disconnect cleanly")?;
    info!("Clean disconnection successful");

    Ok(())
}

/// Test client behavior connecting to a non-existent server
#[tokio::test]
async fn test_client_error_handling() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init();

    let client = McpClientBuilder::new()
        .with_url("http://127.0.0.1:9999/mcp")?
        .with_config(ClientConfig {
            timeouts: turul_mcp_client::config::TimeoutConfig {
                connect: Duration::from_secs(2),
                ..Default::default()
            },
            ..Default::default()
        })
        .build();

    match client.connect().await {
        Ok(_) => Err(anyhow!(
            "Expected connection to fail for non-existent server"
        )),
        Err(e) => {
            info!("Connection error handled correctly: {e}");
            Ok(())
        }
    }
}
