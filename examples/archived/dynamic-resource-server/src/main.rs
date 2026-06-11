//! # Enterprise API Data Gateway Server
//!
//! This server provides a unified data access layer for enterprise systems, enabling
//! seamless integration across multiple APIs, databases, and third-party services.
//! It demonstrates real-world patterns for API orchestration, data transformation,
//! and enterprise system integration.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;
use tracing::{info, warn};
use turul_mcp_derive::McpTool;
use turul_mcp_protocol::{McpError, McpResult};
use turul_mcp_server::prelude::*;
use turul_mcp_server::{McpServer, SessionContext};

// Module-level static for shared gateway state
static GATEWAY: OnceLock<EnterpriseApiGateway> = OnceLock::new();

fn get_gateway() -> McpResult<&'static EnterpriseApiGateway> {
    GATEWAY
        .get()
        .ok_or_else(|| McpError::tool_execution("Gateway not initialized"))
}

/// Configuration for enterprise API endpoints loaded from external files
#[derive(Debug, Deserialize, Serialize, Clone)]
struct EnterpriseApiConfig {
    enterprise_apis: HashMap<String, ApiService>,
    third_party_integrations: HashMap<String, ThirdPartyService>,
    data_transformations: HashMap<String, DataTransformation>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ApiService {
    name: String,
    base_url: String,
    authentication: AuthConfig,
    rate_limits: RateLimits,
    endpoints: Vec<ApiEndpoint>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ThirdPartyService {
    name: String,
    #[serde(rename = "type")]
    service_type: String,
    base_url: String,
    authentication: AuthConfig,
    endpoints: Vec<ApiEndpoint>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct DataTransformation {
    name: String,
    description: String,
    sources: Vec<String>,
    transformations: Vec<TransformationRule>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct TransformationRule {
    field: String,
    #[serde(flatten)]
    rule: Value,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct AuthConfig {
    #[serde(rename = "type")]
    auth_type: String,
    #[serde(flatten)]
    config: Value,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct RateLimits {
    requests_per_minute: u32,
    burst_limit: u32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ApiEndpoint {
    id: String,
    method: String,
    path: String,
    description: String,
    #[serde(default)]
    parameters: HashMap<String, ParameterSpec>,
    #[serde(default)]
    response_schema: Value,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ParameterSpec {
    #[serde(rename = "type")]
    param_type: String,
    required: bool,
    #[serde(flatten)]
    spec: Value,
}

#[derive(Debug, Deserialize, Clone)]
struct DataSources {
    data_sources: DataSourceConfig,
    access_patterns: AccessPatterns,
}

#[derive(Debug, Deserialize, Clone)]
struct DataSourceConfig {
    databases: HashMap<String, DatabaseConfig>,
    data_warehouses: HashMap<String, DataWarehouseConfig>,
    streaming_sources: HashMap<String, StreamingConfig>,
    #[allow(dead_code)]
    file_systems: HashMap<String, FileSystemConfig>,
    #[allow(dead_code)]
    monitoring_systems: HashMap<String, MonitoringConfig>,
}

#[derive(Debug, Deserialize, Clone)]
struct DatabaseConfig {
    #[allow(dead_code)]
    name: String,
    #[serde(rename = "type")]
    db_type: String,
    connection: ConnectionConfig,
    schemas: HashMap<String, SchemaConfig>,
}

#[derive(Debug, Deserialize, Clone)]
struct DataWarehouseConfig {
    #[allow(dead_code)]
    name: String,
    #[serde(rename = "type")]
    dwh_type: String,
    #[allow(dead_code)]
    connection: Value,
    fact_tables: HashMap<String, TableConfig>,
    dimension_tables: HashMap<String, TableConfig>,
}

#[derive(Debug, Deserialize, Clone)]
struct StreamingConfig {
    name: String,
    #[allow(dead_code)]
    connection: Value,
    #[allow(dead_code)]
    #[serde(flatten)]
    config: Value,
}

#[derive(Debug, Deserialize, Clone)]
struct FileSystemConfig {
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    #[serde(rename = "type")]
    fs_type: String,
    #[allow(dead_code)]
    connection: Value,
    #[allow(dead_code)]
    #[serde(flatten)]
    config: Value,
}

#[derive(Debug, Deserialize, Clone)]
struct MonitoringConfig {
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    #[serde(rename = "type")]
    monitoring_type: String,
    #[allow(dead_code)]
    connection: Value,
    #[allow(dead_code)]
    #[serde(flatten)]
    config: Value,
}

#[derive(Debug, Deserialize, Clone)]
struct ConnectionConfig {
    host: String,
    #[allow(dead_code)]
    port: u16,
    database: String,
    ssl: bool,
    #[allow(dead_code)]
    #[serde(flatten)]
    extra: Value,
}

#[derive(Debug, Deserialize, Clone)]
struct SchemaConfig {
    #[allow(dead_code)]
    table: String,
    #[allow(dead_code)]
    primary_key: Value,
    #[allow(dead_code)]
    fields: HashMap<String, String>,
    #[allow(dead_code)]
    #[serde(default)]
    indexes: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct TableConfig {
    #[allow(dead_code)]
    description: String,
    #[allow(dead_code)]
    #[serde(flatten)]
    config: Value,
}

#[derive(Debug, Deserialize, Clone)]
struct AccessPatterns {
    data_governance: DataGovernance,
    security_controls: SecurityControls,
    #[allow(dead_code)]
    performance_optimization: PerformanceOptimization,
}

#[derive(Debug, Deserialize, Clone)]
struct DataGovernance {
    classification_levels: Vec<String>,
    data_lineage_tracking: bool,
    retention_policies: HashMap<String, String>,
}

#[derive(Debug, Deserialize, Clone)]
struct SecurityControls {
    encryption: EncryptionConfig,
    access_control: AccessControlConfig,
    auditing: AuditingConfig,
}

#[derive(Debug, Deserialize, Clone)]
struct EncryptionConfig {
    #[allow(dead_code)]
    at_rest: String,
    #[allow(dead_code)]
    in_transit: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct AccessControlConfig {
    #[allow(dead_code)]
    authentication: String,
    #[allow(dead_code)]
    authorization: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct AuditingConfig {
    #[allow(dead_code)]
    all_data_access: bool,
    #[allow(dead_code)]
    privileged_operations: bool,
    #[allow(dead_code)]
    compliance_reporting: String,
}

#[derive(Debug, Deserialize, Clone)]
struct PerformanceOptimization {
    #[allow(dead_code)]
    caching: CachingConfig,
    #[allow(dead_code)]
    query_optimization: QueryOptimizationConfig,
    #[allow(dead_code)]
    connection_pooling: ConnectionPoolingConfig,
}

#[derive(Debug, Deserialize, Clone)]
struct CachingConfig {
    #[allow(dead_code)]
    strategy: String,
    #[allow(dead_code)]
    #[serde(flatten)]
    config: Value,
}

#[derive(Debug, Deserialize, Clone)]
struct QueryOptimizationConfig {
    #[allow(dead_code)]
    automatic_indexing: bool,
    #[allow(dead_code)]
    query_plan_analysis: bool,
    #[allow(dead_code)]
    slow_query_alerts: bool,
}

#[derive(Debug, Deserialize, Clone)]
struct ConnectionPoolingConfig {
    #[allow(dead_code)]
    enabled: bool,
    #[allow(dead_code)]
    max_connections_per_pool: u32,
    #[allow(dead_code)]
    idle_timeout: String,
}

/// Enterprise API Gateway handler that manages all data source integrations
#[derive(Clone, Debug)]
struct EnterpriseApiGateway {
    api_config: EnterpriseApiConfig,
    data_sources: DataSources,
    #[allow(dead_code)]
    connection_cache: HashMap<String, String>,
}

impl EnterpriseApiGateway {
    fn new() -> McpResult<Self> {
        let api_config = load_api_config()
            .map_err(|e| McpError::tool_execution(&format!("Failed to load API config: {}", e)))?;

        let data_sources = load_data_sources().map_err(|e| {
            McpError::tool_execution(&format!("Failed to load data sources: {}", e))
        })?;

        Ok(Self {
            api_config,
            data_sources,
            connection_cache: HashMap::new(),
        })
    }

    /// Simulate API call to enterprise service
    fn call_enterprise_api(
        &self,
        service: &str,
        endpoint_id: &str,
        params: &Value,
    ) -> McpResult<Value> {
        let api_service = self
            .api_config
            .enterprise_apis
            .get(service)
            .ok_or_else(|| McpError::tool_execution(&format!("Service '{}' not found", service)))?;

        let endpoint = api_service
            .endpoints
            .iter()
            .find(|e| e.id == endpoint_id)
            .ok_or_else(|| {
                McpError::tool_execution(&format!("Endpoint '{}' not found", endpoint_id))
            })?;

        info!(
            "Calling {} API: {} {}",
            api_service.name, endpoint.method, endpoint.path
        );

        // Simulate API response based on endpoint
        match (service, endpoint_id) {
            ("customer_management", "get_customer") => {
                let customer_id = params
                    .get("customer_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::missing_param("customer_id"))?;

                Ok(json!({
                    "customer_id": customer_id,
                    "company_name": format!("Enterprise Corp {}", &customer_id[5..]),
                    "contact_info": {
                        "email": format!("contact@enterprise{}.com", &customer_id[5..]),
                        "phone": "+1-555-0100"
                    },
                    "account_status": "active",
                    "credit_limit": 50000.00,
                    "industry": "Technology",
                    "created_at": "2024-01-15T10:00:00Z"
                }))
            }
            ("inventory_management", "get_product") => {
                let sku = params
                    .get("sku")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::missing_param("sku"))?;

                Ok(json!({
                    "sku": sku,
                    "name": format!("Product {}", &sku[4..]),
                    "category": "Electronics",
                    "price": 299.99,
                    "inventory_levels": [
                        { "warehouse_id": "WH-EAST", "quantity": 150, "reserved": 25 },
                        { "warehouse_id": "WH-WEST", "quantity": 89, "reserved": 12 }
                    ],
                    "suppliers": [
                        { "supplier_id": "SUP-001", "name": "Primary Electronics Supplier", "lead_time": "14 days" }
                    ]
                }))
            }
            ("financial_reporting", "get_financial_report") => {
                let report_type = params
                    .get("report_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("income_statement");

                Ok(json!({
                    "report_type": report_type,
                    "period": {
                        "start_date": params.get("start_date"),
                        "end_date": params.get("end_date")
                    },
                    "data": {
                        "revenue": 2450000.00,
                        "cost_of_goods_sold": 1225000.00,
                        "gross_profit": 1225000.00,
                        "operating_expenses": 950000.00,
                        "net_income": 275000.00
                    },
                    "generated_at": Utc::now().to_rfc3339()
                }))
            }
            ("human_resources", "get_employee") => {
                let employee_id = params
                    .get("employee_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::missing_param("employee_id"))?;

                Ok(json!({
                    "employee_id": employee_id,
                    "name": format!("Employee {}", &employee_id[4..]),
                    "department": "Engineering",
                    "position": "Senior Software Developer",
                    "hire_date": "2022-03-15",
                    "manager_id": "EMP-00001",
                    "salary": if params.get("include_salary").and_then(|v| v.as_bool()).unwrap_or(false) {
                        Some(95000.00)
                    } else {
                        None
                    }
                }))
            }
            _ => Err(McpError::tool_execution(&format!(
                "Unsupported endpoint: {}.{}",
                service, endpoint_id
            ))),
        }
    }

    /// Apply data transformation rules
    fn apply_transformation(&self, transformation_name: &str, data: &Value) -> McpResult<Value> {
        let transformation = self
            .api_config
            .data_transformations
            .get(transformation_name)
            .ok_or_else(|| {
                McpError::tool_execution(&format!(
                    "Transformation '{}' not found",
                    transformation_name
                ))
            })?;

        info!("Applying transformation: {}", transformation.name);

        match transformation_name {
            "customer_360" => Ok(json!({
                "customer_360_view": {
                    "unified_customer_id": data.get("customer_id"),
                    "profile_data": data,
                    "aggregated_metrics": {
                        "total_orders": 45,
                        "lifetime_value": 125000.00,
                        "last_interaction": "2025-01-15T14:30:00Z",
                        "satisfaction_score": 4.2
                    },
                    "data_sources": ["customer_management", "salesforce", "stripe"],
                    "last_updated": Utc::now().to_rfc3339()
                }
            })),
            "financial_consolidation" => Ok(json!({
                "consolidated_financials": {
                    "revenue_total": data.get("data").and_then(|d| d.get("revenue")).unwrap_or(&json!(0)),
                    "consolidation_adjustments": {
                        "currency_conversion": 15000.00,
                        "intercompany_elimination": -25000.00
                    },
                    "data_sources": ["financial_reporting", "stripe", "salesforce"],
                    "consolidation_date": Utc::now().to_rfc3339()
                }
            })),
            _ => {
                warn!("Unknown transformation: {}", transformation_name);
                Ok(data.clone())
            }
        }
    }

    /// Get data source connection information
    fn get_data_source_info(&self, source_type: &str) -> McpResult<Value> {
        match source_type {
            "databases" => {
                let db_info = self
                    .data_sources
                    .data_sources
                    .databases
                    .iter()
                    .map(|(name, config)| {
                        json!({
                            "name": name,
                            "type": config.db_type,
                            "connection_info": {
                                "host": config.connection.host,
                                "database": config.connection.database,
                                "ssl": config.connection.ssl
                            },
                            "schema_count": config.schemas.len()
                        })
                    })
                    .collect::<Vec<_>>();

                Ok(json!({
                    "source_type": "databases",
                    "count": db_info.len(),
                    "databases": db_info
                }))
            }
            "data_warehouses" => {
                let dwh_info = self
                    .data_sources
                    .data_sources
                    .data_warehouses
                    .iter()
                    .map(|(name, config)| {
                        json!({
                            "name": name,
                            "type": config.dwh_type,
                            "fact_tables": config.fact_tables.keys().collect::<Vec<_>>(),
                            "dimension_tables": config.dimension_tables.keys().collect::<Vec<_>>()
                        })
                    })
                    .collect::<Vec<_>>();

                Ok(json!({
                    "source_type": "data_warehouses",
                    "count": dwh_info.len(),
                    "data_warehouses": dwh_info
                }))
            }
            "streaming" => {
                let stream_info = self
                    .data_sources
                    .data_sources
                    .streaming_sources
                    .iter()
                    .map(|(name, config)| {
                        json!({
                            "name": name,
                            "streaming_platform": config.name
                        })
                    })
                    .collect::<Vec<_>>();

                Ok(json!({
                    "source_type": "streaming_sources",
                    "count": stream_info.len(),
                    "streaming_sources": stream_info
                }))
            }
            _ => Err(McpError::invalid_param_type(
                "source_type",
                "databases|data_warehouses|streaming",
                source_type,
            )),
        }
    }
}

/// Tool for calling enterprise API endpoints
#[derive(McpTool, Clone, Default, Deserialize)]
#[tool(
    name = "call_enterprise_api",
    description = "Call enterprise API endpoints with parameter validation and response transformation"
)]
pub struct EnterpriseApiTool {
    #[param(description = "Enterprise service name")]
    pub service: String,

    #[param(description = "Endpoint ID to call")]
    pub endpoint_id: String,

    #[param(description = "Request parameters", optional)]
    pub parameters: Option<Value>,

    #[param(description = "Apply data transformation to response", optional)]
    pub apply_transformation: Option<bool>,
}

impl EnterpriseApiTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<Value> {
        let gateway = get_gateway()?;
        let default_params = json!({});
        let parameters = self.parameters.as_ref().unwrap_or(&default_params);
        let apply_transformation = self.apply_transformation.unwrap_or(false);

        // Call the enterprise API
        let mut response =
            gateway.call_enterprise_api(&self.service, &self.endpoint_id, parameters)?;

        // Apply transformation if requested
        if apply_transformation {
            match self.service.as_str() {
                "customer_management" => {
                    response = gateway.apply_transformation("customer_360", &response)?;
                }
                "financial_reporting" => {
                    response =
                        gateway.apply_transformation("financial_consolidation", &response)?;
                }
                _ => {}
            }
        }

        Ok(json!({
            "service": self.service,
            "endpoint": self.endpoint_id,
            "response": response,
            "transformation_applied": apply_transformation,
            "timestamp": Utc::now().to_rfc3339()
        }))
    }
}

/// Tool for querying data sources and connections
#[derive(McpTool, Clone, Default, Deserialize)]
#[tool(
    name = "query_data_sources",
    description = "Query information about available data sources, connections, and schemas"
)]
pub struct DataSourceQueryTool {
    #[param(description = "Type of data source: databases, data_warehouses, streaming")]
    pub source_type: String,

    #[param(description = "Include schema details and governance info", optional)]
    pub include_schema_details: Option<bool>,

    #[param(description = "Filter by data classification level", optional)]
    pub filter_by_classification: Option<String>,
}

impl DataSourceQueryTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<Value> {
        let gateway = get_gateway()?;
        let include_details = self.include_schema_details.unwrap_or(false);

        let mut response = gateway.get_data_source_info(&self.source_type)?;

        // Add governance and security information
        if include_details {
            let governance_info = json!({
                "data_governance": {
                    "classification_levels": gateway.data_sources.access_patterns.data_governance.classification_levels,
                    "lineage_tracking": gateway.data_sources.access_patterns.data_governance.data_lineage_tracking,
                    "retention_policies": gateway.data_sources.access_patterns.data_governance.retention_policies
                },
                "security_controls": {
                    "encryption": {
                        "at_rest": gateway.data_sources.access_patterns.security_controls.encryption.at_rest,
                        "in_transit": gateway.data_sources.access_patterns.security_controls.encryption.in_transit
                    },
                    "access_control": gateway.data_sources.access_patterns.security_controls.access_control,
                    "auditing": gateway.data_sources.access_patterns.security_controls.auditing
                }
            });

            if let Some(obj) = response.as_object_mut() {
                obj.insert("governance".to_string(), governance_info);
            }
        }

        Ok(response)
    }
}

/// Tool for listing available APIs and integrations
#[derive(McpTool, Clone, Default, Deserialize)]
#[tool(
    name = "discover_apis",
    description = "Discover available enterprise APIs and third-party integrations with their capabilities"
)]
pub struct ApiDiscoveryTool {
    #[param(
        description = "Category filter: all, enterprise, third_party, transformations",
        optional
    )]
    pub category: Option<String>,

    #[param(description = "Include endpoint details", optional)]
    pub include_endpoints: Option<bool>,

    #[param(description = "Include authentication information", optional)]
    pub include_auth_info: Option<bool>,
}

impl ApiDiscoveryTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<Value> {
        let gateway = get_gateway()?;
        let category = self.category.as_deref().unwrap_or("all");
        let include_endpoints = self.include_endpoints.unwrap_or(true);
        let include_auth = self.include_auth_info.unwrap_or(false);

        let mut result = json!({
            "discovery_timestamp": Utc::now().to_rfc3339(),
            "category": category
        });

        match category {
            "all" | "enterprise" => {
                let enterprise_apis = gateway
                    .api_config
                    .enterprise_apis
                    .iter()
                    .map(|(key, api)| {
                        let mut api_info = json!({
                            "service_id": key,
                            "name": api.name,
                            "base_url": api.base_url,
                            "rate_limits": api.rate_limits
                        });

                        if include_endpoints {
                            api_info["endpoints"] = json!(
                                api.endpoints
                                    .iter()
                                    .map(|ep| json!({
                                        "id": ep.id,
                                        "method": ep.method,
                                        "path": ep.path,
                                        "description": ep.description
                                    }))
                                    .collect::<Vec<_>>()
                            );
                        }

                        if include_auth {
                            api_info["authentication"] = json!(api.authentication);
                        }

                        api_info
                    })
                    .collect::<Vec<_>>();

                result["enterprise_apis"] = json!(enterprise_apis);
            }
            _ => {}
        }

        if category == "all" || category == "third_party" {
            let third_party_apis = gateway
                .api_config
                .third_party_integrations
                .iter()
                .map(|(key, service)| {
                    json!({
                        "service_id": key,
                        "name": service.name,
                        "type": service.service_type,
                        "base_url": service.base_url,
                        "endpoint_count": service.endpoints.len()
                    })
                })
                .collect::<Vec<_>>();

            result["third_party_integrations"] = json!(third_party_apis);
        }

        if category == "all" || category == "transformations" {
            let transformations = gateway
                .api_config
                .data_transformations
                .iter()
                .map(|(key, transform)| {
                    json!({
                        "transformation_id": key,
                        "name": transform.name,
                        "description": transform.description,
                        "sources": transform.sources,
                        "rule_count": transform.transformations.len()
                    })
                })
                .collect::<Vec<_>>();

            result["data_transformations"] = json!(transformations);
        }

        Ok(result)
    }
}

/// Tool for testing API connectivity and health
#[derive(McpTool, Clone, Default, Deserialize)]
#[tool(
    name = "check_api_health",
    description = "Perform health checks on enterprise APIs and data source connections"
)]
pub struct ApiHealthCheckTool {
    #[param(description = "List of service names to check", optional)]
    pub services: Option<Vec<String>>,

