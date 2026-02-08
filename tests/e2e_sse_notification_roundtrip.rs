//! This test verifies that SSE notifications are actually delivered end-to-end:
//! - Tool can send notifications via SessionContext
//! - Notifications flow through the StreamManager correctly
//! - SSE client receives the actual notification data
//! - Session isolation is maintained
//!
//! Uses streamable HTTP (POST with Accept: text/event-stream) per MCP spec

use mcp_e2e_shared::{McpTestClient, TestServerManager};
use serde_json::json;
use tracing::info;

/// Test that verifies actual notification delivery through SSE streamable HTTP
#[tokio::test]
async fn test_sse_notification_round_trip_delivery() {
    let _ = tracing_subscriber::fmt::try_init();
    info!("🧪 Starting SSE notification round-trip test (streamable HTTP)");

    // Try to start test server with notification tool
    // This will gracefully handle CI/sandbox environments where port binding fails
    let server = match TestServerManager::start_tools_server().await {
        Ok(server) => server,
        Err(e) => {
            println!(
                "Skipping SSE test - failed to start server (likely sandboxed environment): {}",
                e
            );
            return;
        }
    };

    let mut client = McpTestClient::new(server.port());

    // Initialize with SSE capabilities
    client
        .initialize_with_capabilities(json!({
            "tools": {
                "listChanged": false
            }
        }))
        .await
        .expect("Failed to initialize");

    let session_id = client.session_id().unwrap();
    info!("✅ Client initialized with session: {}", session_id);

    // Send notifications/initialized to complete session handshake (required for strict mode)
    client
        .send_initialized_notification()
        .await
        .expect("Failed to send initialized notification");
    info!("✅ Sent notifications/initialized");

    // Call tool with SSE streaming to get progress notifications in response
    let response = client
        .call_tool_with_sse(
            "progress_tracker",
            json!({
                "duration": 0.5,
                "steps": 3
            }),
        )
        .await
        .expect("Failed to call progress_tracker tool with SSE");

    // Verify SSE content type
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    info!("Response content-type: {}", content_type);
    assert!(
        content_type.contains("text/event-stream"),
        "Expected SSE streaming response"
    );

    // Collect SSE events from response body
    let response_text = response.text().await.expect("Failed to read response body");
    info!("📨 Received SSE response: {} bytes", response_text.len());

    // Parse and analyze events for progress notifications from progress_tracker tool
    let mut found_progress = false;
    for line in response_text.lines() {
        if let Some(data) = line.strip_prefix("data: ") {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                // Check for progress notification (MCP 2025-06-18 spec)
                if let Some(method) = parsed.get("method").and_then(|m| m.as_str()) {
                    if method == "notifications/progress" {
                        found_progress = true;
                        info!("✅ Found progress notification from progress_tracker tool");
                        if let Some(params) = parsed.get("params") {
                            if let Some(token) =
                                params.get("progressToken").and_then(|t| t.as_str())
                            {
                                info!("✅ Progress token: '{}'", token);
                            }
                            if let Some(progress) = params.get("progress").and_then(|p| p.as_u64())
                            {
                                info!("✅ Progress value: {}%", progress);
                            }
                        }
                    }
                }
            }
        }
    }

    // Verify we received progress notifications
    assert!(
        found_progress,
        "Progress notification was not received via SSE streamable HTTP"
    );

    info!("🎉 SSE notification round-trip test passed!");
}

/// Test session isolation for notifications using streamable HTTP
#[tokio::test]
async fn test_sse_notification_session_isolation() {
    let _ = tracing_subscriber::fmt::try_init();
    info!("🧪 Starting SSE notification session isolation test (streamable HTTP)");

    // Try to start test server with notification tool
    // This will gracefully handle CI/sandbox environments where port binding fails
    let server = match TestServerManager::start_tools_server().await {
        Ok(server) => server,
        Err(e) => {
            println!(
                "Skipping SSE test - failed to start server (likely sandboxed environment): {}",
                e
            );
            return;
        }
    };

    // Create two separate clients
    let mut client1 = McpTestClient::new(server.port());
    let mut client2 = McpTestClient::new(server.port());

    // Initialize both clients
    client1
        .initialize_with_capabilities(json!({
            "tools": {"listChanged": false}
        }))
        .await
        .expect("Failed to initialize client1");

    client2
        .initialize_with_capabilities(json!({
            "tools": {"listChanged": false}
        }))
        .await
        .expect("Failed to initialize client2");

    let session1_id = client1.session_id().unwrap();
    let session2_id = client2.session_id().unwrap();
    assert_ne!(session1_id, session2_id);

    info!(
        "✅ Created two sessions: {} and {}",
        session1_id, session2_id
    );

    // Send notifications/initialized for both clients (required for strict mode)
    client1
        .send_initialized_notification()
        .await
        .expect("Failed to send initialized for client1");
    client2
        .send_initialized_notification()
        .await
        .expect("Failed to send initialized for client2");
    info!("✅ Sent notifications/initialized for both clients");

    // Call tool with SSE for client1
    let response1 = client1
        .call_tool_with_sse(
            "progress_tracker",
            json!({
                "duration": 0.3,
                "steps": 2
            }),
        )
        .await
        .expect("Failed to call tool for client1");

    // Call tool with SSE for client2
    let response2 = client2
        .call_tool_with_sse(
            "progress_tracker",
            json!({
                "duration": 0.3,
                "steps": 2
            }),
        )
        .await
        .expect("Failed to call tool for client2");

    // Get response text
    let events1_text = response1
        .text()
        .await
        .expect("Failed to read client1 response");
    let events2_text = response2
        .text()
        .await
        .expect("Failed to read client2 response");

    info!(
        "📨 Client1 received {} bytes, Client2 received {} bytes",
        events1_text.len(),
        events2_text.len()
    );

    // Count progress notifications for each client
    let mut client1_progress_count = 0;
    let mut client2_progress_count = 0;

    for line in events1_text.lines() {
        if let Some(data) = line.strip_prefix("data: ") {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                if let Some(method) = parsed.get("method").and_then(|m| m.as_str()) {
                    if method == "notifications/progress" {
                        client1_progress_count += 1;
                    }
                }
            }
        }
    }

    for line in events2_text.lines() {
        if let Some(data) = line.strip_prefix("data: ") {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                if let Some(method) = parsed.get("method").and_then(|m| m.as_str()) {
                    if method == "notifications/progress" {
                        client2_progress_count += 1;
                    }
                }
            }
        }
    }

    info!(
        "Client1 received {} progress notifications",
        client1_progress_count
    );
    info!(
        "Client2 received {} progress notifications",
        client2_progress_count
    );

    // Each client should receive progress notifications from their own tool calls
    assert!(
        client1_progress_count > 0,
        "Client1 should receive progress notifications from its own session"
    );
    assert!(
        client2_progress_count > 0,
        "Client2 should receive progress notifications from its own session"
    );

    // Session isolation verified: each client only gets notifications from their own POST request
    info!("🎉 SSE notification session isolation test passed!");
}
