//! MCP 2025-11-25 Task Types Showcase
//!
//! Demonstrates the task system types for tracking long-running operations:
//! Task, TaskStatus, TaskMetadata, and the CRUD request/result types
//! (get, cancel, list — no tasks/create method exists).
//!
//! Tasks are created implicitly when requests include `task: TaskMetadata`
//! in their params (e.g., CallToolParams, CreateMessageParams).

use turul_mcp_protocol::{
    CancelTaskRequest, CancelTaskResult, GetTaskRequest, GetTaskResult, ListTasksRequest,
    ListTasksResult, Task, TaskMetadata, TaskStatus, meta::Cursor,
};

fn main() {
    println!("=== MCP 2025-11-25 Task Types Showcase ===\n");

    // --- TaskStatus values ---
    println!("1. TaskStatus serialization (snake_case strings):");
    let statuses = [
        TaskStatus::Working,
        TaskStatus::InputRequired,
        TaskStatus::Completed,
        TaskStatus::Failed,
        TaskStatus::Cancelled,
    ];
    for status in &statuses {
        println!(
            "   {:?} -> {}",
            status,
            serde_json::to_string(status).unwrap()
        );
    }
    println!();

    // --- TaskMetadata (used in task-augmented requests) ---
    println!("2. TaskMetadata (attached to request params to create tasks):");
    let metadata = TaskMetadata::new().with_ttl(30000);
    println!("{}\n", serde_json::to_string_pretty(&metadata).unwrap());

    // --- Task: a working task ---
    let now = "2025-01-15T10:00:00Z";
    let working_task = Task::new("task-abc-123", TaskStatus::Working, now, now)
        .with_status_message("Processing batch 42 of 100")
        .with_ttl(60000)
        .with_poll_interval(5000);

    println!("3. Task (working with status message):");
    println!("{}\n", serde_json::to_string_pretty(&working_task).unwrap());

    // --- Task: a completed task ---
    let completed_task = Task::new(
        "task-def-456",
        TaskStatus::Completed,
        "2025-01-15T09:00:00Z",
        "2025-01-15T09:30:00Z",
    )
    .with_status_message("All records processed successfully");

    println!("4. Task (completed):");
    println!(
        "{}\n",
        serde_json::to_string_pretty(&completed_task).unwrap()
    );

    // --- Task: a failed task ---
    let failed_task = Task::new(
        "task-ghi-789",
        TaskStatus::Failed,
        "2025-01-15T08:00:00Z",
        "2025-01-15T08:05:00Z",
    )
    .with_status_message("Connection timeout after 30s");

    println!("5. Task (failed):");
    println!("{}\n", serde_json::to_string_pretty(&failed_task).unwrap());

    // --- Task: input required ---
    let input_task = Task::new(
        "task-jkl-012",
        TaskStatus::InputRequired,
        "2025-01-15T10:00:00Z",
        "2025-01-15T10:02:00Z",
    )
    .with_status_message("Please confirm the deletion of 50 records");

    println!("6. Task (input required):");
    println!("{}\n", serde_json::to_string_pretty(&input_task).unwrap());

    // --- tasks/get request and result ---
    let get_request = GetTaskRequest::new("task-abc-123");

    println!("7. tasks/get request:");
    println!("{}\n", serde_json::to_string_pretty(&get_request).unwrap());

    let get_result = GetTaskResult::new(
        Task::new(
            "task-abc-123",
            TaskStatus::Working,
            "2025-01-15T10:00:00Z",
            "2025-01-15T10:05:00Z",
        )
        .with_status_message("Processing batch 75 of 100"),
    );

    println!("8. tasks/get result (in-progress update):");
    println!("{}\n", serde_json::to_string_pretty(&get_result).unwrap());

    // --- tasks/cancel request and result ---
    let cancel_request = CancelTaskRequest::new("task-abc-123");

    println!("9. tasks/cancel request:");
    println!(
        "{}\n",
        serde_json::to_string_pretty(&cancel_request).unwrap()
    );

    let cancel_result = CancelTaskResult::new(
        Task::new(
            "task-abc-123",
            TaskStatus::Cancelled,
            "2025-01-15T10:00:00Z",
            "2025-01-15T10:06:00Z",
        )
        .with_status_message("Cancelled by user"),
    );

    println!("10. tasks/cancel result:");
    println!(
        "{}\n",
        serde_json::to_string_pretty(&cancel_result).unwrap()
    );

    // --- tasks/list request and result ---
    let list_request = ListTasksRequest::new().with_limit(10);

    println!("11. tasks/list request (with limit):");
    println!("{}\n", serde_json::to_string_pretty(&list_request).unwrap());

    let list_result = ListTasksResult::new(vec![
        Task::new(
            "task-001",
            TaskStatus::Working,
            "2025-01-15T10:00:00Z",
            "2025-01-15T10:05:00Z",
        )
        .with_status_message("Indexing documents"),
        Task::new(
            "task-002",
            TaskStatus::Completed,
            "2025-01-15T09:00:00Z",
            "2025-01-15T09:30:00Z",
        )
        .with_status_message("Export finished"),
        Task::new(
            "task-003",
            TaskStatus::Failed,
            "2025-01-15T08:00:00Z",
            "2025-01-15T08:01:00Z",
        )
        .with_status_message("Permission denied"),
    ])
    .with_next_cursor(Cursor::new("page-2"));

    println!("12. tasks/list result (paginated):");
    println!("{}\n", serde_json::to_string_pretty(&list_result).unwrap());

    println!("=== Task lifecycle: Working -> Completed | Failed | Cancelled | InputRequired ===");
    println!("Note: Tasks are created via task-augmented requests (no tasks/create method).");
}
