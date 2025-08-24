//! SNS Event Processor
//!
//! Processes external events from SNS for global event distribution

use crate::global_events::{
    GlobalEvent, SessionEventType, ToolExecutionStatus,
    broadcast_global_event, broadcast_session_event, broadcast_tool_progress, broadcast_monitoring_update
};
use aws_sdk_sns::{Client as SnsClient, types::MessageAttributeValue};
use serde_json::{Value, json};
use std::collections::HashMap;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// External event types that can be received via SNS
#[derive(Debug, Clone)]
pub enum ExternalEventType {
    /// Session-related events from other services
    SessionEvent {
        session_id: String,
        event_type: SessionEventType,
        data: Option<Value>,
    },
    /// Tool execution events from distributed systems
    ToolEvent {
        tool_name: String,
        session_id: String,
        status: ToolExecutionStatus,
        data: Option<Value>,
    },
    /// Monitoring updates from external services
    MonitoringEvent {
        resource_type: String,
        region: String,
        correlation_id: String,
        data: Value,
    },
    /// System health events
    SystemEvent {
        event_type: String,
        source: String,
        data: Value,
    },
}

/// SNS event processor for external event integration
pub struct SnsEventProcessor {
    /// SNS client for publishing events
    sns_client: SnsClient,
    /// SNS topic ARN for publishing events
    topic_arn: String,
}

impl SnsEventProcessor {
    /// Create new SNS event processor
    pub async fn new(topic_arn: String) -> Result<Self, aws_sdk_sns::Error> {
        info!("🔗 Initializing SNS event processor...");
        
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest()).load().await;
        let sns_client = SnsClient::new(&config);

        // Test SNS connection
        match sns_client.list_topics().send().await {
            Ok(_) => {
                info!("✅ SNS connection established successfully");
                info!("📡 SNS topic: {}", topic_arn);
            }
            Err(e) => {
                warn!("⚠️  SNS connection test failed: {:?} (continuing anyway)", e);
            }
        }

        info!("✅ SNS Event Processor initialized - Topic: {}", topic_arn);

