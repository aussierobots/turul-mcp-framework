//! AWS DynamoDB Session Storage Implementation
//!
//! This module provides a DynamoDB-backed session storage implementation for
//! serverless and AWS-native MCP deployments.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

use async_trait::async_trait;
use serde_json::Value;
use thiserror::Error;
use tracing::{debug, error, info, warn};

use mcp_protocol::ServerCapabilities;

use crate::{SessionStorage, SessionInfo, SseEvent, SessionStorageError};

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
    /// Session TTL in hours (DynamoDB TTL attribute)
    pub session_ttl_hours: u64,
    /// Event TTL in hours (separate from sessions)
    pub event_ttl_hours: u64,
    /// Maximum events per session (for cleanup)
    pub max_events_per_session: u64,
    /// Enable point-in-time recovery
    pub enable_backup: bool,
    /// Enable encryption at rest
    pub enable_encryption: bool,
}

impl Default for DynamoDbConfig {
    fn default() -> Self {
        Self {
            table_name: "mcp-sessions".to_string(),
            region: "us-east-1".to_string(),
            session_ttl_hours: 24,
            event_ttl_hours: 24,
            max_events_per_session: 1000,
            enable_backup: true,
            enable_encryption: true,
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
    /// Create a new DynamoDB session storage with default configuration
    pub async fn new() -> Result<Self, DynamoDbError> {
        Self::with_config(DynamoDbConfig::default()).await
    }

    /// Create a new DynamoDB session storage with custom configuration
    pub async fn with_config(config: DynamoDbConfig) -> Result<Self, DynamoDbError> {
        info!(
            "Initializing DynamoDB session storage with table: {}",
            config.table_name
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

            info!("DynamoDB session storage initialized successfully");
            Ok(storage)
        }
        
        #[cfg(not(feature = "dynamodb"))]
        {
            error!("DynamoDB feature is not enabled");
            Err(DynamoDbError::ConfigError(
                "DynamoDB feature is not enabled".to_string()
            ))
        }
    }

    /// Verify that the DynamoDB table exists and has the correct schema
    async fn verify_table_schema(&self) -> Result<(), DynamoDbError> {
        #[cfg(feature = "dynamodb")]
        {
            debug!("Verifying table schema for: {}", self.config.table_name);
            
            match self.client.describe_table()
                .table_name(&self.config.table_name)
                .send()
                .await 
            {
                Ok(output) => {
                    if let Some(table) = output.table() {
                        if let Some(status) = table.table_status() {
                            match status {
                                TableStatus::Active => {
                                    info!("DynamoDB table '{}' is active and ready", self.config.table_name);
                                    Ok(())
                                }
                                _ => {
                                    warn!("DynamoDB table '{}' is not active: {:?}", self.config.table_name, status);
                                    Err(DynamoDbError::TableNotFound(format!(
                                        "Table '{}' exists but is not active: {:?}",
                                        self.config.table_name, status
                                    )))
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
                Err(err) => {
                    error!("Failed to describe DynamoDB table '{}': {}", self.config.table_name, err);
                    Err(DynamoDbError::AwsError(format!(
                        "Failed to describe table '{}': {}",
                        self.config.table_name, err
                    )))
                }
            }
        }
        
        #[cfg(not(feature = "dynamodb"))]
        Ok(())
    }

    /// Convert SessionInfo to DynamoDB AttributeValue format
    #[cfg(feature = "dynamodb")]
    fn session_to_dynamodb_item(&self, session: &SessionInfo) -> Result<HashMap<String, AttributeValue>, DynamoDbError> {
        use aws_sdk_dynamodb::types::AttributeValue;
        
        let mut item = HashMap::new();
        
        // Primary key
        item.insert("session_id".to_string(), 
                   AttributeValue::S(session.session_id.clone()));
        
        // Session data as JSON strings
        if let Some(ref caps) = session.client_capabilities {
            item.insert("client_capabilities".to_string(), 
                       AttributeValue::S(serde_json::to_string(caps)?));
        }
        
        if let Some(ref caps) = session.server_capabilities {
            item.insert("server_capabilities".to_string(),
                       AttributeValue::S(serde_json::to_string(caps)?));
        }
        
        item.insert("state".to_string(), 
                   AttributeValue::S(serde_json::to_string(&session.state)?));
        item.insert("created_at".to_string(), 
                   AttributeValue::N(session.created_at.to_string()));
        item.insert("last_activity".to_string(),
                   AttributeValue::N(session.last_activity.to_string()));
        item.insert("is_initialized".to_string(),
                   AttributeValue::Bool(session.is_initialized));
        item.insert("metadata".to_string(),
                   AttributeValue::S(serde_json::to_string(&session.metadata)?));
        
        // TTL attribute for automatic cleanup
        let ttl = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() + (self.config.session_ttl_hours * 3600);
        item.insert("ttl".to_string(), AttributeValue::N(ttl.to_string()));
        
        Ok(item)
    }

    /// Convert DynamoDB item to SessionInfo
    #[cfg(feature = "dynamodb")]
    fn dynamodb_item_to_session(&self, item: &HashMap<String, AttributeValue>) -> Result<SessionInfo, DynamoDbError> {
        
        let session_id = item.get("session_id")
            .and_then(|v| v.as_s().ok())
            .ok_or_else(|| DynamoDbError::InvalidSessionData("Missing session_id".to_string()))?
            .clone();

        let client_capabilities = item.get("client_capabilities")
            .and_then(|v| v.as_s().ok())
            .map(|s| serde_json::from_str(s))
            .transpose()?;

        let server_capabilities = item.get("server_capabilities")
            .and_then(|v| v.as_s().ok())
            .map(|s| serde_json::from_str(s))
            .transpose()?;

        let state = item.get("state")
            .and_then(|v| v.as_s().ok())
            .map(|s| serde_json::from_str(s))
            .transpose()?
            .unwrap_or_default();

        let created_at = item.get("created_at")
            .and_then(|v| v.as_n().ok())
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| DynamoDbError::InvalidSessionData("Invalid created_at".to_string()))?;

        let last_activity = item.get("last_activity")
            .and_then(|v| v.as_n().ok())
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| DynamoDbError::InvalidSessionData("Invalid last_activity".to_string()))?;

        let is_initialized = item.get("is_initialized")
            .and_then(|v| v.as_bool().ok())
            .copied()
            .unwrap_or(false);

        let metadata = item.get("metadata")
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
    fn session_to_item(&self, session: &SessionInfo) -> Result<HashMap<String, Value>, DynamoDbError> {
        let mut item = HashMap::new();
        
        // Primary key
        item.insert("session_id".to_string(), Value::String(session.session_id.clone()));
        
        // Session data
        item.insert("client_capabilities".to_string(), 
                   serde_json::to_value(&session.client_capabilities)?);
        item.insert("server_capabilities".to_string(),
                   serde_json::to_value(&session.server_capabilities)?);
        item.insert("state".to_string(), 
                   serde_json::to_value(&session.state)?);
        item.insert("created_at".to_string(), 
                   Value::Number(session.created_at.into()));
        item.insert("last_activity".to_string(),
                   Value::Number(session.last_activity.into()));
        item.insert("is_initialized".to_string(),
                   Value::Bool(session.is_initialized));
        item.insert("metadata".to_string(),
                   serde_json::to_value(&session.metadata)?);
        
        // TTL attribute for automatic cleanup
        let ttl = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() + (self.config.session_ttl_hours * 3600);
        item.insert("ttl".to_string(), Value::Number(ttl.into()));
        
        Ok(item)
    }

    /// Convert DynamoDB item to SessionInfo (legacy JSON format for tests)
    fn item_to_session(&self, item: &HashMap<String, Value>) -> Result<SessionInfo, DynamoDbError> {
        let session_id = item.get("session_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| DynamoDbError::InvalidSessionData("Missing session_id".to_string()))?
            .to_string();

        let client_capabilities = item.get("client_capabilities")
            .map(|v| serde_json::from_value(v.clone()))
            .transpose()?;

        let server_capabilities = item.get("server_capabilities")
            .map(|v| serde_json::from_value(v.clone()))
            .transpose()?;

        let state = item.get("state")
            .map(|v| serde_json::from_value(v.clone()))
            .transpose()?
            .unwrap_or_default();

        let created_at = item.get("created_at")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| DynamoDbError::InvalidSessionData("Invalid created_at".to_string()))?;

        let last_activity = item.get("last_activity")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| DynamoDbError::InvalidSessionData("Invalid last_activity".to_string()))?;

        let is_initialized = item.get("is_initialized")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let metadata = item.get("metadata")
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
}

#[async_trait]
impl SessionStorage for DynamoDbSessionStorage {
    type Error = SessionStorageError;

    async fn create_session(&self, capabilities: ServerCapabilities) -> Result<SessionInfo, Self::Error> {
        let mut session = SessionInfo::new();
        session.server_capabilities = Some(capabilities);
        
        #[cfg(feature = "dynamodb")]
        {
            let item = self.session_to_dynamodb_item(&session)?;
            
            match self.client.put_item()
                .table_name(&self.config.table_name)
                .set_item(Some(item))
                .send()
                .await 
            {
                Ok(_) => {
                    debug!("Successfully created session in DynamoDB: {}", session.session_id);
                }
                Err(err) => {
                    error!("Failed to create session in DynamoDB: {}", err);
                    return Err(SessionStorageError::DatabaseError(format!(
                        "Failed to create session: {}", err
                    )));
                }
            }
        }
        
        #[cfg(not(feature = "dynamodb"))]
        {
            debug!("Creating session in DynamoDB (placeholder): {}", session.session_id);
        }
        
        Ok(session)
    }

    async fn create_session_with_id(&self, session_id: String, capabilities: ServerCapabilities) -> Result<SessionInfo, Self::Error> {
        let mut session = SessionInfo::with_id(session_id.clone());
        session.server_capabilities = Some(capabilities);
        
        #[cfg(feature = "dynamodb")]
        {
            let item = self.session_to_dynamodb_item(&session)?;
            
            match self.client.put_item()
                .table_name(&self.config.table_name)
                .set_item(Some(item))
                .send()
                .await 
            {
                Ok(_) => {
                    debug!("Successfully created session with ID in DynamoDB: {}", session_id);
                }
                Err(err) => {
                    error!("Failed to create session with ID in DynamoDB: {}", err);
                    return Err(SessionStorageError::DatabaseError(format!(
                        "Failed to create session with ID '{}': {}", session_id, err
                    )));
                }
            }
        }
        
        #[cfg(not(feature = "dynamodb"))]
        {
            debug!("Creating session with ID in DynamoDB (placeholder): {}", session_id);
        }
        
        Ok(session)
    }

    async fn get_session(&self, session_id: &str) -> Result<Option<SessionInfo>, Self::Error> {
        #[cfg(feature = "dynamodb")]
        {
            use aws_sdk_dynamodb::types::AttributeValue;
            
            let key = HashMap::from([
                ("session_id".to_string(), AttributeValue::S(session_id.to_string()))
            ]);
            
            match self.client.get_item()
                .table_name(&self.config.table_name)
                .set_key(Some(key))
                .send()
                .await 
            {
                Ok(output) => {
                    if let Some(item) = output.item() {
                        let session = self.dynamodb_item_to_session(item)?;
                        debug!("Successfully retrieved session from DynamoDB: {}", session_id);
                        Ok(Some(session))
                    } else {
                        debug!("Session not found in DynamoDB: {}", session_id);
                        Ok(None)
                    }
                }
                Err(err) => {
                    error!("Failed to get session from DynamoDB: {}", err);
                    Err(SessionStorageError::DatabaseError(format!(
                        "Failed to get session '{}': {}", session_id, err
                    )))
                }
            }
        }
        
        #[cfg(not(feature = "dynamodb"))]
        {
            debug!("Getting session from DynamoDB (placeholder): {}", session_id);
            Ok(None)
        }
    }

    async fn update_session(&self, session_info: SessionInfo) -> Result<(), Self::Error> {
        #[cfg(feature = "dynamodb")]
        {
            let item = self.session_to_dynamodb_item(&session_info)?;
            
            match self.client.put_item()
                .table_name(&self.config.table_name)
                .set_item(Some(item))
                .send()
                .await 
            {
                Ok(_) => {
                    debug!("Successfully updated session in DynamoDB: {}", session_info.session_id);
                    Ok(())
                }
                Err(err) => {
                    error!("Failed to update session in DynamoDB: {}", err);
                    Err(SessionStorageError::DatabaseError(format!(
                        "Failed to update session '{}': {}", session_info.session_id, err
                    )))
                }
            }
        }
        
        #[cfg(not(feature = "dynamodb"))]
        {
            debug!("Updating session in DynamoDB (placeholder): {}", session_info.session_id);
            Ok(())
        }
    }

    async fn set_session_state(&self, session_id: &str, key: &str, value: Value) -> Result<(), Self::Error> {
        #[cfg(feature = "dynamodb")]
        {
            use aws_sdk_dynamodb::types::AttributeValue;
            
            // Convert serde_json::Value to JSON string for DynamoDB storage
            let value_json = serde_json::to_string(&value)
                .map_err(|e| SessionStorageError::SerializationError(e.to_string()))?;
            
            let session_key = HashMap::from([
                ("session_id".to_string(), AttributeValue::S(session_id.to_string()))
            ]);
            
            // Use UpdateExpression to set a nested attribute in the state map
            let update_expression = format!("SET #state.#key = :value, #last_activity = :timestamp");
            let expression_attribute_names = HashMap::from([
                ("#state".to_string(), "state".to_string()),
                ("#key".to_string(), key.to_string()),
                ("#last_activity".to_string(), "last_activity".to_string()),
            ]);
            let expression_attribute_values = HashMap::from([
                (":value".to_string(), AttributeValue::S(value_json)),
                (":timestamp".to_string(), AttributeValue::N(
                    chrono::Utc::now().timestamp_millis().to_string()
                )),
            ]);
            
            match self.client.update_item()
                .table_name(&self.config.table_name)
                .set_key(Some(session_key))
                .update_expression(update_expression)
                .set_expression_attribute_names(Some(expression_attribute_names))
                .set_expression_attribute_values(Some(expression_attribute_values))
                .send()
                .await 
            {
                Ok(_) => {
                    debug!("Successfully set session state in DynamoDB: {} -> {}", session_id, key);
                    Ok(())
                }
                Err(err) => {
                    error!("Failed to set session state in DynamoDB: {}", err);
                    Err(SessionStorageError::DatabaseError(format!(
                        "Failed to set session state '{}' -> '{}': {}", session_id, key, err
                    )))
                }
            }
        }
        
        #[cfg(not(feature = "dynamodb"))]
        {
            debug!("Setting session state in DynamoDB (placeholder): {} -> {}", session_id, key);
            Ok(())
        }
    }

    async fn get_session_state(&self, session_id: &str, key: &str) -> Result<Option<Value>, Self::Error> {
        #[cfg(feature = "dynamodb")]
        {
            // Get the entire session and extract the state value
            if let Some(session) = self.get_session(session_id).await? {
                if let Some(value) = session.state.get(key) {
                    debug!("Successfully retrieved session state from DynamoDB: {} -> {}", session_id, key);
                    Ok(Some(value.clone()))
                } else {
                    debug!("Session state key not found in DynamoDB: {} -> {}", session_id, key);
                    Ok(None)
                }
            } else {
                debug!("Session not found for state retrieval in DynamoDB: {}", session_id);
                Ok(None)
            }
        }
        
        #[cfg(not(feature = "dynamodb"))]
        {
            debug!("Getting session state from DynamoDB (placeholder): {} -> {}", session_id, key);
            Ok(None)
        }
    }

    async fn remove_session_state(&self, session_id: &str, key: &str) -> Result<Option<Value>, Self::Error> {
        #[cfg(feature = "dynamodb")]
        {
            use aws_sdk_dynamodb::types::AttributeValue;
            
            // First get the current value
            let current_value = self.get_session_state(session_id, key).await?;
            
            if current_value.is_some() {
                let session_key = HashMap::from([
                    ("session_id".to_string(), AttributeValue::S(session_id.to_string()))
                ]);
                
                // Use UpdateExpression to remove the key from the state map
                let update_expression = format!("REMOVE #state.#key SET #last_activity = :timestamp");
                let expression_attribute_names = HashMap::from([
                    ("#state".to_string(), "state".to_string()),
                    ("#key".to_string(), key.to_string()),
                    ("#last_activity".to_string(), "last_activity".to_string()),
                ]);
                let expression_attribute_values = HashMap::from([
                    (":timestamp".to_string(), AttributeValue::N(
                        chrono::Utc::now().timestamp_millis().to_string()
                    )),
                ]);
                
                match self.client.update_item()
                    .table_name(&self.config.table_name)
                    .set_key(Some(session_key))
                    .update_expression(update_expression)
                    .set_expression_attribute_names(Some(expression_attribute_names))
                    .set_expression_attribute_values(Some(expression_attribute_values))
                    .send()
                    .await 
                {
                    Ok(_) => {
                        debug!("Successfully removed session state from DynamoDB: {} -> {}", session_id, key);
                        Ok(current_value)
                    }
                    Err(err) => {
                        error!("Failed to remove session state from DynamoDB: {}", err);
                        Err(SessionStorageError::DatabaseError(format!(
                            "Failed to remove session state '{}' -> '{}': {}", session_id, key, err
                        )))
                    }
                }
            } else {
                debug!("Session state key not found for removal in DynamoDB: {} -> {}", session_id, key);
                Ok(None)
            }
        }
        
        #[cfg(not(feature = "dynamodb"))]
        {
            debug!("Removing session state from DynamoDB (placeholder): {} -> {}", session_id, key);
            Ok(None)
        }
    }

    async fn delete_session(&self, session_id: &str) -> Result<bool, Self::Error> {
        #[cfg(feature = "dynamodb")]
        {
            use aws_sdk_dynamodb::types::AttributeValue;
            
            let key = HashMap::from([
                ("session_id".to_string(), AttributeValue::S(session_id.to_string()))
            ]);
            
            match self.client.delete_item()
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
                        "Failed to delete session '{}': {}", session_id, err
                    )))
                }
            }
        }
        
        #[cfg(not(feature = "dynamodb"))]
        {
            debug!("Deleting session from DynamoDB (placeholder): {}", session_id);
            Ok(true)
        }
    }

    async fn list_sessions(&self) -> Result<Vec<String>, Self::Error> {
        #[cfg(feature = "dynamodb")]
        {
            debug!("Scanning DynamoDB table for all session IDs");
            
            // Note: Scan is expensive for large tables - consider using pagination
            match self.client.scan()
                .table_name(&self.config.table_name)
                .projection_expression("session_id")
                .send()
                .await 
            {
                Ok(output) => {
                    let mut session_ids = Vec::new();
                    
                    for item in output.items() {
                        if let Some(session_id_attr) = item.get("session_id") {
                            if let Ok(session_id) = session_id_attr.as_s() {
                                session_ids.push(session_id.clone());
                            }
                        }
                    }
                    
                    debug!("Listed {} session IDs from DynamoDB", session_ids.len());
                    Ok(session_ids)
                }
                Err(err) => {
                    error!("Failed to list sessions from DynamoDB: {}", err);
                    Err(SessionStorageError::DatabaseError(format!(
                        "Failed to list sessions: {}", err
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

    async fn store_event(&self, session_id: &str, mut event: SseEvent) -> Result<SseEvent, Self::Error> {
        // Assign unique event ID
        event.id = self.event_counter.fetch_add(1, Ordering::SeqCst);
        
        #[cfg(feature = "dynamodb")]
        {
            use aws_sdk_dynamodb::types::AttributeValue;
            
            // For DynamoDB, we'll store events in a separate table or as items with composite keys
            // Here we use a composite key approach: session_id (partition) + event_id (sort)
            let event_table = format!("{}-events", self.config.table_name);
            
            let _item = HashMap::from([
                ("session_id".to_string(), AttributeValue::S(session_id.to_string())),
                ("event_id".to_string(), AttributeValue::N(event.id.to_string())),
                ("timestamp".to_string(), AttributeValue::N(event.timestamp.to_string())),
                ("event_type".to_string(), AttributeValue::S(event.event_type.clone())),
                ("data".to_string(), AttributeValue::S(serde_json::to_string(&event.data)?)),
                ("retry".to_string(), match event.retry {
                    Some(retry) => AttributeValue::N(retry.to_string()),
                    None => AttributeValue::Null(true)
                }),
                // TTL for event cleanup
                ("ttl".to_string(), AttributeValue::N(
                    (SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() 
                     + (self.config.event_ttl_hours * 3600)).to_string()
                ))
            ]);
            
            // Note: In a production setup, you'd want a separate events table
            // For now, we'll log what we would do
            debug!("Would store SSE event in DynamoDB table '{}': {} -> {}", event_table, session_id, event.id);
        }
        
        #[cfg(not(feature = "dynamodb"))]
        {
            debug!("Storing SSE event in DynamoDB (placeholder): {} -> {}", session_id, event.id);
        }
        
        Ok(event)
    }

    async fn get_events_after(&self, session_id: &str, after_event_id: u64) -> Result<Vec<SseEvent>, Self::Error> {
        #[cfg(feature = "dynamodb")]
        {
            // In a production setup, this would query a separate events table
            // using the session_id as partition key and event_id > after_event_id as condition
            debug!("Would query events after {} from DynamoDB events table for session: {}", after_event_id, session_id);
            
            // For now, return empty as we're not implementing the full events table
            Ok(vec![])
        }
        
        #[cfg(not(feature = "dynamodb"))]
        {
            debug!("Getting events after {} from DynamoDB (placeholder): {}", after_event_id, session_id);
            Ok(vec![])
        }
    }

    async fn get_recent_events(&self, session_id: &str, limit: usize) -> Result<Vec<SseEvent>, Self::Error> {
        #[cfg(feature = "dynamodb")]
        {
            // In a production setup, this would query the most recent N events
            // from the events table for the given session_id, ordered by event_id DESC
            debug!("Would query {} recent events from DynamoDB events table for session: {}", limit, session_id);
            
            // For now, return empty as we're not implementing the full events table
            Ok(vec![])
        }
        
        #[cfg(not(feature = "dynamodb"))]
        {
            debug!("Getting {} recent events from DynamoDB (placeholder): {}", limit, session_id);
            Ok(vec![])
        }
    }

    async fn delete_events_before(&self, session_id: &str, before_event_id: u64) -> Result<u64, Self::Error> {
        #[cfg(feature = "dynamodb")]
        {
            // In a production setup, this would scan and delete events
            // where session_id = session_id AND event_id < before_event_id
            debug!("Would delete events before {} from DynamoDB events table for session: {}", before_event_id, session_id);
            
            // For now, return 0 as we're not implementing the full events table
            Ok(0)
        }
        
        #[cfg(not(feature = "dynamodb"))]
        {
            debug!("Deleting events before {} from DynamoDB (placeholder): {}", before_event_id, session_id);
            Ok(0)
        }
    }

    async fn expire_sessions(&self, older_than: SystemTime) -> Result<Vec<String>, Self::Error> {
        #[cfg(feature = "dynamodb")]
        {
            let timestamp = older_than.duration_since(SystemTime::UNIX_EPOCH)
                .unwrap().as_secs();
            
            // DynamoDB TTL handles automatic deletion, but for manual cleanup:
            // We would scan for sessions where last_activity < timestamp
            debug!("Would scan for sessions with last_activity < {} in DynamoDB", timestamp);
            
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
            let timestamp = older_than.duration_since(SystemTime::UNIX_EPOCH)
                .unwrap().as_secs();
            debug!("Expiring sessions older than {} from DynamoDB (placeholder)", timestamp);
            Ok(vec![])
        }
    }

    async fn session_count(&self) -> Result<usize, Self::Error> {
        #[cfg(feature = "dynamodb")]
        {
            // In production, this would use scan with count-only projection
            // or maintain a counter in a separate item
            debug!("Would scan DynamoDB table to count sessions");
            
            match self.client.scan()
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
                        "Failed to count sessions: {}", err
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
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mcp_protocol::ServerCapabilities;

    #[tokio::test]
    async fn test_dynamodb_config() {
        let config = DynamoDbConfig::default();
        assert_eq!(config.table_name, "mcp-sessions");
        assert_eq!(config.region, "us-east-1");
        assert_eq!(config.session_ttl_hours, 24);
    }

    #[tokio::test]
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