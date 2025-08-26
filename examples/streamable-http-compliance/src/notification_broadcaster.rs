//! Notification Broadcasting Service for MCP 2025-06-18 SSE Compliance
//!
//! This module provides a service for broadcasting notifications over SSE streams
//! according to the MCP specification, including:
//! - Progress notifications with progressToken tracking
//! - System notifications with fan-out to all sessions
//! - Session-specific notifications
//! - Bidirectional notification support

use std::sync::Arc;
use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::broadcast;
use tracing::{debug, info, error};

/// Notification broadcaster trait for sending notifications over SSE
#[async_trait]
pub trait NotificationBroadcaster: Send + Sync {
    /// Send a progress notification to a specific session
    async fn send_progress_notification(
        &self,
        session_id: &str,
        progress_token: &str,
        progress: u64,
        total: Option<u64>,
        message: Option<String>,
    ) -> Result<(), BroadcastError>;
    
    /// Send a system notification to all sessions (fan-out)
    async fn send_system_notification(
        &self,
        notification_type: &str,
        component: &str,
        status: &str,
        value: f64,
    ) -> Result<Vec<String>, BroadcastError>; // Returns failed session IDs
    
    /// Send a session-specific notification
    async fn send_session_notification(
        &self,
        session_id: &str,
        action: &str,
        details: &str,
    ) -> Result<(), BroadcastError>;
    
    /// Send raw notification data to specific session
    async fn send_raw_notification(
        &self,
        session_id: &str,
        event_type: &str,
        data: Value,
    ) -> Result<u64, BroadcastError>; // Returns event ID
}

/// Errors that can occur during notification broadcasting
#[derive(Debug, thiserror::Error)]
pub enum BroadcastError {
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    #[error("Broadcasting failed: {0}")]
    BroadcastFailed(String),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

/// In-memory notification broadcaster using broadcast channels
pub struct ChannelNotificationBroadcaster {
    /// Per-session broadcast channels
    session_channels: Arc<tokio::sync::RwLock<std::collections::HashMap<String, broadcast::Sender<NotificationEvent>>>>,
    /// Configuration
    channel_buffer_size: usize,
}

/// SSE notification event structure
#[derive(Debug, Clone)]
pub struct NotificationEvent {
    pub id: u64,
    pub timestamp: u64,
    pub session_id: String,
    pub event_type: String,
    pub data: Value,
    pub retry: Option<u64>,
}

impl NotificationEvent {
    /// Format as SSE event string
    pub fn format_sse(&self) -> String {
        let mut sse_output = String::new();
        
        // Event ID for resumability
        sse_output.push_str(&format!("id: {}\n", self.id));
        
        // Event type
        sse_output.push_str(&format!("event: {}\n", self.event_type));
        
        // Data payload
        let data_json = serde_json::to_string(&self.data).unwrap_or_else(|_| "{}".to_string());
        sse_output.push_str(&format!("data: {}\n", data_json));
        
        // Retry interval if specified
        if let Some(retry) = self.retry {
            sse_output.push_str(&format!("retry: {}\n", retry));
        }
        
        // End event with blank line
        sse_output.push('\n');
        
        sse_output
    }
}

impl ChannelNotificationBroadcaster {
    /// Create new notification broadcaster with default configuration
    pub fn new() -> Self {
        Self {
            session_channels: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            channel_buffer_size: 1000,
        }
    }
    
    /// Create broadcaster with custom buffer size
    pub fn with_buffer_size(buffer_size: usize) -> Self {
        Self {
            session_channels: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            channel_buffer_size: buffer_size,
        }
    }
    
    /// Get or create broadcast channel for a session
    async fn get_or_create_channel(&self, session_id: &str) -> broadcast::Sender<NotificationEvent> {
        let mut channels = self.session_channels.write().await;
        
        if let Some(sender) = channels.get(session_id) {
            sender.clone()
        } else {
            let (sender, _receiver) = broadcast::channel(self.channel_buffer_size);
            channels.insert(session_id.to_string(), sender.clone());
            debug!("Created notification channel for session: {}", session_id);
            sender
        }
    }
    
    /// Subscribe to notifications for a session
    pub async fn subscribe(&self, session_id: &str) -> broadcast::Receiver<NotificationEvent> {
        let sender = self.get_or_create_channel(session_id).await;
        sender.subscribe()
    }
    
    /// Generate unique event ID (simple counter-based implementation)
    fn generate_event_id(&self) -> u64 {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }
    
    /// Get list of active session IDs
    pub async fn get_active_sessions(&self) -> Vec<String> {
        let channels = self.session_channels.read().await;
        channels.keys().cloned().collect()
    }
    
