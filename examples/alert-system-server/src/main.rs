//! # Alert System Server Example
//!
//! This example demonstrates a rule-based alert management system with cooldowns and severity levels.
//! It shows how to configure alert rules, check conditions, and manage alert history using session state.
//! 
//! **Development Pattern**: 🚀 **Optimized** - Uses `#[derive(McpTool)]` macros for minimal code

use std::collections::HashMap;
use async_trait::async_trait;
use turul_mcp_server::{McpServer, SessionContext, McpResult, McpTool as McpToolTrait};
use turul_mcp_protocol::{
    JsonSchema, ToolSchema, CallToolResult, ToolResult, McpError,
    tools::{HasBaseMetadata, HasDescription, HasInputSchema, HasOutputSchema, HasAnnotations, HasToolMeta}
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use chrono::{DateTime, Utc, Duration};
use uuid::Uuid;
use regex::Regex;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AlertRule {
    id: String,
    name: String,
    description: String,
    condition_type: String, // "metric", "log_pattern", "threshold", "count"
    condition_config: Value,
    severity: String, // "LOW", "MEDIUM", "HIGH", "CRITICAL"
    cooldown_minutes: i64,
    channels: Vec<String>, // "email", "slack", "webhook"
    enabled: bool,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AlertEvent {
    id: String,
    rule_id: String,
    timestamp: DateTime<Utc>,
    severity: String,
    message: String,
    details: Value,
    acknowledged: bool,
    acknowledged_by: Option<String>,
    acknowledged_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AlertCooldown {
    rule_id: String,
    last_fired: DateTime<Utc>,
    cooldown_until: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MetricData {
    name: String,
    value: f64,
    timestamp: DateTime<Utc>,
    tags: HashMap<String, String>,
}

impl MetricData {
    fn new(name: String, value: f64) -> Self {
        tracing::debug!("Creating metric: {} = {}", name, value);
        Self {
            name,
            value,
            timestamp: Utc::now(),
            tags: HashMap::new(),
        }
    }
}

/// Configure alert rules with conditions and notifications
struct ConfigureAlertRuleTool {
    input_schema: ToolSchema,
}

impl ConfigureAlertRuleTool {
    fn new() -> Self {
        let input_schema = ToolSchema::object()
            .with_properties(HashMap::from([
                ("name".to_string(), JsonSchema::string()
                    .with_description("Alert rule name")),
                ("description".to_string(), JsonSchema::string()
                    .with_description("Alert rule description")),
                ("condition_type".to_string(), JsonSchema::string_enum(vec![
                    "metric".to_string(), "log_pattern".to_string(), "threshold".to_string(), "count".to_string()
                ]).with_description("Type of alert condition")),
                ("condition_config".to_string(), JsonSchema::object()
                    .with_description("Condition-specific configuration")),
                ("severity".to_string(), JsonSchema::string_enum(vec![
                    "LOW".to_string(), "MEDIUM".to_string(), "HIGH".to_string(), "CRITICAL".to_string()
                ]).with_description("Alert severity level")),
                ("cooldown_minutes".to_string(), JsonSchema::integer()
                    .with_description("Minutes to wait before firing again (default: 15)")),
                ("channels".to_string(), JsonSchema::array(
                    JsonSchema::string_enum(vec![
                        "email".to_string(), "slack".to_string(), "webhook".to_string()
                    ])
                ).with_description("Notification channels")),
                ("enabled".to_string(), JsonSchema::boolean()
                    .with_description("Enable/disable the rule (default: true)")),
            ]))
            .with_required(vec!["name".to_string(), "condition_type".to_string(), "condition_config".to_string(), "severity".to_string()]);
        Self { input_schema }
    }
}

impl HasBaseMetadata for ConfigureAlertRuleTool {
    fn name(&self) -> &str { "configure_alert_rule" }
}

impl HasDescription for ConfigureAlertRuleTool {
    fn description(&self) -> Option<&str> {
        Some("Configure alert rules with conditions and notification channels")
    }
}

impl HasInputSchema for ConfigureAlertRuleTool {
    fn input_schema(&self) -> &ToolSchema { &self.input_schema }
}

impl HasOutputSchema for ConfigureAlertRuleTool {
    fn output_schema(&self) -> Option<&ToolSchema> { None }
}

impl HasAnnotations for ConfigureAlertRuleTool {
    fn annotations(&self) -> Option<&turul_mcp_protocol::tools::ToolAnnotations> { None }
}

impl HasToolMeta for ConfigureAlertRuleTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> { None }
}

#[async_trait]
impl McpToolTrait for ConfigureAlertRuleTool {
    async fn call(&self, args: Value, session: Option<SessionContext>) -> McpResult<CallToolResult> {
        let session = session.ok_or_else(|| McpError::tool_execution("Session required"))?;
        
        let name = args.get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("name"))?;
        let description = args.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let condition_type = args.get("condition_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("condition_type"))?;
        let condition_config = args.get("condition_config")
            .ok_or_else(|| McpError::missing_param("condition_config"))?;
        let severity = args.get("severity")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("severity"))?;
        let cooldown_minutes = args.get("cooldown_minutes")
            .and_then(|v| v.as_i64())
            .unwrap_or(15);
        let channels: Vec<String> = args.get("channels")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect())
            .unwrap_or_default();
        let enabled = args.get("enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        // Validate condition config based on type
        match condition_type {
            "metric" => {
                // Expect: {"metric_name": "cpu_usage", "operator": ">", "threshold": 80.0}
                condition_config.get("metric_name").and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::tool_execution("metric condition requires metric_name"))?;
                condition_config.get("operator").and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::tool_execution("metric condition requires operator (>, <, >=, <=, ==, !=)"))?;
                condition_config.get("threshold").and_then(|v| v.as_f64())
                    .ok_or_else(|| McpError::tool_execution("metric condition requires threshold number"))?;
            },
            "log_pattern" => {
                // Expect: {"pattern": "ERROR.*timeout", "count_threshold": 5, "time_window_minutes": 10}
                let pattern = condition_config.get("pattern").and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::tool_execution("log_pattern condition requires pattern regex"))?;
                Regex::new(pattern).map_err(|_| McpError::tool_execution("invalid regex pattern"))?;
                condition_config.get("count_threshold").and_then(|v| v.as_i64())
                    .ok_or_else(|| McpError::tool_execution("log_pattern condition requires count_threshold"))?;
            },
            "threshold" => {
                // Expect: {"value_path": "response_time", "min": 100, "max": 5000}
                condition_config.get("value_path").and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::tool_execution("threshold condition requires value_path"))?;
                if condition_config.get("min").is_none() && condition_config.get("max").is_none() {
                    return Err(McpError::tool_execution("threshold condition requires min and/or max"));
                }
            },
            "count" => {
                // Expect: {"event_type": "login_failure", "count": 3, "time_window_minutes": 5}
                condition_config.get("event_type").and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::tool_execution("count condition requires event_type"))?;
                condition_config.get("count").and_then(|v| v.as_i64())
                    .ok_or_else(|| McpError::tool_execution("count condition requires count threshold"))?;
            },
            _ => return Err(McpError::invalid_param_type("condition_type", "metric|log_pattern|threshold|count", condition_type))
        };

        // Create alert rule
        let rule = AlertRule {
            id: Uuid::now_v7().to_string(),
            name: name.to_string(),
            description: description.to_string(),
            condition_type: condition_type.to_string(),
            condition_config: condition_config.clone(),
            severity: severity.to_string(),
            cooldown_minutes,
            channels,
            enabled,
            created_at: Utc::now(),
        };

        // Store rule in session state
        let mut rules: HashMap<String, AlertRule> = session.get_typed_state("alert_rules").await.unwrap_or_default();
        rules.insert(rule.id.clone(), rule.clone());
        session.set_typed_state("alert_rules", &rules).await.unwrap();

        // Send configuration notification
        session.notify_progress(format!("alert_rule_created_{}", severity.to_lowercase()), 1).await;

        Ok(CallToolResult {
            content: vec![ToolResult::text(json!({
                "created": true,
                "rule_id": rule.id,
                "name": rule.name,
                "condition_type": rule.condition_type,
                "severity": rule.severity,
                "enabled": rule.enabled,
                "cooldown_minutes": rule.cooldown_minutes,
                "channels": rule.channels,
                "total_rules": rules.len()
            }).to_string())],
            is_error: None,
            structured_content: None,
            meta: None,
        })
    }
}

