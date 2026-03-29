//! AWS DynamoDB server state storage implementation.
//!
//! Serverless-native backend for persistent server-global entity state.
//! Ideal for AWS deployments, Lambda functions, and multi-instance clusters.
//!
//! ## Table Schema
//!
//! Single table with camelCase attribute names:
//! - **Partition key**: `entityType` (String) — e.g., `"tools"`, `"resources"`, `"prompts"`
//! - **Sort key**: `entityId` (String) — entity name or `"#fingerprint"` for fingerprint records
//! - **Attributes**: `active` (Boolean), `metadata` (String/JSON), `updatedAt` (String/ISO 8601)
//! - **Fingerprint records**: `entityId = "#fingerprint"`, `fingerprint` (String), `updatedAt` (String)

use async_trait::async_trait;
use aws_config::{BehaviorVersion, Region};
use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_dynamodb::Client;
use tracing::{debug, error, info, warn};

use crate::error::ServerStateError;
use crate::traits::{EntityState, RegistrySnapshot, ServerStateStorage};

/// Special sort key value used to store fingerprint records.
const FINGERPRINT_ENTITY_ID: &str = "#fingerprint";

/// Configuration for DynamoDB server state storage.
#[derive(Debug, Clone)]
pub struct DynamoDbServerStateConfig {
    /// DynamoDB table name.
    pub table_name: String,
    /// AWS region (defaults to `AWS_REGION` env var or `"us-east-1"`).
    pub region: String,
    /// Auto-create the table if it does not exist (default: false).
    pub create_table: bool,
}

impl Default for DynamoDbServerStateConfig {
    fn default() -> Self {
        Self {
            table_name: "mcp-server-state".to_string(),
            region: std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
            create_table: false,
        }
    }
}

/// DynamoDB-backed server state storage.
///
/// Uses a single-table design with `entityType` (partition key) and `entityId`
/// (sort key). Fingerprints are stored as special items with `entityId = "#fingerprint"`.
pub struct DynamoDbServerStateStorage {
    client: Client,
    table_name: String,
}

impl DynamoDbServerStateStorage {
    /// Create with default configuration.
    pub async fn new() -> Result<Self, ServerStateError> {
        Self::with_config(DynamoDbServerStateConfig::default()).await
    }

    /// Create with custom configuration.
    pub async fn with_config(config: DynamoDbServerStateConfig) -> Result<Self, ServerStateError> {
        info!(
            "Initializing DynamoDB server state storage: table={}, region={}",
            config.table_name, config.region
        );

        let aws_config = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(config.region.clone()))
            .load()
            .await;

        let client = Client::new(&aws_config);

        let storage = Self {
            client,
            table_name: config.table_name.clone(),
        };

        if config.create_table {
            storage.ensure_table().await?;
        }

