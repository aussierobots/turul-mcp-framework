//! AWS DynamoDB Setup for MCP Session Storage
//!
//! This utility sets up AWS DynamoDB tables required for MCP session storage.
//! It creates tables with proper indexes and configures settings needed for
//! the simple-dynamodb-session example.

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_dynamodb::types::{
    AttributeDefinition, BillingMode, GlobalSecondaryIndex, KeySchemaElement, KeyType,
    Projection, ProjectionType, ScalarAttributeType, StreamSpecification, StreamViewType,
    TableStatus, Tag, TimeToLiveSpecification,
};
use aws_sdk_dynamodb::Client;
use std::env;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("☁️  AWS DynamoDB MCP Session Storage Setup");
    println!("==========================================");
    
    // Initialize tracing for AWS SDK
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN) // Reduce AWS SDK noise
        .init();
    
    // Get configuration from environment or use defaults
    let region = env::var("AWS_REGION")
        .or_else(|_| env::var("AWS_DEFAULT_REGION"))
        .unwrap_or_else(|_| "us-east-1".to_string());
    let sessions_table = env::var("DYNAMODB_SESSIONS_TABLE")
        .unwrap_or_else(|_| "mcp-sessions".to_string());
    let events_table = env::var("DYNAMODB_EVENTS_TABLE")
        .unwrap_or_else(|_| "mcp-session-events".to_string());
    let environment = env::var("ENVIRONMENT")
        .unwrap_or_else(|_| "development".to_string());
    
    println!("📋 Configuration:");
    println!("   Region: {}", region);
    println!("   Sessions Table: {}", sessions_table);
    println!("   Events Table: {}", events_table);
    println!("   Environment: {}", environment);
    println!();
    
    // Check AWS credentials
    println!("🔐 Checking AWS credentials...");
    let region_provider = RegionProviderChain::default_provider().or_else(&region);
    let config = aws_config::from_env().region(region_provider).load().await;
    let client = Client::new(&config);
    
    // Test credentials by listing tables
    match client.list_tables().send().await {
        Ok(_) => println!("✅ AWS credentials configured correctly"),
        Err(e) => {
            eprintln!("❌ AWS credentials error: {}", e);
            eprintln!("💡 Please configure AWS credentials:");
            eprintln!("   export AWS_ACCESS_KEY_ID=your_access_key");
            eprintln!("   export AWS_SECRET_ACCESS_KEY=your_secret_key");
            eprintln!("   export AWS_REGION={}", region);
            eprintln!("   # Or use AWS CLI: aws configure");
            return Err("AWS credentials not configured".into());
        }
    }
    
    println!();
    println!("🔧 Setting up DynamoDB tables...");
    
    // Setup sessions table
    setup_sessions_table(&client, &sessions_table, &environment).await?;
    
    // Setup events table
    setup_events_table(&client, &events_table, &environment).await?;
    
    // Wait for tables to be active
    println!("⏳ Waiting for tables to become active...");
    wait_for_table_active(&client, &sessions_table).await?;
    wait_for_table_active(&client, &events_table).await?;
    
    // Configure TTL for cleanup
    configure_ttl(&client, &sessions_table, "ttl").await?;
    configure_ttl(&client, &events_table, "ttl").await?;
    
    println!();
    println!("🎉 DynamoDB setup complete!");
    print_usage_info(&sessions_table, &events_table, &region, &environment);
    
    Ok(())
}

