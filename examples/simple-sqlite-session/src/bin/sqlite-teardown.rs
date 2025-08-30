//! SQLite Database Teardown for MCP Session Storage
//!
//! This utility cleans up the SQLite database created for MCP session storage
//! testing. It can clear data, drop tables, backup, or delete the database file.

use std::env;
use std::path::Path;
use std::fs;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧹 SQLite MCP Session Storage Teardown");
    println!("======================================");
    
    // Get database configuration from environment or use defaults
    let db_path = env::var("SQLITE_DB_PATH")
        .unwrap_or_else(|_| "./mcp_sessions.db".to_string());
    let data_dir = env::var("SQLITE_DATA_DIR")
        .unwrap_or_else(|_| "./data".to_string());
    
    println!("📋 Configuration:");
    println!("   Database Path: {}", db_path);
    println!("   Data Directory: {}", data_dir);
    println!();
    
    // Check if database exists
    if !Path::new(&db_path).exists() {
        println!("ℹ️  Database file not found: {}", db_path);
        println!("Nothing to clean up!");
        return Ok(());
    }
    
    // Get file size for display
    let db_size = fs::metadata(&db_path)?.len();
    let size_str = if db_size < 1024 {
        format!("{} bytes", db_size)
    } else if db_size < 1024 * 1024 {
        format!("{:.1} KB", db_size as f64 / 1024.0)
    } else {
        format!("{:.1} MB", db_size as f64 / (1024.0 * 1024.0))
    };
    
    println!("📄 Found database: {} ({})", db_path, size_str);
    
    // Check command line arguments for options
    let args: Vec<String> = env::args().collect();
    let clear_data = args.contains(&"--clear-data".to_string()) || args.contains(&"--all".to_string());
    let drop_tables = args.contains(&"--drop-tables".to_string()) || args.contains(&"--all".to_string());
    let backup_db = args.contains(&"--backup".to_string()) || args.contains(&"--all".to_string());
    let delete_db = args.contains(&"--delete".to_string()) || args.contains(&"--all".to_string());
    let vacuum_db = args.contains(&"--vacuum".to_string()) || args.contains(&"--all".to_string());
    
    // If no specific options, ask user what to do
    if !clear_data && !drop_tables && !backup_db && !delete_db && !vacuum_db {
        println!("🤔 What would you like to clean up?");
        println!("   [1] Clear session data only (keep tables and schema)");
        println!("   [2] Drop tables (removes all tables and data)");
        println!("   [3] Backup database (create timestamped backup)");
        println!("   [4] Vacuum database (reclaim space, optimize)");
        println!("   [5] Delete database file (complete removal)");
        println!("   [6] Full cleanup (backup + delete)");
        println!("   [0] Cancel");
        println!();
        print!("Choose option (0-6): ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        match input.trim() {
            "1" => {
                clear_session_data(&db_path).await?;
            }
            "2" => {
                drop_tables(&db_path).await?;
            }
            "3" => {
                backup_database(&db_path, &data_dir)?;
            }
            "4" => {
                vacuum_database(&db_path).await?;
            }
            "5" => {
                delete_database(&db_path)?;
            }
            "6" => {
                backup_database(&db_path, &data_dir)?;
                delete_database(&db_path)?;
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
        if backup_db {
            backup_database(&db_path, &data_dir)?;
        }
        if clear_data {
            clear_session_data(&db_path).await?;
        }
        if drop_tables {
            drop_tables(&db_path).await?;
        }
        if vacuum_db {
            vacuum_database(&db_path).await?;
        }
        if delete_db {
            delete_database(&db_path)?;
        }
    }
    
    println!();
    println!("🎉 Teardown complete!");
    println!();
    println!("💡 Command line options for next time:");
    println!("   --clear-data    Clear session data but keep schema");
    println!("   --drop-tables   Drop all tables");
    println!("   --backup        Create backup before other operations");
    println!("   --vacuum        Optimize database file size");
    println!("   --delete        Delete database file completely");
    println!("   --all           Backup + clear everything");
    
    Ok(())
}

async fn clear_session_data(db_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("🧹 Clearing session data...");
    
    let connection = rusqlite::Connection::open(db_path)?;
    
    // Count existing data
    let session_count: i32 = connection.query_row("SELECT COUNT(*) FROM mcp_sessions", [], |row| row.get(0))?;
    let event_count: i32 = connection.query_row("SELECT COUNT(*) FROM mcp_session_events", [], |row| row.get(0))?;
    
    println!("📊 Found {} sessions and {} events", session_count, event_count);
    
    if session_count == 0 && event_count == 0 {
        println!("ℹ️  Database is already empty");
        return Ok(());
    }
    
    // Clear all data but keep schema
    let mut deleted_events = 0;
    let mut deleted_sessions = 0;
    
    if event_count > 0 {
        deleted_events = connection.execute("DELETE FROM mcp_session_events", [])?;
        println!("   🗑️  Deleted {} events", deleted_events);
    }
    
    if session_count > 0 {
        deleted_sessions = connection.execute("DELETE FROM mcp_sessions", [])?;
        println!("   🗑️  Deleted {} sessions", deleted_sessions);
    }
    
    // Reset auto-increment counter
    connection.execute("DELETE FROM sqlite_sequence WHERE name='mcp_session_events'", [])?;
    
    println!("✅ Session data cleared successfully");
    Ok(())
}

async fn drop_tables(db_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("🗑️  Dropping MCP session tables...");
    
    let connection = rusqlite::Connection::open(db_path)?;
    
    // Drop tables in correct order (events first due to foreign key)
    connection.execute("DROP TABLE IF EXISTS mcp_session_events", [])?;
    println!("   ✅ Dropped mcp_session_events table");
    
    connection.execute("DROP TABLE IF EXISTS mcp_sessions", [])?;
    println!("   ✅ Dropped mcp_sessions table");
    
    // Clean up sequence table
    connection.execute("DELETE FROM sqlite_sequence WHERE name IN ('mcp_session_events', 'mcp_sessions')", [])?;
    
    println!("✅ Tables dropped successfully");
    Ok(())
}

async fn vacuum_database(db_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("🧹 Vacuuming database (optimizing and reclaiming space)...");
    
    // Get size before vacuum
    let size_before = fs::metadata(db_path)?.len();
    
    let connection = rusqlite::Connection::open(db_path)?;
    
    // Run VACUUM to reclaim space and optimize
    connection.execute("VACUUM", [])?;
    
    // Also run ANALYZE to update statistics
    connection.execute("ANALYZE", [])?;
    
    drop(connection);
    
    // Get size after vacuum
    let size_after = fs::metadata(db_path)?.len();
    
    let saved = size_before.saturating_sub(size_after);
    let saved_str = if saved < 1024 {
        format!("{} bytes", saved)
    } else if saved < 1024 * 1024 {
        format!("{:.1} KB", saved as f64 / 1024.0)
    } else {
        format!("{:.1} MB", saved as f64 / (1024.0 * 1024.0))
    };
    
    println!("✅ Database vacuumed successfully");
    println!("   📏 Size before: {:.1} KB", size_before as f64 / 1024.0);
    println!("   📏 Size after: {:.1} KB", size_after as f64 / 1024.0);
    if saved > 0 {
        println!("   💾 Space saved: {}", saved_str);
    }
    
    Ok(())
}

fn backup_database(db_path: &str, data_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("💾 Creating database backup...");
    
    // Create data directory if it doesn't exist
    if !Path::new(data_dir).exists() {
        fs::create_dir_all(data_dir)?;
        println!("   📁 Created data directory: {}", data_dir);
    }
    
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let backup_path = format!("{}/mcp_sessions_backup_{}.db", data_dir, timestamp);
    
    fs::copy(db_path, &backup_path)?;
    
    let backup_size = fs::metadata(&backup_path)?.len();
    let size_str = if backup_size < 1024 * 1024 {
        format!("{:.1} KB", backup_size as f64 / 1024.0)
    } else {
        format!("{:.1} MB", backup_size as f64 / (1024.0 * 1024.0))
    };
    
    println!("✅ Backup created: {} ({})", backup_path, size_str);
    Ok(())
}

fn delete_database(db_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("💥 Deleting database file...");
    
    if Path::new(db_path).exists() {
        fs::remove_file(db_path)?;
        println!("✅ Database file deleted: {}", db_path);
    } else {
        println!("ℹ️  Database file not found: {}", db_path);
    }
    
    // Also remove any SQLite temporary files
    let temp_files = [
        format!("{}-shm", db_path),  // Shared memory file
        format!("{}-wal", db_path),  // Write-ahead log file
    ];
    
    for temp_file in temp_files.iter() {
        if Path::new(temp_file).exists() {
            fs::remove_file(temp_file)?;
            println!("✅ Removed temporary file: {}", temp_file);
        }
    }
    
    Ok(())
}