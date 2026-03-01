// turul-mcp-server v0.3
// Shared Application State — OnceLock pattern for database access
//
// This is the framework-idiomatic pattern for tools that need shared
// dependencies (database pools, API clients, configuration).
// DO NOT use ToolBuilder just because a tool needs a database connection.
//
// Source pattern: examples/audit-trail-server, examples/dynamic-resource-server

use std::sync::{Arc, OnceLock};

use schemars::JsonSchema;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use turul_mcp_derive::mcp_tool;
use turul_mcp_protocol::McpError;
use turul_mcp_server::{McpResult, McpServer};

// ── Shared state via OnceLock ──────────────────────────────────────

/// Module-level shared state. Initialized once at startup, immutable after.
static DB: OnceLock<Arc<DatabaseConnection>> = OnceLock::new();

/// Helper to access the database connection from any tool.
fn get_db() -> McpResult<&'static Arc<DatabaseConnection>> {
    DB.get()
        .ok_or_else(|| McpError::tool_execution("Database not initialized"))
}

// ── Output types with schemars ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProfileSummary {
    /// User's display name
    pub name: String,
    /// Number of followers
    pub followers_count: u64,
    /// Profile image URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchResponse {
    /// Matching profiles
    pub profiles: Vec<ProfileSummary>,
}

// ── Tools using function macros + OnceLock ──────────────────────────

/// Simple lookup tool — function macro with OnceLock database access.
/// Output schema auto-detected from return type (ProfileSummary derives JsonSchema).
#[mcp_tool(name = "get_profile", description = "Get user profile by username")]
async fn get_profile(
    #[param(description = "Username to look up")] username: String,
) -> McpResult<ProfileSummary> {
    let db = get_db()?;

    // Your database query here
    let _profile = db; // placeholder for: queries::latest_profile(db, &username).await
    Ok(ProfileSummary {
        name: username,
        followers_count: 42,
        image_url: None,
    })
}

/// Search tool returning Vec<T> — output schema auto-detected.
#[mcp_tool(name = "search_profiles", description = "Search profiles by keyword")]
async fn search_profiles(
    #[param(description = "Search query")] query: String,
    #[param(description = "Max results to return")] limit: Option<u64>,
) -> McpResult<SearchResponse> {
    let db = get_db()?;
    let _limit = limit.unwrap_or(10);

    // Your database search here
    let _results = db; // placeholder for: queries::search(db, &query, limit).await
    Ok(SearchResponse {
        profiles: vec![ProfileSummary {
            name: format!("Match for: {}", query),
            followers_count: 100,
            image_url: None,
        }],
    })
}

// ── Server setup ───────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create your database connection (sea-orm, sqlx, etc.)
    let db: Arc<DatabaseConnection> = todo!("establish database connection");

    // Initialize the OnceLock BEFORE building the server
    DB.set(db).expect("DB already initialized");

    let server = McpServer::builder()
        .name("profile-server")
        .version("0.1.0")
        .tool_fn(get_profile)       // Function macros use .tool_fn()
        .tool_fn(search_profiles)
        .build()?;

    server.run().await?;
    Ok(())
}