        Ok(Self {
            sns_client,
            topic_arn,
        })
    }

    /// Publish event to SNS topic (for outgoing events)
    pub async fn publish_event(&self, event: GlobalEvent) -> Result<(), aws_sdk_sns::Error> {
        // Convert global event to SNS message
        let (message, subject, attributes) = self.prepare_sns_message(event);

        debug!("Publishing event to SNS topic: {}", self.topic_arn);

        self.sns_client
            .publish()
            .topic_arn(&self.topic_arn)
            .message(message)
            .subject(subject)
            .set_message_attributes(Some(attributes))
            .send()
            .await?;

        Ok(())
    }

    /// Prepare SNS message from global event
    fn prepare_sns_message(&self, event: GlobalEvent) -> (String, String, HashMap<String, MessageAttributeValue>) {
        let mut attributes = HashMap::new();
        
        // Add correlation ID
        attributes.insert(
            "correlation_id".to_string(),
            MessageAttributeValue::builder()
                .data_type("String")
                .string_value(Uuid::now_v7().to_string())
                .build()
                .unwrap()
        );

        // Add timestamp
        attributes.insert(
            "timestamp".to_string(),
            MessageAttributeValue::builder()
                .data_type("String")
                .string_value(chrono::Utc::now().to_rfc3339())
                .build()
                .unwrap()
        );

        match event {
            GlobalEvent::ToolExecution { tool_name, session_id, status, result, .. } => {
                let message = json!({
                    "event_type": "tool",
                    "tool_name": tool_name,
                    "session_id": session_id,
                    "status": match status {
                        ToolExecutionStatus::Started => "started",
                        ToolExecutionStatus::InProgress { .. } => "in_progress",
                        ToolExecutionStatus::Completed => "completed",
                        ToolExecutionStatus::Failed { .. } => "failed",
                    },
                    "data": result,
                    "timestamp": chrono::Utc::now().to_rfc3339()
                });

                attributes.insert(
                    "event_type".to_string(),
                    MessageAttributeValue::builder()
                        .data_type("String")
                        .string_value("tool")
                        .build()
                        .unwrap()
                );

                (
                    serde_json::to_string(&message).unwrap_or_default(),
                    format!("Tool Execution: {}", tool_name),
                    attributes
                )
            }

            GlobalEvent::SessionUpdate { session_id, event_type, data, .. } => {
                let message = json!({
                    "event_type": "session",
                    "session_id": session_id,
                    "session_event_type": match event_type {
                        SessionEventType::Created => "created",
                        SessionEventType::Initialized => "initialized",
                        SessionEventType::Updated => "updated",
                        SessionEventType::Expired => "expired",
                        SessionEventType::Deleted => "deleted",
                        SessionEventType::CleanupTriggered => "cleanup_triggered",
                        SessionEventType::InfoRequested => "info_requested",
                        SessionEventType::SessionsListed => "sessions_listed",
                    },
                    "data": data,
                    "timestamp": chrono::Utc::now().to_rfc3339()
                });

                attributes.insert(
                    "event_type".to_string(),
                    MessageAttributeValue::builder()
                        .data_type("String")
                        .string_value("session")
                        .build()
                        .unwrap()
                );

                (
                    serde_json::to_string(&message).unwrap_or_default(),
                    format!("Session Update: {}", session_id),
                    attributes
                )
            }

            GlobalEvent::MonitoringUpdate { resource_type, region, correlation_id, data, .. } => {
                let message = json!({
                    "event_type": "monitoring",
                    "resource_type": resource_type,
                    "region": region,
                    "correlation_id": correlation_id,
                    "data": data,
                    "timestamp": chrono::Utc::now().to_rfc3339()
                });

                attributes.insert(
                    "event_type".to_string(),
                    MessageAttributeValue::builder()
                        .data_type("String")
                        .string_value("monitoring")
                        .build()
                        .unwrap()
                );

                (
                    serde_json::to_string(&message).unwrap_or_default(),
                    format!("Monitoring Update: {}/{}", resource_type, region),
                    attributes
                )
            }

            GlobalEvent::SystemHealth { component, status, details, .. } => {
                let message = json!({
                    "event_type": "system",
                    "system_event_type": status,
                    "source": component,
                    "data": details,
                    "timestamp": chrono::Utc::now().to_rfc3339()
                });

                attributes.insert(
                    "event_type".to_string(),
                    MessageAttributeValue::builder()
                        .data_type("String")
                        .string_value("system")
                        .build()
                        .unwrap()
                );

                (
                    serde_json::to_string(&message).unwrap_or_default(),
                    format!("System Health: {}", component),
                    attributes
                )
            }

            GlobalEvent::ExternalEvent { source, event_type, payload, .. } => {
                let message = json!({
                    "event_type": "external",
                    "source": source,
                    "external_event_type": event_type,
                    "data": payload,
                    "timestamp": chrono::Utc::now().to_rfc3339()
                });

                attributes.insert(
                    "event_type".to_string(),
                    MessageAttributeValue::builder()
                        .data_type("String")
                        .string_value("external")
                        .build()
                        .unwrap()
                );

                (
                    serde_json::to_string(&message).unwrap_or_default(),
                    format!("External Event: {}", source),
                    attributes
                )
            }

            GlobalEvent::ServerLifecycle { event, instance_id, .. } => {
                let message = json!({
                    "event_type": "lifecycle",
                    "lifecycle_event": format!("{:?}", event),
                    "instance_id": instance_id,
                    "timestamp": chrono::Utc::now().to_rfc3339()
                });

                attributes.insert(
                    "event_type".to_string(),
                    MessageAttributeValue::builder()
                        .data_type("String")
                        .string_value("lifecycle")
                        .build()
                        .unwrap()
                );

                (
                    serde_json::to_string(&message).unwrap_or_default(),
                    "Server Lifecycle Event".to_string(),
                    attributes
                )
            }
        }
    }

    /// Process incoming SNS message (for Lambda SNS triggers)
    pub async fn process_sns_message(&self, sns_message: Value) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Processing SNS message: {}", sns_message);

        // Extract message content
        let message = sns_message.get("Message")
            .and_then(|m| m.as_str())
            .ok_or("No Message field in SNS message")?;
        
        let event_data: Value = serde_json::from_str(message)?;
        
        // Convert to external event and broadcast internally
        let external_event = self.parse_external_event(event_data)?;
        self.broadcast_external_event(external_event).await?;

        debug!("Successfully processed SNS message");
        Ok(())
    }

    /// Parse external event from message data
    fn parse_external_event(&self, data: Value) -> Result<ExternalEventType, Box<dyn std::error::Error + Send + Sync>> {
        let event_type = data.get("event_type")
            .and_then(|t| t.as_str())
            .ok_or("Missing event_type in message")?;

        match event_type {
            "session" => {
                let session_id = data.get("session_id")
                    .and_then(|s| s.as_str())
                    .ok_or("Missing session_id for session event")?
                    .to_string();

                let session_event_type = data.get("session_event_type")
                    .and_then(|t| t.as_str())
                    .and_then(|t| match t {
                        "created" => Some(SessionEventType::Created),
                        "initialized" => Some(SessionEventType::Initialized),
                        "updated" => Some(SessionEventType::Updated),
                        "expired" => Some(SessionEventType::Expired),
                        "deleted" => Some(SessionEventType::Deleted),
                        "cleanup_triggered" => Some(SessionEventType::CleanupTriggered),
                        "info_requested" => Some(SessionEventType::InfoRequested),
                        "sessions_listed" => Some(SessionEventType::SessionsListed),
                        _ => None,
                    })
                    .ok_or("Invalid session_event_type")?;

                Ok(ExternalEventType::SessionEvent {
                    session_id,
                    event_type: session_event_type,
                    data: data.get("data").cloned(),
                })
            }
            
            "tool" => {
                let tool_name = data.get("tool_name")
                    .and_then(|t| t.as_str())
                    .ok_or("Missing tool_name for tool event")?
                    .to_string();

                let session_id = data.get("session_id")
                    .and_then(|s| s.as_str())
                    .ok_or("Missing session_id for tool event")?
                    .to_string();

                let status = data.get("status")
                    .and_then(|s| s.as_str())
                    .and_then(|s| match s {
                        "started" => Some(ToolExecutionStatus::Started),
                        "in_progress" => Some(ToolExecutionStatus::InProgress { progress: None }),
                        "completed" => Some(ToolExecutionStatus::Completed),
                        "failed" => Some(ToolExecutionStatus::Failed { 
                            error: data.get("error").and_then(|e| e.as_str()).unwrap_or("Unknown error").to_string() 
                        }),
                        _ => None,
                    })
                    .ok_or("Invalid tool execution status")?;

                Ok(ExternalEventType::ToolEvent {
                    tool_name,
                    session_id,
                    status,
                    data: data.get("data").cloned(),
                })
            }
            
            "monitoring" => {
                let resource_type = data.get("resource_type")
                    .and_then(|r| r.as_str())
                    .ok_or("Missing resource_type for monitoring event")?
                    .to_string();

                let region = data.get("region")
                    .and_then(|r| r.as_str())
                    .ok_or("Missing region for monitoring event")?
                    .to_string();

                let correlation_id = data.get("correlation_id")
                    .and_then(|c| c.as_str())
                    .ok_or("Missing correlation_id for monitoring event")?
                    .to_string();

                let event_data = data.get("data")
                    .cloned()
                    .ok_or("Missing data for monitoring event")?;

                Ok(ExternalEventType::MonitoringEvent {
                    resource_type,
                    region,
                    correlation_id,
                    data: event_data,
                })
            }
            
            "system" => {
                let system_event_type = data.get("system_event_type")
                    .and_then(|t| t.as_str())
                    .ok_or("Missing system_event_type for system event")?
                    .to_string();

                let source = data.get("source")
                    .and_then(|s| s.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                let event_data = data.get("data")
                    .cloned()
                    .unwrap_or_default();

                Ok(ExternalEventType::SystemEvent {
                    event_type: system_event_type,
                    source,
                    data: event_data,
                })
            }
            
            _ => Err(format!("Unknown external event type: {}", event_type).into())
        }
    }

    /// Broadcast external event to internal global event system
    async fn broadcast_external_event(&self, external_event: ExternalEventType) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match external_event {
            ExternalEventType::SessionEvent { session_id, event_type, data } => {
                broadcast_session_event(&session_id, event_type, data).await?;
            }
            
            ExternalEventType::ToolEvent { tool_name, session_id, status, data } => {
                broadcast_tool_progress(&tool_name, &session_id, status, data).await?;
            }
            
            ExternalEventType::MonitoringEvent { resource_type, region, correlation_id, data } => {
                broadcast_monitoring_update(&resource_type, &region, &correlation_id, data).await?;
            }
            
            ExternalEventType::SystemEvent { event_type, source, data } => {
                let global_event = GlobalEvent::SystemHealth {
                    component: source,
                    status: event_type,
                    details: data,
                    timestamp: chrono::Utc::now(),
                };
                broadcast_global_event(global_event).await?;
            }
        }

        Ok(())
    }
}

