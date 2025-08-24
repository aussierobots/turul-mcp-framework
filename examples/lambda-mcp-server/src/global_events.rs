//! Global Event Broadcasting System
//!
//! Two-tier notification architecture:
//! 1. Internal Events: tokio::broadcast for tool calls, server health, session management
//! 2. External Events: SNS → Lambda → tokio::broadcast distribution

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::OnceLock;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

/// Global broadcast channel for internal events
static GLOBAL_EVENT_CHANNEL: OnceLock<broadcast::Sender<GlobalEvent>> = OnceLock::new();

/// Channel capacity for global events
const GLOBAL_EVENT_CAPACITY: usize = 1000;

/// Global event types for MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GlobalEvent {
    /// System health and status events
    SystemHealth {
        component: String,
        status: String,
        details: Value,
        timestamp: DateTime<Utc>,
    },
    /// Tool execution events (during tools/call)
    ToolExecution {
        tool_name: String,
        session_id: String,
        status: ToolExecutionStatus,
        result: Option<Value>,
        timestamp: DateTime<Utc>,
    },
    /// Session management events
    SessionUpdate {
        session_id: String,
        event_type: SessionEventType,
        data: Option<Value>,
        timestamp: DateTime<Utc>,
    },
    /// Real-time monitoring events (from AWS tools)
    MonitoringUpdate {
        resource_type: String,
        region: String,
        correlation_id: String,
        data: Value,
        timestamp: DateTime<Utc>,
    },
    /// External SNS events
    ExternalEvent {
        source: String,
        event_type: String,
        payload: Value,
        timestamp: DateTime<Utc>,
    },
    /// Server lifecycle events
    ServerLifecycle {
        event: LifecycleEvent,
        instance_id: String,
        timestamp: DateTime<Utc>,
    },
}

/// Tool execution status for tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolExecutionStatus {
    Started,
    InProgress { progress: Option<f64> },
    Completed,
    Failed { error: String },
}

/// Session management actions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionAction {
    Created,
    Initialized,
    Updated,
    Expired,
    Deleted,
    CleanupTriggered,
    InfoRequested,
    SessionsListed,
}

/// Session event types (alias for compatibility)
pub type SessionEventType = SessionAction;

/// Server lifecycle events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleEvent {
    Starting,
    Ready,
    Stopping,
    Error { message: String },
}

impl GlobalEvent {
    /// Create a system health event
    pub fn system_health(component: impl Into<String>, status: impl Into<String>, details: Value) -> Self {
        Self::SystemHealth {
            component: component.into(),
            status: status.into(),
            details,
            timestamp: Utc::now(),
        }
    }

    /// Create a tool execution event
    pub fn tool_execution(
        tool_name: impl Into<String>,
        session_id: impl Into<String>,
        status: ToolExecutionStatus,
        result: Option<Value>,
    ) -> Self {
        Self::ToolExecution {
            tool_name: tool_name.into(),
            session_id: session_id.into(),
            status,
            result,
            timestamp: Utc::now(),
        }
    }

    /// Create a session management event
    pub fn session_update(
        session_id: impl Into<String>,
        event_type: SessionEventType,
        data: Option<Value>,
    ) -> Self {
        Self::SessionUpdate {
            session_id: session_id.into(),
            event_type,
            data,
            timestamp: Utc::now(),
        }
    }

    /// Create a monitoring update event
    pub fn monitoring_update(
        resource_type: impl Into<String>,
        region: impl Into<String>,
        correlation_id: impl Into<String>,
        data: Value,
    ) -> Self {
        Self::MonitoringUpdate {
            resource_type: resource_type.into(),
            region: region.into(),
            correlation_id: correlation_id.into(),
            data,
            timestamp: Utc::now(),
        }
    }

    /// Create an external SNS event
    pub fn _external_event(
        source: impl Into<String>,
        event_type: impl Into<String>,
        payload: Value,
    ) -> Self {
        Self::ExternalEvent {
            source: source.into(),
            event_type: event_type.into(),
            payload,
            timestamp: Utc::now(),
        }
    }

    /// Create a server lifecycle event
    pub fn _server_lifecycle(event: LifecycleEvent, instance_id: impl Into<String>) -> Self {
        Self::ServerLifecycle {
            event,
            instance_id: instance_id.into(),
            timestamp: Utc::now(),
        }
    }

