//! DynamoDB [`TaskStore`] — durable and shared across instances, including
//! Lambda, where there is no long-lived process to hold state at all.
//!
//! Two things genuinely differ from the SQL backends, and both are visible in
//! the code rather than papered over:
//!
//! **1. No read-modify-write transaction.** SQL takes a row lock
//! (`SELECT … FOR UPDATE`); DynamoDB has no equivalent, so this uses
//! optimistic concurrency: every item carries a `rev`, writes are guarded by
//! `ConditionExpression: rev = :expected`, and a losing writer retries against
//! the winner's state. Without this, two instances mutating one task would
//! last-write-wins and silently drop an input round.
//!
//! **2. Native TTL.** DynamoDB deletes items itself from a numeric
//! epoch-seconds attribute (`ttl`), so old tasks cost nothing to reap —
//! no sweep job, no scan. The catch, and the reason [`TaskStore::sweep`] is
//! still implemented and reads still filter: **that deletion is eventual**,
//! documented by AWS as typically within 48 hours of expiry. An expired item
//! is readable until DynamoDB gets to it, so a `tasks/get` that trusted
//! storage alone would resurrect a task the other backends consider gone.
//! Filtering on read is what keeps all three backends behaving identically.

use std::collections::HashMap;

use aws_sdk_dynamodb::Client;
use aws_sdk_dynamodb::error::SdkError;
use aws_sdk_dynamodb::types::{
    AttributeDefinition, AttributeValue, BillingMode, KeySchemaElement, KeyType,
    ScalarAttributeType, TimeToLiveSpecification,
};
use serde_json::Value;
use turul_mcp_protocol_2026_07_28::input_required::InputRequests;

use super::traits::{
    InputDelivery, RetentionPolicy, SweepAction, SweepReport, TaskState, TaskStore, TaskStoreError,
    apply_cancel, apply_complete, apply_fail, apply_provide_input, apply_require_input,
    now_rfc3339, owner_matches, sweep_action, sweep_error,
};
use super::types::TaskStatus;

/// How many times a mutation retries when another writer won the race.
/// Contention is per-task, so this is generous in practice.
const MAX_ATTEMPTS: usize = 8;

const PK: &str = "taskId";
const ATTR_STATE: &str = "state";
const ATTR_REV: &str = "rev";
const ATTR_STATUS: &str = "status";
/// DynamoDB TTL attribute: **Number, Unix epoch SECONDS**. DynamoDB
/// silently ignores any other unit or type, so this is not a free choice.
///
/// Named `ttlEpoch`, matching `turul-mcp-task-storage` — this crate is that
/// store's successor and holds the same concept. The name is deliberately
/// NOT `ttl`, because `TaskFields.ttlMs` inside the `state` document is a
/// DURATION while this is an absolute INSTANT; one crate carrying both under
/// near-identical names is how someone eventually writes milliseconds into
/// the attribute DynamoDB reads as seconds, which fails silently.
/// (`turul-mcp-session-storage` uses plain `ttl`, but has no competing
/// duration field, so the ambiguity does not arise there.)
const ATTR_TTL_EPOCH: &str = "ttlEpoch";

fn backend(e: impl std::fmt::Display) -> TaskStoreError {
    TaskStoreError::Backend(e.to_string())
}

/// DynamoDB-backed [`TaskStore`].
pub struct DynamoDbTaskStore {
    client: Client,
    table: String,
    /// Used at write time to compute `ttlEpoch`, because DynamoDB's TTL is a
    /// per-item attribute rather than a query-time policy. `None` writes no
    /// TTL attribute at all, so nothing is ever auto-deleted.
    ///
    /// Behind a lock because the server builder sets it via
    /// [`TaskStore::configure_retention`] after the store is already an
    /// `Arc<dyn TaskStore>` — that hook is what makes
    /// `with_ext_tasks_retention` configure BOTH the sweep and this.
    retention: std::sync::RwLock<Option<RetentionPolicy>>,
}

impl DynamoDbTaskStore {
    /// Use an existing client (the normal path — an application configures
    /// region and credentials once).
    pub fn new(client: Client, table: impl Into<String>) -> Self {
        Self {
            client,
            table: table.into(),
            retention: std::sync::RwLock::new(None),
        }
    }

