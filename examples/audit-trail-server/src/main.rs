//! # Audit Trail Server Example
//!
//! This example demonstrates a compliance-focused audit trail system using SQLite for persistence.
//! It shows how to log immutable audit events, search audit logs, and generate compliance reports.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;
use std::sync::Arc;
use turul_mcp_protocol::tools::{
    CallToolResult, HasAnnotations, HasBaseMetadata, HasDescription, HasInputSchema,
    HasOutputSchema, HasToolMeta,
};
use turul_mcp_protocol::{McpError, McpResult, ToolResult, ToolSchema, schema::JsonSchema};
use turul_mcp_server::{McpServer, McpTool, SessionContext};
use turul_mcp_session_storage::SqliteSessionStorage;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AuditEvent {
    id: String,
    timestamp: DateTime<Utc>,
    session_id: String,
    event_type: String,
    actor: Option<String>,
    resource: Option<String>,
    action: String,
    result: String,
    metadata: serde_json::Value,
}

/// Initialize SQLite database with audit trail schema
async fn init_database(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS audit_logs (
            id TEXT PRIMARY KEY,
            timestamp TEXT NOT NULL,
            session_id TEXT NOT NULL,
            event_type TEXT NOT NULL,
            actor TEXT,
            resource TEXT,
            action TEXT NOT NULL,
            result TEXT NOT NULL,
            metadata TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_logs(timestamp);
        CREATE INDEX IF NOT EXISTS idx_audit_session ON audit_logs(session_id);
        CREATE INDEX IF NOT EXISTS idx_audit_event_type ON audit_logs(event_type);
        CREATE INDEX IF NOT EXISTS idx_audit_actor ON audit_logs(actor);
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Log an immutable audit event to the database
struct LogAuditEventTool {
    input_schema: ToolSchema,
    db_pool: Arc<SqlitePool>,
}

impl LogAuditEventTool {
    fn new(db_pool: Arc<SqlitePool>) -> Self {
        let input_schema = ToolSchema::object()
            .with_properties(HashMap::from([
                (
                    "event_type".to_string(),
                    JsonSchema::string_enum(vec![
                        "ACCESS".to_string(),
                        "MODIFICATION".to_string(),
                        "DELETION".to_string(),
                        "AUTHENTICATION".to_string(),
                        "AUTHORIZATION".to_string(),
                        "SYSTEM".to_string(),
                    ])
                    .with_description("Type of audit event"),
                ),
                (
                    "action".to_string(),
                    JsonSchema::string().with_description("Specific action performed"),
                ),
                (
                    "result".to_string(),
                    JsonSchema::string_enum(vec![
                        "SUCCESS".to_string(),
                        "FAILURE".to_string(),
                        "PARTIAL".to_string(),
                    ])
                    .with_description("Result of the action"),
                ),
                (
                    "actor".to_string(),
                    JsonSchema::string()
                        .with_description("Optional actor who performed the action"),
                ),
                (
                    "resource".to_string(),
                    JsonSchema::string().with_description("Optional resource that was acted upon"),
                ),
                (
                    "metadata".to_string(),
                    JsonSchema::object().with_description("Optional additional metadata"),
                ),
            ]))
            .with_required(vec![
                "event_type".to_string(),
                "action".to_string(),
                "result".to_string(),
            ]);
        Self {
            input_schema,
            db_pool,
        }
    }
}

impl HasBaseMetadata for LogAuditEventTool {
    fn name(&self) -> &str {
        "log_audit_event"
    }
}

impl HasDescription for LogAuditEventTool {
    fn description(&self) -> Option<&str> {
        Some("Log an immutable audit event for compliance tracking")
    }
}

impl HasInputSchema for LogAuditEventTool {
    fn input_schema(&self) -> &ToolSchema {
        &self.input_schema
    }
}

impl HasOutputSchema for LogAuditEventTool {
    fn output_schema(&self) -> Option<&ToolSchema> {
        None
    }
}

impl HasAnnotations for LogAuditEventTool {
    fn annotations(&self) -> Option<&turul_mcp_protocol::tools::ToolAnnotations> {
        None
    }
}

impl HasToolMeta for LogAuditEventTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
        None
    }
}

