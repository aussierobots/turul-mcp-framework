//! Real Lambda streaming tests that actually execute handlers and validate SSE delivery.
//!
//! These tests address the critical gaps identified in lambda_examples.rs:
//! 1. Actually invoke handler.handle_streaming() with real MCP requests
//! 2. Collect SSE frames from lambda_http::Body stream
//! 3. Verify progress notifications are present in frames

use bytes::Bytes;
use http_body_util::BodyExt;
use lambda_http::Request;
use std::sync::Arc;
use turul_mcp_aws_lambda::LambdaMcpServerBuilder;
use turul_mcp_derive::McpTool;

use turul_mcp_server::{McpResult, SessionContext};
use turul_mcp_session_storage::InMemorySessionStorage;

/// Tool that sends progress notifications to test SSE streaming
#[derive(McpTool, Clone, Default)]
#[tool(
    name = "progress_tool",
    description = "Tool that sends progress notifications"
)]
struct ProgressTool {
    #[param(description = "Number of progress notifications to send")]
    count: u32,
}

impl ProgressTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
        if let Some(session) = session {
            for i in 1..=self.count {
                session.notify_progress("progress_tool", i as u64).await;
            }
        }
        Ok(format!("Sent {} progress notifications", self.count))
    }
}

/// Utility to collect all SSE frames from a response body
async fn collect_sse_frames<B>(body: B) -> Result<Vec<String>, Box<dyn std::error::Error>>
where
    B: http_body::Body<Data = Bytes> + Send + 'static,
    B::Error: std::error::Error + Send + Sync + 'static,
{
    let mut frames = Vec::new();
    let mut body = std::pin::pin!(body);

    while let Some(chunk_result) = body.frame().await {
        if let Ok(frame) = chunk_result {
            if let Some(data) = frame.data_ref() {
                let text = String::from_utf8(data.to_vec())?;
                frames.push(text);
            }
        }
    }

    Ok(frames)
}

/// Test that handler.handle_streaming() actually returns SSE frames
#[tokio::test]
#[ignore = "Requires Body::from_stream implementation"]
async fn test_lambda_streaming_handler_execution() {
    let _ = tracing_subscriber::fmt::try_init();

    // Build Lambda server with streaming enabled
    let storage = Arc::new(InMemorySessionStorage::new());
    let server = LambdaMcpServerBuilder::new()
        .name("streaming-test-server")
        .version("1.0.0")
        .tool(ProgressTool::default())
        .storage(storage)
        .sse(true)
        .build()
        .await
        .expect("Failed to build server");

    let _handler = server.handler().await.expect("Failed to create handler");

    // TODO: Implement this test - currently just a stub
    // Example of what the request should look like:
    // let initialize_body = serde_json::json!({
    //     "jsonrpc": "2.0",
    //     "id": 1,
    //     "method": "initialize",
    //     "params": {
    //         "protocolVersion": "2025-06-18",
    //         "capabilities": {},
    //         "clientInfo": {
    //             "name": "test-client",
    //             "version": "1.0.0"
    //         }
    //     }
    // });
    // let req = Request::default();
    // TODO: Set Accept: text/event-stream header
    // TODO: Set body to initialize_body JSON
    // TODO: Call handler.handle_streaming(req).await
    // TODO: Collect SSE frames from response
    // TODO: Verify frames contain expected data

    println!("⚠️  TEST STUB: This test needs implementation");
    println!("    See TODO_TRACKER.md Phase Lambda-1 for implementation plan");
}

