//! AWS DynamoDB Teardown for MCP Session Storage
//!
//! This utility cleans up AWS DynamoDB tables created for MCP session storage.
//! It can clear data, disable streams, create backups, or delete tables entirely.

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_dynamodb::types::TableStatus;
use aws_sdk_dynamodb::Client;
use std::env;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧹 AWS DynamoDB MCP Session Storage Teardown");
    println!("=============================================");
    
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
    
    // Initialize AWS client
    println!("🔐 Connecting to AWS...");
    let region_provider = RegionProviderChain::default_provider().or_else(&region);
    let config = aws_config::from_env().region(region_provider).load().await;
    let client = Client::new(&config);
    
    // Test credentials
    match client.list_tables().send().await {
        Ok(_) => println!("✅ AWS connection established"),
        Err(e) => {
            eprintln!("❌ AWS connection failed: {}", e);
            eprintln!("💡 Please check your AWS credentials and region");
            return Err("AWS connection failed".into());
        }
    }
    
    // Check which tables exist
    let sessions_exists = table_exists(&client, &sessions_table).await?;
    let events_exists = table_exists(&client, &events_table).await?;
    
    if !sessions_exists && !events_exists {
        println!("ℹ️  No MCP session tables found. Nothing to clean up!");
        return Ok(());
    }
    
    println!("📊 Found tables:");
    if sessions_exists {
        println!("   ✅ Sessions table: {}", sessions_table);
        print_table_info(&client, &sessions_table).await?;
    }
    if events_exists {
        println!("   ✅ Events table: {}", events_table);
        print_table_info(&client, &events_table).await?;
    }
    println!();
    
    // Check command line arguments for options
    let args: Vec<String> = env::args().collect();
    let clear_data = args.contains(&"--clear-data".to_string()) || args.contains(&"--all".to_string());
    let backup_tables = args.contains(&"--backup".to_string()) || args.contains(&"--all".to_string());
    let delete_tables = args.contains(&"--delete".to_string()) || args.contains(&"--all".to_string());
    let disable_streams = args.contains(&"--disable-streams".to_string()) || args.contains(&"--all".to_string());
    
    // If no specific options, ask user what to do
    if !clear_data && !backup_tables && !delete_tables && !disable_streams {
        println!("🤔 What would you like to clean up?");
        println!("   [1] Clear table data only (keep tables and structure)");
        println!("   [2] Create on-demand backups before cleanup");
        println!("   [3] Disable streams (reduce costs)");
        println!("   [4] Delete tables completely (⚠️  DESTRUCTIVE)");
        println!("   [5] Full cleanup (backup + delete tables)");
        println!("   [0] Cancel");
        println!();
        print!("Choose option (0-5): ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        match input.trim() {
            "1" => {
                if sessions_exists {
                    clear_table_data(&client, &sessions_table).await?;
                }
                if events_exists {
                    clear_table_data(&client, &events_table).await?;
                }
            }
            "2" => {
                if sessions_exists {
                    create_backup(&client, &sessions_table).await?;
                }
                if events_exists {
                    create_backup(&client, &events_table).await?;
                }
            }
            "3" => {
                if sessions_exists {
                    disable_streams(&client, &sessions_table).await?;
                }
                if events_exists {
                    disable_streams(&client, &events_table).await?;
                }
            }
            "4" => {
                println!();
                println!("⚠️  WARNING: This will permanently delete all tables and data!");
                print!("Are you sure? Type 'DELETE' to confirm: ");
                io::stdout().flush()?;
                
                let mut confirm = String::new();
                io::stdin().read_line(&mut confirm)?;
                
                if confirm.trim() == "DELETE" {
                    if events_exists {
                        delete_table(&client, &events_table).await?;
                    }
                    if sessions_exists {
                        delete_table(&client, &sessions_table).await?;
                    }
                } else {
                    println!("🚫 Deletion canceled");
                }
            }
            "5" => {
                // Backup first
                if sessions_exists {
                    create_backup(&client, &sessions_table).await?;
                }
                if events_exists {
                    create_backup(&client, &events_table).await?;
                }
                
                // Then delete
                println!();
                println!("⚠️  WARNING: This will permanently delete all tables after backup!");
                print!("Are you sure? Type 'DELETE' to confirm: ");
                io::stdout().flush()?;
                
                let mut confirm = String::new();
                io::stdin().read_line(&mut confirm)?;
                
                if confirm.trim() == "DELETE" {
                    if events_exists {
                        delete_table(&client, &events_table).await?;
                    }
                    if sessions_exists {
                        delete_table(&client, &sessions_table).await?;
                    }
                } else {
                    println!("🚫 Deletion canceled (backups were still created)");
                }
            }
            "0" => {
                println!("🚫 Canceled");
                return Ok(());
            }
            _ => {
                println!("❌ Invalid option");
                return Ok(());
            }
        }
    } else {
        // Execute based on command line flags
        if backup_tables {
            if sessions_exists {
                create_backup(&client, &sessions_table).await?;
            }
            if events_exists {
                create_backup(&client, &events_table).await?;
            }
        }
        if clear_data {
            if sessions_exists {
                clear_table_data(&client, &sessions_table).await?;
            }
            if events_exists {
                clear_table_data(&client, &events_table).await?;
            }
        }
        if disable_streams {
            if sessions_exists {
                disable_streams(&client, &sessions_table).await?;
            }
            if events_exists {
                disable_streams(&client, &events_table).await?;
            }
        }
        if delete_tables {
            if events_exists {
                delete_table(&client, &events_table).await?;
            }
            if sessions_exists {
                delete_table(&client, &sessions_table).await?;
            }
        }
    }
    
    println!();
    println!("🎉 Teardown complete!");
    println!();
    println!("💡 Command line options for next time:");
    println!("   --clear-data       Clear all data but keep tables");
    println!("   --backup           Create on-demand backups");
    println!("   --disable-streams  Disable DynamoDB streams");
    println!("   --delete           Delete tables completely");
    println!("   --all              Backup + delete everything");
    
    Ok(())
}

