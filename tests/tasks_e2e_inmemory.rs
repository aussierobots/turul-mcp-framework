//! # Tasks E2E Integration Test (In-Memory Storage)
//!
//! End-to-end tests for the MCP 2025-11-25 task lifecycle using the
//! `tasks-e2e-inmemory-server` example binary.
//!
//! Tests:
//! 1. Task-augmented tools/call returns CreateTaskResult
//! 2. tasks/get shows Working -> Completed status transition
//! 3. tasks/result returns tool output after completion
//! 4. tasks/list returns task entries
//! 5. tasks/cancel transitions task to Cancelled
//! 6. Synchronous call (no task augmentation) still works
//! 7. Server capabilities advertise tasks.requests.tools.call

use mcp_e2e_shared::{McpTestClient, TestServerManager};
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;

/// Start the tasks E2E server and return the manager + initialized client
async fn setup() -> Result<(TestServerManager, McpTestClient), Box<dyn std::error::Error>> {
    let manager = TestServerManager::start("tasks-e2e-inmemory-server").await?;
    let mut client = McpTestClient::new(manager.port());
    client.initialize().await?;
    client.send_initialized_notification().await?;
    Ok((manager, client))
}

#[tokio::test]
async fn test_task_augmented_call_returns_create_task_result() {
    let Ok((_manager, client)) = setup().await else {
        eprintln!("Skipping test: could not start tasks-e2e-inmemory-server");
        return;
    };

    // Call slow_add with task augmentation (delay_ms=500 for fast test)
    let response = client
        .make_request(
            "tools/call",
            json!({
                "name": "slow_add",
                "arguments": {"a": 5, "b": 3, "delay_ms": 500},
                "task": {"ttl": 60000}
            }),
            10,
        )
        .await
        .unwrap();

    let result = response
        .get("result")
        .expect("expected result field in response");

    // CreateTaskResult has a "task" field (not "content")
    let task = result
        .get("task")
        .expect("expected 'task' field in CreateTaskResult");
    let task_id = task
        .get("taskId")
        .expect("expected taskId")
        .as_str()
        .unwrap();
    let status = task
        .get("status")
        .expect("expected status")
        .as_str()
        .unwrap();

    assert!(!task_id.is_empty(), "taskId should not be empty");
    assert_eq!(status, "working", "initial status should be 'working'");

    // Verify created_at and lastUpdatedAt are present
    assert!(
        task.get("createdAt").is_some(),
        "createdAt should be present"
    );
    assert!(
        task.get("lastUpdatedAt").is_some(),
        "lastUpdatedAt should be present"
    );
}

#[tokio::test]
async fn test_task_polling_and_completion() {
    let Ok((_manager, client)) = setup().await else {
        eprintln!("Skipping test: could not start tasks-e2e-inmemory-server");
        return;
    };

    // Create a task with short delay
    let response = client
        .make_request(
            "tools/call",
            json!({
                "name": "slow_add",
                "arguments": {"a": 10, "b": 20, "delay_ms": 300},
                "task": {"ttl": 60000}
            }),
            11,
        )
        .await
        .unwrap();

    let task_id = response["result"]["task"]["taskId"]
        .as_str()
        .unwrap()
        .to_string();

    // Poll tasks/get until completed (max 10 attempts)
    let mut final_status = String::new();
    for _ in 0..20 {
        sleep(Duration::from_millis(200)).await;
        let get_response = client
            .make_request("tasks/get", json!({"taskId": task_id}), 12)
            .await
            .unwrap();

        let status = get_response["result"]["status"]
            .as_str()
            .unwrap_or("unknown");
        final_status = status.to_string();

        if status == "completed" || status == "failed" || status == "cancelled" {
            break;
        }
    }

    assert_eq!(
        final_status, "completed",
        "task should reach completed status"
    );
}

