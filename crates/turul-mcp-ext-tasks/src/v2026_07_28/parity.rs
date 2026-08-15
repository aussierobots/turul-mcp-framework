//! Behaviour contract every [`TaskStore`] backend must satisfy, written once.
//!
//! Each function takes `&dyn TaskStore` and asserts one invariant from the
//! trait's documented contract. A backend is correct when it passes all of
//! them; a backend that passes none of them is what this crate shipped
//! before, because a per-backend test suite tests whatever that backend
//! happens to do.
//!
//! Precedent, and the reason this module is *called* rather than merely
//! written: `turul-mcp-task-storage` (the superseded 2025-11-25 store) has an
//! equivalent `parity_tests.rs` of the same design, and nothing anywhere
//! invokes it — its SQLite, Postgres and DynamoDB backends have never been
//! executed by any test. Design was not the failure there; wiring was.
//!
//! Enabled by the `parity-harness` feature so implementors outside this
//! workspace can hold their own backends to the same contract.

use std::collections::HashMap;

use serde_json::{Value, json};
use turul_mcp_protocol_2026_07_28::elicitation::{ElicitRequest, ElicitationSchema};
use turul_mcp_protocol_2026_07_28::input_required::{InputRequest, InputRequests};

use super::traits::{InputDelivery, RetentionPolicy, TaskState, TaskStore, TaskStoreError};
use super::types::{Nullable, TaskFields, TaskStatus};

/// A `working` task with the given id and owner.
pub fn seed(task_id: &str, owner: Option<&str>) -> TaskState {
    TaskState {
        fields: TaskFields {
            task_id: task_id.to_string(),
            status_message: Some("executing tool".to_string()),
            // Fixed timestamps: a backend must round-trip what it was given,
            // not substitute its own clock.
            created_at: "2026-08-15T00:00:00.000Z".to_string(),
            last_updated_at: "2026-08-15T00:00:00.000Z".to_string(),
            ttl_ms: Nullable(None),
            poll_interval_ms: Some(500.0),
        },
        status: TaskStatus::Working,
        owner: owner.map(str::to_string),
        input_requests: None,
        collected_responses: Default::default(),
        request_state: None,
        result: None,
        error: None,
    }
}

/// One elicit request under `key`.
pub fn one_request(key: &str) -> InputRequests {
    let schema = ElicitationSchema::new();
    let mut requests = InputRequests::new();
    requests.insert(
        key.to_string(),
        InputRequest::Elicit(ElicitRequest::new_form("?", schema)),
    );
    requests
}

/// An accepted elicit response, as it arrives on the wire.
pub fn accept() -> Value {
    json!({ "action": "accept", "content": {} })
}

/// SEP-2663 line 302: a `CreateTaskResult` must not be returned until a
/// `tasks/get` would resolve — so `create` must be durable before it returns.
pub async fn create_is_visible_immediately(store: &dyn TaskStore) {
    store.create(seed("t-create", None)).await.expect("create");
    let got = store.get("t-create", None).await.expect("get");
    let got = got.expect("a task must be findable the instant create() returns");
    assert_eq!(got.fields.task_id, "t-create");
    assert_eq!(got.status, TaskStatus::Working);
    assert_eq!(
        got.fields.created_at, "2026-08-15T00:00:00.000Z",
        "a backend must round-trip the caller's timestamps, not its own clock"
    );
}

/// An unknown id and someone else's task must be indistinguishable — the
/// State Handle Hijacking guidance turns a task id into a bearer token.
pub async fn unknown_and_foreign_tasks_are_both_none(store: &dyn TaskStore) {
    store
        .create(seed("t-owned", Some("alice")))
        .await
        .expect("create");

    assert!(
        store
            .get("t-owned", Some("bob"))
            .await
            .expect("get")
            .is_none(),
        "another principal's task must read as absent, not as forbidden"
    );
    assert!(
        store.get("t-owned", None).await.expect("get").is_none(),
        "an unauthenticated caller must not reach an owned task"
    );
    assert!(
        store
            .get("t-nonexistent", Some("alice"))
            .await
            .expect("get")
            .is_none(),
        "an unknown id reads as absent"
    );
    assert!(
        store
            .get("t-owned", Some("alice"))
            .await
            .expect("get")
            .is_some(),
        "the owner still reaches their own task"
    );
}

