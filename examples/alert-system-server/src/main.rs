//! # Alert System Server Example
//!
//! This example demonstrates a rule-based alert management system with cooldowns and severity levels.
//! It shows how to configure alert rules, check conditions, and manage alert history using session state.
//!
//! **Development Pattern**: Uses `#[derive(McpTool)]` macros for minimal code

use chrono::{DateTime, Duration, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use turul_mcp_derive::McpTool;
use turul_mcp_protocol::McpError;
use turul_mcp_server::prelude::*;
use uuid::Uuid;

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
#[derive(McpTool, Clone, Default, Deserialize)]
#[tool(
    name = "configure_alert_rule",
    description = "Configure alert rules with conditions and notification channels"
)]
pub struct ConfigureAlertRuleTool {
    /// Alert rule name
    #[param(description = "Alert rule name")]
    pub name: String,

    /// Alert rule description
    #[param(description = "Alert rule description", optional)]
    pub description: Option<String>,

    /// Type of alert condition
    #[param(description = "Type of alert condition: metric, log_pattern, threshold, count")]
    pub condition_type: String,

    /// Condition-specific configuration
    #[param(description = "Condition-specific configuration object")]
    pub condition_config: Value,

    /// Alert severity level
    #[param(description = "Alert severity level: LOW, MEDIUM, HIGH, CRITICAL")]
    pub severity: String,

    /// Minutes to wait before firing again
    #[param(
        description = "Minutes to wait before firing again (default: 15)",
        optional
    )]
    pub cooldown_minutes: Option<i64>,

    /// Notification channels
    #[param(description = "Notification channels: email, slack, webhook", optional)]
    pub channels: Option<Vec<String>>,

    /// Enable/disable the rule
    #[param(description = "Enable/disable the rule (default: true)", optional)]
    pub enabled: Option<bool>,
}

impl ConfigureAlertRuleTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<Value> {
        let session = session.ok_or_else(|| McpError::tool_execution("Session required"))?;

        let cooldown_minutes = self.cooldown_minutes.unwrap_or(15);
        let channels = self.channels.clone().unwrap_or_default();
        let enabled = self.enabled.unwrap_or(true);
        let description = self.description.clone().unwrap_or_default();

        // Validate condition config based on type
        match self.condition_type.as_str() {
            "metric" => {
                self.condition_config
                    .get("metric_name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        McpError::tool_execution("metric condition requires metric_name")
                    })?;
                self.condition_config
                    .get("operator")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        McpError::tool_execution(
                            "metric condition requires operator (>, <, >=, <=, ==, !=)",
                        )
                    })?;
                self.condition_config
                    .get("threshold")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| {
                        McpError::tool_execution("metric condition requires threshold number")
                    })?;
            }
            "log_pattern" => {
                let pattern = self
                    .condition_config
                    .get("pattern")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        McpError::tool_execution("log_pattern condition requires pattern regex")
                    })?;
                Regex::new(pattern)
                    .map_err(|_| McpError::tool_execution("invalid regex pattern"))?;
                self.condition_config
                    .get("count_threshold")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| {
                        McpError::tool_execution("log_pattern condition requires count_threshold")
                    })?;
            }
            "threshold" => {
                self.condition_config
                    .get("value_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        McpError::tool_execution("threshold condition requires value_path")
                    })?;
                if self.condition_config.get("min").is_none()
                    && self.condition_config.get("max").is_none()
                {
                    return Err(McpError::tool_execution(
                        "threshold condition requires min and/or max",
                    ));
                }
            }
            "count" => {
                self.condition_config
                    .get("event_type")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        McpError::tool_execution("count condition requires event_type")
                    })?;
                self.condition_config
                    .get("count")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| {
                        McpError::tool_execution("count condition requires count threshold")
                    })?;
            }
            _ => {
                return Err(McpError::invalid_param_type(
                    "condition_type",
                    "metric|log_pattern|threshold|count",
                    &self.condition_type,
                ));
            }
        };

        // Create alert rule
        let rule = AlertRule {
            id: Uuid::now_v7().as_simple().to_string(),
            name: self.name.clone(),
            description,
            condition_type: self.condition_type.clone(),
            condition_config: self.condition_config.clone(),
            severity: self.severity.clone(),
            cooldown_minutes,
            channels,
            enabled,
            created_at: Utc::now(),
        };

        // Store rule in session state
        let mut rules: HashMap<String, AlertRule> = session
            .get_typed_state("alert_rules")
            .await
            .unwrap_or_default();
        rules.insert(rule.id.clone(), rule.clone());
        session
            .set_typed_state("alert_rules", &rules)
            .await
            .unwrap();

        // Send configuration notification
        session
            .notify_progress(
                format!("alert_rule_created_{}", rule.severity.to_lowercase()),
                1,
            )
            .await;

        Ok(json!({
            "created": true,
            "rule_id": rule.id,
            "name": rule.name,
            "condition_type": rule.condition_type,
            "severity": rule.severity,
            "enabled": rule.enabled,
            "cooldown_minutes": rule.cooldown_minutes,
            "channels": rule.channels,
            "total_rules": rules.len()
        }))
    }
}

