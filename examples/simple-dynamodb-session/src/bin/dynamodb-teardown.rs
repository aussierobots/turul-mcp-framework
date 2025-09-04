//! DynamoDB Teardown Utility
//! 
//! Deletes both DynamoDB tables used by the session storage system:
//! 1. Main session table (using MCP_SESSION_TABLE environment variable)
//! 2. Events table ({MCP_SESSION_TABLE}-events)
//!
//! WARNING: This will permanently delete all session data!

use turul_mcp_session_storage::{DynamoDbSessionStorage, DynamoDbConfig};
use tracing::{info, warn, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🧹 DynamoDB Teardown Utility");
    warn!("⚠️  WARNING: This will permanently delete all session data!");
    info!("Deleting both session and events tables for MCP session storage");

    // Get configuration from environment variables (same as main server)
    let table_name = std::env::var("MCP_SESSION_TABLE")
        .unwrap_or_else(|_| "mcp-sessions".to_string());
    let region = std::env::var("AWS_REGION")
        .unwrap_or_else(|_| "us-east-1".to_string());

    info!("Configuration:");
    info!("  Session Table: {}", table_name);
    info!("  Events Table: {}-events", table_name);
    info!("  AWS Region: {}", region);
    info!("");

    // Create DynamoDB configuration
    let config = DynamoDbConfig {
        table_name: table_name.clone(),
        region: region.clone(),
        session_ttl_minutes: 24 * 60,  // Not used for teardown
        event_ttl_minutes: 24 * 60,    // Not used for teardown
        max_events_per_session: 1000,
        enable_backup: true,
        enable_encryption: true,
        create_tables_if_missing: false, // Not creating, just connecting
    };

    // Initialize DynamoDB session storage
    info!("📡 Connecting to AWS DynamoDB...");
    let storage = match DynamoDbSessionStorage::with_config(config).await {
        Ok(storage) => {
            info!("✅ Connected to DynamoDB successfully");
            storage
        },
        Err(e) => {
            error!("❌ Failed to connect to DynamoDB: {}", e);
            error!("");
            error!("Make sure AWS credentials are configured:");
            error!("  export AWS_ACCESS_KEY_ID=your_access_key");
            error!("  export AWS_SECRET_ACCESS_KEY=your_secret_key");  
            error!("  export AWS_REGION=us-east-1");
            error!("");
            error!("Or use AWS profiles:");
            error!("  aws configure");
            return Err(e.into());
        }
    };

    // Confirm deletion
    warn!("⚠️  About to delete the following tables:");
    warn!("  • {} (main session table)", table_name);
    warn!("  • {}-events (events table)", table_name);
    warn!("");
    warn!("💀 ALL SESSION DATA WILL BE PERMANENTLY LOST!");
    warn!("");

    // For safety, require explicit confirmation via environment variable
    if std::env::var("CONFIRM_DELETE").unwrap_or_default() != "yes" {
        error!("❌ Deletion cancelled for safety.");
        error!("");
        error!("To confirm deletion, run:");
        error!("  CONFIRM_DELETE=yes MCP_SESSION_TABLE={} cargo run --bin dynamodb-teardown", table_name);
        error!("");
        return Ok(());
    }

    // Delete both tables
    info!("🗑️  Deleting DynamoDB tables...");
    match storage.delete_tables().await {
        Ok(_) => {
            info!("✅ Successfully initiated deletion of both DynamoDB tables!");
            info!("🗑️  Tables being deleted:");
            info!("  • {} (main session table)", table_name);
            info!("  • {}-events (events table)", table_name);
            info!("");
            info!("ℹ️  Note: It may take a few minutes for AWS to complete the deletion.");
            info!("🎉 Teardown complete!");
        },
        Err(e) => {
            error!("❌ Failed to delete tables: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}