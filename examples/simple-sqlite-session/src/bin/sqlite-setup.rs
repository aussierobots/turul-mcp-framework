//! SQLite Database Setup for MCP Session Storage
//!
//! This utility sets up the SQLite database and tables required for
//! MCP session storage. It creates the database file, tables, and indexes needed
//! for the simple-sqlite-session example.

use std::env;
use std::path::Path;
use std::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🗄️  SQLite MCP Session Storage Setup");
    println!("===================================");
    
    // Get database configuration from environment or use defaults
    let db_path = env::var("SQLITE_DB_PATH")
        .unwrap_or_else(|_| "./mcp_sessions.db".to_string());
    let data_dir = env::var("SQLITE_DATA_DIR")
        .unwrap_or_else(|_| "./data".to_string());
    
    println!("📋 Configuration:");
    println!("   Database Path: {}", db_path);
    println!("   Data Directory: {}", data_dir);
    println!();
    
    // Create data directory if it doesn't exist
    if let Some(parent) = Path::new(&db_path).parent() {
        if !parent.exists() {
            println!("📁 Creating directory: {}", parent.display());
            fs::create_dir_all(parent)?;
        }
    }
    
    // Create data directory for backups and logs
    if !Path::new(&data_dir).exists() {
        println!("📁 Creating data directory: {}", data_dir);
        fs::create_dir_all(&data_dir)?;
    }
    
    // Check if database already exists
    let db_exists = Path::new(&db_path).exists();
    if db_exists {
        println!("ℹ️  Database file already exists: {}", db_path);
        
        // Ask user if they want to recreate it
        println!("🤔 What would you like to do?");
        println!("   [1] Keep existing database (recommended)");
        println!("   [2] Backup and recreate database");
        println!("   [3] Delete and recreate database");
        println!("   [0] Cancel");
        
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        
        match input.trim() {
            "1" => {
                println!("✅ Keeping existing database");
                verify_schema(&db_path).await?;
                println!();
                println!("🎉 Setup complete! Database is ready to use.");
                print_usage_info(&db_path, &data_dir);
                return Ok(());
            }
            "2" => {
                backup_database(&db_path, &data_dir)?;
                fs::remove_file(&db_path)?;
                println!("🗑️  Old database removed");
            }
            "3" => {
                fs::remove_file(&db_path)?;
                println!("🗑️  Database deleted");
            }
            "0" => {
                println!("🚫 Canceled");
                return Ok(());
            }
            _ => {
                println!("❌ Invalid option, keeping existing database");
                verify_schema(&db_path).await?;
                print_usage_info(&db_path, &data_dir);
                return Ok(());
            }
        }
    }
    
    println!("🔧 Creating SQLite database: {}", db_path);
    
    // Create SQLite connection and schema
    let connection = rusqlite::Connection::open(&db_path)?;
    
    println!("📊 Creating database schema...");
    
    // Create the database schema
    let schema_sql = r#"
-- MCP Session Storage Schema for SQLite
-- This schema supports session state persistence with event streaming

-- Sessions table
CREATE TABLE IF NOT EXISTS mcp_sessions (
    session_id TEXT PRIMARY KEY,
    client_capabilities TEXT, -- JSON as TEXT in SQLite
    server_capabilities TEXT, -- JSON as TEXT in SQLite
    state TEXT NOT NULL DEFAULT '{}', -- JSON as TEXT in SQLite
    created_at INTEGER NOT NULL,
    last_activity INTEGER NOT NULL,
    is_initialized INTEGER NOT NULL DEFAULT 0, -- BOOLEAN as INTEGER in SQLite
    metadata TEXT NOT NULL DEFAULT '{}' -- JSON as TEXT in SQLite
);

-- Events table for SSE resumability
CREATE TABLE IF NOT EXISTS mcp_session_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    event_id INTEGER NOT NULL,
    timestamp INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    data TEXT NOT NULL, -- JSON as TEXT in SQLite
    retry INTEGER,
    FOREIGN KEY (session_id) REFERENCES mcp_sessions(session_id) ON DELETE CASCADE
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_sessions_last_activity ON mcp_sessions(last_activity);
CREATE INDEX IF NOT EXISTS idx_sessions_created_at ON mcp_sessions(created_at);
CREATE INDEX IF NOT EXISTS idx_events_session_id ON mcp_session_events(session_id);
CREATE INDEX IF NOT EXISTS idx_events_event_id ON mcp_session_events(session_id, event_id);
CREATE INDEX IF NOT EXISTS idx_events_timestamp ON mcp_session_events(timestamp);

-- Unique constraint for event IDs within a session
CREATE UNIQUE INDEX IF NOT EXISTS idx_events_session_event_id ON mcp_session_events(session_id, event_id);

-- SQLite-specific optimizations
PRAGMA journal_mode = WAL; -- Write-Ahead Logging for better concurrency
PRAGMA synchronous = NORMAL; -- Balance between safety and performance
PRAGMA cache_size = 10000; -- Increase cache size for better performance
PRAGMA temp_store = memory; -- Store temporary tables in memory
PRAGMA mmap_size = 268435456; -- Use memory mapping for better I/O (256MB)
"#;
    
    // Execute schema creation
    connection.execute_batch(schema_sql)?;
    
    println!("✅ Database schema created successfully!");
    
    // Verify the schema was created correctly
    verify_schema(&db_path).await?;
    
    // Create a simple test to ensure everything works
    println!("🧪 Running database test...");
    test_database_operations(&connection)?;
    
    println!("✅ Database test passed!");
    
    // Close connection
    drop(connection);
    
    println!();
    println!("🎉 SQLite setup complete!");
    print_usage_info(&db_path, &data_dir);
    
    Ok(())
}

