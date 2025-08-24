//! AWS Tools for Real-time Monitoring
//!
//! Tools for monitoring AWS resources with streaming updates and global event broadcasting

use crate::global_events::{broadcast_monitoring_update, broadcast_tool_progress, ToolExecutionStatus};
use async_trait::async_trait;
use mcp_protocol::{ToolResult, ToolSchema, schema::JsonSchema};
use mcp_server::{McpTool, SessionContext, McpResult};
use serde_json::{Value, json};
use std::collections::HashMap;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// AWS Real-time Monitor Tool
/// 
/// Monitors AWS resources with live updates and streaming responses via global events
pub struct AwsRealTimeMonitor;

#[async_trait]
impl McpTool for AwsRealTimeMonitor {
    fn name(&self) -> &str {
        "aws_real_time_monitor"
    }

    fn description(&self) -> &str {
        "Monitor AWS resources with live updates and streaming responses via global event broadcasting"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("resource_type".to_string(), JsonSchema::string_with_description("Type of AWS resource to monitor")),
                ("region".to_string(), JsonSchema::string_with_description("AWS region to monitor")),
                ("update_frequency".to_string(), JsonSchema::number_with_description("Update frequency in milliseconds")
                    .with_minimum(1000.0)
                    .with_maximum(60000.0)),
                ("filter_tags".to_string(), JsonSchema::object().with_description("Tag filters for resource selection")),
            ]))
            .with_required(vec!["resource_type".to_string()])
    }

    async fn call(&self, args: Value, session: Option<SessionContext>) -> McpResult<Vec<ToolResult>> {
        let resource_type = args.get("resource_type")
            .and_then(|v| v.as_str())
            .ok_or("Missing required parameter: resource_type")?;
            
        let region = args.get("region")
            .and_then(|v| v.as_str())
            .unwrap_or("us-east-1");
            
        let update_frequency = args.get("update_frequency")
            .and_then(|v| v.as_f64())
            .unwrap_or(5000.0) as u64;
            
        let filter_tags = args.get("filter_tags")
            .cloned()
            .unwrap_or_else(|| json!({}));

        let session_id = session
            .as_ref()
            .map(|s| s.session_id.clone())
            .unwrap_or_else(|| "unknown".to_string());

        let correlation_id = Uuid::now_v7().to_string();

        info!("Starting AWS resource monitoring: {} in {} (correlation: {})", 
              resource_type, region, correlation_id);

        // Broadcast tool execution start
        if let Err(e) = broadcast_tool_progress(
            self.name(),
            &session_id,
            ToolExecutionStatus::Started,
            None,
        ).await {
            warn!("Failed to broadcast tool start: {:?}", e);
        }

        // Simulate initial resource discovery
        let initial_resources = Self::discover_resources(resource_type, region, &filter_tags).await?;
        
        // Broadcast initial discovery results
        if let Err(e) = broadcast_monitoring_update(
            resource_type,
            region,
            &correlation_id,
            json!({
                "event": "initial_discovery",
                "resources": initial_resources,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }),
        ).await {
            warn!("Failed to broadcast initial discovery: {:?}", e);
        }

        // Start background monitoring task
        let resource_type_clone = resource_type.to_string();
        let region_clone = region.to_string();
        let correlation_id_clone = correlation_id.clone();
        let session_id_clone = session_id.clone();
        
        tokio::spawn(async move {
            Self::monitor_resources_background(
                resource_type_clone,
                region_clone,
                correlation_id_clone,
                session_id_clone,
                update_frequency,
                filter_tags,
            ).await;
        });

        // Broadcast tool execution completion
        let result_data = json!({
            "monitoring_started": true,
            "resource_type": resource_type,
            "region": region,
            "correlation_id": correlation_id,
            "update_frequency_ms": update_frequency,
            "initial_resources_count": initial_resources.as_array().map(|a| a.len()).unwrap_or(0)
        });

        if let Err(e) = broadcast_tool_progress(
            self.name(),
            &session_id,
            ToolExecutionStatus::Completed,
            Some(result_data.clone()),
        ).await {
            warn!("Failed to broadcast tool completion: {:?}", e);
        }

        // Return the actual monitoring data, not a generic success message
        let result = ToolResult::resource(result_data);

        Ok(vec![result])
    }
}

