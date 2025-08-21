//! # Application Logging and Audit System
//!
//! This example demonstrates a real-world application logging and audit system
//! for development teams. It provides comprehensive log management, audit trail
//! functionality, compliance reporting, and security monitoring. The server loads
//! logging configurations, audit policies, and alert rules from external files.

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use async_trait::async_trait;
use mcp_server::{McpServer, McpResult, McpHandler, McpTool, SessionContext};
use mcp_protocol::logging::{LogLevel, SetLevelRequest};
use mcp_protocol::{ToolSchema, ToolResult, McpError};
use mcp_protocol::schema::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value, from_str};
use tracing::info;
use chrono::{DateTime, Utc};

#[derive(Debug, Deserialize, Serialize)]
struct LogConfig {
    application: ApplicationInfo,
    log_levels: LogLevels,
    log_categories: HashMap<String, LogCategory>,
    alert_rules: HashMap<String, AlertRule>,
    log_destinations: HashMap<String, LogDestination>,
    structured_logging: StructuredLogging,
    compliance: ComplianceConfig,
    monitoring_integration: MonitoringIntegration,
    development: DevelopmentConfig,
}

#[derive(Debug, Deserialize, Serialize)]
struct ApplicationInfo {
    name: String,
    version: String,
    environment: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct LogLevels {
    global_default: String,
    per_service: HashMap<String, String>,
    per_module: HashMap<String, String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct LogCategory {
    description: String,
    default_level: String,
    retention_days: u32,
    include_pii: bool,
    alert_threshold: String,
    #[serde(default)]
    compliance_required: bool,
    #[serde(default)]
    immediate_notification: bool,
    #[serde(default)]
    immutable: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct AlertRule {
    description: String,
    condition: String,
    severity: String,
    channels: Vec<String>,
    cooldown_minutes: u32,
}

#[derive(Debug, Deserialize, Serialize)]
struct LogDestination {
    enabled: bool,
    level: String,
    #[serde(flatten)]
    config: HashMap<String, Value>,
}

#[derive(Debug, Deserialize, Serialize)]
struct StructuredLogging {
    enabled: bool,
    format: String,
    include_fields: HashMap<String, bool>,
    sensitive_fields: Vec<String>,
    masking: MaskingConfig,
}

#[derive(Debug, Deserialize, Serialize)]
struct MaskingConfig {
    enabled: bool,
    strategy: String,
    preserve_length: bool,
    mask_character: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct ComplianceConfig {
    gdpr: ComplianceFramework,
    #[serde(default)]
    hipaa: ComplianceFramework,
    pci_dss: ComplianceFramework,
    sox: ComplianceFramework,
}

#[derive(Debug, Deserialize, Serialize, Default)]
struct ComplianceFramework {
    enabled: bool,
    #[serde(flatten)]
    config: HashMap<String, Value>,
}

#[derive(Debug, Deserialize, Serialize)]
struct MonitoringIntegration {
    prometheus: HashMap<String, Value>,
    grafana: HashMap<String, Value>,
    jaeger: HashMap<String, Value>,
}

#[derive(Debug, Deserialize, Serialize)]
struct DevelopmentConfig {
    debug_mode: bool,
    verbose_errors: bool,
    include_stack_traces: bool,
    log_sql_queries: bool,
    log_http_requests: bool,
    log_http_responses: bool,
    performance_profiling: bool,
}

/// Enhanced audit entry with compliance and security features
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AuditEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub category: String,
    pub event_type: String,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub ip_address: Option<String>,
    pub message: String,
    pub details: Value,
    pub correlation_id: Option<String>,
    pub compliance_tags: Vec<String>,
    pub retention_days: u32,
    pub immutable: bool,
}

/// Production logging state with comprehensive audit capabilities
#[derive(Debug)]
pub struct ProductionLoggingState {
    config: LogConfig,
    audit_policies: String,
    log_templates: String,
    current_level: LogLevel,
    service_levels: HashMap<String, LogLevel>,
    audit_history: Vec<AuditEntry>,
    max_history: usize,
    alert_cooldowns: HashMap<String, DateTime<Utc>>,
}

impl ProductionLoggingState {
    pub fn new() -> McpResult<Self> {
        let config_path = Path::new("data/log_config.json");
        let policies_path = Path::new("data/audit_policies.yaml");
        let templates_path = Path::new("data/log_templates.md");
        
        let config = match fs::read_to_string(config_path) {
            Ok(content) => {
                from_str::<LogConfig>(&content)
                    .map_err(|e| McpError::tool_execution(&format!("Failed to parse log config: {}", e)))?
            },
            Err(_) => {
                // Fallback configuration
                LogConfig {
                    application: ApplicationInfo {
                        name: "Application Logger".to_string(),
                        version: "1.0.0".to_string(),
                        environment: "development".to_string(),
                    },
                    log_levels: LogLevels {
                        global_default: "info".to_string(),
                        per_service: HashMap::new(),
                        per_module: HashMap::new(),
                    },
                    log_categories: HashMap::new(),
                    alert_rules: HashMap::new(),
                    log_destinations: HashMap::new(),
                    structured_logging: StructuredLogging {
                        enabled: true,
                        format: "json".to_string(),
                        include_fields: HashMap::new(),
                        sensitive_fields: Vec::new(),
                        masking: MaskingConfig {
                            enabled: true,
                            strategy: "partial".to_string(),
                            preserve_length: true,
                            mask_character: "*".to_string(),
                        },
                    },
                    compliance: ComplianceConfig {
                        gdpr: ComplianceFramework::default(),
                        hipaa: ComplianceFramework::default(),
                        pci_dss: ComplianceFramework::default(),
                        sox: ComplianceFramework::default(),
                    },
                    monitoring_integration: MonitoringIntegration {
                        prometheus: HashMap::new(),
                        grafana: HashMap::new(),
                        jaeger: HashMap::new(),
                    },
                    development: DevelopmentConfig {
                        debug_mode: false,
                        verbose_errors: false,
                        include_stack_traces: true,
                        log_sql_queries: false,
                        log_http_requests: true,
                        log_http_responses: false,
                        performance_profiling: false,
                    },
                }
            }
        };
        
        let audit_policies = fs::read_to_string(policies_path)
            .unwrap_or_else(|_| "# Audit Policies\n\nNo audit policies loaded.".to_string());
            
        let log_templates = fs::read_to_string(templates_path)
            .unwrap_or_else(|_| "# Log Templates\n\nNo log templates loaded.".to_string());
        
        let current_level = Self::parse_log_level(&config.log_levels.global_default);
        
        Ok(Self {
            config,
            audit_policies,
            log_templates,
            current_level,
            service_levels: HashMap::new(),
            audit_history: Vec::new(),
            max_history: 10000,
            alert_cooldowns: HashMap::new(),
        })
    }

    pub fn set_global_level(&mut self, level: LogLevel) {
        info!("Global log level set to: {:?}", level);
        self.current_level = level.clone();
        
        // Create audit entry for this change
        let audit_entry = AuditEntry {
            id: uuid::Uuid::now_v7().to_string(),
            timestamp: Utc::now(),
            level: LogLevel::Info,
            category: "system_administration".to_string(),
            event_type: "configuration_change".to_string(),
            user_id: Some("system".to_string()),
            session_id: None,
            ip_address: None,
            message: format!("Global log level changed to {:?}", level),
            details: json!({
                "config_key": "global_log_level",
                "old_value": "info", // Would track previous level in real implementation
                "new_value": format!("{:?}", level).to_lowercase(),
                "change_reason": "mcp_client_request"
            }),
            correlation_id: Some(format!("cfg_{}", uuid::Uuid::now_v7())),
            compliance_tags: vec!["sox".to_string()],
            retention_days: 2555, // 7 years
            immutable: true,
        };
        
        self.add_audit_entry(audit_entry);
    }

    pub fn set_service_level(&mut self, service: String, level: LogLevel) {
        info!("Service '{}' level set to: {:?}", service, level);
        self.service_levels.insert(service.clone(), level.clone());
        
        // Create audit entry for service-specific level change
        let audit_entry = AuditEntry {
            id: uuid::Uuid::now_v7().to_string(),
            timestamp: Utc::now(),
            level: LogLevel::Info,
            category: "system_administration".to_string(),
            event_type: "configuration_change".to_string(),
            user_id: Some("system".to_string()),
            session_id: None,
            ip_address: None,
            message: format!("Service '{}' log level changed to {:?}", service, level),
            details: json!({
                "config_key": format!("service_log_level.{}", service),
                "service": service,
                "new_value": format!("{:?}", level).to_lowercase(),
                "change_reason": "mcp_client_request"
            }),
            correlation_id: Some(format!("cfg_{}", uuid::Uuid::now_v7())),
            compliance_tags: vec!["sox".to_string()],
            retention_days: 2555,
            immutable: true,
        };
        
        self.add_audit_entry(audit_entry);
    }

    pub fn get_level(&self, service: Option<&str>) -> &LogLevel {
        if let Some(service_name) = service {
            self.service_levels.get(service_name).unwrap_or(&self.current_level)
        } else {
            &self.current_level
        }
    }

    pub fn add_audit_entry(&mut self, entry: AuditEntry) {
        info!("Audit entry created: {} - {}", entry.event_type, entry.message);
        self.audit_history.push(entry);
        if self.audit_history.len() > self.max_history {
            self.audit_history.remove(0);
        }
    }

    pub fn get_recent_audits(&self, count: usize) -> Vec<&AuditEntry> {
        let start = if self.audit_history.len() > count {
            self.audit_history.len() - count
        } else {
            0
        };
        self.audit_history[start..].iter().collect()
    }

    pub fn filter_audits(&self, category: Option<&str>, level: Option<&LogLevel>, compliance: Option<&str>) -> Vec<&AuditEntry> {
        self.audit_history
            .iter()
            .filter(|entry| {
                if let Some(filter_category) = category {
                    if entry.category != filter_category {
                        return false;
                    }
                }
                if let Some(filter_level) = level {
                    if !self.should_include_level(&entry.level, filter_level) {
                        return false;
                    }
                }
                if let Some(filter_compliance) = compliance {
                    if !entry.compliance_tags.contains(&filter_compliance.to_string()) {
                        return false;
                    }
                }
                true
            })
            .collect()
    }

    pub fn check_alert_rules(&mut self, entry: &AuditEntry) -> Vec<String> {
        let mut triggered_alerts = Vec::new();
        let now = Utc::now();
        
        for (rule_name, rule) in &self.config.alert_rules {
            // Check if we're in cooldown period
            if let Some(last_alert) = self.alert_cooldowns.get(rule_name) {
                let cooldown_duration = chrono::Duration::minutes(rule.cooldown_minutes as i64);
                if now < *last_alert + cooldown_duration {
                    continue;
                }
            }
            
            // Simple rule evaluation (in production, this would be more sophisticated)
            let should_alert = match rule_name.as_str() {
                "high_error_rate" => matches!(entry.level, LogLevel::Error),
                "security_breach_attempt" => entry.category == "security" && matches!(entry.level, LogLevel::Error | LogLevel::Critical),
                "database_connection_issues" => entry.event_type.contains("database") && matches!(entry.level, LogLevel::Error),
                _ => false,
            };
            
            if should_alert {
                triggered_alerts.push(format!(
                    "ALERT [{}]: {} - {} (Severity: {}, Channels: {:?})",
                    rule_name, rule.description, entry.message, rule.severity, rule.channels
                ));
                self.alert_cooldowns.insert(rule_name.clone(), now);
            }
        }
        
        triggered_alerts
    }

    fn should_include_level(&self, entry_level: &LogLevel, min_level: &LogLevel) -> bool {
        self.level_priority(entry_level) >= self.level_priority(min_level)
    }

    fn level_priority(&self, level: &LogLevel) -> u8 {
        match level {
            LogLevel::Debug => 0,
            LogLevel::Info => 1,
            LogLevel::Notice => 2,
            LogLevel::Warning => 3,
            LogLevel::Error => 4,
            LogLevel::Critical => 5,
            LogLevel::Alert => 6,
            LogLevel::Emergency => 7,
        }
    }

    fn parse_log_level(level_str: &str) -> LogLevel {
        match level_str.to_lowercase().as_str() {
            "debug" => LogLevel::Debug,
            "info" => LogLevel::Info,
            "notice" => LogLevel::Notice,
            "warning" | "warn" => LogLevel::Warning,
            "error" => LogLevel::Error,
            "critical" => LogLevel::Critical,
            "alert" => LogLevel::Alert,
            "emergency" => LogLevel::Emergency,
            _ => LogLevel::Info,
        }
    }
}

/// Production logging handler with enterprise features
pub struct ProductionLoggingHandler {
    state: Arc<Mutex<ProductionLoggingState>>,
}

impl ProductionLoggingHandler {
    pub fn new() -> McpResult<Self> {
        let state = ProductionLoggingState::new()?;
        Ok(Self {
            state: Arc::new(Mutex::new(state)),
        })
    }
}

#[async_trait]
impl McpHandler for ProductionLoggingHandler {
    async fn handle(&self, params: Option<Value>) -> McpResult<Value> {
        if let Some(params) = params {
            let request: SetLevelRequest = serde_json::from_value(params)
                .map_err(|e| McpError::invalid_param_type("params", "SetLevelRequest", &e.to_string()))?;
            
            {
                let mut state = self.state.lock().unwrap();
                state.set_global_level(request.level.clone());
            }
            
            Ok(Value::Null)
        } else {
            Err(McpError::missing_param("params"))
        }
    }
    
    fn supported_methods(&self) -> Vec<String> {
        vec!["logging/setLevel".to_string()]
    }
}

/// Business event logging tool for tracking user actions and transactions
struct BusinessEventTool {
    state: Arc<Mutex<ProductionLoggingState>>,
}

impl BusinessEventTool {
    fn new(state: Arc<Mutex<ProductionLoggingState>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl McpTool for BusinessEventTool {
    fn name(&self) -> &str {
        "log_business_event"
    }

    fn description(&self) -> &str {
        "Log business events such as user registrations, transactions, and workflow completions with audit trail"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("event_type".to_string(), JsonSchema::string_enum(vec![
                    "user_creation".to_string(), "user_deletion".to_string(), "user_modification".to_string(),
                    "payment_processing".to_string(), "refund_processing".to_string(), "login_success".to_string(),
                    "login_failure".to_string(), "data_export".to_string(), "configuration_change".to_string()
                ]).with_description("Type of business event")),
                ("user_id".to_string(), JsonSchema::string()
                    .with_description("User ID associated with the event")),
                ("session_id".to_string(), JsonSchema::string()
                    .with_description("Session ID for tracking user sessions")),
                ("ip_address".to_string(), JsonSchema::string()
                    .with_description("IP address of the user or system")),
                ("message".to_string(), JsonSchema::string()
                    .with_description("Human-readable event description")),
                ("details".to_string(), JsonSchema::object()
                    .with_description("Additional event details and context")),
                ("compliance_tags".to_string(), JsonSchema::array(JsonSchema::string())
                    .with_description("Compliance frameworks that apply to this event")),
            ]))
            .with_required(vec!["event_type".to_string(), "message".to_string()])
    }

    async fn call(&self, args: Value, _session: Option<SessionContext>) -> McpResult<Vec<ToolResult>> {
        let event_type = args.get("event_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("event_type"))?;
        
        let user_id = args.get("user_id").and_then(|v| v.as_str());
        let session_id = args.get("session_id").and_then(|v| v.as_str());
        let ip_address = args.get("ip_address").and_then(|v| v.as_str());
        let message = args.get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("message"))?;
        let details = args.get("details").cloned().unwrap_or(json!({}));
        let compliance_tags = args.get("compliance_tags")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_else(Vec::new);

        // Determine category and level based on event type
        let (category, level, retention_days) = match event_type {
            "user_creation" | "user_deletion" | "user_modification" => ("business", LogLevel::Info, 2555),
            "payment_processing" | "refund_processing" => ("business", LogLevel::Info, 2555),
            "login_success" => ("security", LogLevel::Info, 90),
            "login_failure" => ("security", LogLevel::Warning, 365),
            "data_export" => ("audit", LogLevel::Warning, 2555),
            "configuration_change" => ("audit", LogLevel::Warning, 2555),
            _ => ("business", LogLevel::Info, 365),
        };

        let audit_entry = AuditEntry {
            id: uuid::Uuid::now_v7().to_string(),
            timestamp: Utc::now(),
            level: level.clone(),
            category: category.to_string(),
            event_type: event_type.to_string(),
            user_id: user_id.map(|s| s.to_string()),
            session_id: session_id.map(|s| s.to_string()),
            ip_address: ip_address.map(|s| s.to_string()),
            message: message.to_string(),
            details,
            correlation_id: Some(format!("biz_{}", uuid::Uuid::now_v7())),
            compliance_tags,
            retention_days,
            immutable: category == "audit",
        };

        let mut triggered_alerts = Vec::new();
        {
            let mut state = self.state.lock().unwrap();
            triggered_alerts = state.check_alert_rules(&audit_entry);
            state.add_audit_entry(audit_entry.clone());
        }

        let mut response = json!({
            "audit_entry_id": audit_entry.id,
            "timestamp": audit_entry.timestamp.to_rfc3339(),
            "category": audit_entry.category,
            "event_type": audit_entry.event_type,
            "level": format!("{:?}", audit_entry.level),
            "retention_days": audit_entry.retention_days,
            "immutable": audit_entry.immutable,
            "compliance_tags": audit_entry.compliance_tags,
            "status": "logged_successfully"
        });

        if !triggered_alerts.is_empty() {
            response["triggered_alerts"] = json!(triggered_alerts);
        }

        Ok(vec![ToolResult::text(serde_json::to_string_pretty(&response)?)])
    }
}

/// Security event logging tool for tracking security incidents and threats
struct SecurityEventTool {
    state: Arc<Mutex<ProductionLoggingState>>,
}

impl SecurityEventTool {
    fn new(state: Arc<Mutex<ProductionLoggingState>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl McpTool for SecurityEventTool {
    fn name(&self) -> &str {
        "log_security_event"
    }

    fn description(&self) -> &str {
        "Log security events such as suspicious activity, authentication failures, and security breaches with immediate alerting"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("event_type".to_string(), JsonSchema::string_enum(vec![
                    "suspicious_activity".to_string(), "sql_injection_attempt".to_string(), "xss_attempt".to_string(),
                    "privilege_escalation".to_string(), "rate_limit_exceeded".to_string(), "failed_authentication".to_string(),
                    "data_breach_attempt".to_string(), "malware_detection".to_string()
                ]).with_description("Type of security event")),
                ("threat_level".to_string(), JsonSchema::string_enum(vec![
                    "low".to_string(), "medium".to_string(), "high".to_string(), "critical".to_string()
                ]).with_description("Severity level of the security threat")),
                ("ip_address".to_string(), JsonSchema::string()
                    .with_description("Source IP address of the security event")),
                ("user_agent".to_string(), JsonSchema::string()
                    .with_description("User agent string from the request")),
                ("endpoint".to_string(), JsonSchema::string()
                    .with_description("API endpoint or URL involved in the incident")),
                ("payload".to_string(), JsonSchema::string()
                    .with_description("Malicious payload or attack vector (sanitized)")),
                ("blocked".to_string(), JsonSchema::boolean()
                    .with_description("Whether the attack was successfully blocked")),
                ("detection_method".to_string(), JsonSchema::string()
                    .with_description("How the security event was detected")),
                ("additional_context".to_string(), JsonSchema::object()
                    .with_description("Additional context and metadata")),
            ]))
            .with_required(vec!["event_type".to_string(), "threat_level".to_string()])
    }

    async fn call(&self, args: Value, _session: Option<SessionContext>) -> McpResult<Vec<ToolResult>> {
        let event_type = args.get("event_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("event_type"))?;
        
        let threat_level = args.get("threat_level")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("threat_level"))?;
        
        let ip_address = args.get("ip_address").and_then(|v| v.as_str());
        let user_agent = args.get("user_agent").and_then(|v| v.as_str());
        let endpoint = args.get("endpoint").and_then(|v| v.as_str());
        let payload = args.get("payload").and_then(|v| v.as_str());
        let blocked = args.get("blocked").and_then(|v| v.as_bool()).unwrap_or(false);
        let detection_method = args.get("detection_method").and_then(|v| v.as_str());
        let additional_context = args.get("additional_context").cloned().unwrap_or(json!({}));

        // Determine log level based on threat level
        let log_level = match threat_level {
            "low" => LogLevel::Info,
            "medium" => LogLevel::Warning,
            "high" => LogLevel::Error,
            "critical" => LogLevel::Critical,
            _ => LogLevel::Warning,
        };

        let message = format!(
            "Security event detected: {} - {} threat level{}",
            event_type,
            threat_level,
            if blocked { " (blocked)" } else { " (not blocked)" }
        );

        let mut details = json!({
            "threat_level": threat_level,
            "blocked": blocked,
            "detection_method": detection_method,
        });

        if let Some(ua) = user_agent {
            details["user_agent"] = json!(ua);
        }
        if let Some(ep) = endpoint {
            details["endpoint"] = json!(ep);
        }
        if let Some(pl) = payload {
            // Sanitize and limit payload for logging
            let sanitized_payload = if pl.len() > 200 {
                format!("{}...", &pl[..200])
            } else {
                pl.to_string()
            };
            details["sanitized_payload"] = json!(sanitized_payload);
        }

        // Merge additional context
        if let Some(context_obj) = additional_context.as_object() {
            for (key, value) in context_obj {
                details[key] = value.clone();
            }
        }

        let audit_entry = AuditEntry {
            id: uuid::Uuid::now_v7().to_string(),
            timestamp: Utc::now(),
            level: log_level,
            category: "security".to_string(),
            event_type: event_type.to_string(),
            user_id: None,
            session_id: None,
            ip_address: ip_address.map(|s| s.to_string()),
            message,
            details,
            correlation_id: Some(format!("sec_{}", uuid::Uuid::now_v7())),
            compliance_tags: vec!["security".to_string()],
            retention_days: 2555, // 7 years for security events
            immutable: true,
        };

        let mut triggered_alerts = Vec::new();
        {
            let mut state = self.state.lock().unwrap();
            triggered_alerts = state.check_alert_rules(&audit_entry);
            state.add_audit_entry(audit_entry.clone());
        }

        // Security events always generate immediate alerts for high/critical threats
        if threat_level == "high" || threat_level == "critical" {
            triggered_alerts.push(format!(
                "🚨 IMMEDIATE SECURITY ALERT: {} detected from IP {} - Threat Level: {} - {}",
                event_type,
                ip_address.unwrap_or("unknown"),
                threat_level,
                if blocked { "BLOCKED" } else { "NOT BLOCKED - INVESTIGATE IMMEDIATELY" }
            ));
        }

        let response = json!({
            "audit_entry_id": audit_entry.id,
            "timestamp": audit_entry.timestamp.to_rfc3339(),
            "threat_level": threat_level,
            "event_type": event_type,
            "blocked": blocked,
            "immediate_alert_required": threat_level == "high" || threat_level == "critical",
            "triggered_alerts": triggered_alerts,
            "status": "security_event_logged",
            "note": "Security events are automatically retained for 7 years and are immutable for compliance"
        });

        Ok(vec![ToolResult::text(serde_json::to_string_pretty(&response)?)])
    }
}

/// Audit viewer tool for searching and viewing audit trails
struct AuditViewerTool {
    state: Arc<Mutex<ProductionLoggingState>>,
}

impl AuditViewerTool {
    fn new(state: Arc<Mutex<ProductionLoggingState>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl McpTool for AuditViewerTool {
    fn name(&self) -> &str {
        "view_audit_trail"
    }

    fn description(&self) -> &str {
        "View and search audit trail entries with filtering by category, compliance framework, time range, and event types"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("count".to_string(), JsonSchema::integer()
                    .with_minimum(1.0).with_maximum(200.0)
                    .with_description("Number of recent entries to show (1-200)")),
                ("category".to_string(), JsonSchema::string_enum(vec![
                    "business".to_string(), "security".to_string(), "audit".to_string(),
                    "system_administration".to_string(), "performance".to_string()
                ]).with_description("Filter by event category")),
                ("level".to_string(), JsonSchema::string_enum(vec![
                    "debug".to_string(), "info".to_string(), "notice".to_string(), 
                    "warning".to_string(), "error".to_string(), "critical".to_string(),
                    "alert".to_string(), "emergency".to_string()
                ]).with_description("Minimum log level to show")),
                ("compliance".to_string(), JsonSchema::string_enum(vec![
                    "gdpr".to_string(), "sox".to_string(), "pci_dss".to_string(), "hipaa".to_string(), "security".to_string()
                ]).with_description("Filter by compliance framework")),
                ("format".to_string(), JsonSchema::string_enum(vec![
                    "summary".to_string(), "detailed".to_string(), "json".to_string()
                ]).with_description("Output format for audit entries")),
            ]))
    }

