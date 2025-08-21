//! # Development Team Notification Server
//!
//! This example demonstrates a real-world notification server for development teams
//! that provides CI/CD pipeline alerts, system monitoring notifications, security
//! alerts, and incident management workflows. The server loads notification
//! templates, team contacts, and escalation rules from external files.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use async_trait::async_trait;
use mcp_server::{McpServer, McpTool, SessionContext};
use mcp_protocol::{ToolSchema, ToolResult, schema::JsonSchema, McpError, McpResult};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json, from_str};
use tracing::info;
use chrono::Utc;
// use chrono::DateTime; // TODO: Use for timestamp parsing

#[derive(Debug, Deserialize, Serialize)]
struct NotificationTemplates {
    notification_types: HashMap<String, NotificationType>,
    notification_priorities: HashMap<String, NotificationPriority>,
    notification_channels: HashMap<String, NotificationChannel>,
    escalation_rules: HashMap<String, EscalationRule>,
}

#[derive(Debug, Deserialize, Serialize)]
struct NotificationType {
    name: String,
    description: String,
    templates: HashMap<String, NotificationTemplate>,
}

#[derive(Debug, Deserialize, Serialize)]
struct NotificationTemplate {
    title: String,
    message: String,
    priority: String,
    channels: Vec<String>,
    color: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct NotificationPriority {
    level: u8,
    description: String,
    default_channels: Vec<String>,
    escalation_delay: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize)]
struct NotificationChannel {
    name: String,
    description: String,
    config: HashMap<String, Value>,
    rate_limit: RateLimit,
}

#[derive(Debug, Deserialize, Serialize)]
struct RateLimit {
    requests_per_minute: u32,
    burst_limit: u32,
}

#[derive(Debug, Deserialize, Serialize)]
struct EscalationRule {
    description: String,
    stages: Vec<EscalationStage>,
}

#[derive(Debug, Deserialize, Serialize)]
struct EscalationStage {
    delay_seconds: u32,
    targets: Vec<String>,
    channels: Vec<String>,
}

/// Development team notification server that loads templates and workflows from external files
/// Real-world use case: Centralized notification management for CI/CD, monitoring, and incidents
struct DevNotificationTool {
    templates: NotificationTemplates,
    team_contacts: String,
    incident_workflows: String,
}

impl DevNotificationTool {
    fn new() -> McpResult<Self> {
        let templates_path = Path::new("data/notification_templates.json");
        let contacts_path = Path::new("data/team_contacts.yaml");
        let workflows_path = Path::new("data/incident_workflows.md");
        
        let templates = match fs::read_to_string(templates_path) {
            Ok(content) => {
                from_str::<NotificationTemplates>(&content)
                    .map_err(|e| McpError::tool_execution(&format!("Failed to parse notification templates: {}", e)))?
            },
            Err(_) => {
                // Fallback templates for basic functionality
                NotificationTemplates {
                    notification_types: HashMap::new(),
                    notification_priorities: HashMap::new(),
                    notification_channels: HashMap::new(),
                    escalation_rules: HashMap::new(),
                }
            }
        };
        
        let team_contacts = fs::read_to_string(contacts_path)
            .unwrap_or_else(|_| "# Team Contacts\n\nNo team contact information available.".to_string());
            
        let incident_workflows = fs::read_to_string(workflows_path)
            .unwrap_or_else(|_| "# Incident Workflows\n\nNo incident workflow documentation available.".to_string());
        
        Ok(Self { templates, team_contacts, incident_workflows })
    }
    
    fn get_notification_template(&self, notification_type: &str, template_name: &str) -> Option<&NotificationTemplate> {
        self.templates.notification_types
            .get(notification_type)
            .and_then(|nt| nt.templates.get(template_name))
    }
    
    fn get_escalation_rule(&self, rule_name: &str) -> Option<&EscalationRule> {
        self.templates.escalation_rules.get(rule_name)
    }
    
    fn format_notification(&self, template: &NotificationTemplate, variables: &HashMap<String, String>) -> String {
        let mut formatted_message = template.message.clone();
        
        for (key, value) in variables {
            let placeholder = format!("{{{}}}", key);
            formatted_message = formatted_message.replace(&placeholder, value);
        }
        
        formatted_message
    }
}

/// CI/CD pipeline notification tool that sends build and deployment alerts
struct CiCdNotificationTool {
    notification_tool: DevNotificationTool,
}

impl CiCdNotificationTool {
    fn new() -> McpResult<Self> {
        let notification_tool = DevNotificationTool::new()?;
        Ok(Self { notification_tool })
    }
}

