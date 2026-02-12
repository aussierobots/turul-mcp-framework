//! Shared parity test functions for task storage backends.
//!
//! Each test takes a `&dyn TaskStorage` so the same assertions apply to InMemory,
//! SQLite, PostgreSQL, and DynamoDB backends. Backend-specific test modules call
//! these functions with their own storage instance.
//!
//! This module is `pub(crate)` and only compiled in test builds.

use crate::error::TaskStorageError;
use crate::traits::{TaskOutcome, TaskRecord, TaskStorage};
use serde_json::json;
use std::sync::Arc;
use turul_mcp_protocol::TaskStatus;

/// Helper: create a TaskRecord with given id, session, and timestamp.
pub fn make_task(task_id: &str, session_id: Option<&str>, created_at: &str) -> TaskRecord {
    TaskRecord {
        task_id: task_id.to_string(),
        session_id: session_id.map(|s| s.to_string()),
        status: TaskStatus::Working,
        status_message: None,
        created_at: created_at.to_string(),
        last_updated_at: created_at.to_string(),
        ttl: None,
        poll_interval: None,
        original_method: "tools/call".to_string(),
        original_params: None,
        result: None,
        meta: None,
    }
}

/// Helper: create a TaskRecord with a specific timestamp and all optional fields populated.
pub fn make_full_task(task_id: &str, session_id: &str, created_at: &str) -> TaskRecord {
    use std::collections::HashMap;
    TaskRecord {
        task_id: task_id.to_string(),
        session_id: Some(session_id.to_string()),
        status: TaskStatus::Working,
        status_message: Some("Processing...".to_string()),
        created_at: created_at.to_string(),
        last_updated_at: created_at.to_string(),
        ttl: Some(60_000),
        poll_interval: Some(5_000),
        original_method: "tools/call".to_string(),
        original_params: Some(json!({"tool": "calculator", "args": {"a": 1, "b": 2}})),
        result: None,
        meta: Some(HashMap::from([
            ("key1".to_string(), json!("value1")),
            ("key2".to_string(), json!(42)),
        ])),
    }
}

// ============================================================
// Parity Test Functions
// ============================================================

/// Basic CRUD round-trip: create, get, update, delete.
pub async fn test_create_and_retrieve(storage: &dyn TaskStorage) {
    let task = make_full_task("parity-crud-1", "sess-1", "2025-06-01T00:00:00Z");
    let created = storage.create_task(task.clone()).await.unwrap();
    assert_eq!(created.task_id, "parity-crud-1");
    assert_eq!(created.status, TaskStatus::Working);
    assert_eq!(created.session_id, Some("sess-1".to_string()));
    assert_eq!(created.original_method, "tools/call");
    assert!(created.original_params.is_some());
    assert!(created.meta.is_some());
    assert_eq!(created.ttl, Some(60_000));
    assert_eq!(created.poll_interval, Some(5_000));

    // Get
    let fetched = storage.get_task("parity-crud-1").await.unwrap().unwrap();
    assert_eq!(fetched.task_id, "parity-crud-1");
    assert_eq!(fetched.status, TaskStatus::Working);
    assert_eq!(fetched.session_id, Some("sess-1".to_string()));
    assert_eq!(fetched.original_params, created.original_params);
    assert_eq!(fetched.meta, created.meta);

    // Get nonexistent
    assert!(storage.get_task("nonexistent").await.unwrap().is_none());

    // Delete
    assert!(storage.delete_task("parity-crud-1").await.unwrap());
    assert!(!storage.delete_task("parity-crud-1").await.unwrap()); // Already deleted
    assert!(storage.get_task("parity-crud-1").await.unwrap().is_none());
}