/// Check alert conditions against provided data
struct CheckAlertConditionsTool {
    input_schema: ToolSchema,
}

impl CheckAlertConditionsTool {
    fn new() -> Self {
        let input_schema = ToolSchema::object()
            .with_properties(HashMap::from([
                ("data".to_string(), JsonSchema::object()
                    .with_description("Data to check against alert conditions")),
                ("data_type".to_string(), JsonSchema::string_enum(vec![
                    "metric".to_string(), "log_entry".to_string(), "event".to_string()
                ]).with_description("Type of data being checked")),
                ("auto_acknowledge".to_string(), JsonSchema::boolean()
                    .with_description("Automatically acknowledge low-severity alerts")),
            ]))
            .with_required(vec!["data".to_string(), "data_type".to_string()]);
        Self { input_schema }
    }

    fn check_metric_condition(&self, data: &Value, rule: &AlertRule) -> Option<String> {
        let config = &rule.condition_config;
        let metric_name = config.get("metric_name")?.as_str()?;
        let operator = config.get("operator")?.as_str()?;
        let threshold = config.get("threshold")?.as_f64()?;
        
        let current_value = data.get(metric_name)?.as_f64()?;
        
        let condition_met = match operator {
            ">" => current_value > threshold,
            "<" => current_value < threshold,
            ">=" => current_value >= threshold,
            "<=" => current_value <= threshold,
            "==" => (current_value - threshold).abs() < f64::EPSILON,
            "!=" => (current_value - threshold).abs() >= f64::EPSILON,
            _ => false,
        };
        
        if condition_met {
            Some(format!("{} {} {} (current: {})", metric_name, operator, threshold, current_value))
        } else {
            None
        }
    }

