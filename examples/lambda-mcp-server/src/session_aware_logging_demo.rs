//! Session-Aware Logging Demo
//!
//! This example demonstrates the new session-aware logging functionality where:
//! - Each session can have its own LoggingLevel filter
//! - Log messages are filtered per-session based on their configured level
//! - LoggingBuilder from turul-mcp-builders can send messages directly to sessions
//! 
//! This is particularly useful for multi-tenant scenarios where different 
//! clients may want different levels of verbosity.

use serde_json::json;
use turul_mcp_derive::McpTool;
use turul_mcp_protocol::logging::LoggingLevel;
use turul_mcp_server::{McpResult, SessionContext};
use turul_mcp_builders::logging::LoggingBuilder;

/// Helper function to convert string level to LoggingLevel enum
fn str_to_logging_level(level: &str) -> LoggingLevel {
    match level.to_lowercase().as_str() {
        "debug" => LoggingLevel::Debug,
        "info" => LoggingLevel::Info,
        "notice" => LoggingLevel::Notice,
        "warning" => LoggingLevel::Warning,
        "error" => LoggingLevel::Error,
        "critical" => LoggingLevel::Critical,
        "alert" => LoggingLevel::Alert,
        "emergency" => LoggingLevel::Emergency,
        _ => LoggingLevel::Info, // Default fallback
    }
}

/// A tool that demonstrates session-aware logging by generating log messages
/// at different levels and showing how they're filtered per session.
#[derive(McpTool, Default, Clone)]
#[tool(
    name = "session_logging_demo",
    description = "Demonstrates session-aware logging with different verbosity levels"
)]
pub struct SessionLoggingDemoTool {
    #[param(description = "Operation to perform")]
    pub operation: String,
    
    #[param(description = "Generate this many log messages")]
    pub count: i32,
}

impl SessionLoggingDemoTool {
    pub async fn execute(&self, session: Option<SessionContext>) -> McpResult<serde_json::Value> {
        let session = session.ok_or("Session context required for logging demo")?;
        
        // Show current session's logging level
        let current_level = session.get_logging_level();
        session.notify_log(str_to_logging_level("info"), serde_json::json!(format!("Current session logging level: {:?}", current_level)), Some("demo".to_string()), None);
        
        let operation = if self.operation.is_empty() { "all" } else { &self.operation };
        let count = if self.count <= 0 { 5 } else { self.count };
        
        match operation {
            "all" => self.demonstrate_all_levels(&session, count).await,
            "cascade" => self.demonstrate_level_cascade(&session).await,
            "builders" => self.demonstrate_logging_builders(&session).await,
            _ => {
                session.notify_log(str_to_logging_level("warning"), serde_json::json!(format!("Unknown operation: {}", operation)), Some("demo".to_string()), None);
                Ok(json!({"error": "Unknown operation", "available": ["all", "cascade", "builders"]}))
            }
        }
    }
    
    /// Demonstrate logging at all different levels
    async fn demonstrate_all_levels(&self, session: &SessionContext, count: i32) -> McpResult<serde_json::Value> {
        session.notify_log(str_to_logging_level("info"), serde_json::json!("🚀 Starting all levels demonstration"), Some("demo".to_string()), None);
        
        let levels = [
            ("debug", "🐛 Debug message - lowest priority"),
            ("info", "ℹ️ Info message - general information"),
            ("notice", "📢 Notice message - significant events"),
            ("warning", "⚠️ Warning message - potential issues"),
            ("error", "❌ Error message - error conditions"),
            ("critical", "🔥 Critical message - critical conditions"),
            ("alert", "🚨 Alert message - immediate action required"),
            ("emergency", "💥 Emergency message - system unusable"),
        ];
        
        for i in 0..count {
            for (level, message) in &levels {
                let full_message = format!("{} (iteration {})", message, i + 1);
                session.notify_log(str_to_logging_level(level), serde_json::json!(full_message), Some("demo".to_string()), None);
            }
        }
        
        session.notify_log(str_to_logging_level("info"), serde_json::json!("✅ All levels demonstration complete"), Some("demo".to_string()), None);
        
        Ok(json!({
            "demonstration": "all_levels",
            "levels_tested": levels.len(),
            "iterations": count,
            "note": "Messages filtered based on your session's logging level"
        }))
    }
    
