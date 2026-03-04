//! Integration tests for client handling of server-initiated requests.
//!
//! Validates the full internal pipeline:
//!   Transport event → StreamHandler → callback → response channel → consumer → transport.send()
//!
//! These tests use the streaming infrastructure directly (not a live HTTP server)
//! to verify that JSON-RPC responses are correctly constructed and forwarded
//! back through the transport layer.

use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use turul_mcp_client::streaming::StreamHandler;
use turul_mcp_client::transport::ServerEvent;

/// Full pipeline test: server request → callback → response channel → verify format.
/// Tests both success and error paths with proper JSON-RPC 2.0 response structure.
#[tokio::test]
async fn test_server_request_response_pipeline_success() {
    let (event_tx, event_rx) = mpsc::unbounded_channel();
    let (response_tx, mut response_rx) = mpsc::unbounded_channel::<Value>();

    let mut handler = StreamHandler::new();
    handler.set_receiver(event_rx);
    handler.set_response_sender(response_tx);

    // Simulate a sampling/createMessage callback
    handler.on_request(|request| {
        let method = request
            .get("method")
            .and_then(|m| m.as_str())
            .unwrap_or("");
        match method {
            "sampling/createMessage" => Ok(json!({
                "role": "assistant",
                "content": {
                    "type": "text",
                    "text": "Hello from the client callback"
                },
                "model": "test-model"
            })),
            _ => Err(format!("Unsupported method: {}", method)),
        }
    });

    handler.start().await.unwrap();

    // Simulate server sending a sampling request with string id
    event_tx
        .send(ServerEvent::Request(json!({
            "jsonrpc": "2.0",
            "id": "srv-sampling-1",
            "method": "sampling/createMessage",
            "params": {
                "messages": [{"role": "user", "content": {"type": "text", "text": "Hi"}}],
                "maxTokens": 100
            }
        })))
        .unwrap();

    let response = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        response_rx.recv(),
    )
    .await
    .expect("timed out waiting for response")
    .expect("channel closed");

    // Verify JSON-RPC 2.0 response structure
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], "srv-sampling-1");
    assert!(response.get("error").is_none(), "should not have error field");

    let result = &response["result"];
    assert_eq!(result["role"], "assistant");
    assert_eq!(result["model"], "test-model");
}

/// Pipeline test with numeric id and error callback path.
#[tokio::test]
async fn test_server_request_response_pipeline_error() {
    let (event_tx, event_rx) = mpsc::unbounded_channel();
    let (response_tx, mut response_rx) = mpsc::unbounded_channel::<Value>();

    let mut handler = StreamHandler::new();
    handler.set_receiver(event_rx);
    handler.set_response_sender(response_tx);

    // Callback that returns an error for unknown methods
    handler.on_request(|request| {
        let method = request
            .get("method")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown");
        Err(format!("Client does not support: {}", method))
    });

    handler.start().await.unwrap();

    // Simulate server sending an elicitation request with numeric id
    event_tx
        .send(ServerEvent::Request(json!({
            "jsonrpc": "2.0",
            "id": 42,
            "method": "elicitation/create",
            "params": {
                "message": "Please provide input",
                "requestedSchema": {}
            }
        })))
        .unwrap();

    let response = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        response_rx.recv(),
    )
    .await
    .expect("timed out waiting for response")
    .expect("channel closed");

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 42);
    assert!(response.get("result").is_none(), "should not have result field");
    assert_eq!(response["error"]["code"], -32603);
    assert!(
        response["error"]["message"]
            .as_str()
            .unwrap()
            .contains("elicitation/create"),
        "error should mention the method"
    );
}

/// Pipeline test: multiple server requests in sequence produce correctly correlated responses.
#[tokio::test]
async fn test_server_request_response_pipeline_multiple_requests() {
    let (event_tx, event_rx) = mpsc::unbounded_channel();
    let (response_tx, mut response_rx) = mpsc::unbounded_channel::<Value>();

    let mut handler = StreamHandler::new();
    handler.set_receiver(event_rx);
    handler.set_response_sender(response_tx);

    let call_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let call_count_clone = Arc::clone(&call_count);

    handler.on_request(move |_request| {
        let n = call_count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(json!({"call_number": n}))
    });

    handler.start().await.unwrap();

    // Send three requests with different id types
    for (i, id) in [json!("alpha"), json!(99), json!("omega")].iter().enumerate() {
        event_tx
            .send(ServerEvent::Request(json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "sampling/createMessage",
                "params": {"seq": i}
            })))
            .unwrap();
    }

    // Collect three responses
    let mut responses = Vec::new();
    for _ in 0..3 {
        let resp = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            response_rx.recv(),
        )
        .await
        .expect("timed out")
        .expect("channel closed");
        responses.push(resp);
    }

    // Verify ids match and call_number increments
    assert_eq!(responses[0]["id"], "alpha");
    assert_eq!(responses[0]["result"]["call_number"], 0);
    assert_eq!(responses[1]["id"], 99);
    assert_eq!(responses[1]["result"]["call_number"], 1);
    assert_eq!(responses[2]["id"], "omega");
    assert_eq!(responses[2]["result"]["call_number"], 2);
}

/// Pipeline test: ServerEvent::Response (id-only) does NOT trigger callback or response.
#[tokio::test]
async fn test_response_event_does_not_trigger_callback() {
    let (event_tx, event_rx) = mpsc::unbounded_channel();
    let (response_tx, mut response_rx) = mpsc::unbounded_channel::<Value>();

    let mut handler = StreamHandler::new();
    handler.set_receiver(event_rx);
    handler.set_response_sender(response_tx);

    let callback_called = Arc::new(Mutex::new(false));
    let callback_called_clone = Arc::clone(&callback_called);

    handler.on_request(move |_req| {
        *callback_called_clone.lock().unwrap() = true;
        Ok(json!({"should_not": "happen"}))
    });

    handler.start().await.unwrap();

    // Send a Response event (id-only, no method) — this is a response to the
    // client's own request, NOT a server-initiated request.
    event_tx
        .send(ServerEvent::Response(json!({
            "jsonrpc": "2.0",
            "id": "client-req-1",
            "result": {"tools": []}
        })))
        .unwrap();

    // Give handler time to process
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Callback should NOT have been called
    assert!(!*callback_called.lock().unwrap(), "Response event should not trigger request callback");

    // No response should be emitted
    let result = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        response_rx.recv(),
    )
    .await;
    assert!(result.is_err(), "Response event should not produce a reply");
}