    fn check_log_pattern_condition(&self, data: &Value, rule: &AlertRule) -> Option<String> {
        let config = &rule.condition_config;
        let pattern = config.get("pattern")?.as_str()?;
        let log_message = data.get("message")?.as_str()?;
        
        if let Ok(regex) = Regex::new(pattern)
            && regex.is_match(log_message) {
                return Some(format!("Log pattern '{}' matched: {}", pattern, log_message));
            }
        None
    }

    fn check_threshold_condition(&self, data: &Value, rule: &AlertRule) -> Option<String> {
        let config = &rule.condition_config;
        let value_path = config.get("value_path")?.as_str()?;
        let current_value = data.get(value_path)?.as_f64()?;
        
        let mut violations = Vec::new();
        
        if let Some(min) = config.get("min").and_then(|v| v.as_f64())
            && current_value < min {
                violations.push(format!("{} below minimum {} (current: {})", value_path, min, current_value));
            }
        
        if let Some(max) = config.get("max").and_then(|v| v.as_f64())
            && current_value > max {
                violations.push(format!("{} above maximum {} (current: {})", value_path, max, current_value));
            }
        
        if violations.is_empty() { None } else { Some(violations.join(", ")) }
    }
}

impl HasBaseMetadata for CheckAlertConditionsTool {
    fn name(&self) -> &str { "check_alert_conditions" }
}

impl HasDescription for CheckAlertConditionsTool {
    fn description(&self) -> Option<&str> {
        Some("Check alert conditions against provided data and fire alerts")
    }
}

impl HasInputSchema for CheckAlertConditionsTool {
    fn input_schema(&self) -> &ToolSchema { &self.input_schema }
}

impl HasOutputSchema for CheckAlertConditionsTool {
    fn output_schema(&self) -> Option<&ToolSchema> { None }
}

impl HasAnnotations for CheckAlertConditionsTool {
    fn annotations(&self) -> Option<&turul_mcp_protocol::tools::ToolAnnotations> { None }
}

