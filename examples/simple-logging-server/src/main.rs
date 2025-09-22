//! # Simple Logging Server Example
//!
//! This example demonstrates basic MCP logging protocol features with session-based log storage.
//! It shows how to create log messages, manage log levels, and query logs using derive macros.
//!
//! **Key Features Demonstrated:**
//! - SessionContext for persistent state across tool calls
//! - Derive macros for 90% code reduction vs manual implementation
//! - Session-based progress notifications
//! - Type-safe parameter extraction and validation

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use turul_mcp_derive::McpTool;
use turul_mcp_protocol::McpError;
use turul_mcp_server::{McpResult, McpServer, SessionContext};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LogEntry {
    timestamp: DateTime<Utc>,
    level: String,
    message: String,
    category: Option<String>,
    session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LogLevelConfig {
    global_level: String,
    category_levels: HashMap<String, String>,
}

impl Default for LogLevelConfig {
    fn default() -> Self {
        Self {
            global_level: "INFO".to_string(),
            category_levels: HashMap::new(),
        }
    }
}

/// Log a message with specified level and category
#[derive(McpTool, Default)]
#[tool(
    name = "log_message",
    description = "Log a message with specified level and optional category"
)]
struct LogMessageTool {
    #[param(description = "Log level (DEBUG, INFO, WARN, ERROR)")]
    level: String,
    #[param(description = "Log message content")]
    message: String,
    #[param(description = "Optional log category", optional)]
    category: Option<String>,
}

impl LogMessageTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<serde_json::Value> {
        let session =
            session.ok_or_else(|| McpError::SessionError("Session required".to_string()))?;

        // Get current log level config to check if we should log
        let log_config: LogLevelConfig = session
            .get_typed_state("log_config")
            .await
            .unwrap_or_default();

        // Simple level checking (DEBUG=0, INFO=1, WARN=2, ERROR=3)
        let level_priority = match self.level.as_str() {
            "DEBUG" => 0,
            "INFO" => 1,
            "WARN" => 2,
            "ERROR" => 3,
            _ => {
                return Err(McpError::invalid_param_type(
                    "level",
                    "DEBUG|INFO|WARN|ERROR",
                    &self.level,
                ));
            }
        };

        let min_priority = match log_config.global_level.as_str() {
            "DEBUG" => 0,
            "INFO" => 1,
            "WARN" => 2,
            "ERROR" => 3,
            _ => 1, // Default to INFO
        };

        if level_priority < min_priority {
            return Ok(json!({
                "logged": false,
                "reason": format!("Message level {} below minimum {}", self.level, log_config.global_level)
            }));
        }

        // Create log entry
        let entry = LogEntry {
            timestamp: Utc::now(),
            level: self.level.clone(),
            message: self.message.clone(),
            category: self.category.clone(),
            session_id: session.session_id.clone(),
        };

        // Store in session logs
        let mut logs: Vec<LogEntry> = session.get_typed_state("logs").await.unwrap_or_default();
        logs.push(entry.clone());
        session.set_typed_state("logs", &logs).await.unwrap();

        // Send progress notification
        session
            .notify_progress(format!("log_{}", self.level.to_lowercase()), 1)
            .await;

        Ok(json!({
            "logged": true,
            "timestamp": entry.timestamp,
            "level": entry.level,
            "message": entry.message,
            "category": entry.category,
            "log_count": logs.len()
        }))
    }
}

/// Set the global log level or category-specific log level
#[derive(McpTool, Default)]
#[tool(
    name = "set_log_level",
    description = "Set global log level or category-specific log level"
)]
struct SetLogLevelTool {
    #[param(description = "Log level to set (DEBUG, INFO, WARN, ERROR)")]
    level: String,
    #[param(
        description = "Optional category (if not provided, sets global level)",
        optional
    )]
    category: Option<String>,
}

impl SetLogLevelTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<serde_json::Value> {
        let session =
            session.ok_or_else(|| McpError::SessionError("Session required".to_string()))?;

