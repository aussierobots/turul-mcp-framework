//! # Session-Aware Resource Server Example
//!
//! This example demonstrates the new session-aware resource capabilities in Phase 6.
//! Resources can now access session context to provide personalized content.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use turul_mcp_derive::McpResource;
use turul_mcp_protocol::resources::ResourceContent;
use turul_mcp_builders::prelude::*;  // HasResourceUri, etc.
use turul_mcp_server::{McpResource, McpResult, McpServer, SessionContext};

/// Session-aware user profile resource that returns different content based on session
#[derive(McpResource, Serialize, Deserialize, Clone)]
#[resource(
    name = "user_profile",
    uri = "file:///session/profile.json",
    description = "Session-aware user profile that adapts content based on session context"
)]
struct SessionAwareProfileResource;

#[async_trait]
impl McpResource for SessionAwareProfileResource {
    async fn read(&self, _params: Option<Value>, session: Option<&SessionContext>) -> McpResult<Vec<ResourceContent>> {
        let content = if let Some(session_ctx) = session {
            // Access session-specific data
            let session_id = &session_ctx.session_id;
            let user_data = session_ctx.get_typed_state::<UserData>("user_data").await
                .unwrap_or_else(UserData::default);

            let profile = serde_json::json!({
                "session_id": session_id.to_string(),
                "username": user_data.username,
                "preferences": user_data.preferences,
                "access_level": user_data.access_level,
                "last_activity": chrono::Utc::now().to_rfc3339(),
                "session_aware": true
            });

            serde_json::to_string_pretty(&profile).unwrap()
        } else {
            // Fallback for when no session is available
            let profile = serde_json::json!({
                "username": "anonymous",
                "session_aware": false,
                "message": "No session context available - using default profile"
            });

            serde_json::to_string_pretty(&profile).unwrap()
        };

        Ok(vec![ResourceContent::blob(
            self.uri().to_string(),
            content,
            "application/json".to_string(),
        )])
    }
}

/// User data stored in session
#[derive(Serialize, Deserialize, Clone)]
struct UserData {
    username: String,
    preferences: std::collections::HashMap<String, String>,
    access_level: String,
}

impl Default for UserData {
    fn default() -> Self {
        let mut preferences = std::collections::HashMap::new();
        preferences.insert("theme".to_string(), "dark".to_string());
        preferences.insert("language".to_string(), "en".to_string());

        Self {
            username: "demo_user".to_string(),
            preferences,
            access_level: "user".to_string(),
        }
    }
}

/// Session-aware activity log resource
#[derive(McpResource, Clone)]
#[resource(
    name = "activity_log",
    uri = "file:///session/activity.log",
    description = "Session-specific activity log"
)]
struct SessionActivityResource;

#[async_trait]
impl McpResource for SessionActivityResource {
    async fn read(&self, _params: Option<Value>, session: Option<&SessionContext>) -> McpResult<Vec<ResourceContent>> {
        let activities = if let Some(session_ctx) = session {
            let session_id = &session_ctx.session_id;

            // Get or initialize activity log from session storage
            let mut activities = session_ctx.get_typed_state::<Vec<String>>("activities").await
                .unwrap_or_else(|| vec![
                    format!("Session {} started", session_id),
                    "User accessed session-aware profile".to_string(),
                ]);

            // Add current access to the log
            activities.push(format!("Activity log accessed at {}", chrono::Utc::now().to_rfc3339()));

            // Update session storage
            let _ = session_ctx.set_typed_state("activities", &activities).await;

            activities
        } else {
            vec!["No session context - cannot track activities".to_string()]
        };

        let content = activities.join("\n");

        Ok(vec![ResourceContent::text(
            self.uri().to_string(),
            content,
        )])
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("Starting Session-Aware MCP Resource Server (Phase 6 Implementation)");

    let server = McpServer::builder()
        .name("session-aware-resource-server")
        .version("0.2.0")
        .title("Session-Aware Resource Server")
        .instructions("This server demonstrates Phase 6 session-aware resources. Resources can access session context to provide personalized content based on user state and preferences.")
        .resource(SessionAwareProfileResource)
        .resource(SessionActivityResource)
        .bind_address("127.0.0.1:8008".parse()?)
        .build()?;

    println!("Session-aware resource server running at: http://127.0.0.1:8008/mcp");
    println!("\nPhase 6 Features Demonstrated:");
    println!("  1. SessionAwareProfileResource - Adapts content based on session state");
    println!("  2. SessionActivityResource - Maintains session-specific activity logs");
    println!("  3. Session state persistence across requests");
    println!("  4. Fallback behavior when no session is available");

    println!("\nResource URIs:");
    println!("  - file:///session/profile.json");
    println!("  - file:///session/activity.log");

    server.run().await?;
    Ok(())
}