//! # Audit Trail Server Example
//!
//! This example demonstrates a compliance-focused audit trail system using SQLite for persistence.
//! It shows how to log immutable audit events, search audit logs, and generate compliance reports.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use turul_mcp_derive::McpTool;
use turul_mcp_protocol::{McpError, McpResult};
use turul_mcp_server::{McpServer, SessionContext};
use turul_mcp_session_storage::SqliteSessionStorage;
use uuid::Uuid;

/// Global database pool shared by all tools
static DB_POOL: OnceLock<Arc<SqlitePool>> = OnceLock::new();

fn get_db_pool() -> McpResult<&'static Arc<SqlitePool>> {
    DB_POOL
        .get()
        .ok_or_else(|| McpError::tool_execution("Database pool not initialized"))
}

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
#[derive(McpTool, Clone, Default, Deserialize)]
#[tool(
    name = "log_audit_event",
    description = "Log an immutable audit event for compliance tracking"
)]
pub struct LogAuditEventTool {
    #[param(
        description = "Type of audit event (ACCESS, MODIFICATION, DELETION, AUTHENTICATION, AUTHORIZATION, SYSTEM)"
    )]
    pub event_type: String,

    #[param(description = "Specific action performed")]
    pub action: String,

    #[param(description = "Result of the action (SUCCESS, FAILURE, PARTIAL)")]
    pub result: String,

    #[param(description = "Optional actor who performed the action", optional)]
    pub actor: Option<String>,

    #[param(description = "Optional resource that was acted upon", optional)]
    pub resource: Option<String>,

    #[param(description = "Optional additional metadata", optional)]
    pub metadata: Option<Value>,
}

impl LogAuditEventTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<Value> {
        let session =
            session.ok_or_else(|| McpError::SessionError("Session required".to_string()))?;
        let db_pool = get_db_pool()?;

        let metadata = self.metadata.clone().unwrap_or(json!({}));

        // Create audit event
        let audit_event = AuditEvent {
            id: Uuid::now_v7().as_simple().to_string(),
            timestamp: Utc::now(),
            session_id: session.session_id.clone(),
            event_type: self.event_type.clone(),
            actor: self.actor.clone(),
            resource: self.resource.clone(),
            action: self.action.clone(),
            result: self.result.clone(),
            metadata,
        };

        // Store in database (immutable)
        let db_result = sqlx::query(
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
        .execute(&**db_pool)
        .await;

        match db_result {
            Ok(_) => {
                // Send progress notification
                session
                    .notify_progress(
                        format!("audit_{}", audit_event.event_type.to_lowercase()),
                        1,
                    )
                    .await;

                Ok(json!({
                    "logged": true,
                    "audit_id": audit_event.id,
                    "timestamp": audit_event.timestamp,
                    "event_type": audit_event.event_type,
                    "action": audit_event.action,
                    "result": audit_event.result,
                    "immutable": true,
                    "compliance": "LOGGED"
                }))
            }
            Err(e) => Err(McpError::tool_execution(&format!(
                "Failed to log audit event: {}",
                e
            ))),
        }
    }
}

/// Search audit trail with filters and pagination
#[derive(McpTool, Clone, Default, Deserialize)]
#[tool(
    name = "search_audit_trail",
    description = "Search audit trail with filters and pagination"
)]
pub struct SearchAuditTrailTool {
    #[param(description = "Start time filter (ISO 8601)", optional)]
    pub start_time: Option<String>,

    #[param(description = "End time filter (ISO 8601)", optional)]
    pub end_time: Option<String>,

    #[param(description = "Filter by event type", optional)]
    pub event_type: Option<String>,

    #[param(description = "Filter by actor", optional)]
    pub actor: Option<String>,

    #[param(description = "Filter by resource", optional)]
    pub resource: Option<String>,

    #[param(description = "Maximum results (default: 50, max: 1000)", optional)]
    pub limit: Option<i64>,

    #[param(description = "Results offset for pagination (default: 0)", optional)]
    pub offset: Option<i64>,
}

impl SearchAuditTrailTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<Value> {
        let db_pool = get_db_pool()?;
        let limit = self.limit.unwrap_or(50).min(1000) as i32;
        let offset = self.offset.unwrap_or(0) as i32;

        // Build dynamic query
        let mut query = "SELECT * FROM audit_logs WHERE 1=1".to_string();
        let mut conditions = Vec::new();

