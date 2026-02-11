//! Task Handlers — MCP request handlers for tasks/get, tasks/list, tasks/cancel, tasks/result.
//!
//! These handlers implement the `McpHandler` trait and delegate to `TaskRuntime`
//! for storage + runtime coordination.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};
use tracing::{debug, warn};

use crate::handlers::McpHandler;
use crate::session::SessionContext;
use crate::task_runtime::TaskRuntime;
use turul_mcp_protocol::McpError;
use turul_mcp_protocol::tasks::*;

/// Fetch a task and validate session ownership.
///
/// Returns `McpError::InvalidParameters("Task not found")` if:
/// - The task doesn't exist, OR
/// - The task belongs to a different session (prevents cross-session access)
///
/// When `session` is `None` (sessionless mode), all tasks are accessible.
async fn get_task_with_session_check(
    runtime: &TaskRuntime,
    task_id: &str,
    session: &Option<SessionContext>,
) -> crate::McpResult<turul_mcp_task_storage::TaskRecord> {
    let task_record = runtime
        .get_task(task_id)
        .await
        .map_err(|e| McpError::ToolExecutionError(e.to_string()))?
        .ok_or_else(|| McpError::InvalidParameters(format!("Task not found: {}", task_id)))?;

    // If session context is provided, enforce isolation
    if let Some(session_ctx) = session {
        let session_id_str = session_ctx.session_id.to_string();
        if let Some(ref task_session) = task_record.session_id {
            if task_session != &session_id_str {
                // Don't leak that the task exists — return "not found"
                return Err(McpError::InvalidParameters(format!(
                    "Task not found: {}",
                    task_id
                )));
            }
        }
    }

    Ok(task_record)
}

// === tasks/get ===

/// Handler for `tasks/get` — retrieves a task's current status.
pub struct TasksGetHandler {
    runtime: Arc<TaskRuntime>,
}

impl TasksGetHandler {
    pub fn new(runtime: Arc<TaskRuntime>) -> Self {
        Self { runtime }
    }
}

#[async_trait]
impl McpHandler for TasksGetHandler {
    async fn handle(&self, params: Option<Value>) -> crate::McpResult<Value> {
        self.handle_with_session(params, None).await
    }

    async fn handle_with_session(
        &self,
        params: Option<Value>,
        session: Option<SessionContext>,
    ) -> crate::McpResult<Value> {
        let params = params.ok_or_else(|| McpError::missing_param("GetTaskParams"))?;
        let get_params: GetTaskParams = serde_json::from_value(params)?;

        debug!(task_id = %get_params.task_id, "tasks/get request");

        let task_record =
            get_task_with_session_check(&self.runtime, &get_params.task_id, &session).await?;

        let result = GetTaskResult::new(task_record.to_protocol_task());
        serde_json::to_value(result).map_err(McpError::from)
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["tasks/get".to_string()]
    }
}

// === tasks/list ===

/// Handler for `tasks/list` — lists tasks with cursor-based pagination.
pub struct TasksListHandler {
    runtime: Arc<TaskRuntime>,
}

impl TasksListHandler {
    pub fn new(runtime: Arc<TaskRuntime>) -> Self {
        Self { runtime }
    }
}

#[async_trait]
impl McpHandler for TasksListHandler {
    async fn handle(&self, params: Option<Value>) -> crate::McpResult<Value> {
        self.handle_with_session(params, None).await
    }