    /// Write `ttl` on every item according to `policy`, letting
    /// DynamoDB reclaim old tasks with no sweep job. Call
    /// [`Self::ensure_table`] (or enable TTL out of band) for it to take
    /// effect — writing the attribute does nothing until TTL is enabled on
    /// the table, which is silent rather than an error.
    pub fn with_retention(self, policy: RetentionPolicy) -> Self {
        *self.retention.write().expect("retention lock") = Some(policy);
        self
    }

    /// Create the table if absent and enable TTL on `ttl`.
    ///
    /// Intended for tests and local development. In a real deployment the
    /// table is usually provisioned by IaC — in which case enable TTL there,
    /// on the same attribute name, or items will simply never expire.
    pub async fn ensure_table(&self) -> Result<(), TaskStoreError> {
        let exists = self
            .client
            .describe_table()
            .table_name(&self.table)
            .send()
            .await
            .is_ok();

        if !exists {
            self.client
                .create_table()
                .table_name(&self.table)
                .billing_mode(BillingMode::PayPerRequest)
                .attribute_definitions(
                    AttributeDefinition::builder()
                        .attribute_name(PK)
                        .attribute_type(ScalarAttributeType::S)
                        .build()
                        .map_err(backend)?,
                )
                .key_schema(
                    KeySchemaElement::builder()
                        .attribute_name(PK)
                        .key_type(KeyType::Hash)
                        .build()
                        .map_err(backend)?,
                )
                .send()
                .await
                .map_err(backend)?;
        }

        // Idempotent: enabling TTL when it is already enabled errors, which is
        // not a failure worth propagating.
        let _ = self
            .client
            .update_time_to_live()
            .table_name(&self.table)
            .time_to_live_specification(
                TimeToLiveSpecification::builder()
                    .enabled(true)
                    .attribute_name(ATTR_TTL_EPOCH)
                    .build()
                    .map_err(backend)?,
            )
            .send()
            .await;

        Ok(())
    }

    /// The epoch-second instant DynamoDB should reclaim this item, if any.
    ///
    /// Mirrors [`super::traits::sweep_action`]'s inputs so auto-deletion and
    /// an explicit sweep agree about which tasks are past their time.
    fn expires_at(&self, state: &TaskState) -> Option<i64> {
        let guard = self.retention.read().expect("retention lock");
        let policy = guard.as_ref()?;
        let parse = |s: &str| {
            chrono::DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|t| t.with_timezone(&chrono::Utc))
        };