        // Validate level
        if !matches!(self.level.as_str(), "DEBUG" | "INFO" | "WARN" | "ERROR") {
            return Err(McpError::invalid_param_type(
                "level",
                "DEBUG|INFO|WARN|ERROR",
                &self.level,
            ));
        }

        // Get or create log config
        let mut log_config: LogLevelConfig = session
            .get_typed_state("log_config")
            .await
            .unwrap_or_default();

        let result = if let Some(cat) = &self.category {
            // Set category-specific level
            log_config
                .category_levels
                .insert(cat.to_string(), self.level.to_string());
            json!({
                "action": "set_category_level",
                "category": cat,
                "level": self.level,
                "message": format!("Set log level for category '{}' to {}", cat, self.level)
            })
        } else {
            // Set global level
            log_config.global_level = self.level.to_string();
            json!({
                "action": "set_global_level",
                "level": self.level,
                "message": format!("Set global log level to {}", self.level)
            })
        };

        // Save config
        session
            .set_typed_state("log_config", &log_config)
            .await
            .unwrap();

        Ok(result)
    }
}

/// Get current log level configuration and recent logs
#[derive(McpTool, Default)]
#[tool(
    name = "get_logs_status",
    description = "Get current log level configuration and recent logs"
)]
struct GetLogsStatusTool {
    #[param(
        description = "Maximum number of recent logs to return (default: 10)",
        optional
    )]
    limit: Option<i64>,
    #[param(description = "Filter logs by level (DEBUG|INFO|WARN|ERROR)", optional)]
    level_filter: Option<String>,
}

impl GetLogsStatusTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<serde_json::Value> {
        let session =
            session.ok_or_else(|| McpError::SessionError("Session required".to_string()))?;

        let limit = self.limit.unwrap_or(10) as usize;
        let level_filter = self.level_filter.as_deref();

        // Get current config and logs
        let log_config: LogLevelConfig = session
            .get_typed_state("log_config")
            .await
            .unwrap_or_default();
        let logs: Vec<LogEntry> = session.get_typed_state("logs").await.unwrap_or_default();

        // Filter and limit logs
        let mut filtered_logs: Vec<&LogEntry> = logs.iter().collect();
        if let Some(filter) = level_filter {
            filtered_logs.retain(|log| log.level == filter);
        }

        // Get most recent logs
        filtered_logs.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        filtered_logs.truncate(limit);

        Ok(json!({
            "log_config": {
                "global_level": log_config.global_level,
                "category_levels": log_config.category_levels
            },
            "logs": {
                "total_count": logs.len(),
                "filtered_count": filtered_logs.len(),
                "recent_logs": filtered_logs.iter().map(|log| json!({
                    "timestamp": log.timestamp,
                    "level": log.level,
                    "message": log.message,
                    "category": log.category
                })).collect::<Vec<_>>()
            },
            "session_id": session.session_id
        }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("Starting Simple Logging MCP Server");

    let server = McpServer::builder()
        .name("simple-logging-server")
        .version("1.0.0")
        .title("Simple Logging Server")
        .instructions("This server demonstrates basic MCP logging protocol features with session-based storage.")
        .tool(LogMessageTool::default())
        .tool(SetLogLevelTool::default())
        .tool(GetLogsStatusTool::default())
        .bind_address("127.0.0.1:8008".parse()?)
        .sse(true)
        .build()?;

    println!("Simple logging server running at: http://127.0.0.1:8008/mcp");
    println!("\nAvailable tools:");
    println!("  - log_message: Log a message with level and optional category");
    println!("  - set_log_level: Set global or category-specific log level");
    println!("  - get_logs_status: Get log configuration and recent logs");
    println!("\nExample usage:");
    println!("  1. log_message(level='INFO', message='Hello world')");
    println!("  2. set_log_level(level='DEBUG')");
    println!("  3. get_logs_status(limit=5)");

    server.run().await?;
    Ok(())
}
