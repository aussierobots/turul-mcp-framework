//! AWS DynamoDB task storage backend.
//!
//! Serverless and AWS-native task storage for distributed MCP deployments.
//! Uses a single table with GSIs for session-based and status-based queries.
//!
//! ## Table Schema
//!
//! New tables (v0.3.4+) use camelCase attribute names:
//! - **Partition Key**: `taskId` (S)
//! - **GSI `SessionIndex`**: PK=`sessionId`, SK=`createdAt`
//! - **GSI `StatusIndex`**: PK=`status`, SK=`createdAt`
//! - **TTL attribute**: `ttlEpoch` (N, Unix epoch seconds)
//!
//! Legacy tables (pre-v0.3.4) use snake_case and are auto-detected via
//! `describe_table()` key schema inspection. Both conventions are fully supported.

use crate::error::TaskStorageError;
use crate::state_machine;
use crate::traits::{TaskListPage, TaskOutcome, TaskRecord, TaskStorage};
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use tracing::{debug, error, info, warn};
use turul_mcp_protocol::TaskStatus;

#[cfg(feature = "dynamodb")]
use aws_config::{BehaviorVersion, Region};
#[cfg(feature = "dynamodb")]
use aws_sdk_dynamodb::Client;
#[cfg(feature = "dynamodb")]
use aws_sdk_dynamodb::types::AttributeValue;
#[cfg(feature = "dynamodb")]
use base64::Engine;

/// Naming convention detected from an existing DynamoDB table's key schema.
///
/// New tables use `CamelCase`; legacy pre-v0.3.4 tables use `SnakeCase`.
/// Detected automatically via `describe_table()` — callers never choose.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum NamingConvention {
    /// Legacy pre-v0.3.4 tables with snake_case attribute names.
    SnakeCase,
    /// New v0.3.4+ tables with camelCase attribute names.
    CamelCase,
}

/// Attribute name mappings for a given naming convention.
#[derive(Debug, Clone)]
pub(crate) struct TaskAttrNames {
    pub task_id: &'static str,
    pub session_id: &'static str,
    pub created_at: &'static str,
    pub last_updated_at: &'static str,
    pub status_message: &'static str,
    pub ttl_epoch: &'static str,
    pub poll_interval: &'static str,
    pub original_method: &'static str,
    pub original_params: &'static str,
    // Single-word attributes (unchanged across conventions):
    // status, ttl, result, meta
}

impl TaskAttrNames {
    const SNAKE: Self = Self {
        task_id: "task_id",
        session_id: "session_id",
        created_at: "created_at",
        last_updated_at: "last_updated_at",
        status_message: "status_message",
        ttl_epoch: "ttl_epoch",
        poll_interval: "poll_interval",
        original_method: "original_method",
        original_params: "original_params",
    };

    const CAMEL: Self = Self {
        task_id: "taskId",
        session_id: "sessionId",
        created_at: "createdAt",
        last_updated_at: "lastUpdatedAt",
        status_message: "statusMessage",
        ttl_epoch: "ttlEpoch",
        poll_interval: "pollInterval",
        original_method: "originalMethod",
        original_params: "originalParams",
    };

    pub(crate) fn for_convention(convention: NamingConvention) -> &'static Self {
        match convention {
            NamingConvention::SnakeCase => &Self::SNAKE,
            NamingConvention::CamelCase => &Self::CAMEL,
        }
    }
}

/// Detect whether a DynamoDB table uses snake_case or camelCase attribute names
/// by inspecting its key schema. Any multi-word key containing an underscore
/// indicates a legacy snake_case table.
#[cfg(feature = "dynamodb")]
fn detect_naming_convention(table: &aws_sdk_dynamodb::types::TableDescription) -> NamingConvention {
    for element in table.key_schema() {
        let name = element.attribute_name();
        if name.contains('_') {
            return NamingConvention::SnakeCase;
        }
    }
    NamingConvention::CamelCase
}

/// Read-tolerance helper: tries `primary` attribute name first, falls back to `fallback`.
/// Used for non-key attributes that may have been written with either naming convention.
#[cfg(feature = "dynamodb")]
fn get_attr<'a>(
    item: &'a HashMap<String, AttributeValue>,
    primary: &str,
    fallback: &str,
) -> Option<&'a AttributeValue> {
    item.get(primary).or_else(|| item.get(fallback))
}

/// Configuration for DynamoDB task storage.
#[derive(Debug, Clone)]
pub struct DynamoDbTaskConfig {
    /// DynamoDB table name for tasks.
    pub table_name: String,
    /// AWS region.
    pub region: String,
    /// Task TTL in minutes (for DynamoDB native TTL).
    pub task_ttl_minutes: u64,
    /// Verify table existence and schema at startup.
    /// When false, tables are assumed to exist with CamelCase naming convention.
    pub verify_tables: bool,
    /// Create tables if they don't exist during verification.
    /// Only has effect when `verify_tables` is true.
    pub create_tables: bool,
    /// Maximum number of tasks (0 = unlimited).
    pub max_tasks: usize,
    /// Default page size for list operations.
    pub default_page_size: u32,
}

impl Default for DynamoDbTaskConfig {
    fn default() -> Self {
        Self {
            table_name: "mcp-tasks".to_string(),
            region: std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
            task_ttl_minutes: 60,
            verify_tables: false,
            create_tables: false,
            max_tasks: 10_000,
            default_page_size: 50,
        }
    }
}

/// DynamoDB-backed task storage implementation.
///
/// Uses a single table with two GSIs (SessionIndex, StatusIndex) and
/// DynamoDB native TTL for automatic task expiry.
///
/// Naming convention (camelCase vs snake_case) is auto-detected from the
/// table's key schema at initialization time.
pub struct DynamoDbTaskStorage {
    config: DynamoDbTaskConfig,
    #[cfg(feature = "dynamodb")]
    client: Client,
    #[cfg(feature = "dynamodb")]
    naming: NamingConvention,
}

#[cfg(feature = "dynamodb")]
impl DynamoDbTaskStorage {
    /// Returns the attribute names for this storage's detected naming convention.
    fn attrs(&self) -> &'static TaskAttrNames {
        TaskAttrNames::for_convention(self.naming)
    }
}

fn status_to_str(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Working => "working",
        TaskStatus::InputRequired => "input_required",
        TaskStatus::Completed => "completed",
        TaskStatus::Failed => "failed",
        TaskStatus::Cancelled => "cancelled",
    }
}

fn str_to_status(s: &str) -> Result<TaskStatus, TaskStorageError> {
    match s {
        "working" => Ok(TaskStatus::Working),
        "input_required" => Ok(TaskStatus::InputRequired),
        "completed" => Ok(TaskStatus::Completed),
        "failed" => Ok(TaskStatus::Failed),
        "cancelled" => Ok(TaskStatus::Cancelled),
        other => Err(TaskStorageError::SerializationError(format!(
            "Unknown task status: {}",
            other
        ))),
    }
}

#[cfg(feature = "dynamodb")]
fn task_record_to_item(
    record: &TaskRecord,
    config: &DynamoDbTaskConfig,
    attrs: &TaskAttrNames,
) -> HashMap<String, AttributeValue> {
    let mut item = HashMap::new();

    item.insert(
        attrs.task_id.to_string(),
        AttributeValue::S(record.task_id.clone()),
    );
    if let Some(ref sid) = record.session_id {
        item.insert(attrs.session_id.to_string(), AttributeValue::S(sid.clone()));
    }
    item.insert(
        "status".to_string(),
        AttributeValue::S(status_to_str(record.status).to_string()),
    );
    if let Some(ref msg) = record.status_message {
        item.insert(
            attrs.status_message.to_string(),
            AttributeValue::S(msg.clone()),
        );
    }
    item.insert(
        attrs.created_at.to_string(),
        AttributeValue::S(record.created_at.clone()),
    );
    item.insert(
        attrs.last_updated_at.to_string(),
        AttributeValue::S(record.last_updated_at.clone()),
    );
    if let Some(ttl) = record.ttl {
        item.insert("ttl".to_string(), AttributeValue::N(ttl.to_string()));
    }
    if let Some(interval) = record.poll_interval {
        item.insert(
            attrs.poll_interval.to_string(),
            AttributeValue::N(interval.to_string()),
        );
    }
    item.insert(
        attrs.original_method.to_string(),
        AttributeValue::S(record.original_method.clone()),
    );
    if let Some(ref params) = record.original_params
        && let Ok(json_str) = serde_json::to_string(params)
    {
        item.insert(
            attrs.original_params.to_string(),
            AttributeValue::S(json_str),
        );
    }
    if let Some(ref result) = record.result
        && let Ok(json_str) = serde_json::to_string(result)
    {
        item.insert("result".to_string(), AttributeValue::S(json_str));
    }
    if let Some(ref meta) = record.meta
        && let Ok(json_str) = serde_json::to_string(meta)
    {
        item.insert("meta".to_string(), AttributeValue::S(json_str));
    }

    // Compute ttl_epoch for DynamoDB native TTL
    if let Some(ttl_ms) = record.ttl {
        if let Ok(created) = chrono::DateTime::parse_from_rfc3339(&record.created_at) {
            let epoch_secs = created.timestamp() + ttl_ms / 1000;
            item.insert(
                attrs.ttl_epoch.to_string(),
                AttributeValue::N(epoch_secs.to_string()),
            );
        }
    } else {
        // Default TTL from config
        if let Ok(created) = chrono::DateTime::parse_from_rfc3339(&record.created_at) {
            let epoch_secs = created.timestamp() + (config.task_ttl_minutes * 60) as i64;
            item.insert(
                attrs.ttl_epoch.to_string(),
                AttributeValue::N(epoch_secs.to_string()),
            );
        }
    }

    item
}