    #[param(description = "Include performance metrics", optional)]
    pub include_performance_metrics: Option<bool>,

    #[param(description = "Include detailed diagnostics", optional)]
    pub detailed_diagnostics: Option<bool>,
}

impl ApiHealthCheckTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<Value> {
        let gateway = get_gateway()?;

        let services: Vec<&str> = match &self.services {
            Some(svc_list) => svc_list.iter().map(|s| s.as_str()).collect(),
            None => gateway
                .api_config
                .enterprise_apis
                .keys()
                .map(|k| k.as_str())
                .collect(),
        };

        let include_metrics = self.include_performance_metrics.unwrap_or(false);
        let detailed = self.detailed_diagnostics.unwrap_or(false);

        let mut health_results = Vec::new();

        for service_name in services {
            if let Some(_service) = gateway.api_config.enterprise_apis.get(service_name) {
                // Simulate health check
                let health_status = match service_name {
                    "customer_management" => {
                        json!({
                            "service": service_name,
                            "status": "healthy",
                            "response_time_ms": 45,
                            "last_check": Utc::now().to_rfc3339(),
                            "availability": "99.9%"
                        })
                    }
                    "inventory_management" => {
                        json!({
                            "service": service_name,
                            "status": "healthy",
                            "response_time_ms": 32,
                            "last_check": Utc::now().to_rfc3339(),
                            "availability": "99.95%"
                        })
                    }
                    "financial_reporting" => {
                        json!({
                            "service": service_name,
                            "status": "degraded",
                            "response_time_ms": 1250,
                            "last_check": Utc::now().to_rfc3339(),
                            "availability": "98.5%",
                            "issues": ["High response times during peak hours"]
                        })
                    }
                    _ => {
                        json!({
                            "service": service_name,
                            "status": "healthy",
                            "response_time_ms": 78,
                            "last_check": Utc::now().to_rfc3339(),
                            "availability": "99.8%"
                        })
                    }
                };

                if include_metrics {
                    let mut metrics = health_status.as_object().unwrap().clone();
                    metrics.insert(
                        "performance_metrics".to_string(),
                        json!({
                            "requests_per_second": 125.5,
                            "error_rate": 0.02,
                            "p95_response_time_ms": 89,
                            "p99_response_time_ms": 156,
                            "throughput_mb_per_sec": 2.3
                        }),
                    );
                    health_results.push(Value::Object(metrics));
                } else {
                    health_results.push(health_status);
                }

                if detailed {
                    let last_result = health_results.last_mut().unwrap();
                    if let Some(obj) = last_result.as_object_mut() {
                        obj.insert(
                            "detailed_diagnostics".to_string(),
                            json!({
                                "connection_pool": {
                                    "active_connections": 8,
                                    "max_connections": 50,
                                    "idle_connections": 12
                                },
                                "circuit_breaker": {
                                    "state": "closed",
                                    "failure_rate": 0.01,
                                    "last_failure": "2025-01-18T10:45:00Z"
                                },
                                "cache_performance": {
                                    "hit_rate": 0.85,
                                    "miss_rate": 0.15,
                                    "eviction_rate": 0.02
                                }
                            }),
                        );
                    }
                }
            } else {
                health_results.push(json!({
                    "service": service_name,
                    "status": "not_found",
                    "error": format!("Service '{}' not configured", service_name)
                }));
            }
        }