    /// Demonstrate how changing logging level affects message delivery
    async fn demonstrate_level_cascade(&self, session: &SessionContext) -> McpResult<serde_json::Value> {
        session.notify_log(str_to_logging_level("info"), serde_json::json!("🔄 Starting level cascade demonstration"), Some("demo".to_string()), None);
        
        let test_levels = [
            LoggingLevel::Debug,
            LoggingLevel::Info,
            LoggingLevel::Warning,
            LoggingLevel::Error,
        ];
        
        let original_level = session.get_logging_level();
        
        for test_level in &test_levels {
            // Temporarily set the session's logging level
            session.set_logging_level(*test_level);
            
            session.notify_log(str_to_logging_level("info"), serde_json::json!(format!("📊 Setting logging level to: {:?}", test_level)), Some("demo".to_string()), None);
            
            // Send messages at different levels to show filtering
            session.notify_log(str_to_logging_level("debug"), serde_json::json!("  → Debug message (priority 0)"), Some("demo".to_string()), None);
            session.notify_log(str_to_logging_level("info"), serde_json::json!("  → Info message (priority 1)"), Some("demo".to_string()), None);
            session.notify_log(str_to_logging_level("warning"), serde_json::json!("  → Warning message (priority 3)"), Some("demo".to_string()), None);
            session.notify_log(str_to_logging_level("error"), serde_json::json!("  → Error message (priority 4)"), Some("demo".to_string()), None);
            
            session.notify_log(str_to_logging_level("info"), serde_json::json!(format!("  Only messages >= {:?} should appear above", test_level)), Some("demo".to_string()), None);
        }
        
        // Restore original level
        session.set_logging_level(original_level);
        session.notify_log(str_to_logging_level("info"), serde_json::json!(format!("🔙 Restored original logging level: {:?}", original_level)), Some("demo".to_string()), None);
        
        Ok(json!({
            "demonstration": "level_cascade",
            "levels_tested": test_levels.len(),
            "original_level": format!("{:?}", original_level),
            "note": "Notice how different levels filter different messages"
        }))
    }
    
    /// Demonstrate using LoggingBuilder with session-aware functionality
    async fn demonstrate_logging_builders(&self, session: &SessionContext) -> McpResult<serde_json::Value> {
        session.notify_log(str_to_logging_level("info"), serde_json::json!("🔧 Starting LoggingBuilder demonstration"), Some("demo".to_string()), None);
        
        // Create various loggers using the builder pattern
        let loggers = vec![
            LoggingBuilder::debug(json!("Debug logger with structured data"))
                .logger("demo-debug")
                .build_session_aware(),
            
            LoggingBuilder::info(json!({"message": "Info with JSON data", "timestamp": "2024-01-01T00:00:00Z"}))
                .logger("demo-info")
                .build_session_aware(),
            
            LoggingBuilder::warning(json!("Warning logger message"))
                .logger("demo-warning")
                .meta_value("demo_id", json!("builder-demo"))
                .build_session_aware(),
            
            LoggingBuilder::error(json!({
                "error": "Demonstration error",
                "code": 500,
                "details": "This is just a demo, not a real error"
            }))
            .logger("demo-error")
            .build_session_aware(),
        ];
        
        session.notify_log(str_to_logging_level("info"), serde_json::json!("📤 Sending messages via LoggingBuilder (filtered by session level):"), Some("demo".to_string()), None);
        
        for (i, logger) in loggers.iter().enumerate() {
            // Check if this message would be sent to the session
            let would_send = logger.would_send_to_target(session);
            let level_str = logger.level_to_string();
            
            session.notify_log(str_to_logging_level("info"), serde_json::json!(format!("  📋 Logger {} ({}): {}", 
                i + 1, level_str, 
                if would_send { "✅ Will send" } else { "❌ Filtered out" }
            )), Some("demo".to_string()), None);
            
            // Send the message (will be filtered automatically)
            logger.send_to_target(session);
        }
        
        session.notify_log(str_to_logging_level("info"), serde_json::json!("✅ LoggingBuilder demonstration complete"), Some("demo".to_string()), None);
        
        Ok(json!({
            "demonstration": "logging_builders",
            "builders_created": loggers.len(),
            "session_level": format!("{:?}", session.get_logging_level()),
            "note": "LoggingBuilder automatically filters based on session level"
        }))
    }
}