/// An unowned task (no authenticated principal at creation) answers to anyone
/// — the deployment's own no-isolation posture, made explicit rather than
/// pretending to isolate callers it cannot identify.
pub async fn unowned_tasks_answer_to_anyone(store: &dyn TaskStore) {
    store.create(seed("t-unowned", None)).await.expect("create");
    assert!(
        store
            .get("t-unowned", Some("anyone"))
            .await
            .expect("get")
            .is_some()
    );
    assert!(store.get("t-unowned", None).await.expect("get").is_some());
}

/// `complete` and `fail` are terminal and carry their payload back out.
pub async fn complete_and_fail_store_their_payloads(store: &dyn TaskStore) {
    store.create(seed("t-ok", None)).await.expect("create");
    let done = store
        .complete(
            "t-ok",
            json!({ "content": [{ "type": "text", "text": "hi" }] }),
        )
        .await
        .expect("complete");
    assert_eq!(done.status, TaskStatus::Completed);
    assert_eq!(done.result.expect("result")["content"][0]["text"], "hi");

    store.create(seed("t-err", None)).await.expect("create");
    let failed = store
        .fail("t-err", json!({ "code": -32603, "message": "boom" }))
        .await
        .expect("fail");
    assert_eq!(failed.status, TaskStatus::Failed);
    assert_eq!(failed.error.expect("error")["code"], -32603);
}

/// Terminal states are immutable: nothing moves a finished task.
pub async fn terminal_states_are_immutable(store: &dyn TaskStore) {
    store.create(seed("t-term", None)).await.expect("create");
    store.complete("t-term", json!({})).await.expect("complete");

    assert!(
        matches!(
            store.complete("t-term", json!({})).await,
            Err(TaskStoreError::InvalidStatus { .. })
        ),
        "completing a completed task must be rejected"
    );
    assert!(
        matches!(
            store.fail("t-term", json!({})).await,
            Err(TaskStoreError::InvalidStatus { .. })
        ),
        "failing a completed task must be rejected"
    );
    assert!(
        matches!(
            store.require_input("t-term", one_request("k"), None).await,
            Err(TaskStoreError::InvalidStatus { .. })
        ),
        "a completed task cannot start an input round"
    );
}

/// `cancel` flips a live task and acks a dead one with `Ok(None)`.
pub async fn cancel_flips_live_and_acks_terminal(store: &dyn TaskStore) {
    store.create(seed("t-cancel", None)).await.expect("create");
    let cancelled = store
        .cancel("t-cancel", None)
        .await
        .expect("cancel")
        .expect("a non-terminal task yields its new state");
    assert_eq!(cancelled.status, TaskStatus::Cancelled);

    assert!(
        store
            .cancel("t-cancel", None)
            .await
            .expect("cancel")
            .is_none(),
        "cancelling an already-terminal task acks with Ok(None), not an error"
    );
}

/// Cancel is owner-bound, and reports a foreign task as unknown.
pub async fn cancel_is_owner_bound(store: &dyn TaskStore) {
    store
        .create(seed("t-cancel-owned", Some("alice")))
        .await
        .expect("create");
    assert!(
        matches!(
            store.cancel("t-cancel-owned", Some("bob")).await,
            Err(TaskStoreError::NotFound(_))
        ),
        "another principal's task must report as NotFound, leaking nothing"
    );
}