#[tokio::test]
async fn test_tasks_result_returns_tool_output() {
    let Ok((_manager, client)) = setup().await else {
        eprintln!("Skipping test: could not start tasks-e2e-inmemory-server");
        return;
    };

    // Create a task
    let response = client
        .make_request(
            "tools/call",
            json!({
                "name": "slow_add",
                "arguments": {"a": 7, "b": 8, "delay_ms": 300},
                "task": {"ttl": 60000}
            }),
            13,
        )
        .await
        .unwrap();

    let task_id = response["result"]["task"]["taskId"]
        .as_str()
        .unwrap()
        .to_string();

    // Wait for completion
    for _ in 0..20 {
        sleep(Duration::from_millis(200)).await;
        let get_response = client
            .make_request("tasks/get", json!({"taskId": task_id}), 14)
            .await
            .unwrap();
        if get_response["result"]["status"].as_str() == Some("completed") {
            break;
        }
    }

    // Get the result via tasks/result
    let result_response = client
        .make_request("tasks/result", json!({"taskId": task_id}), 15)
        .await
        .unwrap();

    // The result should be the original CallToolResult shape
    let result = &result_response["result"];
    assert!(
        result.get("content").is_some(),
        "tasks/result should return CallToolResult with 'content' field, got: {}",
        result
    );

    // The content should contain the sum (7 + 8 = 15)
    let content = &result["content"];
    let content_str = serde_json::to_string(content).unwrap();
    assert!(
        content_str.contains("15"),
        "expected sum 15 in content, got: {}",
        content_str
    );
}

#[tokio::test]
async fn test_tasks_list() {
    let Ok((_manager, client)) = setup().await else {
        eprintln!("Skipping test: could not start tasks-e2e-inmemory-server");
        return;
    };

    // Create a task
    let response = client
        .make_request(
            "tools/call",
            json!({
                "name": "slow_add",
                "arguments": {"a": 1, "b": 2, "delay_ms": 200},
                "task": {"ttl": 60000}
            }),
            16,
        )
        .await
        .unwrap();

    assert!(
        response["result"]["task"]["taskId"].is_string(),
        "should have created a task"
    );

    // List tasks
    let list_response = client
        .make_request("tasks/list", json!({}), 17)
        .await
        .unwrap();

    let tasks = list_response["result"]["tasks"]
        .as_array()
        .expect("tasks should be an array");
    assert!(
        !tasks.is_empty(),
        "tasks/list should return at least 1 task"
    );
}

#[tokio::test]
async fn test_task_cancellation() {
    let Ok((_manager, client)) = setup().await else {
        eprintln!("Skipping test: could not start tasks-e2e-inmemory-server");
        return;
    };

    // Create a long-running task that we'll cancel
    let response = client
        .make_request(
            "tools/call",
            json!({
                "name": "slow_cancelable",
                "arguments": {"duration_ms": 30000},
                "task": {"ttl": 60000}
            }),
            18,
        )
        .await
        .unwrap();

    let task_id = response["result"]["task"]["taskId"]
        .as_str()
        .unwrap()
        .to_string();

    // Wait a moment for the task to start executing
    sleep(Duration::from_millis(200)).await;

    // Cancel the task
    let cancel_response = client
        .make_request("tasks/cancel", json!({"taskId": task_id}), 19)
        .await
        .unwrap();

    let cancelled_status = cancel_response["result"]["status"]
        .as_str()
        .unwrap_or("unknown");
    assert_eq!(
        cancelled_status, "cancelled",
        "tasks/cancel should transition to cancelled"
    );
}

#[tokio::test]
async fn test_synchronous_call_without_task() {
    let Ok((_manager, client)) = setup().await else {
        eprintln!("Skipping test: could not start tasks-e2e-inmemory-server");
        return;
    };

    // Call without task field — should execute synchronously
    let response = client
        .call_tool("slow_add", json!({"a": 1, "b": 2, "delay_ms": 100}))
        .await
        .unwrap();

    let result = &response["result"];

    // Should be a standard CallToolResult (has "content", no "task")
    assert!(
        result.get("content").is_some(),
        "synchronous call should return CallToolResult with 'content'"
    );
    assert!(
        result.get("task").is_none(),
        "synchronous call should NOT have 'task' field"
    );
}

#[tokio::test]
async fn test_capabilities_advertise_task_support() {
    let Ok((_manager, _client)) = setup().await else {
        eprintln!("Skipping test: could not start tasks-e2e-inmemory-server");
        return;
    };

    // Re-initialize to capture capabilities
    let mut fresh_client = McpTestClient::new(_manager.port());
    let init_response = fresh_client.initialize().await.unwrap();

    let caps = &init_response["result"]["capabilities"];

    // Verify tasks capabilities
    let tasks = &caps["tasks"];
    assert!(
        tasks.get("list").is_some(),
        "capabilities should advertise tasks.list"
    );
    assert!(
        tasks.get("cancel").is_some(),
        "capabilities should advertise tasks.cancel"
    );

    // Verify task-augmented request support
    let requests = &tasks["requests"];
    assert!(
        requests.get("tools").is_some(),
        "capabilities should advertise tasks.requests.tools"
    );
    assert!(
        requests["tools"].get("call").is_some(),
        "capabilities should advertise tasks.requests.tools.call"
    );
}