/// Check alert conditions against provided data
#[derive(McpTool, Clone, Default, Deserialize)]
#[tool(
    name = "check_alert_conditions",
    description = "Check alert conditions against provided data and fire alerts"
)]
pub struct CheckAlertConditionsTool {
    /// Data to check against alert conditions
    #[param(description = "Data to check against alert conditions")]
    pub data: Value,

    /// Type of data being checked
    #[param(description = "Type of data being checked: metric, log_entry, event")]
    pub data_type: String,

    /// Automatically acknowledge low-severity alerts
    #[param(
        description = "Automatically acknowledge low-severity alerts",
        optional
    )]
    pub auto_acknowledge: Option<bool>,
}

impl CheckAlertConditionsTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<Value> {
        let session = session.ok_or_else(|| McpError::tool_execution("Session required"))?;

        let auto_acknowledge = self.auto_acknowledge.unwrap_or(false);

        // Get alert rules and cooldowns
        let rules: HashMap<String, AlertRule> = session
            .get_typed_state("alert_rules")
            .await
            .unwrap_or_default();
        let mut cooldowns: HashMap<String, AlertCooldown> = session
            .get_typed_state("alert_cooldowns")
            .await
            .unwrap_or_default();
        let mut alert_events: Vec<AlertEvent> = session
            .get_typed_state("alert_events")
            .await
            .unwrap_or_default();

        let now = Utc::now();
        let mut fired_alerts = Vec::new();
        let mut checked_rules = 0;
        let mut skipped_cooldown = 0;

        // Check each enabled rule
        for rule in rules.values().filter(|r| r.enabled) {
            checked_rules += 1;

            // Check cooldown
            if let Some(cooldown) = cooldowns.get(&rule.id)
                && now < cooldown.cooldown_until
            {
                skipped_cooldown += 1;
                continue;
            }

            // Check condition based on rule type and data type compatibility
            let alert_message = match (&rule.condition_type[..], self.data_type.as_str()) {
                ("metric", "metric") => check_metric_condition(&self.data, rule),
                ("log_pattern", "log_entry") => check_log_pattern_condition(&self.data, rule),
                ("threshold", _) => check_threshold_condition(&self.data, rule),
                ("count", "event") => {
                    let event_type = rule
                        .condition_config
                        .get("event_type")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| McpError::missing_param("event_type"))?;
                    if self.data.get("type").and_then(|v| v.as_str()) == Some(event_type) {
                        Some(format!("Event type '{}' occurred", event_type))
                    } else {
                        None
                    }
                }
                _ => None,
            };

