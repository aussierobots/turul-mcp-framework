//! SQLite Setup Utility
//!
//! Creates the SQLite database and required tables for the session storage system.

use std::path::PathBuf;
use tracing::{error, info};
use turul_mcp_session_storage::{SqliteConfig, SqliteSessionStorage};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🚀 SQLite Setup Utility");
    info!("Creating SQLite database and tables for MCP session storage");

    // Get configuration from environment variables (same as main server)
    let database_path = std::env::var("SQLITE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("./sessions.db"));

    info!("Configuration:");
    info!("  Database Path: {}", database_path.display());
    info!("");

    // Create SQLite configuration
    let config = SqliteConfig {
        database_path: database_path.clone(),
        max_connections: 5,
        connection_timeout_secs: 30,
        session_timeout_minutes: 30,
        cleanup_interval_minutes: 5,
        max_events_per_session: 500,
        create_tables_if_missing: true,   // Always true for setup
        create_database_if_missing: true, // Always true for setup
    };

    // Initialize SQLite session storage (this will create tables)
    info!("📁 Creating SQLite database and tables...");
    let _storage = match SqliteSessionStorage::with_config(config).await {
        Ok(storage) => {
            info!("✅ Successfully created SQLite database and tables!");
            info!("📊 Database created at: {}", database_path.display());
            info!("");
            info!("🎉 Setup complete! You can now run the MCP server:");
            info!(
                "  SQLITE_PATH={} cargo run --bin simple-sqlite-session",
                database_path.display()
            );
            storage
        }
        Err(e) => {
            error!("❌ Failed to create SQLite database: {}", e);
            error!("");
            error!("Make sure the directory path exists and is writable:");
            if let Some(parent) = std::path::Path::new(&database_path).parent() {
                error!("  Directory: {}", parent.display());
            }
            return Err(e.into());
        }
    };

    Ok(())
}