    /// Clean up inactive channels
    pub async fn cleanup_inactive_channels(&self) -> usize {
        let mut channels = self.session_channels.write().await;
        let initial_count = channels.len();
        
        channels.retain(|session_id, sender| {
            let has_receivers = sender.receiver_count() > 0;
            if !has_receivers {
                debug!("Cleaned up notification channel for session: {}", session_id);
            }
            has_receivers
        });
        
        let cleaned_count = initial_count - channels.len();
        if cleaned_count > 0 {
            info!("Cleaned up {} inactive notification channels", cleaned_count);
        }
        
        cleaned_count
    }
}

#[async_trait]
impl NotificationBroadcaster for ChannelNotificationBroadcaster {
    async fn send_progress_notification(
        &self,
        session_id: &str,
        progress_token: &str,
        progress: u64,
        total: Option<u64>,
        message: Option<String>,
    ) -> Result<(), BroadcastError> {
        let data = serde_json::json!({
            "type": "progress",
            "progressToken": progress_token,
            "progress": progress,
            "total": total,
            "message": message
        });
        
        let event = NotificationEvent {
            id: self.generate_event_id(),
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            session_id: session_id.to_string(),
            event_type: "progress".to_string(),
            data,
            retry: Some(5000),
        };
        
        let sender = self.get_or_create_channel(session_id).await;
        sender.send(event.clone()).map_err(|_| {
            BroadcastError::BroadcastFailed(format!("Failed to send progress notification to session: {}", session_id))
        })?;
        
        info!("📊 Sent progress notification: session={}, token={}, progress={}/{:?}", 
              session_id, progress_token, progress, total);
        
        Ok(())
    }
    
    async fn send_system_notification(
        &self,
        notification_type: &str,
        component: &str,
        status: &str,
        value: f64,
    ) -> Result<Vec<String>, BroadcastError> {
        let data = serde_json::json!({
            "type": "system",
            "notificationType": notification_type,
            "component": component,
            "status": status,
            "value": value,
            "timestamp": chrono::Utc::now().timestamp()
        });
        
        let sessions = self.get_active_sessions().await;
        let mut failed_sessions = Vec::new();
        
        for session_id in &sessions {
            let event = NotificationEvent {
                id: self.generate_event_id(),
                timestamp: chrono::Utc::now().timestamp_millis() as u64,
                session_id: session_id.clone(),
                event_type: "system".to_string(),
                data: data.clone(),
                retry: Some(3000),
            };
            
            if let Some(sender) = self.session_channels.read().await.get(session_id) {
                if let Err(_) = sender.send(event) {
                    failed_sessions.push(session_id.clone());
                }
            } else {
                failed_sessions.push(session_id.clone());
            }
        }
        
        info!("🔔 Sent system notification: type={}, component={}, status={}, value={:.2} to {} sessions ({} failed)", 
              notification_type, component, status, value, sessions.len(), failed_sessions.len());
        
        Ok(failed_sessions)
    }
    
    async fn send_session_notification(
        &self,
        session_id: &str,
        action: &str,
        details: &str,
    ) -> Result<(), BroadcastError> {
        let data = serde_json::json!({
            "type": "session",
            "action": action,
            "details": details,
            "timestamp": chrono::Utc::now().timestamp()
        });
        
        let event = NotificationEvent {
            id: self.generate_event_id(),
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            session_id: session_id.to_string(),
            event_type: "session".to_string(),
            data,
            retry: Some(3000),
        };
        
        let sender = self.get_or_create_channel(session_id).await;
        sender.send(event).map_err(|_| {
            BroadcastError::BroadcastFailed(format!("Failed to send session notification to session: {}", session_id))
        })?;
        
        info!("🎯 Sent session notification: session={}, action={}, details={}", 
              session_id, action, details);
        
        Ok(())
    }
    
    async fn send_raw_notification(
        &self,
        session_id: &str,
        event_type: &str,
        data: Value,
    ) -> Result<u64, BroadcastError> {
        let event = NotificationEvent {
            id: self.generate_event_id(),
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            session_id: session_id.to_string(),
            event_type: event_type.to_string(),
            data,
            retry: Some(3000),
        };
        
        let event_id = event.id;
        let sender = self.get_or_create_channel(session_id).await;
        sender.send(event).map_err(|_| {
            BroadcastError::BroadcastFailed(format!("Failed to send raw notification to session: {}", session_id))
        })?;
        
        debug!("📤 Sent raw notification: session={}, event_type={}, event_id={}", 
               session_id, event_type, event_id);
        
        Ok(event_id)
    }
}

impl Default for ChannelNotificationBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}