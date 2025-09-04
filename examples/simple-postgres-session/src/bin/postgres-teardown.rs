//! PostgreSQL Teardown Utility
//! 
//! Drops the PostgreSQL tables used by the session storage system.
//!
//! WARNING: This will permanently delete all session data!

use tracing::{info, warn, error};
use sqlx::{PgPool, Executor};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🧹 PostgreSQL Teardown Utility");
    warn!("⚠️  WARNING: This will permanently delete all session data!");
    info!("Dropping PostgreSQL tables for MCP session storage");

    // Get configuration from environment variables (same as main server)
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://mcp:mcp_pass@localhost:5432/mcp_sessions".to_string());

    info!("Configuration:");
    info!("  Database URL: {}", mask_password(&database_url));
    info!("");

    // Confirm deletion
    warn!("⚠️  About to drop the following PostgreSQL tables:");
    warn!("  • mcp_sessions (main session table)");
    warn!("  • mcp_session_events (events table)");
    warn!("");
    warn!("💀 ALL SESSION DATA WILL BE PERMANENTLY LOST!");
    warn!("");

    // For safety, require explicit confirmation via environment variable
    if std::env::var("CONFIRM_DELETE").unwrap_or_default() != "yes" {
        error!("❌ Deletion cancelled for safety.");
        error!("");
        error!("To confirm deletion, run:");
        error!("  CONFIRM_DELETE=yes DATABASE_URL='{}' cargo run --bin postgres-teardown", mask_password(&database_url));
        error!("");
        return Ok(());
    }

    // Connect to PostgreSQL
    info!("📡 Connecting to PostgreSQL...");
    let pool = match PgPool::connect(&database_url).await {
        Ok(pool) => {
            info!("✅ Connected to PostgreSQL successfully");
            pool
        },
        Err(e) => {
            error!("❌ Failed to connect to PostgreSQL: {}", e);
            error!("Database URL: {}", mask_password(&database_url));
            return Err(e.into());
        }
    };

    // Drop the tables
    info!("🗑️  Dropping PostgreSQL tables...");
    
    // Drop events table first (due to foreign key constraints)
    match pool.execute("DROP TABLE IF EXISTS mcp_session_events").await {
        Ok(_) => info!("✅ Dropped table: mcp_session_events"),
        Err(e) => {
            error!("❌ Failed to drop table mcp_session_events: {}", e);
            return Err(e.into());
        }
    }

    // Drop sessions table
    match pool.execute("DROP TABLE IF EXISTS mcp_sessions").await {
        Ok(_) => info!("✅ Dropped table: mcp_sessions"),
        Err(e) => {
            error!("❌ Failed to drop table mcp_sessions: {}", e);
            return Err(e.into());
        }
    }

    info!("✅ Successfully dropped all PostgreSQL tables!");
    info!("🗑️  Tables dropped:");
    info!("  • mcp_sessions (main session table)");
    info!("  • mcp_session_events (events table)");
    info!("");
    info!("🎉 Teardown complete!");

    Ok(())
}

/// Mask the password in a database URL for logging
fn mask_password(url: &str) -> String {
    if let Ok(parsed) = url::Url::parse(url) {
        if parsed.password().is_some() {
            let mut masked = parsed.clone();
            let _ = masked.set_password(Some("***"));
            return masked.to_string();
        }
    }
    url.to_string()
}