//! PostgreSQL Setup Utility
//! 
//! Creates the PostgreSQL tables required for the session storage system.

use turul_mcp_session_storage::{PostgresSessionStorage, PostgresConfig};
use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🚀 PostgreSQL Setup Utility");
    info!("Creating PostgreSQL tables for MCP session storage");

    // Get configuration from environment variables (same as main server)
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://mcp:mcp_pass@localhost:5432/mcp_sessions".to_string());

    info!("Configuration:");
    info!("  Database URL: {}", mask_password(&database_url));
    info!("");

    // Create PostgreSQL configuration
    let config = PostgresConfig {
        database_url: database_url.clone(),
        max_connections: 10,
        min_connections: 2,
        connection_timeout_secs: 30,
        session_timeout_minutes: 60,
        cleanup_interval_minutes: 10,
        max_events_per_session: 1000,
        enable_pooling_optimizations: true,
        statement_timeout_secs: 30,
        create_tables_if_missing: true, // Always true for setup
    };

    // Initialize PostgreSQL session storage (this will create tables)
    info!("📡 Connecting to PostgreSQL and creating tables...");
    let _storage = match PostgresSessionStorage::with_config(config).await {
        Ok(storage) => {
            info!("✅ Successfully created PostgreSQL tables!");
            info!("📊 Tables created in database: {}", mask_password(&database_url));
            info!("");
            info!("🎉 Setup complete! You can now run the MCP server:");
            info!("  DATABASE_URL='{}' cargo run --bin simple-postgres-session", mask_password(&database_url));
            storage
        },
        Err(e) => {
            error!("❌ Failed to create PostgreSQL tables: {}", e);
            error!("");
            error!("Make sure PostgreSQL is running and accessible:");
            error!("  Database URL: {}", mask_password(&database_url));
            error!("");
            error!("Example Docker setup:");
            error!("  docker run -d --name postgres-session \\");
            error!("    -e POSTGRES_DB=mcp_sessions \\");
            error!("    -e POSTGRES_USER=mcp \\");
            error!("    -e POSTGRES_PASSWORD=mcp_pass \\");
            error!("    -p 5432:5432 \\");
            error!("    postgres:15");
            return Err(e.into());
        }
    };

    Ok(())
}

/// Mask the password in a database URL for logging
fn mask_password(url: &str) -> String {
    if let Ok(parsed) = url::Url::parse(url)
        && parsed.password().is_some() {
            let mut masked = parsed.clone();
            let _ = masked.set_password(Some("***"));
            return masked.to_string();
        }
    url.to_string()
}