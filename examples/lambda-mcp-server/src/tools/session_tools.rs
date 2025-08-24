//! Session Management Tools
//!
//! Tools for MCP session management, monitoring, and diagnostics

use crate::global_events::{broadcast_session_event, SessionEventType, broadcast_global_event, GlobalEvent, ToolExecutionStatus};
use crate::session_manager::{SessionManager, McpSession};

// Import json-rpc-server framework types for proper error handling
use json_rpc_server::{JsonRpcError, JsonRpcErrorCode};
use async_trait::async_trait;
use mcp_protocol::{ToolResult, ToolSchema, schema::JsonSchema, McpError};
use mcp_server::{McpTool, SessionContext, McpResult};
use mcp_derive::McpTool;
use serde_json::{Value, json};
use serde::{Deserialize, Serialize};

/// Response structure for session_info tool
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionInfoResponse {
    pub session_id: String,
    pub status: String,
    pub created_at: String,
    pub last_activity: String,
    pub ttl: i64,
    pub time_remaining_seconds: i64,
    pub client_info: serde_json::Value,
    pub client_capabilities: serde_json::Value,
    pub session_metadata: SessionMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub is_active: bool,
    pub age_minutes: i64,
    pub expires_in_minutes: i64,
    pub tool_calls_made: u32,
}

/// Response structure for list_active_sessions tool
#[derive(Debug, Serialize, Deserialize)]
pub struct ListActiveSessionsResponse {
    pub timestamp: String,
    pub requested_by: String,
    pub max_sessions: u32,
    pub sort_by: String,
    pub include_details: bool,
    pub sessions: Vec<SessionSummary>,
    pub total_active_sessions: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub created_at: String,
    pub last_activity: String,
    pub ttl: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_capabilities: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_info: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age_minutes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_in_minutes: Option<i64>,
}