/// The full input round: `require_input` parks, `provide_input` completes it
/// and hands back the responses plus the tool's echoed `requestState`.
pub async fn input_round_completes_and_returns_request_state(store: &dyn TaskStore) {
    store.create(seed("t-input", None)).await.expect("create");
    let parked = store
        .require_input("t-input", one_request("k1"), Some("state-1".into()))
        .await
        .expect("require_input");
    assert_eq!(parked.status, TaskStatus::InputRequired);
    assert!(parked.input_requests.expect("requests").contains_key("k1"));

    let mut responses = HashMap::new();
    responses.insert("k1".to_string(), accept());
    match store.provide_input("t-input", None, responses).await {
        Ok(InputDelivery::Complete {
            responses,
            request_state,
        }) => {
            assert!(responses.contains_key("k1"));
            assert_eq!(request_state.as_deref(), Some("state-1"));
        }
        other => panic!("a fully answered round must deliver Complete; got {other:?}"),
    }

    let resumed = store
        .get("t-input", None)
        .await
        .expect("get")
        .expect("task");
    assert_eq!(
        resumed.status,
        TaskStatus::Working,
        "a completed round returns the task to working"
    );
}

/// Partial fulfilment keeps the task parked AND drops the keys already
/// answered, so a following `tasks/get` advertises only what is still
/// outstanding. Leaving them in told the client to answer questions it had
/// already answered, with no way to tell the difference (found by the
/// conformance scenario `tasks-mrtr-input`).
pub async fn partial_delivery_drops_answered_keys(store: &dyn TaskStore) {
    store.create(seed("t-partial", None)).await.expect("create");
    let mut requests = one_request("first");
    requests.extend(one_request("second"));
    store
        .require_input("t-partial", requests, None)
        .await
        .expect("require_input");

    let mut responses = HashMap::new();
    responses.insert("first".to_string(), accept());
    assert!(
        matches!(
            store.provide_input("t-partial", None, responses).await,
            Ok(InputDelivery::Partial)
        ),
        "one of two answers is a partial delivery"
    );

    let still = store
        .get("t-partial", None)
        .await
        .expect("get")
        .expect("task");
    assert_eq!(still.status, TaskStatus::InputRequired);
    let outstanding = still.input_requests.expect("requests");
    assert!(
        !outstanding.contains_key("first"),
        "an answered key must be removed from inputRequests"
    );
    assert!(
        outstanding.contains_key("second"),
        "the unanswered key must remain outstanding"
    );
}

/// A response for a key the task is not waiting on is inert: ignored, and
/// delivery still succeeds. SEP-2663's ack-only design reserves errors for
/// "clearly invalid requests — such as an unknown `taskId`".
pub async fn inert_keys_are_ignored_not_rejected(store: &dyn TaskStore) {
    store.create(seed("t-inert", None)).await.expect("create");
    store
        .require_input("t-inert", one_request("wanted"), None)
        .await
        .expect("require_input");

    let mut responses = HashMap::new();
    // Not an InputResponse of any variant, under a key never asked for.
    responses.insert("never-asked".to_string(), json!({ "ignored": true }));
    assert!(
        matches!(
            store.provide_input("t-inert", None, responses).await,
            Ok(InputDelivery::Partial)
        ),
        "an inert key must not fail the delivery"
    );

    let still = store
        .get("t-inert", None)
        .await
        .expect("get")
        .expect("task");
    assert_eq!(still.status, TaskStatus::InputRequired);
    assert!(
        still
            .input_requests
            .expect("requests")
            .contains_key("wanted"),
        "the real outstanding request must survive an ignored key"
    );
}

/// The other half: a malformed response for a key the task IS waiting on
/// blocks the round, so it errors — and the error names the key.
pub async fn malformed_response_for_an_outstanding_key_names_it(store: &dyn TaskStore) {
    store.create(seed("t-bad", None)).await.expect("create");
    store
        .require_input("t-bad", one_request("wanted"), None)
        .await
        .expect("require_input");

    let mut responses = HashMap::new();
    responses.insert("wanted".to_string(), json!({ "ignored": true }));
    match store.provide_input("t-bad", None, responses).await {
        Err(TaskStoreError::InvalidInputResponse { key, .. }) => {
            assert_eq!(key, "wanted", "the error must name the offending key");
        }
        other => panic!("expected InvalidInputResponse; got {other:?}"),
    }
}

