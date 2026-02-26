//! Tests for client streaming functionality with MCP Streamable HTTP

use anyhow::Result;
use futures::Stream;
use serde_json::json;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{info, warn};
use turul_mcp_client::transport::{HttpTransport, ServerEvent, Transport};

/// Test helper to create an in-memory byte stream for testing
fn create_test_stream(
    data: Vec<Vec<u8>>,
) -> impl Stream<Item = Result<Vec<u8>, std::io::Error>> + Unpin {
    futures::stream::iter(data.into_iter().map(Ok))
}

/// Test that single JSON responses work correctly (in-memory mode)
#[tokio::test]
async fn test_single_json_response() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init();

    info!("🧪 Testing single JSON response handling (in-memory)");

    let mut transport = HttpTransport::new("http://localhost:8080/mcp")?;

    // Create a mock tools/list response
    let response_json = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "tools": [
                {
                    "name": "calculator",
                    "description": "A calculator tool for basic arithmetic"
                }
            ]
        }
    });

    let response_bytes = serde_json::to_vec(&response_json)?;
    let stream = create_test_stream(vec![response_bytes]);

    // Process the stream
    let result = transport.test_handle_byte_stream(stream).await?;

    // Should get a valid JSON-RPC response with tools list
    assert!(result.get("result").is_some(), "Should have result field");
    assert_eq!(result["jsonrpc"], "2.0");
    assert_eq!(result["id"], 1);

    info!("✅ Single JSON response test passed");

    Ok(())
}

/// Test that streaming JSON frames work correctly (in-memory mode)
#[tokio::test]
async fn test_streaming_json_frames() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init();

    info!("🧪 Testing streaming JSON frames (in-memory)");

    let mut transport = HttpTransport::new("http://localhost:8080/mcp")?;

    // Start event listener to collect progress notifications
    let mut events = transport.start_event_listener().await?;

    // Create progress notifications followed by final response
    let progress1 = json!({
        "jsonrpc": "2.0",
        "method": "notifications/progress",
        "params": {
            "progressToken": "test_stream",
            "progress": 30,
            "total": 100
        }
    });

    let progress2 = json!({
        "jsonrpc": "2.0",
        "method": "notifications/progress",
        "params": {
            "progressToken": "test_stream",
            "progress": 70,
            "total": 100
        }
    });

    let final_response = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "tools": [
                {
                    "name": "advanced_tool",
                    "description": "An advanced tool that reports progress"
                }
            ]
        }
    });

    // Create separate chunks for each frame to test streaming
    let progress1_bytes = serde_json::to_vec(&progress1)?;
    let progress2_bytes = serde_json::to_vec(&progress2)?;
    let final_bytes = serde_json::to_vec(&final_response)?;

    let stream = create_test_stream(vec![progress1_bytes, progress2_bytes, final_bytes]);

    // Start listening for events in the background
    let event_listener = tokio::spawn(async move {
        let mut received_events = Vec::new();

        // Listen for progress notifications with timeout to avoid hanging
        while let Some(event) = timeout(Duration::from_millis(100), events.recv())
            .await
            .ok()
            .flatten()
        {
            match event {
                ServerEvent::Notification(notification) => {
                    info!("📈 Received notification: {:?}", notification);
                    received_events.push(notification);
                }
                ServerEvent::ConnectionLost => {
                    warn!("Connection lost");
                    break;
                }
                _ => {}
            }

            // Don't wait too long for events
            if received_events.len() >= 3 {
                break;
            }
        }

        received_events
    });

    // Process the stream and get final result
    let result = transport.test_handle_byte_stream(stream).await?;

    assert!(result.get("result").is_some(), "Should have result field");
    assert_eq!(result["jsonrpc"], "2.0");
    assert_eq!(result["id"], 1);
    info!("✅ Final result received: {:?}", result["result"]);

    // Collect any events that might have been sent
    let received_events = timeout(Duration::from_secs(1), event_listener)
        .await
        .map_err(|_| anyhow::anyhow!("Event listener timed out"))??;

    // Verify we received the progress notifications
    assert!(
        received_events.len() >= 2,
        "Should have received at least 2 progress notifications"
    );

    info!(
        "✅ Streaming test completed with {} events received",
        received_events.len()
    );
    Ok(())
}

/// Test error response handling (in-memory mode)
#[tokio::test]
async fn test_error_response() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init();

    info!("🧪 Testing error response handling (in-memory)");

    let mut transport = HttpTransport::new("http://localhost:8080/mcp")?;

    // Create a mock error response for a nonexistent method
    let error_response = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "error": {
            "code": -32601,
            "message": "Method not found: nonexistent_method_that_should_fail"
        }
    });

    let error_bytes = serde_json::to_vec(&error_response)?;
    let stream = create_test_stream(vec![error_bytes]);

    // Process the stream - should result in an error
    let result = transport.test_handle_byte_stream(stream).await;

    assert!(
        result.is_err(),
        "Expected error response for nonexistent method"
    );
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("Server error"),
        "Error should mention server error"
    );
    info!("✅ Error response test passed with error: {}", error_msg);

    Ok(())
}
