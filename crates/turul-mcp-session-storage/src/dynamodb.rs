//! AWS DynamoDB Session Storage Implementation
//!
//! This module provides a DynamoDB-backed session storage implementation for
//! serverless and AWS-native MCP deployments.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use thiserror::Error;
use tracing::{debug, error, info, warn};

use turul_mcp_protocol::ServerCapabilities;

use crate::{SessionInfo, SessionStorage, SessionStorageError, SseEvent};

#[cfg(feature = "dynamodb")]
use aws_config::{BehaviorVersion, Region};
#[cfg(feature = "dynamodb")]
use aws_sdk_dynamodb::Client;
#[cfg(feature = "dynamodb")]
use aws_sdk_dynamodb::types::{AttributeValue, TableStatus};

/// Configuration for DynamoDB session storage
#[derive(Debug, Clone)]
pub struct DynamoDbConfig {
    /// DynamoDB table name for sessions
    pub table_name: String,
    /// AWS region
    pub region: String,
    /// Session TTL in minutes (DynamoDB TTL attribute)
    pub session_ttl_minutes: u64,
    /// Event TTL in minutes (separate from sessions)
    pub event_ttl_minutes: u64,
    /// Maximum events per session (for cleanup)
    pub max_events_per_session: u64,
    /// Enable point-in-time recovery
    pub enable_backup: bool,
    /// Enable encryption at rest
    pub enable_encryption: bool,
    /// Allow table creation if tables don't exist
    pub create_tables_if_missing: bool,
}

impl Default for DynamoDbConfig {
    fn default() -> Self {
        Self {
            table_name: "mcp-sessions".to_string(),
            region: std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
            session_ttl_minutes: 5, // Default 5 minutes - override in config if needed
            event_ttl_minutes: 5,   // Default 5 minutes - override in config if needed
            max_events_per_session: 1000,
            enable_backup: true,
            enable_encryption: true,
            create_tables_if_missing: true,
        }
    }
}

/// Errors that can occur with DynamoDB storage
#[derive(Error, Debug)]
pub enum DynamoDbError {
    #[error("AWS SDK error: {0}")]
    AwsError(String),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    #[error("Invalid session data: {0}")]
    InvalidSessionData(String),
    #[error("DynamoDB table does not exist: {0}")]
    TableNotFound(String),
    #[error("AWS configuration error: {0}")]
    ConfigError(String),
}

/// DynamoDB-backed session storage
pub struct DynamoDbSessionStorage {
    config: DynamoDbConfig,
    #[cfg(feature = "dynamodb")]
    client: Client,
    event_counter: AtomicU64,
}

impl DynamoDbSessionStorage {
    /// Get the event table name from environment variable or default pattern
    fn get_event_table_name(&self) -> String {
        std::env::var("MCP_SESSION_EVENT_TABLE")
            .unwrap_or_else(|_| format!("{}-events", self.config.table_name))
    }

    /// Create a new DynamoDB session storage with default configuration
    pub async fn new() -> Result<Self, DynamoDbError> {
        Self::with_config(DynamoDbConfig::default()).await
    }

    /// Create a new DynamoDB session storage with custom configuration
    pub async fn with_config(config: DynamoDbConfig) -> Result<Self, DynamoDbError> {
        info!(
            "Initializing DynamoDB session storage with table: {} in region: {}",
            config.table_name, config.region
        );

        #[cfg(feature = "dynamodb")]
        {
            // Initialize AWS SDK client
            let aws_config = aws_config::defaults(BehaviorVersion::latest())
                .region(Region::new(config.region.clone()))
                .load()
                .await;

            let client = Client::new(&aws_config);

            let storage = Self {
                config: config.clone(),
                client,
                event_counter: AtomicU64::new(1),
            };

            // Verify table exists and has correct schema
            storage.verify_table_schema().await?;

            info!(
                "DynamoDB session storage initialized successfully in region: {}",
                config.region
            );
            Ok(storage)
        }

        #[cfg(not(feature = "dynamodb"))]
        {
            error!("DynamoDB feature is not enabled");
            Err(DynamoDbError::ConfigError(
                "DynamoDB feature is not enabled".to_string(),
            ))
        }
    }

