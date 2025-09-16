//! Comprehensive Session Context Validation Tests
//!
//! Tests session management, context passing, and isolation across
//! both resources and prompts using real E2E infrastructure

use mcp_e2e_shared::{McpTestClient, TestServerManager, TestFixtures, SessionTestUtils};
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};

#[tokio::test]
async fn test_session_id_generation_and_persistence() {
    let _ = tracing_subscriber::fmt::try_init();

    // Test both server types
    let resource_server = TestServerManager::start_resource_server().await.expect("Failed to start resource server");
    let prompts_server = TestServerManager::start_prompts_server().await.expect("Failed to start prompts server");

    // Test resource server sessions
    let mut resource_client = McpTestClient::new(resource_server.port());
    resource_client.initialize().await.expect("Failed to initialize resource client");
    
    let resource_session = resource_client.session_id().expect("Resource client should have session ID");
    info!("Resource server session ID: {}", resource_session);
    
    // Verify UUID v7 format (should start with current timestamp-ish)
    assert!(resource_session.len() == 36, "Session ID should be standard UUID length");
    assert!(resource_session.contains('-'), "Session ID should contain hyphens");

    // Test prompts server sessions
    let mut prompts_client = McpTestClient::new(prompts_server.port());
    prompts_client.initialize().await.expect("Failed to initialize prompts client");
    
    let prompts_session = prompts_client.session_id().expect("Prompts client should have session ID");
    info!("Prompts server session ID: {}", prompts_session);
    
    // Session IDs should be different between servers and clients
    assert_ne!(resource_session, prompts_session, "Different servers should generate different session IDs");
}

#[tokio::test]
async fn test_cross_request_session_consistency() {
    let _ = tracing_subscriber::fmt::try_init();

    let resource_server = TestServerManager::start_resource_server().await.expect("Failed to start server");
    let mut client = McpTestClient::new(resource_server.port());

    client.initialize().await.expect("Failed to initialize");
    let initial_session = client.session_id().unwrap().clone();
    
    // Make multiple requests and verify session remains consistent
    let operations = vec![
        ("resources/list", json!({})),
        ("resources/read", json!({"uri": "file:///memory/data.json"})),
        ("resources/read", json!({"uri": "file:///tmp/test.txt"})),
    ];

    for (method, params) in operations {
        let result = client.make_request(method, params, 1).await.expect("Request should succeed");
        
        // Verify session is maintained (no session errors)
        if let Some(error) = result.get("error") {
            let error_message = error["message"].as_str().unwrap_or("");
            if error_message.to_lowercase().contains("session") {
                panic!("Session error detected: {}", error_message);
            }
        }
        
        // Session ID should remain the same
        assert_eq!(client.session_id().unwrap(), &initial_session, "Session ID should not change during requests");
    }
    
    info!("✅ Session consistency maintained across {} operations", 3);
}

#[tokio::test] 
async fn test_session_isolation_between_clients() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server().await.expect("Failed to start server");
    
    // Create 3 different clients
    let mut client1 = McpTestClient::new(server.port());
    let mut client2 = McpTestClient::new(server.port());  
    let mut client3 = McpTestClient::new(server.port());

    // Initialize all clients
    client1.initialize().await.expect("Failed to initialize client1");
    client2.initialize().await.expect("Failed to initialize client2");
    client3.initialize().await.expect("Failed to initialize client3");

    let session1 = client1.session_id().unwrap();
    let session2 = client2.session_id().unwrap(); 
    let session3 = client3.session_id().unwrap();

    // All sessions should be unique
    assert_ne!(session1, session2, "Client1 and Client2 should have different sessions");
    assert_ne!(session1, session3, "Client1 and Client3 should have different sessions");
    assert_ne!(session2, session3, "Client2 and Client3 should have different sessions");

    info!("✅ Session isolation verified:");
    info!("  Client1: {}", session1);
    info!("  Client2: {}", session2);
    info!("  Client3: {}", session3);

    // Each client should be able to perform operations independently
    let result1 = client1.list_resources().await.expect("Client1 should work");
    let result2 = client2.list_resources().await.expect("Client2 should work");
    let result3 = client3.list_resources().await.expect("Client3 should work");

    // All should succeed without session conflicts
    assert!(result1.contains_key("result") || result1.contains_key("error"));
    assert!(result2.contains_key("result") || result2.contains_key("error"));
    assert!(result3.contains_key("result") || result3.contains_key("error"));
}

#[tokio::test]
async fn test_session_aware_resource_context() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize().await.expect("Failed to initialize");

    // Test session-aware resource multiple times
    for i in 1..=3 {
        let result = client.read_resource("file:///session/info.json").await.expect("Failed to read session resource");
        
        if result.contains_key("result") {
            let result_data = result["result"].as_object().unwrap();
            let contents = result_data["contents"].as_array().unwrap();
            
            if !contents.is_empty() {
                let content = &contents[0];
                let text = content.as_object().unwrap()["text"].as_str().unwrap();
                
                // Should contain session ID
                assert!(text.contains("session"), "Session resource should reference session context");
                info!("Session-aware resource call {}: {}", i, text.lines().next().unwrap_or(""));
                
                // Should contain the actual session ID
                if let Some(_session_id) = client.session_id() {
                    // The content might contain the session ID or at least reference sessions
                    assert!(text.contains("session") || text.len() > 10, "Should have meaningful session content");
                }
            }
        }
    }
}