/// All valid state transitions succeed, invalid ones return the correct error variant.
pub async fn test_state_machine_enforcement(storage: &dyn TaskStorage) {
    // Working -> InputRequired
    let t1 = make_task("parity-sm-1", None, "2025-06-01T00:00:00Z");
    storage.create_task(t1).await.unwrap();
    let updated = storage
        .update_task_status(
            "parity-sm-1",
            TaskStatus::InputRequired,
            Some("Need input".to_string()),
        )
        .await
        .unwrap();
    assert_eq!(updated.status, TaskStatus::InputRequired);

    // InputRequired -> Working
    let updated = storage
        .update_task_status(
            "parity-sm-1",
            TaskStatus::Working,
            Some("Resuming".to_string()),
        )
        .await
        .unwrap();
    assert_eq!(updated.status, TaskStatus::Working);

    // Working -> Completed
    let updated = storage
        .update_task_status("parity-sm-1", TaskStatus::Completed, None)
        .await
        .unwrap();
    assert_eq!(updated.status, TaskStatus::Completed);

    // Working -> Working should fail
    let t2 = make_task("parity-sm-2", None, "2025-06-01T00:00:01Z");
    storage.create_task(t2).await.unwrap();
    let err = storage
        .update_task_status("parity-sm-2", TaskStatus::Working, None)
        .await
        .unwrap_err();
    match err {
        TaskStorageError::InvalidTransition { current, requested } => {
            assert_eq!(current, TaskStatus::Working);
            assert_eq!(requested, TaskStatus::Working);
        }
        other => panic!("Expected InvalidTransition, got: {:?}", other),
    }

    // Nonexistent task
    let err = storage
        .update_task_status("nonexistent", TaskStatus::Completed, None)
        .await
        .unwrap_err();
    match err {
        TaskStorageError::TaskNotFound(id) => assert_eq!(id, "nonexistent"),
        other => panic!("Expected TaskNotFound, got: {:?}", other),
    }
}

/// Terminal states (Completed, Failed, Cancelled) reject ALL transitions.
pub async fn test_terminal_state_rejection(storage: &dyn TaskStorage) {
    for (i, terminal) in [
        TaskStatus::Completed,
        TaskStatus::Failed,
        TaskStatus::Cancelled,
    ]
    .iter()
    .enumerate()
    {
        let task_id = format!("parity-term-{}", i);
        let task = make_task(&task_id, None, &format!("2025-06-01T00:00:0{}Z", i));
        storage.create_task(task).await.unwrap();

        // Move to terminal state
        storage
            .update_task_status(&task_id, *terminal, None)
            .await
            .unwrap();

        // Try every possible transition — all should fail with TerminalState
        for target in [
            TaskStatus::Working,
            TaskStatus::InputRequired,
            TaskStatus::Completed,
            TaskStatus::Failed,
            TaskStatus::Cancelled,
        ] {
            let err = storage
                .update_task_status(&task_id, target, None)
                .await
                .unwrap_err();
            match err {
                TaskStorageError::TerminalState(s) => assert_eq!(s, *terminal),
                other => panic!(
                    "Expected TerminalState({:?}) for {:?} -> {:?}, got: {:?}",
                    terminal, terminal, target, other
                ),
            }
        }
    }
}

/// 10 tasks with the same `created_at` — pages return deterministic order by (created_at, task_id).
///
/// **DynamoDB note**: This test is valid for `list_tasks_for_session` on all backends.
/// For global `list_tasks`, DynamoDB is best-effort — call this test with the appropriate
/// list function.
pub async fn test_cursor_determinism(storage: &dyn TaskStorage) {
    // Create 10 tasks with SAME created_at, different task_ids
    let ts = "2025-06-01T12:00:00Z";
    let mut ids: Vec<String> = (0..10).map(|i| format!("parity-cursor-{:02}", i)).collect();

    for id in &ids {
        let task = make_task(id, Some("cursor-session"), ts);
        storage.create_task(task).await.unwrap();
    }

    // Expected order: sorted by (created_at, task_id) — since created_at is the same,
    // it's just alphabetical task_id
    ids.sort();

    // Page through with limit=3
    let mut collected = Vec::new();
    let mut cursor: Option<String> = None;

    loop {
        let page = storage
            .list_tasks_for_session("cursor-session", cursor.as_deref(), Some(3))
            .await
            .unwrap();

        for task in &page.tasks {
            collected.push(task.task_id.clone());
        }

        if page.next_cursor.is_none() {
            break;
        }
        cursor = page.next_cursor;
    }

    assert_eq!(
        collected, ids,
        "Tasks should be in deterministic (created_at, task_id) order"
    );
}

/// Tasks from session A never appear in session B listing.
pub async fn test_session_scoping(storage: &dyn TaskStorage) {
    let t1 = make_task(
        "parity-scope-a1",
        Some("scope-sess-A"),
        "2025-06-01T00:00:00Z",
    );
    let t2 = make_task(
        "parity-scope-a2",
        Some("scope-sess-A"),
        "2025-06-01T00:00:01Z",
    );
    let t3 = make_task(
        "parity-scope-b1",
        Some("scope-sess-B"),
        "2025-06-01T00:00:02Z",
    );

    storage.create_task(t1).await.unwrap();
    storage.create_task(t2).await.unwrap();
    storage.create_task(t3).await.unwrap();

    let page_a = storage
        .list_tasks_for_session("scope-sess-A", None, None)
        .await
        .unwrap();
    assert_eq!(page_a.tasks.len(), 2);
    assert!(
        page_a
            .tasks
            .iter()
            .all(|t| t.session_id.as_deref() == Some("scope-sess-A"))
    );

    let page_b = storage
        .list_tasks_for_session("scope-sess-B", None, None)
        .await
        .unwrap();
    assert_eq!(page_b.tasks.len(), 1);
    assert_eq!(page_b.tasks[0].task_id, "parity-scope-b1");

    let page_empty = storage
        .list_tasks_for_session("scope-sess-C", None, None)
        .await
        .unwrap();
    assert_eq!(page_empty.tasks.len(), 0);
}

