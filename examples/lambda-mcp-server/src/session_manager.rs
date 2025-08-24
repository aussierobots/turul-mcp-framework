//! Session Management for Lambda MCP Server
//!
//! DynamoDB-backed session persistence with TTL, cleanup, and MCP protocol compliance.

use aws_sdk_dynamodb::{Client as DynamoClient, Error as DynamoError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Session management for MCP protocol compliance
#[derive(Debug, Clone)]
pub struct SessionManager {
    /// DynamoDB client
    dynamo_client: DynamoClient,
    /// Session table name
    table_name: String,
    /// Session configuration
    config: SessionConfig,
}

/// Session configuration from mcp_config.json
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Default session TTL in seconds
    pub default_ttl_seconds: u64,
    /// Maximum idle time before cleanup
    pub max_idle_seconds: u64,
    /// Cleanup interval in seconds
    pub cleanup_interval_seconds: u64,
    /// Maximum sessions per client
    pub max_sessions_per_client: u32,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            default_ttl_seconds: 1800, // 30 minutes
            max_idle_seconds: 300,     // 5 minutes
            cleanup_interval_seconds: 60, // 1 minute
            max_sessions_per_client: 10,
        }
    }
}

/// MCP Session data structure for DynamoDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSession {
    /// Primary key: session_id
    pub session_id: String,
    /// Client information from MCP initialize
    pub client_info: Option<Value>,
    /// Negotiated client capabilities
    pub client_capabilities: Value,
    /// Session status
    pub status: SessionStatus,
    /// Session statistics
    pub statistics: SessionStatistics,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,
    /// TTL for automatic cleanup (Unix timestamp)
    pub ttl: u64,
    /// Protocol version negotiated
    pub protocol_version: String,
}

/// Session status enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    /// Session created but not initialized
    Created,
    /// Session initialized and active
    Active,
    /// Session idle but valid
    Idle,
    /// Session expired or invalid
    Expired,
}

/// Session statistics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionStatistics {
    /// Number of tool calls made
    pub tool_calls: u64,
    /// Number of resource accesses
    pub resource_accesses: u64,
    /// Number of streaming responses sent
    pub streaming_responses: u64,
    /// Number of notifications received
    pub notifications_received: u64,
}

impl SessionManager {
    /// Create new session manager
    pub async fn new() -> Result<Self, DynamoError> {
        info!("🔗 Initializing DynamoDB session manager...");
        
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest()).load().await;
        let dynamo_client = DynamoClient::new(&config);
        
        let table_name = std::env::var("SESSION_TABLE_NAME")
            .unwrap_or_else(|_| "mcp-sessions".to_string());
        
        // Test DynamoDB connection
        match dynamo_client.list_tables().limit(1).send().await {
            Ok(_) => {
                info!("✅ DynamoDB connection established successfully");
                info!("📊 Session table: {}", table_name);
            }
            Err(e) => {
                warn!("⚠️  DynamoDB connection test failed: {:?} (continuing anyway)", e);
            }
        }
        
        let session_config = SessionConfig::default();
        info!("⚙️  Session config: TTL={}s, Max idle={}s", 
              session_config.default_ttl_seconds, 
              session_config.max_idle_seconds);
        
        info!("✅ SessionManager initialized with table: {}", table_name);
        