#[async_trait]
impl McpTool for CiCdNotificationTool {
    fn name(&self) -> &str {
        "send_cicd_notification"
    }

    fn description(&self) -> &str {
        "Send CI/CD pipeline notifications (build status, deployments) using team templates loaded from external files"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("event_type".to_string(), JsonSchema::string_enum(vec![
                    "build_started".to_string(), "build_success".to_string(), "build_failed".to_string(),
                    "deploy_started".to_string(), "deploy_success".to_string(), "deploy_failed".to_string()
                ]).with_description("Type of CI/CD event")),
                ("build_number".to_string(), JsonSchema::string()
                    .with_description("Build or deployment number")),
                ("branch".to_string(), JsonSchema::string()
                    .with_description("Git branch name")),
                ("author".to_string(), JsonSchema::string()
                    .with_description("Commit author or deployer")),
                ("service".to_string(), JsonSchema::string()
                    .with_description("Service or application name")),
                ("version".to_string(), JsonSchema::string()
                    .with_description("Version being deployed")),
                ("environment".to_string(), JsonSchema::string()
                    .with_description("Target environment (staging, production, etc.)")),
                ("additional_context".to_string(), JsonSchema::object()
                    .with_description("Additional context variables for template substitution")),
            ]))
            .with_required(vec!["event_type".to_string()])
    }

    async fn call(&self, args: Value, _session: Option<SessionContext>) -> McpResult<Vec<ToolResult>> {
        let event_type = args.get("event_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("event_type"))?;
        
        let build_number = args.get("build_number")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
            
        let branch = args.get("branch")
            .and_then(|v| v.as_str())
            .unwrap_or("main");
            
        let author = args.get("author")
            .and_then(|v| v.as_str())
            .unwrap_or("system");
            
        let service = args.get("service")
            .and_then(|v| v.as_str())
            .unwrap_or("application");
            
        let version = args.get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("latest");
            
        let environment = args.get("environment")
            .and_then(|v| v.as_str())
            .unwrap_or("production");

        let notification_id = uuid::Uuid::new_v4().to_string();
        let timestamp = Utc::now();
        
        // Get the appropriate notification template
        if let Some(template) = self.notification_tool.get_notification_template("ci_cd", event_type) {
            // Build variables for template substitution
            let mut variables = HashMap::new();
            variables.insert("build_number".to_string(), build_number.to_string());
            variables.insert("branch".to_string(), branch.to_string());
            variables.insert("author".to_string(), author.to_string());
            variables.insert("service".to_string(), service.to_string());
            variables.insert("version".to_string(), version.to_string());
            variables.insert("environment".to_string(), environment.to_string());
            variables.insert("timestamp".to_string(), timestamp.to_rfc3339());
            
            // Add any additional context variables
            if let Some(additional) = args.get("additional_context").and_then(|v| v.as_object()) {
                for (key, value) in additional {
                    if let Some(val_str) = value.as_str() {
                        variables.insert(key.clone(), val_str.to_string());
                    }
                }
            }
            
            let formatted_message = self.notification_tool.format_notification(template, &variables);
            
            info!("Sending CI/CD notification: {} - {}", event_type, formatted_message);
            
            // In a real implementation, this would send notifications via configured channels
            // For demonstration, we show what would be sent
            
            Ok(vec![ToolResult::text(json!({
                "notification_id": notification_id,
                "event_type": event_type,
                "template_used": format!("ci_cd.{}", event_type),
                "formatted_message": formatted_message,
                "priority": template.priority,
                "channels": template.channels,
                "color": template.color,
                "timestamp": timestamp.to_rfc3339(),
                "variables_used": variables,
                "status": "notification_prepared",
                "note": "In production, this would be sent via configured channels (Slack, Email, SMS, PagerDuty)"
            }).to_string())])
        } else {
            Err(McpError::tool_execution(&format!("No template found for CI/CD event type: {}", event_type)))
        }
    }
}

/// System monitoring notification tool for infrastructure alerts
struct MonitoringNotificationTool {
    notification_tool: DevNotificationTool,
}

impl MonitoringNotificationTool {
    fn new() -> McpResult<Self> {
        let notification_tool = DevNotificationTool::new()?;
        Ok(Self { notification_tool })
    }
}

#[async_trait]
impl McpTool for MonitoringNotificationTool {
    fn name(&self) -> &str {
        "send_monitoring_alert"
    }