/// Task with expired TTL is removed by `expire_tasks()`, non-TTL tasks survive.
pub async fn test_ttl_expiry(storage: &dyn TaskStorage) {
    // Task with very short TTL and old timestamp (already expired)
    let mut expired_task = make_task("parity-ttl-expired", None, "2020-01-01T00:00:00Z");
    expired_task.ttl = Some(1); // 1ms TTL, created in 2020 = definitely expired
    storage.create_task(expired_task).await.unwrap();

    // Task without TTL (should survive)
    let no_ttl_task = make_task("parity-ttl-keep", None, "2020-01-01T00:00:00Z");
    storage.create_task(no_ttl_task).await.unwrap();

    // Task with TTL but not yet expired
    let mut future_task = make_task("parity-ttl-future", None, "2099-01-01T00:00:00Z");
    future_task.ttl = Some(999_999_999); // Very long TTL
    storage.create_task(future_task).await.unwrap();

    let expired = storage.expire_tasks().await.unwrap();
    assert!(
        expired.contains(&"parity-ttl-expired".to_string()),
        "Expired task should be in the returned list"
    );
    assert!(
        !expired.contains(&"parity-ttl-keep".to_string()),
        "Non-TTL task should NOT be expired"
    );
    assert!(
        !expired.contains(&"parity-ttl-future".to_string()),
        "Future-expiry task should NOT be expired"
    );

    // Verify expired task is gone
    assert!(
        storage
            .get_task("parity-ttl-expired")
            .await
            .unwrap()
            .is_none()
    );
    // Verify others still exist
    assert!(storage.get_task("parity-ttl-keep").await.unwrap().is_some());
    assert!(
        storage
            .get_task("parity-ttl-future")
            .await
            .unwrap()
            .is_some()
    );
}

/// `TaskOutcome::Success` and `TaskOutcome::Error` survive store → retrieve.
pub async fn test_task_result_round_trip(storage: &dyn TaskStorage) {
    // Success outcome
    let t1 = make_task("parity-result-ok", None, "2025-06-01T00:00:00Z");
    storage.create_task(t1).await.unwrap();

    let success = TaskOutcome::Success(json!({
        "content": [{"type": "text", "text": "Result data"}],
        "isError": false,
        "structuredContent": {"value": 42}
    }));
    storage
        .store_task_result("parity-result-ok", success.clone())
        .await
        .unwrap();

    let fetched = storage
        .get_task_result("parity-result-ok")
        .await
        .unwrap()
        .unwrap();
    match fetched {
        TaskOutcome::Success(v) => {
            assert_eq!(v["content"][0]["text"], "Result data");
            assert_eq!(v["structuredContent"]["value"], 42);
        }
        other => panic!("Expected Success, got: {:?}", other),
    }

    // Error outcome
    let t2 = make_task("parity-result-err", None, "2025-06-01T00:00:01Z");
    storage.create_task(t2).await.unwrap();

    let error_outcome = TaskOutcome::Error {
        code: -32010,
        message: "Tool execution failed".to_string(),
        data: Some(json!({"detail": "division by zero"})),
    };
    storage
        .store_task_result("parity-result-err", error_outcome)
        .await
        .unwrap();

    let fetched = storage
        .get_task_result("parity-result-err")
        .await
        .unwrap()
        .unwrap();
    match fetched {
        TaskOutcome::Error {
            code,
            message,
            data,
        } => {
            assert_eq!(code, -32010);
            assert_eq!(message, "Tool execution failed");
            assert_eq!(data.unwrap()["detail"], "division by zero");
        }
        other => panic!("Expected Error, got: {:?}", other),
    }

    // Nonexistent task result should error
    let err = storage.get_task_result("nonexistent").await.unwrap_err();
    match err {
        TaskStorageError::TaskNotFound(id) => assert_eq!(id, "nonexistent"),
        other => panic!("Expected TaskNotFound, got: {:?}", other),
    }
}

