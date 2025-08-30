//! PostgreSQL Database Setup for MCP Session Storage
//!
//! This utility sets up the PostgreSQL database and tables required for
//! MCP session storage. It creates the database, tables, and indexes needed
//! for the simple-postgres-session example.

use std::process::Command;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🐘 PostgreSQL MCP Session Storage Setup");
    println!("======================================");
    
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
    
    // Check if Docker is available
    println!("🔍 Checking if Docker is available...");
    let docker_check = Command::new("docker")
        .arg("--version")
        .output();
    
    match docker_check {
        Ok(output) if output.status.success() => {
            println!("✅ Docker is available");
            
            // Check if PostgreSQL container is already running
            let container_check = Command::new("docker")
                .args(&["ps", "--filter", "name=postgres-session", "--format", "table {{.Names}}"])
                .output()?;
            
            let container_output = String::from_utf8_lossy(&container_check.stdout);
            if container_output.contains("postgres-session") {
                println!("✅ PostgreSQL container 'postgres-session' is already running");
            } else {
                println!("🚀 Starting PostgreSQL container...");
                let start_result = Command::new("docker")
                    .args(&[
                        "run", "-d", "--name", "postgres-session",
                        "-e", &format!("POSTGRES_DB={}", db_name),
                        "-e", &format!("POSTGRES_USER={}", db_user),
                        "-e", &format!("POSTGRES_PASSWORD={}", db_password),
                        "-p", &format!("{}:5432", db_port),
                        "postgres:15"
                    ])
                    .output()?;
                
                if start_result.status.success() {
                    println!("✅ PostgreSQL container started successfully");
                    println!("⏳ Waiting 10 seconds for PostgreSQL to initialize...");
                    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                } else {
                    let error_output = String::from_utf8_lossy(&start_result.stderr);
                    if error_output.contains("already in use") {
                        println!("ℹ️  Container name 'postgres-session' already exists, trying to start it...");
                        let start_existing = Command::new("docker")
                            .args(&["start", "postgres-session"])
                            .output()?;
                        
                        if start_existing.status.success() {
                            println!("✅ Existing PostgreSQL container started");
                            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                        } else {
                            eprintln!("❌ Failed to start existing container: {}", String::from_utf8_lossy(&start_existing.stderr));
                            return Err("Failed to start PostgreSQL container".into());
                        }
                    } else {
                        eprintln!("❌ Failed to start PostgreSQL container: {}", error_output);
                        return Err("Failed to start PostgreSQL container".into());
                    }
                }
            }
        }
        Ok(_) => {
            println!("⚠️  Docker is installed but not working properly");
            println!("ℹ️  Please make sure PostgreSQL is running manually on {}:{}", db_host, db_port);
        }
        Err(_) => {
            println!("⚠️  Docker not found - assuming PostgreSQL is running manually");
            println!("ℹ️  Please make sure PostgreSQL is running on {}:{}", db_host, db_port);
        }
    }
    
    println!();
    println!("🔧 Setting up database schema...");
    
    // Database URL for connection
    let database_url = format!(
        "postgres://{}:{}@{}:{}/{}",
        db_user, db_password, db_host, db_port, db_name
    );
    
    // Create the database schema using psql if available
    let schema_sql = r#"
-- MCP Session Storage Schema
-- This schema supports session state persistence with event streaming

-- Sessions table
CREATE TABLE IF NOT EXISTS mcp_sessions (
    session_id TEXT PRIMARY KEY,
    client_capabilities JSONB,
    server_capabilities JSONB,
    state JSONB NOT NULL DEFAULT '{}',
    created_at BIGINT NOT NULL,
    last_activity BIGINT NOT NULL,
    is_initialized BOOLEAN NOT NULL DEFAULT FALSE,
    metadata JSONB NOT NULL DEFAULT '{}'
);

-- Events table for SSE resumability
CREATE TABLE IF NOT EXISTS mcp_session_events (
    id BIGSERIAL PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES mcp_sessions(session_id) ON DELETE CASCADE,
    event_id BIGINT NOT NULL,
    timestamp BIGINT NOT NULL,
    event_type TEXT NOT NULL,
    data JSONB NOT NULL,
    retry INTEGER
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_sessions_last_activity ON mcp_sessions(last_activity);
CREATE INDEX IF NOT EXISTS idx_sessions_created_at ON mcp_sessions(created_at);
CREATE INDEX IF NOT EXISTS idx_events_session_id ON mcp_session_events(session_id);
CREATE INDEX IF NOT EXISTS idx_events_event_id ON mcp_session_events(session_id, event_id);
CREATE INDEX IF NOT EXISTS idx_events_timestamp ON mcp_session_events(timestamp);

-- Unique constraint for event IDs within a session
CREATE UNIQUE INDEX IF NOT EXISTS idx_events_session_event_id ON mcp_session_events(session_id, event_id);
"#;
    
    // Try to use psql command if available
    let psql_check = Command::new("psql")
        .arg("--version")
        .output();
    
    match psql_check {
        Ok(output) if output.status.success() => {
            println!("✅ psql found, creating schema...");
            
            let psql_result = Command::new("psql")
                .arg(&database_url)
                .arg("-c")
                .arg(schema_sql)
                .output()?;
            
            if psql_result.status.success() {
                println!("✅ Database schema created successfully!");
            } else {
                let error_output = String::from_utf8_lossy(&psql_result.stderr);
                eprintln!("❌ Failed to create schema: {}", error_output);
                return Err("Failed to create database schema".into());
            }
        }
        _ => {
            println!("⚠️  psql not found - you'll need to create the schema manually");
            println!("📝 Please run the following SQL commands in your PostgreSQL database:");
            println!("   Database: {}", db_name);
            println!();
            println!("{}", schema_sql);
            println!();
            println!("💡 You can also run:");
            println!("   psql '{}' < schema.sql", database_url.replace(&db_password, "***"));
        }
    }
    
    println!();
    println!("🎉 PostgreSQL setup complete!");
    println!();
    println!("📋 Connection details:");
    println!("   Database URL: {}", database_url.replace(&db_password, "***"));
    println!();
    println!("🚀 You can now run the MCP server:");
    println!("   cargo run --bin server");
    println!();
    println!("🧪 Test the connection:");
    println!("   psql '{}'", database_url.replace(&db_password, "***"));
    
    Ok(())
}