    fn description(&self) -> &str {
        "Send system monitoring alerts (CPU, memory, service health) using predefined templates and escalation rules"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("alert_type".to_string(), JsonSchema::string_enum(vec![
                    "high_cpu".to_string(), "high_memory".to_string(), "service_down".to_string(),
                    "disk_space_low".to_string(), "database_slow".to_string()
                ]).with_description("Type of monitoring alert")),
                ("server".to_string(), JsonSchema::string()
                    .with_description("Server or instance name")),
                ("service".to_string(), JsonSchema::string()
                    .with_description("Service name (for service-related alerts)")),
                ("metric_value".to_string(), JsonSchema::number()
                    .with_description("Current metric value (percentage, response time, etc.)")),
                ("threshold".to_string(), JsonSchema::number()
                    .with_description("Alert threshold that was exceeded")),
                ("duration".to_string(), JsonSchema::string()
                    .with_description("How long the condition has persisted")),
                ("additional_context".to_string(), JsonSchema::object()
                    .with_description("Additional context for template substitution")),
            ]))
            .with_required(vec!["alert_type".to_string(), "server".to_string()])
    }

    async fn call(&self, args: Value, _session: Option<SessionContext>) -> McpResult<Vec<ToolResult>> {
        let alert_type = args.get("alert_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("alert_type"))?;
        
        let server = args.get("server")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("server"))?;
            
        let service = args.get("service")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
            
        let metric_value = args.get("metric_value")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
            
        let threshold = args.get("threshold")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
            
        let duration = args.get("duration")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let notification_id = uuid::Uuid::new_v4().to_string();
        let timestamp = Utc::now();
        
        // Get the appropriate monitoring alert template
        if let Some(template) = self.notification_tool.get_notification_template("monitoring", alert_type) {
            // Build variables for template substitution
            let mut variables = HashMap::new();
            variables.insert("server".to_string(), server.to_string());
            variables.insert("service".to_string(), service.to_string());
            variables.insert("duration".to_string(), duration.to_string());
            variables.insert("timestamp".to_string(), timestamp.to_rfc3339());
            
            // Add metric-specific variables
            match alert_type {
                "high_cpu" => {
                    variables.insert("cpu_percentage".to_string(), format!("{:.1}", metric_value));
                },
                "high_memory" => {
                    variables.insert("memory_percentage".to_string(), format!("{:.1}", metric_value));
                },
                "disk_space_low" => {
                    variables.insert("disk_percentage".to_string(), format!("{:.1}", metric_value));
                    variables.insert("filesystem".to_string(), "/var".to_string()); // Could be parameterized
                },
                "database_slow" => {
                    variables.insert("avg_time".to_string(), format!("{:.0}", metric_value));
                    variables.insert("database".to_string(), service.to_string());
                },
                _ => {}
            }
            
            // Add any additional context variables
            if let Some(additional) = args.get("additional_context").and_then(|v| v.as_object()) {
                for (key, value) in additional {
                    if let Some(val_str) = value.as_str() {
                        variables.insert(key.clone(), val_str.to_string());
                    }
                }
            }
            
            let formatted_message = self.notification_tool.format_notification(template, &variables);
            
            // Determine escalation based on priority
            let escalation_rule = match template.priority.as_str() {
                "critical" => self.notification_tool.get_escalation_rule("critical_system"),
                "warning" | "error" => self.notification_tool.get_escalation_rule("default"),
                _ => None,
            };
            
            info!("Sending monitoring alert: {} - {}", alert_type, formatted_message);
            
            Ok(vec![ToolResult::text(json!({
                "notification_id": notification_id,
                "alert_type": alert_type,
                "template_used": format!("monitoring.{}", alert_type),
                "formatted_message": formatted_message,
                "priority": template.priority,
                "channels": template.channels,
                "color": template.color,
                "server": server,
                "metric_value": metric_value,
                "threshold": threshold,
                "escalation_rule": escalation_rule.map(|r| r.description.clone()),
                "timestamp": timestamp.to_rfc3339(),
                "status": "alert_prepared",
                "note": "In production, this would trigger immediate notifications via configured channels and escalation rules"
            }).to_string())])
        } else {
            Err(McpError::tool_execution(&format!("No template found for monitoring alert type: {}", alert_type)))
        }
    }
}

/// Incident management tool for creating and managing incidents with automated workflows
struct IncidentManagementTool {
    notification_tool: DevNotificationTool,
}

impl IncidentManagementTool {
    fn new() -> McpResult<Self> {
        let notification_tool = DevNotificationTool::new()?;
        Ok(Self { notification_tool })
    }
}

#[async_trait]
impl McpTool for IncidentManagementTool {
    fn name(&self) -> &str {
        "declare_incident"
    }