        if let Some(ref start_time) = self.start_time {
            conditions.push(format!(" AND timestamp >= '{}'", start_time));
        }
        if let Some(ref end_time) = self.end_time {
            conditions.push(format!(" AND timestamp <= '{}'", end_time));
        }
        if let Some(ref event_type) = self.event_type {
            conditions.push(format!(" AND event_type = '{}'", event_type));
        }
        if let Some(ref actor) = self.actor {
            conditions.push(format!(" AND actor = '{}'", actor));
        }
        if let Some(ref resource) = self.resource {
            conditions.push(format!(" AND resource = '{}'", resource));
        }

        for condition in conditions {
            query.push_str(&condition);
        }
        query.push_str(" ORDER BY timestamp DESC LIMIT ? OFFSET ?");

        let rows = sqlx::query(&query)
            .bind(limit)
            .bind(offset)
            .fetch_all(&**db_pool)
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
            .fetch_one(&**db_pool)
            .await
            .map_err(|e| McpError::tool_execution(&format!("Count query failed: {}", e)))?;
        let total: i32 = total_row.get("total");

        Ok(json!({
            "results": audit_events,
            "pagination": {
                "total": total,
                "limit": limit,
                "offset": offset,
                "has_more": offset + limit < total
            },
            "search_criteria": {
                "start_time": self.start_time,
                "end_time": self.end_time,
                "event_type": self.event_type,
                "actor": self.actor,
                "resource": self.resource,
                "limit": limit,
                "offset": offset
            }
        }))
    }
}

/// Generate compliance report with audit statistics
#[derive(McpTool, Clone, Default, Deserialize)]
#[tool(
    name = "generate_compliance_report",
    description = "Generate compliance report with audit trail statistics"
)]
pub struct GenerateComplianceReportTool {
    #[param(description = "Type of compliance report (SUMMARY, DETAILED, COMPLIANCE)")]
    pub report_type: String,

    #[param(description = "Start date for report (ISO 8601)", optional)]
    pub start_date: Option<String>,

    #[param(description = "End date for report (ISO 8601)", optional)]
    pub end_date: Option<String>,
}

impl GenerateComplianceReportTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<Value> {
        let db_pool = get_db_pool()?;

        // Build time filter
        let mut time_filter = String::new();
        if let Some(ref start_date) = self.start_date {
            time_filter.push_str(&format!(" AND timestamp >= '{}'", start_date));
        }
        if let Some(ref end_date) = self.end_date {
            time_filter.push_str(&format!(" AND timestamp <= '{}'", end_date));
        }

        // Get total event counts by type
        let event_counts_query = format!(
            "SELECT event_type, COUNT(*) as count FROM audit_logs WHERE 1=1 {} GROUP BY event_type",
            time_filter
        );
        let event_rows = sqlx::query(&event_counts_query)
            .fetch_all(&**db_pool)
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
            .fetch_all(&**db_pool)
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
            .fetch_one(&**db_pool)
            .await
            .map_err(|e| McpError::tool_execution(&format!("Session stats query failed: {}", e)))?;

        let unique_sessions: i32 = stats_row.get("unique_sessions");
        let total_events: i32 = stats_row.get("total_events");

        let report = match self.report_type.as_str() {
            "SUMMARY" => json!({
                "report_type": "SUMMARY",
                "generated_at": Utc::now(),
                "period": {
                    "start_date": self.start_date,
                    "end_date": self.end_date
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
                    "start_date": self.start_date,
                    "end_date": self.end_date
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
                    "start_date": self.start_date,
                    "end_date": self.end_date
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
                    &self.report_type,
                ));
            }
        };

        Ok(report)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("Starting Audit Trail MCP Server with SQLite persistence");

    // Initialize SQLite database with create mode
    let db_url = "sqlite://audit_trail.db?mode=rwc";
    let pool = SqlitePool::connect(db_url).await.map_err(|e| {
        eprintln!("Failed to connect to SQLite database: {}", e);
        e
    })?;
    init_database(&pool).await?;
    let db_pool = Arc::new(pool);

    // Set global database pool
    DB_POOL.set(db_pool).expect("DB_POOL already initialized");

    // Create SQLite session storage
    let session_storage = Arc::new(SqliteSessionStorage::new().await?);
    println!("Session storage initialized successfully");

    let server = McpServer::builder()
        .name("audit-trail-server")
        .version("1.0.0")
        .title("Audit Trail Server")
        .instructions(
            "This server provides compliance-focused audit trail logging with SQLite persistence.",
        )
        .with_session_storage(session_storage)
        .tool(LogAuditEventTool::default())
        .tool(SearchAuditTrailTool::default())
        .tool(GenerateComplianceReportTool::default())
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
