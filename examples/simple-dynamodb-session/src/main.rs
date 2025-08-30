//! # Simple AWS DynamoDB Session Storage Example
//!
//! This example demonstrates DynamoDB-backed session storage for MCP servers
//! in AWS serverless and cloud-native environments. Perfect for Lambda functions,
//! auto-scaling applications, and multi-region deployments.
//!
//! ## Features Demonstrated
//!
//! - DynamoDB-backed session persistence
//! - Serverless-friendly storage (no connection pools)
//! - AWS-native integration with IAM, CloudWatch, etc.
//! - Automatic scaling and TTL-based cleanup
//! - Multi-region capabilities with Global Tables

use std::sync::Arc;
use mcp_server::{McpServer, McpResult, SessionContext};
use mcp_session_storage::{DynamoDbSessionStorage, DynamoDbConfig};
use mcp_derive::McpTool;
use serde_json::{json, Value};
use tracing::{info, error, debug, warn};

/// Tool that stores application state in DynamoDB
#[derive(McpTool, Default)]
#[tool(name = "store_app_state", description = "Store application state in AWS DynamoDB with automatic TTL")]
struct StoreAppStateTool {
    #[param(description = "State key (e.g., 'user_profile', 'workflow_step')")]
    key: String,
    #[param(description = "State data to store")]
    data: Value,
}

impl StoreAppStateTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<Value> {
        let session = session.ok_or_else(|| mcp_protocol::McpError::SessionError("Session required".to_string()))?;

        debug!("Storing app state in DynamoDB: {} = {}", self.key, self.data);

        // Store state in DynamoDB-backed session storage
        (session.set_state)(&format!("app_{}", self.key), self.data.clone());

        // Send CloudWatch-style notification
        (session.send_notification)(mcp_server::SessionEvent::Notification(json!({
            "jsonrpc": "2.0",
            "method": "notifications/progress",
            "params": {
                "progressToken": format!("state_{}", self.key),
                "progress": 1,
                "total": 1,
                "message": format!("State '{}' stored in DynamoDB", self.key)
            }
        })));

        Ok(json!({
            "stored": true,
            "key": self.key,
            "data": self.data,
            "storage": "AWS DynamoDB",
            "features": ["Automatic TTL", "Auto-scaling", "Multi-region support"],
            "message": format!("Application state '{}' stored in DynamoDB with TTL", self.key),
            "timestamp": chrono::Utc::now()
        }))
    }
}

/// Tool that retrieves application state from DynamoDB
#[derive(McpTool, Default)]
#[tool(name = "get_app_state", description = "Retrieve application state from AWS DynamoDB")]
struct GetAppStateTool {
    #[param(description = "State key to retrieve")]
    key: String,
}

impl GetAppStateTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<Value> {
        let session = session.ok_or_else(|| mcp_protocol::McpError::SessionError("Session required".to_string()))?;

        debug!("Getting app state from DynamoDB: {}", self.key);

        // Retrieve state from DynamoDB-backed session storage
        let data = (session.get_state)(&format!("app_{}", self.key));

        Ok(json!({
            "found": data.is_some(),
            "key": self.key,
            "data": data,
            "storage": "AWS DynamoDB",
            "region": "us-east-1", // Would be from config in real implementation
            "message": if data.is_some() {
                format!("State '{}' retrieved from DynamoDB", self.key)
            } else {
                format!("State '{}' not found or expired (TTL)", self.key)
            },
            "timestamp": chrono::Utc::now()
        }))
    }
}

/// Tool that demonstrates Lambda-friendly stateless operations
#[derive(McpTool, Default)]
#[tool(name = "lambda_operation", description = "Simulate serverless function with DynamoDB state")]
struct LambdaOperationTool {
    #[param(description = "Operation to perform")]
    operation: String,
    #[param(description = "Input data")]
    input: Value,
}

impl LambdaOperationTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<Value> {
        let session = session.ok_or_else(|| mcp_protocol::McpError::SessionError("Session required".to_string()))?;

        debug!("Performing Lambda-style operation: {}", self.operation);