    fn description(&self) -> &str {
        "Declare a new incident and trigger automated notification workflows based on severity and escalation rules"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("title".to_string(), JsonSchema::string()
                    .with_description("Brief incident title")),
                ("description".to_string(), JsonSchema::string()
                    .with_description("Detailed incident description")),
                ("severity".to_string(), JsonSchema::string_enum(vec![
                    "P0".to_string(), "P1".to_string(), "P2".to_string(), "P3".to_string()
                ]).with_description("Incident severity level")),
                ("affected_service".to_string(), JsonSchema::string()
                    .with_description("Primary affected service or system")),
                ("impact".to_string(), JsonSchema::string()
                    .with_description("Description of user/business impact")),
                ("commander".to_string(), JsonSchema::string()
                    .with_description("Incident commander (defaults to on-call engineer)")),
            ]))
            .with_required(vec!["title".to_string(), "severity".to_string()])
    }

    async fn call(&self, args: Value, _session: Option<SessionContext>) -> McpResult<Vec<ToolResult>> {
        let title = args.get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("title"))?;
        
        let severity = args.get("severity")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("severity"))?;
            
        let description = args.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("No additional details provided");
            
        let affected_service = args.get("affected_service")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown service");
            
        let impact = args.get("impact")
            .and_then(|v| v.as_str())
            .unwrap_or("Impact assessment pending");
            
        let commander = args.get("commander")
            .and_then(|v| v.as_str())
            .unwrap_or("On-call Engineer");

        let incident_id = format!("INC-{}", Utc::now().format("%Y%m%d-%H%M%S"));
        let timestamp = Utc::now();
        
        // Determine escalation rule based on severity
        let escalation_rule_name = match severity {
            "P0" => "critical_system",
            "P1" => "default",
            _ => "default",
        };
        
        let escalation_rule = self.notification_tool.get_escalation_rule(escalation_rule_name);
        
        // Get notification priority based on severity
        let priority = match severity {
            "P0" => "critical",
            "P1" => "error", 
            "P2" => "warning",
            "P3" => "info",
            _ => "warning",
        };
        
        info!("Declaring incident {}: {} ({})", incident_id, title, severity);
        
        // In a real implementation, this would:
        // 1. Create incident in tracking system (Jira, PagerDuty, etc.)
        // 2. Create war room (Slack channel, Zoom meeting)
        // 3. Send notifications according to escalation rules
        // 4. Update status page
        // 5. Start automated monitoring and tracking
        
        let war_room_link = format!("https://company.slack.com/channels/incident-{}", 
                                   incident_id.to_lowercase().replace("-", ""));
        
        let notification_summary = json!({
            "incident_declared": {
                "incident_id": incident_id,
                "title": title,
                "severity": severity,
                "priority": priority,
                "affected_service": affected_service,
                "description": description,
                "impact": impact,
                "commander": commander,
                "war_room": war_room_link,
                "declared_at": timestamp.to_rfc3339()
            },
            "automated_actions": {
                "war_room_created": war_room_link,
                "status_page_updated": true,
                "escalation_rule_applied": escalation_rule_name,
                "initial_notifications_sent": true
            },
            "escalation_plan": escalation_rule.map(|rule| {
                json!({
                    "description": rule.description,
                    "stages": rule.stages.iter().map(|stage| {
                        json!({
                            "delay_seconds": stage.delay_seconds,
                            "targets": stage.targets,
                            "channels": stage.channels
                        })
                    }).collect::<Vec<_>>()
                })
            }),
            "workflows_reference": "See data/incident_workflows.md for complete procedures"
        });
        
        Ok(vec![ToolResult::text(json!({
            "incident_id": incident_id,
            "status": "incident_declared",
            "severity": severity,
            "priority": priority,
            "title": title,
            "war_room": war_room_link,
            "commander": commander,
            "escalation_rule": escalation_rule_name,
            "timestamp": timestamp.to_rfc3339(),
            "notification_summary": notification_summary,
            "next_steps": [
                "Join the war room to coordinate response",
                "Begin initial assessment and triage",
                "Update incident status every 15-30 minutes",
                "Escalate if not resolved within SLA timeframes"
            ],
            "note": "In production, this would trigger immediate war room creation, status page updates, and multi-channel notifications based on severity"
        }).to_string())])
    }
}

/// Team information and workflow documentation tool
struct TeamInfoTool {
    notification_tool: DevNotificationTool,
}