impl AwsRealTimeMonitor {
    /// Discover initial resources (simulated for demo)
    async fn discover_resources(
        resource_type: &str,
        region: &str,
        filter_tags: &Value,
    ) -> McpResult<Value> {
        debug!("Discovering {} resources in {} with filters: {}", resource_type, region, filter_tags);

        // Simulate resource discovery delay
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Generate simulated resources based on type
        let resources = match resource_type {
            "EC2" => json!([
                {
                    "id": "i-1234567890abcdef0",
                    "state": "running",
                    "instance_type": "t3.micro",
                    "tags": {"Environment": "dev", "Application": "web"},
                    "last_updated": chrono::Utc::now().to_rfc3339()
                },
                {
                    "id": "i-0987654321fedcba0",
                    "state": "stopped",
                    "instance_type": "t3.small",
                    "tags": {"Environment": "prod", "Application": "api"},
                    "last_updated": chrono::Utc::now().to_rfc3339()
                }
            ]),
            "Lambda" => json!([
                {
                    "id": "arn:aws:lambda:us-east-1:123456789012:function:my-function",
                    "state": "Active",
                    "runtime": "rust",
                    "last_modified": chrono::Utc::now().to_rfc3339(),
                    "tags": {"Environment": "prod"}
                }
            ]),
            "RDS" => json!([
                {
                    "id": "database-1",
                    "state": "available",
                    "engine": "postgres",
                    "engine_version": "13.7",
                    "tags": {"Environment": "prod", "Application": "api"},
                    "last_updated": chrono::Utc::now().to_rfc3339()
                }
            ]),
            _ => json!([]) // Unknown resource types return empty
        };

        Ok(resources)
    }

    /// Background monitoring task that sends periodic updates
    async fn monitor_resources_background(
        resource_type: String,
        region: String,
        correlation_id: String,
        session_id: String,
        update_frequency_ms: u64,
        _filter_tags: Value,
    ) {
        let mut update_count = 0;
        let max_updates = 20; // Limit for demo purposes
        
        info!("Starting background monitoring for {} in {} ({})", resource_type, region, correlation_id);

        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(update_frequency_ms));
        
        while update_count < max_updates {
            interval.tick().await;
            update_count += 1;

            // Generate simulated update
            let update_data = json!({
                "event": "resource_update",
                "update_number": update_count,
                "total_updates": max_updates,
                "resource_changes": Self::generate_resource_update(&resource_type, update_count),
                "timestamp": chrono::Utc::now().to_rfc3339()
            });

            // Broadcast monitoring update
            if let Err(e) = broadcast_monitoring_update(
                &resource_type,
                &region,
                &correlation_id,
                update_data,
            ).await {
                warn!("Failed to broadcast monitoring update {}: {:?}", update_count, e);
                break;
            }

            debug!("Sent monitoring update {}/{} for {}", update_count, max_updates, correlation_id);
        }

        // Send completion notification
        let completion_data = json!({
            "event": "monitoring_completed",
            "total_updates_sent": update_count,
            "reason": "max_updates_reached",
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        if let Err(e) = broadcast_monitoring_update(
            &resource_type,
            &region,
            &correlation_id,
            completion_data,
        ).await {
            warn!("Failed to broadcast monitoring completion: {:?}", e);
        }

        info!("Background monitoring completed for {} (sent {} updates)", correlation_id, update_count);
    }

    /// Generate simulated resource update data
    fn generate_resource_update(resource_type: &str, update_number: u32) -> Value {
        match resource_type {
            "EC2" => json!({
                "instances_updated": 1,
                "changes": [
                    {
                        "instance_id": "i-1234567890abcdef0",
                        "previous_state": "running",
                        "current_state": if update_number % 3 == 0 { "stopping" } else { "running" },
                        "cpu_utilization": 20.5 + (update_number as f64 * 2.5) % 80.0,
                        "memory_utilization": 45.2 + (update_number as f64 * 1.8) % 50.0
                    }
                ]
            }),
            "Lambda" => json!({
                "functions_updated": 1,
                "changes": [
                    {
                        "function_name": "my-function",
                        "invocations_count": update_number * 10,
                        "duration_ms": 150 + (update_number % 5) * 20,
                        "error_rate": if update_number % 7 == 0 { 2.1 } else { 0.0 }
                    }
                ]
            }),
            "RDS" => json!({
                "databases_updated": 1,
                "changes": [
                    {
                        "db_instance": "database-1",
                        "cpu_utilization": 15.3 + (update_number as f64 * 1.2) % 30.0,
                        "connections": 5 + (update_number % 15),
                        "read_iops": 100 + (update_number * 10) % 500,
                        "write_iops": 50 + (update_number * 5) % 200
                    }
                ]
            }),
            _ => json!({
                "message": "Unknown resource type",
                "update_number": update_number
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_aws_real_time_monitor() {
        let tool = AwsRealTimeMonitor;
        
        assert_eq!(tool.name(), "aws_real_time_monitor");
        assert!(!tool.description().is_empty());
        
        let schema = tool.input_schema();
        assert!(schema.properties.contains_key("resource_type"));
    }

    #[tokio::test]
    async fn test_discover_resources() {
        let result = AwsRealTimeMonitor::discover_resources("EC2", "us-east-1", &json!({})).await;
        assert!(result.is_ok());
        
        let resources = result.unwrap();
        assert!(resources.is_array());
        assert!(resources.as_array().unwrap().len() > 0);
    }

    #[test]
    fn test_generate_resource_update() {
        let update = AwsRealTimeMonitor::generate_resource_update("EC2", 1);
        assert!(update.get("instances_updated").is_some());
        assert!(update.get("changes").is_some());
    }
}