async fn verify_schema(db_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Verifying database schema...");
    
    let connection = rusqlite::Connection::open(db_path)?;
    
    // Check if required tables exist
    let mut stmt = connection.prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")?;
    let table_names: Vec<String> = stmt.query_map([], |row| {
        Ok(row.get::<_, String>(0)?)
    })?.collect::<Result<Vec<_>, _>>()?;
    
    let required_tables = ["mcp_sessions", "mcp_session_events"];
    
    println!("📋 Found tables: {:?}", table_names);
    
    for required_table in required_tables.iter() {
        if table_names.contains(&required_table.to_string()) {
            println!("   ✅ {} - OK", required_table);
        } else {
            println!("   ❌ {} - MISSING", required_table);
            return Err(format!("Required table '{}' not found", required_table).into());
        }
    }
    
    // Check indexes
    let mut stmt = connection.prepare("SELECT name FROM sqlite_master WHERE type='index' AND name LIKE 'idx_%' ORDER BY name")?;
    let index_names: Vec<String> = stmt.query_map([], |row| {
        Ok(row.get::<_, String>(0)?)
    })?.collect::<Result<Vec<_>, _>>()?;
    
    println!("🗂️  Found indexes: {}", index_names.len());
    for index in index_names.iter() {
        println!("   ✅ {}", index);
    }
    
    println!("✅ Schema verification passed!");
    Ok(())
}

fn backup_database(db_path: &str, data_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let backup_path = format!("{}/mcp_sessions_backup_{}.db", data_dir, timestamp);
    
    println!("💾 Creating backup: {}", backup_path);
    
    fs::copy(db_path, &backup_path)?;
    
    println!("✅ Backup created successfully");
    Ok(())
}

fn test_database_operations(connection: &rusqlite::Connection) -> Result<(), Box<dyn std::error::Error>> {
    // Test session insertion
    let test_session_id = "test_setup_session";
    connection.execute(
        "INSERT OR REPLACE INTO mcp_sessions (session_id, state, created_at, last_activity) VALUES (?1, ?2, ?3, ?4)",
        (test_session_id, "{\"test\": true}", chrono::Utc::now().timestamp_millis(), chrono::Utc::now().timestamp_millis()),
    )?;
    
    // Test session retrieval
    let mut stmt = connection.prepare("SELECT state FROM mcp_sessions WHERE session_id = ?1")?;
    let state: String = stmt.query_row([test_session_id], |row| row.get(0))?;
    
    if state.contains("test") {
        println!("   ✅ Session storage test passed");
    } else {
        return Err("Session storage test failed".into());
    }
    
    // Test event insertion
    connection.execute(
        "INSERT INTO mcp_session_events (session_id, event_id, timestamp, event_type, data) VALUES (?1, ?2, ?3, ?4, ?5)",
        (test_session_id, 1, chrono::Utc::now().timestamp_millis(), "test", "{\"message\": \"test event\"}"),
    )?;
    
    // Test event retrieval
    let mut stmt = connection.prepare("SELECT COUNT(*) FROM mcp_session_events WHERE session_id = ?1")?;
    let count: i32 = stmt.query_row([test_session_id], |row| row.get(0))?;
    
    if count > 0 {
        println!("   ✅ Event storage test passed");
    } else {
        return Err("Event storage test failed".into());
    }
    
    // Clean up test data
    connection.execute("DELETE FROM mcp_session_events WHERE session_id = ?1", [test_session_id])?;
    connection.execute("DELETE FROM mcp_sessions WHERE session_id = ?1", [test_session_id])?;
    
    Ok(())
}

fn print_usage_info(db_path: &str, data_dir: &str) {
    println!();
    println!("📋 Database Information:");
    println!("   📄 Database File: {}", db_path);
    println!("   📁 Data Directory: {}", data_dir);
    
    // Get file size
    if let Ok(metadata) = fs::metadata(db_path) {
        let size = metadata.len();
        if size < 1024 {
            println!("   📏 Database Size: {} bytes", size);
        } else if size < 1024 * 1024 {
            println!("   📏 Database Size: {:.1} KB", size as f64 / 1024.0);
        } else {
            println!("   📏 Database Size: {:.1} MB", size as f64 / (1024.0 * 1024.0));
        }
    }
    
    println!();
    println!("🚀 You can now run the MCP server:");
    println!("   cargo run --bin server");
    println!();
    println!("🧪 Test the database:");
    println!("   sqlite3 '{}'", db_path);
    println!("   > .tables");
    println!("   > .schema mcp_sessions");
    println!();
    println!("🔧 Manage the database:");
    println!("   cargo run --bin sqlite-teardown  # Clean up when done");
    println!();
    println!("⚙️  Environment variables:");
    println!("   export SQLITE_DB_PATH={}  # Database file path", db_path);
    println!("   export SQLITE_DATA_DIR={}   # Data directory", data_dir);
}