        Ok(Self {
            dynamo_client,
            table_name,
            config: session_config,
        })
    }

    /// Create new session manager for testing
    #[cfg(test)]
    pub fn new_for_test(client: DynamoClient, table_name: String) -> Self {
        Self {
            dynamo_client: client,
            table_name,
            config: SessionConfig::default(),
        }
    }

    /// Generate a new MCP session ID
    pub fn generate_session_id() -> String {
        Uuid::now_v7().to_string()
    }

    /// Create a new MCP session
    pub async fn create_session(
        &self,
        session_id: &str,
        client_capabilities: Value,
        client_info: Option<Value>,
    ) -> Result<McpSession, DynamoError> {
        let now = Utc::now();
        let ttl = (now.timestamp() as u64) + self.config.default_ttl_seconds;
        
        let session = McpSession {
            session_id: session_id.to_string(),
            client_info,
            client_capabilities,
            status: SessionStatus::Created,
            statistics: SessionStatistics::default(),
            created_at: now,
            last_activity: now,
            ttl,
            protocol_version: "2025-06-18".to_string(),
        };

        // Store in DynamoDB
        let mut item = HashMap::new();
        item.insert("session_id".to_string(), aws_sdk_dynamodb::types::AttributeValue::S(session.session_id.clone()));
        item.insert("client_capabilities".to_string(), aws_sdk_dynamodb::types::AttributeValue::S(serde_json::to_string(&session.client_capabilities).unwrap()));
        item.insert("status".to_string(), aws_sdk_dynamodb::types::AttributeValue::S("created".to_string()));
        item.insert("created_at".to_string(), aws_sdk_dynamodb::types::AttributeValue::S(session.created_at.to_rfc3339()));
        item.insert("last_activity".to_string(), aws_sdk_dynamodb::types::AttributeValue::S(session.last_activity.to_rfc3339()));
        item.insert("ttl".to_string(), aws_sdk_dynamodb::types::AttributeValue::N(session.ttl.to_string()));
        item.insert("protocol_version".to_string(), aws_sdk_dynamodb::types::AttributeValue::S(session.protocol_version.clone()));
        item.insert("statistics".to_string(), aws_sdk_dynamodb::types::AttributeValue::S(serde_json::to_string(&session.statistics).unwrap()));
        
        if let Some(ref client_info) = session.client_info {
            item.insert("client_info".to_string(), aws_sdk_dynamodb::types::AttributeValue::S(serde_json::to_string(client_info).unwrap()));
        }

        self.dynamo_client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .send()
            .await?;

        info!("Created MCP session: {}", session_id);
        Ok(session)
    }

    /// Mark session as initialized (active)
    pub async fn mark_session_initialized(&self, session_id: &str) -> Result<(), DynamoError> {
        let now = Utc::now();
        
        self.dynamo_client
            .update_item()
            .table_name(&self.table_name)
            .key("session_id", aws_sdk_dynamodb::types::AttributeValue::S(session_id.to_string()))
            .update_expression("SET #status = :status, last_activity = :activity")
            .expression_attribute_names("#status", "status")
            .expression_attribute_values(":status", aws_sdk_dynamodb::types::AttributeValue::S("active".to_string()))
            .expression_attribute_values(":activity", aws_sdk_dynamodb::types::AttributeValue::S(now.to_rfc3339()))
            .send()
            .await?;

        debug!("Marked session as initialized: {}", session_id);
        Ok(())
    }

    /// Update session activity timestamp
    pub async fn update_session_activity(&self, session_id: &str) -> Result<(), DynamoError> {
        let now = Utc::now();
        
        self.dynamo_client
            .update_item()
            .table_name(&self.table_name)
            .key("session_id", aws_sdk_dynamodb::types::AttributeValue::S(session_id.to_string()))
            .update_expression("SET last_activity = :activity")
            .expression_attribute_values(":activity", aws_sdk_dynamodb::types::AttributeValue::S(now.to_rfc3339()))
            .send()
            .await?;

        debug!("Updated session activity: {}", session_id);
        Ok(())
    }

    /// Get session information
    pub async fn get_session(&self, session_id: &str) -> Result<Option<McpSession>, DynamoError> {
        let result = self.dynamo_client
            .get_item()
            .table_name(&self.table_name)
            .key("session_id", aws_sdk_dynamodb::types::AttributeValue::S(session_id.to_string()))
            .send()
            .await?;

        if let Some(item) = result.item {
            // Parse DynamoDB item back to McpSession
            let session = self.parse_session_from_item(item)?;
            Ok(Some(session))
        } else {
            Ok(None)
        }
    }

    /// Increment session statistics
    pub async fn increment_tool_calls(&self, session_id: &str) -> Result<(), DynamoError> {
        self.dynamo_client
            .update_item()
            .table_name(&self.table_name)
            .key("session_id", aws_sdk_dynamodb::types::AttributeValue::S(session_id.to_string()))
            .update_expression("ADD statistics.tool_calls :inc SET last_activity = :activity")
            .expression_attribute_values(":inc", aws_sdk_dynamodb::types::AttributeValue::N("1".to_string()))
            .expression_attribute_values(":activity", aws_sdk_dynamodb::types::AttributeValue::S(Utc::now().to_rfc3339()))
            .send()
            .await?;

        debug!("Incremented tool calls for session: {}", session_id);
        Ok(())
    }

    /// List active sessions (for admin tools)
    pub async fn list_active_sessions(&self, limit: Option<i32>) -> Result<Vec<McpSession>, DynamoError> {
        let mut scan = self.dynamo_client
            .scan()
            .table_name(&self.table_name)
            .filter_expression("#status = :status")
            .expression_attribute_names("#status", "status")
            .expression_attribute_values(":status", aws_sdk_dynamodb::types::AttributeValue::S("active".to_string()));

        if let Some(limit) = limit {
            scan = scan.limit(limit);
        }

        let result = scan.send().await?;
        
        let mut sessions = Vec::new();
        if let Some(items) = result.items {
            for item in items {
                if let Ok(session) = self.parse_session_from_item(item) {
                    sessions.push(session);
                }
            }
        }

        info!("Listed {} active sessions", sessions.len());
        Ok(sessions)
    }

    /// Delete expired sessions (cleanup)
    pub async fn cleanup_expired_sessions(&self) -> Result<u32, DynamoError> {
        let now_timestamp = Utc::now().timestamp() as u64;
        
        // Scan for expired sessions
        let result = self.dynamo_client
            .scan()
            .table_name(&self.table_name)
            .filter_expression("ttl < :now")
            .expression_attribute_values(":now", aws_sdk_dynamodb::types::AttributeValue::N(now_timestamp.to_string()))
            .send()
            .await?;

        let mut cleanup_count = 0;
        if let Some(items) = result.items {
            for item in items {
                if let Some(session_id) = item.get("session_id") {
                    if let aws_sdk_dynamodb::types::AttributeValue::S(id) = session_id {
                        match self.delete_session(id).await {
                            Ok(_) => cleanup_count += 1,
                            Err(e) => warn!("Failed to delete expired session {}: {:?}", id, e),
                        }
                    }
                }
            }
        }

        if cleanup_count > 0 {
            info!("Cleaned up {} expired sessions", cleanup_count);
        }
        Ok(cleanup_count)
    }

    /// Delete a specific session
    pub async fn delete_session(&self, session_id: &str) -> Result<(), DynamoError> {
        self.dynamo_client
            .delete_item()
            .table_name(&self.table_name)
            .key("session_id", aws_sdk_dynamodb::types::AttributeValue::S(session_id.to_string()))
            .send()
            .await?;

        debug!("Deleted session: {}", session_id);
        Ok(())
    }

    /// Parse DynamoDB item to McpSession
    fn parse_session_from_item(&self, item: HashMap<String, aws_sdk_dynamodb::types::AttributeValue>) -> Result<McpSession, DynamoError> {
        let session_id = item.get("session_id")
            .and_then(|v| v.as_s().ok())
            .ok_or_else(|| {
                DynamoError::InternalServerError(
                    aws_sdk_dynamodb::types::error::InternalServerError::builder()
                        .message("Missing session_id")
                        .build()
                )
            })?
            .clone();

        let client_capabilities = item.get("client_capabilities")
            .and_then(|v| v.as_s().ok())
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_else(|| serde_json::json!({}));

        let client_info = item.get("client_info")
            .and_then(|v| v.as_s().ok())
            .and_then(|s| serde_json::from_str(s).ok());

        let status_str = item.get("status")
            .and_then(|v| v.as_s().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "created".to_string());
        
        let status = match status_str.as_str() {
            "active" => SessionStatus::Active,
            "idle" => SessionStatus::Idle,
            "expired" => SessionStatus::Expired,
            _ => SessionStatus::Created,
        };

        let created_at = item.get("created_at")
            .and_then(|v| v.as_s().ok())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        let last_activity = item.get("last_activity")
            .and_then(|v| v.as_s().ok())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        let ttl = item.get("ttl")
            .and_then(|v| v.as_n().ok())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let protocol_version = item.get("protocol_version")
            .and_then(|v| v.as_s().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "2025-06-18".to_string());

        let statistics = item.get("statistics")
            .and_then(|v| v.as_s().ok())
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        Ok(McpSession {
            session_id,
            client_info,
            client_capabilities,
            status,
            statistics,
            created_at,
            last_activity,
            ttl,
            protocol_version,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_session_config_default() {
        let config = SessionConfig::default();
        assert_eq!(config.default_ttl_seconds, 1800);
        assert_eq!(config.max_idle_seconds, 300);
        assert_eq!(config.cleanup_interval_seconds, 60);
        assert_eq!(config.max_sessions_per_client, 10);
    }

    #[test]
    fn test_generate_session_id() {
        let id1 = SessionManager::generate_session_id();
        let id2 = SessionManager::generate_session_id();
        
        assert_ne!(id1, id2);
        assert!(Uuid::parse_str(&id1).is_ok());
        assert!(Uuid::parse_str(&id2).is_ok());
    }

    #[test]
    fn test_session_statistics_default() {
        let stats = SessionStatistics::default();
        assert_eq!(stats.tool_calls, 0);
        assert_eq!(stats.resource_accesses, 0);
        assert_eq!(stats.streaming_responses, 0);
        assert_eq!(stats.notifications_received, 0);
    }
}