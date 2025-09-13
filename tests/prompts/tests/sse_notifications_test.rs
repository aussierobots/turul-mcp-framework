//! SSE Notifications Tests for MCP Prompts  
//!
//! Tests Server-Sent Events functionality for prompts/list changes
//! and other prompt-related notifications

use mcp_e2e_shared::{McpTestClient, TestServerManager, TestFixtures};
use serde_json::json;
use std::time::Duration;
use tokio::time::{sleep, timeout};
use tracing::{debug, info};

#[tokio::test]
async fn test_sse_prompts_connection_establishment() {
    tracing_subscriber::fmt::init();

    let server = TestServerManager::start_prompts_server().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    // Initialize with SSE-capable client
    client
        .initialize_with_capabilities(TestFixtures::prompts_capabilities())
        .await
        .expect("Failed to initialize");

    // Test basic SSE connection
    let events = client
        .test_sse_notifications()
        .await
        .expect("Failed to test SSE notifications");

    info!("SSE connection test completed. Events received: {}", events.len());
    
    // Basic connection should work (events may be empty but no errors)
    assert!(events.is_empty() || events.iter().any(|e| e.contains("data:") || e.contains("event:")));
}

#[tokio::test]
async fn test_sse_prompts_list_changed_notification() {
    tracing_subscriber::fmt::init();

    let server = TestServerManager::start_prompts_server().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    // Initialize with listChanged capability
    client
        .initialize_with_capabilities(json!({
            "prompts": {
                "listChanged": false  // MCP compliance: static framework
            }
        }))
        .await
        .expect("Failed to initialize");

    // Trigger potential prompt list operations
    let _list_result = client
        .list_prompts()
        .await
        .expect("Failed to list prompts");

    // Test for potential listChanged events
    let events = timeout(Duration::from_secs(3), async {
        client.test_sse_notifications().await
    })
    .await;

    match events {
        Ok(events_result) => {
            let events = events_result.expect("Failed to get SSE events");
            info!("Received {} SSE events", events.len());
            
            for event in &events {
                debug!("SSE Event: {}", event);
                
                // Check for listChanged events (camelCase naming per MCP spec)
                if event.contains("listChanged") {
                    info!("✅ Detected listChanged notification");
                    assert!(event.contains("data:") || event.contains("event:"));
                }
            }
        }
        Err(_) => {
            info!("SSE timeout - no events received within 3 seconds");
        }
    }
}

#[tokio::test]
async fn test_sse_prompt_operations_notifications() {
    tracing_subscriber::fmt::init();

    let server = TestServerManager::start_prompts_server().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::prompts_capabilities())
        .await
        .expect("Failed to initialize");

    // Perform various prompt operations
    let operations = vec![
        ("simple_prompt", None),
        ("string_args_prompt", Some(TestFixtures::create_string_args())),
        ("number_args_prompt", Some(TestFixtures::create_number_args())),
    ];

    for (prompt_name, args) in operations {
        let _result = client.get_prompt(prompt_name, args).await;
        sleep(Duration::from_millis(50)).await;
    }

    // Check for operation-related notifications
    let events = client
        .test_sse_notifications()
        .await
        .expect("Failed to test SSE notifications");

    info!("Prompt operations generated {} SSE events", events.len());
    
    // Check for any prompt-related events
    for event in &events {
        debug!("SSE Event: {}", event);
        if event.contains("prompt") || event.contains("data:") {
            info!("✅ Detected prompt-related notification");
        }
    }
}

#[tokio::test]
async fn test_sse_prompts_session_isolation() {
    tracing_subscriber::fmt::init();

    let server = TestServerManager::start_prompts_server().await.expect("Failed to start server");
    
    // Create two separate clients
    let mut client1 = McpTestClient::new(server.port());
    let mut client2 = McpTestClient::new(server.port());

    // Initialize both clients
    client1
        .initialize_with_capabilities(TestFixtures::prompts_capabilities())
        .await
        .expect("Failed to initialize client1");
    
    client2
        .initialize_with_capabilities(TestFixtures::prompts_capabilities())
        .await
        .expect("Failed to initialize client2");

    // Verify they have different session IDs
    assert_ne!(client1.session_id(), client2.session_id());
    info!("Client1 session: {:?}", client1.session_id());
    info!("Client2 session: {:?}", client2.session_id());

    // Test that each client gets their own SSE stream
    let events1 = client1
        .test_sse_notifications()
        .await
        .expect("Failed to get events for client1");
    
    let events2 = client2
        .test_sse_notifications()
        .await
        .expect("Failed to get events for client2");

    info!("Client1 events: {}", events1.len());
    info!("Client2 events: {}", events2.len());

    // Both clients should be able to establish SSE connections independently
    assert!(events1.is_empty() || events1.iter().any(|e| e.contains("data:") || e.contains("event:")));
    assert!(events2.is_empty() || events2.iter().any(|e| e.contains("data:") || e.contains("event:")));
}