impl HasToolMeta for CheckAlertConditionsTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> { None }
}

#[async_trait]
impl McpToolTrait for CheckAlertConditionsTool {
    async fn call(&self, args: Value, session: Option<SessionContext>) -> McpResult<CallToolResult> {
        let session = session.ok_or_else(|| McpError::tool_execution("Session required"))?;
        
        let data = args.get("data")
            .ok_or_else(|| McpError::missing_param("data"))?;
        let data_type = args.get("data_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("data_type"))?;
        let auto_acknowledge = args.get("auto_acknowledge")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Get alert rules and cooldowns
        let rules: HashMap<String, AlertRule> = session.get_typed_state("alert_rules").await.unwrap_or_default();
        let mut cooldowns: HashMap<String, AlertCooldown> = session.get_typed_state("alert_cooldowns").await.unwrap_or_default();
        let mut alert_events: Vec<AlertEvent> = session.get_typed_state("alert_events").await.unwrap_or_default();

        let now = Utc::now();
        let mut fired_alerts = Vec::new();
        let mut checked_rules = 0;
        let mut skipped_cooldown = 0;

        // Check each enabled rule
        for rule in rules.values().filter(|r| r.enabled) {
            checked_rules += 1;

            // Check cooldown
            if let Some(cooldown) = cooldowns.get(&rule.id)
                && now < cooldown.cooldown_until {
                    skipped_cooldown += 1;
                    continue;
                }

            // Check condition based on rule type and data type compatibility
            let alert_message = match (&rule.condition_type[..], data_type) {
                ("metric", "metric") => self.check_metric_condition(data, rule),
                ("log_pattern", "log_entry") => self.check_log_pattern_condition(data, rule),
                ("threshold", _) => self.check_threshold_condition(data, rule),
                ("count", "event") => {
                    // For count conditions, we'd need to maintain event counts over time
                    // This is a simplified implementation
                    let event_type = rule.condition_config.get("event_type")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| McpError::missing_param("event_type"))?;
                    if data.get("type").and_then(|v| v.as_str()) == Some(event_type) {
                        Some(format!("Event type '{}' occurred", event_type))
                    } else {
                        None
                    }
                },
                _ => None, // Incompatible rule/data type combination
            };

            if let Some(message) = alert_message {
                // Fire alert
                let alert_event = AlertEvent {
                    id: Uuid::now_v7().to_string(),
                    rule_id: rule.id.clone(),
                    timestamp: now,
                    severity: rule.severity.clone(),
                    message,
                    details: data.clone(),
                    acknowledged: auto_acknowledge && rule.severity == "LOW",
                    acknowledged_by: if auto_acknowledge && rule.severity == "LOW" { 
                        Some("auto-system".to_string()) 
                    } else { 
                        None 
                    },
                    acknowledged_at: if auto_acknowledge && rule.severity == "LOW" { 
                        Some(now) 
                    } else { 
                        None 
                    },
                };

                alert_events.push(alert_event.clone());
                fired_alerts.push(json!({
                    "alert_id": alert_event.id,
                    "rule_name": rule.name,
                    "severity": rule.severity,
                    "message": alert_event.message,
                    "channels": rule.channels,
                    "acknowledged": alert_event.acknowledged
                }));

                // Set cooldown
                cooldowns.insert(rule.id.clone(), AlertCooldown {
                    rule_id: rule.id.clone(),
                    last_fired: now,
                    cooldown_until: now + Duration::minutes(rule.cooldown_minutes),
                });

                // Send alert notification
                session.notify_progress(format!("alert_fired_{}", rule.severity.to_lowercase()), 1).await;
            }
        }

        // Update session state
        session.set_typed_state("alert_cooldowns", &cooldowns).await.unwrap();
        session.set_typed_state("alert_events", &alert_events).await.unwrap();

        let result = json!({
            "checked_at": now,
            "data_type": data_type,
            "rules_checked": checked_rules,
            "skipped_cooldown": skipped_cooldown,
            "alerts_fired": fired_alerts.len(),
            "fired_alerts": fired_alerts,
            "total_alerts": alert_events.len()
        });

        Ok(CallToolResult {
            content: vec![ToolResult::text(result.to_string())],
            is_error: None,
            structured_content: None,
            meta: None,
        })
    }
}