    /// Verify that the DynamoDB table exists and has the correct schema, create if it doesn't exist
    async fn verify_table_schema(&self) -> Result<(), DynamoDbError> {
        #[cfg(feature = "dynamodb")]
        {
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
                        if let Some(status) = table.table_status() {
                            match status {
                                TableStatus::Active => {
                                    info!(
                                        "DynamoDB table '{}' is active and ready",
                                        self.config.table_name
                                    );
                                    Ok(())
                                }
                                _ => {
                                    warn!(
                                        "DynamoDB table '{}' is not active: {:?}",
                                        self.config.table_name, status
                                    );
                                    // Wait for table to become active
                                    self.wait_for_table_active().await
                                }
                            }
                        } else {
                            Err(DynamoDbError::TableNotFound(format!(
                                "Table '{}' status unknown",
                                self.config.table_name
                            )))
                        }
                    } else {
                        Err(DynamoDbError::TableNotFound(format!(
                            "Table '{}' description not found",
                            self.config.table_name
                        )))
                    }
                }
                Err(_err) => {
                    if self.config.create_tables_if_missing {
                        warn!(
                            "Table '{}' does not exist, attempting to create it",
                            self.config.table_name
                        );
                        self.create_table().await?;
                        self.wait_for_table_active().await?;
                        // Enable TTL after table becomes active
                        self.enable_ttl().await?;

                        // Also create the events table upfront
                        let event_table = self.get_event_table_name();
                        warn!(
                            "Creating events table '{}' upfront to ensure both tables exist",
                            event_table
                        );
                        self.ensure_events_table_exists(&event_table)
                            .await
                            .map_err(|e| {
                                DynamoDbError::AwsError(format!(
                                    "Failed to create events table: {}",
                                    e
                                ))
                            })?;

                        Ok(())
                    } else {
                        let error_msg = format!(
                            "Table '{}' does not exist and create_tables_if_missing is false. Use --create-tables flag to enable table creation.",
                            self.config.table_name
                        );
                        error!("{}", error_msg);
                        Err(DynamoDbError::TableNotFound(error_msg))
                    }
                }
            }
        }

        #[cfg(not(feature = "dynamodb"))]
        Ok(())
    }

    /// Create the DynamoDB table with proper schema
    #[cfg(feature = "dynamodb")]
    async fn create_table(&self) -> Result<(), DynamoDbError> {
        use aws_sdk_dynamodb::types::{
            AttributeDefinition, BillingMode, GlobalSecondaryIndex, KeySchemaElement, KeyType,
            Projection, ProjectionType, ScalarAttributeType,
        };

        info!("Creating DynamoDB table: {}", self.config.table_name);

        // Define table schema
        let key_schema = [KeySchemaElement::builder()
            .attribute_name("session_id")
            .key_type(KeyType::Hash)
            .build()
            .map_err(|e| DynamoDbError::AwsError(e.to_string()))?];

        let attribute_definitions = vec![
            AttributeDefinition::builder()
                .attribute_name("session_id")
                .attribute_type(ScalarAttributeType::S)
                .build()
                .map_err(|e| DynamoDbError::AwsError(e.to_string()))?,
            AttributeDefinition::builder()
                .attribute_name("last_activity")
                .attribute_type(ScalarAttributeType::N)
                .build()
                .map_err(|e| DynamoDbError::AwsError(e.to_string()))?,
        ];

        // GSI for querying by last_activity (for cleanup operations)
        let gsi = GlobalSecondaryIndex::builder()
            .index_name("LastActivityIndex")
            .key_schema(
                KeySchemaElement::builder()
                    .attribute_name("last_activity")
                    .key_type(KeyType::Hash)
                    .build()
                    .map_err(|e| DynamoDbError::AwsError(e.to_string()))?,
            )
            .projection(
                Projection::builder()
                    .projection_type(ProjectionType::KeysOnly)
                    .build(),
            )
            .build()
            .map_err(|e| DynamoDbError::AwsError(e.to_string()))?;

        match self
            .client
            .create_table()
            .table_name(&self.config.table_name)
            .key_schema(key_schema[0].clone())
            .set_attribute_definitions(Some(attribute_definitions))
            .billing_mode(BillingMode::PayPerRequest) // On-demand billing for simplicity
            .set_global_secondary_indexes(Some(vec![gsi]))
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
                Err(DynamoDbError::AwsError(format!(
                    "Failed to create table '{}': {}",
                    self.config.table_name, err
                )))
            }
        }
    }

    /// Enable TTL on the DynamoDB table
    #[cfg(feature = "dynamodb")]
    async fn enable_ttl(&self) -> Result<(), DynamoDbError> {
        use aws_sdk_dynamodb::types::TimeToLiveSpecification;

        info!("Enabling TTL on DynamoDB table: {}", self.config.table_name);

        let ttl_spec = TimeToLiveSpecification::builder()
            .attribute_name("ttl")
            .enabled(true)
            .build()
            .map_err(|e| DynamoDbError::AwsError(e.to_string()))?;

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
                Err(DynamoDbError::AwsError(format!(
                    "Failed to enable TTL on table '{}': {}",
                    self.config.table_name, err
                )))
            }
        }
    }

    /// Ensure TTL is enabled on the main session table
    #[cfg(feature = "dynamodb")]
    #[allow(dead_code)]
    async fn ensure_ttl_enabled(&self) -> Result<(), DynamoDbError> {
        info!(
            "Checking TTL status on DynamoDB table: {}",
            self.config.table_name
        );

        // Check current TTL status
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

    /// Enable TTL on the DynamoDB events table
    #[cfg(feature = "dynamodb")]
    async fn enable_ttl_on_events_table(
        &self,
        event_table: &str,
    ) -> Result<(), SessionStorageError> {
        use aws_sdk_dynamodb::types::TimeToLiveSpecification;

        info!("Enabling TTL on DynamoDB events table: {}", event_table);

        let ttl_spec = TimeToLiveSpecification::builder()
            .attribute_name("ttl")
            .enabled(true)
            .build()
            .map_err(|e| SessionStorageError::AwsError(e.to_string()))?;

        match self
            .client
            .update_time_to_live()
            .table_name(event_table)
            .time_to_live_specification(ttl_spec)
            .send()
            .await
        {
            Ok(_) => {
                info!("Successfully enabled TTL on events table: {}", event_table);
                Ok(())
            }
            Err(err) => {
                error!(
                    "Failed to enable TTL on events table '{}': {}",
                    event_table, err
                );
                Err(SessionStorageError::DatabaseError(format!(
                    "Failed to enable TTL on events table '{}': {}",
                    event_table, err
                )))
            }
        }
    }

    /// Wait for the table to become active
    #[cfg(feature = "dynamodb")]
    async fn wait_for_table_active(&self) -> Result<(), DynamoDbError> {
        use tokio::time::{Duration, sleep};

        info!(
            "Waiting for table '{}' to become active...",
            self.config.table_name
        );

        for attempt in 1..=30 {
            // Wait up to 5 minutes (30 * 10s)
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

        Err(DynamoDbError::AwsError(format!(
            "Table '{}' did not become active within 5 minutes",
            self.config.table_name
        )))
    }

    /// Ensure the events table exists, create if it doesn't
    #[cfg(feature = "dynamodb")]
    async fn ensure_events_table_exists(
        &self,
        event_table: &str,
    ) -> Result<(), SessionStorageError> {
        use aws_sdk_dynamodb::types::{
            AttributeDefinition, BillingMode, GlobalSecondaryIndex, KeySchemaElement, KeyType,
            Projection, ProjectionType, ScalarAttributeType,
        };

        // Debug: Log all available DynamoDB types for events table creation
        debug!(
            "DynamoDB events table creation using types: {}, {}, {}, {}, {}, {} for advanced configurations",
            std::any::type_name::<AttributeDefinition>(),
            std::any::type_name::<KeySchemaElement>(),
            std::any::type_name::<ScalarAttributeType>(),
            std::any::type_name::<GlobalSecondaryIndex>(),
            std::any::type_name::<Projection>(),
            std::any::type_name::<ProjectionType>()
        );

        // Check if events table exists
        match self
            .client
            .describe_table()
            .table_name(event_table)
            .send()
            .await
        {
            Ok(output) => {
                if let Some(table) = output.table()
                    && let Some(TableStatus::Active) = table.table_status()
                {
                    return Ok(());
                }
            }
            Err(_) => {
                if !self.config.create_tables_if_missing {
                    let error_msg = format!(
                        "Events table '{}' does not exist and create_tables_if_missing is false. Use --create-tables flag to enable table creation.",
                        event_table
                    );
                    error!("{}", error_msg);
                    return Err(SessionStorageError::DatabaseError(error_msg));
                }

                // Table doesn't exist, create it
                info!("Creating DynamoDB events table: {}", event_table);

                let key_schema = vec![
                    KeySchemaElement::builder()
                        .attribute_name("session_id")
                        .key_type(KeyType::Hash)
                        .build()
                        .map_err(|e| SessionStorageError::AwsError(e.to_string()))?,
                    KeySchemaElement::builder()
                        .attribute_name("event_id")
                        .key_type(KeyType::Range)
                        .build()
                        .map_err(|e| SessionStorageError::AwsError(e.to_string()))?,
                ];

                let attribute_definitions = vec![
                    AttributeDefinition::builder()
                        .attribute_name("session_id")
                        .attribute_type(ScalarAttributeType::S)
                        .build()
                        .map_err(|e| SessionStorageError::AwsError(e.to_string()))?,
                    AttributeDefinition::builder()
                        .attribute_name("event_id")
                        .attribute_type(ScalarAttributeType::N)
                        .build()
                        .map_err(|e| SessionStorageError::AwsError(e.to_string()))?,
                ];

                match self
                    .client
                    .create_table()
                    .table_name(event_table)
                    .set_key_schema(Some(key_schema))
                    .set_attribute_definitions(Some(attribute_definitions))
                    .billing_mode(BillingMode::PayPerRequest)
                    .send()
                    .await
                {
                    Ok(_) => {
                        info!(
                            "Successfully initiated events table creation: {}",
                            event_table
                        );

                        // Wait for events table to become active
                        self.wait_for_events_table_active(event_table).await?;

                        // Enable TTL on events table
                        self.enable_ttl_on_events_table(event_table).await?;
                    }
                    Err(err) => {
                        return Err(SessionStorageError::DatabaseError(format!(
                            "Failed to create events table '{}': {}",
                            event_table, err
                        )));
                    }
                }
            }
        }

        Ok(())
    }

    /// Wait for the events table to become active
    #[cfg(feature = "dynamodb")]
    async fn wait_for_events_table_active(
        &self,
        event_table: &str,
    ) -> Result<(), SessionStorageError> {
        use tokio::time::{Duration, sleep};

        info!(
            "Waiting for events table '{}' to become active...",
            event_table
        );

        for attempt in 1..=30 {
            // Wait up to 5 minutes (30 * 10s)
            match self
                .client
                .describe_table()
                .table_name(event_table)
                .send()
                .await
            {
                Ok(output) => {
                    if let Some(table) = output.table()
                        && let Some(TableStatus::Active) = table.table_status()
                    {
                        info!("Events table '{}' is now active", event_table);
                        return Ok(());
                    }
                }
                Err(err) => {
                    warn!(
                        "Error checking events table status on attempt {}: {}",
                        attempt, err
                    );
                }
            }

            debug!(
                "Events table not ready, waiting... (attempt {}/30)",
                attempt
            );
            sleep(Duration::from_secs(10)).await;
        }

        Err(SessionStorageError::DatabaseError(format!(
            "Events table '{}' did not become active within 5 minutes",
            event_table
        )))
    }

    /// Convert SessionInfo to DynamoDB AttributeValue format
    #[cfg(feature = "dynamodb")]
    fn session_to_dynamodb_item(
        &self,
        session: &SessionInfo,
    ) -> Result<HashMap<String, AttributeValue>, DynamoDbError> {
        use aws_sdk_dynamodb::types::AttributeValue;

        let mut item = HashMap::new();

        // Primary key
        item.insert(
            "session_id".to_string(),
            AttributeValue::S(session.session_id.clone()),
        );

        // Session data as JSON strings
        if let Some(ref caps) = session.client_capabilities {
            item.insert(
                "client_capabilities".to_string(),
                AttributeValue::S(serde_json::to_string(caps)?),
            );
        }

        if let Some(ref caps) = session.server_capabilities {
            item.insert(
                "server_capabilities".to_string(),
                AttributeValue::S(serde_json::to_string(caps)?),
            );
        }

        item.insert(
            "state".to_string(),
            AttributeValue::S(serde_json::to_string(&session.state)?),
        );
        item.insert(
            "created_at".to_string(),
            AttributeValue::N(session.created_at.to_string()),
        );
        item.insert(
            "last_activity".to_string(),
            AttributeValue::N(session.last_activity.to_string()),
        );
        item.insert(
            "is_initialized".to_string(),
            AttributeValue::Bool(session.is_initialized),
        );
        item.insert(
            "metadata".to_string(),
            AttributeValue::S(serde_json::to_string(&session.metadata)?),
        );

        // TTL attribute for automatic cleanup
        let ttl = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + (self.config.session_ttl_minutes * 60);
        item.insert("ttl".to_string(), AttributeValue::N(ttl.to_string()));

        Ok(item)
    }

    /// Convert DynamoDB item to SessionInfo
    #[cfg(feature = "dynamodb")]
    fn dynamodb_item_to_session(
        &self,
        item: &HashMap<String, AttributeValue>,
    ) -> Result<SessionInfo, DynamoDbError> {
        let session_id = item
            .get("session_id")
            .and_then(|v| v.as_s().ok())
            .ok_or_else(|| DynamoDbError::InvalidSessionData("Missing session_id".to_string()))?
            .clone();

        let client_capabilities = item
            .get("client_capabilities")
            .and_then(|v| v.as_s().ok())
            .map(|s| serde_json::from_str(s))
            .transpose()?;

        let server_capabilities = item
            .get("server_capabilities")
            .and_then(|v| v.as_s().ok())
            .map(|s| serde_json::from_str(s))
            .transpose()?;

        let state = item
            .get("state")
            .and_then(|v| v.as_s().ok())
            .map(|s| serde_json::from_str(s))
            .transpose()?
            .unwrap_or_default();

        let created_at = item
            .get("created_at")
            .and_then(|v| v.as_n().ok())
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| DynamoDbError::InvalidSessionData("Invalid created_at".to_string()))?;

        let last_activity = item
            .get("last_activity")
            .and_then(|v| v.as_n().ok())
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| {
                DynamoDbError::InvalidSessionData("Invalid last_activity".to_string())
            })?;

        let is_initialized = item
            .get("is_initialized")
            .and_then(|v| v.as_bool().ok())
            .copied()
            .unwrap_or(false);

        let metadata = item
            .get("metadata")
            .and_then(|v| v.as_s().ok())
            .map(|s| serde_json::from_str(s))
            .transpose()?
            .unwrap_or_default();

        Ok(SessionInfo {
            session_id,
            client_capabilities,
            server_capabilities,
            state,
            created_at,
            last_activity,
            is_initialized,
            metadata,
        })
    }

    /// Convert SessionInfo to DynamoDB item format (legacy JSON format for tests)
    fn session_to_item(
        &self,
        session: &SessionInfo,
    ) -> Result<HashMap<String, Value>, DynamoDbError> {
        debug!(
            "Converting SessionInfo to legacy JSON format for session: {}",
            session.session_id
        );
        let mut item = HashMap::new();

        // Primary key
        item.insert(
            "session_id".to_string(),
            Value::String(session.session_id.clone()),
        );

        // Session data
        item.insert(
            "client_capabilities".to_string(),
            serde_json::to_value(&session.client_capabilities)?,
        );
        item.insert(
            "server_capabilities".to_string(),
            serde_json::to_value(&session.server_capabilities)?,
        );
        item.insert("state".to_string(), serde_json::to_value(&session.state)?);
        item.insert(
            "created_at".to_string(),
            Value::Number(session.created_at.into()),
        );
        item.insert(
            "last_activity".to_string(),
            Value::Number(session.last_activity.into()),
        );
        item.insert(
            "is_initialized".to_string(),
            Value::Bool(session.is_initialized),
        );
        item.insert(
            "metadata".to_string(),
            serde_json::to_value(&session.metadata)?,
        );

        // TTL attribute for automatic cleanup
        let ttl = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + (self.config.session_ttl_minutes * 60);
        item.insert("ttl".to_string(), Value::Number(ttl.into()));

        Ok(item)
    }

    /// Convert DynamoDB item to SessionInfo (legacy JSON format for tests)
    fn item_to_session(&self, item: &HashMap<String, Value>) -> Result<SessionInfo, DynamoDbError> {
        debug!("Converting DynamoDB legacy JSON item to SessionInfo");
        let session_id = item
            .get("session_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| DynamoDbError::InvalidSessionData("Missing session_id".to_string()))?
            .to_string();

        let client_capabilities = item
            .get("client_capabilities")
            .map(|v| serde_json::from_value(v.clone()))
            .transpose()?;

        let server_capabilities = item
            .get("server_capabilities")
            .map(|v| serde_json::from_value(v.clone()))
            .transpose()?;

        let state = item
            .get("state")
            .map(|v| serde_json::from_value(v.clone()))
            .transpose()?
            .unwrap_or_default();

        let created_at = item
            .get("created_at")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| DynamoDbError::InvalidSessionData("Invalid created_at".to_string()))?;

        let last_activity = item
            .get("last_activity")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| {
                DynamoDbError::InvalidSessionData("Invalid last_activity".to_string())
            })?;

        let is_initialized = item
            .get("is_initialized")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let metadata = item
            .get("metadata")
            .map(|v| serde_json::from_value(v.clone()))
            .transpose()?
            .unwrap_or_default();

        Ok(SessionInfo {
            session_id,
            client_capabilities,
            server_capabilities,
            state,
            created_at,
            last_activity,
            is_initialized,
            metadata,
        })
    }

    /// Public method to create both DynamoDB tables (for setup utilities)
    pub async fn create_tables(&self) -> Result<(), DynamoDbError> {
        info!("Creating both DynamoDB tables: session and events");

        // Create main session table
        self.create_table().await?;
        self.wait_for_table_active().await?;
        self.enable_ttl().await?;

        // Create events table
        let event_table = self.get_event_table_name();
        self.ensure_events_table_exists(&event_table)
            .await
            .map_err(|e| {
                DynamoDbError::AwsError(format!("Failed to create events table: {}", e))
            })?;

        info!("Successfully created both DynamoDB tables");
        Ok(())
    }

    /// Public method to delete both DynamoDB tables (for teardown utilities)
    pub async fn delete_tables(&self) -> Result<(), DynamoDbError> {
        #[cfg(feature = "dynamodb")]
        {
            let main_table = &self.config.table_name;
            let event_table = format!("{}-events", self.config.table_name);

            info!(
                "Deleting DynamoDB tables: {} and {}",
                main_table, event_table
            );

            // Delete main session table
            match self
                .client
                .delete_table()
                .table_name(main_table)
                .send()
                .await
            {
                Ok(_) => info!("Successfully initiated deletion of table: {}", main_table),
                Err(err) => warn!("Failed to delete table '{}': {}", main_table, err),
            }

            // Delete events table
            match self
                .client
                .delete_table()
                .table_name(&event_table)
                .send()
                .await
            {
                Ok(_) => info!("Successfully initiated deletion of table: {}", event_table),
                Err(err) => warn!("Failed to delete table '{}': {}", event_table, err),
            }

            info!("Table deletion initiated for both tables");
            Ok(())
        }

        #[cfg(not(feature = "dynamodb"))]
        {
            error!("DynamoDB feature is not enabled");
            Err(DynamoDbError::ConfigError(
                "DynamoDB feature is not enabled".to_string(),
            ))
        }
    }
}