async fn table_exists(client: &Client, table_name: &str) -> Result<bool, Box<dyn std::error::Error>> {
    match client.describe_table().table_name(table_name).send().await {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

async fn print_table_info(client: &Client, table_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    match client.describe_table().table_name(table_name).send().await {
        Ok(response) => {
            if let Some(table) = response.table() {
                println!("     Status: {:?}", table.table_status().unwrap_or(&TableStatus::Unknown));
                if let Some(item_count) = table.item_count() {
                    println!("     Items: {}", item_count);
                }
                if let Some(size) = table.table_size_bytes() {
                    let size_mb = *size as f64 / (1024.0 * 1024.0);
                    println!("     Size: {:.2} MB", size_mb);
                }
            }
        }
        Err(e) => {
            println!("     Error getting info: {}", e);
        }
    }
    Ok(())
}

async fn clear_table_data(client: &Client, table_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("🧹 Clearing data from table '{}'...", table_name);
    
    // For simplicity, we'll use scan and delete. In production, you might want to use
    // parallel scan or consider recreating the table for better performance.
    
    let mut item_count = 0;
    let mut exclusive_start_key = None;
    
    loop {
        let mut scan_request = client.scan().table_name(table_name);
        
        if let Some(start_key) = exclusive_start_key {
            scan_request = scan_request.set_exclusive_start_key(Some(start_key));
        }
        
        match scan_request.send().await {
            Ok(response) => {
                if let Some(items) = response.items() {
                    for item in items {
                        // Delete each item
                        let mut delete_request = client.delete_item().table_name(table_name);
                        
                        // Extract key attributes based on table schema
                        if table_name.contains("events") {
                            // Events table has composite key: session_id, event_id
                            if let Some(session_id) = item.get("session_id") {
                                delete_request = delete_request.key("session_id", session_id.clone());
                            }
                            if let Some(event_id) = item.get("event_id") {
                                delete_request = delete_request.key("event_id", event_id.clone());
                            }
                        } else {
                            // Sessions table has simple key: session_id
                            if let Some(session_id) = item.get("session_id") {
                                delete_request = delete_request.key("session_id", session_id.clone());
                            }
                        }
                        
                        match delete_request.send().await {
                            Ok(_) => item_count += 1,
                            Err(e) => eprintln!("   ⚠️  Failed to delete item: {}", e),
                        }
                    }
                }
                
                exclusive_start_key = response.last_evaluated_key().cloned();
                if exclusive_start_key.is_none() {
                    break; // No more items
                }
                
                print!("   Deleted {} items...\r", item_count);
                io::stdout().flush()?;
            }
            Err(e) => {
                eprintln!("❌ Failed to scan table '{}': {}", table_name, e);
                return Err(e.into());
            }
        }
    }
    
    println!();
    println!("✅ Cleared {} items from table '{}'", item_count, table_name);
    Ok(())
}

async fn create_backup(client: &Client, table_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let backup_name = format!("{}-backup-{}", table_name, timestamp);
    
    println!("💾 Creating backup '{}' for table '{}'...", backup_name, table_name);
    
    match client
        .create_backup()
        .table_name(table_name)
        .backup_name(&backup_name)
        .send()
        .await
    {
        Ok(response) => {
            if let Some(backup_details) = response.backup_details() {
                println!("✅ Backup created successfully");
                println!("   Backup ARN: {}", backup_details.backup_arn());
                if let Some(status) = backup_details.backup_status() {
                    println!("   Status: {:?}", status);
                }
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to create backup for '{}': {}", table_name, e);
            return Err(e.into());
        }
    }
    
    Ok(())
}

async fn disable_streams(client: &Client, table_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("🚫 Disabling streams for table '{}'...", table_name);
    
    match client
        .update_table()
        .table_name(table_name)
        .stream_specification(
            aws_sdk_dynamodb::types::StreamSpecification::builder()
                .stream_enabled(false)
                .build(),
        )
        .send()
        .await
    {
        Ok(_) => {
            println!("✅ Streams disabled for table '{}'", table_name);
        }
        Err(e) => {
            eprintln!("❌ Failed to disable streams for '{}': {}", table_name, e);
            return Err(e.into());
        }
    }
    
    Ok(())
}

async fn delete_table(client: &Client, table_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("💥 Deleting table '{}'...", table_name);
    
    match client.delete_table().table_name(table_name).send().await {
        Ok(_) => {
            println!("✅ Table '{}' deletion initiated", table_name);
            println!("   (Table deletion may take a few minutes to complete)");
        }
        Err(e) => {
            eprintln!("❌ Failed to delete table '{}': {}", table_name, e);
            return Err(e.into());
        }
    }
    
    Ok(())
}