        Ok(json!({
            "health_check_summary": {
                "total_services": health_results.len(),
                "healthy": health_results.iter().filter(|r| r["status"] == "healthy").count(),
                "degraded": health_results.iter().filter(|r| r["status"] == "degraded").count(),
                "unhealthy": health_results.iter().filter(|r| r["status"] == "unhealthy").count(),
                "not_found": health_results.iter().filter(|r| r["status"] == "not_found").count(),
                "check_timestamp": Utc::now().to_rfc3339()
            },
            "service_health_details": health_results
        }))
    }
}

// Helper functions to load external configuration files

fn load_api_config() -> Result<EnterpriseApiConfig, Box<dyn std::error::Error>> {
    let path = "data/api_endpoints.json";
    if Path::new(path).exists() {
        let content = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&content)?)
    } else {
        warn!("API config file not found, using minimal fallback");
        Ok(EnterpriseApiConfig {
            enterprise_apis: HashMap::new(),
            third_party_integrations: HashMap::new(),
            data_transformations: HashMap::new(),
        })
    }
}

fn load_data_sources() -> Result<DataSources, Box<dyn std::error::Error>> {
    let path = "data/data_sources.yaml";
    if Path::new(path).exists() {
        let content = fs::read_to_string(path)?;
        Ok(serde_yaml::from_str(&content)?)
    } else {
        warn!("Data sources config file not found, using minimal fallback");
        Ok(DataSources {
            data_sources: DataSourceConfig {
                databases: HashMap::new(),
                data_warehouses: HashMap::new(),
                streaming_sources: HashMap::new(),
                file_systems: HashMap::new(),
                monitoring_systems: HashMap::new(),
            },
            access_patterns: AccessPatterns {
                data_governance: DataGovernance {
                    classification_levels: vec!["public".to_string(), "internal".to_string()],
                    data_lineage_tracking: true,
                    retention_policies: HashMap::new(),
                },
                security_controls: SecurityControls {
                    encryption: EncryptionConfig {
                        at_rest: "AES-256".to_string(),
                        in_transit: "TLS 1.3".to_string(),
                    },
                    access_control: AccessControlConfig {
                        authentication: "OAuth 2.0".to_string(),
                        authorization: "RBAC".to_string(),
                    },
                    auditing: AuditingConfig {
                        all_data_access: true,
                        privileged_operations: true,
                        compliance_reporting: "automated".to_string(),
                    },
                },
                performance_optimization: PerformanceOptimization {
                    caching: CachingConfig {
                        strategy: "multi-level".to_string(),
                        config: json!({}),
                    },
                    query_optimization: QueryOptimizationConfig {
                        automatic_indexing: true,
                        query_plan_analysis: true,
                        slow_query_alerts: true,
                    },
                    connection_pooling: ConnectionPoolingConfig {
                        enabled: true,
                        max_connections_per_pool: 50,
                        idle_timeout: "300 seconds".to_string(),
                    },
                },
            },
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting Enterprise API Data Gateway Server");

    // Create the enterprise gateway and store in global static
    let gateway = EnterpriseApiGateway::new()?;

    info!("Gateway configuration loaded:");
    info!(
        "  {} enterprise APIs",
        gateway.api_config.enterprise_apis.len()
    );
    info!(
        "  {} third-party integrations",
        gateway.api_config.third_party_integrations.len()
    );
    info!(
        "  {} data transformations",
        gateway.api_config.data_transformations.len()
    );
    info!(
        "  {} data sources configured",
        gateway.data_sources.data_sources.databases.len()
            + gateway.data_sources.data_sources.data_warehouses.len()
            + gateway.data_sources.data_sources.streaming_sources.len()
    );

    GATEWAY.set(gateway).expect("Gateway already initialized");

    let server = McpServer::builder()
        .name("enterprise-api-gateway")
        .version("1.0.0")
        .title("Enterprise API Data Gateway Server")
        .instructions("This server provides unified access to enterprise APIs, data sources, and third-party integrations. Use the tools to call APIs, query data sources, discover available services, and monitor system health.")
        .tool(EnterpriseApiTool::default())
        .tool(DataSourceQueryTool::default())
        .tool(ApiDiscoveryTool::default())
        .tool(ApiHealthCheckTool::default())
        .bind_address("127.0.0.1:8048".parse()?)
        .build()?;

    info!("Enterprise API Gateway running at: http://127.0.0.1:8048/mcp");

    server.run().await?;
    Ok(())
}
