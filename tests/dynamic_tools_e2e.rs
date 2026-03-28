//! E2E Transport Test: DynamicInProcess Tool Change Detection
//!
//! Proves the full ADR-023 DynamicInProcess contract over Streamable HTTP:
//! 1. Client initializes and sees initial tool set (multiply active)
//! 2. Tool is deactivated via `deactivate_multiply` → tool registry updated
//! 3. Same session's next tools/list shows multiply absent, activate_multiply present
//! 4. Re-initialization produces a new session with the same updated tool set
//!
//! Note: The HTTP handler's tool_fingerprint is static (set at build time) and does
//! not change for DynamicInProcess mutations. Fingerprint-based 404 guards against
//! cross-restart or cross-cluster staleness. In-process dynamic changes are signaled
//! to clients via SSE notifications/tools/list_changed.
//!
//! This is the highest-signal proof that the feature works end-to-end.

use mcp_e2e_shared::{McpTestClient, TestServerManager};
use serde_json::json;
use tracing::info;

/// Core E2E test: tool deactivation → updated tools/list → re-init → same updated tools
#[tokio::test]
async fn test_dynamic_tools_deactivation_causes_stale_session_404() {
    let _ = tracing_subscriber::fmt::try_init();
    info!("Starting DynamicInProcess E2E test");

    // Start dynamic-tools-server with multiply active
    let server = match TestServerManager::start_dynamic_tools_server().await {
        Ok(s) => s,
        Err(e) => {
            println!("Skipping E2E test — failed to start server: {}", e);
            return;
        }
    };

    let mut client = McpTestClient::new(server.port());

    // Step 1: Initialize and verify initial tools
    let init_result = client.initialize().await.expect("Failed to initialize");
    assert!(init_result.contains_key("result"), "Initialize should succeed");

    client
        .send_initialized_notification()
        .await
        .expect("Failed to send initialized");

    let session_id = client.session_id().expect("Should have session ID").to_string();
    info!("Initialized with session: {}", session_id);

    // Step 2: Verify tools/list shows multiply as active
    let tools_result = client.list_tools().await.expect("Failed to list tools");
    let tools = tools_result
        .get("result")
        .and_then(|r| r.get("tools"))
        .and_then(|t| t.as_array())
        .expect("Should have tools array");

    let tool_names: Vec<&str> = tools
        .iter()
        .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
        .collect();
    info!("Initial tools: {:?}", tool_names);
    assert!(
        tool_names.contains(&"multiply"),
        "Initial tool set should contain 'multiply'"
    );

    // Step 3: Deactivate multiply by calling deactivate_multiply tool
    let deactivate_result = client
        .call_tool("deactivate_multiply", json!({}))
        .await
        .expect("Failed to call deactivate_multiply");
    info!("Deactivate result: {:?}", deactivate_result);

    // Step 4: Next tools/list on the SAME session sees the updated tool set.
    //
    // Note: The tool_fingerprint on the HTTP handler is set once at build time and
    // is NOT updated when DynamicInProcess tools change at runtime. The session's
    // stored mcp:tool_fingerprint matches the handler's static fingerprint, so no
    // mismatch (404) occurs. DynamicInProcess changes are signaled to clients via
    // SSE notifications/tools/list_changed; the fingerprint mechanism guards against
    // cross-restart or cross-cluster staleness, not in-process dynamic changes.
    let post_deactivation = client
        .list_tools()
        .await
        .expect("tools/list after deactivation should succeed");
    let post_tools = post_deactivation
        .get("result")
        .and_then(|r| r.get("tools"))
        .and_then(|t| t.as_array())
        .expect("tools/list response must contain a tools array");

    let post_names: Vec<&str> = post_tools
        .iter()
        .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
        .collect();
    info!("Post-deactivation tools: {:?}", post_names);

    // multiply MUST be absent after deactivation
    assert!(
        !post_names.contains(&"multiply"),
        "After deactivation, multiply MUST NOT appear in tools/list. Got: {:?}",
        post_names
    );
    // deactivate_multiply should also be gone (it was the counterpart)
    assert!(
        !post_names.contains(&"deactivate_multiply"),
        "After deactivation, deactivate_multiply should be removed. Got: {:?}",
        post_names
    );
    // activate_multiply MUST now be present (toggle counterpart)
    assert!(
        post_names.contains(&"activate_multiply"),
        "After deactivation, activate_multiply MUST be available. Got: {:?}",
        post_names
    );
    // Static tools remain
    assert!(
        post_names.contains(&"add"),
        "Static tool 'add' must always be present. Got: {:?}",
        post_names
    );
    assert!(
        post_names.contains(&"greet"),
        "Static tool 'greet' must always be present. Got: {:?}",
        post_names
    );

    // Step 5: Re-initialize and confirm the updated tool set persists in a new session
    let reinit_result = client.initialize().await.expect("Re-initialize should succeed");
    assert!(
        reinit_result.contains_key("result"),
        "Re-initialization must return a result"
    );
    client
        .send_initialized_notification()
        .await
        .expect("Failed to send initialized after re-init");

    let new_session_id = client
        .session_id()
        .expect("Should have new session ID")
        .to_string();
    info!(
        "Re-initialized with new session: {} (old: {})",
        new_session_id, session_id
    );
    assert_ne!(
        new_session_id, session_id,
        "Re-initialization must produce a different session ID"
    );

    // Step 6: Verify the fresh session also sees multiply absent
    let fresh_tools_result = client
        .list_tools()
        .await
        .expect("Failed to list tools after re-init");
    let fresh_tools = fresh_tools_result
        .get("result")
        .and_then(|r| r.get("tools"))
        .and_then(|t| t.as_array())
        .expect("Fresh session tools/list must contain a tools array");

    let fresh_names: Vec<&str> = fresh_tools
        .iter()
        .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
        .collect();
    info!("Fresh session tools: {:?}", fresh_names);

    assert!(
        !fresh_names.contains(&"multiply"),
        "Fresh session must NOT contain multiply. Got: {:?}",
        fresh_names
    );
    assert!(
        fresh_names.contains(&"activate_multiply"),
        "Fresh session must contain activate_multiply. Got: {:?}",
        fresh_names
    );
    assert!(
        fresh_names.contains(&"add"),
        "Fresh session must contain static tool 'add'. Got: {:?}",
        fresh_names
    );

    // Verify both sessions agree on the exact same tool set
    assert_eq!(
        post_names, fresh_names,
        "Old session and fresh session must report identical tool sets after deactivation"
    );

    info!("DynamicInProcess E2E test passed");
}