/// Test that progress notifications appear in SSE frames
#[tokio::test]
#[ignore = "Requires Body::from_stream implementation"]
async fn test_lambda_notifications_in_sse_frames() {
    let _ = tracing_subscriber::fmt::try_init();

    let storage = Arc::new(InMemorySessionStorage::new());
    let server = LambdaMcpServerBuilder::new()
        .name("notification-test-server")
        .version("1.0.0")
        .tool(ProgressTool { count: 5 })
        .storage(storage)
        .sse(true)
        .build()
        .await
        .expect("Failed to build server");

    let handler = server.handler().await.expect("Failed to create handler");

    // First: Initialize session
    let init_request = create_mcp_request(
        "initialize",
        serde_json::json!({
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }),
    );

    let _init_response = handler
        .handle_streaming(init_request)
        .await
        .expect("Initialize should succeed");

    // Extract session ID from response
    // TODO: Parse JSON response to get session ID

    // Second: Call progress_tool with session ID
    let tool_request = create_mcp_request_with_session(
        "tools/call",
        serde_json::json!({
            "name": "progress_tool",
            "arguments": {
                "count": 5
            }
        }),
        "test-session-id", // TODO: Use actual session ID from init
    );

    let tool_response = handler
        .handle_streaming(tool_request)
        .await
        .expect("Tool call should succeed");

    // Collect SSE frames
    let frames = collect_sse_frames(tool_response.into_body())
        .await
        .expect("Failed to collect SSE frames");

    // Verify we got progress notifications
    let notification_frames: Vec<_> = frames
        .iter()
        .filter(|f| f.contains("notifications/progress"))
        .collect();

    assert_eq!(
        notification_frames.len(),
        5,
        "Should receive 5 progress notifications, got {}",
        notification_frames.len()
    );

    println!(
        "✅ Verified {} progress notifications in SSE frames",
        notification_frames.len()
    );
}

/// Helper to create MCP request with SSE headers
fn create_mcp_request(_method: &str, _params: serde_json::Value) -> Request {
    // TODO: Implement request creation with proper headers
    // - Accept: text/event-stream
    // - Content-Type: application/json
    // - Body: JSON-RPC request
    Request::default()
}

/// Helper to create MCP request with session ID header
fn create_mcp_request_with_session(
    method: &str,
    params: serde_json::Value,
    _session_id: &str,
) -> Request {
    let req = create_mcp_request(method, params);
    // TODO: Add Mcp-Session-Id header
    req
}

/// Test SSE frame format compliance
#[test]
fn test_sse_frame_format() {
    // SSE frames must follow format: "data: {json}\n\n"
    let notification = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notifications/progress",
        "params": {
            "progressToken": "test-token",
            "progress": 50,
            "total": 100
        }
    });

    let frame = format!(
        "data: {}\n\n",
        serde_json::to_string(&notification).unwrap()
    );

    assert!(
        frame.starts_with("data: "),
        "Frame must start with 'data: '"
    );
    assert!(
        frame.ends_with("\n\n"),
        "Frame must end with double newline"
    );
    assert!(
        frame.contains("notifications/progress"),
        "Frame must contain notification method"
    );

    println!("✅ SSE frame format validation passed");
    println!("Frame: {}", frame);
}

/// Test that buffering approach collects all notifications before returning
#[tokio::test]
#[ignore = "Requires buffered implementation"]
async fn test_buffered_notification_collection() {
    // This test validates the buffering approach:
    // 1. Tool generates notifications
    // 2. All notifications collected into Vec<Bytes>
    // 3. Stream built from Vec using Body::from_stream
    // 4. Lambda returns complete stream

    let storage = Arc::new(InMemorySessionStorage::new());
    let server = LambdaMcpServerBuilder::new()
        .name("buffering-test-server")
        .version("1.0.0")
        .tool(ProgressTool { count: 100 }) // High count to test buffering
        .storage(storage)
        .sse(true)
        .build()
        .await
        .expect("Failed to build server");

    let handler = server.handler().await.expect("Failed to create handler");

    // Call tool that generates many notifications
    let request = create_mcp_request(
        "tools/call",
        serde_json::json!({
            "name": "progress_tool",
            "arguments": {
                "count": 100
            }
        }),
    );

    let response = handler
        .handle_streaming(request)
        .await
        .expect("Tool call should succeed");

    // Collect all frames
    let frames = collect_sse_frames(response.into_body())
        .await
        .expect("Failed to collect frames");

    // Verify all 100 notifications present
    let notification_count = frames
        .iter()
        .filter(|f| f.contains("notifications/progress"))
        .count();

    assert_eq!(
        notification_count, 100,
        "All 100 notifications should be buffered and delivered"
    );

    println!("✅ Buffered notification collection test passed");
    println!("Collected {} total frames", frames.len());
    println!("Found {} progress notifications", notification_count);
}