        if state.status.is_terminal() {
            let limit = policy.delete_terminal_after_ms?;
            let base = parse(&state.fields.last_updated_at)?;
            return Some((base + chrono::Duration::milliseconds(limit as i64)).timestamp());
        }
        // A live task expires by its own ttlMs, if it declared one. Orphan
        // recovery is deliberately NOT an expiry: an abandoned task should
        // become `failed` and stay readable, not vanish.
        if policy.honour_task_ttl
            && let Some(ttl) = state.fields.ttl_ms.0
            && let Some(base) = parse(&state.fields.created_at)
        {
            return Some((base + chrono::Duration::milliseconds(ttl as i64)).timestamp());
        }
        None
    }

    fn item_for(
        &self,
        state: &TaskState,
        rev: i64,
    ) -> Result<HashMap<String, AttributeValue>, TaskStoreError> {
        let doc = serde_json::to_string(state).map_err(backend)?;
        let mut item = HashMap::from([
            (
                PK.to_string(),
                AttributeValue::S(state.fields.task_id.clone()),
            ),
            (ATTR_STATE.to_string(), AttributeValue::S(doc)),
            (ATTR_REV.to_string(), AttributeValue::N(rev.to_string())),
            (
                ATTR_STATUS.to_string(),
                AttributeValue::S(super::traits::status_name(state.status).to_string()),
            ),
        ]);
        if let Some(exp) = self.expires_at(state) {
            item.insert(
                ATTR_TTL_EPOCH.to_string(),
                AttributeValue::N(exp.to_string()),
            );
        }
        Ok(item)
    }

    /// Fetch an item as `(state, rev)`, treating a TTL-expired item as absent.
    async fn load(&self, task_id: &str) -> Result<Option<(TaskState, i64)>, TaskStoreError> {
        let out = self
            .client
            .get_item()
            .table_name(&self.table)
            .key(PK, AttributeValue::S(task_id.to_string()))
            .consistent_read(true)
            .send()
            .await
            .map_err(backend)?;

        let Some(item) = out.item else {
            return Ok(None);
        };

        // DynamoDB deletes expired items only eventually (AWS documents up to
        // 48 hours). Until then the item is still readable, so honour the
        // expiry here or this backend would keep serving tasks the others
        // have already dropped.
        if let Some(AttributeValue::N(exp)) = item.get(ATTR_TTL_EPOCH)
            && let Ok(exp) = exp.parse::<i64>()
            && exp <= chrono::Utc::now().timestamp()
        {
            return Ok(None);
        }

        let Some(AttributeValue::S(doc)) = item.get(ATTR_STATE) else {
            return Err(TaskStoreError::Backend(format!(
                "task {task_id:?} item has no {ATTR_STATE} attribute"
            )));
        };
        let rev = match item.get(ATTR_REV) {
            Some(AttributeValue::N(n)) => n.parse::<i64>().unwrap_or(0),
            _ => 0,
        };
        Ok(Some((serde_json::from_str(doc).map_err(backend)?, rev)))
    }

    /// Load, apply a shared transition, store — retrying when another writer
    /// changed the item first. This is the moral equivalent of the SQL
    /// backends' `SELECT … FOR UPDATE`, achieved with a condition instead of
    /// a lock.
    async fn with_task<T>(
        &self,
        task_id: &str,
        mut f: impl FnMut(&mut TaskState) -> Result<T, TaskStoreError>,
    ) -> Result<T, TaskStoreError> {
        for _ in 0..MAX_ATTEMPTS {
            let (mut state, rev) = self
                .load(task_id)
                .await?
                .ok_or_else(|| TaskStoreError::NotFound(task_id.to_string()))?;

            let out = f(&mut state)?;
            let item = self.item_for(&state, rev + 1)?;

            let put = self
                .client
                .put_item()
                .table_name(&self.table)
                .set_item(Some(item))
                .condition_expression("attribute_not_exists(#r) OR #r = :expected")
                .expression_attribute_names("#r", ATTR_REV)
                .expression_attribute_values(":expected", AttributeValue::N(rev.to_string()))
                .send()
                .await;

            match put {
                Ok(_) => return Ok(out),
                Err(SdkError::ServiceError(e))
                    if e.err().is_conditional_check_failed_exception() =>
                {
                    // Someone else wrote first. Re-read and reapply, rather
                    // than clobbering their change.
                    continue;
                }
                Err(e) => return Err(backend(e)),
            }
        }
        Err(TaskStoreError::Backend(format!(
            "task {task_id:?} lost {MAX_ATTEMPTS} consecutive write races; giving up rather than clobbering another writer"
        )))
    }
}

#[async_trait::async_trait]
impl TaskStore for DynamoDbTaskStore {
    async fn create(&self, state: TaskState) -> Result<(), TaskStoreError> {
        // A durable write before returning: SEP-2663 line 302 requires a
        // `tasks/get` — possibly on another instance — to resolve the instant
        // `CreateTaskResult` is answered.
        let item = self.item_for(&state, 1)?;
        self.client
            .put_item()
            .table_name(&self.table)
            .set_item(Some(item))
            .send()
            .await
            .map_err(backend)?;
        Ok(())
    }

    async fn get(
        &self,
        task_id: &str,
        owner: Option<&str>,
    ) -> Result<Option<TaskState>, TaskStoreError> {
        let Some((state, _)) = self.load(task_id).await? else {
            return Ok(None);
        };
        // A foreign task reads as absent, never as forbidden.
        Ok(owner_matches(&state, owner).then_some(state))
    }

    async fn complete(&self, task_id: &str, result: Value) -> Result<TaskState, TaskStoreError> {
        let result = std::sync::Arc::new(result);
        self.with_task(task_id, move |state| {
            apply_complete(state, (*result).clone())?;
            Ok(state.clone())
        })
        .await
    }

    async fn fail(&self, task_id: &str, error: Value) -> Result<TaskState, TaskStoreError> {
        let error = std::sync::Arc::new(error);
        self.with_task(task_id, move |state| {
            apply_fail(state, (*error).clone())?;
            Ok(state.clone())
        })
        .await
    }

    async fn cancel(
        &self,
        task_id: &str,
        owner: Option<&str>,
    ) -> Result<Option<TaskState>, TaskStoreError> {
        self.with_task(task_id, |state| {
            Ok(apply_cancel(state, owner)?.then(|| state.clone()))
        })
        .await
    }