/// Tool to change the current session's logging level
#[derive(McpTool, Default, Clone)]
#[tool(
    name = "set_logging_level",
    description = "Change the logging level for the current session"
)]
pub struct SetLoggingLevelTool {
    #[param(description = "New logging level (debug, info, notice, warning, error, critical, alert, emergency)")]
    pub level: String,
}

impl SetLoggingLevelTool {
    pub async fn execute(&self, session: Option<SessionContext>) -> McpResult<serde_json::Value> {
        let session = session.ok_or("Session context required")?;
        
        let new_level = match self.level.to_lowercase().as_str() {
            "debug" => LoggingLevel::Debug,
            "info" => LoggingLevel::Info,
            "notice" => LoggingLevel::Notice,
            "warning" => LoggingLevel::Warning,
            "error" => LoggingLevel::Error,
            "critical" => LoggingLevel::Critical,
            "alert" => LoggingLevel::Alert,
            "emergency" => LoggingLevel::Emergency,
            _ => return Ok(json!({
                "error": "Invalid logging level",
                "provided": self.level,
                "valid_levels": ["debug", "info", "notice", "warning", "error", "critical", "alert", "emergency"]
            }))
        };
        
        let old_level = session.get_logging_level();
        session.set_logging_level(new_level);
        
        session.notify_log(
            turul_mcp_protocol::logging::LoggingLevel::Info, 
            serde_json::json!(format!(
                "🎯 Logging level changed from {:?} to {:?}", 
                old_level, new_level
            )),
            Some("system".to_string()),
            None
        );
        
        session.notify_log(
            turul_mcp_protocol::logging::LoggingLevel::Info, 
            serde_json::json!("Test the change by running the session_logging_demo tool!"),
            Some("system".to_string()),
            None
        );
        
        Ok(json!({
            "success": true,
            "old_level": format!("{:?}", old_level),
            "new_level": format!("{:?}", new_level),
            "note": "This change only affects your session - other sessions are unaffected"
        }))
    }
}

/// Tool to check current session's logging configuration
#[derive(McpTool, Default, Clone)]
#[tool(
    name = "check_logging_status",
    description = "Check the current session's logging level and configuration"
)]
pub struct CheckLoggingStatusTool {
    // This tool takes no parameters but needs a named field for the derive macro
    #[param(description = "Placeholder parameter (not used)")]
    pub _placeholder: Option<String>,
}

impl CheckLoggingStatusTool {
    pub async fn execute(&self, session: Option<SessionContext>) -> McpResult<serde_json::Value> {
        let session = session.ok_or("Session context required")?;
        
        let current_level = session.get_logging_level();
        let session_id = session.session_id.clone();
        
        // Show which message types would be received
        let all_levels = [
            LoggingLevel::Debug,
            LoggingLevel::Info,
            LoggingLevel::Notice,
            LoggingLevel::Warning,
            LoggingLevel::Error,
            LoggingLevel::Critical,
            LoggingLevel::Alert,
            LoggingLevel::Emergency,
        ];
        
        let allowed_levels: Vec<String> = all_levels
            .iter()
            .filter(|level| session.should_log(**level))
            .map(|level| format!("{:?}", level).to_lowercase())
            .collect();
        
        let blocked_levels: Vec<String> = all_levels
            .iter()
            .filter(|level| !session.should_log(**level))
            .map(|level| format!("{:?}", level).to_lowercase())
            .collect();
        
        session.notify_log(str_to_logging_level("info"), serde_json::json!(format!("📊 Session {} logging status checked", session_id)), Some("demo".to_string()), None);
        
        Ok(json!({
            "session_id": session_id,
            "current_level": format!("{:?}", current_level).to_lowercase(),
            "priority": current_level.priority(),
            "allowed_levels": allowed_levels,
            "blocked_levels": blocked_levels,
            "note": "Messages at or above your level will be delivered to your session"
        }))
    }
}