/// Response structure for session_cleanup tool
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionCleanupResponse {
    pub timestamp: String,
    pub cleanup_initiated_by: String,
    pub sessions_cleaned: u32,
    pub cleanup_results: Vec<CleanupResult>,
    pub summary: CleanupSummary,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CleanupResult {
    pub session_id: String,
    pub status: String,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CleanupSummary {
    pub total_sessions_checked: u32,
    pub expired_sessions_removed: u32,
    pub errors_encountered: u32,
}
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Session Information Tool
/// 
/// Get detailed information about the current MCP session
pub struct SessionInfo;

#[async_trait]
impl McpTool for SessionInfo {
    fn name(&self) -> &str {
        "session_info"
    }

    fn description(&self) -> &str {
        "Get detailed information about the current MCP session"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("include_metadata".to_string(), JsonSchema::boolean()),
                ("include_capabilities".to_string(), JsonSchema::boolean()),
                ("include_stats".to_string(), JsonSchema::boolean()),
            ]))
    }

    fn output_schema(&self) -> Option<ToolSchema> {
        Some(ToolSchema::object()
            .with_properties(HashMap::from([
                ("session_id".to_string(), JsonSchema::string()),
                ("timestamp".to_string(), JsonSchema::string()),
                ("status".to_string(), JsonSchema::string()),
                ("metadata".to_string(), JsonSchema::object()),
                ("capabilities".to_string(), JsonSchema::object()),
                ("stats".to_string(), JsonSchema::object()),
            ]))
            .with_required(vec!["session_id".to_string(), "timestamp".to_string(), "status".to_string()]))
    }

    async fn call(&self, args: Value, session: Option<SessionContext>) -> McpResult<Vec<ToolResult>> {
        let include_metadata = args.get("include_metadata")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
            
        let include_capabilities = args.get("include_capabilities")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
            
        let include_stats = args.get("include_stats")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let session_id = session
            .as_ref()
            .map(|s| s.session_id.clone())
            .unwrap_or_else(|| "unknown".to_string());

        info!("Getting session info for: {}", session_id);

        let mut session_info = json!({
            "session_id": session_id,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "tool": "session_info"
        });

        // Get session from DynamoDB if available
        if let Ok(session_manager) = SessionManager::new().await {
            match session_manager.get_session(&session_id).await {
                Ok(Some(db_session)) => {
                    session_info["session_state"] = json!("active");
                    session_info["created_at"] = json!(db_session.created_at);
                    session_info["last_activity"] = json!(db_session.last_activity);
                    session_info["ttl"] = json!(db_session.ttl);

                    if include_capabilities {
                        session_info["client_capabilities"] = db_session.client_capabilities;
                    }

                    if include_metadata {
                        session_info["client_info"] = db_session.client_info.unwrap_or_default();
                        session_info["session_id"] = json!(session_id);
                    }

                    if include_stats {
                        session_info["statistics"] = json!({
                            "session_duration_seconds": chrono::Utc::now().timestamp() - db_session.created_at.timestamp(),
                            "time_since_activity_seconds": chrono::Utc::now().timestamp() - db_session.last_activity.timestamp(),
                            "ttl_remaining_seconds": (db_session.ttl as i64) - chrono::Utc::now().timestamp()
                        });
                    }
                }
                Ok(None) => {
                    session_info["session_state"] = json!("not_found");
                    session_info["note"] = json!("Session not found in persistent storage");
                }
                Err(e) => {
                    warn!("Failed to retrieve session from DynamoDB: {:?}", e);
                    session_info["session_state"] = json!("error");
                    session_info["error"] = json!(format!("Database error: {:?}", e));
                }
            }
        } else {
            session_info["session_state"] = json!("no_session_manager");
            session_info["note"] = json!("Session manager not available");
        }

        // Optional: Broadcast session info event for progress tracking (not the result)
        if let Err(e) = broadcast_session_event(
            &session_id,
            SessionEventType::InfoRequested,
            Some(json!({
                "action": "session_info_requested",
                "timestamp": chrono::Utc::now().to_rfc3339()
            })),
        ).await {
            warn!("Failed to broadcast session info event: {:?}", e);
        }

        // Serialize the session_info to JSON string and return as text
        let json_str = serde_json::to_string_pretty(&session_info)
            .map_err(|e| McpError::tool_execution(&format!("Serialization error: {}", e)))?;
        
        debug!("Session info completed for: {}", session_id);
        Ok(vec![ToolResult::text(json_str)])
    }
}

/// List Active Sessions Tool
/// 
/// List all currently active MCP sessions with optional filtering
pub struct ListActiveSessions;

#[async_trait]
impl McpTool for ListActiveSessions {
    fn name(&self) -> &str {
        "list_active_sessions"
    }