async fn setup_sessions_table(
    client: &Client,
    table_name: &str,
    environment: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Setting up sessions table: {}", table_name);
    
    // Check if table already exists
    match client.describe_table().table_name(table_name).send().await {
        Ok(response) => {
            if let Some(table) = response.table() {
                println!("ℹ️  Table '{}' already exists with status: {:?}", table_name, table.table_status());
                return Ok(());
            }
        }
        Err(_) => {
            // Table doesn't exist, proceed with creation
        }
    }
    
    println!("🔨 Creating sessions table...");
    
    let create_table_request = client
        .create_table()
        .table_name(table_name)
        .billing_mode(BillingMode::PayPerRequest) // Serverless billing
        // Primary key
        .key_schema(
            KeySchemaElement::builder()
                .attribute_name("session_id")
                .key_type(KeyType::Hash)
                .build()?,
        )
        // Attribute definitions
        .attribute_definitions(
            AttributeDefinition::builder()
                .attribute_name("session_id")
                .attribute_type(ScalarAttributeType::S)
                .build()?,
        )
        .attribute_definitions(
            AttributeDefinition::builder()
                .attribute_name("last_activity")
                .attribute_type(ScalarAttributeType::N)
                .build()?,
        )
        .attribute_definitions(
            AttributeDefinition::builder()
                .attribute_name("created_at")
                .attribute_type(ScalarAttributeType::N)
                .build()?,
        )
        // GSI for querying by last activity (for cleanup)
        .global_secondary_indexes(
            GlobalSecondaryIndex::builder()
                .index_name("LastActivityIndex")
                .key_schema(
                    KeySchemaElement::builder()
                        .attribute_name("last_activity")
                        .key_type(KeyType::Hash)
                        .build()?,
                )
                .projection(
                    Projection::builder()
                        .projection_type(ProjectionType::KeysOnly)
                        .build(),
                )
                .build()?,
        )
        // GSI for querying by creation time
        .global_secondary_indexes(
            GlobalSecondaryIndex::builder()
                .index_name("CreatedAtIndex")
                .key_schema(
                    KeySchemaElement::builder()
                        .attribute_name("created_at")
                        .key_type(KeyType::Hash)
                        .build()?,
                )
                .projection(
                    Projection::builder()
                        .projection_type(ProjectionType::KeysOnly)
                        .build(),
                )
                .build()?,
        )
        // Enable streams for real-time updates
        .stream_specification(
            StreamSpecification::builder()
                .stream_enabled(true)
                .stream_view_type(StreamViewType::NewAndOldImages)
                .build(),
        )
        // Tags for management
        .tags(
            Tag::builder()
                .key("Environment")
                .value(environment)
                .build()?,
        )
        .tags(
            Tag::builder()
                .key("Application")
                .value("MCP-Session-Storage")
                .build()?,
        )
        .tags(
            Tag::builder()
                .key("Purpose")
                .value("SessionManagement")
                .build()?,
        );
    
    match create_table_request.send().await {
        Ok(_) => {
            println!("✅ Sessions table '{}' created successfully", table_name);
        }
        Err(e) => {
            eprintln!("❌ Failed to create sessions table: {}", e);
            return Err(e.into());
        }
    }
    
    Ok(())
}

async fn setup_events_table(
    client: &Client,
    table_name: &str,
    environment: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Setting up events table: {}", table_name);
    
    // Check if table already exists
    match client.describe_table().table_name(table_name).send().await {
        Ok(response) => {
            if let Some(table) = response.table() {
                println!("ℹ️  Table '{}' already exists with status: {:?}", table_name, table.table_status());
                return Ok(());
            }
        }
        Err(_) => {
            // Table doesn't exist, proceed with creation
        }
    }
    
    println!("🔨 Creating events table...");
    
    let create_table_request = client
        .create_table()
        .table_name(table_name)
        .billing_mode(BillingMode::PayPerRequest) // Serverless billing
        // Composite primary key
        .key_schema(
            KeySchemaElement::builder()
                .attribute_name("session_id")
                .key_type(KeyType::Hash)
                .build()?,
        )
        .key_schema(
            KeySchemaElement::builder()
                .attribute_name("event_id")
                .key_type(KeyType::Range)
                .build()?,
        )
        // Attribute definitions
        .attribute_definitions(
            AttributeDefinition::builder()
                .attribute_name("session_id")
                .attribute_type(ScalarAttributeType::S)
                .build()?,
        )
        .attribute_definitions(
            AttributeDefinition::builder()
                .attribute_name("event_id")
                .attribute_type(ScalarAttributeType::N)
                .build()?,
        )
        .attribute_definitions(
            AttributeDefinition::builder()
                .attribute_name("timestamp")
                .attribute_type(ScalarAttributeType::N)
                .build()?,
        )
        // GSI for querying by timestamp (for cleanup and ordering)
        .global_secondary_indexes(
            GlobalSecondaryIndex::builder()
                .index_name("TimestampIndex")
                .key_schema(
                    KeySchemaElement::builder()
                        .attribute_name("timestamp")
                        .key_type(KeyType::Hash)
                        .build()?,
                )
                .key_schema(
                    KeySchemaElement::builder()
                        .attribute_name("session_id")
                        .key_type(KeyType::Range)
                        .build()?,
                )
                .projection(
                    Projection::builder()
                        .projection_type(ProjectionType::All)
                        .build(),
                )
                .build()?,
        )
        // Enable streams for real-time event processing
        .stream_specification(
            StreamSpecification::builder()
                .stream_enabled(true)
                .stream_view_type(StreamViewType::NewAndOldImages)
                .build(),
        )
        // Tags
        .tags(
            Tag::builder()
                .key("Environment")
                .value(environment)
                .build()?,
        )
        .tags(
            Tag::builder()
                .key("Application")
                .value("MCP-Session-Storage")
                .build()?,
        )
        .tags(
            Tag::builder()
                .key("Purpose")
                .value("EventStreaming")
                .build()?,
        );
    
    match create_table_request.send().await {
        Ok(_) => {
            println!("✅ Events table '{}' created successfully", table_name);
        }
        Err(e) => {
            eprintln!("❌ Failed to create events table: {}", e);
            return Err(e.into());
        }
    }
    
    Ok(())
}