        // Get operation counter from DynamoDB
        let counter_key = format!("lambda_ops_{}", self.operation);
        let current_count = (session.get_state)(&counter_key)
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let new_count = current_count + 1;
        (session.set_state)(&counter_key, json!(new_count));

        // Store operation result
        let result_key = format!("lambda_result_{}", self.operation);
        let result = json!({
            "operation": self.operation,
            "input": self.input,
            "execution_count": new_count,
            "processed_at": chrono::Utc::now(),
            "lambda_context": {
                "request_id": format!("req-{}", uuid::Uuid::new_v4()),
                "memory_limit": "512MB",
                "timeout": "30s"
            }
        });
        (session.set_state)(&result_key, result.clone());

        Ok(json!({
            "operation": self.operation,
            "execution_count": new_count,
            "result": result,
            "storage": "DynamoDB (Serverless)",
            "benefits": [
                "No connection pools (Lambda-friendly)",
                "Pay-per-request pricing", 
                "Automatic scaling",
                "Built-in TTL cleanup",
                "Multi-AZ availability"
            ],
            "message": format!("Lambda operation '{}' completed, state persisted in DynamoDB", self.operation),
            "timestamp": chrono::Utc::now()
        }))
    }
}

/// Tool that shows DynamoDB storage capabilities
#[derive(McpTool, Default)]
#[tool(name = "dynamodb_info", description = "Show AWS DynamoDB storage information and capabilities")]
struct DynamoDbInfoTool {}

impl DynamoDbInfoTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<Value> {
        let session = session.ok_or_else(|| mcp_protocol::McpError::SessionError("Session required".to_string()))?;

        Ok(json!({
            "session_id": session.session_id,
            "storage_backend": "AWS DynamoDB",
            "table_name": "mcp-sessions",
            "region": "us-east-1",
            "features": {
                "persistence": "Fully managed NoSQL database",
                "scaling": "Automatic scaling based on demand", 
                "ttl": "Automatic item expiration with TTL",
                "backup": "Point-in-time recovery available",
                "encryption": "Encryption at rest and in transit",
                "global_tables": "Multi-region replication support",
                "streams": "DynamoDB Streams for change capture",
                "cloudwatch": "Built-in CloudWatch metrics"
            },
            "use_cases": [
                "AWS Lambda functions", 
                "Serverless applications",
                "Auto-scaling web services",
                "Multi-region deployments",
                "IoT and mobile backends"
            ],
            "pricing_model": "Pay-per-request (on-demand) or provisioned capacity",
            "limits": {
                "item_size": "400KB maximum",
                "batch_operations": "25 items per batch",
                "query_result": "1MB per query"
            },
            "aws_integration": {
                "iam": "Fine-grained access control",
                "cloudtrail": "API call logging",
                "vpc_endpoints": "Private network access"
            },
            "message": "Session data stored in AWS DynamoDB with enterprise features",
            "timestamp": chrono::Utc::now()
        }))
    }
}

/// Tool that demonstrates DynamoDB Global Tables for multi-region
#[derive(McpTool, Default)]
#[tool(name = "global_state", description = "Demonstrate multi-region state with DynamoDB Global Tables")]
struct GlobalStateTool {
    #[param(description = "Region identifier")]
    region: String,
    #[param(description = "State data")]
    data: Value,
}

impl GlobalStateTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<Value> {
        let session = session.ok_or_else(|| mcp_protocol::McpError::SessionError("Session required".to_string()))?;

        debug!("Setting global state for region: {}", self.region);

        // Store region-specific state
        let global_key = format!("global_{}_{}", self.region, chrono::Utc::now().timestamp());
        (session.set_state)(&global_key, json!({
            "region": self.region,
            "data": self.data,
            "timestamp": chrono::Utc::now(),
            "replicated": true
        }));