            if let Some(message) = alert_message {
                let alert_event = AlertEvent {
                    id: Uuid::now_v7().as_simple().to_string(),
                    rule_id: rule.id.clone(),
                    timestamp: now,
                    severity: rule.severity.clone(),
                    message,
                    details: self.data.clone(),
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

                cooldowns.insert(
                    rule.id.clone(),
                    AlertCooldown {
                        rule_id: rule.id.clone(),
                        last_fired: now,
                        cooldown_until: now + Duration::minutes(rule.cooldown_minutes),
                    },
                );

                session
                    .notify_progress(format!("alert_fired_{}", rule.severity.to_lowercase()), 1)
                    .await;
            }
        }

        // Update session state
        session
            .set_typed_state("alert_cooldowns", &cooldowns)
            .await
            .unwrap();
        session
            .set_typed_state("alert_events", &alert_events)
            .await
            .unwrap();

        Ok(json!({
            "checked_at": now,
            "data_type": self.data_type,
            "rules_checked": checked_rules,
            "skipped_cooldown": skipped_cooldown,
            "alerts_fired": fired_alerts.len(),
            "fired_alerts": fired_alerts,
            "total_alerts": alert_events.len()
        }))
    }
}

fn check_metric_condition(data: &Value, rule: &AlertRule) -> Option<String> {
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
        Some(format!(
            "{} {} {} (current: {})",
            metric_name, operator, threshold, current_value
        ))
    } else {
        None
    }
}

fn check_log_pattern_condition(data: &Value, rule: &AlertRule) -> Option<String> {
    let config = &rule.condition_config;
    let pattern = config.get("pattern")?.as_str()?;
    let log_message = data.get("message")?.as_str()?;

    if let Ok(regex) = Regex::new(pattern)
        && regex.is_match(log_message)
    {
        return Some(format!(
            "Log pattern '{}' matched: {}",
            pattern, log_message
        ));
    }
    None
}

fn check_threshold_condition(data: &Value, rule: &AlertRule) -> Option<String> {
    let config = &rule.condition_config;
    let value_path = config.get("value_path")?.as_str()?;
    let current_value = data.get(value_path)?.as_f64()?;

    let mut violations = Vec::new();

    if let Some(min) = config.get("min").and_then(|v| v.as_f64())
        && current_value < min
    {
        violations.push(format!(
            "{} below minimum {} (current: {})",
            value_path, min, current_value
        ));
    }

    if let Some(max) = config.get("max").and_then(|v| v.as_f64())
        && current_value > max
    {
        violations.push(format!(
            "{} above maximum {} (current: {})",
            value_path, max, current_value
        ));
    }

    if violations.is_empty() {
        None
    } else {
        Some(violations.join(", "))
    }
}

/// Get alert history and manage alert acknowledgments
#[derive(McpTool, Clone, Default, Deserialize)]
#[tool(
    name = "get_alert_history",
    description = "Get alert history and manage alert acknowledgments"
)]
pub struct GetAlertHistoryTool {
    /// Maximum number of alerts to return
    #[param(
        description = "Maximum number of alerts to return (default: 20)",
        optional
    )]
    pub limit: Option<i64>,

    /// Filter by severity level
    #[param(
        description = "Filter by severity level: LOW, MEDIUM, HIGH, CRITICAL",
        optional
    )]
    pub severity_filter: Option<String>,

    /// Filter by acknowledgment status
    #[param(
        description = "Filter by acknowledgment status: all, acknowledged, unacknowledged",
        optional
    )]
    pub acknowledged_filter: Option<String>,

    /// Alert ID to acknowledge
    #[param(description = "Alert ID to acknowledge", optional)]
    pub acknowledge_alert_id: Option<String>,

    /// Person acknowledging the alert
    #[param(description = "Person acknowledging the alert", optional)]
    pub acknowledged_by: Option<String>,
}

impl GetAlertHistoryTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<Value> {
        let session = session.ok_or_else(|| McpError::tool_execution("Session required"))?;

        let limit = self.limit.unwrap_or(20) as usize;
        let acknowledged_filter = self.acknowledged_filter.as_deref().unwrap_or("all");

        // Get alert events and rules
        let mut alert_events: Vec<AlertEvent> = session
            .get_typed_state("alert_events")
            .await
            .unwrap_or_default();
        let rules: HashMap<String, AlertRule> = session
            .get_typed_state("alert_rules")
            .await
            .unwrap_or_default();