#[async_trait]
impl McpTool for LogAuditEventTool {
    async fn call(
        &self,
        args: Value,
        session: Option<SessionContext>,
    ) -> McpResult<CallToolResult> {
        let session =
            session.ok_or_else(|| McpError::SessionError("Session required".to_string()))?;

        let event_type = args
            .get("event_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("event_type"))?;
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("action"))?;
        let result = args
            .get("result")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("result"))?;
        let actor = args.get("actor").and_then(|v| v.as_str());
        let resource = args.get("resource").and_then(|v| v.as_str());
        let metadata = args.get("metadata").cloned().unwrap_or(json!({}));

        // Create audit event
        let audit_event = AuditEvent {
            id: Uuid::now_v7().to_string(),
            timestamp: Utc::now(),
            session_id: session.session_id.clone(),
            event_type: event_type.to_string(),
            actor: actor.map(|s| s.to_string()),
            resource: resource.map(|s| s.to_string()),
            action: action.to_string(),
            result: result.to_string(),
            metadata,
        };

        // Store in database (immutable)
        let result = sqlx::query(
            r#"
            INSERT INTO audit_logs (id, timestamp, session_id, event_type, actor, resource, action, result, metadata)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&audit_event.id)
        .bind(audit_event.timestamp.to_rfc3339())
        .bind(&audit_event.session_id)
        .bind(&audit_event.event_type)
        .bind(&audit_event.actor)
        .bind(&audit_event.resource)
        .bind(&audit_event.action)
        .bind(&audit_event.result)
        .bind(serde_json::to_string(&audit_event.metadata).unwrap_or_default())
        .execute(&*self.db_pool)
        .await;

        match result {
            Ok(_) => {
                // Send progress notification
                session
                    .notify_progress(format!("audit_{}", event_type.to_lowercase()), 1)
                    .await;

                Ok(CallToolResult {
                    content: vec![ToolResult::text(
                        json!({
                            "logged": true,
                            "audit_id": audit_event.id,
                            "timestamp": audit_event.timestamp,
                            "event_type": audit_event.event_type,
                            "action": audit_event.action,
                            "result": audit_event.result,
                            "immutable": true,
                            "compliance": "LOGGED"
                        })
                        .to_string(),
                    )],
                    is_error: None,
                    structured_content: None,
                    meta: None,
                })
            }
            Err(e) => Err(McpError::tool_execution(&format!(
                "Failed to log audit event: {}",
                e
            ))),
        }
    }
}

/// Search audit trail with filters and pagination
struct SearchAuditTrailTool {
    input_schema: ToolSchema,
    db_pool: Arc<SqlitePool>,
}

impl SearchAuditTrailTool {
    fn new(db_pool: Arc<SqlitePool>) -> Self {
        let input_schema = ToolSchema::object().with_properties(HashMap::from([
            (
                "start_time".to_string(),
                JsonSchema::string().with_description("Start time filter (ISO 8601)"),
            ),
            (
                "end_time".to_string(),
                JsonSchema::string().with_description("End time filter (ISO 8601)"),
            ),
            (
                "event_type".to_string(),
                JsonSchema::string().with_description("Filter by event type"),
            ),
            (
                "actor".to_string(),
                JsonSchema::string().with_description("Filter by actor"),
            ),
            (
                "resource".to_string(),
                JsonSchema::string().with_description("Filter by resource"),
            ),
            (
                "limit".to_string(),
                JsonSchema::integer().with_description("Maximum results (default: 50, max: 1000)"),
            ),
            (
                "offset".to_string(),
                JsonSchema::integer()
                    .with_description("Results offset for pagination (default: 0)"),
            ),
        ]));
        Self {
            input_schema,
            db_pool,
        }
    }
}

impl HasBaseMetadata for SearchAuditTrailTool {
    fn name(&self) -> &str {
        "search_audit_trail"
    }
}

impl HasDescription for SearchAuditTrailTool {
    fn description(&self) -> Option<&str> {
        Some("Search audit trail with filters and pagination")
    }
}