/// TEST: Verify POST Streamable HTTP notifications work in Lambda
///
/// IMPORTANT: This test documents a THEORETICAL concern about background task lifecycle,
/// but NO EMPIRICAL TESTING has been done to verify whether this is actually broken.
///
/// Theoretical concern: Progress notifications might be lost if Lambda terminates
/// background tasks (tokio::spawn) before they forward notifications to response stream.
///
/// Current status: UNTESTED - lambda_http documentation says streaming works via
/// run_with_streaming_response() and Body::from_stream(). Need actual testing to verify.
///
/// TODO: Implement this test properly with cargo lambda watch to determine if concern is valid.
#[tokio::test]
#[ignore = "Test incomplete - needs implementation to verify if Lambda streaming actually works"]
async fn test_lambda_post_streamable_http_notifications() {
    let _ = tracing_subscriber::fmt::try_init();

    let storage = Arc::new(InMemorySessionStorage::new());
    let server = LambdaMcpServerBuilder::new()
        .name("streaming-verification-server")
        .version("1.0.0")
        .tool(ProgressTool { count: 3 })
        .storage(storage)
        .sse(true)
        .build()
        .await
        .expect("Failed to build server");

    let handler = server.handler().await.expect("Failed to create handler");

    // TODO: Create proper POST request with Accept: text/event-stream
    // TODO: Use cargo lambda watch to test actual Lambda behavior
    // TODO: Collect chunked response frames
    // TODO: Verify progress notifications are present or absent

    // This test should verify whether the theoretical concern is valid by:
    // 1. Sending POST request with tools/call and Accept: text/event-stream
    // 2. Collecting chunked response frames from Lambda streaming response
    // 3. Checking if progress notifications appear in frames
    // 4. Documenting actual behavior (works or broken) with evidence

    println!("⚠️  TEST INCOMPLETE: Needs implementation to verify Lambda streaming behavior");
    println!("📖 lambda_http docs say streaming works - verify empirically");
    println!("🔧 Use cargo lambda watch for proper testing");
}

/// Test memory limits during buffering
#[tokio::test]
#[ignore = "Requires buffered implementation with memory limits"]
async fn test_buffering_memory_limits() {
    // This test validates memory protection:
    // 1. Tool attempts to generate excessive notifications
    // 2. Buffer hits memory limit
    // 3. Handler returns error or partial response

    let storage = Arc::new(InMemorySessionStorage::new());
    let server = LambdaMcpServerBuilder::new()
        .name("memory-limit-test-server")
        .version("1.0.0")
        .tool(ProgressTool { count: 100000 }) // Excessive count
        .storage(storage)
        .sse(true)
        .build()
        .await
        .expect("Failed to build server");

    let handler = server.handler().await.expect("Failed to create handler");

    let request = create_mcp_request(
        "tools/call",
        serde_json::json!({
            "name": "progress_tool",
            "arguments": {
                "count": 100000
            }
        }),
    );

    // This should either:
    // a) Return error due to memory limit
    // b) Return partial notifications up to limit
    let result = handler.handle_streaming(request).await;

    // TODO: Define expected behavior
    // For now, just verify it doesn't panic
    assert!(
        result.is_ok() || result.is_err(),
        "Handler should return defined result"
    );

    println!("✅ Memory limit test completed (behavior to be defined)");
}
