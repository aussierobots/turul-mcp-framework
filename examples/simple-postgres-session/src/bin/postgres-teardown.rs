//! PostgreSQL Database Teardown for MCP Session Storage
//!
//! This utility cleans up the PostgreSQL database and containers created for
//! MCP session storage testing. It removes tables, data, and optionally
//! stops/removes the Docker container.

use std::process::Command;
use std::env;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧹 PostgreSQL MCP Session Storage Teardown");
    println!("==========================================");
    
    // Get database configuration from environment or use defaults
    let db_host = env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string());
    let db_port = env::var("POSTGRES_PORT").unwrap_or_else(|_| "5432".to_string());
    let db_name = env::var("POSTGRES_DB").unwrap_or_else(|_| "mcp_sessions".to_string());
    let db_user = env::var("POSTGRES_USER").unwrap_or_else(|_| "mcp".to_string());
    let db_password = env::var("POSTGRES_PASSWORD").unwrap_or_else(|_| "mcp_pass".to_string());
    
    println!("📋 Configuration:");
    println!("   Host: {}", db_host);
    println!("   Port: {}", db_port);
    println!("   Database: {}", db_name);
    println!("   User: {}", db_user);
    println!("   Password: [masked]");
    println!();
    
    // Check command line arguments for options
    let args: Vec<String> = env::args().collect();
    let drop_tables = args.contains(&"--drop-tables".to_string()) || args.contains(&"--all".to_string());
    let drop_database = args.contains(&"--drop-database".to_string()) || args.contains(&"--all".to_string());
    let stop_container = args.contains(&"--stop-container".to_string()) || args.contains(&"--all".to_string());
    let remove_container = args.contains(&"--remove-container".to_string()) || args.contains(&"--all".to_string());
    
    // If no specific options, ask user what to do
    if !drop_tables && !drop_database && !stop_container && !remove_container {
        println!("🤔 What would you like to clean up?");
        println!("   [1] Clear session data only (recommended for development)");
        println!("   [2] Drop tables (removes all session schema)");
        println!("   [3] Drop database (removes entire database)");
        println!("   [4] Stop Docker container (keeps container for later)");
        println!("   [5] Remove Docker container (deletes container completely)");
        println!("   [6] Full cleanup (tables + container removal)");
        println!("   [0] Cancel");
        println!();
        print!("Choose option (0-6): ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        match input.trim() {
            "1" => {
                clear_session_data(&db_host, &db_port, &db_name, &db_user, &db_password).await?;
            }
            "2" => {
                drop_tables(&db_host, &db_port, &db_name, &db_user, &db_password).await?;
            }
            "3" => {
                drop_database(&db_host, &db_port, &db_name, &db_user, &db_password).await?;
            }
            "4" => {
                stop_docker_container().await?;
            }
            "5" => {
                remove_docker_container().await?;
            }
            "6" => {
                drop_tables(&db_host, &db_port, &db_name, &db_user, &db_password).await?;
                remove_docker_container().await?;
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
        if drop_tables {
            drop_tables(&db_host, &db_port, &db_name, &db_user, &db_password).await?;
        }
        if drop_database {
            drop_database(&db_host, &db_port, &db_name, &db_user, &db_password).await?;
        }
        if stop_container {
            stop_docker_container().await?;
        }
        if remove_container {
            remove_docker_container().await?;
        }
    }
    
    println!();
    println!("🎉 Teardown complete!");
    println!();
    println!("💡 Command line options for next time:");
    println!("   --drop-tables      Drop MCP session tables");
    println!("   --drop-database    Drop entire database");
    println!("   --stop-container   Stop Docker container");
    println!("   --remove-container Remove Docker container");
    println!("   --all              Do everything");
    
    Ok(())
}

async fn clear_session_data(
    host: &str,
    port: &str,
    db_name: &str,
    user: &str,
    password: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🧹 Clearing session data...");
    
    let database_url = format!(
        "postgres://{}:{}@{}:{}/{}",
        user, password, host, port, db_name
    );
    
    let clear_sql = r#"
-- Clear all session data but keep schema
DELETE FROM mcp_session_events;
DELETE FROM mcp_sessions;
-- Reset sequences
ALTER SEQUENCE IF EXISTS mcp_session_events_id_seq RESTART WITH 1;
"#;
    
    execute_sql(&database_url, clear_sql, "Session data cleared successfully").await
}

async fn drop_tables(
    host: &str,
    port: &str,
    db_name: &str,
    user: &str,
    password: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🗑️  Dropping MCP session tables...");
    
    let database_url = format!(
        "postgres://{}:{}@{}:{}/{}",
        user, password, host, port, db_name
    );
    
    let drop_sql = r#"
-- Drop MCP session storage tables
DROP TABLE IF EXISTS mcp_session_events;
DROP TABLE IF EXISTS mcp_sessions;
"#;
    
    execute_sql(&database_url, drop_sql, "Tables dropped successfully").await
}

async fn drop_database(
    host: &str,
    port: &str,
    db_name: &str,
    user: &str,
    password: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("💥 Dropping entire database...");
    
    // Connect to postgres database to drop the target database
    let postgres_url = format!(
        "postgres://{}:{}@{}:{}/postgres",
        user, password, host, port
    );
    
    let drop_sql = &format!("DROP DATABASE IF EXISTS {};", db_name);
    
    execute_sql(&postgres_url, drop_sql, "Database dropped successfully").await
}

async fn stop_docker_container() -> Result<(), Box<dyn std::error::Error>> {
    println!("🛑 Stopping Docker container 'postgres-session'...");
    
    let stop_result = Command::new("docker")
        .args(&["stop", "postgres-session"])
        .output()?;
    
    if stop_result.status.success() {
        println!("✅ Docker container stopped");
    } else {
        let error_output = String::from_utf8_lossy(&stop_result.stderr);
        if error_output.contains("No such container") {
            println!("ℹ️  Container 'postgres-session' not found");
        } else {
            eprintln!("⚠️  Failed to stop container: {}", error_output);
        }
    }
    
    Ok(())
}

async fn remove_docker_container() -> Result<(), Box<dyn std::error::Error>> {
    println!("🗑️  Removing Docker container 'postgres-session'...");
    
    // First stop the container
    let _ = Command::new("docker")
        .args(&["stop", "postgres-session"])
        .output();
    
    // Then remove it
    let remove_result = Command::new("docker")
        .args(&["rm", "postgres-session"])
        .output()?;
    
    if remove_result.status.success() {
        println!("✅ Docker container removed");
    } else {
        let error_output = String::from_utf8_lossy(&remove_result.stderr);
        if error_output.contains("No such container") {
            println!("ℹ️  Container 'postgres-session' not found");
        } else {
            eprintln!("⚠️  Failed to remove container: {}", error_output);
        }
    }
    
    Ok(())
}

async fn execute_sql(
    database_url: &str,
    sql: &str,
    success_msg: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Try to use psql command if available
    let psql_check = Command::new("psql")
        .arg("--version")
        .output();
    
    match psql_check {
        Ok(output) if output.status.success() => {
            let psql_result = Command::new("psql")
                .arg(database_url)
                .arg("-c")
                .arg(sql)
                .output()?;
            
            if psql_result.status.success() {
                println!("✅ {}", success_msg);
            } else {
                let error_output = String::from_utf8_lossy(&psql_result.stderr);
                eprintln!("❌ SQL execution failed: {}", error_output);
                return Err("SQL execution failed".into());
            }
        }
        _ => {
            println!("⚠️  psql not found - you'll need to execute SQL manually");
            println!("📝 Please run the following SQL commands:");
            println!("   Database: {}", database_url.split('@').nth(1).unwrap_or("unknown"));
            println!();
            println!("{}", sql);
            println!();
        }
    }
    
    Ok(())
}