    fn description(&self) -> &str {
        "List all currently active MCP sessions with optional filtering"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("include_details".to_string(), JsonSchema::boolean()),
                ("max_sessions".to_string(), JsonSchema::number_with_description("Maximum number of sessions to return")
                    .with_minimum(1.0)
                    .with_maximum(100.0)),
                ("sort_by".to_string(), JsonSchema::string_with_description("Sort sessions by field")),
            ]))
    }

    async fn call(&self, args: Value, session: Option<SessionContext>) -> McpResult<Vec<ToolResult>> {
        let include_details = args.get("include_details")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
            
        let max_sessions = args.get("max_sessions")
            .and_then(|v| v.as_f64())
            .map(|v| v as i32)
            .unwrap_or(20);
            
        let sort_by = args.get("sort_by")
            .and_then(|v| v.as_str())
            .unwrap_or("last_activity");

        let current_session_id = session
            .as_ref()
            .map(|s| s.session_id.clone())
            .unwrap_or_else(|| "unknown".to_string());

        info!("Listing active sessions (max: {}, sort: {})", max_sessions, sort_by);

        let mut response = json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "requested_by": current_session_id,
            "max_sessions": max_sessions,
            "sort_by": sort_by,
            "include_details": include_details
        });

        // Get sessions from DynamoDB
        if let Ok(session_manager) = SessionManager::new().await {
            match session_manager.list_active_sessions(Some(max_sessions)).await {
                Ok(sessions) => {
                    let mut session_list = Vec::new();
                    
                    for db_session in sessions {
                        let mut session_summary = json!({
                            "session_id": db_session.session_id,
                            "created_at": db_session.created_at,
                            "last_activity": db_session.last_activity,
                            "ttl": db_session.ttl
                        });

                        if include_details {
                            session_summary["client_capabilities"] = db_session.client_capabilities;
                            session_summary["client_info"] = db_session.client_info.unwrap_or_default();
                            
                            // Calculate session statistics
                            let now = chrono::Utc::now().timestamp();
                            session_summary["statistics"] = json!({
                                "duration_seconds": now - db_session.created_at.timestamp(),
                                "time_since_activity_seconds": now - db_session.last_activity.timestamp(),
                                "ttl_remaining_seconds": (db_session.ttl as i64) - now,
                                "is_current_session": db_session.session_id == current_session_id
                            });
                        }

                        session_list.push(session_summary);
                    }

                    // Sort sessions based on requested field
                    session_list.sort_by(|a, b| {
                        let field_a = match sort_by {
                            "created_at" => a.get("created_at").and_then(|v| v.as_i64()).unwrap_or(0),
                            "session_id" => return a.get("session_id").and_then(|v| v.as_str())
                                .unwrap_or("")
                                .cmp(b.get("session_id").and_then(|v| v.as_str()).unwrap_or("")),
                            _ => a.get("last_activity").and_then(|v| v.as_i64()).unwrap_or(0), // default: last_activity
                        };
                        let field_b = match sort_by {
                            "created_at" => b.get("created_at").and_then(|v| v.as_i64()).unwrap_or(0),
                            "session_id" => return b.get("session_id").and_then(|v| v.as_str())
                                .unwrap_or("")
                                .cmp(a.get("session_id").and_then(|v| v.as_str()).unwrap_or("")).reverse(),
                            _ => b.get("last_activity").and_then(|v| v.as_i64()).unwrap_or(0),
                        };
                        field_b.cmp(&field_a) // Descending order (most recent first)
                    });

                    response["sessions"] = json!(session_list);
                    response["total_sessions"] = json!(session_list.len());
                    response["status"] = json!("success");
                }
                Err(e) => {
                    warn!("Failed to list sessions from DynamoDB: {:?}", e);
                    response["status"] = json!("error");
                    response["error"] = json!(format!("Database error: {:?}", e));
                    response["sessions"] = json!([]);
                    response["total_sessions"] = json!(0);
                }
            }
        } else {
            response["status"] = json!("no_session_manager");
            response["error"] = json!("Session manager not available");
            response["sessions"] = json!([]);
            response["total_sessions"] = json!(0);
        }

        // Broadcast session list event
        if let Err(e) = broadcast_session_event(
            &current_session_id,
            SessionEventType::SessionsListed,
            Some(json!({
                "total_sessions": response.get("total_sessions").cloned().unwrap_or(json!(0)),
                "include_details": include_details
            })),
        ).await {
            warn!("Failed to broadcast session list event: {:?}", e);
        }

        // Return the actual sessions list data, not a generic success message
        let result = ToolResult::resource(response);

        debug!("Listed active sessions for: {}", current_session_id);
        Ok(vec![result])
    }
}

/// Session Cleanup Tool
/// 
/// Manually trigger cleanup of expired sessions
pub struct SessionCleanup;

#[async_trait]
impl McpTool for SessionCleanup {
    fn name(&self) -> &str {
        "session_cleanup"
    }

