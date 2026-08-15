//! Wire-shape tests against the vendored SEP-2663 draft schema
//! (`schema/draft-schema.ts` / `schema/draft-schema.json`).

use serde_json::{Value, json};

use super::capability::{EXTENSION_IDENTIFIER, validate_identifier};
use super::lifecycle::{
    GetTaskParams, GetTaskResult, TaskAckResult, TaskStatusNotificationParams,
    TaskSubscriptionNotifications, UpdateTaskParams,
};
use super::types::{CreateTaskResult, DetailedTask, Nullable, Task, TaskFields, TaskStatus};

fn fields(task_id: &str) -> TaskFields {
    TaskFields {
        task_id: task_id.to_string(),
        status_message: None,
        created_at: "2026-06-12T00:00:00Z".to_string(),
        last_updated_at: "2026-06-12T00:00:05Z".to_string(),
        ttl_ms: Nullable(Some(60000.0)),
        poll_interval_ms: Some(500.0),
    }
}

/// `Task`: camelCase keys, snake_case status values.
#[test]
fn task_wire_shape() {
    let task = Task {
        status: TaskStatus::Working,
        fields: fields("t-1"),
    };
    let v = serde_json::to_value(&task).unwrap();
    assert_eq!(
        v,
        json!({
            "status": "working",
            "taskId": "t-1",
            "createdAt": "2026-06-12T00:00:00Z",
            "lastUpdatedAt": "2026-06-12T00:00:05Z",
            "ttlMs": 60000.0,
            "pollIntervalMs": 500.0
        })
    );
}

/// `ttlMs: number | null` is REQUIRED and nullable — `None` must serialize
/// as an explicit `null`, never as an absent key.
#[test]
fn ttl_ms_null_is_explicit() {
    let mut f = fields("t-2");
    f.ttl_ms = Nullable::null();
    f.poll_interval_ms = None;
    let v = serde_json::to_value(&Task {
        status: TaskStatus::Working,
        fields: f,
    })
    .unwrap();
    assert!(v.as_object().unwrap().contains_key("ttlMs"));
    assert_eq!(v["ttlMs"], Value::Null);
    // pollIntervalMs is optional (not nullable) — absent when None.
    assert!(!v.as_object().unwrap().contains_key("pollIntervalMs"));
}

/// Every `TaskStatus` round-trips through its snake_case wire string.
#[test]
fn task_status_wire_strings() {
    for (status, wire) in [
        (TaskStatus::Working, "working"),
        (TaskStatus::InputRequired, "input_required"),
        (TaskStatus::Completed, "completed"),
        (TaskStatus::Failed, "failed"),
        (TaskStatus::Cancelled, "cancelled"),
    ] {
        assert_eq!(serde_json::to_value(status).unwrap(), json!(wire));
        assert_eq!(
            serde_json::from_value::<TaskStatus>(json!(wire)).unwrap(),
            status
        );
    }
}

/// `DetailedTask` is discriminated by `status` with variant fields inlined:
/// `CompletedTask` carries `result`, `FailedTask` carries `error`.
#[test]
fn detailed_task_variants() {
    let completed = DetailedTask::Completed {
        fields: fields("t-3"),
        result: json!({"content": [{"type": "text", "text": "done"}]}),
    };
    let v = serde_json::to_value(&completed).unwrap();
    assert_eq!(v["status"], "completed");
    assert_eq!(v["result"]["content"][0]["text"], "done");
    let back: DetailedTask = serde_json::from_value(v.clone()).unwrap();
    assert_eq!(serde_json::to_value(&back).unwrap(), v);

    let failed = DetailedTask::Failed {
        fields: fields("t-4"),
        error: json!({"code": -32603, "message": "boom"}),
    };
    let v = serde_json::to_value(&failed).unwrap();
    assert_eq!(v["status"], "failed");
    assert_eq!(v["error"]["code"], -32603);

    let working = serde_json::from_value::<DetailedTask>(json!({
        "status": "working",
        "taskId": "t-5",
        "createdAt": "2026-06-12T00:00:00Z",
        "lastUpdatedAt": "2026-06-12T00:00:00Z",
        "ttlMs": null
    }))
    .unwrap();
    assert_eq!(working.status(), TaskStatus::Working);
    assert_eq!(working.fields().ttl_ms, Nullable::null());
}

/// `InputRequiredTask` carries `inputRequests` keyed by arbitrary ids; the
/// request values are the core protocol's MRTR `InputRequest` shapes.
#[test]
fn input_required_task_carries_input_requests() {
    let v: DetailedTask = serde_json::from_value(json!({
        "status": "input_required",
        "taskId": "t-6",
        "createdAt": "2026-06-12T00:00:00Z",
        "lastUpdatedAt": "2026-06-12T00:00:01Z",
        "ttlMs": null,
        "inputRequests": {
            "q1": {
                "method": "elicitation/create",
                "params": {
                    "message": "Proceed?",
                    "requestedSchema": {"type": "object", "properties": {"ok": {"type": "boolean"}}}
                }
            }
        }
    }))
    .expect("InputRequiredTask must parse with a core MRTR elicit request");
    let DetailedTask::InputRequired { input_requests, .. } = &v else {
        panic!("wrong variant: {v:?}");
    };
    assert!(input_requests.contains_key("q1"));
}