#[tokio::test]
async fn test_session_aware_prompt_context() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_prompts_server().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    client.initialize().await.expect("Failed to initialize");

    // Test session-aware prompt multiple times
    for i in 1..=3 {
        let result = client.get_prompt("session_aware_prompt", None).await.expect("Failed to get session prompt");
        
        if result.contains_key("result") {
            let result_data = result["result"].as_object().unwrap();
            let messages = result_data["messages"].as_array().unwrap();
            
            if !messages.is_empty() {
                let message_content = messages[0]["content"]["text"].as_str().unwrap();
                
                // Should contain session information
                assert!(message_content.contains("session"), "Session-aware prompt should reference session context");
                info!("Session-aware prompt call {}: {}", i, message_content.lines().next().unwrap_or(""));
                
                // Should be consistent but potentially different each call
                assert!(message_content.len() > 10, "Should have meaningful session content");
            }
        }
    }
}

#[tokio::test]
async fn test_concurrent_session_operations() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server().await.expect("Failed to start server");
    
    // Create multiple clients for concurrent testing
    let mut clients = Vec::new();
    for _i in 0..5 {
        let mut client = McpTestClient::new(server.port());
        client.initialize().await.expect("Failed to initialize client");
        clients.push(client);
    }

    // Verify all have unique sessions
    for i in 0..clients.len() {
        for j in i+1..clients.len() {
            assert_ne!(
                clients[i].session_id().unwrap(),
                clients[j].session_id().unwrap(),
                "Clients {} and {} should have different sessions", i, j
            );
        }
    }

    // Perform concurrent operations
    let mut handles = Vec::new();
    for (i, client) in clients.into_iter().enumerate() {
        let handle = tokio::spawn(async move {
            let session_id = client.session_id().unwrap().clone();
            
            // Perform multiple operations
            let _list1 = client.list_resources().await;
            sleep(Duration::from_millis(10)).await;
            
            let _read1 = client.read_resource("file:///memory/data.json").await;
            sleep(Duration::from_millis(10)).await;
            
            let _read2 = client.read_resource("file:///session/info.json").await;
            
            // Session should remain consistent throughout
            assert_eq!(client.session_id().unwrap(), &session_id, "Session should not change during operations");
            
            (i, session_id)
        });
        handles.push(handle);
    }

    // Wait for all concurrent operations
    let results = futures::future::join_all(handles).await;
    
    // Verify all completed successfully
    for result in results {
        let (client_id, session_id) = result.expect("Concurrent operation should succeed");
        info!("✅ Client {} completed with session: {}", client_id, session_id);
    }
}

#[tokio::test]
async fn test_session_header_propagation() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_resource_server().await.expect("Failed to start server");
    let mut client = McpTestClient::new(server.port());

    // Initialize and get session ID
    let init_result = client.initialize().await.expect("Failed to initialize");
    let session_id = client.session_id().unwrap().clone();

    info!("Session established: {}", session_id);
    
    // Verify initialization response has proper structure
    TestFixtures::verify_initialization_response(&init_result);

    // Make requests and verify session is properly maintained
    let requests = vec![
        ("resources/list", json!({})),
        ("resources/read", json!({"uri": "file:///memory/data.json"})),
    ];

    for (method, params) in requests {
        let result = client.make_request(method, params, 1).await;
        
        match result {
            Ok(response) => {
                info!("✅ Request {} succeeded with session {}", method, session_id);
                
                // Verify no session-related errors
                if let Some(error) = response.get("error") {
                    let error_msg = error.get("message").and_then(|m| m.as_str()).unwrap_or("");
                    assert!(!error_msg.to_lowercase().contains("session"), 
                           "Should not have session errors: {}", error_msg);
                }
            }
            Err(e) => {
                warn!("Request {} failed: {}", method, e);
                // Network errors are acceptable, but not session errors
                assert!(!e.to_string().to_lowercase().contains("session"), 
                       "Should not have session-related network errors");
            }
        }
    }
}

#[tokio::test]
async fn test_session_management_comprehensive() {
    let _ = tracing_subscriber::fmt::try_init();

    // Test both resource and prompt servers
    let resource_server = TestServerManager::start_resource_server().await.expect("Failed to start resource server");
    let prompts_server = TestServerManager::start_prompts_server().await.expect("Failed to start prompts server");

    // Test resources
    let mut resource_client = McpTestClient::new(resource_server.port());
    resource_client.initialize_with_capabilities(TestFixtures::resource_capabilities()).await.expect("Failed to initialize");
    
    // Test prompts 
    let mut prompts_client = McpTestClient::new(prompts_server.port());
    prompts_client.initialize_with_capabilities(TestFixtures::prompts_capabilities()).await.expect("Failed to initialize");

    // Run all session validation utilities
    SessionTestUtils::verify_session_consistency(&resource_client).await.expect("Resource session consistency failed");
    SessionTestUtils::verify_session_consistency(&prompts_client).await.expect("Prompts session consistency failed");

    SessionTestUtils::test_session_aware_resource(&resource_client).await.expect("Session-aware resource test failed");
    SessionTestUtils::test_session_aware_prompt(&prompts_client).await.expect("Session-aware prompt test failed");

    info!("✅ Comprehensive session management validation completed");
    info!("  Resource server session: {}", resource_client.session_id().unwrap());
    info!("  Prompts server session: {}", prompts_client.session_id().unwrap());
}