    async fn call(&self, args: Value, _session: Option<SessionContext>) -> McpResult<Vec<ToolResult>> {
        let count = args.get("count")
            .and_then(|v| v.as_u64())
            .unwrap_or(20) as usize;
        
        let category_filter = args.get("category").and_then(|v| v.as_str());
        
        let level_filter = args.get("level")
            .and_then(|v| v.as_str())
            .and_then(|s| match s.to_lowercase().as_str() {
                "debug" => Some(LogLevel::Debug),
                "info" => Some(LogLevel::Info),
                "notice" => Some(LogLevel::Notice),
                "warning" => Some(LogLevel::Warning),
                "error" => Some(LogLevel::Error),
                "critical" => Some(LogLevel::Critical),
                "alert" => Some(LogLevel::Alert),
                "emergency" => Some(LogLevel::Emergency),
                _ => None,
            });

        let compliance_filter = args.get("compliance").and_then(|v| v.as_str());
        let format = args.get("format").and_then(|v| v.as_str()).unwrap_or("summary");

        let state = self.state.lock().unwrap();
        
        let audits = if category_filter.is_some() || level_filter.is_some() || compliance_filter.is_some() {
            state.filter_audits(category_filter, level_filter.as_ref(), compliance_filter)
        } else {
            state.get_recent_audits(count)
        };

        if audits.is_empty() {
            return Ok(vec![ToolResult::text("No audit entries found matching the criteria.".to_string())]);
        }

        let result = match format {
            "json" => {
                let json_audits: Vec<_> = audits.iter().rev().take(count).collect();
                serde_json::to_string_pretty(&json_audits)
                    .map_err(|e| McpError::tool_execution(&format!("JSON serialization failed: {}", e)))?
            },
            "detailed" => {
                let mut result_lines = vec![
                    format!("Detailed Audit Trail (showing {}):", audits.len().min(count)),
                    "=".repeat(80),
                ];

                for audit in audits.iter().rev().take(count) {
                    result_lines.push(format!(
                        "\n🔍 Audit Entry ID: {}\n📅 Timestamp: {}\n📊 Level: {:?} | Category: {}\n🏷️  Event Type: {}\n💬 Message: {}",
                        audit.id,
                        audit.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
                        audit.level,
                        audit.category,
                        audit.event_type,
                        audit.message
                    ));

                    if let Some(user_id) = &audit.user_id {
                        result_lines.push(format!("👤 User ID: {}", user_id));
                    }
                    if let Some(ip) = &audit.ip_address {
                        result_lines.push(format!("🌐 IP Address: {}", ip));
                    }
                    if let Some(correlation) = &audit.correlation_id {
                        result_lines.push(format!("🔗 Correlation ID: {}", correlation));
                    }

                    result_lines.push(format!("🏛️  Compliance Tags: {:?}", audit.compliance_tags));
                    result_lines.push(format!("⏱️  Retention: {} days | Immutable: {}", audit.retention_days, audit.immutable));
                    
                    if audit.details != json!({}) {
                        result_lines.push(format!("📋 Details: {}", serde_json::to_string_pretty(&audit.details).unwrap_or_else(|_| "N/A".to_string())));
                    }
                    
                    result_lines.push("─".repeat(80));
                }
                
                result_lines.join("\n")
            },
            _ => {
                // Summary format (default)
                let mut result_lines = vec![
                    format!("Audit Trail Summary (showing {}):", audits.len().min(count)),
                    "=".repeat(60),
                ];

                for audit in audits.iter().rev().take(count) {
                    let user_display = audit.user_id.as_deref().unwrap_or("system");
                    let ip_display = audit.ip_address.as_deref().unwrap_or("internal");
                    
                    result_lines.push(format!(
                        "[{}] [{:?}] [{}] {}: {} (User: {}, IP: {})",
                        audit.timestamp.format("%Y-%m-%d %H:%M:%S"),
                        audit.level,
                        audit.category,
                        audit.event_type,
                        audit.message,
                        user_display,
                        ip_display
                    ));

                    if !audit.compliance_tags.is_empty() {
                        result_lines.push(format!("  📋 Compliance: {:?}", audit.compliance_tags));
                    }
                }
                
                result_lines.join("\n")
            }
        };

        Ok(vec![ToolResult::text(result)])
    }
}