#[cfg(feature = "dynamodb")]
fn item_to_task_record(
    item: &HashMap<String, AttributeValue>,
    attrs: &TaskAttrNames,
) -> Result<TaskRecord, TaskStorageError> {
    // Key attributes: use primary name first, fallback to opposite convention
    let task_id = get_attr(item, attrs.task_id, TaskAttrNames::SNAKE.task_id)
        .or_else(|| get_attr(item, TaskAttrNames::CAMEL.task_id, ""))
        .and_then(|v| v.as_s().ok())
        .ok_or_else(|| TaskStorageError::SerializationError("Missing task_id".to_string()))?
        .clone();

    let session_id = get_attr(item, attrs.session_id, TaskAttrNames::SNAKE.session_id)
        .or_else(|| get_attr(item, TaskAttrNames::CAMEL.session_id, ""))
        .and_then(|v| v.as_s().ok())
        .cloned();

    let status_str = item
        .get("status")
        .and_then(|v| v.as_s().ok())
        .ok_or_else(|| TaskStorageError::SerializationError("Missing status".to_string()))?;
    let status = str_to_status(status_str)?;

    let status_message = get_attr(
        item,
        attrs.status_message,
        TaskAttrNames::SNAKE.status_message,
    )
    .or_else(|| get_attr(item, TaskAttrNames::CAMEL.status_message, ""))
    .and_then(|v| v.as_s().ok())
    .cloned();

    let created_at = get_attr(item, attrs.created_at, TaskAttrNames::SNAKE.created_at)
        .or_else(|| get_attr(item, TaskAttrNames::CAMEL.created_at, ""))
        .and_then(|v| v.as_s().ok())
        .ok_or_else(|| TaskStorageError::SerializationError("Missing created_at".to_string()))?
        .clone();

    let last_updated_at = get_attr(
        item,
        attrs.last_updated_at,
        TaskAttrNames::SNAKE.last_updated_at,
    )
    .or_else(|| get_attr(item, TaskAttrNames::CAMEL.last_updated_at, ""))
    .and_then(|v| v.as_s().ok())
    .ok_or_else(|| TaskStorageError::SerializationError("Missing last_updated_at".to_string()))?
    .clone();

    let ttl = item
        .get("ttl")
        .and_then(|v| v.as_n().ok())
        .and_then(|n| n.parse::<i64>().ok());

    let poll_interval = get_attr(
        item,
        attrs.poll_interval,
        TaskAttrNames::SNAKE.poll_interval,
    )
    .or_else(|| get_attr(item, TaskAttrNames::CAMEL.poll_interval, ""))
    .and_then(|v| v.as_n().ok())
    .and_then(|n| n.parse::<u64>().ok());

    let original_method = get_attr(
        item,
        attrs.original_method,
        TaskAttrNames::SNAKE.original_method,
    )
    .or_else(|| get_attr(item, TaskAttrNames::CAMEL.original_method, ""))
    .and_then(|v| v.as_s().ok())
    .ok_or_else(|| TaskStorageError::SerializationError("Missing original_method".to_string()))?
    .clone();

    let original_params = get_attr(
        item,
        attrs.original_params,
        TaskAttrNames::SNAKE.original_params,
    )
    .or_else(|| get_attr(item, TaskAttrNames::CAMEL.original_params, ""))
    .and_then(|v| v.as_s().ok())
    .map(|s| serde_json::from_str(s))
    .transpose()
    .map_err(|e| TaskStorageError::SerializationError(e.to_string()))?;

    let result = item
        .get("result")
        .and_then(|v| v.as_s().ok())
        .map(|s| serde_json::from_str::<TaskOutcome>(s))
        .transpose()
        .map_err(|e| TaskStorageError::SerializationError(e.to_string()))?;

    let meta = item
        .get("meta")
        .and_then(|v| v.as_s().ok())
        .map(|s| serde_json::from_str(s))
        .transpose()
        .map_err(|e| TaskStorageError::SerializationError(e.to_string()))?;

    Ok(TaskRecord {
        task_id,
        session_id,
        status,
        status_message,
        created_at,
        last_updated_at,
        ttl,
        poll_interval,
        original_method,
        original_params,
        result,
        meta,
    })
}

impl DynamoDbTaskStorage {
    /// Create a new DynamoDB task storage with default configuration.
    pub async fn new() -> Result<Self, TaskStorageError> {
        Self::with_config(DynamoDbTaskConfig::default()).await
    }

    /// Create a new DynamoDB task storage with custom configuration.
    ///
    /// Auto-detects the table's naming convention (camelCase vs snake_case)
    /// from its key schema. New tables are created with camelCase.
    pub async fn with_config(config: DynamoDbTaskConfig) -> Result<Self, TaskStorageError> {
        info!(
            "Initializing DynamoDB task storage with table: {} in region: {}",
            config.table_name, config.region
        );

        #[cfg(feature = "dynamodb")]
        {
            let aws_config = aws_config::defaults(BehaviorVersion::latest())
                .region(Region::new(config.region.clone()))
                .load()
                .await;

            let client = Client::new(&aws_config);

            // Default to CamelCase; verify_table_schema will detect and override
            let mut storage = Self {
                config: config.clone(),
                client,
                naming: NamingConvention::CamelCase,
            };

            if config.verify_tables {
                storage.verify_table_schema().await?;
            } else {
                info!(
                    "Skipping table verification (verify_tables=false), using {:?} naming",
                    storage.naming
                );
            }

            info!(
                "DynamoDB task storage initialized successfully in region: {} (naming: {:?})",
                config.region, storage.naming
            );
            Ok(storage)
        }

        #[cfg(not(feature = "dynamodb"))]
        {
            error!("DynamoDB feature is not enabled");
            Err(TaskStorageError::Generic(
                "DynamoDB feature is not enabled".to_string(),
            ))
        }
    }

    /// Verify that the DynamoDB table exists and has the correct schema.
    /// Also detects and stores the table's naming convention.
    #[cfg(feature = "dynamodb")]
    async fn verify_table_schema(&mut self) -> Result<(), TaskStorageError> {
        use aws_sdk_dynamodb::types::TableStatus;

        debug!("Verifying table schema for: {}", self.config.table_name);

        match self
            .client
            .describe_table()
            .table_name(&self.config.table_name)
            .send()
            .await
        {
            Ok(output) => {
                if let Some(table) = output.table() {
                    // Detect naming convention from key schema
                    self.naming = detect_naming_convention(table);
                    info!(
                        "Detected naming convention for table '{}': {:?}",
                        self.config.table_name, self.naming
                    );

                    if let Some(status) = table.table_status() {
                        match status {
                            TableStatus::Active => {
                                info!(
                                    "DynamoDB table '{}' is active and ready",
                                    self.config.table_name
                                );
                                self.ensure_ttl_enabled().await?;
                                Ok(())
                            }
                            _ => {
                                warn!(
                                    "DynamoDB table '{}' is not active: {:?}",
                                    self.config.table_name, status
                                );
                                self.wait_for_table_active().await
                            }
                        }
                    } else {
                        Err(TaskStorageError::DatabaseError(format!(
                            "Table '{}' status unknown",
                            self.config.table_name
                        )))
                    }
                } else {
                    Err(TaskStorageError::DatabaseError(format!(
                        "Table '{}' description not found",
                        self.config.table_name
                    )))
                }
            }
            Err(_err) => {
                if self.config.create_tables {
                    warn!(
                        "Table '{}' does not exist, attempting to create it",
                        self.config.table_name
                    );
                    // New tables always use CamelCase
                    self.naming = NamingConvention::CamelCase;
                    self.create_table().await?;
                    self.wait_for_table_active().await?;
                    self.enable_ttl().await?;
                    Ok(())
                } else {
                    let error_msg = format!(
                        "Table '{}' does not exist and create_tables is false.",
                        self.config.table_name
                    );
                    error!("{}", error_msg);
                    Err(TaskStorageError::DatabaseError(error_msg))
                }
            }
        }
    }