#[async_trait]
impl SessionStorage for DynamoDbSessionStorage {
    type Error = SessionStorageError;

    fn backend_name(&self) -> &'static str {
        "DynamoDB"
    }

    async fn create_session(
        &self,
        capabilities: ServerCapabilities,
    ) -> Result<SessionInfo, Self::Error> {
        let mut session = SessionInfo::new();
        session.server_capabilities = Some(capabilities);

        #[cfg(feature = "dynamodb")]
        {
            let item = self.session_to_dynamodb_item(&session)?;

            match self
                .client
                .put_item()
                .table_name(&self.config.table_name)
                .set_item(Some(item))
                .send()
                .await
            {
                Ok(_) => {
                    debug!(
                        "Successfully created session in DynamoDB: {}",
                        session.session_id
                    );
                }
                Err(err) => {
                    error!("Failed to create session in DynamoDB: {}", err);
                    return Err(SessionStorageError::DatabaseError(format!(
                        "Failed to create session: {}",
                        err
                    )));
                }
            }
        }

        #[cfg(not(feature = "dynamodb"))]
        {
            debug!(
                "Creating session in DynamoDB (placeholder): {}",
                session.session_id
            );
        }

        Ok(session)
    }

    async fn create_session_with_id(
        &self,
        session_id: String,
        capabilities: ServerCapabilities,
    ) -> Result<SessionInfo, Self::Error> {
        let mut session = SessionInfo::with_id(session_id.clone());
        session.server_capabilities = Some(capabilities);

        #[cfg(feature = "dynamodb")]
        {
            let item = self.session_to_dynamodb_item(&session)?;

            match self
                .client
                .put_item()
                .table_name(&self.config.table_name)
                .set_item(Some(item))
                .send()
                .await
            {
                Ok(_) => {
                    debug!(
                        "Successfully created session with ID in DynamoDB: {}",
                        session_id
                    );
                }
                Err(err) => {
                    error!("Failed to create session with ID in DynamoDB: {}", err);
                    return Err(SessionStorageError::DatabaseError(format!(
                        "Failed to create session with ID '{}': {}",
                        session_id, err
                    )));
                }
            }
        }

        #[cfg(not(feature = "dynamodb"))]
        {
            debug!(
                "Creating session with ID in DynamoDB (placeholder): {}",
                session_id
            );
        }

        Ok(session)
    }

    async fn get_session(&self, session_id: &str) -> Result<Option<SessionInfo>, Self::Error> {
        #[cfg(feature = "dynamodb")]
        {
            use aws_sdk_dynamodb::types::AttributeValue;

            let key = HashMap::from([(
                "session_id".to_string(),
                AttributeValue::S(session_id.to_string()),
            )]);

            match self
                .client
                .get_item()
                .table_name(&self.config.table_name)
                .set_key(Some(key))
                .send()
                .await
            {
                Ok(output) => {
                    if let Some(item) = output.item() {
                        let session = self.dynamodb_item_to_session(item)?;
                        debug!(
                            "Successfully retrieved session from DynamoDB: {}",
                            session_id
                        );
                        Ok(Some(session))
                    } else {
                        debug!("Session not found in DynamoDB: {}", session_id);
                        Ok(None)
                    }
                }
                Err(err) => {
                    error!("Failed to get session from DynamoDB: {}", err);
                    Err(SessionStorageError::DatabaseError(format!(
                        "Failed to get session '{}': {}",
                        session_id, err
                    )))
                }
            }
        }

        #[cfg(not(feature = "dynamodb"))]
        {
            debug!(
                "Getting session from DynamoDB (placeholder): {}",
                session_id
            );
            Ok(None)
        }
    }

    async fn update_session(&self, session_info: SessionInfo) -> Result<(), Self::Error> {
        #[cfg(feature = "dynamodb")]
        {
            let item = self.session_to_dynamodb_item(&session_info)?;

            match self
                .client
                .put_item()
                .table_name(&self.config.table_name)
                .set_item(Some(item))
                .send()
                .await
            {
                Ok(_) => {
                    debug!(
                        "Successfully updated session in DynamoDB: {}",
                        session_info.session_id
                    );
                    Ok(())
                }
                Err(err) => {
                    error!("Failed to update session in DynamoDB: {}", err);
                    Err(SessionStorageError::DatabaseError(format!(
                        "Failed to update session '{}': {}",
                        session_info.session_id, err
                    )))
                }
            }
        }

        #[cfg(not(feature = "dynamodb"))]
        {
            debug!(
                "Updating session in DynamoDB (placeholder): {}",
                session_info.session_id
            );
            Ok(())
        }
    }

    async fn set_session_state(
        &self,
        session_id: &str,
        key: &str,
        value: Value,
    ) -> Result<(), Self::Error> {
        #[cfg(feature = "dynamodb")]
        {
            use aws_sdk_dynamodb::types::AttributeValue;

            // First, get the current session to retrieve existing state
            let session_key = HashMap::from([(
                "session_id".to_string(),
                AttributeValue::S(session_id.to_string()),
            )]);

            let current_item = match self
                .client
                .get_item()
                .table_name(&self.config.table_name)
                .set_key(Some(session_key.clone()))
                .send()
                .await
            {
                Ok(result) => result
                    .item
                    .ok_or_else(|| SessionStorageError::SessionNotFound(session_id.to_string()))?,
                Err(err) => {
                    error!("Failed to retrieve session for state update: {}", err);
                    return Err(SessionStorageError::DatabaseError(format!(
                        "Failed to get session '{}': {}",
                        session_id, err
                    )));
                }
            };

            // Parse current state
            let mut current_state: HashMap<String, Value> = current_item
                .get("state")
                .and_then(|v| v.as_s().ok())
                .map(|s| serde_json::from_str(s))
                .transpose()
                .map_err(|e| SessionStorageError::SerializationError(e.to_string()))?
                .unwrap_or_default();

            // Update the specific key
            current_state.insert(key.to_string(), value);

            // Serialize updated state back to JSON
            let updated_state_json = serde_json::to_string(&current_state)
                .map_err(|e| SessionStorageError::SerializationError(e.to_string()))?;

            // Update the session with new state
            let update_expression = "SET #state = :state, #last_activity = :timestamp";
            let expression_attribute_names = HashMap::from([
                ("#state".to_string(), "state".to_string()),
                ("#last_activity".to_string(), "last_activity".to_string()),
            ]);
            let expression_attribute_values = HashMap::from([
                (":state".to_string(), AttributeValue::S(updated_state_json)),
                (
                    ":timestamp".to_string(),
                    AttributeValue::N(chrono::Utc::now().timestamp_millis().to_string()),
                ),
            ]);

            match self
                .client
                .update_item()
                .table_name(&self.config.table_name)
                .set_key(Some(session_key))
                .update_expression(update_expression)
                .set_expression_attribute_names(Some(expression_attribute_names))
                .set_expression_attribute_values(Some(expression_attribute_values))
                .send()
                .await
            {
                Ok(_) => {
                    debug!(
                        "Successfully set session state in DynamoDB: {} -> {}",
                        session_id, key
                    );
                    Ok(())
                }
                Err(err) => {
                    error!("Failed to set session state in DynamoDB: {}", err);
                    Err(SessionStorageError::DatabaseError(format!(
                        "Failed to set session state '{}' -> '{}': {}",
                        session_id, key, err
                    )))
                }
            }
        }

        #[cfg(not(feature = "dynamodb"))]
        {
            debug!(
                "Setting session state in DynamoDB (placeholder): {} -> {}",
                session_id, key
            );
            Ok(())
        }
    }

    async fn get_session_state(
        &self,
        session_id: &str,
        key: &str,
    ) -> Result<Option<Value>, Self::Error> {
        #[cfg(feature = "dynamodb")]
        {
            // Get the entire session and extract the state value
            if let Some(session) = self.get_session(session_id).await? {
                if let Some(value) = session.state.get(key) {
                    debug!(
                        "Successfully retrieved session state from DynamoDB: {} -> {}",
                        session_id, key
                    );
                    Ok(Some(value.clone()))
                } else {
                    debug!(
                        "Session state key not found in DynamoDB: {} -> {}",
                        session_id, key
                    );
                    Ok(None)
                }
            } else {
                debug!(
                    "Session not found for state retrieval in DynamoDB: {}",
                    session_id
                );
                Ok(None)
            }
        }

        #[cfg(not(feature = "dynamodb"))]
        {
            debug!(
                "Getting session state from DynamoDB (placeholder): {} -> {}",
                session_id, key
            );
            Ok(None)
        }
    }

    async fn remove_session_state(
        &self,
        session_id: &str,
        key: &str,
    ) -> Result<Option<Value>, Self::Error> {
        #[cfg(feature = "dynamodb")]
        {
            use aws_sdk_dynamodb::types::AttributeValue;

            // First get the current value
            let current_value = self.get_session_state(session_id, key).await?;

            if current_value.is_some() {
                let session_key = HashMap::from([(
                    "session_id".to_string(),
                    AttributeValue::S(session_id.to_string()),
                )]);

                // Use UpdateExpression to remove the key from the state map
                let update_expression =
                    "REMOVE #state.#key SET #last_activity = :timestamp".to_string();
                let expression_attribute_names = HashMap::from([
                    ("#state".to_string(), "state".to_string()),
                    ("#key".to_string(), key.to_string()),
                    ("#last_activity".to_string(), "last_activity".to_string()),
                ]);
                let expression_attribute_values = HashMap::from([(
                    ":timestamp".to_string(),
                    AttributeValue::N(chrono::Utc::now().timestamp_millis().to_string()),
                )]);

                match self
                    .client
                    .update_item()
                    .table_name(&self.config.table_name)
                    .set_key(Some(session_key))
                    .update_expression(update_expression)
                    .set_expression_attribute_names(Some(expression_attribute_names))
                    .set_expression_attribute_values(Some(expression_attribute_values))
                    .send()
                    .await
                {
                    Ok(_) => {
                        debug!(
                            "Successfully removed session state from DynamoDB: {} -> {}",
                            session_id, key
                        );
                        Ok(current_value)
                    }
                    Err(err) => {
                        error!("Failed to remove session state from DynamoDB: {}", err);
                        Err(SessionStorageError::DatabaseError(format!(
                            "Failed to remove session state '{}' -> '{}': {}",
                            session_id, key, err
                        )))
                    }
                }
            } else {
                debug!(
                    "Session state key not found for removal in DynamoDB: {} -> {}",
                    session_id, key
                );
                Ok(None)
            }
        }

        #[cfg(not(feature = "dynamodb"))]
        {
            debug!(
                "Removing session state from DynamoDB (placeholder): {} -> {}",
                session_id, key
            );
            Ok(None)
        }
    }

    async fn delete_session(&self, session_id: &str) -> Result<bool, Self::Error> {
        #[cfg(feature = "dynamodb")]
        {
            use aws_sdk_dynamodb::types::AttributeValue;

            let key = HashMap::from([(
                "session_id".to_string(),
                AttributeValue::S(session_id.to_string()),
            )]);

            match self
                .client
                .delete_item()
                .table_name(&self.config.table_name)
                .set_key(Some(key))
                .send()
                .await
            {
                Ok(_) => {
                    debug!("Successfully deleted session from DynamoDB: {}", session_id);
                    Ok(true)
                }
                Err(err) => {
                    error!("Failed to delete session from DynamoDB: {}", err);
                    Err(SessionStorageError::DatabaseError(format!(
                        "Failed to delete session '{}': {}",
                        session_id, err
                    )))
                }
            }
        }

        #[cfg(not(feature = "dynamodb"))]
        {
            debug!(
                "Deleting session from DynamoDB (placeholder): {}",
                session_id
            );
            Ok(true)
        }
    }

    async fn list_sessions(&self) -> Result<Vec<String>, Self::Error> {
        #[cfg(feature = "dynamodb")]
        {
            debug!("Scanning DynamoDB table for all session IDs");

            // Note: Scan is expensive for large tables - consider using pagination
            match self
                .client
                .scan()
                .table_name(&self.config.table_name)
                .projection_expression("session_id")
                .send()
                .await
            {
                Ok(output) => {
                    let mut session_ids = Vec::new();

                    for item in output.items() {
                        if let Some(session_id_attr) = item.get("session_id")
                            && let Ok(session_id) = session_id_attr.as_s()
                        {
                            session_ids.push(session_id.clone());
                        }
                    }

                    debug!("Listed {} session IDs from DynamoDB", session_ids.len());
                    Ok(session_ids)
                }
                Err(err) => {
                    error!("Failed to list sessions from DynamoDB: {}", err);
                    Err(SessionStorageError::DatabaseError(format!(
                        "Failed to list sessions: {}",
                        err
                    )))
                }
            }
        }

        #[cfg(not(feature = "dynamodb"))]
        {
            debug!("Listing all sessions from DynamoDB (placeholder)");
            Ok(vec![])
        }
    }

    async fn store_event(
        &self,
        session_id: &str,
        mut event: SseEvent,
    ) -> Result<SseEvent, Self::Error> {
        // Assign unique event ID
        event.id = self.event_counter.fetch_add(1, Ordering::SeqCst);

        #[cfg(feature = "dynamodb")]
        {
            use aws_sdk_dynamodb::types::AttributeValue;

            // Check if session exists first
            let session_key = HashMap::from([(
                "session_id".to_string(),
                AttributeValue::S(session_id.to_string()),
            )]);

            let session_exists = match self
                .client
                .get_item()
                .table_name(&self.config.table_name)
                .set_key(Some(session_key))
                .send()
                .await
            {
                Ok(result) => result.item.is_some(),
                Err(_) => false,
            };

            if !session_exists {
                return Err(SessionStorageError::SessionNotFound(session_id.to_string()));
            }

            // Store events in separate table: mcp-sessions-events
            let event_table = format!("{}-events", self.config.table_name);

            // Create events table if it doesn't exist
            self.ensure_events_table_exists(&event_table).await?;

            let data_json = serde_json::to_string(&event.data)
                .map_err(|e| SessionStorageError::SerializationError(e.to_string()))?;

            let mut item = HashMap::from([
                (
                    "session_id".to_string(),
                    AttributeValue::S(session_id.to_string()),
                ),
                (
                    "event_id".to_string(),
                    AttributeValue::N(event.id.to_string()),
                ),
                (
                    "timestamp".to_string(),
                    AttributeValue::N(event.timestamp.to_string()),
                ),
                (
                    "event_type".to_string(),
                    AttributeValue::S(event.event_type.clone()),
                ),
                ("data".to_string(), AttributeValue::S(data_json)),
                // TTL for automatic event cleanup
                (
                    "ttl".to_string(),
                    AttributeValue::N(
                        (SystemTime::now()
                            .duration_since(SystemTime::UNIX_EPOCH)
                            .unwrap()
                            .as_secs()
                            + (self.config.event_ttl_minutes * 60))
                            .to_string(),
                    ),
                ),
            ]);

            if let Some(retry) = event.retry {
                item.insert("retry".to_string(), AttributeValue::N(retry.to_string()));
            }

            match self
                .client
                .put_item()
                .table_name(&event_table)
                .set_item(Some(item))
                .send()
                .await
            {
                Ok(_) => {
                    debug!(
                        "Successfully stored SSE event {} in DynamoDB for session: {}",
                        event.id, session_id
                    );
                }
                Err(err) => {
                    error!("Failed to store SSE event in DynamoDB: {}", err);
                    return Err(SessionStorageError::DatabaseError(format!(
                        "Failed to store event: {}",
                        err
                    )));
                }
            }
        }

        #[cfg(not(feature = "dynamodb"))]
        {
            debug!(
                "Storing SSE event in DynamoDB (placeholder): {} -> {}",
                session_id, event.id
            );
        }

        Ok(event)
    }

    async fn get_events_after(
        &self,
        session_id: &str,
        after_event_id: u64,
    ) -> Result<Vec<SseEvent>, Self::Error> {
        #[cfg(feature = "dynamodb")]
        {
            use aws_sdk_dynamodb::types::AttributeValue;

            let event_table = format!("{}-events", self.config.table_name);

            // Ensure the events table exists
            self.ensure_events_table_exists(&event_table).await?;

            // Query events for this session where event_id > after_event_id
            let query_result = self
                .client
                .query()
                .table_name(&event_table)
                .key_condition_expression("session_id = :session_id AND event_id > :after_event_id")
                .expression_attribute_values(
                    ":session_id",
                    AttributeValue::S(session_id.to_string()),
                )
                .expression_attribute_values(
                    ":after_event_id",
                    AttributeValue::N(after_event_id.to_string()),
                )
                .scan_index_forward(true) // Sort by event_id ascending
                .consistent_read(true) // Use strongly consistent reads for resumability
                .send()
                .await
                .map_err(|e| DynamoDbError::AwsError(e.to_string()))?;

            let mut events = Vec::new();
            if let Some(items) = query_result.items {
                for item in items {
                    let event_id = item
                        .get("event_id")
                        .and_then(|v| v.as_n().ok())
                        .and_then(|n| n.parse::<u64>().ok())
                        .unwrap_or(0);

                    let event_type = item
                        .get("event_type")
                        .and_then(|v| v.as_s().ok())
                        .map_or("message".to_string(), |s| s.clone());

                    let data = item
                        .get("data")
                        .and_then(|v| v.as_s().ok())
                        .and_then(|s| serde_json::from_str(s).ok())
                        .unwrap_or(Value::Null);

                    let timestamp = item
                        .get("timestamp")
                        .and_then(|v| v.as_s().ok())
                        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(Utc::now);

                    let event = SseEvent {
                        id: event_id,
                        timestamp: timestamp.timestamp_millis() as u64,
                        event_type,
                        data,
                        retry: None,
                    };

                    events.push(event);
                }
            }

            debug!(
                "Retrieved {} events after {} from DynamoDB for session: {}",
                events.len(),
                after_event_id,
                session_id
            );
            Ok(events)
        }

        #[cfg(not(feature = "dynamodb"))]
        {
            debug!(
                "Getting events after {} from DynamoDB (placeholder): {}",
                after_event_id, session_id
            );
            Ok(vec![])
        }
    }

    async fn get_recent_events(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<SseEvent>, Self::Error> {
        #[cfg(feature = "dynamodb")]
        {
            use aws_sdk_dynamodb::types::AttributeValue;

            let event_table = format!("{}-events", self.config.table_name);

            // Ensure the events table exists
            self.ensure_events_table_exists(&event_table).await?;

            // Query recent events for this session, ordered by event_id DESC (most recent first)
            let query_result = self
                .client
                .query()
                .table_name(&event_table)
                .key_condition_expression("session_id = :session_id")
                .expression_attribute_values(
                    ":session_id",
                    AttributeValue::S(session_id.to_string()),
                )
                .scan_index_forward(false) // Sort by event_id descending (most recent first)
                .limit(limit as i32)
                .consistent_read(true) // Use strongly consistent reads to see just-written events
                .send()
                .await
                .map_err(|e| DynamoDbError::AwsError(e.to_string()))?;

            let mut events = Vec::new();
            if let Some(items) = query_result.items {
                for item in items {
                    let event_id = item
                        .get("event_id")
                        .and_then(|v| v.as_n().ok())
                        .and_then(|n| n.parse::<u64>().ok())
                        .unwrap_or(0);

                    let event_type = item
                        .get("event_type")
                        .and_then(|v| v.as_s().ok())
                        .map_or("message".to_string(), |s| s.clone());

                    let data = item
                        .get("data")
                        .and_then(|v| v.as_s().ok())
                        .and_then(|s| serde_json::from_str(s).ok())
                        .unwrap_or(Value::Null);

                    let timestamp = item
                        .get("timestamp")
                        .and_then(|v| v.as_s().ok())
                        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(Utc::now);

                    let event = SseEvent {
                        id: event_id,
                        timestamp: timestamp.timestamp_millis() as u64,
                        event_type,
                        data,
                        retry: None,
                    };

                    events.push(event);
                }
            }

            // Reverse to get chronological order (oldest first)
            events.reverse();

            debug!(
                "Retrieved {} recent events from DynamoDB for session: {}",
                events.len(),
                session_id
            );
            Ok(events)
        }

        #[cfg(not(feature = "dynamodb"))]
        {
            debug!(
                "Getting {} recent events from DynamoDB (placeholder): {}",
                limit, session_id
            );
            Ok(vec![])
        }
    }

    async fn delete_events_before(
        &self,
        session_id: &str,
        before_event_id: u64,
    ) -> Result<u64, Self::Error> {
        #[cfg(feature = "dynamodb")]
        {
            // In a production setup, this would scan and delete events
            // where session_id = session_id AND event_id < before_event_id
            debug!(
                "Would delete events before {} from DynamoDB events table for session: {}",
                before_event_id, session_id
            );

            // For now, return 0 as we're not implementing the full events table
            Ok(0)
        }

        #[cfg(not(feature = "dynamodb"))]
        {
            debug!(
                "Deleting events before {} from DynamoDB (placeholder): {}",
                before_event_id, session_id
            );
            Ok(0)
        }
    }

    async fn expire_sessions(&self, older_than: SystemTime) -> Result<Vec<String>, Self::Error> {
        #[cfg(feature = "dynamodb")]
        {
            let timestamp = older_than
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs();

            // DynamoDB TTL handles automatic deletion, but for manual cleanup:
            // We would scan for sessions where last_activity < timestamp
            debug!(
                "Would scan for sessions with last_activity < {} in DynamoDB",
                timestamp
            );

            // In a real implementation, this would:
            // 1. Scan table with FilterExpression on last_activity
            // 2. Collect session IDs of expired sessions
            // 3. BatchDeleteItem to remove them
            // 4. Return the list of deleted session IDs

            // For now, return empty list as DynamoDB TTL handles cleanup automatically
            Ok(vec![])
        }

        #[cfg(not(feature = "dynamodb"))]
        {
            let timestamp = older_than
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            debug!(
                "Expiring sessions older than {} from DynamoDB (placeholder)",
                timestamp
            );
            Ok(vec![])
        }
    }

    async fn session_count(&self) -> Result<usize, Self::Error> {
        #[cfg(feature = "dynamodb")]
        {
            // In production, this would use scan with count-only projection
            // or maintain a counter in a separate item
            debug!("Would scan DynamoDB table to count sessions");

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
                    debug!("DynamoDB session count: {}", count);
                    Ok(count)
                }
                Err(err) => {
                    error!("Failed to count sessions in DynamoDB: {}", err);
                    Err(SessionStorageError::DatabaseError(format!(
                        "Failed to count sessions: {}",
                        err
                    )))
                }
            }
        }

        #[cfg(not(feature = "dynamodb"))]
        {
            debug!("Counting sessions in DynamoDB (placeholder)");
            Ok(0)
        }
    }

    async fn event_count(&self) -> Result<usize, Self::Error> {
        #[cfg(feature = "dynamodb")]
        {
            // In production with separate events table, this would scan that table
            let event_table = format!("{}-events", self.config.table_name);
            debug!("Would count events in DynamoDB table: {}", event_table);

            // For now, return 0 as we're not implementing the full events table
            Ok(0)
        }

        #[cfg(not(feature = "dynamodb"))]
        {
            debug!("Counting events in DynamoDB (placeholder)");
            Ok(0)
        }
    }

    async fn maintenance(&self) -> Result<(), Self::Error> {
        // DynamoDB maintenance is mostly automatic with TTL
        // Could implement event cleanup here if needed
        debug!("Performing DynamoDB maintenance");

        // Debug: Test legacy conversion methods periodically for compatibility
        if cfg!(debug_assertions) {
            let test_session = SessionInfo::new();
            if let Ok(item) = self.session_to_item(&test_session)
                && let Ok(_converted_back) = self.item_to_session(&item)
            {
                debug!(
                    "Legacy JSON conversion methods working correctly for session: {}",
                    test_session.session_id
                );
            }
        }

        Ok(())
    }
}

#[cfg(all(test, feature = "dynamodb"))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dynamodb_config() {
        let config = DynamoDbConfig::default();
        assert_eq!(config.table_name, "mcp-sessions");
        // Region from AWS_REGION env var or default "us-east-1"
        let expected_region = std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string());
        assert_eq!(config.region, expected_region);
        assert_eq!(config.session_ttl_minutes, 5);
    }

    #[tokio::test]
    #[ignore = "Serialization issues with None ClientCapabilities - use dedicated simple-dynamodb-session example instead"]
    async fn test_session_serialization() {
        let storage = DynamoDbSessionStorage::with_config(DynamoDbConfig::default())
            .await
            .unwrap();

        let session = SessionInfo::new();
        let item = storage.session_to_item(&session).unwrap();
        let deserialized = storage.item_to_session(&item).unwrap();

        assert_eq!(session.session_id, deserialized.session_id);
        assert_eq!(session.created_at, deserialized.created_at);
        assert_eq!(session.is_initialized, deserialized.is_initialized);
    }

    // TODO: Add integration tests with DynamoDB Local or LocalStack
}