    async fn handle_with_session(
        &self,
        params: Option<Value>,
        session: Option<SessionContext>,
    ) -> crate::McpResult<Value> {
        let list_params = if let Some(params_value) = params {
            serde_json::from_value::<ListTasksParams>(params_value).map_err(|e| {
                McpError::InvalidParameters(format!("Invalid parameters for tasks/list: {}", e))
            })?
        } else {
            ListTasksParams::new()
        };

        let cursor_str = list_params.cursor.as_ref().map(|c| c.as_str().to_string());
        let cursor_ref = cursor_str.as_deref();

        debug!(
            cursor = ?cursor_ref,
            limit = ?list_params.limit,
            "tasks/list request"
        );

        // If we have a session, scope to that session's tasks
        let page = if let Some(session_ctx) = &session {
            self.runtime
                .list_tasks_for_session(
                    &session_ctx.session_id.to_string(),
                    cursor_ref,
                    list_params.limit,
                )
                .await
        } else {
            self.runtime.list_tasks(cursor_ref, list_params.limit).await
        }
        .map_err(|e| McpError::ToolExecutionError(e.to_string()))?;

        let tasks: Vec<Task> = page.tasks.iter().map(|r| r.to_protocol_task()).collect();
        let mut result = ListTasksResult::new(tasks);

        if let Some(next_cursor) = page.next_cursor {
            result = result.with_next_cursor(turul_mcp_protocol::meta::Cursor::new(&next_cursor));
        }

        serde_json::to_value(result).map_err(McpError::from)
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["tasks/list".to_string()]
    }
}

// === tasks/cancel ===

/// Handler for `tasks/cancel` — cancels an in-flight task.
pub struct TasksCancelHandler {
    runtime: Arc<TaskRuntime>,
}

impl TasksCancelHandler {
    pub fn new(runtime: Arc<TaskRuntime>) -> Self {
        Self { runtime }
    }
}

#[async_trait]
impl McpHandler for TasksCancelHandler {
    async fn handle(&self, params: Option<Value>) -> crate::McpResult<Value> {
        self.handle_with_session(params, None).await
    }

    async fn handle_with_session(
        &self,
        params: Option<Value>,
        session: Option<SessionContext>,
    ) -> crate::McpResult<Value> {
        let params = params.ok_or_else(|| McpError::missing_param("CancelTaskParams"))?;
        let cancel_params: CancelTaskParams = serde_json::from_value(params)?;

        debug!(task_id = %cancel_params.task_id, "tasks/cancel request");

        // Verify session ownership before allowing cancel
        get_task_with_session_check(&self.runtime, &cancel_params.task_id, &session).await?;

        let task_record = self
            .runtime
            .cancel_task(&cancel_params.task_id)
            .await
            .map_err(|e| match e {
                turul_mcp_task_storage::TaskStorageError::TaskNotFound(id) => {
                    McpError::InvalidParameters(format!("Task not found: {}", id))
                }
                turul_mcp_task_storage::TaskStorageError::TerminalState(status) => {
                    McpError::InvalidParameters(format!(
                        "Task is already in terminal state: {:?}",
                        status
                    ))
                }
                turul_mcp_task_storage::TaskStorageError::InvalidTransition {
                    current,
                    requested,
                } => McpError::InvalidParameters(format!(
                    "Cannot cancel task: invalid transition {:?} -> {:?}",
                    current, requested
                )),
                other => McpError::ToolExecutionError(other.to_string()),
            })?;

        let result = CancelTaskResult::new(task_record.to_protocol_task());
        serde_json::to_value(result).map_err(McpError::from)
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["tasks/cancel".to_string()]
    }
}

// === tasks/result ===

/// Handler for `tasks/result` — retrieves the original operation's result.
///
/// Per MCP spec: "When a receiver receives a `tasks/result` request for a task in any
/// other non-terminal status (`working` or `input_required`), it **MUST** block the
/// response until the task reaches a terminal status."
pub struct TasksResultHandler {
    runtime: Arc<TaskRuntime>,
}

impl TasksResultHandler {
    pub fn new(runtime: Arc<TaskRuntime>) -> Self {
        Self { runtime }
    }
}

#[async_trait]
impl McpHandler for TasksResultHandler {
    async fn handle(&self, params: Option<Value>) -> crate::McpResult<Value> {
        self.handle_with_session(params, None).await
    }