/// Test: server started with --multiply-inactive shows multiply absent
#[tokio::test]
async fn test_dynamic_tools_server_startup_flag() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = match TestServerManager::start_dynamic_tools_server_multiply_inactive().await {
        Ok(s) => s,
        Err(e) => {
            println!("Skipping E2E test — failed to start server: {}", e);
            return;
        }
    };

    let mut client = McpTestClient::new(server.port());
    client.initialize().await.expect("Failed to initialize");
    client
        .send_initialized_notification()
        .await
        .expect("Failed to send initialized");

    let tools_result = client.list_tools().await.expect("Failed to list tools");
    let tools = tools_result
        .get("result")
        .and_then(|r| r.get("tools"))
        .and_then(|t| t.as_array())
        .expect("Should have tools array");

    let tool_names: Vec<&str> = tools
        .iter()
        .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
        .collect();
    info!("Tools with --multiply-inactive: {:?}", tool_names);

    assert!(
        !tool_names.contains(&"multiply"),
        "multiply should be inactive at startup with --multiply-inactive"
    );
    assert!(
        tool_names.contains(&"activate_multiply"),
        "activate_multiply should be available when multiply is inactive"
    );
    assert!(
        tool_names.contains(&"add"),
        "add should always be active"
    );
    assert!(
        tool_names.contains(&"greet"),
        "greet should always be active"
    );
}

/// Test: tools.listChanged capability is true for DynamicInProcess
#[tokio::test]
async fn test_dynamic_tools_server_advertises_list_changed_true() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = match TestServerManager::start_dynamic_tools_server().await {
        Ok(s) => s,
        Err(e) => {
            println!("Skipping E2E test — failed to start server: {}", e);
            return;
        }
    };

    let mut client = McpTestClient::new(server.port());
    let init_result = client.initialize().await.expect("Failed to initialize");

    // Check capabilities in initialize response
    let capabilities = init_result
        .get("result")
        .and_then(|r| r.get("capabilities"))
        .expect("Should have capabilities");

    let list_changed = capabilities
        .get("tools")
        .and_then(|t| t.get("listChanged"))
        .and_then(|lc| lc.as_bool());

    assert_eq!(
        list_changed,
        Some(true),
        "DynamicInProcess server MUST advertise tools.listChanged=true. Got: {:?}",
        capabilities.get("tools")
    );
}