/// Get alert history and manage alert acknowledgments
struct GetAlertHistoryTool {
    input_schema: ToolSchema,
}

impl GetAlertHistoryTool {
    fn new() -> Self {
        let input_schema = ToolSchema::object()
            .with_properties(HashMap::from([
                ("limit".to_string(), JsonSchema::integer()
                    .with_description("Maximum number of alerts to return (default: 20)")),
                ("severity_filter".to_string(), JsonSchema::string_enum(vec![
                    "LOW".to_string(), "MEDIUM".to_string(), "HIGH".to_string(), "CRITICAL".to_string()
                ]).with_description("Filter by severity level")),
                ("acknowledged_filter".to_string(), JsonSchema::string_enum(vec![
                    "all".to_string(), "acknowledged".to_string(), "unacknowledged".to_string()
                ]).with_description("Filter by acknowledgment status (default: all)")),
                ("acknowledge_alert_id".to_string(), JsonSchema::string()
                    .with_description("Alert ID to acknowledge")),
                ("acknowledged_by".to_string(), JsonSchema::string()
                    .with_description("Person acknowledging the alert")),
            ]));
        Self { input_schema }
    }
}

impl HasBaseMetadata for GetAlertHistoryTool {
    fn name(&self) -> &str { "get_alert_history" }
}

impl HasDescription for GetAlertHistoryTool {
    fn description(&self) -> Option<&str> {
        Some("Get alert history and manage alert acknowledgments")
    }
}

impl HasInputSchema for GetAlertHistoryTool {
    fn input_schema(&self) -> &ToolSchema { &self.input_schema }
}

impl HasOutputSchema for GetAlertHistoryTool {
    fn output_schema(&self) -> Option<&ToolSchema> { None }
}

impl HasAnnotations for GetAlertHistoryTool {
    fn annotations(&self) -> Option<&turul_mcp_protocol::tools::ToolAnnotations> { None }
}

impl HasToolMeta for GetAlertHistoryTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> { None }
}