/// Start SNS event processing in background
pub async fn start_sns_processing(topic_arn: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let processor = SnsEventProcessor::new(topic_arn).await?;
    
    info!("SNS event processor initialized and ready for publishing");
    
    // For Lambda environment, we don't need background processing
    // The processor will be used to publish events as they occur
    Ok(())
}

/// Global SNS processor instance for publishing events
static SNS_PROCESSOR: std::sync::OnceLock<SnsEventProcessor> = std::sync::OnceLock::new();

/// Initialize global SNS processor
pub async fn initialize_sns_processor(topic_arn: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let processor = SnsEventProcessor::new(topic_arn).await?;
    
    if SNS_PROCESSOR.set(processor).is_err() {
        warn!("SNS processor was already initialized");
    } else {
        info!("Global SNS processor initialized");
    }
    
    Ok(())
}

/// Publish event to SNS using global processor
pub async fn publish_to_sns(event: GlobalEvent) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Some(processor) = SNS_PROCESSOR.get() {
        processor.publish_event(event).await?;
        Ok(())
    } else {
        warn!("SNS processor not initialized, skipping event publication");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_external_event_type_creation() {
        let session_event = ExternalEventType::SessionEvent {
            session_id: "test-session".to_string(),
            event_type: SessionEventType::Created,
            data: Some(json!({"test": "data"})),
        };

        match session_event {
            ExternalEventType::SessionEvent { session_id, .. } => {
                assert_eq!(session_id, "test-session");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_parse_sns_message() {
        let sns_body = r#"{
            "Type": "Notification",
            "MessageId": "test-id",
            "TopicArn": "arn:aws:sns:us-east-1:123456789012:test-topic",
            "Message": "{\"event_type\":\"session\",\"session_id\":\"test\",\"session_event_type\":\"created\"}"
        }"#;

        let parsed: Value = serde_json::from_str(sns_body).unwrap();
        let inner_message = parsed.get("Message").and_then(|m| m.as_str()).unwrap();
        let event_data: Value = serde_json::from_str(inner_message).unwrap();
        
        assert_eq!(event_data.get("event_type").and_then(|t| t.as_str()), Some("session"));
    }
}