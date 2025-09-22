//! SQLite Teardown Utility
//!
//! Deletes the SQLite database file used by the session storage system.
//!
//! WARNING: This will permanently delete all session data!

use std::fs;
use std::path::Path;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🧹 SQLite Teardown Utility");
    warn!("⚠️  WARNING: This will permanently delete all session data!");
    info!("Deleting SQLite database file for MCP session storage");

    // Get configuration from environment variables (same as main server)
    let database_path =
        std::env::var("SQLITE_PATH").unwrap_or_else(|_| "./sessions.db".to_string());

    info!("Configuration:");
    info!("  Database Path: {}", database_path);
    info!("");

    // Check if database file exists
    let path = Path::new(&database_path);
    if !path.exists() {
        info!("ℹ️  Database file does not exist: {}", database_path);
        info!("Nothing to delete.");
        return Ok(());
    }

    // Confirm deletion
    warn!("⚠️  About to delete the SQLite database file:");
    warn!("  • {}", database_path);
    warn!("");
    warn!("💀 ALL SESSION DATA WILL BE PERMANENTLY LOST!");
    warn!("");

    // For safety, require explicit confirmation via environment variable
    if std::env::var("CONFIRM_DELETE").unwrap_or_default() != "yes" {
        error!("❌ Deletion cancelled for safety.");
        error!("");
        error!("To confirm deletion, run:");
        error!(
            "  CONFIRM_DELETE=yes SQLITE_PATH={} cargo run --bin sqlite-teardown",
            database_path
        );
        error!("");
        return Ok(());
    }

    // Delete the database file
    info!("🗑️  Deleting SQLite database file...");
    match fs::remove_file(&database_path) {
        Ok(_) => {
            info!("✅ Successfully deleted SQLite database file!");
            info!("🗑️  Deleted: {}", database_path);
            info!("");
            info!("🎉 Teardown complete!");
        }
        Err(e) => {
            error!("❌ Failed to delete database file: {}", e);
            error!("Path: {}", database_path);
            return Err(e.into());
        }
    }

    // Also try to delete WAL and SHM files if they exist
    let wal_path = format!("{}-wal", database_path);
    let shm_path = format!("{}-shm", database_path);

    if Path::new(&wal_path).exists() {
        if let Err(e) = fs::remove_file(&wal_path) {
            warn!("⚠️  Failed to delete WAL file {}: {}", wal_path, e);
        } else {
            info!("🗑️  Also deleted WAL file: {}", wal_path);
        }
    }

    if Path::new(&shm_path).exists() {
        if let Err(e) = fs::remove_file(&shm_path) {
            warn!("⚠️  Failed to delete SHM file {}: {}", shm_path, e);
        } else {
            info!("🗑️  Also deleted SHM file: {}", shm_path);
        }
    }

    Ok(())
}