/// Old non-terminal tasks become Failed; recent and terminal tasks are untouched.
pub async fn test_recover_stuck_tasks(storage: &dyn TaskStorage) {
    // Old "stuck" working task
    let mut stuck = make_task("parity-stuck-1", None, "2020-01-01T00:00:00Z");
    stuck.last_updated_at = "2020-01-01T00:00:00Z".to_string();
    storage.create_task(stuck).await.unwrap();

    // Old "stuck" input_required task
    let mut stuck_ir = make_task("parity-stuck-2", None, "2020-01-01T00:00:01Z");
    stuck_ir.status = TaskStatus::InputRequired;
    stuck_ir.last_updated_at = "2020-01-01T00:00:01Z".to_string();
    storage.create_task(stuck_ir).await.unwrap();

    // Recent working task (should NOT be recovered)
    let recent = make_task("parity-recent", None, "2099-01-01T00:00:00Z");
    storage.create_task(recent).await.unwrap();

    // Old completed task (terminal, should NOT be touched)
    let mut completed = make_task("parity-done", None, "2020-01-01T00:00:02Z");
    completed.status = TaskStatus::Completed;
    completed.last_updated_at = "2020-01-01T00:00:02Z".to_string();
    storage.create_task(completed).await.unwrap();

    // Recover with 5-minute threshold
    let recovered = storage.recover_stuck_tasks(300_000).await.unwrap();
    assert!(recovered.contains(&"parity-stuck-1".to_string()));
    assert!(recovered.contains(&"parity-stuck-2".to_string()));
    assert!(!recovered.contains(&"parity-recent".to_string()));
    assert!(!recovered.contains(&"parity-done".to_string()));

    // Verify stuck tasks are now Failed
    let s1 = storage.get_task("parity-stuck-1").await.unwrap().unwrap();
    assert_eq!(s1.status, TaskStatus::Failed);

    let s2 = storage.get_task("parity-stuck-2").await.unwrap().unwrap();
    assert_eq!(s2.status, TaskStatus::Failed);

    // Verify recent task is still Working
    let recent = storage.get_task("parity-recent").await.unwrap().unwrap();
    assert_eq!(recent.status, TaskStatus::Working);

    // Verify completed task is untouched
    let done = storage.get_task("parity-done").await.unwrap().unwrap();
    assert_eq!(done.status, TaskStatus::Completed);
}

/// `MaxTasksReached` error when limit exceeded.
pub async fn test_max_tasks_limit(storage: &dyn TaskStorage, max_tasks: usize) {
    // This test requires a storage instance configured with a low max_tasks.
    // The caller is responsible for providing one with max_tasks set appropriately.

    // Fill to capacity
    for i in 0..max_tasks {
        let task = make_task(
            &format!("parity-max-{}", i),
            None,
            &format!("2025-06-01T00:00:{:02}Z", i),
        );
        storage.create_task(task).await.unwrap();
    }

    // One more should fail
    let overflow = make_task("parity-max-overflow", None, "2025-06-01T00:01:00Z");
    let err = storage.create_task(overflow).await.unwrap_err();
    match err {
        TaskStorageError::MaxTasksReached(n) => assert_eq!(n, max_tasks),
        other => panic!("Expected MaxTasksReached({}), got: {:?}", max_tasks, other),
    }
}

/// `TaskNotFound`, `InvalidTransition`, `TerminalState` errors have consistent variant shapes.
pub async fn test_error_mapping_parity(storage: &dyn TaskStorage) {
    // TaskNotFound
    let err = storage
        .update_task_status("error-parity-missing", TaskStatus::Completed, None)
        .await
        .unwrap_err();
    assert!(
        matches!(err, TaskStorageError::TaskNotFound(ref id) if id == "error-parity-missing"),
        "Expected TaskNotFound('error-parity-missing'), got: {:?}",
        err
    );

    // InvalidTransition
    let task = make_task("error-parity-inv", None, "2025-06-01T00:00:00Z");
    storage.create_task(task).await.unwrap();
    let err = storage
        .update_task_status("error-parity-inv", TaskStatus::Working, None)
        .await
        .unwrap_err();
    assert!(
        matches!(
            err,
            TaskStorageError::InvalidTransition {
                current: TaskStatus::Working,
                requested: TaskStatus::Working
            }
        ),
        "Expected InvalidTransition(Working->Working), got: {:?}",
        err
    );

    // TerminalState
    storage
        .update_task_status("error-parity-inv", TaskStatus::Completed, None)
        .await
        .unwrap();
    let err = storage
        .update_task_status("error-parity-inv", TaskStatus::Working, None)
        .await
        .unwrap_err();
    assert!(
        matches!(err, TaskStorageError::TerminalState(TaskStatus::Completed)),
        "Expected TerminalState(Completed), got: {:?}",
        err
    );
}