    /// Convert to SSE-formatted message
    pub fn to_sse_message(&self) -> String {
        match serde_json::to_string(self) {
            Ok(json_str) => format!("data: {}\\n\\n", json_str),
            Err(e) => {
                error!("Failed to serialize event for SSE: {:?}", e);
                format!("data: {{\"type\":\"error\",\"message\":\"Serialization failed\"}}\\n\\n")
            }
        }
    }

    /// Get event type as string
    pub fn event_type(&self) -> &'static str {
        match self {
            GlobalEvent::SystemHealth { .. } => "system_health",
            GlobalEvent::ToolExecution { .. } => "tool_execution",
            GlobalEvent::SessionUpdate { .. } => "session_update",
            GlobalEvent::MonitoringUpdate { .. } => "monitoring_update",
            GlobalEvent::ExternalEvent { .. } => "external_event",
            GlobalEvent::ServerLifecycle { .. } => "server_lifecycle",
        }
    }
}

/// Initialize the global event broadcasting system
pub fn init_global_events() -> broadcast::Receiver<GlobalEvent> {
    let (tx, rx) = broadcast::channel(GLOBAL_EVENT_CAPACITY);
    
    // Store the sender globally for broadcasting
    if GLOBAL_EVENT_CHANNEL.set(tx).is_err() {
        warn!("Global event channel was already initialized");
    }
    
    info!("Initialized global event broadcasting system with capacity: {}", GLOBAL_EVENT_CAPACITY);
    rx
}

/// Initialize global events (alias for convenience)
pub fn initialize_global_events() {
    let _ = init_global_events();
}

/// Broadcast a global event to all subscribers
/// Returns Ok(0) if no subscribers (normal case), Ok(n) if broadcast to n receivers
pub async fn broadcast_global_event(event: GlobalEvent) -> Result<usize, broadcast::error::SendError<GlobalEvent>> {
    if let Some(sender) = GLOBAL_EVENT_CHANNEL.get() {
        let subscriber_count = sender.receiver_count();
        debug!("Broadcasting {} event to {} subscribers", event.event_type(), subscriber_count);
        
        // If no subscribers, this is normal - just log and return success
        if subscriber_count == 0 {
            debug!("No active subscribers for {} event - this is normal", event.event_type());
            return Ok(0);
        }
        
        match sender.send(event) {
            Ok(receiver_count) => {
                debug!("Successfully broadcast event to {} receivers", receiver_count);
                Ok(receiver_count)
            }
            Err(e) => {
                // This should not happen if subscriber_count > 0, but handle gracefully
                warn!("Failed to broadcast global event despite having {} subscribers: {:?}", subscriber_count, e);
                Ok(0) // Return success to not break tool execution
            }
        }
    } else {
        error!("Global event channel not initialized");
        Err(broadcast::error::SendError(event))
    }
}

/// Get a new receiver for global events
pub fn subscribe_to_global_events() -> Option<broadcast::Receiver<GlobalEvent>> {
    GLOBAL_EVENT_CHANNEL.get().map(|sender| sender.subscribe())
}

/// Get the number of active subscribers
pub fn get_subscriber_count() -> usize {
    GLOBAL_EVENT_CHANNEL
        .get()
        .map(|sender| sender.receiver_count())
        .unwrap_or(0)
}

/// Process external SNS event and broadcast internally
pub async fn _process_external_event(
    source: impl Into<String>,
    event_type: impl Into<String>,
    payload: Value,
) -> Result<usize, broadcast::error::SendError<GlobalEvent>> {
    let event = GlobalEvent::_external_event(source, event_type, payload);
    broadcast_global_event(event).await
}

/// Broadcast tool execution progress (for streaming tool responses)
pub async fn broadcast_tool_progress(
    tool_name: impl Into<String>,
    session_id: impl Into<String>,
    status: ToolExecutionStatus,
    result: Option<Value>,
) -> Result<usize, broadcast::error::SendError<GlobalEvent>> {
    let event = GlobalEvent::tool_execution(tool_name, session_id, status, result);
    broadcast_global_event(event).await
}

/// Broadcast system health update
pub async fn _broadcast_system_health(
    component: impl Into<String>,
    status: impl Into<String>,
    details: Value,
) -> Result<usize, broadcast::error::SendError<GlobalEvent>> {
    let event = GlobalEvent::system_health(component, status, details);
    broadcast_global_event(event).await
}