impl TeamInfoTool {
    fn new() -> McpResult<Self> {
        let notification_tool = DevNotificationTool::new()?;
        Ok(Self { notification_tool })
    }
}

#[async_trait]
impl McpTool for TeamInfoTool {
    fn name(&self) -> &str {
        "get_team_info"
    }

    fn description(&self) -> &str {
        "Get team contact information, on-call rotations, and incident response workflows loaded from external files"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("info_type".to_string(), JsonSchema::string_enum(vec![
                    "contacts".to_string(), "workflows".to_string(), "templates".to_string(), "all".to_string()
                ]).with_description("Type of team information to retrieve")),
            ]))
            .with_required(vec!["info_type".to_string()])
    }

    async fn call(&self, args: Value, _session: Option<SessionContext>) -> McpResult<Vec<ToolResult>> {
        let info_type = args.get("info_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("info_type"))?;
        
        match info_type {
            "contacts" => {
                Ok(vec![ToolResult::text(format!(
                    "# Team Contacts and On-Call Information\n\n{}\n\n## Loading Source\nLoaded from: data/team_contacts.yaml",
                    self.notification_tool.team_contacts
                ))])
            },
            "workflows" => {
                Ok(vec![ToolResult::text(format!(
                    "{}\n\n## Loading Source\nLoaded from: data/incident_workflows.md",
                    self.notification_tool.incident_workflows
                ))])
            },
            "templates" => {
                let templates_summary = json!({
                    "notification_types": self.notification_tool.templates.notification_types.keys().collect::<Vec<_>>(),
                    "priorities": self.notification_tool.templates.notification_priorities.keys().collect::<Vec<_>>(),
                    "channels": self.notification_tool.templates.notification_channels.keys().collect::<Vec<_>>(),
                    "escalation_rules": self.notification_tool.templates.escalation_rules.keys().collect::<Vec<_>>(),
                    "total_templates": self.notification_tool.templates.notification_types.values()
                        .map(|nt| nt.templates.len()).sum::<usize>(),
                    "source": "data/notification_templates.json"
                });
                Ok(vec![ToolResult::text(serde_json::to_string_pretty(&templates_summary)?)])
            },
            "all" => {
                let summary = json!({
                    "team_contacts": "Available - see contacts info_type",
                    "incident_workflows": "Available - see workflows info_type", 
                    "notification_templates": "Available - see templates info_type",
                    "data_sources": {
                        "contacts": "data/team_contacts.yaml",
                        "workflows": "data/incident_workflows.md",
                        "templates": "data/notification_templates.json"
                    },
                    "capabilities": [
                        "CI/CD pipeline notifications",
                        "System monitoring alerts",
                        "Incident management workflows",
                        "Automated escalation rules",
                        "Multi-channel notifications"
                    ]
                });
                Ok(vec![ToolResult::text(serde_json::to_string_pretty(&summary)?)])
            },
            _ => Err(McpError::invalid_param_type("info_type", "all|contacts|workflows|templates", info_type))
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting Development Team Notification Server");

    let cicd_tool = CiCdNotificationTool::new()?;
    let monitoring_tool = MonitoringNotificationTool::new()?;
    let incident_tool = IncidentManagementTool::new()?;
    let team_info_tool = TeamInfoTool::new()?;

    let server = McpServer::builder()
        .name("dev-team-notifications")
        .version("1.0.0")
        .title("Development Team Notification Server")
        .instructions("Real-world notification server for development teams. Provides CI/CD pipeline alerts, system monitoring notifications, security alerts, and incident management workflows using templates and escalation rules loaded from external files.")
        .tool(cicd_tool)
        .tool(monitoring_tool)
        .tool(incident_tool)
        .tool(team_info_tool)
        .bind_address("127.0.0.1:8005".parse()?)
        .sse(true)
        .build()?;

    info!("Development team notification server running at: http://127.0.0.1:8005/mcp");
    info!("SSE endpoint available at: GET http://127.0.0.1:8005/mcp (Accept: text/event-stream)");
    info!("Real-world development team notifications:");
    info!("  - send_cicd_notification: CI/CD pipeline status (builds, deployments, failures)");
    info!("  - send_monitoring_alert: System monitoring (CPU, memory, service health, disk space)");
    info!("  - declare_incident: Incident management with automated workflows and escalation");
    info!("  - get_team_info: Team contacts, on-call rotations, and workflow documentation");
    info!("External data files: data/notification_templates.json, data/team_contacts.yaml, data/incident_workflows.md");
    info!("Notification channels: Slack, Email, SMS, PagerDuty (configured via templates)");

    server.run().await?;
    Ok(())
}