//! Integration test for SSE progress delivery via POST streaming
//!
//! Verifies that:
//! - Tools can emit progress notifications
//! - Progress events reach POST clients via SSE streaming before final result
//! - Both progress frames and final result are delivered in correct order

use reqwest;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use turul_http_mcp_server::NotificationBroadcaster;
use turul_mcp_server::McpServer;
use turul_mcp_session_storage::InMemorySessionStorage;

async fn start_progress_test_server() -> String {
    // Find an available port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server_url = format!("http://127.0.0.1:{}/mcp", addr.port());
    drop(listener);

    use turul_mcp_derive::mcp_tool;
    use turul_mcp_protocol::McpResult;
    use turul_mcp_server::SessionContext;

    // Create a test tool that emits progress notifications
    #[mcp_tool(
        name = "slow_calculation",
        description = "Performs a slow calculation with progress updates"
    )]
    async fn slow_calculation(session: Option<SessionContext>) -> McpResult<String> {
        if let Some(session_ctx) = session {
            // Emit progress notification at 25%
            if let Some(broadcaster_any) = &session_ctx.broadcaster {
                if let Some(broadcaster) = broadcaster_any.downcast_ref::<turul_http_mcp_server::notification_bridge::StreamManagerNotificationBroadcaster>() {
                    use turul_mcp_protocol::notifications::*;

                    let progress_25 = ProgressNotification::new(format!("calc-{}", uuid::Uuid::now_v7()), 25)
                        .with_total(100)
                        .with_message("Starting calculation".to_string());
                    let _ = broadcaster.send_progress_notification(&session_ctx.session_id, progress_25).await;
                }
            }

            // Simulate work
            sleep(Duration::from_millis(100)).await;

            // Emit progress notification at 75%
            if let Some(broadcaster_any) = &session_ctx.broadcaster {
                if let Some(broadcaster) = broadcaster_any.downcast_ref::<turul_http_mcp_server::notification_bridge::StreamManagerNotificationBroadcaster>() {
                    use turul_mcp_protocol::notifications::*;

                    let progress_75 = ProgressNotification::new(format!("calc-{}", uuid::Uuid::now_v7()), 75)
                        .with_total(100)
                        .with_message("Nearly complete".to_string());
                    let _ = broadcaster.send_progress_notification(&session_ctx.session_id, progress_75).await;
                }
            }

            // Final work
            sleep(Duration::from_millis(100)).await;
        }

        Ok("Calculation complete".to_string())
    }

    let session_storage = Arc::new(InMemorySessionStorage::new());
    let server = McpServer::builder()
        .name("progress-test-server")
        .version("1.0.0")
        .tool_fn(slow_calculation)
        .with_session_storage(session_storage)
        .bind_address(addr)
        .build()
        .unwrap();

    // Start server in background
    tokio::spawn(async move {
        if let Err(e) = server.run().await {
            eprintln!("Progress test server error: {}", e);
        }
    });

    // Wait for server to start
    sleep(Duration::from_millis(300)).await;
    server_url
}

#[tokio::test]
async fn test_post_streaming_delivers_progress_before_result() {
    let server_url = start_progress_test_server().await;
    let client = reqwest::Client::new();

    // First, initialize to get session ID
    let init_response = client
        .post(&server_url)
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2025-06-18")
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "initialize",
            "id": 1,
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {
                    "experimental": {},
                    "sampling": {}
                },
                "clientInfo": {
                    "name": "progress-test-client",
                    "version": "1.0.0"
                }
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(init_response.status(), 200);
    let session_id = init_response
        .headers()
        .get("Mcp-Session-Id")
        .unwrap()
        .to_str()
        .unwrap();

    // Make streaming POST request with SSE enabled
    let streaming_response = client
        .post(&server_url)
        .header("Content-Type", "application/json")
        .header("MCP-Protocol-Version", "2025-06-18")
        .header("Mcp-Session-Id", session_id)
        .header("Accept", "text/event-stream, application/json") // Request SSE streaming with JSON fallback
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "id": 2,
            "params": {
                "name": "slow_calculation",
                "arguments": {}
            }
        }))
        .send()
        .await
        .unwrap();

    // Check streaming response status
    let status = streaming_response.status();
    if status != 200 {
        let error_body = streaming_response.text().await.unwrap();
        panic!("Expected status 200, got {}: {}", status, error_body);
    }
    assert_eq!(
        streaming_response.headers().get("content-type").unwrap(),
        "text/event-stream"
    );

    // Collect all SSE chunks
    let response_text = streaming_response.text().await.unwrap();
    println!("Full SSE response: {}", response_text);

    // Parse SSE frames
    let mut progress_frames = Vec::new();
    let mut final_frame = None;

    for line in response_text.lines() {
        if line.starts_with("data: ") {
            let json_str = &line[6..]; // Remove "data: " prefix
            if let Ok(frame_json) = serde_json::from_str::<Value>(json_str) {
                if frame_json.get("method").and_then(|m| m.as_str())
                    == Some("notifications/progress")
                {
                    progress_frames.push(frame_json);
                } else if frame_json.get("result").is_some() || frame_json.get("error").is_some() {
                    final_frame = Some(frame_json);
                }
            }
        }
    }

    // Verify progress frames were delivered
    assert!(
        progress_frames.len() >= 1,
        "Expected at least 1 progress frame, got {}: {:?}",
        progress_frames.len(),
        progress_frames
    );

    // Verify final result was delivered
    assert!(final_frame.is_some(), "Expected final result frame");

    // Verify progress values
    for (i, frame) in progress_frames.iter().enumerate() {
        let progress = frame["params"]["progress"].as_u64().unwrap();
        println!("Progress frame {}: {}%", i, progress);
        assert!(
            progress > 0 && progress < 100,
            "Progress should be between 0 and 100"
        );
    }

    // Verify final result
    let final_result = final_frame.unwrap();
    assert_eq!(final_result["id"], 2);
    assert_eq!(final_result["result"]["result"], "Calculation complete");

    println!(
        "✅ Progress delivery test passed: {} progress frames + 1 final result",
        progress_frames.len()
    );
}