impl HasInputSchema for SearchAuditTrailTool {
    fn input_schema(&self) -> &ToolSchema {
        &self.input_schema
    }
}

impl HasOutputSchema for SearchAuditTrailTool {
    fn output_schema(&self) -> Option<&ToolSchema> {
        None
    }
}

impl HasAnnotations for SearchAuditTrailTool {
    fn annotations(&self) -> Option<&turul_mcp_protocol::tools::ToolAnnotations> {
        None
    }
}

impl HasToolMeta for SearchAuditTrailTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
        None
    }
}

#[async_trait]
impl McpTool for SearchAuditTrailTool {
    async fn call(
        &self,
        args: Value,
        _session: Option<SessionContext>,
    ) -> McpResult<CallToolResult> {
        let limit = args
            .get("limit")
            .and_then(|v| v.as_i64())
            .unwrap_or(50)
            .min(1000) as i32;
        let offset = args.get("offset").and_then(|v| v.as_i64()).unwrap_or(0) as i32;

        // Build dynamic query
        let mut query = "SELECT * FROM audit_logs WHERE 1=1".to_string();
        let mut conditions = Vec::new();

        if let Some(start_time) = args.get("start_time").and_then(|v| v.as_str()) {
            conditions.push(format!(" AND timestamp >= '{}'", start_time));
        }
        if let Some(end_time) = args.get("end_time").and_then(|v| v.as_str()) {
            conditions.push(format!(" AND timestamp <= '{}'", end_time));
        }
        if let Some(event_type) = args.get("event_type").and_then(|v| v.as_str()) {
            conditions.push(format!(" AND event_type = '{}'", event_type));
        }
        if let Some(actor) = args.get("actor").and_then(|v| v.as_str()) {
            conditions.push(format!(" AND actor = '{}'", actor));
        }
        if let Some(resource) = args.get("resource").and_then(|v| v.as_str()) {
            conditions.push(format!(" AND resource = '{}'", resource));
        }

        for condition in conditions {
            query.push_str(&condition);
        }
        query.push_str(" ORDER BY timestamp DESC LIMIT ? OFFSET ?");

        let rows = sqlx::query(&query)
            .bind(limit)
            .bind(offset)
            .fetch_all(&*self.db_pool)
            .await
            .map_err(|e| McpError::tool_execution(&format!("Database query failed: {}", e)))?;

        let audit_events: Vec<Value> = rows
            .iter()
            .map(|row| {
                let timestamp_str: String = row.get("timestamp");
                let metadata_str: String = row.get("metadata");

                json!({
                    "id": row.get::<String, _>("id"),
                    "timestamp": timestamp_str,
                    "session_id": row.get::<String, _>("session_id"),
                    "event_type": row.get::<String, _>("event_type"),
                    "actor": row.get::<Option<String>, _>("actor"),
                    "resource": row.get::<Option<String>, _>("resource"),
                    "action": row.get::<String, _>("action"),
                    "result": row.get::<String, _>("result"),
                    "metadata": serde_json::from_str::<Value>(&metadata_str).unwrap_or(json!({}))
                })
            })
            .collect();

        // Get total count for pagination
        let count_query = "SELECT COUNT(*) as total FROM audit_logs WHERE 1=1".to_string();
        let total_row = sqlx::query(&count_query)
            .fetch_one(&*self.db_pool)
            .await
            .map_err(|e| McpError::tool_execution(&format!("Count query failed: {}", e)))?;
        let total: i32 = total_row.get("total");

        let result = json!({
            "results": audit_events,
            "pagination": {
                "total": total,
                "limit": limit,
                "offset": offset,
                "has_more": offset + limit < total
            },
            "search_criteria": args
        });

        Ok(CallToolResult {
            content: vec![ToolResult::text(result.to_string())],
            is_error: None,
            structured_content: None,
            meta: None,
        })
    }
}

/// Generate compliance report with audit statistics
struct GenerateComplianceReportTool {
    input_schema: ToolSchema,
    db_pool: Arc<SqlitePool>,
}