/// `CreateTaskResult` is `Result & Task` flat with `resultType: "task"`.
#[test]
fn create_task_result_wire_shape() {
    let r = CreateTaskResult::new(Task {
        status: TaskStatus::Working,
        fields: fields("t-7"),
    });
    assert!(r.has_task_discriminator());
    let v = serde_json::to_value(&r).unwrap();
    assert_eq!(v["resultType"], "task");
    assert_eq!(v["taskId"], "t-7"); // flat — no nested "task" object
    assert!(v.get("task").is_none());

    let back: CreateTaskResult = serde_json::from_value(v).unwrap();
    assert!(back.has_task_discriminator());
    assert_eq!(back.task.fields.task_id, "t-7");
}

/// `tasks/get` params and result; the get's own `resultType` is `"complete"`
/// regardless of the task's inner status.
#[test]
fn get_task_round_trip() {
    let params = GetTaskParams {
        task_id: "t-8".to_string(),
        meta: None,
    };
    assert_eq!(
        serde_json::to_value(&params).unwrap(),
        json!({"taskId": "t-8"})
    );

    let result = GetTaskResult::new(DetailedTask::Cancelled {
        fields: fields("t-8"),
    });
    let v = serde_json::to_value(&result).unwrap();
    assert_eq!(v["resultType"], "complete");
    assert_eq!(v["status"], "cancelled");
}

/// `tasks/update` carries `inputResponses` keyed to outstanding requests.
#[test]
fn update_task_params_wire_shape() {
    let params: UpdateTaskParams = serde_json::from_value(json!({
        "taskId": "t-9",
        "inputResponses": {
            "q1": {"action": "accept", "content": {"ok": true}}
        }
    }))
    .unwrap();
    assert_eq!(params.task_id, "t-9");
    assert!(params.input_responses.contains_key("q1"));
}

/// Update/cancel acks are plain `Result`s defaulting to `"complete"`.
#[test]
fn ack_result_defaults_complete() {
    let ack: TaskAckResult = serde_json::from_value(json!({})).unwrap();
    assert_eq!(ack.result_type.as_str(), "complete");
    assert_eq!(
        serde_json::to_value(TaskAckResult::default()).unwrap()["resultType"],
        "complete"
    );
}

/// `notifications/tasks` params flatten a `DetailedTask`.
#[test]
fn task_status_notification_params() {
    let p = TaskStatusNotificationParams {
        meta: None,
        task: DetailedTask::Working {
            fields: fields("t-10"),
        },
    };
    let v = serde_json::to_value(&p).unwrap();
    assert_eq!(v["status"], "working");
    assert_eq!(v["taskId"], "t-10");
}

/// `subscriptions/listen` filter addition: `taskIds`.
#[test]
fn task_subscription_filter() {
    let f = TaskSubscriptionNotifications {
        task_ids: Some(vec!["t-11".to_string()]),
    };
    assert_eq!(
        serde_json::to_value(&f).unwrap(),
        json!({"taskIds": ["t-11"]})
    );
}

/// The extension identifier itself passes SEP-2133 validation; malformed
/// identifiers are rejected.
#[test]
fn extension_identifier_validation() {
    validate_identifier(EXTENSION_IDENTIFIER).unwrap();
    validate_identifier("com.example/my-ext").unwrap();
    assert!(validate_identifier("no-separator").is_err());
    assert!(validate_identifier("io.modelcontextprotocol/").is_err());
    assert!(validate_identifier("bare/name").is_err());
}

/// Capability declaration: empty object under the identifier.
#[test]
fn capability_negotiation_shape() {
    use turul_mcp_protocol_2026_07_28::initialize::ClientCapabilities;

    let caps: ClientCapabilities = serde_json::from_value(json!({
        "extensions": { EXTENSION_IDENTIFIER: {} }
    }))
    .unwrap();
    assert!(super::capability::declared_by_client(&caps));

    let none: ClientCapabilities = serde_json::from_value(json!({})).unwrap();
    assert!(!super::capability::declared_by_client(&none));
    assert_eq!(super::capability::capability(), json!({}));
}

/// `ttlMs` is in the schema's `Task.required` list: a payload missing the
/// key must FAIL to parse (null is fine; absence is not).
#[test]
fn missing_ttl_ms_is_a_parse_error() {
    let err = serde_json::from_value::<Task>(json!({
        "status": "working",
        "taskId": "t-12",
        "createdAt": "2026-06-12T00:00:00Z",
        "lastUpdatedAt": "2026-06-12T00:00:00Z"
    }))
    .unwrap_err();
    assert!(err.to_string().contains("ttlMs"), "{err}");

    // Same contract through the DetailedTask union.
    assert!(
        serde_json::from_value::<DetailedTask>(json!({
            "status": "cancelled",
            "taskId": "t-13",
            "createdAt": "2026-06-12T00:00:00Z",
            "lastUpdatedAt": "2026-06-12T00:00:00Z"
        }))
        .is_err()
    );
}