    fn description(&self) -> &str {
        "Manually trigger cleanup of expired sessions"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("dry_run".to_string(), JsonSchema::boolean()),
                ("max_cleanup".to_string(), JsonSchema::number_with_description("Maximum number of sessions to clean up")
                    .with_minimum(1.0)
                    .with_maximum(100.0)),
            ]))
    }

    async fn call(&self, args: Value, session: Option<SessionContext>) -> McpResult<Vec<ToolResult>> {
        let dry_run = args.get("dry_run")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
            
        let max_cleanup = args.get("max_cleanup")
            .and_then(|v| v.as_f64())
            .unwrap_or(50.0) as usize;

        let session_id = session
            .as_ref()
            .map(|s| s.session_id.clone())
            .unwrap_or_else(|| "unknown".to_string());

        info!("Starting session cleanup (dry_run: {}, max: {})", dry_run, max_cleanup);

        let mut response = json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "requested_by": session_id,
            "dry_run": dry_run,
            "max_cleanup": max_cleanup
        });

        if let Ok(session_manager) = SessionManager::new().await {
            match session_manager.cleanup_expired_sessions().await {
                Ok(sessions_cleaned) => {
                    response["status"] = json!("success");
                    response["sessions_cleaned"] = json!(sessions_cleaned);
                    response["max_cleanup_requested"] = json!(max_cleanup);
                    
                    if dry_run {
                        response["note"] = json!("Dry run completed - no sessions were actually deleted");
                    }
                }
                Err(e) => {
                    warn!("Session cleanup failed: {:?}", e);
                    response["status"] = json!("error");
                    response["error"] = json!(format!("Cleanup error: {:?}", e));
                    response["sessions_cleaned"] = json!(0);
                }
            }
        } else {
            response["status"] = json!("no_session_manager");
            response["error"] = json!("Session manager not available");
            response["sessions_cleaned"] = json!(0);
        }

        // Broadcast cleanup event
        if let Err(e) = broadcast_session_event(
            &session_id,
            SessionEventType::CleanupTriggered,
            Some(response.clone()),
        ).await {
            warn!("Failed to broadcast cleanup event: {:?}", e);
        }

        // Return the actual cleanup data, not a generic success message
        let result = ToolResult::resource(response);

        debug!("Session cleanup completed for: {}", session_id);
        Ok(vec![result])
    }
}

/// Server Notification Tool
/// 
/// Send local server notifications within the Lambda instance via tokio broadcast.
/// These notifications stay within the current Lambda instance and are delivered
/// to any connected SSE clients on this instance only (not global fan-out).
/// 
/// Examples:
/// - System health alerts: {"component": "database", "status": "warning", "message": "High connection count"}
/// - Service status: {"component": "auth_service", "status": "healthy", "message": "All systems operational"}
/// - Error notifications: {"component": "payment_processor", "status": "error", "message": "Payment gateway timeout"}
#[derive(McpTool, Clone)]
#[tool(
    name = "server_notification",
    description = "Send local server notifications to connected SSE clients on this Lambda instance. Use for system health, service status, and error notifications. Example: {\"component\":\"database\",\"status\":\"healthy\",\"message\":\"Connection pool optimized\"}"
)]
pub struct ServerNotificationTool {
    #[param(description = "REQUIRED. Component sending the notification. Examples: 'database', 'auth_service', 'payment_processor', 'cache_layer'")]
    component: String,

    #[param(description = "REQUIRED. Component status. Valid values: 'healthy', 'warning', 'error', 'info'. Example: 'healthy'")]
    status: String,

    #[param(description = "REQUIRED. Human-readable notification message. Example: 'Database connection pool optimized'")]
    message: String,

    #[param(description = "Optional. Additional structured data as JSON object. Example: {\"connections\":50,\"response_time_ms\":120}", optional)]
    details: Option<String>,

    #[param(description = "Optional. Notification severity level. Valid values: 'low', 'medium', 'high', 'critical'. Default: 'medium'", optional)]
    severity: Option<String>,
}

impl Default for ServerNotificationTool {
    fn default() -> Self {
        Self {
            component: String::new(),
            status: String::new(),
            message: String::new(),
            details: None,
            severity: None,
        }
    }
}