async fn wait_for_table_active(
    client: &Client,
    table_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("⏳ Waiting for table '{}' to become active...", table_name);
    
    let mut attempts = 0;
    let max_attempts = 30; // 5 minutes with 10-second intervals
    
    loop {
        match client.describe_table().table_name(table_name).send().await {
            Ok(response) => {
                if let Some(table) = response.table() {
                    match table.table_status() {
                        Some(TableStatus::Active) => {
                            println!("✅ Table '{}' is now active", table_name);
                            return Ok(());
                        }
                        Some(status) => {
                            println!("   Status: {:?} (attempt {}/{})", status, attempts + 1, max_attempts);
                        }
                        None => {
                            println!("   Status: Unknown (attempt {}/{})", attempts + 1, max_attempts);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("❌ Error checking table status: {}", e);
                return Err(e.into());
            }
        }
        
        attempts += 1;
        if attempts >= max_attempts {
            return Err(format!("Table '{}' did not become active within 5 minutes", table_name).into());
        }
        
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    }
}

async fn configure_ttl(
    client: &Client,
    table_name: &str,
    ttl_attribute: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("⏰ Configuring TTL for table '{}' with attribute '{}'", table_name, ttl_attribute);
    
    let ttl_spec = TimeToLiveSpecification::builder()
        .attribute_name(ttl_attribute)
        .enabled(true)
        .build()?;
    
    match client
        .update_time_to_live()
        .table_name(table_name)
        .time_to_live_specification(ttl_spec)
        .send()
        .await
    {
        Ok(_) => {
            println!("✅ TTL configured for table '{}'", table_name);
        }
        Err(e) => {
            // TTL configuration might fail if already set, which is okay
            println!("⚠️  TTL configuration for '{}': {}", table_name, e);
        }
    }
    
    Ok(())
}

fn print_usage_info(sessions_table: &str, events_table: &str, region: &str, environment: &str) {
    println!();
    println!("📋 DynamoDB Tables Created:");
    println!("   🗃️  Sessions: {}", sessions_table);
    println!("   📊 Events: {}", events_table);
    println!("   🌍 Region: {}", region);
    println!("   🏷️  Environment: {}", environment);
    println!();
    println!("🚀 You can now run the MCP server:");
    println!("   cargo run --bin server");
    println!();
    println!("🧪 Test the tables:");
    println!("   aws dynamodb describe-table --table-name {}", sessions_table);
    println!("   aws dynamodb scan --table-name {} --max-items 5", sessions_table);
    println!();
    println!("💰 Cost Information:");
    println!("   • Pay-per-request billing (no upfront costs)");
    println!("   • GSI costs: Additional read/write charges");
    println!("   • Streams: Change data capture included");
    println!("   • TTL: Automatic cleanup at no extra cost");
    println!();
    println!("🔧 Environment variables for the server:");
    println!("   export DYNAMODB_SESSIONS_TABLE={}", sessions_table);
    println!("   export DYNAMODB_EVENTS_TABLE={}", events_table);
    println!("   export AWS_REGION={}", region);
    println!();
    println!("🧹 Clean up when done:");
    println!("   cargo run --bin dynamodb-teardown");
}