        Ok(json!({
            "region": self.region,
            "data": self.data,
            "global_tables": true,
            "replication": {
                "enabled": true,
                "regions": ["us-east-1", "us-west-2", "eu-west-1"],
                "consistency": "Eventually consistent across regions",
                "conflict_resolution": "Last writer wins"
            },
            "benefits": [
                "Low-latency reads from nearest region",
                "Automatic multi-region replication", 
                "Built-in conflict resolution",
                "99.999% availability SLA"
            ],
            "message": format!("Global state set for region '{}' - replicated to other regions", self.region),
            "timestamp": chrono::Utc::now()
        }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting Simple AWS DynamoDB Session Storage Example");

    // DynamoDB configuration
    let dynamodb_config = DynamoDbConfig {
        table_name: std::env::var("DYNAMODB_TABLE")
            .unwrap_or_else(|_| "mcp-sessions".to_string()),
        region: std::env::var("AWS_REGION")
            .unwrap_or_else(|_| "us-east-1".to_string()),
        session_ttl_hours: 24,
        event_ttl_hours: 24,
        max_events_per_session: 2000,
        enable_backup: true,
        enable_encryption: true,
    };

    info!("AWS DynamoDB Configuration:");
    info!("  Table: {}", dynamodb_config.table_name);
    info!("  Region: {}", dynamodb_config.region);
    info!("  TTL: {} hours", dynamodb_config.session_ttl_hours);

    // Create DynamoDB session storage
    let dynamodb_storage = match DynamoDbSessionStorage::with_config(dynamodb_config.clone()).await {
        Ok(storage) => {
            info!("✅ AWS DynamoDB session storage initialized successfully");
            info!("🌍 Table: {} in region {}", dynamodb_config.table_name, dynamodb_config.region);
            Arc::new(storage)
        }
        Err(e) => {
            error!("❌ Failed to initialize DynamoDB session storage: {}", e);
            warn!("This example requires AWS credentials and DynamoDB access");
            warn!("For local development, consider using DynamoDB Local:");
            warn!("  docker run -p 8000:8000 amazon/dynamodb-local");
            warn!("  AWS_ENDPOINT_URL=http://localhost:8000 cargo run --bin server");
            return Err(e.into());
        }
    };

    // Build MCP server with DynamoDB session storage
    let server = McpServer::builder()
        .name("simple-dynamodb-session")
        .version("1.0.0")
        .title("AWS DynamoDB Session Storage Example")
        .instructions("Demonstrates AWS DynamoDB-backed session storage for serverless and cloud-native MCP applications.")
        .with_session_storage(dynamodb_storage)
        .tool(StoreAppStateTool::default())
        .tool(GetAppStateTool::default())
        .tool(LambdaOperationTool::default())
        .tool(DynamoDbInfoTool::default())
        .tool(GlobalStateTool::default())
        .bind_address("127.0.0.1:8062".parse()?)
        .sse(true)
        .build()?;

    info!("🎉 AWS DynamoDB session storage example server ready!");
    info!("🚀 Server running at: http://127.0.0.1:8062/mcp");
    info!("📊 Session Storage: AWS DynamoDB (Serverless & Scalable)");
    info!("🔄 SSE Notifications: Enabled with DynamoDB event storage");
    info!("🌍 Region: {}", dynamodb_config.region);
    info!("📅 TTL: {} hours (automatic cleanup)", dynamodb_config.session_ttl_hours);
    info!("");
    info!("Available tools:");
    info!("  • store_app_state   - Store application state in DynamoDB");
    info!("  • get_app_state     - Retrieve state from DynamoDB");
    info!("  • lambda_operation  - Simulate serverless function with state");
    info!("  • dynamodb_info     - View DynamoDB capabilities & features");
    info!("  • global_state      - Demonstrate multi-region Global Tables");
    info!("");
    info!("Example usage:");
    info!("  1. store_app_state(key='user_profile', data={{'name': 'Alice'}})");
    info!("  2. lambda_operation(operation='process_data', input={{'batch': 123}})");
    info!("  3. global_state(region='us-west-2', data={{'cached': true}})");
    info!("  4. dynamodb_info()  // View AWS DynamoDB features");
    info!("");
    info!("🏗️  AWS Features:");
    info!("  • Automatic scaling based on demand");
    info!("  • TTL-based automatic cleanup"); 
    info!("  • Multi-region replication (Global Tables)");
    info!("  • Built-in encryption and backup");
    info!("  • CloudWatch monitoring integration");

    server.run().await?;
    Ok(())
}