        info!(
            "DynamoDB server state storage initialized: table={}",
            config.table_name
        );
        Ok(storage)
    }

    /// Create with a pre-configured AWS SDK client (useful for testing/custom endpoints).
    pub fn with_client(client: Client, table_name: String) -> Self {
        Self { client, table_name }
    }

    /// Convert an AWS SDK error to a ServerStateError with full error chain.
    /// Uses Debug formatting ({:?}) instead of Display to surface the error code,
    /// message, HTTP status, and request ID — not just generic "service error".
    fn dynamo_err_debug(e: impl std::fmt::Debug) -> ServerStateError {
        ServerStateError::DatabaseError(format!("{e:?}"))
    }

    /// Ensure the table exists, creating it if necessary.
    async fn ensure_table(&self) -> Result<(), ServerStateError> {
        match self
            .client
            .describe_table()
            .table_name(&self.table_name)
            .send()
            .await
        {
            Ok(output) => {
                if let Some(table) = output.table() {
                    if let Some(status) = table.table_status() {
                        use aws_sdk_dynamodb::types::TableStatus;
                        match status {
                            TableStatus::Active => {
                                info!(
                                    "DynamoDB table '{}' is active and ready",
                                    self.table_name
                                );
                                Ok(())
                            }
                            _ => {
                                info!(
                                    "DynamoDB table '{}' exists but status is {:?}, waiting...",
                                    self.table_name, status
                                );
                                self.wait_for_table_active().await
                            }
                        }
                    } else {
                        Err(ServerStateError::ConfigError(format!(
                            "Table '{}' status unknown",
                            self.table_name
                        )))
                    }
                } else {
                    Err(ServerStateError::ConfigError(format!(
                        "Table '{}' description not found",
                        self.table_name
                    )))
                }
            }
            Err(_) => {
                warn!(
                    "Table '{}' does not exist, creating it",
                    self.table_name
                );
                self.create_table().await?;
                self.wait_for_table_active().await
            }
        }
    }

    /// Create the DynamoDB table with camelCase key schema.
    async fn create_table(&self) -> Result<(), ServerStateError> {
        use aws_sdk_dynamodb::types::{
            AttributeDefinition, BillingMode, KeySchemaElement, KeyType, ScalarAttributeType,
        };

        info!("Creating DynamoDB table: {}", self.table_name);

        let key_schema = vec![
            KeySchemaElement::builder()
                .attribute_name("entityType")
                .key_type(KeyType::Hash)
                .build()
                .map_err(Self::dynamo_err_debug)?,
            KeySchemaElement::builder()
                .attribute_name("entityId")
                .key_type(KeyType::Range)
                .build()
                .map_err(Self::dynamo_err_debug)?,
        ];

        let attribute_definitions = vec![
            AttributeDefinition::builder()
                .attribute_name("entityType")
                .attribute_type(ScalarAttributeType::S)
                .build()
                .map_err(Self::dynamo_err_debug)?,
            AttributeDefinition::builder()
                .attribute_name("entityId")
                .attribute_type(ScalarAttributeType::S)
                .build()
                .map_err(Self::dynamo_err_debug)?,
        ];

        match self
            .client
            .create_table()
            .table_name(&self.table_name)
            .set_key_schema(Some(key_schema))
            .set_attribute_definitions(Some(attribute_definitions))
            .billing_mode(BillingMode::PayPerRequest)
            .send()
            .await
        {
            Ok(_) => {
                info!(
                    "Successfully initiated table creation: {}",
                    self.table_name
                );
                Ok(())
            }
            Err(err) => {
                error!(
                    "Failed to create table '{}': {}",
                    self.table_name, err
                );
                Err(ServerStateError::DatabaseError(format!(
                    "Failed to create table '{}': {}",
                    self.table_name, err
                )))
            }
        }
    }

    /// Wait for the table to become ACTIVE (polls every 500ms, up to 30s).
    async fn wait_for_table_active(&self) -> Result<(), ServerStateError> {
        use aws_sdk_dynamodb::types::TableStatus;

        let max_attempts = 60;
        for attempt in 1..=max_attempts {
            match self
                .client
                .describe_table()
                .table_name(&self.table_name)
                .send()
                .await
            {
                Ok(output) => {
                    if let Some(table) = output.table() {
                        if table.table_status() == Some(&TableStatus::Active) {
                            info!(
                                "Table '{}' is now ACTIVE (attempt {})",
                                self.table_name, attempt
                            );
                            return Ok(());
                        }
                    }
                }
                Err(err) => {
                    debug!(
                        "DescribeTable attempt {} for '{}': {}",
                        attempt, self.table_name, err
                    );
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        Err(ServerStateError::DatabaseError(format!(
            "Table '{}' did not become ACTIVE within 30s",
            self.table_name
        )))
    }

    /// Extract a string attribute from a DynamoDB item.
    fn get_s(item: &std::collections::HashMap<String, AttributeValue>, key: &str) -> Option<String> {
        item.get(key).and_then(|v| v.as_s().ok()).cloned()
    }

    /// Extract a boolean attribute from a DynamoDB item.
    fn get_bool(item: &std::collections::HashMap<String, AttributeValue>, key: &str) -> Option<bool> {
        item.get(key).and_then(|v| v.as_bool().ok()).copied()
    }

    /// Parse a DynamoDB item into an EntityState.
    fn item_to_entity_state(
        item: &std::collections::HashMap<String, AttributeValue>,
    ) -> Option<EntityState> {
        let entity_id = Self::get_s(item, "entityId")?;
        let active = Self::get_bool(item, "active").unwrap_or(false);
        let updated_at = Self::get_s(item, "updatedAt").unwrap_or_default();

        let metadata = Self::get_s(item, "metadata").and_then(|s| {
            serde_json::from_str(&s).ok()
        });

        Some(EntityState {
            entity_id,
            active,
            metadata,
            updated_at,
        })
    }
}

#[async_trait]
impl ServerStateStorage for DynamoDbServerStateStorage {
    fn backend_name(&self) -> &'static str {
        "DynamoDB"
    }

    async fn get_entity_state(
        &self,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Option<EntityState>, ServerStateError> {
        debug!("DynamoDB get_entity_state: {}/{}", entity_type, entity_id);

        let result = self
            .client
            .get_item()
            .table_name(&self.table_name)
            .key("entityType", AttributeValue::S(entity_type.to_string()))
            .key("entityId", AttributeValue::S(entity_id.to_string()))
            .send()
            .await
            .map_err(Self::dynamo_err_debug)?;

        Ok(result.item().and_then(Self::item_to_entity_state))
    }

    async fn set_entity_state(
        &self,
        entity_type: &str,
        entity_id: &str,
        state: EntityState,
    ) -> Result<(), ServerStateError> {
        debug!("DynamoDB set_entity_state: {}/{}", entity_type, entity_id);

        let mut request = self
            .client
            .put_item()
            .table_name(&self.table_name)
            .item("entityType", AttributeValue::S(entity_type.to_string()))
            .item("entityId", AttributeValue::S(entity_id.to_string()))
            .item("active", AttributeValue::Bool(state.active))
            .item("updatedAt", AttributeValue::S(state.updated_at));

        if let Some(metadata) = &state.metadata {
            let json = serde_json::to_string(metadata)?;
            request = request.item("metadata", AttributeValue::S(json));
        }

        request
            .send()
            .await
            .map_err(Self::dynamo_err_debug)?;

        Ok(())
    }

    async fn delete_entity_state(
        &self,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<(), ServerStateError> {
        debug!("DynamoDB delete_entity_state: {}/{}", entity_type, entity_id);

        self.client
            .delete_item()
            .table_name(&self.table_name)
            .key("entityType", AttributeValue::S(entity_type.to_string()))
            .key("entityId", AttributeValue::S(entity_id.to_string()))
            .send()
            .await
            .map_err(Self::dynamo_err_debug)?;

        Ok(())
    }

    async fn get_active_entities(
        &self,
        entity_type: &str,
    ) -> Result<Vec<String>, ServerStateError> {
        debug!("DynamoDB get_active_entities: {}", entity_type);

        let result = self
            .client
            .query()
            .table_name(&self.table_name)
            .key_condition_expression("entityType = :et")
            .filter_expression("active = :active AND entityId <> :fp")
            .expression_attribute_values(":et", AttributeValue::S(entity_type.to_string()))
            .expression_attribute_values(":active", AttributeValue::Bool(true))
            .expression_attribute_values(
                ":fp",
                AttributeValue::S(FINGERPRINT_ENTITY_ID.to_string()),
            )
            .send()
            .await
            .map_err(Self::dynamo_err_debug)?;

        let entities = result
            .items()
            .iter()
            .filter_map(|item| Self::get_s(item, "entityId"))
            .collect();

        Ok(entities)
    }

    async fn get_fingerprint(
        &self,
        entity_type: &str,
    ) -> Result<Option<String>, ServerStateError> {
        debug!("DynamoDB get_fingerprint: {}", entity_type);

        let result = self
            .client
            .get_item()
            .table_name(&self.table_name)
            .key("entityType", AttributeValue::S(entity_type.to_string()))
            .key(
                "entityId",
                AttributeValue::S(FINGERPRINT_ENTITY_ID.to_string()),
            )
            .send()
            .await
            .map_err(Self::dynamo_err_debug)?;

        Ok(result
            .item()
            .and_then(|item| Self::get_s(item, "fingerprint")))
    }

    async fn set_fingerprint(
        &self,
        entity_type: &str,
        fingerprint: String,
    ) -> Result<(), ServerStateError> {
        debug!("DynamoDB set_fingerprint: {} = {}", entity_type, fingerprint);

        let now = chrono::Utc::now().to_rfc3339();

        self.client
            .put_item()
            .table_name(&self.table_name)
            .item("entityType", AttributeValue::S(entity_type.to_string()))
            .item(
                "entityId",
                AttributeValue::S(FINGERPRINT_ENTITY_ID.to_string()),
            )
            .item("fingerprint", AttributeValue::S(fingerprint))
            .item("updatedAt", AttributeValue::S(now))
            .send()
            .await
            .map_err(Self::dynamo_err_debug)?;

        Ok(())
    }

    async fn get_registry_snapshot(
        &self,
        entity_type: &str,
    ) -> Result<Option<RegistrySnapshot>, ServerStateError> {
        debug!("DynamoDB get_registry_snapshot: {}", entity_type);

        // Get fingerprint first — if none, no snapshot exists
        let fingerprint = match self.get_fingerprint(entity_type).await? {
            Some(fp) => fp,
            None => return Ok(None),
        };

        let active_entities = self.get_active_entities(entity_type).await?;

        Ok(Some(RegistrySnapshot {
            entity_type: entity_type.to_string(),
            fingerprint,
            active_entities,
            updated_at: chrono::Utc::now().to_rfc3339(),
        }))
    }

    async fn maintenance(&self) -> Result<(), ServerStateError> {
        // No-op: DynamoDB manages capacity and cleanup via TTL natively.
        debug!("DynamoDB maintenance: no-op");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::entity_types;

    fn test_entity(id: &str, active: bool) -> EntityState {
        EntityState {
            entity_id: id.to_string(),
            active,
            metadata: None,
            updated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    // All tests require a real DynamoDB instance and are ignored by default.
    // Run with: cargo test -p turul-mcp-server-state-storage --features dynamodb -- --ignored

    #[tokio::test]
    #[ignore = "requires DynamoDB"]
    async fn test_entity_crud() {
        let storage = DynamoDbServerStateStorage::new().await.unwrap();

        // Set
        storage
            .set_entity_state(entity_types::TOOLS, "add", test_entity("add", true))
            .await
            .unwrap();

        // Get
        let state = storage
            .get_entity_state(entity_types::TOOLS, "add")
            .await
            .unwrap();
        assert!(state.is_some());
        assert!(state.unwrap().active);

        // Get missing
        let missing = storage
            .get_entity_state(entity_types::TOOLS, "nonexistent")
            .await
            .unwrap();
        assert!(missing.is_none());

        // Delete
        storage
            .delete_entity_state(entity_types::TOOLS, "add")
            .await
            .unwrap();
        let deleted = storage
            .get_entity_state(entity_types::TOOLS, "add")
            .await
            .unwrap();
        assert!(deleted.is_none());
    }

    #[tokio::test]
    #[ignore = "requires DynamoDB"]
    async fn test_active_entities() {
        let storage = DynamoDbServerStateStorage::new().await.unwrap();

        storage
            .set_entity_state(entity_types::TOOLS, "add", test_entity("add", true))
            .await
            .unwrap();
        storage
            .set_entity_state(
                entity_types::TOOLS,
                "multiply",
                test_entity("multiply", true),
            )
            .await
            .unwrap();
        storage
            .set_entity_state(
                entity_types::TOOLS,
                "disabled",
                test_entity("disabled", false),
            )
            .await
            .unwrap();

        let active = storage
            .get_active_entities(entity_types::TOOLS)
            .await
            .unwrap();
        assert_eq!(active.len(), 2);
        assert!(active.contains(&"add".to_string()));
        assert!(active.contains(&"multiply".to_string()));
        assert!(!active.contains(&"disabled".to_string()));

        // Cleanup
        for id in &["add", "multiply", "disabled"] {
            storage
                .delete_entity_state(entity_types::TOOLS, id)
                .await
                .unwrap();
        }
    }

    #[tokio::test]
    #[ignore = "requires DynamoDB"]
    async fn test_fingerprint_crud() {
        let storage = DynamoDbServerStateStorage::new().await.unwrap();

        // Set
        storage
            .set_fingerprint(entity_types::TOOLS, "abc123".to_string())
            .await
            .unwrap();

        let fp = storage
            .get_fingerprint(entity_types::TOOLS)
            .await
            .unwrap();
        assert_eq!(fp, Some("abc123".to_string()));

        // Update
        storage
            .set_fingerprint(entity_types::TOOLS, "def456".to_string())
            .await
            .unwrap();
        let fp = storage
            .get_fingerprint(entity_types::TOOLS)
            .await
            .unwrap();
        assert_eq!(fp, Some("def456".to_string()));

        // Cleanup
        storage
            .delete_entity_state(entity_types::TOOLS, FINGERPRINT_ENTITY_ID)
            .await
            .unwrap();
    }

    #[tokio::test]
    #[ignore = "requires DynamoDB"]
    async fn test_entity_type_isolation() {
        let storage = DynamoDbServerStateStorage::new().await.unwrap();

        storage
            .set_entity_state(entity_types::TOOLS, "add", test_entity("add", true))
            .await
            .unwrap();
        storage
            .set_entity_state(
                entity_types::RESOURCES,
                "file",
                test_entity("file", true),
            )
            .await
            .unwrap();
        storage
            .set_fingerprint(entity_types::TOOLS, "fp_tools".to_string())
            .await
            .unwrap();
        storage
            .set_fingerprint(entity_types::RESOURCES, "fp_resources".to_string())
            .await
            .unwrap();

        // Entity types are isolated
        let tools = storage
            .get_active_entities(entity_types::TOOLS)
            .await
            .unwrap();
        assert_eq!(tools.len(), 1);
        assert!(tools.contains(&"add".to_string()));

        let resources = storage
            .get_active_entities(entity_types::RESOURCES)
            .await
            .unwrap();
        assert_eq!(resources.len(), 1);
        assert!(resources.contains(&"file".to_string()));

        // Fingerprints are isolated
        assert_eq!(
            storage
                .get_fingerprint(entity_types::TOOLS)
                .await
                .unwrap(),
            Some("fp_tools".to_string())
        );
        assert_eq!(
            storage
                .get_fingerprint(entity_types::RESOURCES)
                .await
                .unwrap(),
            Some("fp_resources".to_string())
        );

        // Cleanup
        storage
            .delete_entity_state(entity_types::TOOLS, "add")
            .await
            .unwrap();
        storage
            .delete_entity_state(entity_types::RESOURCES, "file")
            .await
            .unwrap();
        storage
            .delete_entity_state(entity_types::TOOLS, FINGERPRINT_ENTITY_ID)
            .await
            .unwrap();
        storage
            .delete_entity_state(entity_types::RESOURCES, FINGERPRINT_ENTITY_ID)
            .await
            .unwrap();
    }

    #[tokio::test]
    #[ignore = "requires DynamoDB"]
    async fn test_registry_snapshot() {
        let storage = DynamoDbServerStateStorage::new().await.unwrap();

        // No snapshot before fingerprint set
        let snap = storage
            .get_registry_snapshot(entity_types::TOOLS)
            .await
            .unwrap();
        // Note: may or may not be None depending on prior test state;
        // clean first for a deterministic test
        storage
            .delete_entity_state(entity_types::TOOLS, FINGERPRINT_ENTITY_ID)
            .await
            .unwrap();
        let snap = storage
            .get_registry_snapshot(entity_types::TOOLS)
            .await
            .unwrap();
        assert!(snap.is_none());

        // Set entities and fingerprint
        storage
            .set_entity_state(entity_types::TOOLS, "add", test_entity("add", true))
            .await
            .unwrap();
        storage
            .set_entity_state(entity_types::TOOLS, "off", test_entity("off", false))
            .await
            .unwrap();
        storage
            .set_fingerprint(entity_types::TOOLS, "snap_fp".to_string())
            .await
            .unwrap();

        let snap = storage
            .get_registry_snapshot(entity_types::TOOLS)
            .await
            .unwrap();
        assert!(snap.is_some());
        let snap = snap.unwrap();
        assert_eq!(snap.entity_type, "tools");
        assert_eq!(snap.fingerprint, "snap_fp");
        assert_eq!(snap.active_entities.len(), 1);
        assert!(snap.active_entities.contains(&"add".to_string()));

        // Cleanup
        storage
            .delete_entity_state(entity_types::TOOLS, "add")
            .await
            .unwrap();
        storage
            .delete_entity_state(entity_types::TOOLS, "off")
            .await
            .unwrap();
        storage
            .delete_entity_state(entity_types::TOOLS, FINGERPRINT_ENTITY_ID)
            .await
            .unwrap();
    }

    #[tokio::test]
    #[ignore = "requires DynamoDB"]
    async fn test_backend_name() {
        let storage = DynamoDbServerStateStorage::new().await.unwrap();
        assert_eq!(storage.backend_name(), "DynamoDB");
    }

    #[tokio::test]
    #[ignore = "requires DynamoDB"]
    async fn test_entity_with_metadata() {
        let storage = DynamoDbServerStateStorage::new().await.unwrap();

        let state = EntityState {
            entity_id: "meta_tool".to_string(),
            active: true,
            metadata: Some(serde_json::json!({"version": "1.0", "description": "A tool"})),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };

        storage
            .set_entity_state(entity_types::TOOLS, "meta_tool", state)
            .await
            .unwrap();

        let retrieved = storage
            .get_entity_state(entity_types::TOOLS, "meta_tool")
            .await
            .unwrap()
            .unwrap();

        assert!(retrieved.active);
        assert!(retrieved.metadata.is_some());
        let meta = retrieved.metadata.unwrap();
        assert_eq!(meta["version"], "1.0");
        assert_eq!(meta["description"], "A tool");

        // Cleanup
        storage
            .delete_entity_state(entity_types::TOOLS, "meta_tool")
            .await
            .unwrap();
    }

    #[tokio::test]
    #[ignore = "requires DynamoDB"]
    async fn test_fingerprint_not_in_active_entities() {
        let storage = DynamoDbServerStateStorage::new().await.unwrap();

        // Set a fingerprint and an entity
        storage
            .set_fingerprint(entity_types::TOOLS, "fp_test".to_string())
            .await
            .unwrap();
        storage
            .set_entity_state(entity_types::TOOLS, "real_tool", test_entity("real_tool", true))
            .await
            .unwrap();

        let active = storage
            .get_active_entities(entity_types::TOOLS)
            .await
            .unwrap();

        // Fingerprint record must NOT appear in active entities
        assert!(!active.contains(&FINGERPRINT_ENTITY_ID.to_string()));
        assert!(active.contains(&"real_tool".to_string()));

        // Cleanup
        storage
            .delete_entity_state(entity_types::TOOLS, "real_tool")
            .await
            .unwrap();
        storage
            .delete_entity_state(entity_types::TOOLS, FINGERPRINT_ENTITY_ID)
            .await
            .unwrap();
    }

    #[tokio::test]
    #[ignore = "requires DynamoDB"]
    async fn test_create_table() {
        let config = DynamoDbServerStateConfig {
            table_name: format!("mcp-test-state-{}", uuid::Uuid::now_v7().as_simple()),
            create_table: true,
            ..Default::default()
        };

        let storage = DynamoDbServerStateStorage::with_config(config.clone())
            .await
            .unwrap();

        // Verify the table works
        storage
            .set_entity_state(entity_types::TOOLS, "test", test_entity("test", true))
            .await
            .unwrap();
        let state = storage
            .get_entity_state(entity_types::TOOLS, "test")
            .await
            .unwrap();
        assert!(state.is_some());

        // Clean up: delete the test table
        let aws_config = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(config.region))
            .load()
            .await;
        let client = Client::new(&aws_config);
        let _ = client
            .delete_table()
            .table_name(&config.table_name)
            .send()
            .await;
    }
}
