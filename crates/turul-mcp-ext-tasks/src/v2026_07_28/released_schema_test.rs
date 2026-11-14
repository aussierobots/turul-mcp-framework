//! The wire types validate against the vendored released schema
//! (`schema/schema.json`, ext-tasks `schema/2026-07-28/` at `0d0a6bd4`).
//!
//! ADR-028 (2026-11-14, e): the dated schema is normative, so every result and
//! notification this crate serializes must validate against its `$defs`.

use serde_json::{Value, json};

use super::lifecycle::{GetTaskResult, TaskAckResult, TaskStatusNotificationParams};
use super::types::{CreateTaskResult, DetailedTask, Nullable, Task, TaskFields, TaskStatus};

const SCHEMA: &str = include_str!("../../schema/schema.json");

fn assert_valid(def: &str, instance: &Value) {
    let mut schema: Value = serde_json::from_str(SCHEMA).expect("schema.json parses");
    schema["$ref"] = json!(format!("#/$defs/{def}"));
    let validator = jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .build(&schema)
        .expect("schema.json compiles");
    let errors: Vec<String> = validator
        .iter_errors(instance)
        .map(|e| e.to_string())
        .collect();
    assert!(errors.is_empty(), "{def} rejects {instance}: {errors:?}");
}

fn fields() -> TaskFields {
    TaskFields {
        task_id: "786512e2-9e0d-44bd-8f29-789f320fe840".to_string(),
        status_message: Some("ok".to_string()),
        created_at: "2026-11-14T00:00:00Z".to_string(),
        last_updated_at: "2026-11-14T00:00:05Z".to_string(),
        ttl_ms: Nullable(Some(60000.0)),
        poll_interval_ms: Some(500.0),
    }
}

fn every_detailed_task() -> Vec<DetailedTask> {
    vec![
        DetailedTask::Working { fields: fields() },
        DetailedTask::InputRequired {
            fields: fields(),
            input_requests: serde_json::from_value(json!({
                "name": {
                    "method": "elicitation/create",
                    "params": {
                        "mode": "form",
                        "message": "Your name?",
                        "requestedSchema": {
                            "type": "object",
                            "properties": { "name": { "type": "string" } }
                        }
                    }
                }
            }))
            .expect("input requests"),
        },
        DetailedTask::Completed {
            fields: fields(),
            result: json!({ "content": [{ "type": "text", "text": "done" }], "isError": false }),
        },
        DetailedTask::Failed {
            fields: fields(),
            error: serde_json::to_value(
                turul_mcp_protocol_2026_07_28::McpError::ToolExecutionError("boom".into())
                    .to_error_object(),
            )
            .unwrap(),
        },
        DetailedTask::Cancelled { fields: fields() },
    ]
}

/// `CreateTaskResult`: `Result & Task & { resultType: "task" }`.
#[test]
fn create_task_result_matches_released_schema() {
    let r = CreateTaskResult::new(Task {
        status: TaskStatus::Working,
        fields: fields(),
    });
    assert_valid("CreateTaskResult", &serde_json::to_value(r).unwrap());
}

/// `GetTaskResult`: `Result & DetailedTask & { resultType: "complete" }`, for
/// every status, including `FailedTask.error` as a JSON-RPC error object.
#[test]
fn get_task_result_matches_released_schema_for_every_status() {
    for task in every_detailed_task() {
        let v = serde_json::to_value(GetTaskResult::new(task)).unwrap();
        assert_valid("GetTaskResult", &v);
    }
}

/// `UpdateTaskResult` and `CancelTaskResult`: `Result & { resultType: "complete" }`.
#[test]
fn ack_results_match_released_schema() {
    let v = serde_json::to_value(TaskAckResult::default()).unwrap();
    assert_valid("UpdateTaskResult", &v);
    assert_valid("CancelTaskResult", &v);
}

/// `TaskStatusNotificationParams`: `NotificationParams & DetailedTask`.
#[test]
fn status_notification_params_match_released_schema() {
    for task in every_detailed_task() {
        let v = serde_json::to_value(TaskStatusNotificationParams { meta: None, task }).unwrap();
        assert_valid("TaskStatusNotificationParams", &v);
    }
}

/// The validator is live: the released schema rejects a `tasks/get` result
/// claiming `resultType: "task"`, which only `CreateTaskResult` may carry.
#[test]
fn released_schema_rejects_task_discriminator_on_get() {
    let mut v = serde_json::to_value(GetTaskResult::new(DetailedTask::Working {
        fields: fields(),
    }))
    .unwrap();
    v["resultType"] = json!("task");
    let mut schema: Value = serde_json::from_str(SCHEMA).unwrap();
    schema["$ref"] = json!("#/$defs/GetTaskResult");
    let validator = jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .build(&schema)
        .unwrap();
    assert!(!validator.is_valid(&v), "schema must reject {v}");
}