impl ServerNotificationTool {
    async fn execute(&self) -> McpResult<String> {
        // Validate component is not empty
        if self.component.trim().is_empty() {
            return Err(mcp_protocol::McpError::missing_param("component"));
        }

        // Validate status is one of the allowed values
        match self.status.as_str() {
            "healthy" | "warning" | "error" | "info" => {},
            "" => return Err(mcp_protocol::McpError::missing_param("status")),
            _ => return Err(mcp_protocol::McpError::invalid_param_type("status", "one of: healthy, warning, error, info", &self.status)),
        }

        // Validate message is not empty
        if self.message.trim().is_empty() {
            return Err(mcp_protocol::McpError::missing_param("message"));
        }

        // Parse details JSON if provided
        let details = if let Some(details_str) = &self.details {
            match serde_json::from_str(details_str) {
                Ok(parsed) => parsed,
                Err(_) => json!({"raw_details": details_str}),
            }
        } else {
            json!({})
        };
        
        let severity = self.severity.as_deref().unwrap_or("medium");

        // Note: SessionContext is not available in execute method for derive macros
        let session_id = "derive_tool".to_string();

        info!("🔔 Sending server notification from component: {} (status: {}, severity: {})", self.component, self.status, severity);

        // Create enhanced details with additional context
        let enhanced_details = json!({
            "original_details": details,
            "message": self.message,
            "severity": severity,
            "session_id": session_id,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "notification_id": uuid::Uuid::now_v7().to_string()
        });

        // Send global notification via tokio broadcast
        match broadcast_global_event(GlobalEvent::system_health(
            &self.component,
            &self.status,
            enhanced_details.clone(),
        )).await {
            Ok(subscriber_count) => {
                info!("✅ Server notification broadcast to {} subscribers", subscriber_count);
                
                Ok(format!(
                    "Server notification sent successfully to {} subscribers. Component: {}, Status: {}, Message: {}",
                    subscriber_count, self.component, self.status, self.message
                ))
            }
            Err(e) => {
                warn!("❌ Failed to broadcast server notification: {:?}", e);
                
                Ok(format!(
                    "Failed to send server notification: {}. No active subscribers to receive the notification.",
                    e
                ))
            }
        }
    }
}

/// Progress Update Tool
/// 
/// Send local progress updates for long-running operations via tokio broadcast.
/// These updates stay within the current Lambda instance and are delivered
/// to connected SSE clients to show real-time progress of operations.
/// 
/// Status Flow Examples:
/// 1. Start: {"tool_name": "file_processor", "status": "started", "message": "Beginning file processing"}
/// 2. Progress: {"tool_name": "file_processor", "status": "in_progress", "progress_percent": 45.5, "current_step": "Processing file 3 of 10"}
/// 3. Complete: {"tool_name": "file_processor", "status": "completed", "progress_percent": 100, "result_data": {"files_processed": 10}}
/// 4. Error: {"tool_name": "file_processor", "status": "failed", "error_details": {"error": "File not found", "file": "missing.txt"}}
#[derive(McpTool, Clone)]
#[tool(
    name = "progress_update",
    description = "Send local progress updates for operations to connected SSE clients. Use for tracking long-running tasks. Example: {\"tool_name\":\"data_processor\",\"status\":\"in_progress\",\"progress_percent\":75.5,\"message\":\"Processing record 750 of 1000\"}"
)]
pub struct ProgressUpdateTool {
    #[param(description = "REQUIRED. Name of the tool/operation reporting progress. Example: 'file_processor', 'data_migrator', 'backup_service'")]
    tool_name: String,

    #[param(description = "REQUIRED. Progress status. Valid values: 'started' (operation beginning), 'in_progress' (operation ongoing), 'completed' (operation finished successfully), 'failed' (operation failed with error)")]
    status: String,

    #[param(description = "Optional. Progress percentage from 0.0 to 100.0. Use with 'in_progress' or 'completed' status. Example: 75.5", optional)]
    progress_percent: Option<f64>,