impl GenerateComplianceReportTool {
    fn new(db_pool: Arc<SqlitePool>) -> Self {
        let input_schema = ToolSchema::object()
            .with_properties(HashMap::from([
                (
                    "report_type".to_string(),
                    JsonSchema::string_enum(vec![
                        "SUMMARY".to_string(),
                        "DETAILED".to_string(),
                        "COMPLIANCE".to_string(),
                    ])
                    .with_description("Type of compliance report"),
                ),
                (
                    "start_date".to_string(),
                    JsonSchema::string().with_description("Start date for report (ISO 8601)"),
                ),
                (
                    "end_date".to_string(),
                    JsonSchema::string().with_description("End date for report (ISO 8601)"),
                ),
            ]))
            .with_required(vec!["report_type".to_string()]);
        Self {
            input_schema,
            db_pool,
        }
    }
}

impl HasBaseMetadata for GenerateComplianceReportTool {
    fn name(&self) -> &str {
        "generate_compliance_report"
    }
}

impl HasDescription for GenerateComplianceReportTool {
    fn description(&self) -> Option<&str> {
        Some("Generate compliance report with audit trail statistics")
    }
}

impl HasInputSchema for GenerateComplianceReportTool {
    fn input_schema(&self) -> &ToolSchema {
        &self.input_schema
    }
}

impl HasOutputSchema for GenerateComplianceReportTool {
    fn output_schema(&self) -> Option<&ToolSchema> {
        None
    }
}

impl HasAnnotations for GenerateComplianceReportTool {
    fn annotations(&self) -> Option<&turul_mcp_protocol::tools::ToolAnnotations> {
        None
    }
}

impl HasToolMeta for GenerateComplianceReportTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
        None
    }
}