    async fn require_input(
        &self,
        task_id: &str,
        requests: InputRequests,
        request_state: Option<String>,
    ) -> Result<TaskState, TaskStoreError> {
        let requests = std::sync::Arc::new(requests);
        let request_state = std::sync::Arc::new(request_state);
        self.with_task(task_id, move |state| {
            apply_require_input(state, (*requests).clone(), (*request_state).clone())?;
            Ok(state.clone())
        })
        .await
    }

    async fn provide_input(
        &self,
        task_id: &str,
        owner: Option<&str>,
        responses: HashMap<String, Value>,
    ) -> Result<InputDelivery, TaskStoreError> {
        let responses = std::sync::Arc::new(responses);
        self.with_task(task_id, move |state| {
            apply_provide_input(state, owner, (*responses).clone())
        })
        .await
    }

    fn configure_retention(&self, policy: &RetentionPolicy) {
        *self.retention.write().expect("retention lock") = Some(policy.clone());
    }

    async fn sweep(
        &self,
        now: chrono::DateTime<chrono::Utc>,
        policy: &RetentionPolicy,
    ) -> Result<SweepReport, TaskStoreError> {
        // Deletion of expired items is DynamoDB's job (see `expires_at`), so
        // this exists mainly for orphan recovery and for deployments that
        // never enabled TTL on the table. A scan is acceptable because a
        // sweep is a maintenance operation, never a request path.
        let mut report = SweepReport::default();
        let mut last_key = None;

        loop {
            let page = self
                .client
                .scan()
                .table_name(&self.table)
                .set_exclusive_start_key(last_key.clone())
                .send()
                .await
                .map_err(backend)?;

            for item in page.items() {
                let Some(AttributeValue::S(doc)) = item.get(ATTR_STATE) else {
                    continue;
                };
                let state: TaskState = serde_json::from_str(doc).map_err(backend)?;
                let id = state.fields.task_id.clone();
                match sweep_action(&state, now, policy) {
                    SweepAction::Keep => {}
                    SweepAction::Delete => {
                        self.client
                            .delete_item()
                            .table_name(&self.table)
                            .key(PK, AttributeValue::S(id.clone()))
                            .send()
                            .await
                            .map_err(backend)?;
                        report.deleted.push(id);
                    }
                    SweepAction::MarkFailed(reason) => {
                        let outcome = self
                            .with_task(&id, |s| {
                                s.status = TaskStatus::Failed;
                                s.error = Some(sweep_error(reason));
                                s.input_requests = None;
                                s.fields.last_updated_at = now_rfc3339();
                                Ok(())
                            })
                            .await;
                        match outcome {
                            Ok(()) => report.failed.push(id),
                            // Raced with a real writer, which is the writer
                            // that should win: the task was not abandoned.
                            Err(TaskStoreError::NotFound(_)) => {}
                            Err(e) => return Err(e),
                        }
                    }
                }
            }

            last_key = page.last_evaluated_key().cloned();
            if last_key.is_none() {
                break;
            }
        }
        Ok(report)
    }
}

#[cfg(test)]
mod conformance {
    use super::*;
    use crate::v2026_07_28::parity;
    use aws_sdk_dynamodb::config::{BehaviorVersion, Credentials, Region};

    fn endpoint() -> String {
        std::env::var("TURUL_TEST_DDB_URL").unwrap_or_else(|_| "http://127.0.0.1:8123".to_string())
    }