    /// Create the DynamoDB table with camelCase key schema and GSIs.
    #[cfg(feature = "dynamodb")]
    async fn create_table(&self) -> Result<(), TaskStorageError> {
        use aws_sdk_dynamodb::types::{
            AttributeDefinition, BillingMode, GlobalSecondaryIndex, KeySchemaElement, KeyType,
            Projection, ProjectionType, ScalarAttributeType,
        };

        let attrs = self.attrs();
        info!(
            "Creating DynamoDB table: {} (naming: {:?})",
            self.config.table_name, self.naming
        );

        let key_schema = vec![
            KeySchemaElement::builder()
                .attribute_name(attrs.task_id)
                .key_type(KeyType::Hash)
                .build()
                .map_err(|e| TaskStorageError::DatabaseError(e.to_string()))?,
        ];

        let attribute_definitions = vec![
            AttributeDefinition::builder()
                .attribute_name(attrs.task_id)
                .attribute_type(ScalarAttributeType::S)
                .build()
                .map_err(|e| TaskStorageError::DatabaseError(e.to_string()))?,
            AttributeDefinition::builder()
                .attribute_name(attrs.session_id)
                .attribute_type(ScalarAttributeType::S)
                .build()
                .map_err(|e| TaskStorageError::DatabaseError(e.to_string()))?,
            AttributeDefinition::builder()
                .attribute_name(attrs.created_at)
                .attribute_type(ScalarAttributeType::S)
                .build()
                .map_err(|e| TaskStorageError::DatabaseError(e.to_string()))?,
            AttributeDefinition::builder()
                .attribute_name("status")
                .attribute_type(ScalarAttributeType::S)
                .build()
                .map_err(|e| TaskStorageError::DatabaseError(e.to_string()))?,
        ];

        // GSI: SessionIndex — PK: sessionId, SK: createdAt
        let session_index = GlobalSecondaryIndex::builder()
            .index_name("SessionIndex")
            .key_schema(
                KeySchemaElement::builder()
                    .attribute_name(attrs.session_id)
                    .key_type(KeyType::Hash)
                    .build()
                    .map_err(|e| TaskStorageError::DatabaseError(e.to_string()))?,
            )
            .key_schema(
                KeySchemaElement::builder()
                    .attribute_name(attrs.created_at)
                    .key_type(KeyType::Range)
                    .build()
                    .map_err(|e| TaskStorageError::DatabaseError(e.to_string()))?,
            )
            .projection(
                Projection::builder()
                    .projection_type(ProjectionType::All)
                    .build(),
            )
            .build()
            .map_err(|e| TaskStorageError::DatabaseError(e.to_string()))?;

        // GSI: StatusIndex — PK: status, SK: createdAt
        let status_index = GlobalSecondaryIndex::builder()
            .index_name("StatusIndex")
            .key_schema(
                KeySchemaElement::builder()
                    .attribute_name("status")
                    .key_type(KeyType::Hash)
                    .build()
                    .map_err(|e| TaskStorageError::DatabaseError(e.to_string()))?,
            )
            .key_schema(
                KeySchemaElement::builder()
                    .attribute_name(attrs.created_at)
                    .key_type(KeyType::Range)
                    .build()
                    .map_err(|e| TaskStorageError::DatabaseError(e.to_string()))?,
            )
            .projection(
                Projection::builder()
                    .projection_type(ProjectionType::All)
                    .build(),
            )
            .build()
            .map_err(|e| TaskStorageError::DatabaseError(e.to_string()))?;

        match self
            .client
            .create_table()
            .table_name(&self.config.table_name)
            .set_key_schema(Some(key_schema))
            .set_attribute_definitions(Some(attribute_definitions))
            .billing_mode(BillingMode::PayPerRequest)
            .set_global_secondary_indexes(Some(vec![session_index, status_index]))
            .send()
            .await
        {
            Ok(_) => {
                info!(
                    "Successfully initiated table creation: {}",
                    self.config.table_name
                );
                Ok(())
            }
            Err(err) => {
                error!(
                    "Failed to create table '{}': {}",
                    self.config.table_name, err
                );
                Err(TaskStorageError::DatabaseError(format!(
                    "Failed to create table '{}': {}",
                    self.config.table_name, err
                )))
            }
        }
    }

    /// Ensure TTL is enabled on the table, enabling it if necessary.
    #[cfg(feature = "dynamodb")]
    async fn ensure_ttl_enabled(&self) -> Result<(), TaskStorageError> {
        info!(
            "Checking TTL status on DynamoDB table: {}",
            self.config.table_name
        );

        match self
            .client
            .describe_time_to_live()
            .table_name(&self.config.table_name)
            .send()
            .await
        {
            Ok(output) => {
                if let Some(ttl_description) = output.time_to_live_description() {
                    match ttl_description.time_to_live_status() {
                        Some(aws_sdk_dynamodb::types::TimeToLiveStatus::Enabled) => {
                            info!(
                                "TTL is already enabled on table: {}",
                                self.config.table_name
                            );
                            return Ok(());
                        }
                        Some(aws_sdk_dynamodb::types::TimeToLiveStatus::Enabling) => {
                            info!(
                                "TTL is currently being enabled on table: {}",
                                self.config.table_name
                            );
                            return Ok(());
                        }
                        Some(status) => {
                            info!(
                                "TTL status is {:?}, will enable it on table: {}",
                                status, self.config.table_name
                            );
                        }
                        None => {
                            info!(
                                "TTL status unknown, will enable it on table: {}",
                                self.config.table_name
                            );
                        }
                    }
                } else {
                    info!(
                        "No TTL description found, will enable TTL on table: {}",
                        self.config.table_name
                    );
                }

                // TTL is not enabled, so enable it
                self.enable_ttl().await
            }
            Err(err) => {
                warn!(
                    "Failed to describe TTL for table '{}': {}, attempting to enable",
                    self.config.table_name, err
                );
                // If we can't describe TTL, just try to enable it
                self.enable_ttl().await
            }
        }
    }

    /// Enable TTL on the `ttlEpoch` (or `ttl_epoch` for legacy) attribute.
    #[cfg(feature = "dynamodb")]
    async fn enable_ttl(&self) -> Result<(), TaskStorageError> {
        use aws_sdk_dynamodb::types::TimeToLiveSpecification;

        let attrs = self.attrs();
        info!(
            "Enabling TTL on DynamoDB table: {} (attribute: {})",
            self.config.table_name, attrs.ttl_epoch
        );

        let ttl_spec = TimeToLiveSpecification::builder()
            .attribute_name(attrs.ttl_epoch)
            .enabled(true)
            .build()
            .map_err(|e| TaskStorageError::DatabaseError(e.to_string()))?;

        match self
            .client
            .update_time_to_live()
            .table_name(&self.config.table_name)
            .time_to_live_specification(ttl_spec)
            .send()
            .await
        {
            Ok(_) => {
                info!(
                    "Successfully enabled TTL on table: {}",
                    self.config.table_name
                );
                Ok(())
            }
            Err(err) => {
                error!(
                    "Failed to enable TTL on table '{}': {}",
                    self.config.table_name, err
                );
                Err(TaskStorageError::DatabaseError(format!(
                    "Failed to enable TTL on table '{}': {}",
                    self.config.table_name, err
                )))
            }
        }
    }

    /// Wait for the table to become active (10s intervals, 5 min max).
    #[cfg(feature = "dynamodb")]
    async fn wait_for_table_active(&self) -> Result<(), TaskStorageError> {
        use aws_sdk_dynamodb::types::TableStatus;
        use tokio::time::{Duration, sleep};

        info!(
            "Waiting for table '{}' to become active...",
            self.config.table_name
        );

        for attempt in 1..=30 {
            match self
                .client
                .describe_table()
                .table_name(&self.config.table_name)
                .send()
                .await
            {
                Ok(output) => {
                    if let Some(table) = output.table()
                        && let Some(TableStatus::Active) = table.table_status()
                    {
                        info!("Table '{}' is now active", self.config.table_name);
                        return Ok(());
                    }
                }
                Err(err) => {
                    warn!(
                        "Error checking table status on attempt {}: {}",
                        attempt, err
                    );
                }
            }

            debug!("Table not ready, waiting... (attempt {}/30)", attempt);
            sleep(Duration::from_secs(10)).await;
        }

        Err(TaskStorageError::DatabaseError(format!(
            "Table '{}' did not become active within 5 minutes",
            self.config.table_name
        )))
    }