    async fn handle_with_session(
        &self,
        params: Option<Value>,
        session: Option<SessionContext>,
    ) -> crate::McpResult<Value> {
        let params = params.ok_or_else(|| McpError::missing_param("GetTaskPayloadParams"))?;
        let payload_params: GetTaskPayloadParams = serde_json::from_value(params)?;

        debug!(task_id = %payload_params.task_id, "tasks/result request");

        // Check current task status (with session isolation)
        let task =
            get_task_with_session_check(&self.runtime, &payload_params.task_id, &session).await?;

        // If not terminal, block until it reaches terminal state
        if !turul_mcp_task_storage::is_terminal(task.status) {
            debug!(
                task_id = %payload_params.task_id,
                status = ?task.status,
                "Task not terminal, blocking until completion"
            );

            // Block until terminal via executor abstraction
            if let Some(terminal_status) =
                self.runtime.await_terminal(&payload_params.task_id).await
            {
                debug!(
                    task_id = %payload_params.task_id,
                    status = ?terminal_status,
                    "Task reached terminal status"
                );
            } else {
                // No executor entry — task may have completed between our check and await,
                // or may be running on a different executor (e.g., after restart).
                // Fall back to polling storage until terminal (spec: MUST block).
                //
                // Note: The spec says "MUST block until terminal status." We impose a
                // safety timeout to prevent indefinite resource consumption when a task
                // is stuck without an executor (e.g., orphaned after crash). The
                // recover_stuck_tasks() mechanism on startup should prevent this case,
                // but the timeout provides defense-in-depth.
                let poll_interval = std::time::Duration::from_millis(500);
                let max_wait = std::time::Duration::from_secs(300); // 5 minutes
                let start = std::time::Instant::now();

                loop {
                    let refreshed = self
                        .runtime
                        .get_task(&payload_params.task_id)
                        .await
                        .map_err(|e| McpError::ToolExecutionError(e.to_string()))?
                        .ok_or_else(|| {
                            McpError::InvalidParameters(format!(
                                "Task not found: {}",
                                payload_params.task_id
                            ))
                        })?;

                    if turul_mcp_task_storage::is_terminal(refreshed.status) {
                        debug!(
                            task_id = %payload_params.task_id,
                            status = ?refreshed.status,
                            "Task reached terminal status (via storage polling)"
                        );
                        break;
                    }

                    if start.elapsed() > max_wait {
                        return Err(McpError::ToolExecutionError(format!(
                            "Task {} did not reach terminal state within timeout",
                            payload_params.task_id
                        )));
                    }

                    tokio::time::sleep(poll_interval).await;
                }
            }
        }

        // Task is now terminal — retrieve the stored outcome
        let outcome = self
            .runtime
            .get_task_result(&payload_params.task_id)
            .await
            .map_err(|e| McpError::ToolExecutionError(e.to_string()))?;

        match outcome {
            Some(turul_mcp_task_storage::TaskOutcome::Success(mut value)) => {
                // Inject _meta.io.modelcontextprotocol/related-task per spec.
                // Only inject if the result is an object (all standard MCP results are).
                // Non-object results are returned verbatim without _meta injection.
                if let Some(obj) = value.as_object_mut() {
                    let meta = obj.entry("_meta").or_insert_with(|| json!({}));
                    if let Some(meta_obj) = meta.as_object_mut() {
                        meta_obj.insert(
                            "io.modelcontextprotocol/related-task".to_string(),
                            json!({ "taskId": payload_params.task_id }),
                        );
                    }
                }

                Ok(value)
            }
            Some(turul_mcp_task_storage::TaskOutcome::Error {
                code,
                message,
                data,
            }) => {
                // Return as JSON-RPC error with original code/message/data preserved.
                // Spec: "tasks/result MUST return that same JSON-RPC error."
                Err(McpError::json_rpc_error(code, message, data))
            }
            None => {
                // Terminal but no result stored — shouldn't happen, but handle gracefully
                warn!(
                    task_id = %payload_params.task_id,
                    "Task is terminal but has no stored result"
                );
                Err(McpError::ToolExecutionError(format!(
                    "Task {} completed but no result was stored",
                    payload_params.task_id
                )))
            }
        }
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["tasks/result".to_string()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task_runtime::TaskRuntime;
    use turul_mcp_protocol::TaskStatus;
    use turul_mcp_task_storage::{InMemoryTaskStorage, TaskOutcome, TaskRecord};

    fn create_test_runtime() -> Arc<TaskRuntime> {
        Arc::new(TaskRuntime::in_memory())
    }

    async fn create_test_task(runtime: &TaskRuntime) -> TaskRecord {
        let record = TaskRecord {
            task_id: InMemoryTaskStorage::generate_task_id(),
            session_id: Some("test-session".to_string()),
            status: TaskStatus::Working,
            status_message: Some("Processing...".to_string()),
            created_at: chrono::Utc::now().to_rfc3339(),
            last_updated_at: chrono::Utc::now().to_rfc3339(),
            ttl: Some(60_000),
            poll_interval: Some(5_000),
            original_method: "tools/call".to_string(),
            original_params: None,
            result: None,
            meta: None,
        };
        runtime.register_task(record).await.unwrap()
    }

    #[tokio::test]
    async fn test_tasks_get_handler() {
        let runtime = create_test_runtime();
        let handler = TasksGetHandler::new(Arc::clone(&runtime));
        let task = create_test_task(&runtime).await;

        let params = serde_json::json!({ "taskId": task.task_id });
        let result = handler.handle(Some(params)).await.unwrap();

        // Result should be flattened GetTaskResult
        assert_eq!(result["taskId"], task.task_id);
        assert_eq!(result["status"], "working");
    }

    #[tokio::test]
    async fn test_tasks_get_handler_not_found() {
        let runtime = create_test_runtime();
        let handler = TasksGetHandler::new(Arc::clone(&runtime));

        let params = serde_json::json!({ "taskId": "nonexistent" });
        let result = handler.handle(Some(params)).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_tasks_list_handler() {
        let runtime = create_test_runtime();
        let handler = TasksListHandler::new(Arc::clone(&runtime));

        // Create a couple of tasks
        let _task1 = create_test_task(&runtime).await;
        let _task2 = create_test_task(&runtime).await;

        let result = handler.handle(None).await.unwrap();

        let tasks = result["tasks"].as_array().unwrap();
        assert_eq!(tasks.len(), 2);
    }

    #[tokio::test]
    async fn test_tasks_cancel_handler() {
        let runtime = create_test_runtime();
        let handler = TasksCancelHandler::new(Arc::clone(&runtime));
        let task = create_test_task(&runtime).await;

        let params = serde_json::json!({ "taskId": task.task_id });
        let result = handler.handle(Some(params)).await.unwrap();

        // Result should show cancelled status (flattened)
        assert_eq!(result["taskId"], task.task_id);
        assert_eq!(result["status"], "cancelled");
    }

    #[tokio::test]
    async fn test_tasks_cancel_handler_already_terminal() {
        let runtime = create_test_runtime();
        let handler = TasksCancelHandler::new(Arc::clone(&runtime));
        let task = create_test_task(&runtime).await;

        // Complete the task first
        runtime
            .complete_task(
                &task.task_id,
                TaskOutcome::Success(serde_json::json!({"result": "done"})),
                TaskStatus::Completed,
                Some("Done".to_string()),
            )
            .await
            .unwrap();

        // Try to cancel — should fail
        let params = serde_json::json!({ "taskId": task.task_id });
        let result = handler.handle(Some(params)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_tasks_result_handler_completed_task() {
        let runtime = create_test_runtime();
        let handler = TasksResultHandler::new(Arc::clone(&runtime));
        let task = create_test_task(&runtime).await;

        // Complete the task with a result
        let tool_result = serde_json::json!({
            "content": [{"type": "text", "text": "42"}],
            "isError": false
        });
        runtime
            .complete_task(
                &task.task_id,
                TaskOutcome::Success(tool_result.clone()),
                TaskStatus::Completed,
                Some("Done".to_string()),
            )
            .await
            .unwrap();

        let params = serde_json::json!({ "taskId": task.task_id });
        let result = handler.handle(Some(params)).await.unwrap();

        // Should contain the original result fields
        assert_eq!(result["content"][0]["text"], "42");
        assert_eq!(result["isError"], false);

        // Should have _meta.io.modelcontextprotocol/related-task injected
        let related_task = &result["_meta"]["io.modelcontextprotocol/related-task"];
        assert_eq!(related_task["taskId"], task.task_id);
    }

    #[tokio::test]
    async fn test_tasks_result_handler_failed_task() {
        let runtime = create_test_runtime();
        let handler = TasksResultHandler::new(Arc::clone(&runtime));
        let task = create_test_task(&runtime).await;

        // Fail the task with a specific error code
        let error_data = serde_json::json!({"detail": "division by zero"});
        runtime
            .complete_task(
                &task.task_id,
                TaskOutcome::Error {
                    code: -32010,
                    message: "Tool execution failed".to_string(),
                    data: Some(error_data.clone()),
                },
                TaskStatus::Failed,
                Some("Tool error".to_string()),
            )
            .await
            .unwrap();

        let params = serde_json::json!({ "taskId": task.task_id });
        let result = handler.handle(Some(params)).await;

        // Should return an error with preserved code/message/data
        let err = result.unwrap_err();
        match err {
            McpError::JsonRpcError {
                code,
                message,
                data,
            } => {
                assert_eq!(code, -32010, "Original error code must be preserved");
                assert_eq!(message, "Tool execution failed");
                assert_eq!(
                    data,
                    Some(error_data),
                    "Original error data must be preserved"
                );
            }
            other => panic!("Expected McpError::JsonRpcError, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_tasks_result_handler_blocks_until_completion() {
        let runtime = create_test_runtime();
        let handler = Arc::new(TasksResultHandler::new(Arc::clone(&runtime)));
        let task = create_test_task(&runtime).await;
        let task_id = task.task_id.clone();
        let task_id_for_work = task.task_id.clone();

        // Start task work in the executor so await_terminal works
        runtime
            .executor()
            .start_task(
                &task_id,
                Box::new(move || {
                    Box::pin(async move {
                        // Simulate long-running work — will be completed externally
                        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                        TaskOutcome::Success(serde_json::json!({"answer": 42}))
                    })
                }),
            )
            .await
            .unwrap();

        // Spawn the handler in a separate task — it should block
        let handler_clone = Arc::clone(&handler);
        let tid = task_id.clone();
        let result_handle = tokio::spawn(async move {
            let params = serde_json::json!({ "taskId": tid });
            handler_clone.handle(Some(params)).await
        });

        // Give the handler time to start blocking
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Complete the task — this should unblock the handler
        runtime
            .complete_task(
                &task_id_for_work,
                TaskOutcome::Success(serde_json::json!({"answer": 42})),
                TaskStatus::Completed,
                None,
            )
            .await
            .unwrap();

        // Cancel the executor entry to unblock await_terminal
        let _ = runtime.executor().cancel_task(&task_id_for_work).await;

        // Wait for the handler with timeout
        let result = tokio::time::timeout(std::time::Duration::from_secs(2), result_handle)
            .await
            .expect("Handler should complete within timeout")
            .expect("Handler task should not panic");

        let value = result.unwrap();
        assert_eq!(value["answer"], 42);
        assert!(value["_meta"]["io.modelcontextprotocol/related-task"]["taskId"].is_string());
    }

    #[tokio::test]
    async fn test_session_isolation_tasks_get() {
        let runtime = create_test_runtime();
        let handler = TasksGetHandler::new(Arc::clone(&runtime));
        let task = create_test_task(&runtime).await;

        // Task has session_id "test-session". Create a different session context.
        let other_session = SessionContext::new_test();
        // Sanity: other_session.session_id != "test-session"
        assert_ne!(other_session.session_id, "test-session");

        // Same-session access should work
        let matching_session = SessionContext {
            session_id: "test-session".to_string(),
            ..SessionContext::new_test()
        };
        let params = serde_json::json!({ "taskId": task.task_id });
        let result = handler
            .handle_with_session(Some(params.clone()), Some(matching_session))
            .await;
        assert!(result.is_ok(), "Same-session access should succeed");

        // Cross-session access should fail with "Task not found"
        let result = handler
            .handle_with_session(Some(params.clone()), Some(other_session))
            .await;
        assert!(result.is_err(), "Cross-session access should fail");

        // No session (sessionless mode) should still work
        let result = handler.handle_with_session(Some(params), None).await;
        assert!(result.is_ok(), "Sessionless access should succeed");
    }

    #[tokio::test]
    async fn test_session_isolation_tasks_cancel() {
        let runtime = create_test_runtime();
        let handler = TasksCancelHandler::new(Arc::clone(&runtime));
        let task = create_test_task(&runtime).await;

        let other_session = SessionContext::new_test();
        let params = serde_json::json!({ "taskId": task.task_id });

        // Cross-session cancel should fail
        let result = handler
            .handle_with_session(Some(params), Some(other_session))
            .await;
        assert!(result.is_err(), "Cross-session cancel should fail");
    }

    #[tokio::test]
    async fn test_builder_with_task_storage() {
        use crate::McpServer;

        let server = McpServer::builder()
            .name("task-test-server")
            .with_task_storage(Arc::new(InMemoryTaskStorage::new()))
            .build()
            .unwrap();

        // Verify tasks capability is advertised
        let caps = server.capabilities();
        assert!(caps.tasks.is_some());

        let tasks_caps = caps.tasks.as_ref().unwrap();
        assert!(tasks_caps.list.is_some());
        assert!(tasks_caps.cancel.is_some());
        // Task-augmented tools/call is advertised when task runtime is configured
        assert!(tasks_caps.requests.is_some());

        // Verify task runtime is available
        assert!(server.task_runtime().is_some());
    }

    #[tokio::test]
    async fn test_builder_without_task_storage() {
        use crate::McpServer;

        let server = McpServer::builder().name("no-task-server").build().unwrap();

        // Verify tasks capability is NOT advertised
        let caps = server.capabilities();
        assert!(caps.tasks.is_none());

        // Verify no task runtime
        assert!(server.task_runtime().is_none());
    }

    #[tokio::test]
    async fn test_builder_with_tasks_and_tools() {
        use crate::McpServer;
        use turul_mcp_builders::prelude::*;

        // Create a minimal test tool
        struct DummyTool;
        impl HasBaseMetadata for DummyTool {
            fn name(&self) -> &str {
                "dummy"
            }
            fn title(&self) -> Option<&str> {
                None
            }
        }
        impl HasDescription for DummyTool {
            fn description(&self) -> Option<&str> {
                Some("test")
            }
        }
        impl HasInputSchema for DummyTool {
            fn input_schema(&self) -> &turul_mcp_protocol::ToolSchema {
                static SCHEMA: std::sync::OnceLock<turul_mcp_protocol::ToolSchema> =
                    std::sync::OnceLock::new();
                SCHEMA.get_or_init(|| turul_mcp_protocol::ToolSchema::object())
            }
        }
        impl HasOutputSchema for DummyTool {
            fn output_schema(&self) -> Option<&turul_mcp_protocol::ToolSchema> {
                None
            }
        }
        impl HasAnnotations for DummyTool {
            fn annotations(&self) -> Option<&turul_mcp_protocol::tools::ToolAnnotations> {
                None
            }
        }
        impl HasToolMeta for DummyTool {
            fn tool_meta(&self) -> Option<&std::collections::HashMap<String, serde_json::Value>> {
                None
            }
        }
        impl HasIcons for DummyTool {}

        #[async_trait::async_trait]
        impl crate::McpTool for DummyTool {
            async fn call(
                &self,
                _args: serde_json::Value,
                _session: Option<crate::SessionContext>,
            ) -> crate::McpResult<turul_mcp_protocol::tools::CallToolResult> {
                Ok(turul_mcp_protocol::tools::CallToolResult::success(vec![
                    turul_mcp_protocol::ToolResult::text("done"),
                ]))
            }
        }

        let server = McpServer::builder()
            .name("task-tool-server")
            .tool(DummyTool)
            .with_task_storage(Arc::new(InMemoryTaskStorage::new()))
            .build()
            .unwrap();

        let caps = server.capabilities();
        let tasks_caps = caps.tasks.as_ref().unwrap();
        // tasks.list, tasks.cancel, and task-augmented requests are all supported
        assert!(tasks_caps.list.is_some());
        assert!(tasks_caps.cancel.is_some());
        assert!(tasks_caps.requests.is_some());
        let requests = tasks_caps.requests.as_ref().unwrap();
        assert!(requests.tools.is_some());
        assert!(requests.tools.as_ref().unwrap().call.is_some());
    }
}