/// Broadcast session management event
pub async fn broadcast_session_event(
    session_id: impl Into<String>,
    event_type: SessionEventType,
    data: Option<Value>,
) -> Result<usize, broadcast::error::SendError<GlobalEvent>> {
    let event = GlobalEvent::session_update(session_id, event_type, data);
    broadcast_global_event(event).await
}

/// Broadcast monitoring update from AWS tools
pub async fn broadcast_monitoring_update(
    resource_type: impl Into<String>,
    region: impl Into<String>,
    correlation_id: impl Into<String>,
    data: Value,
) -> Result<usize, broadcast::error::SendError<GlobalEvent>> {
    let event = GlobalEvent::monitoring_update(resource_type, region, correlation_id, data);
    broadcast_global_event(event).await
}

/// Event filtering for SSE streams
#[derive(Debug, Clone)]
pub struct EventFilter {
    /// Event types to include
    pub event_types: Option<Vec<String>>,
    /// Session ID filter (only events for specific session)
    pub session_id: Option<String>,
    /// Source filter for external events
    pub source_filter: Option<String>,
}

impl EventFilter {
    /// Create a new event filter
    pub fn new() -> Self {
        Self {
            event_types: None,
            session_id: None,
            source_filter: None,
        }
    }

    /// Filter by event types
    #[allow(dead_code)]
    pub fn with_event_types(mut self, types: Vec<String>) -> Self {
        debug!("Setting event type filter: {:?}", types);
        self.event_types = Some(types);
        self
    }

    /// Filter by session ID
    pub fn with_session_id(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    /// Filter by source
    #[allow(dead_code)]
    pub fn with_source(mut self, source: String) -> Self {
        debug!("Setting source filter: {}", source);
        self.source_filter = Some(source);
        self
    }

    /// Check if event matches filter
    pub fn matches(&self, event: &GlobalEvent) -> bool {
        // Check event type filter
        if let Some(ref types) = self.event_types {
            if !types.contains(&event.event_type().to_string()) {
                return false;
            }
        }

        // Check session ID filter
        if let Some(ref session_id) = self.session_id {
            match event {
                GlobalEvent::ToolExecution { session_id: event_session, .. } => {
                    if session_id != event_session {
                        return false;
                    }
                }
                GlobalEvent::SessionUpdate { session_id: event_session, .. } => {
                    if session_id != event_session {
                        return false;
                    }
                }
                _ => {} // Other events don't have session IDs
            }
        }

        // Check source filter
        if let Some(ref source) = self.source_filter {
            if let GlobalEvent::ExternalEvent { source: event_source, .. } = event {
                if source != event_source {
                    return false;
                }
            } else {
                return false; // Only external events have sources
            }
        }

        true
    }
}

impl Default for EventFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_global_event_creation() {
        let event = GlobalEvent::system_health("system", "healthy", json!({"cpu": 45.2}));
        assert_eq!(event.event_type(), "system_health");
        
        let event = GlobalEvent::tool_execution(
            "aws_monitor", 
            "session-123", 
            ToolExecutionStatus::Completed, 
            Some(json!({"result": "success"}))
        );
        assert_eq!(event.event_type(), "tool_execution");
    }

    #[test]
    fn test_event_filter() {
        let filter = EventFilter::new()
            .with_event_types(vec!["tool_execution".to_string()])
            .with_session_id("session-123".to_string());

        let matching_event = GlobalEvent::tool_execution(
            "test_tool",
            "session-123",
            ToolExecutionStatus::Started,
            None,
        );

        let non_matching_event = GlobalEvent::system_health("system", "healthy", json!({}));

        assert!(filter.matches(&matching_event));
        assert!(!filter.matches(&non_matching_event));
    }

    #[test]
    fn test_sse_message_format() {
        let event = GlobalEvent::system_health("system", "healthy", json!({"cpu": 45.2}));
        let sse_message = event.to_sse_message();
        
        assert!(sse_message.starts_with("data: "));
        assert!(sse_message.ends_with("\\n\\n"));
        assert!(sse_message.contains("system_health"));
    }

    #[tokio::test]
    async fn test_event_broadcasting() {
        let _rx = init_global_events();
        
        let event = GlobalEvent::system_health("system", "test", json!({}));
        let result = broadcast_global_event(event).await;
        
        // Should succeed even with no active receivers
        assert!(result.is_ok());
    }
}