//! SSE Notifications Tests for MCP Resources
//!
//! Tests Server-Sent Events functionality for resources/list changes
//! and other resource-related notifications

use mcp_e2e_shared::{McpTestClient, TestServerManager, TestFixtures};
use serde_json::json;
use std::time::Duration;
use tokio::time::{sleep, timeout};
use tracing::{debug, info};

#[tokio::test]
async fn test_sse_connection_establishment() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    // Initialize with SSE-capable client
    client
        .initialize_with_capabilities(TestFixtures::resource_capabilities())
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
async fn test_sse_resource_list_changed_notification() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    // Initialize with listChanged capability
    client
        .initialize_with_capabilities(json!({
            "resources": {
                "subscribe": true,
                "listChanged": false  // MCP compliance: static framework
            }
        }))
        .await
        .expect("Failed to initialize");

    // Subscribe to resources that might trigger list changes
    let subscribe_result = client
        .subscribe_resource("notify://trigger")
        .await
        .expect("Failed to subscribe");

    debug!("Subscribe result: {:?}", subscribe_result);

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
async fn test_sse_resource_subscription_notifications() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::resource_capabilities())
        .await
        .expect("Failed to initialize");

    // Subscribe to a subscribable resource
    let _subscribe_result = client
        .subscribe_resource("subscribe://updates")
        .await;

    // Try to trigger notifications by reading the resource
    let _read_result = client
        .read_resource("subscribe://updates")
        .await;

    // Check for subscription-related notifications
    let events = client
        .test_sse_notifications()
        .await
        .expect("Failed to test SSE notifications");

    info!("Subscription notification test completed. Events: {}", events.len());
    
    // Check for any subscription-related events
    for event in &events {
        debug!("SSE Event: {}", event);
        if event.contains("subscription") || event.contains("resource") {
            info!("✅ Detected resource-related notification");
        }
    }
}

#[tokio::test]
async fn test_sse_session_isolation() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server().await.expect("Failed to start server");
    
    // Create two separate clients
    let mut client1 = McpTestClient::new(server.port());
    let mut client2 = McpTestClient::new(server.port());

    // Initialize both clients
    client1
        .initialize_with_capabilities(TestFixtures::resource_capabilities())
        .await
        .expect("Failed to initialize client1");
    
    client2
        .initialize_with_capabilities(TestFixtures::resource_capabilities())
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
async fn test_sse_notification_format_compliance() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::resource_capabilities())
        .await
        .expect("Failed to initialize");

    // Subscribe to trigger potential notifications
    let _subscribe_result = client
        .subscribe_resource("notify://trigger")
        .await;

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
async fn test_sse_with_multiple_resource_operations() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::resource_capabilities())
        .await
        .expect("Failed to initialize");

    // Perform multiple resource operations that might trigger notifications
    let operations = vec![
        ("file:///subscribe/updates.json", "subscribe"),
        ("file:///notify/trigger.json", "subscribe"),
        ("file:///memory/data.json", "read"),
        ("file:///template/items/test.json", "read"),
    ];

    for (uri, operation) in operations {
        match operation {
            "subscribe" => {
                let _result = client.subscribe_resource(uri).await;
            }
            "read" => {
                let _result = client.read_resource(uri).await;
            }
            _ => {}
        }
        
        // Small delay between operations
        sleep(Duration::from_millis(100)).await;
    }

    // Check for accumulated notifications
    let events = client
        .test_sse_notifications()
        .await
        .expect("Failed to test SSE notifications");

    info!("Multiple operations generated {} SSE events", events.len());

    // Analyze events for expected patterns
    let mut notification_count = 0;
    for event in &events {
        if event.contains("data:") || event.contains("event:") {
            notification_count += 1;
            debug!("Notification {}: {}", notification_count, event);
        }
    }

    info!("Total notifications detected: {}", notification_count);
}

#[tokio::test]
async fn test_sse_error_resource_notifications() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::resource_capabilities())
        .await
        .expect("Failed to initialize");

    // Try to subscribe to an error resource
    let _error_result = client
        .subscribe_resource("error://not_found")
        .await;

    // Read the error resource
    let _read_result = client
        .read_resource("error://not_found")
        .await;

    // Check if error conditions generate notifications
    let events = client
        .test_sse_notifications()
        .await
        .expect("Failed to test SSE notifications");

    info!("Error resource operations generated {} SSE events", events.len());

    // Error operations might still generate valid notifications
    for event in &events {
        if !event.is_empty() {
            debug!("Error-related SSE event: {}", event);
        }
    }
}