        // Handle alert acknowledgment
        if let Some(alert_id) = &self.acknowledge_alert_id {
            if let Some(acknowledged_by_user) = &self.acknowledged_by {
                if let Some(alert) = alert_events.iter_mut().find(|a| a.id == *alert_id) {
                    if !alert.acknowledged {
                        alert.acknowledged = true;
                        alert.acknowledged_by = Some(acknowledged_by_user.to_string());
                        let ack_time = Some(Utc::now());
                        alert.acknowledged_at = ack_time;

                        session
                            .set_typed_state("alert_events", &alert_events)
                            .await
                            .unwrap();

                        return Ok(json!({
                            "action": "acknowledged",
                            "alert_id": alert_id,
                            "acknowledged_by": acknowledged_by_user,
                            "acknowledged_at": ack_time
                        }));
                    } else {
                        return Err(McpError::tool_execution(&format!(
                            "Alert {} is already acknowledged",
                            alert_id
                        )));
                    }
                } else {
                    return Err(McpError::tool_execution(&format!(
                        "Alert {} not found",
                        alert_id
                    )));
                }
            } else {
                return Err(McpError::missing_param("acknowledged_by"));
            }
        }

        // Filter alerts
        let mut filtered_alerts: Vec<&AlertEvent> = alert_events.iter().collect();

        if let Some(severity) = &self.severity_filter {
            filtered_alerts.retain(|alert| alert.severity == *severity);
        }

        match acknowledged_filter {
            "acknowledged" => filtered_alerts.retain(|alert| alert.acknowledged),
            "unacknowledged" => filtered_alerts.retain(|alert| !alert.acknowledged),
            _ => {}
        }

        // Sort by timestamp (most recent first) and limit
        filtered_alerts.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        filtered_alerts.truncate(limit);

        // Build response with rule information
        let alerts_with_rules: Vec<Value> = filtered_alerts
            .iter()
            .map(|alert| {
                let rule_name = rules
                    .get(&alert.rule_id)
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
            })
            .collect();

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

        Ok(json!({
            "alerts": alerts_with_rules,
            "statistics": {
                "total_alerts": total_alerts,
                "unacknowledged": unacknowledged_count,
                "acknowledged": total_alerts - unacknowledged_count,
                "severity_breakdown": severity_counts,
                "filtered_count": filtered_alerts.len()
            },
            "filters": {
                "severity": self.severity_filter,
                "acknowledged_status": acknowledged_filter,
                "limit": limit
            }
        }))
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
    println!(
        "Sample metric: {} = {} (timestamp: {})",
        demo_metric.name, demo_metric.value, demo_metric.timestamp
    );

    let server = McpServer::builder()
        .name("alert-system-server")
        .version("1.0.0")
        .title("Alert System Server")
        .instructions(
            "This server provides rule-based alert management with cooldowns and severity levels.",
        )
        .tool(ConfigureAlertRuleTool::default())
        .tool(CheckAlertConditionsTool::default())
        .tool(GetAlertHistoryTool::default())
        .bind_address("127.0.0.1:8010".parse()?)
        .sse(true)
        .build()?;

    println!("Alert system server running at: http://127.0.0.1:8010/mcp");
    println!("\nAvailable tools:");
    println!("  - configure_alert_rule: Create and configure alert rules");
    println!("  - check_alert_conditions: Check data against alert conditions");
    println!("  - get_alert_history: View and acknowledge alerts");
    println!("\nExample usage:");
    println!(
        "  1. configure_alert_rule(name='CPU High', condition_type='metric', condition_config={{'metric_name':'cpu_usage','operator':'>','threshold':80}}, severity='HIGH')"
    );
    println!("  2. check_alert_conditions(data={{'cpu_usage':85.2}}, data_type='metric')");
    println!("  3. get_alert_history(acknowledged_filter='unacknowledged', limit=10)");

    server.run().await?;
    Ok(())
}