    /// A client for DynamoDB Local. Credentials are required by the SDK's
    /// signer but never checked by DynamoDB Local — they are placeholders,
    /// not secrets, and no real AWS account is involved.
    async fn local_client() -> Client {
        let cfg = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new("us-east-1"))
            .endpoint_url(endpoint())
            .credentials_provider(Credentials::new("local", "local", None, None, "turul-test"))
            .load()
            .await;
        Client::new(&cfg)
    }

    /// A fresh table per test, so runs cannot collide on the fixed task ids
    /// the parity contract uses.
    async fn store(retention: Option<RetentionPolicy>) -> Option<DynamoDbTaskStore> {
        if std::env::var("TURUL_SKIP_DDB_TESTS").is_ok() {
            eprintln!("TURUL_SKIP_DDB_TESTS set — skipping DynamoDB conformance deliberately");
            return None;
        }
        let client = local_client().await;
        // Reachability is checked here so an unreachable endpoint FAILS.
        // These tests must never report green without contacting a server —
        // the Postgres backend silently skipped for exactly that reason
        // before this pattern was adopted.
        if let Err(e) = client.list_tables().send().await {
            panic!(
                "DynamoDB Local unreachable at {}: {e}\n\
                 Start it with:\n  java -Djava.library.path=./DynamoDBLocal_lib \\\n    \
                 -jar DynamoDBLocal.jar -inMemory -port 8123\n\
                 or set TURUL_SKIP_DDB_TESTS=1 to opt out on purpose.",
                endpoint()
            );
        }
        let table = format!(
            "turul_ext_tasks_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        let mut s = DynamoDbTaskStore::new(client, table);
        if let Some(p) = retention {
            s = s.with_retention(p);
        }
        s.ensure_table().await.expect("create table");
        Some(s)
    }

    /// The same contract the other three backends satisfy, against real
    /// DynamoDB — including optimistic concurrency standing in for the SQL
    /// row lock.
    #[tokio::test]
    async fn dynamodb_satisfies_the_contract() {
        let Some(store) = store(None).await else {
            return;
        };
        parity::run_all(&store).await;
    }

    /// The TTL attribute is the point of this backend's retention: DynamoDB
    /// reclaims items itself, with no sweep job. Two things are asserted —
    /// that `ttl` is actually written (an unwritten attribute expires
    /// nothing), and that an already-expired item reads as absent despite
    /// DynamoDB's deletion being eventual.
    #[tokio::test]
    async fn ttl_attribute_is_written_and_honoured_on_read() {
        let policy = RetentionPolicy {
            honour_task_ttl: true,
            delete_terminal_after_ms: Some(60_000),
            ..Default::default()
        };
        let Some(store) = store(Some(policy)).await else {
            return;
        };

        // A live task with its own ttlMs carries ttl = createdAt + ttlMs.
        let mut live = parity::seed("t-ttl", None);
        live.fields.ttl_ms = crate::v2026_07_28::types::Nullable(Some(3_600_000.0));
        live.fields.created_at =
            chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        live.fields.last_updated_at = live.fields.created_at.clone();
        store.create(live).await.expect("create");

        let raw = store
            .client
            .get_item()
            .table_name(&store.table)
            .key(PK, AttributeValue::S("t-ttl".into()))
            .send()
            .await
            .expect("get_item");
        let item = raw.item.expect("item");
        let expires = match item.get(ATTR_TTL_EPOCH) {
            Some(AttributeValue::N(n)) => n.parse::<i64>().expect("epoch seconds"),
            other => panic!("ttl must be a Number for DynamoDB TTL; got {other:?}"),
        };
        let now = chrono::Utc::now().timestamp();
        assert!(
            expires > now + 3_000 && expires < now + 4_200,
            "ttl must be createdAt + ttlMs in EPOCH SECONDS (DynamoDB \
             ignores an attribute in any other unit); got {expires} vs now {now}"
        );

        // TTL is enabled on the attribute DynamoDB will actually look at.
        let ttl_desc = store
            .client
            .describe_time_to_live()
            .table_name(&store.table)
            .send()
            .await
            .expect("describe_time_to_live");
        let spec = ttl_desc
            .time_to_live_description()
            .expect("ttl description");
        assert_eq!(spec.attribute_name(), Some(ATTR_TTL_EPOCH));

        // An item already past its expiry must read as absent even though
        // DynamoDB has not deleted it yet — deletion is eventual (AWS
        // documents up to 48h), so storage alone cannot be trusted.
        let mut stale = parity::seed("t-expired", None);
        stale.fields.ttl_ms = crate::v2026_07_28::types::Nullable(Some(1.0));
        stale.fields.created_at = (chrono::Utc::now() - chrono::Duration::hours(2))
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        stale.fields.last_updated_at = stale.fields.created_at.clone();
        store.create(stale).await.expect("create");

        assert!(
            store
                .client
                .get_item()
                .table_name(&store.table)
                .key(PK, AttributeValue::S("t-expired".into()))
                .send()
                .await
                .expect("raw get")
                .item
                .is_some(),
            "precondition: DynamoDB Local has not physically deleted it yet"
        );
        assert!(
            store.get("t-expired", None).await.expect("get").is_none(),
            "an expired task must read as absent, or this backend serves tasks \
             the other backends consider gone"
        );
    }
}