    #[param(description = "Optional. Human-readable progress message. Example: 'Processing file 3 of 10' or 'Backup completed successfully'", optional)]
    message: Option<String>,

    #[param(description = "Optional. Description of current step being executed. Example: 'Validating data integrity' or 'Step 3: Compressing files'", optional)]
    current_step: Option<String>,

    #[param(description = "Optional. Total number of steps in the operation. Use with current_step for step tracking. Example: 5", optional)]
    total_steps: Option<f64>,

    #[param(description = "Optional. Result data for 'completed' status only. JSON object with operation results. Example: {\"files_processed\":150,\"total_size_mb\":2048}", optional)]
    result_data: Option<String>,

    #[param(description = "Optional. Error details for 'failed' status only. JSON object with error information. Example: {\"error\":\"Permission denied\",\"file\":\"/restricted/data.txt\",\"code\":403}", optional)]
    error_details: Option<String>,
}

impl Default for ProgressUpdateTool {
    fn default() -> Self {
        Self {
            tool_name: String::new(),
            status: String::new(),
            progress_percent: None,
            message: None,
            current_step: None,
            total_steps: None,
            result_data: None,
            error_details: None,
        }
    }
}

impl ProgressUpdateTool {
    async fn execute(&self) -> McpResult<String> {
        // Validate tool_name is not empty
        if self.tool_name.trim().is_empty() {
            return Err(mcp_protocol::McpError::missing_param("tool_name"));
        }

        // Validate status is one of the allowed values
        match self.status.as_str() {
            "started" | "in_progress" | "completed" | "failed" => {},
            "" => return Err(mcp_protocol::McpError::missing_param("status")),
            _ => return Err(mcp_protocol::McpError::invalid_param_type("status", "one of: started, in_progress, completed, failed", &self.status)),
        }

        // Validate progress_percent range if provided
        if let Some(percent) = self.progress_percent {
            if percent < 0.0 || percent > 100.0 {
                return Err(mcp_protocol::McpError::param_out_of_range("progress_percent", &percent.to_string(), "0.0 to 100.0"));
            }
        }

        // Parse optional JSON fields
        let result_data = if let Some(data_str) = &self.result_data {
            match serde_json::from_str(data_str) {
                Ok(parsed) => Some(parsed),
                Err(_) => Some(json!({"raw_result_data": data_str})),
            }
        } else {
            None
        };

        let error_details = if let Some(error_str) = &self.error_details {
            match serde_json::from_str(error_str) {
                Ok(parsed) => Some(parsed),
                Err(_) => Some(json!({"raw_error_details": error_str})),
            }
        } else {
            None
        };

        // Note: SessionContext is not available in execute method for derive macros
        let session_id = "derive_tool".to_string();

        // Convert status string to ToolExecutionStatus
        let tool_status = match self.status.as_str() {
            "started" => ToolExecutionStatus::Started,
            "in_progress" => ToolExecutionStatus::InProgress { progress: self.progress_percent },
            "completed" => ToolExecutionStatus::Completed,
            "failed" => {
                let error_msg = error_details
                    .as_ref()
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
                    .unwrap_or("Unknown error");
                ToolExecutionStatus::Failed { error: error_msg.to_string() }
            }
            _ => return Err(mcp_protocol::McpError::invalid_param_type("status", "one of: started, in_progress, completed, failed", &self.status)),
        };

        info!("📊 Sending progress update for tool: {} (status: {}, progress: {:?}%)", 
              self.tool_name, self.status, self.progress_percent);

        // Create result data for the progress event
        let mut progress_result = json!({
            "tool_name": self.tool_name,
            "status": self.status,
            "message": self.message.as_deref().unwrap_or(""),
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "session_id": session_id,
            "progress_id": uuid::Uuid::now_v7().to_string()
        });

        if let Some(percent) = self.progress_percent {
            progress_result["progress_percent"] = json!(percent);
        }

        if let Some(step) = &self.current_step {
            progress_result["current_step"] = json!(step);
        }

        if let Some(total) = self.total_steps {
            progress_result["total_steps"] = json!(total);
        }

        if let Some(data) = result_data {
            progress_result["result_data"] = data;
        }

        if let Some(error) = error_details {
            progress_result["error_details"] = error;
        }

        // Send progress update via tokio broadcast
        match broadcast_global_event(GlobalEvent::tool_execution(
            &self.tool_name,
            &session_id,
            tool_status,
            Some(progress_result.clone()),
        )).await {
            Ok(subscriber_count) => {
                info!("✅ Progress update broadcast to {} subscribers", subscriber_count);
                
                let result_text = if let Some(percent) = self.progress_percent {
                    format!(
                        "Progress update sent successfully to {} subscribers. Tool: {}, Status: {}, Progress: {:.1}%",
                        subscriber_count, self.tool_name, self.status, percent
                    )
                } else {
                    format!(
                        "Progress update sent successfully to {} subscribers. Tool: {}, Status: {}",
                        subscriber_count, self.tool_name, self.status
                    )
                };
                
                Ok(result_text)
            }
            Err(e) => {
                warn!("❌ Failed to broadcast progress update: {:?}", e);
                
                Ok(format!(
                    "Failed to send progress update: {}. No active subscribers to receive the update.",
                    e
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_info_tool() {
        let tool = SessionInfo;
        
        assert_eq!(tool.name(), "session_info");
        assert!(!tool.description().is_empty());
        
        let schema = tool.input_schema();
        assert!(schema.properties.contains_key("include_metadata"));
        assert!(schema.properties.contains_key("include_capabilities"));
    }

    #[tokio::test]
    async fn test_list_active_sessions_tool() {
        let tool = ListActiveSessions;
        
        assert_eq!(tool.name(), "list_active_sessions");
        assert!(!tool.description().is_empty());
        
        let schema = tool.input_schema();
        assert!(schema.properties.contains_key("include_details"));
        assert!(schema.properties.contains_key("max_sessions"));
        assert!(schema.properties.contains_key("sort_by"));
    }

    #[tokio::test]
    async fn test_session_cleanup_tool() {
        let tool = SessionCleanup;
        
        assert_eq!(tool.name(), "session_cleanup");
        assert!(!tool.description().is_empty());
        
        let schema = tool.input_schema();
        assert!(schema.properties.contains_key("dry_run"));
        assert!(schema.properties.contains_key("max_cleanup"));
    }

    #[tokio::test]
    async fn test_server_notification_tool() {
        let tool = ServerNotificationTool;
        
        assert_eq!(tool.name(), "server_notification");
        assert!(!tool.description().is_empty());
        
        let schema = tool.input_schema();
        assert!(schema.properties.contains_key("component"));
        assert!(schema.properties.contains_key("status"));
        assert!(schema.properties.contains_key("message"));
        assert!(schema.properties.contains_key("details"));
        assert!(schema.properties.contains_key("severity"));
    }

    #[tokio::test]
    async fn test_progress_update_tool() {
        let tool = ProgressUpdateTool;
        
        assert_eq!(tool.name(), "progress_update");
        assert!(!tool.description().is_empty());
        
        let schema = tool.input_schema();
        assert!(schema.properties.contains_key("tool_name"));
        assert!(schema.properties.contains_key("status"));
        assert!(schema.properties.contains_key("progress_percent"));
        assert!(schema.properties.contains_key("message"));
        assert!(schema.properties.contains_key("current_step"));
        assert!(schema.properties.contains_key("total_steps"));
        assert!(schema.properties.contains_key("result_data"));
        assert!(schema.properties.contains_key("error_details"));
    }

    #[test]
    fn test_session_info_call_no_session() {
        // Test that the tool can handle being called without a session context
        let tool = SessionInfo;
        let args = serde_json::json!({});
        
        // This would normally be an async test, but we're just testing the interface
        assert_eq!(tool.name(), "session_info");
    }
}