#[async_trait]
impl McpTool for GenerateComplianceReportTool {
    async fn call(
        &self,
        args: Value,
        _session: Option<SessionContext>,
    ) -> McpResult<CallToolResult> {
        let report_type = args
            .get("report_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("report_type"))?;

        // Build time filter
        let mut time_filter = String::new();
        if let Some(start_date) = args.get("start_date").and_then(|v| v.as_str()) {
            time_filter.push_str(&format!(" AND timestamp >= '{}'", start_date));
        }
        if let Some(end_date) = args.get("end_date").and_then(|v| v.as_str()) {
            time_filter.push_str(&format!(" AND timestamp <= '{}'", end_date));
        }

        // Get total event counts by type
        let event_counts_query = format!(
            "SELECT event_type, COUNT(*) as count FROM audit_logs WHERE 1=1 {} GROUP BY event_type",
            time_filter
        );
        let event_rows = sqlx::query(&event_counts_query)
            .fetch_all(&*self.db_pool)
            .await
            .map_err(|e| McpError::tool_execution(&format!("Event counts query failed: {}", e)))?;

        let event_counts: HashMap<String, i32> = event_rows
            .iter()
            .map(|row| {
                (
                    row.get::<String, _>("event_type"),
                    row.get::<i32, _>("count"),
                )
            })
            .collect();

        // Get result counts
        let result_counts_query = format!(
            "SELECT result, COUNT(*) as count FROM audit_logs WHERE 1=1 {} GROUP BY result",
            time_filter
        );
        let result_rows = sqlx::query(&result_counts_query)
            .fetch_all(&*self.db_pool)
            .await
            .map_err(|e| McpError::tool_execution(&format!("Result counts query failed: {}", e)))?;

        let result_counts: HashMap<String, i32> = result_rows
            .iter()
            .map(|row| (row.get::<String, _>("result"), row.get::<i32, _>("count")))
            .collect();

        // Get session statistics
        let session_stats_query = format!(
            "SELECT COUNT(DISTINCT session_id) as unique_sessions, COUNT(*) as total_events FROM audit_logs WHERE 1=1 {}",
            time_filter
        );
        let stats_row = sqlx::query(&session_stats_query)
            .fetch_one(&*self.db_pool)
            .await
            .map_err(|e| McpError::tool_execution(&format!("Session stats query failed: {}", e)))?;

        let unique_sessions: i32 = stats_row.get("unique_sessions");
        let total_events: i32 = stats_row.get("total_events");

        let report = match report_type {
            "SUMMARY" => json!({
                "report_type": "SUMMARY",
                "generated_at": Utc::now(),
                "period": {
                    "start_date": args.get("start_date"),
                    "end_date": args.get("end_date")
                },
                "summary": {
                    "total_events": total_events,
                    "unique_sessions": unique_sessions,
                    "events_per_session": if unique_sessions > 0 { total_events as f64 / unique_sessions as f64 } else { 0.0 },
                    "success_rate": *result_counts.get("SUCCESS").unwrap_or(&0) as f64 / total_events.max(1) as f64 * 100.0
                }
            }),
            "DETAILED" => json!({
                "report_type": "DETAILED",
                "generated_at": Utc::now(),
                "period": {
                    "start_date": args.get("start_date"),
                    "end_date": args.get("end_date")
                },
                "statistics": {
                    "total_events": total_events,
                    "unique_sessions": unique_sessions,
                    "event_types": event_counts,
                    "results": result_counts
                }
            }),
            "COMPLIANCE" => json!({
                "report_type": "COMPLIANCE",
                "generated_at": Utc::now(),
                "period": {
                    "start_date": args.get("start_date"),
                    "end_date": args.get("end_date")
                },
                "compliance_status": {
                    "audit_logging_enabled": true,
                    "immutable_records": true,
                    "retention_policy": "Indefinite",
                    "total_audit_events": total_events,
                    "failed_operations_logged": result_counts.contains_key("FAILURE"),
                    "success_rate_percent": *result_counts.get("SUCCESS").unwrap_or(&0) as f64 / total_events.max(1) as f64 * 100.0
                },
                "recommendations": if result_counts.get("FAILURE").unwrap_or(&0) > &0 {
                    vec!["Review failed operations for security concerns"]
                } else {
                    vec!["Audit trail is healthy"]
                }
            }),
            _ => {
                return Err(McpError::invalid_param_type(
                    "report_type",
                    "SUMMARY|DETAILED|COMPLIANCE",
                    report_type,
                ));
            }
        };

        Ok(CallToolResult {
            content: vec![ToolResult::text(report.to_string())],
            is_error: None,
            structured_content: None,
            meta: None,
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("Starting Audit Trail MCP Server with SQLite persistence");

    // Initialize SQLite database
    let db_url = "sqlite:audit_trail.db";
    let pool = SqlitePool::connect(db_url).await?;
    init_database(&pool).await?;
    let db_pool = Arc::new(pool);

    // Create SQLite session storage
    let _session_storage = Arc::new(SqliteSessionStorage::new().await?);

    let server = McpServer::builder()
        .name("audit-trail-server")
        .version("1.0.0")
        .title("Audit Trail Server")
        .instructions(
            "This server provides compliance-focused audit trail logging with SQLite persistence.",
        )
        .tool(LogAuditEventTool::new(db_pool.clone()))
        .tool(SearchAuditTrailTool::new(db_pool.clone()))
        .tool(GenerateComplianceReportTool::new(db_pool.clone()))
        .bind_address("127.0.0.1:8009".parse()?)
        .bind_address("127.0.0.1:8009".parse()?)
        .sse(true)
        .build()?;

    println!("Audit trail server running at: http://127.0.0.1:8009/mcp");
    println!("SQLite database: audit_trail.db");
    println!("\nAvailable tools:");
    println!("  - log_audit_event: Log immutable compliance audit events");
    println!("  - search_audit_trail: Search audit logs with filters");
    println!("  - generate_compliance_report: Generate compliance reports");
    println!("\nExample usage:");
    println!(
        "  1. log_audit_event(event_type='ACCESS', action='login', result='SUCCESS', actor='user123')"
    );
    println!("  2. search_audit_trail(event_type='ACCESS', limit=10)");
    println!("  3. generate_compliance_report(report_type='SUMMARY')");

    server.run().await?;
    Ok(())
}