/// Two concurrent `Working → Completed` updates: at least one succeeds, at most one fails.
///
/// In practice, serialized backends (InMemory/SQLite) always produce exactly one winner
/// because `RwLock` / connection serialization means the second call sees `Completed` and
/// gets a `TerminalState` error. However, the test does not assume serialization — it
/// accepts both `(Ok, Err)` and `(Ok, Ok)` as valid outcomes, since a backend that
/// processes both transitions before either commits would still be correct.
///
/// Acceptable loser error variants:
/// - `TerminalState` — serialized backends (InMemory/SQLite) where the second call sees Completed
/// - `ConcurrentModification` — optimistic-locking backends (PostgreSQL version column, DynamoDB conditional write)
/// - `InvalidTransition` — if backend sees the same-status transition as invalid
///
/// The test also verifies:
/// - Final persisted status is `Completed`
/// - The winning write's `status_message` persisted ("winner")
/// - No phantom duplicate records (`task_count` unchanged)
///
/// **NOTE**: This function takes `Arc<dyn TaskStorage>` (not `&dyn TaskStorage`) because it
/// needs to move the storage reference into spawned tasks for true concurrency.
pub async fn test_concurrent_status_updates(storage: Arc<dyn TaskStorage>) {
    // 1. Create a task in Working status
    let task = make_task("parity-concurrent-1", None, "2025-06-01T00:00:00Z");
    storage.create_task(task).await.unwrap();

    let initial_count = storage.task_count().await.unwrap();

    // 2. Spawn two concurrent update_task_status calls
    let s1 = Arc::clone(&storage);
    let s2 = Arc::clone(&storage);

    let handle1 = tokio::spawn(async move {
        s1.update_task_status(
            "parity-concurrent-1",
            TaskStatus::Completed,
            Some("winner".to_string()),
        )
        .await
    });

    let handle2 = tokio::spawn(async move {
        s2.update_task_status(
            "parity-concurrent-1",
            TaskStatus::Completed,
            Some("winner".to_string()),
        )
        .await
    });

    let (result1, result2) = tokio::join!(handle1, handle2);
    let result1 = result1.expect("task 1 panicked");
    let result2 = result2.expect("task 2 panicked");

    // 3. At least one succeeds; if there is a loser, its error must be acceptable
    let (winner_count, loser_err) = match (&result1, &result2) {
        (Ok(_), Err(e)) => (1, Some(e)),
        (Err(e), Ok(_)) => (1, Some(e)),
        (Ok(_), Ok(_)) => {
            // Both succeeded — acceptable if the backend does not serialize transitions
            // (e.g., a hypothetical non-locking backend). Current backends (InMemory/SQLite/
            // PostgreSQL/DynamoDB) always serialize, so this branch is not expected in practice.
            (2, None)
        }
        (Err(e1), Err(e2)) => {
            panic!(
                "Both concurrent updates failed — at least one must succeed.\n  err1: {:?}\n  err2: {:?}",
                e1, e2
            );
        }
    };

    // If there was a loser, verify its error variant is acceptable
    if let Some(err) = loser_err {
        let is_acceptable = matches!(
            err,
            TaskStorageError::ConcurrentModification(_)
                | TaskStorageError::InvalidTransition { .. }
                | TaskStorageError::TerminalState(_)
        );
        assert!(
            is_acceptable,
            "Loser error must be ConcurrentModification, InvalidTransition, or TerminalState, got: {:?}",
            err
        );
    }

    assert!(
        winner_count >= 1,
        "At least one concurrent update must succeed"
    );

    // 4. Final persisted status is Completed
    let final_task = storage
        .get_task("parity-concurrent-1")
        .await
        .unwrap()
        .expect("Task should still exist after concurrent updates");
    assert_eq!(
        final_task.status,
        TaskStatus::Completed,
        "Final persisted status must be Completed"
    );

    // 5. Message stability — the winning write's message persisted
    assert_eq!(
        final_task.status_message,
        Some("winner".to_string()),
        "Winning write's status_message must persist"
    );

    // 6. No phantom duplicates — task count unchanged
    let final_count = storage.task_count().await.unwrap();
    assert_eq!(
        final_count, initial_count,
        "No phantom duplicate records should be created by concurrent updates"
    );
}