/// `provide_input` is owner-bound like `get` and `cancel`.
pub async fn provide_input_is_owner_bound(store: &dyn TaskStore) {
    store
        .create(seed("t-input-owned", Some("alice")))
        .await
        .expect("create");
    store
        .require_input("t-input-owned", one_request("k"), None)
        .await
        .expect("require_input");

    let mut responses = HashMap::new();
    responses.insert("k".to_string(), accept());
    assert!(
        matches!(
            store
                .provide_input("t-input-owned", Some("bob"), responses)
                .await,
            Err(TaskStoreError::NotFound(_))
        ),
        "another principal must not answer someone else's input round"
    );
}

/// Operations on an unknown task id report `NotFound` rather than panicking
/// or silently succeeding.
pub async fn unknown_task_operations_report_not_found(store: &dyn TaskStore) {
    assert!(matches!(
        store.complete("t-ghost", json!({})).await,
        Err(TaskStoreError::NotFound(_))
    ));
    assert!(matches!(
        store.fail("t-ghost", json!({})).await,
        Err(TaskStoreError::NotFound(_))
    ));
    assert!(matches!(
        store.require_input("t-ghost", one_request("k"), None).await,
        Err(TaskStoreError::NotFound(_))
    ));
    assert!(matches!(
        store.cancel("t-ghost", None).await,
        Err(TaskStoreError::NotFound(_))
    ));
    assert!(matches!(
        store.provide_input("t-ghost", None, HashMap::new()).await,
        Err(TaskStoreError::NotFound(_))
    ));
}

/// A `working` task last touched `age_ms` before `now`, with an optional
/// per-task TTL measured from `createdAt`.
pub fn aged(
    task_id: &str,
    now: chrono::DateTime<chrono::Utc>,
    age_ms: i64,
    ttl_ms: Option<f64>,
) -> TaskState {
    let stamp =
        |d: chrono::DateTime<chrono::Utc>| d.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let then = now - chrono::Duration::milliseconds(age_ms);
    let mut state = seed(task_id, None);
    state.fields.created_at = stamp(then);
    state.fields.last_updated_at = stamp(then);
    state.fields.ttl_ms = Nullable(ttl_ms);
    state
}