#[async_trait]
impl McpToolTrait for GetAlertHistoryTool {
    async fn call(&self, args: Value, session: Option<SessionContext>) -> McpResult<CallToolResult> {
        let session = session.ok_or_else(|| McpError::tool_execution("Session required"))?;
        
        let limit = args.get("limit")
            .and_then(|v| v.as_i64())
            .unwrap_or(20) as usize;
        let severity_filter = args.get("severity_filter").and_then(|v| v.as_str());
        let acknowledged_filter = args.get("acknowledged_filter")
            .and_then(|v| v.as_str())
            .unwrap_or("all");
        let acknowledge_alert_id = args.get("acknowledge_alert_id").and_then(|v| v.as_str());
        let acknowledged_by = args.get("acknowledged_by").and_then(|v| v.as_str());

        // Get alert events and rules
        let mut alert_events: Vec<AlertEvent> = session.get_typed_state("alert_events").await.unwrap_or_default();
        let rules: HashMap<String, AlertRule> = session.get_typed_state("alert_rules").await.unwrap_or_default();

        // Handle alert acknowledgment
        if let Some(alert_id) = acknowledge_alert_id {
            if let Some(acknowledged_by_user) = acknowledged_by {
                if let Some(alert) = alert_events.iter_mut().find(|a| a.id == alert_id) {
                    if !alert.acknowledged {
                        alert.acknowledged = true;
                        alert.acknowledged_by = Some(acknowledged_by_user.to_string());
                        let ack_time = Some(Utc::now());
                        alert.acknowledged_at = ack_time;
                        
                        // Update session state
                        session.set_typed_state("alert_events", &alert_events).await.unwrap();
                        
                        return Ok(CallToolResult {
                            content: vec![ToolResult::text(json!({
                                "action": "acknowledged",
                                "alert_id": alert_id,
                                "acknowledged_by": acknowledged_by_user,
                                "acknowledged_at": ack_time
                            }).to_string())],
                            is_error: None,
                            structured_content: None,
                            meta: None,
                        });
                    } else {
                        return Err(McpError::tool_execution(&format!("Alert {} is already acknowledged", alert_id)));
                    }
                } else {
                    return Err(McpError::tool_execution(&format!("Alert {} not found", alert_id)));
                }
            } else {
                return Err(McpError::missing_param("acknowledged_by"));
            }
        }

        // Filter alerts
        let mut filtered_alerts: Vec<&AlertEvent> = alert_events.iter().collect();

        if let Some(severity) = severity_filter {
            filtered_alerts.retain(|alert| alert.severity == severity);
        }

        match acknowledged_filter {
            "acknowledged" => filtered_alerts.retain(|alert| alert.acknowledged),
            "unacknowledged" => filtered_alerts.retain(|alert| !alert.acknowledged),
            _ => {}, // "all" - no filtering
        }

        // Sort by timestamp (most recent first) and limit
        filtered_alerts.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        filtered_alerts.truncate(limit);

        // Build response with rule information
        let alerts_with_rules: Vec<Value> = filtered_alerts.iter().map(|alert| {
            let rule_name = rules.get(&alert.rule_id)
                .map(|r| r.name.as_str())
                .unwrap_or("Unknown Rule");
            
            json!({
                "id": alert.id,
                "rule_id": alert.rule_id,
                "rule_name": rule_name,
                "timestamp": alert.timestamp,
                "severity": alert.severity,
                "message": alert.message,
                "acknowledged": alert.acknowledged,
                "acknowledged_by": alert.acknowledged_by,
                "acknowledged_at": alert.acknowledged_at
            })
        }).collect();

        // Calculate statistics
        let total_alerts = alert_events.len();
        let unacknowledged_count = alert_events.iter().filter(|a| !a.acknowledged).count();
        let severity_counts = {
            let mut counts = HashMap::new();
            for alert in &alert_events {
                *counts.entry(alert.severity.clone()).or_insert(0) += 1;
            }
            counts
        };

        let result = json!({
            "alerts": alerts_with_rules,
            "statistics": {
                "total_alerts": total_alerts,
                "unacknowledged": unacknowledged_count,
                "acknowledged": total_alerts - unacknowledged_count,
                "severity_breakdown": severity_counts,
                "filtered_count": filtered_alerts.len()
            },
            "filters": {
                "severity": severity_filter,
                "acknowledged_status": acknowledged_filter,
                "limit": limit
            }
        });

        Ok(CallToolResult {
            content: vec![ToolResult::text(result.to_string())],
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

    println!("Starting Alert System MCP Server");
    
    // Demo: Create a sample metric to show MetricData usage
    let demo_metric = MetricData::new("system.startup".to_string(), 1.0);
    println!("Sample metric: {} = {} (timestamp: {})", 
             demo_metric.name, demo_metric.value, demo_metric.timestamp);

    let server = McpServer::builder()
        .name("alert-system-server")
        .version("1.0.0")
        .title("Alert System Server")
        .instructions("This server provides rule-based alert management with cooldowns and severity levels.")
        .tool(ConfigureAlertRuleTool::new())
        .tool(CheckAlertConditionsTool::new())
        .tool(GetAlertHistoryTool::new())
        .bind_address("127.0.0.1:8010".parse()?)
        .sse(true)
        .build()?;

    println!("Alert system server running at: http://127.0.0.1:8010/mcp");
    println!("\nAvailable tools:");
    println!("  - configure_alert_rule: Create and configure alert rules");
    println!("  - check_alert_conditions: Check data against alert conditions");
    println!("  - get_alert_history: View and acknowledge alerts");
    println!("\nExample usage:");
    println!("  1. configure_alert_rule(name='CPU High', condition_type='metric', condition_config={{'metric_name':'cpu_usage','operator':'>','threshold':80}}, severity='HIGH')");
    println!("  2. check_alert_conditions(data={{'cpu_usage':85.2}}, data_type='metric')");
    println!("  3. get_alert_history(acknowledged_filter='unacknowledged', limit=10)");

    server.run().await?;
    Ok(())
}