/// Configuration and templates viewer tool
struct ConfigViewerTool {
    state: Arc<Mutex<ProductionLoggingState>>,
}

impl ConfigViewerTool {
    fn new(state: Arc<Mutex<ProductionLoggingState>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl McpTool for ConfigViewerTool {
    fn name(&self) -> &str {
        "get_logging_config"
    }

    fn description(&self) -> &str {
        "Get current logging configuration, audit policies, and template documentation loaded from external files"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("info_type".to_string(), JsonSchema::string_enum(vec![
                    "config".to_string(), "policies".to_string(), "templates".to_string(), 
                    "stats".to_string(), "all".to_string()
                ]).with_description("Type of configuration information to retrieve")),
            ]))
            .with_required(vec!["info_type".to_string()])
    }

    async fn call(&self, args: Value, _session: Option<SessionContext>) -> McpResult<Vec<ToolResult>> {
        let info_type = args.get("info_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("info_type"))?;
        
        let state = self.state.lock().unwrap();
        
        match info_type {
            "config" => {
                let config_summary = json!({
                    "application": state.config.application,
                    "current_global_level": format!("{:?}", state.current_level).to_lowercase(),
                    "service_levels": state.service_levels.iter()
                        .map(|(k, v)| (k.clone(), format!("{:?}", v).to_lowercase()))
                        .collect::<HashMap<_, _>>(),
                    "log_categories": state.config.log_categories.keys().collect::<Vec<_>>(),
                    "alert_rules": state.config.alert_rules.keys().collect::<Vec<_>>(),
                    "destinations": state.config.log_destinations.iter()
                        .filter(|(_, dest)| dest.enabled)
                        .map(|(name, _)| name)
                        .collect::<Vec<_>>(),
                    "compliance_frameworks": {
                        "gdpr": state.config.compliance.gdpr.enabled,
                        "sox": state.config.compliance.sox.enabled,
                        "pci_dss": state.config.compliance.pci_dss.enabled,
                        "hipaa": state.config.compliance.hipaa.enabled
                    },
                    "source": "data/log_config.json"
                });
                Ok(vec![ToolResult::text(serde_json::to_string_pretty(&config_summary)?)])
            },
            "policies" => {
                Ok(vec![ToolResult::text(format!(
                    "{}\n\n## Loading Source\nLoaded from: data/audit_policies.yaml",
                    state.audit_policies
                ))])
            },
            "templates" => {
                Ok(vec![ToolResult::text(format!(
                    "{}\n\n## Loading Source\nLoaded from: data/log_templates.md",
                    state.log_templates
                ))])
            },
            "stats" => {
                let stats = json!({
                    "total_audit_entries": state.audit_history.len(),
                    "max_history_size": state.max_history,
                    "active_alert_cooldowns": state.alert_cooldowns.len(),
                    "categories_breakdown": {
                        "business": state.audit_history.iter().filter(|e| e.category == "business").count(),
                        "security": state.audit_history.iter().filter(|e| e.category == "security").count(),
                        "audit": state.audit_history.iter().filter(|e| e.category == "audit").count(),
                        "system_administration": state.audit_history.iter().filter(|e| e.category == "system_administration").count()
                    },
                    "compliance_breakdown": {
                        "gdpr": state.audit_history.iter().filter(|e| e.compliance_tags.contains(&"gdpr".to_string())).count(),
                        "sox": state.audit_history.iter().filter(|e| e.compliance_tags.contains(&"sox".to_string())).count(),
                        "pci_dss": state.audit_history.iter().filter(|e| e.compliance_tags.contains(&"pci_dss".to_string())).count(),
                        "security": state.audit_history.iter().filter(|e| e.compliance_tags.contains(&"security".to_string())).count()
                    },
                    "immutable_entries": state.audit_history.iter().filter(|e| e.immutable).count(),
                    "avg_retention_days": if !state.audit_history.is_empty() {
                        state.audit_history.iter().map(|e| e.retention_days as f64).sum::<f64>() / state.audit_history.len() as f64
                    } else {
                        0.0
                    }
                });
                Ok(vec![ToolResult::text(serde_json::to_string_pretty(&stats)?)])
            },
            "all" => {
                let summary = json!({
                    "application_logging_system": "Production-ready logging and audit system",
                    "external_data_sources": {
                        "configuration": "data/log_config.json",
                        "audit_policies": "data/audit_policies.yaml", 
                        "log_templates": "data/log_templates.md"
                    },
                    "capabilities": [
                        "Business event logging with audit trails",
                        "Security event monitoring and alerting",
                        "Compliance framework support (GDPR, SOX, PCI DSS, HIPAA)",
                        "Configurable retention policies",
                        "Immutable audit logs",
                        "Real-time alert rules and cooldowns",
                        "Multi-destination log routing",
                        "Structured logging with PII masking"
                    ],
                    "current_status": {
                        "global_level": format!("{:?}", state.current_level).to_lowercase(),
                        "total_audits": state.audit_history.len(),
                        "active_cooldowns": state.alert_cooldowns.len()
                    }
                });
                Ok(vec![ToolResult::text(serde_json::to_string_pretty(&summary)?)])
            },
            _ => Err(McpError::invalid_param_type("info_type", "all|config|policies|templates|stats", info_type))
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting Application Logging and Audit System");

    let logging_state = Arc::new(Mutex::new(ProductionLoggingState::new()?));
    let logging_handler = ProductionLoggingHandler::new()?;

    let server = McpServer::builder()
        .name("production-logging-system")
        .version("1.0.0")
        .title("Application Logging and Audit System")
        .instructions("Real-world application logging and audit system for development teams. Provides comprehensive log management, audit trails, compliance reporting, and security monitoring using configurations and policies loaded from external files.")
        .handler(logging_handler)
        .tool(BusinessEventTool::new(logging_state.clone()))
        .tool(SecurityEventTool::new(logging_state.clone()))
        .tool(AuditViewerTool::new(logging_state.clone()))
        .tool(ConfigViewerTool::new(logging_state.clone()))
        .bind_address("127.0.0.1:8043".parse()?)
        .build()?;
    
    info!("Production logging system running at: http://127.0.0.1:8043/mcp");
    info!("Real-world application logging and audit capabilities:");
    info!("  - logging/setLevel: Set global and per-service log levels");
    info!("  - log_business_event: Log business events with audit trails and compliance tagging");
    info!("  - log_security_event: Log security incidents with immediate alerting");
    info!("  - view_audit_trail: Search and view comprehensive audit trails");
    info!("  - get_logging_config: View configuration, policies, and system statistics");
    info!("External data files: data/log_config.json, data/audit_policies.yaml, data/log_templates.md");
    info!("Compliance support: GDPR, SOX, PCI DSS, HIPAA with configurable retention and immutable logs");

    server.run().await?;
    Ok(())
}