/// Retention, which the in-memory store used to get for free by losing
/// everything on restart. A durable table only grows, so every backend has to
/// agree on exactly when a task is swept — hence these boundaries are pinned
/// here rather than per backend.
///
/// SEP-2663 makes all of this OPTIONAL (line 342: servers "MAY mark a task as
/// `failed` at any point after the TTL elapses, and subsequently delete it at
/// any time"), so the contract is that the POLICY is obeyed, not that any
/// particular sweeping happens.
pub async fn sweep_honours_the_policy(store: &dyn TaskStore) {
    let now = chrono::Utc::now();

    // An empty policy must be a no-op — otherwise enabling retention later
    // would silently delete history for anyone who never asked for it.
    store
        .create(aged("s-untouched", now, 10_000_000, None))
        .await
        .expect("create");
    let report = store
        .sweep(now, &RetentionPolicy::default())
        .await
        .expect("sweep");
    assert_eq!(
        report,
        Default::default(),
        "the default policy must change nothing"
    );
    assert!(
        store.get("s-untouched", None).await.expect("get").is_some(),
        "a no-op policy must not remove anything"
    );

    // Orphan recovery: a non-terminal task silent past the threshold is
    // presumed abandoned by an instance that died.
    store
        .create(aged("s-orphan", now, 60_000, None))
        .await
        .expect("create");
    store
        .create(aged("s-fresh", now, 1_000, None))
        .await
        .expect("create");
    let report = store
        .sweep(
            now,
            &RetentionPolicy {
                orphan_after_ms: Some(30_000),
                ..Default::default()
            },
        )
        .await
        .expect("sweep");
    assert!(
        report.failed.contains(&"s-orphan".to_string()),
        "{report:?}"
    );
    assert!(
        !report.failed.contains(&"s-fresh".to_string()),
        "a task inside the threshold must be left alone: {report:?}"
    );
    let orphan = store
        .get("s-orphan", None)
        .await
        .expect("get")
        .expect("task");
    assert_eq!(orphan.status, TaskStatus::Failed);
    assert!(
        orphan.error.expect("error")["message"]
            .as_str()
            .unwrap_or("")
            .contains("abandoned"),
        "a swept task must say why it failed"
    );

    // Per-task ttlMs, measured from createdAt. `ttlMs: null` is unlimited and
    // must never be swept — that is what the wire value means.
    store
        .create(aged("s-ttl", now, 90_000, Some(60_000.0)))
        .await
        .expect("create");
    store
        .create(aged("s-unlimited", now, 90_000, None))
        .await
        .expect("create");
    let report = store
        .sweep(
            now,
            &RetentionPolicy {
                honour_task_ttl: true,
                ..Default::default()
            },
        )
        .await
        .expect("sweep");
    assert!(report.failed.contains(&"s-ttl".to_string()), "{report:?}");
    assert!(
        !report.failed.contains(&"s-unlimited".to_string()),
        "ttlMs: null means unlimited and must never be swept: {report:?}"
    );

    // Terminal retention: old finished tasks are deleted outright.
    store
        .create(aged("s-old-done", now, 90_000, None))
        .await
        .expect("create");
    store
        .complete("s-old-done", json!({}))
        .await
        .expect("complete");
    // `complete` stamps last_updated_at to now, so it is NOT old enough yet.
    let report = store
        .sweep(
            now,
            &RetentionPolicy {
                delete_terminal_after_ms: Some(60_000),
                ..Default::default()
            },
        )
        .await
        .expect("sweep");
    assert!(
        !report.deleted.contains(&"s-old-done".to_string()),
        "a just-completed task is not old: {report:?}"
    );
    let later = now + chrono::Duration::milliseconds(120_000);
    let report = store
        .sweep(
            later,
            &RetentionPolicy {
                delete_terminal_after_ms: Some(60_000),
                ..Default::default()
            },
        )
        .await
        .expect("sweep");
    assert!(
        report.deleted.contains(&"s-old-done".to_string()),
        "{report:?}"
    );
    assert!(
        store.get("s-old-done", None).await.expect("get").is_none(),
        "a deleted task must really be gone"
    );
}

/// Run the whole contract against one backend.
///
/// Each check uses its own task ids, so a single fresh store can host all of
/// them; backends that need isolation may call the checks individually.
pub async fn run_all(store: &dyn TaskStore) {
    create_is_visible_immediately(store).await;
    unknown_and_foreign_tasks_are_both_none(store).await;
    unowned_tasks_answer_to_anyone(store).await;
    complete_and_fail_store_their_payloads(store).await;
    terminal_states_are_immutable(store).await;
    cancel_flips_live_and_acks_terminal(store).await;
    cancel_is_owner_bound(store).await;
    input_round_completes_and_returns_request_state(store).await;
    partial_delivery_drops_answered_keys(store).await;
    inert_keys_are_ignored_not_rejected(store).await;
    malformed_response_for_an_outstanding_key_names_it(store).await;
    provide_input_is_owner_bound(store).await;
    unknown_task_operations_report_not_found(store).await;
    sweep_honours_the_policy(store).await;
}

#[cfg(test)]
mod in_memory_conformance {
    use super::*;
    use crate::v2026_07_28::in_memory::InMemoryTaskStore;

    /// The reference backend must satisfy the contract it defines. If this
    /// ever fails, the contract and the reference have diverged and one of
    /// them is wrong — resolve that before touching any other backend.
    #[tokio::test]
    async fn in_memory_satisfies_the_contract() {
        run_all(&InMemoryTaskStore::default()).await;
    }
}