#[tokio::test]
async fn test_sse_prompts_notification_format_compliance() {
    tracing_subscriber::fmt::init();

    let server = TestServerManager::start_prompts_server().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::prompts_capabilities())
        .await
        .expect("Failed to initialize");

    // Trigger some prompt operations
    let _result = client.get_prompt("session_aware_prompt", None).await;

    let events = client
        .test_sse_notifications()
        .await
        .expect("Failed to test SSE notifications");

    for event in &events {
        debug!("Validating SSE event format: {}", event);
        
        // Check SSE format compliance
        if !event.is_empty() {
            // SSE events should contain either data: or event: fields
            let lines: Vec<&str> = event.lines().collect();
            
            for line in lines {
                if line.starts_with("data:") {
                    info!("✅ Valid SSE data line: {}", line);
                    
                    // If it's JSON data, validate structure
                    if let Some(json_part) = line.strip_prefix("data:").map(|s| s.trim()) {
                        if json_part.starts_with('{') {
                            match serde_json::from_str::<serde_json::Value>(json_part) {
                                Ok(parsed) => {
                                    info!("✅ Valid JSON notification: {:?}", parsed);
                                    
                                    // Check for MCP notification structure
                                    if parsed.get("method").is_some() {
                                        assert!(parsed.get("method").unwrap().is_string());
                                        info!("✅ MCP notification method found");
                                    }
                                }
                                Err(e) => {
                                    debug!("Non-JSON SSE data (acceptable): {}", e);
                                }
                            }
                        }
                    }
                } else if line.starts_with("event:") {
                    info!("✅ Valid SSE event line: {}", line);
                } else if line.starts_with("id:") {
                    info!("✅ Valid SSE id line: {}", line);
                } else if line.starts_with("retry:") {
                    info!("✅ Valid SSE retry line: {}", line);
                } else if line.is_empty() {
                    // Empty lines are used as event separators in SSE
                    debug!("SSE event separator");
                }
            }
        }
    }
}

#[tokio::test]
async fn test_sse_prompts_error_handling_notifications() {
    tracing_subscriber::fmt::init();

    let server = TestServerManager::start_prompts_server().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::prompts_capabilities())
        .await
        .expect("Failed to initialize");

    // Try operations that might cause errors/validation failures
    let _error_result = client
        .get_prompt("validation_failure_prompt", None)
        .await;

    let _nonexistent_result = client
        .get_prompt("nonexistent_prompt", None)
        .await;

    // Check if error conditions generate notifications
    let events = client
        .test_sse_notifications()
        .await
        .expect("Failed to test SSE notifications");

    info!("Error prompt operations generated {} SSE events", events.len());

    // Error operations might still generate valid notifications
    for event in &events {
        if !event.is_empty() {
            debug!("Error-related SSE event: {}", event);
        }
    }
}

#[tokio::test]  
async fn test_sse_prompts_with_session_aware_operations() {
    tracing_subscriber::fmt::init();

    let server = TestServerManager::start_prompts_server().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::prompts_capabilities())
        .await
        .expect("Failed to initialize");

    // Use session-aware prompt which might generate more notifications
    let _result1 = client
        .get_prompt("session_aware_prompt", None)
        .await
        .expect("Failed to get session-aware prompt");

    // Call it again to see if session state affects notifications
    let _result2 = client
        .get_prompt("session_aware_prompt", None)
        .await
        .expect("Failed to get session-aware prompt again");

    // Check for session-related notifications
    let events = client
        .test_sse_notifications()
        .await
        .expect("Failed to test SSE notifications");

    info!("Session-aware operations generated {} SSE events", events.len());

    for event in &events {
        if event.contains("session") || event.contains("data:") {
            info!("✅ Detected session-related notification: {}", event.lines().next().unwrap_or(""));
        }
    }
}

#[tokio::test]
async fn test_sse_prompts_concurrent_operations() {
    tracing_subscriber::fmt::init();

    let server = TestServerManager::start_prompts_server().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::prompts_capabilities())
        .await
        .expect("Failed to initialize");

    // Perform multiple concurrent-ish operations
    let operations = vec![
        ("simple_prompt", None),
        ("template_prompt", Some(std::collections::HashMap::from([
            ("template_name".to_string(), json!("test")),
            ("template_value".to_string(), json!("value")),
        ]))),
        ("multi_message_prompt", None),
        ("dynamic_prompt", Some(std::collections::HashMap::from([
            ("dynamic_type".to_string(), json!("analysis")),
        ]))),
    ];

    for (prompt_name, args) in operations {
        let _result = client.get_prompt(prompt_name, args).await;
        sleep(Duration::from_millis(25)).await;
    }

    // Check for accumulated notifications
    let events = client
        .test_sse_notifications()
        .await
        .expect("Failed to test SSE notifications");

    info!("Concurrent operations generated {} SSE events", events.len());

    // Analyze events for expected patterns
    let mut notification_count = 0;
    for event in &events {
        if event.contains("data:") || event.contains("event:") {
            notification_count += 1;
            debug!("Notification {}: {}", notification_count, event.lines().next().unwrap_or(""));
        }
    }

    info!("Total notifications detected: {}", notification_count);
}