    fn now_iso8601() -> String {
        Utc::now().to_rfc3339()
    }
}

#[async_trait]
impl TaskStorage for DynamoDbTaskStorage {
    fn backend_name(&self) -> &'static str {
        "dynamodb"
    }

    async fn create_task(&self, mut task: TaskRecord) -> Result<TaskRecord, TaskStorageError> {
        #[cfg(feature = "dynamodb")]
        {
            // Check max_tasks limit
            if self.config.max_tasks > 0 {
                let count = self.task_count().await?;
                if count >= self.config.max_tasks {
                    return Err(TaskStorageError::MaxTasksReached(self.config.max_tasks));
                }
            }

            // Ensure timestamps are set
            if task.created_at.is_empty() {
                task.created_at = Self::now_iso8601();
            }
            if task.last_updated_at.is_empty() {
                task.last_updated_at = task.created_at.clone();
            }

            let attrs = self.attrs();
            let item = task_record_to_item(&task, &self.config, attrs);

            match self
                .client
                .put_item()
                .table_name(&self.config.table_name)
                .set_item(Some(item))
                .condition_expression(format!("attribute_not_exists({})", attrs.task_id))
                .send()
                .await
            {
                Ok(_) => {
                    debug!("Successfully created task in DynamoDB: {}", task.task_id);
                    Ok(task)
                }
                Err(err) => {
                    let err_str = err.to_string();
                    if err_str.contains("ConditionalCheckFailed") {
                        Err(TaskStorageError::ConcurrentModification(format!(
                            "Task '{}' already exists",
                            task.task_id
                        )))
                    } else {
                        error!("Failed to create task in DynamoDB: {}", err);
                        Err(TaskStorageError::DatabaseError(format!(
                            "Failed to create task: {}",
                            err
                        )))
                    }
                }
            }
        }

        #[cfg(not(feature = "dynamodb"))]
        {
            let _ = task;
            Err(TaskStorageError::Generic(
                "DynamoDB feature is not enabled".to_string(),
            ))
        }
    }

    async fn get_task(&self, task_id: &str) -> Result<Option<TaskRecord>, TaskStorageError> {
        #[cfg(feature = "dynamodb")]
        {
            let attrs = self.attrs();
            let key = HashMap::from([(
                attrs.task_id.to_string(),
                AttributeValue::S(task_id.to_string()),
            )]);

            match self
                .client
                .get_item()
                .table_name(&self.config.table_name)
                .set_key(Some(key))
                .consistent_read(true)
                .send()
                .await
            {
                Ok(output) => {
                    if let Some(item) = output.item() {
                        let record = item_to_task_record(item, attrs)?;
                        Ok(Some(record))
                    } else {
                        Ok(None)
                    }
                }
                Err(err) => {
                    error!("Failed to get task from DynamoDB: {}", err);
                    Err(TaskStorageError::DatabaseError(format!(
                        "Failed to get task '{}': {}",
                        task_id, err
                    )))
                }
            }
        }

        #[cfg(not(feature = "dynamodb"))]
        {
            let _ = task_id;
            Err(TaskStorageError::Generic(
                "DynamoDB feature is not enabled".to_string(),
            ))
        }
    }

    async fn update_task(&self, task: TaskRecord) -> Result<(), TaskStorageError> {
        #[cfg(feature = "dynamodb")]
        {
            let attrs = self.attrs();
            let item = task_record_to_item(&task, &self.config, attrs);

            match self
                .client
                .put_item()
                .table_name(&self.config.table_name)
                .set_item(Some(item))
                .condition_expression(format!("attribute_exists({})", attrs.task_id))
                .send()
                .await
            {
                Ok(_) => {
                    debug!("Successfully updated task in DynamoDB: {}", task.task_id);
                    Ok(())
                }
                Err(err) => {
                    let err_str = err.to_string();
                    if err_str.contains("ConditionalCheckFailed") {
                        Err(TaskStorageError::TaskNotFound(task.task_id.clone()))
                    } else {
                        error!("Failed to update task in DynamoDB: {}", err);
                        Err(TaskStorageError::DatabaseError(format!(
                            "Failed to update task '{}': {}",
                            task.task_id, err
                        )))
                    }
                }
            }
        }

        #[cfg(not(feature = "dynamodb"))]
        {
            let _ = task;
            Err(TaskStorageError::Generic(
                "DynamoDB feature is not enabled".to_string(),
            ))
        }
    }

    async fn delete_task(&self, task_id: &str) -> Result<bool, TaskStorageError> {
        #[cfg(feature = "dynamodb")]
        {
            let attrs = self.attrs();
            let key = HashMap::from([(
                attrs.task_id.to_string(),
                AttributeValue::S(task_id.to_string()),
            )]);

            match self
                .client
                .delete_item()
                .table_name(&self.config.table_name)
                .set_key(Some(key))
                .return_values(aws_sdk_dynamodb::types::ReturnValue::AllOld)
                .send()
                .await
            {
                Ok(output) => {
                    let existed = output.attributes().is_some();
                    debug!(
                        "Delete task '{}' from DynamoDB: existed={}",
                        task_id, existed
                    );
                    Ok(existed)
                }
                Err(err) => {
                    error!("Failed to delete task from DynamoDB: {}", err);
                    Err(TaskStorageError::DatabaseError(format!(
                        "Failed to delete task '{}': {}",
                        task_id, err
                    )))
                }
            }
        }

        #[cfg(not(feature = "dynamodb"))]
        {
            let _ = task_id;
            Err(TaskStorageError::Generic(
                "DynamoDB feature is not enabled".to_string(),
            ))
        }
    }

    async fn list_tasks(
        &self,
        cursor: Option<&str>,
        limit: Option<u32>,
    ) -> Result<TaskListPage, TaskStorageError> {
        #[cfg(feature = "dynamodb")]
        {
            let attrs = self.attrs();
            let limit = limit.unwrap_or(self.config.default_page_size);

            // Decode cursor from base64 to DynamoDB ExclusiveStartKey
            let exclusive_start_key: Option<HashMap<String, AttributeValue>> =
                if let Some(cursor_str) = cursor {
                    let decoded = base64::engine::general_purpose::STANDARD
                        .decode(cursor_str)
                        .map_err(|e| {
                            TaskStorageError::SerializationError(format!("Invalid cursor: {}", e))
                        })?;
                    let json_str = String::from_utf8(decoded).map_err(|e| {
                        TaskStorageError::SerializationError(format!(
                            "Invalid cursor encoding: {}",
                            e
                        ))
                    })?;
                    let key_map: HashMap<String, String> = serde_json::from_str(&json_str)
                        .map_err(|e| {
                            TaskStorageError::SerializationError(format!(
                                "Invalid cursor JSON: {}",
                                e
                            ))
                        })?;
                    let mut av_map = HashMap::new();
                    for (k, v) in key_map {
                        av_map.insert(k, AttributeValue::S(v));
                    }
                    Some(av_map)
                } else {
                    None
                };

            // Use Scan — DynamoDB does not support ordered Scan without GSI.
            // We over-fetch and sort in Rust for best-effort ordering.
            let mut builder = self
                .client
                .scan()
                .table_name(&self.config.table_name)
                .limit(limit as i32);

            if let Some(start_key) = exclusive_start_key {
                builder = builder.set_exclusive_start_key(Some(start_key));
            }

            match builder.send().await {
                Ok(output) => {
                    let mut records: Vec<TaskRecord> = Vec::new();
                    for item in output.items() {
                        match item_to_task_record(item, attrs) {
                            Ok(record) => records.push(record),
                            Err(e) => {
                                warn!("Skipping malformed task record: {}", e);
                            }
                        }
                    }

                    // Sort by (created_at, task_id) for best-effort determinism
                    records.sort_by(|a, b| {
                        a.created_at
                            .cmp(&b.created_at)
                            .then_with(|| a.task_id.cmp(&b.task_id))
                    });

                    // Encode next cursor from LastEvaluatedKey
                    let next_cursor = output.last_evaluated_key().map(|key| {
                        let mut key_map = HashMap::new();
                        for (k, v) in key {
                            if let Ok(s) = v.as_s() {
                                key_map.insert(k.clone(), s.clone());
                            } else if let Ok(n) = v.as_n() {
                                key_map.insert(k.clone(), n.clone());
                            }
                        }
                        let json = serde_json::to_string(&key_map).unwrap_or_default();
                        base64::engine::general_purpose::STANDARD.encode(json)
                    });

                    Ok(TaskListPage {
                        tasks: records,
                        next_cursor,
                    })
                }
                Err(err) => {
                    error!("Failed to scan tasks from DynamoDB: {}", err);
                    Err(TaskStorageError::DatabaseError(format!(
                        "Failed to list tasks: {}",
                        err
                    )))
                }
            }
        }

        #[cfg(not(feature = "dynamodb"))]
        {
            let _ = (cursor, limit);
            Err(TaskStorageError::Generic(
                "DynamoDB feature is not enabled".to_string(),
            ))
        }
    }

    async fn update_task_status(
        &self,
        task_id: &str,
        new_status: TaskStatus,
        status_message: Option<String>,
    ) -> Result<TaskRecord, TaskStorageError> {
        #[cfg(feature = "dynamodb")]
        {
            let task_attrs = self.attrs();

            // First, get the current task to validate the state transition
            let current = self
                .get_task(task_id)
                .await?
                .ok_or_else(|| TaskStorageError::TaskNotFound(task_id.to_string()))?;

            // Validate state machine transition
            state_machine::validate_transition(current.status, new_status)?;

            let expected_status = status_to_str(current.status).to_string();
            let new_status_str = status_to_str(new_status).to_string();
            let now = Self::now_iso8601();

            // Build update expression
            let mut update_expr = "SET #status = :new_status, #last_updated_at = :now".to_string();
            let mut expr_names = HashMap::from([
                ("#status".to_string(), "status".to_string()),
                (
                    "#last_updated_at".to_string(),
                    task_attrs.last_updated_at.to_string(),
                ),
            ]);
            let mut expr_values: HashMap<String, AttributeValue> = HashMap::from([
                (":new_status".to_string(), AttributeValue::S(new_status_str)),
                (":now".to_string(), AttributeValue::S(now.clone())),
                (
                    ":expected_status".to_string(),
                    AttributeValue::S(expected_status.clone()),
                ),
            ]);

            if let Some(ref msg) = status_message {
                update_expr.push_str(", #status_message = :msg");
                expr_names.insert(
                    "#status_message".to_string(),
                    task_attrs.status_message.to_string(),
                );
                expr_values.insert(":msg".to_string(), AttributeValue::S(msg.clone()));
            } else {
                update_expr.push_str(" REMOVE #status_message");
                expr_names.insert(
                    "#status_message".to_string(),
                    task_attrs.status_message.to_string(),
                );
            }

            let key = HashMap::from([(
                task_attrs.task_id.to_string(),
                AttributeValue::S(task_id.to_string()),
            )]);

            match self
                .client
                .update_item()
                .table_name(&self.config.table_name)
                .set_key(Some(key))
                .update_expression(&update_expr)
                .condition_expression("#status = :expected_status")
                .set_expression_attribute_names(Some(expr_names))
                .set_expression_attribute_values(Some(expr_values))
                .return_values(aws_sdk_dynamodb::types::ReturnValue::AllNew)
                .send()
                .await
            {
                Ok(output) => {
                    if let Some(returned) = output.attributes() {
                        item_to_task_record(returned, task_attrs)
                    } else {
                        // Fallback: re-read the task
                        self.get_task(task_id)
                            .await?
                            .ok_or_else(|| TaskStorageError::TaskNotFound(task_id.to_string()))
                    }
                }
                Err(err) => {
                    let err_str = err.to_string();
                    if err_str.contains("ConditionalCheckFailed") {
                        // Retry once: re-read task and check if transition is still valid
                        let refreshed = self
                            .get_task(task_id)
                            .await?
                            .ok_or_else(|| TaskStorageError::TaskNotFound(task_id.to_string()))?;

                        // Re-validate transition with fresh state
                        state_machine::validate_transition(refreshed.status, new_status)?;

                        let retry_expected = status_to_str(refreshed.status).to_string();
                        let retry_new = status_to_str(new_status).to_string();
                        let retry_now = Self::now_iso8601();

                        let mut retry_update =
                            "SET #status = :new_status, #last_updated_at = :now".to_string();
                        let mut retry_names = HashMap::from([
                            ("#status".to_string(), "status".to_string()),
                            (
                                "#last_updated_at".to_string(),
                                task_attrs.last_updated_at.to_string(),
                            ),
                        ]);
                        let mut retry_values: HashMap<String, AttributeValue> = HashMap::from([
                            (":new_status".to_string(), AttributeValue::S(retry_new)),
                            (":now".to_string(), AttributeValue::S(retry_now)),
                            (
                                ":expected_status".to_string(),
                                AttributeValue::S(retry_expected),
                            ),
                        ]);

                        if let Some(ref msg) = status_message {
                            retry_update.push_str(", #status_message = :msg");
                            retry_names.insert(
                                "#status_message".to_string(),
                                task_attrs.status_message.to_string(),
                            );
                            retry_values.insert(":msg".to_string(), AttributeValue::S(msg.clone()));
                        } else {
                            retry_update.push_str(" REMOVE #status_message");
                            retry_names.insert(
                                "#status_message".to_string(),
                                task_attrs.status_message.to_string(),
                            );
                        }

                        let retry_key = HashMap::from([(
                            task_attrs.task_id.to_string(),
                            AttributeValue::S(task_id.to_string()),
                        )]);

                        match self
                            .client
                            .update_item()
                            .table_name(&self.config.table_name)
                            .set_key(Some(retry_key))
                            .update_expression(&retry_update)
                            .condition_expression("#status = :expected_status")
                            .set_expression_attribute_names(Some(retry_names))
                            .set_expression_attribute_values(Some(retry_values))
                            .return_values(aws_sdk_dynamodb::types::ReturnValue::AllNew)
                            .send()
                            .await
                        {
                            Ok(retry_output) => {
                                if let Some(returned) = retry_output.attributes() {
                                    item_to_task_record(returned, task_attrs)
                                } else {
                                    self.get_task(task_id).await?.ok_or_else(|| {
                                        TaskStorageError::TaskNotFound(task_id.to_string())
                                    })
                                }
                            }
                            Err(_) => Err(TaskStorageError::ConcurrentModification(format!(
                                "Failed to update task '{}' status after retry",
                                task_id
                            ))),
                        }
                    } else {
                        error!("Failed to update task status in DynamoDB: {}", err);
                        Err(TaskStorageError::DatabaseError(format!(
                            "Failed to update task '{}' status: {}",
                            task_id, err
                        )))
                    }
                }
            }
        }

        #[cfg(not(feature = "dynamodb"))]
        {
            let _ = (task_id, new_status, status_message);
            Err(TaskStorageError::Generic(
                "DynamoDB feature is not enabled".to_string(),
            ))
        }
    }

    async fn store_task_result(
        &self,
        task_id: &str,
        result: TaskOutcome,
    ) -> Result<(), TaskStorageError> {
        #[cfg(feature = "dynamodb")]
        {
            let task_attrs = self.attrs();
            let result_json = serde_json::to_string(&result)?;
            let now = Self::now_iso8601();

            let key = HashMap::from([(
                task_attrs.task_id.to_string(),
                AttributeValue::S(task_id.to_string()),
            )]);

            let update_expr = "SET #result = :result, #last_updated_at = :now";
            let expr_names = HashMap::from([
                ("#result".to_string(), "result".to_string()),
                (
                    "#last_updated_at".to_string(),
                    task_attrs.last_updated_at.to_string(),
                ),
            ]);
            let expr_values = HashMap::from([
                (":result".to_string(), AttributeValue::S(result_json)),
                (":now".to_string(), AttributeValue::S(now)),
            ]);

            match self
                .client
                .update_item()
                .table_name(&self.config.table_name)
                .set_key(Some(key))
                .update_expression(update_expr)
                .condition_expression(format!("attribute_exists({})", task_attrs.task_id))
                .set_expression_attribute_names(Some(expr_names))
                .set_expression_attribute_values(Some(expr_values))
                .send()
                .await
            {
                Ok(_) => {
                    debug!("Successfully stored task result in DynamoDB: {}", task_id);
                    Ok(())
                }
                Err(err) => {
                    let err_str = err.to_string();
                    if err_str.contains("ConditionalCheckFailed") {
                        Err(TaskStorageError::TaskNotFound(task_id.to_string()))
                    } else {
                        error!("Failed to store task result in DynamoDB: {}", err);
                        Err(TaskStorageError::DatabaseError(format!(
                            "Failed to store result for task '{}': {}",
                            task_id, err
                        )))
                    }
                }
            }
        }

        #[cfg(not(feature = "dynamodb"))]
        {
            let _ = (task_id, result);
            Err(TaskStorageError::Generic(
                "DynamoDB feature is not enabled".to_string(),
            ))
        }
    }

    async fn get_task_result(
        &self,
        task_id: &str,
    ) -> Result<Option<TaskOutcome>, TaskStorageError> {
        let task = self
            .get_task(task_id)
            .await?
            .ok_or_else(|| TaskStorageError::TaskNotFound(task_id.to_string()))?;

        Ok(task.result)
    }

    async fn expire_tasks(&self) -> Result<Vec<String>, TaskStorageError> {
        #[cfg(feature = "dynamodb")]
        {
            let attrs = self.attrs();
            // DynamoDB native TTL handles most expiry automatically.
            // For immediate cleanup, query non-terminal statuses and filter by expired TTL.
            let now = Utc::now();
            let mut expired = Vec::new();

            for status_str in &["working", "input_required"] {
                let expr_values = HashMap::from([(
                    ":status".to_string(),
                    AttributeValue::S(status_str.to_string()),
                )]);

                let result = self
                    .client
                    .query()
                    .table_name(&self.config.table_name)
                    .index_name("StatusIndex")
                    .key_condition_expression("#status = :status")
                    .expression_attribute_names("#status", "status")
                    .set_expression_attribute_values(Some(expr_values))
                    .send()
                    .await;

                match result {
                    Ok(output) => {
                        for item in output.items() {
                            if let Ok(record) = item_to_task_record(item, attrs)
                                && let Some(ttl_ms) = record.ttl
                                && let Ok(created) =
                                    chrono::DateTime::parse_from_rfc3339(&record.created_at)
                            {
                                let expiry = created.with_timezone(&Utc)
                                    + chrono::Duration::milliseconds(ttl_ms);
                                if now > expiry {
                                    // Delete the expired task
                                    if self.delete_task(&record.task_id).await? {
                                        expired.push(record.task_id);
                                    }
                                }
                            }
                        }
                    }
                    Err(err) => {
                        warn!(
                            "Failed to query tasks for expiry with status '{}': {}",
                            status_str, err
                        );
                    }
                }
            }

            Ok(expired)
        }

        #[cfg(not(feature = "dynamodb"))]
        {
            Err(TaskStorageError::Generic(
                "DynamoDB feature is not enabled".to_string(),
            ))
        }
    }

    async fn task_count(&self) -> Result<usize, TaskStorageError> {
        #[cfg(feature = "dynamodb")]
        {
            match self
                .client
                .scan()
                .table_name(&self.config.table_name)
                .select(aws_sdk_dynamodb::types::Select::Count)
                .send()
                .await
            {
                Ok(output) => {
                    let count = output.count() as usize;
                    debug!("DynamoDB task count: {}", count);
                    Ok(count)
                }
                Err(err) => {
                    error!("Failed to count tasks in DynamoDB: {}", err);
                    Err(TaskStorageError::DatabaseError(format!(
                        "Failed to count tasks: {}",
                        err
                    )))
                }
            }
        }

        #[cfg(not(feature = "dynamodb"))]
        {
            Err(TaskStorageError::Generic(
                "DynamoDB feature is not enabled".to_string(),
            ))
        }
    }

    async fn maintenance(&self) -> Result<(), TaskStorageError> {
        // DynamoDB TTL handles cleanup automatically. No-op.
        debug!("DynamoDB maintenance: no-op (TTL handles cleanup)");
        Ok(())
    }

    async fn list_tasks_for_session(
        &self,
        session_id: &str,
        cursor: Option<&str>,
        limit: Option<u32>,
    ) -> Result<TaskListPage, TaskStorageError> {
        #[cfg(feature = "dynamodb")]
        {
            let attrs = self.attrs();
            let limit = limit.unwrap_or(self.config.default_page_size);

            let exclusive_start_key: Option<HashMap<String, AttributeValue>> =
                if let Some(cursor_str) = cursor {
                    let decoded = base64::engine::general_purpose::STANDARD
                        .decode(cursor_str)
                        .map_err(|e| {
                            TaskStorageError::SerializationError(format!("Invalid cursor: {}", e))
                        })?;
                    let json_str = String::from_utf8(decoded).map_err(|e| {
                        TaskStorageError::SerializationError(format!(
                            "Invalid cursor encoding: {}",
                            e
                        ))
                    })?;
                    let key_map: HashMap<String, String> = serde_json::from_str(&json_str)
                        .map_err(|e| {
                            TaskStorageError::SerializationError(format!(
                                "Invalid cursor JSON: {}",
                                e
                            ))
                        })?;
                    let mut av_map = HashMap::new();
                    for (k, v) in key_map {
                        av_map.insert(k, AttributeValue::S(v));
                    }
                    Some(av_map)
                } else {
                    None
                };

            let expr_values = HashMap::from([(
                ":session_id".to_string(),
                AttributeValue::S(session_id.to_string()),
            )]);

            let mut builder = self
                .client
                .query()
                .table_name(&self.config.table_name)
                .index_name("SessionIndex")
                .key_condition_expression(format!("{} = :session_id", attrs.session_id))
                .set_expression_attribute_values(Some(expr_values))
                .scan_index_forward(true)
                .limit(limit as i32);

            if let Some(start_key) = exclusive_start_key {
                builder = builder.set_exclusive_start_key(Some(start_key));
            }

            match builder.send().await {
                Ok(output) => {
                    let mut records: Vec<TaskRecord> = Vec::new();
                    for item in output.items() {
                        match item_to_task_record(item, attrs) {
                            Ok(record) => records.push(record),
                            Err(e) => {
                                warn!("Skipping malformed task record: {}", e);
                            }
                        }
                    }

                    // Post-query sort by task_id within same created_at for determinism
                    records.sort_by(|a, b| {
                        a.created_at
                            .cmp(&b.created_at)
                            .then_with(|| a.task_id.cmp(&b.task_id))
                    });

                    let next_cursor = output.last_evaluated_key().map(|key| {
                        let mut key_map = HashMap::new();
                        for (k, v) in key {
                            if let Ok(s) = v.as_s() {
                                key_map.insert(k.clone(), s.clone());
                            } else if let Ok(n) = v.as_n() {
                                key_map.insert(k.clone(), n.clone());
                            }
                        }
                        let json = serde_json::to_string(&key_map).unwrap_or_default();
                        base64::engine::general_purpose::STANDARD.encode(json)
                    });

                    Ok(TaskListPage {
                        tasks: records,
                        next_cursor,
                    })
                }
                Err(err) => {
                    error!(
                        "Failed to query tasks for session '{}' from DynamoDB: {}",
                        session_id, err
                    );
                    Err(TaskStorageError::DatabaseError(format!(
                        "Failed to list tasks for session '{}': {}",
                        session_id, err
                    )))
                }
            }
        }

        #[cfg(not(feature = "dynamodb"))]
        {
            let _ = (session_id, cursor, limit);
            Err(TaskStorageError::Generic(
                "DynamoDB feature is not enabled".to_string(),
            ))
        }
    }

    async fn recover_stuck_tasks(&self, max_age_ms: u64) -> Result<Vec<String>, TaskStorageError> {
        #[cfg(feature = "dynamodb")]
        {
            let task_attrs = self.attrs();
            let now = Utc::now();
            let mut recovered = Vec::new();

            // Query for "working" and "input_required" statuses via StatusIndex GSI
            for status_str in &["working", "input_required"] {
                let expr_values = HashMap::from([(
                    ":status".to_string(),
                    AttributeValue::S(status_str.to_string()),
                )]);

                let result = self
                    .client
                    .query()
                    .table_name(&self.config.table_name)
                    .index_name("StatusIndex")
                    .key_condition_expression("#status = :status")
                    .expression_attribute_names("#status", "status")
                    .set_expression_attribute_values(Some(expr_values))
                    .send()
                    .await;

                match result {
                    Ok(output) => {
                        for item in output.items() {
                            if let Ok(record) = item_to_task_record(item, task_attrs) {
                                // Check age based on last_updated_at
                                if let Ok(updated) =
                                    chrono::DateTime::parse_from_rfc3339(&record.last_updated_at)
                                {
                                    let age_ms =
                                        (now - updated.with_timezone(&Utc)).num_milliseconds();
                                    if age_ms > max_age_ms as i64 {
                                        // Mark as Failed using conditional update
                                        let key = HashMap::from([(
                                            task_attrs.task_id.to_string(),
                                            AttributeValue::S(record.task_id.clone()),
                                        )]);
                                        let update_now = Self::now_iso8601();

                                        let update_result = self
                                            .client
                                            .update_item()
                                            .table_name(&self.config.table_name)
                                            .set_key(Some(key))
                                            .update_expression(
                                                "SET #status = :failed, #last_updated_at = :now, #status_message = :msg",
                                            )
                                            .condition_expression("#status = :expected")
                                            .expression_attribute_names("#status", "status")
                                            .expression_attribute_names(
                                                "#last_updated_at",
                                                task_attrs.last_updated_at,
                                            )
                                            .expression_attribute_names(
                                                "#status_message",
                                                task_attrs.status_message,
                                            )
                                            .expression_attribute_values(
                                                ":failed",
                                                AttributeValue::S("failed".to_string()),
                                            )
                                            .expression_attribute_values(
                                                ":now",
                                                AttributeValue::S(update_now),
                                            )
                                            .expression_attribute_values(
                                                ":msg",
                                                AttributeValue::S(
                                                    "Server restarted \u{2014} task interrupted"
                                                        .to_string(),
                                                ),
                                            )
                                            .expression_attribute_values(
                                                ":expected",
                                                AttributeValue::S(status_str.to_string()),
                                            )
                                            .send()
                                            .await;

                                        match update_result {
                                            Ok(_) => {
                                                recovered.push(record.task_id.clone());
                                            }
                                            Err(err) => {
                                                warn!(
                                                    "Failed to recover stuck task '{}': {}",
                                                    record.task_id, err
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(err) => {
                        warn!(
                            "Failed to query stuck tasks with status '{}': {}",
                            status_str, err
                        );
                    }
                }
            }

            Ok(recovered)
        }

        #[cfg(not(feature = "dynamodb"))]
        {
            let _ = max_age_ms;
            Err(TaskStorageError::Generic(
                "DynamoDB feature is not enabled".to_string(),
            ))
        }
    }
}

#[cfg(all(test, feature = "dynamodb"))]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_dynamodb_config() {
        let config = DynamoDbTaskConfig::default();
        assert_eq!(config.table_name, "mcp-tasks");
        let expected_region =
            std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string());
        assert_eq!(config.region, expected_region);
        assert_eq!(config.task_ttl_minutes, 60);
        assert_eq!(config.max_tasks, 10_000);
        assert_eq!(config.default_page_size, 50);
        assert!(!config.verify_tables);
        assert!(!config.create_tables);
    }

    /// Helper to build a full test TaskRecord.
    fn full_test_record() -> TaskRecord {
        TaskRecord {
            task_id: "test-task-123".to_string(),
            session_id: Some("session-456".to_string()),
            status: TaskStatus::Working,
            status_message: Some("Processing data".to_string()),
            created_at: "2025-06-01T12:00:00+00:00".to_string(),
            last_updated_at: "2025-06-01T12:00:05+00:00".to_string(),
            ttl: Some(60_000),
            poll_interval: Some(5_000),
            original_method: "tools/call".to_string(),
            original_params: Some(json!({"tool": "calculator", "args": {"a": 1, "b": 2}})),
            result: Some(TaskOutcome::Success(json!({"value": 3}))),
            meta: Some(HashMap::from([("key".to_string(), json!("value"))])),
        }
    }

    /// Verify round-trip for a given naming convention.
    fn assert_round_trip(convention: NamingConvention) {
        let record = full_test_record();
        let config = DynamoDbTaskConfig::default();
        let attrs = TaskAttrNames::for_convention(convention);
        let item = task_record_to_item(&record, &config, attrs);
        let restored = item_to_task_record(&item, attrs).unwrap();

        assert_eq!(restored.task_id, record.task_id);
        assert_eq!(restored.session_id, record.session_id);
        assert_eq!(restored.status, record.status);
        assert_eq!(restored.status_message, record.status_message);
        assert_eq!(restored.created_at, record.created_at);
        assert_eq!(restored.last_updated_at, record.last_updated_at);
        assert_eq!(restored.ttl, record.ttl);
        assert_eq!(restored.poll_interval, record.poll_interval);
        assert_eq!(restored.original_method, record.original_method);
        assert_eq!(restored.original_params, record.original_params);
        assert!(restored.result.is_some());
        assert!(restored.meta.is_some());

        // Verify ttl_epoch uses the correct attribute name
        assert!(item.contains_key(attrs.ttl_epoch));
    }

    #[tokio::test]
    async fn test_camel_case_task_item_round_trip() {
        assert_round_trip(NamingConvention::CamelCase);
    }

    #[tokio::test]
    async fn test_snake_case_task_item_round_trip() {
        assert_round_trip(NamingConvention::SnakeCase);
    }

    #[tokio::test]
    async fn test_dynamodb_item_conversion_round_trip() {
        // Default: CamelCase (new convention)
        assert_round_trip(NamingConvention::CamelCase);
    }

    #[tokio::test]
    async fn test_dynamodb_item_conversion_minimal() {
        // Test with minimal fields (no optional fields set)
        let record = TaskRecord {
            task_id: "minimal-task".to_string(),
            session_id: None,
            status: TaskStatus::Completed,
            status_message: None,
            created_at: "2025-06-01T12:00:00+00:00".to_string(),
            last_updated_at: "2025-06-01T12:00:00+00:00".to_string(),
            ttl: None,
            poll_interval: None,
            original_method: "sampling/createMessage".to_string(),
            original_params: None,
            result: None,
            meta: None,
        };

        let config = DynamoDbTaskConfig::default();
        let attrs = TaskAttrNames::for_convention(NamingConvention::CamelCase);
        let item = task_record_to_item(&record, &config, attrs);
        let restored = item_to_task_record(&item, attrs).unwrap();

        assert_eq!(restored.task_id, "minimal-task");
        assert_eq!(restored.session_id, None);
        assert_eq!(restored.status, TaskStatus::Completed);
        assert_eq!(restored.status_message, None);
        assert_eq!(restored.original_method, "sampling/createMessage");
        assert_eq!(restored.original_params, None);
        assert!(restored.result.is_none());
        assert!(restored.meta.is_none());
    }

    #[tokio::test]
    async fn test_dynamodb_item_conversion_error_result() {
        let record = TaskRecord {
            task_id: "error-task".to_string(),
            session_id: Some("sess-1".to_string()),
            status: TaskStatus::Failed,
            status_message: Some("Tool execution failed".to_string()),
            created_at: "2025-06-01T12:00:00+00:00".to_string(),
            last_updated_at: "2025-06-01T12:00:10+00:00".to_string(),
            ttl: Some(30_000),
            poll_interval: None,
            original_method: "tools/call".to_string(),
            original_params: None,
            result: Some(TaskOutcome::Error {
                code: -32010,
                message: "Tool not found".to_string(),
                data: Some(json!({"detail": "calculator not registered"})),
            }),
            meta: None,
        };

        let config = DynamoDbTaskConfig::default();
        let attrs = TaskAttrNames::for_convention(NamingConvention::CamelCase);
        let item = task_record_to_item(&record, &config, attrs);
        let restored = item_to_task_record(&item, attrs).unwrap();

        match restored.result {
            Some(TaskOutcome::Error {
                code,
                message,
                data,
            }) => {
                assert_eq!(code, -32010);
                assert_eq!(message, "Tool not found");
                assert_eq!(data.unwrap()["detail"], "calculator not registered");
            }
            other => panic!("Expected Error outcome, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_detect_task_table_snake_case() {
        use aws_sdk_dynamodb::types::{KeySchemaElement, KeyType, TableDescription};

        let table = TableDescription::builder()
            .key_schema(
                KeySchemaElement::builder()
                    .attribute_name("task_id")
                    .key_type(KeyType::Hash)
                    .build()
                    .unwrap(),
            )
            .build();

        assert_eq!(
            detect_naming_convention(&table),
            NamingConvention::SnakeCase
        );
    }

    #[tokio::test]
    async fn test_detect_task_table_camel_case() {
        use aws_sdk_dynamodb::types::{KeySchemaElement, KeyType, TableDescription};

        let table = TableDescription::builder()
            .key_schema(
                KeySchemaElement::builder()
                    .attribute_name("taskId")
                    .key_type(KeyType::Hash)
                    .build()
                    .unwrap(),
            )
            .build();

        assert_eq!(
            detect_naming_convention(&table),
            NamingConvention::CamelCase
        );
    }

    #[tokio::test]
    async fn test_detect_single_word_key_defaults_to_camel() {
        use aws_sdk_dynamodb::types::{KeySchemaElement, KeyType, TableDescription};

        // Single-word key "status" has no underscore → defaults to CamelCase
        let table = TableDescription::builder()
            .key_schema(
                KeySchemaElement::builder()
                    .attribute_name("status")
                    .key_type(KeyType::Hash)
                    .build()
                    .unwrap(),
            )
            .build();

        assert_eq!(
            detect_naming_convention(&table),
            NamingConvention::CamelCase
        );
    }

    #[tokio::test]
    async fn test_task_read_tolerance_fallback() {
        // Write with snake_case, read with camelCase primary + fallback
        let record = full_test_record();
        let config = DynamoDbTaskConfig::default();
        let snake_attrs = TaskAttrNames::for_convention(NamingConvention::SnakeCase);
        let camel_attrs = TaskAttrNames::for_convention(NamingConvention::CamelCase);

        // Write with snake_case
        let item = task_record_to_item(&record, &config, snake_attrs);

        // Read with camelCase (will fall back to snake_case via get_attr)
        let restored = item_to_task_record(&item, camel_attrs).unwrap();

        assert_eq!(restored.task_id, record.task_id);
        assert_eq!(restored.session_id, record.session_id);
        assert_eq!(restored.status_message, record.status_message);
        assert_eq!(restored.created_at, record.created_at);
        assert_eq!(restored.last_updated_at, record.last_updated_at);
        assert_eq!(restored.poll_interval, record.poll_interval);
        assert_eq!(restored.original_method, record.original_method);
    }

    #[tokio::test]
    async fn test_dynamodb_status_helpers() {
        // Round-trip all statuses
        let statuses = vec![
            TaskStatus::Working,
            TaskStatus::InputRequired,
            TaskStatus::Completed,
            TaskStatus::Failed,
            TaskStatus::Cancelled,
        ];

        for status in statuses {
            let s = status_to_str(status);
            let restored = str_to_status(s).unwrap();
            assert_eq!(restored, status, "Round-trip failed for {:?}", status);
        }

        // Verify string representations
        assert_eq!(status_to_str(TaskStatus::Working), "working");
        assert_eq!(status_to_str(TaskStatus::InputRequired), "input_required");
        assert_eq!(status_to_str(TaskStatus::Completed), "completed");
        assert_eq!(status_to_str(TaskStatus::Failed), "failed");
        assert_eq!(status_to_str(TaskStatus::Cancelled), "cancelled");

        // Invalid status string
        let err = str_to_status("invalid").unwrap_err();
        match err {
            TaskStorageError::SerializationError(msg) => {
                assert!(msg.contains("Unknown task status"));
            }
            other => panic!("Expected SerializationError, got: {:?}", other),
        }
    }

    #[tokio::test]
    #[ignore = "Requires AWS DynamoDB connection"]
    async fn test_dynamodb_create_and_get_task() {
        let storage = DynamoDbTaskStorage::new().await.unwrap();

        let task = TaskRecord {
            task_id: uuid::Uuid::now_v7().as_simple().to_string(),
            session_id: Some("test-session".to_string()),
            status: TaskStatus::Working,
            status_message: Some("Testing".to_string()),
            created_at: Utc::now().to_rfc3339(),
            last_updated_at: Utc::now().to_rfc3339(),
            ttl: Some(60_000),
            poll_interval: Some(5_000),
            original_method: "tools/call".to_string(),
            original_params: None,
            result: None,
            meta: None,
        };

        let created = storage.create_task(task.clone()).await.unwrap();
        assert_eq!(created.task_id, task.task_id);

        let fetched = storage.get_task(&task.task_id).await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().task_id, task.task_id);

        // Cleanup
        storage.delete_task(&task.task_id).await.unwrap();
    }

    #[tokio::test]
    #[ignore = "Requires AWS DynamoDB connection"]
    async fn test_dynamodb_task_lifecycle() {
        let storage = DynamoDbTaskStorage::new().await.unwrap();

        let task = TaskRecord {
            task_id: uuid::Uuid::now_v7().as_simple().to_string(),
            session_id: None,
            status: TaskStatus::Working,
            status_message: None,
            created_at: Utc::now().to_rfc3339(),
            last_updated_at: Utc::now().to_rfc3339(),
            ttl: None,
            poll_interval: None,
            original_method: "tools/call".to_string(),
            original_params: None,
            result: None,
            meta: None,
        };

        storage.create_task(task.clone()).await.unwrap();

        // Working -> Completed
        let updated = storage
            .update_task_status(
                &task.task_id,
                TaskStatus::Completed,
                Some("Done".to_string()),
            )
            .await
            .unwrap();
        assert_eq!(updated.status, TaskStatus::Completed);

        // Completed -> Working should fail (terminal state)
        let result = storage
            .update_task_status(&task.task_id, TaskStatus::Working, None)
            .await;
        assert!(result.is_err());

        // Cleanup
        storage.delete_task(&task.task_id).await.unwrap();
    }

    // === Parity tests (all require AWS DynamoDB) ===

    #[tokio::test]
    #[ignore = "Requires AWS DynamoDB connection"]
    async fn parity_create_and_retrieve() {
        let storage = DynamoDbTaskStorage::new().await.unwrap();
        crate::parity_tests::test_create_and_retrieve(&storage).await;
    }

    #[tokio::test]
    #[ignore = "Requires AWS DynamoDB connection"]
    async fn parity_state_machine_enforcement() {
        let storage = DynamoDbTaskStorage::new().await.unwrap();
        crate::parity_tests::test_state_machine_enforcement(&storage).await;
    }

    #[tokio::test]
    #[ignore = "Requires AWS DynamoDB connection"]
    async fn parity_terminal_state_rejection() {
        let storage = DynamoDbTaskStorage::new().await.unwrap();
        crate::parity_tests::test_terminal_state_rejection(&storage).await;
    }

    #[tokio::test]
    #[ignore = "Requires AWS DynamoDB connection"]
    async fn parity_cursor_determinism() {
        // Note: DynamoDB parity cursor test uses list_tasks_for_session (deterministic).
        // Global list_tasks is best-effort ordered and not tested for cross-page determinism.
        let storage = DynamoDbTaskStorage::new().await.unwrap();
        crate::parity_tests::test_cursor_determinism(&storage).await;
    }

    #[tokio::test]
    #[ignore = "Requires AWS DynamoDB connection"]
    async fn parity_session_scoping() {
        let storage = DynamoDbTaskStorage::new().await.unwrap();
        crate::parity_tests::test_session_scoping(&storage).await;
    }

    #[tokio::test]
    #[ignore = "Requires AWS DynamoDB connection"]
    async fn parity_ttl_expiry() {
        let storage = DynamoDbTaskStorage::new().await.unwrap();
        crate::parity_tests::test_ttl_expiry(&storage).await;
    }

    #[tokio::test]
    #[ignore = "Requires AWS DynamoDB connection"]
    async fn parity_task_result_round_trip() {
        let storage = DynamoDbTaskStorage::new().await.unwrap();
        crate::parity_tests::test_task_result_round_trip(&storage).await;
    }

    #[tokio::test]
    #[ignore = "Requires AWS DynamoDB connection"]
    async fn parity_recover_stuck_tasks() {
        let storage = DynamoDbTaskStorage::new().await.unwrap();
        crate::parity_tests::test_recover_stuck_tasks(&storage).await;
    }

    #[tokio::test]
    #[ignore = "Requires AWS DynamoDB connection"]
    async fn parity_max_tasks_limit() {
        let config = DynamoDbTaskConfig {
            max_tasks: 5,
            ..DynamoDbTaskConfig::default()
        };
        let storage = DynamoDbTaskStorage::with_config(config).await.unwrap();
        crate::parity_tests::test_max_tasks_limit(&storage, 5).await;
    }

    #[tokio::test]
    #[ignore = "Requires AWS DynamoDB connection"]
    async fn parity_error_mapping() {
        let storage = DynamoDbTaskStorage::new().await.unwrap();
        crate::parity_tests::test_error_mapping_parity(&storage).await;
    }

    #[tokio::test]
    #[ignore = "Requires AWS DynamoDB connection"]
    async fn parity_concurrent_status_updates() {
        let storage = std::sync::Arc::new(DynamoDbTaskStorage::new().await.unwrap());
        crate::parity_tests::test_concurrent_status_updates(storage).await;
    }
}
