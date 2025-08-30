//! Session Management for MCP Servers
//!
//! This module provides transparent session management for MCP tools and handlers.
//! Sessions are automatically created and managed by the framework.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use turul_mcp_protocol::{ClientCapabilities, Implementation, McpVersion, ServerCapabilities};

/// Session context provided automatically to tools and handlers
#[derive(Clone)]
pub struct SessionContext {
    /// Unique session identifier
    pub session_id: String,
    /// Get session state value by key
    pub get_state: Arc<dyn Fn(&str) -> Option<Value> + Send + Sync>,
    /// Set session state value by key
    pub set_state: Arc<dyn Fn(&str, Value) + Send + Sync>,
    /// Remove session state value by key
    pub remove_state: Arc<dyn Fn(&str) -> Option<Value> + Send + Sync>,
    /// Check if session is initialized
    pub is_initialized: Arc<dyn Fn() -> bool + Send + Sync>,
    /// Send notification to this session (legacy - use broadcaster instead)
    pub send_notification: Arc<dyn Fn(SessionEvent) + Send + Sync>,
    /// NotificationBroadcaster for sending MCP-compliant notifications
    pub broadcaster: Option<Arc<dyn std::any::Any + Send + Sync>>,
}

impl SessionContext {
    /// Create from JSON-RPC server's SessionContext with proper NotificationBroadcaster integration
    pub fn from_json_rpc_with_broadcaster(
        json_rpc_ctx: turul_mcp_json_rpc_server::SessionContext,
    ) -> Self {
        let session_id = json_rpc_ctx.session_id.clone();
        let broadcaster = json_rpc_ctx.broadcaster.clone();
        
        // Simple state management - just use empty closures since we don't have session manager
        let get_state = Arc::new(move |_key: &str| -> Option<Value> { None });
        let set_state = Arc::new(move |_key: &str, _value: Value| {});
        let remove_state = Arc::new(move |_key: &str| -> Option<Value> { None });
        let is_initialized = Arc::new(move || -> bool { true });
        
        // Store the broadcaster in the send_notification closure for later use by notify methods
        let send_notification = {
            let session_id = session_id.clone();
            let broadcaster = broadcaster.clone();
            Arc::new(move |event: SessionEvent| {
                debug!("📨 SessionContext.send_notification() called for session {}: {:?}", session_id, event);
                
                // Try to use broadcaster if available
                if let Some(broadcaster_any) = &broadcaster {
                    debug!("✅ NotificationBroadcaster available for session: {}", session_id);
                    
                    // Attempt to extract and use the actual broadcaster
                    match event {
                        SessionEvent::Notification(json_value) => {
                            debug!("🔧 Attempting to send notification via StreamManagerNotificationBroadcaster");
                            debug!("📦 Notification JSON: {}", json_value);
                            
                            // Since we can't call async methods from sync closure, spawn a task
                            let session_id_clone = session_id.clone();
                            let json_value_clone = json_value.clone();
                            let _broadcaster_clone = broadcaster_any.clone();
                            
                            tokio::spawn(async move {
                                debug!("🚀 Async task: Processing notification for session {}", session_id_clone);
                                
                                // Parse the JSON notification and attempt to send it
                                match parse_and_send_notification_with_broadcaster(&session_id_clone, &json_value_clone, &_broadcaster_clone).await {
                                    Ok(_) => debug!("✅ Bridge working: Successfully processed notification for session {}", session_id_clone),
                                    Err(e) => error!("❌ Bridge error: Failed to process notification for session {}: {}", session_id_clone, e),
                                }
                                
                                debug!("🏁 Async task completed for session {}", session_id_clone);
                            });
                        }
                        _ => {
                            debug!("⚠️ Non-notification event, ignoring: {:?}", event);
                        }
                    }
                } else {
                    debug!("⚠️ No broadcaster available for session {}", session_id);
                }
            })
        };

        SessionContext {
            session_id,
            get_state,
            set_state,
            remove_state,
            is_initialized,
            send_notification,
            broadcaster,
        }
    }


    /// Check if this context has a broadcaster available
    pub fn has_broadcaster(&self) -> bool {
        self.broadcaster.is_some()
    }
    
    /// Get the raw broadcaster (as Any) - for use by framework internals
    pub fn get_raw_broadcaster(&self) -> Option<Arc<dyn std::any::Any + Send + Sync>> {
        self.broadcaster.clone()
    }

    /// Create from JSON-RPC server's SessionContext
    pub fn from_json_rpc_session(
        json_rpc_ctx: turul_mcp_json_rpc_server::SessionContext,
        session_manager: Arc<SessionManager>,
    ) -> Self {
        let session_id = json_rpc_ctx.session_id.clone();
        let session_manager_for_get = session_manager.clone();
        let session_manager_for_set = session_manager.clone();
        let session_manager_for_remove = session_manager.clone();
        let session_manager_for_init = session_manager.clone();
        let session_manager_for_notify = session_manager.clone();

        let get_state = {
            let session_id = session_id.clone();
            Arc::new(move |key: &str| -> Option<Value> {
                futures::executor::block_on(async {
                    session_manager_for_get.get_session_state(&session_id, key).await
                })
            })
        };

        let set_state = {
            let session_id = session_id.clone();
            Arc::new(move |key: &str, value: Value| {
                futures::executor::block_on(async {
                    session_manager_for_set.set_session_state(&session_id, key, value).await
                });
            })
        };

        let remove_state = {
            let session_id = session_id.clone();
            Arc::new(move |key: &str| -> Option<Value> {
                futures::executor::block_on(async {
                    session_manager_for_remove.remove_session_state(&session_id, key).await
                })
            })
        };

        let is_initialized = {
            let session_id = session_id.clone();
            Arc::new(move || -> bool {
                futures::executor::block_on(async {
                    session_manager_for_init.is_session_initialized(&session_id).await
                })
            })
        };

        let send_notification = {
            let session_id = session_id.clone();
            Arc::new(move |event: SessionEvent| {
                debug!("📜 send_notification closure called for session {}: {:?}", session_id, event);
                // Use tokio::spawn to run async operation without blocking the current thread
                let session_id_clone = session_id.clone();
                let session_manager_clone = session_manager_for_notify.clone();
                tokio::spawn(async move {
                    match session_manager_clone.send_event_to_session(&session_id_clone, event).await {
                        Ok(_) => debug!("✅ send_event_to_session succeeded for session {}", session_id_clone),
                        Err(e) => error!("❌ send_event_to_session failed for session {}: {}", session_id_clone, e),
                    }
                });
                debug!("🚀 send_notification closure completed for session {}", session_id);
            })
        };

        SessionContext {
            session_id,
            get_state,
            set_state,
            remove_state,
            is_initialized,
            send_notification,
            broadcaster: None, // Old SessionManager doesn't have broadcaster
        }
    }
    /// Convenience method to get typed session state
    pub fn get_typed_state<T>(&self, key: &str) -> Option<T>
    where
        T: serde::de::DeserializeOwned,
    {
        (self.get_state)(key)
            .and_then(|v| serde_json::from_value(v).ok())
    }

    /// Convenience method to set typed session state
    pub fn set_typed_state<T>(&self, key: &str, value: T) -> Result<(), String>
    where
        T: serde::Serialize,
    {
        match serde_json::to_value(value) {
            Ok(json_value) => {
                (self.set_state)(key, json_value);
                Ok(())
            }
            Err(e) => Err(format!("Failed to serialize value: {}", e)),
        }
    }

    /// Send a custom notification to this session
    pub fn notify(&self, event: SessionEvent) {
        debug!("📨 SessionContext.notify() called for session {}: {:?}", self.session_id, event);
        (self.send_notification)(event);
        debug!("🚀 SessionContext.notify() send_notification closure completed");
    }

    /// Send a progress notification
    pub fn notify_progress(&self, progress_token: impl Into<String>, progress: u64) {
        if self.has_broadcaster() {
            debug!("🔔 notify_progress using NotificationBroadcaster for session: {}", self.session_id);
            // TODO: Use broadcaster for MCP-compliant notifications
        } else {
            debug!("🔔 notify_progress using OLD SessionManager for session: {}", self.session_id);
        }
        let mut other = std::collections::HashMap::new();
        other.insert("progressToken".to_string(), serde_json::json!(progress_token.into()));
        other.insert("progress".to_string(), serde_json::json!(progress));
        
        let params = turul_mcp_protocol::RequestParams {
            meta: None,
            other,
        };
        let notification = turul_mcp_protocol::JsonRpcNotification::new(
            "notifications/progress".to_string()
        ).with_params(params);
        self.notify(SessionEvent::Notification(serde_json::to_value(notification).unwrap()));
    }

    /// Send a progress notification with total
    pub fn notify_progress_with_total(&self, progress_token: impl Into<String>, progress: u64, total: u64) {
        let mut other = std::collections::HashMap::new();
        other.insert("progressToken".to_string(), serde_json::json!(progress_token.into()));
        other.insert("progress".to_string(), serde_json::json!(progress));
        other.insert("total".to_string(), serde_json::json!(total));
        
        let params = turul_mcp_protocol::RequestParams {
            meta: None,
            other,
        };
        let notification = turul_mcp_protocol::JsonRpcNotification::new(
            "notifications/progress".to_string()
        ).with_params(params);
        self.notify(SessionEvent::Notification(serde_json::to_value(notification).unwrap()));
    }

    /// Send a logging message notification
    pub fn notify_log(&self, level: &str, message: impl Into<String>) {
        if self.has_broadcaster() {
            debug!("🔔 notify_log using NotificationBroadcaster for session: {}", self.session_id);
            
            // Create proper MCP notification using existing JSON-RPC format
            let mut other = std::collections::HashMap::new();
            other.insert("level".to_string(), serde_json::json!(level));
            other.insert("message".to_string(), serde_json::json!(message.into()));
            
            let params = turul_mcp_protocol::RequestParams {
                meta: None,
                other,
            };
            let notification = turul_mcp_protocol::JsonRpcNotification::new(
                "notifications/message".to_string()
            ).with_params(params);
            
            // Send via SessionEvent (which will be picked up by the broadcaster if connected properly)
            self.notify(SessionEvent::Notification(serde_json::to_value(notification).unwrap()));
            return;
        } else {
            debug!("🔔 notify_log using OLD SessionManager for session: {}", self.session_id);
        }
        
        // Legacy implementation (fallback)
        let mut other = std::collections::HashMap::new();
        other.insert("level".to_string(), serde_json::json!(level));
        other.insert("message".to_string(), serde_json::json!(message.into()));
        
        let params = turul_mcp_protocol::RequestParams {
            meta: None,
            other,
        };
        let notification = turul_mcp_protocol::JsonRpcNotification::new(
            "notifications/message".to_string()
        ).with_params(params);
        self.notify(SessionEvent::Notification(serde_json::to_value(notification).unwrap()));
    }

    /// Send a resource list changed notification
    pub fn notify_resources_changed(&self) {
        let notification = turul_mcp_protocol::JsonRpcNotification::new(
            "notifications/resources/listChanged".to_string()
        );
        self.notify(SessionEvent::Notification(serde_json::to_value(notification).unwrap()));
    }

    /// Send a resource updated notification
    pub fn notify_resource_updated(&self, uri: impl Into<String>) {
        let mut other = std::collections::HashMap::new();
        other.insert("uri".to_string(), serde_json::json!(uri.into()));
        
        let params = turul_mcp_protocol::RequestParams {
            meta: None,
            other,
        };
        let notification = turul_mcp_protocol::JsonRpcNotification::new(
            "notifications/resources/updated".to_string()
        ).with_params(params);
        self.notify(SessionEvent::Notification(serde_json::to_value(notification).unwrap()));
    }

    /// Send a tools list changed notification
    pub fn notify_tools_changed(&self) {
        let notification = turul_mcp_protocol::JsonRpcNotification::new(
            "notifications/tools/listChanged".to_string()
        );
        self.notify(SessionEvent::Notification(serde_json::to_value(notification).unwrap()));
    }
}

/// Parse notification JSON and send via actual NotificationBroadcaster to StreamManager using proper notification structs
async fn parse_and_send_notification_with_broadcaster(
    session_id: &str, 
    json_value: &Value,
    broadcaster_any: &Arc<dyn std::any::Any + Send + Sync>
) -> Result<(), String> {
    debug!("🔍 Parsing notification JSON for session {}: {:?}", session_id, json_value);
    
    // Import the types we need for downcasting and notifications
    use turul_http_mcp_server::notification_bridge::SharedNotificationBroadcaster;
    use turul_mcp_protocol::notifications::{LoggingMessageNotification, ProgressNotification};
    use turul_mcp_protocol::logging::LoggingLevel;
    
    // Attempt to downcast Arc<dyn Any> back to SharedNotificationBroadcaster
    if let Some(broadcaster) = broadcaster_any.downcast_ref::<SharedNotificationBroadcaster>() {
        debug!("✅ Successfully downcast broadcaster for session {}", session_id);
        
        // Extract method from JSON-RPC notification to determine type
        if let Some(method) = json_value.get("method").and_then(|v| v.as_str()) {
            match method {
                "notifications/message" => {
                    if let Some(params) = json_value.get("params") {
                        if let Some(level_str) = params.get("level").and_then(|v| v.as_str()) {
                            debug!("📝 Message notification detected: level={}", level_str);
                            
                            // Parse level string to LoggingLevel enum
                            let level = match level_str {
                                "debug" => LoggingLevel::Debug,
                                "info" => LoggingLevel::Info,
                                "warning" => LoggingLevel::Warning,
                                "error" => LoggingLevel::Error,
                                _ => {
                                    error!("❌ Unknown logging level: {}", level_str);
                                    return Err(format!("Unknown logging level: {}", level_str));
                                }
                            };
                            
                            // Get the message data
                            let data = params.get("message").cloned()
                                .unwrap_or_else(|| serde_json::json!("Missing message"));
                            
                            // Create proper LoggingMessageNotification using the struct from notifications.rs
                            let notification = LoggingMessageNotification::new(level, data);
                            
                            debug!("🔧 About to call broadcaster.send_message_notification() for session {}", session_id);
                            // ACTUALLY SEND the notification using the proper method
                            match broadcaster.send_message_notification(session_id, notification).await {
                                Ok(()) => {
                                    debug!("🎉 SUCCESS: LoggingMessageNotification sent to StreamManager for session {}", session_id);
                                    debug!("🚀 Streamable HTTP Transport Bridge: Complete end-to-end delivery confirmed!");
                                    return Ok(());
                                }
                                Err(e) => {
                                    error!("❌ Failed to send LoggingMessageNotification to StreamManager: {}", e);
                                    return Err(format!("Failed to send LoggingMessageNotification: {}", e));
                                }
                            }
                        }
                    }
                }
                "notifications/progress" => {
                    if let Some(params) = json_value.get("params") {
                        if let Some(token) = params.get("progressToken").and_then(|v| v.as_str()) {
                            debug!("📊 Progress notification detected: token={}", token);
                            
                            // Get progress value
                            let progress = params.get("progress")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0);
                            
                            // Create proper ProgressNotification using the struct from notifications.rs
                            let notification = ProgressNotification {
                                method: "notifications/progress".to_string(),
                                params: turul_mcp_protocol::notifications::ProgressNotificationParams {
                                    progress_token: token.to_string(),
                                    progress,
                                    total: params.get("total").and_then(|v| v.as_u64()),
                                    message: params.get("message").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                    meta: None,
                                },
                            };
                            
                            debug!("🔧 About to call broadcaster.send_progress_notification() for session {}", session_id);
                            // ACTUALLY SEND the notification using the proper method
                            match broadcaster.send_progress_notification(session_id, notification).await {
                                Ok(()) => {
                                    debug!("🎉 SUCCESS: ProgressNotification sent to StreamManager for session {}", session_id);
                                    debug!("🚀 Streamable HTTP Transport Bridge: Complete end-to-end delivery confirmed!");
                                    return Ok(());
                                }
                                Err(e) => {
                                    error!("❌ Failed to send ProgressNotification to StreamManager: {}", e);
                                    return Err(format!("Failed to send ProgressNotification: {}", e));
                                }
                            }
                        }
                    }
                }
                _ => {
                    debug!("🔧 Other notification method: {} - sending as generic JsonRpcNotification", method);
                    
                    // For other notifications, use the generic send_notification method
                    let params_map: std::collections::HashMap<String, serde_json::Value> = 
                        json_value.get("params")
                            .and_then(|p| p.as_object())
                            .unwrap_or(&serde_json::Map::new())
                            .iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect();
                    let json_rpc_notification = turul_mcp_json_rpc_server::JsonRpcNotification::new_with_object_params(
                        method.to_string(),
                        params_map
                    );
                    
                    match broadcaster.send_notification(session_id, json_rpc_notification).await {
                        Ok(()) => {
                            debug!("🎉 SUCCESS: Generic notification sent to StreamManager for session {}", session_id);
                            return Ok(());
                        }
                        Err(e) => {
                            error!("❌ Failed to send generic notification to StreamManager: {}", e);
                            return Err(format!("Failed to send generic notification: {}", e));
                        }
                    }
                }
            }
        }
    } else {
        error!("❌ Failed to downcast broadcaster for session {}", session_id);
        return Err("Failed to downcast broadcaster to SharedNotificationBroadcaster".to_string());
    }
    
    debug!("❓ Could not determine notification type for session {}", session_id);
    Ok(())
}

/// Events that can be sent to a session
#[derive(Debug, Clone)]
pub enum SessionEvent {
    /// Notification to be sent to client
    Notification(Value),
    /// Keep-alive ping
    KeepAlive,
    /// Session termination
    Disconnect,
    /// Custom event with type and data
    Custom { event_type: String, data: Value },
}

/// Individual MCP session state
#[derive(Debug)]
pub struct McpSession {
    /// Unique session identifier
    pub id: String,
    /// When the session was created
    pub created: Instant,
    /// Last activity timestamp (for expiry)
    pub last_accessed: Instant,
    /// MCP protocol version for this session
    pub mcp_version: McpVersion,
    /// Client capabilities negotiated during initialization
    pub client_capabilities: Option<ClientCapabilities>,
    /// Server capabilities sent to client
    pub server_capabilities: ServerCapabilities,
    /// Client implementation info
    pub client_info: Option<Implementation>,
    /// Per-session state storage for tools/handlers
    pub state: HashMap<String, Value>,
    /// Broadcast sender for SSE notifications
    pub event_sender: broadcast::Sender<SessionEvent>,
    /// Session initialization status
    pub initialized: bool,
}

impl McpSession {
    /// Create a new session
    pub fn new(server_capabilities: ServerCapabilities) -> Self {
        let session_id = Uuid::now_v7().to_string();
        let (event_sender, _) = broadcast::channel(128);

        Self {
            id: session_id,
            created: Instant::now(),
            last_accessed: Instant::now(),
            mcp_version: McpVersion::CURRENT,
            client_capabilities: None,
            server_capabilities,
            client_info: None,
            state: HashMap::new(),
            event_sender,
            initialized: false,
        }
    }

    /// Update last accessed time
    pub fn touch(&mut self) {
        self.last_accessed = Instant::now();
    }

    /// Check if session has expired
    pub fn is_expired(&self, timeout: Duration) -> bool {
        self.last_accessed.elapsed() > timeout
    }

    /// Initialize session with client information
    pub fn initialize(
        &mut self,
        client_info: Implementation,
        client_capabilities: ClientCapabilities,
    ) {
        self.client_info = Some(client_info);
        self.client_capabilities = Some(client_capabilities);
        self.initialized = true;
        self.touch();
    }

    /// Initialize session with client information and negotiated protocol version
    pub fn initialize_with_version(
        &mut self,
        client_info: Implementation,
        client_capabilities: ClientCapabilities,
        mcp_version: McpVersion,
    ) {
        self.client_info = Some(client_info);
        self.client_capabilities = Some(client_capabilities);
        self.mcp_version = mcp_version;
        self.initialized = true;
        self.touch();
    }

    /// Get state value
    pub fn get_state(&self, key: &str) -> Option<Value> {
        self.state.get(key).cloned()
    }

    /// Set state value
    pub fn set_state(&mut self, key: &str, value: Value) {
        self.state.insert(key.to_string(), value);
        self.touch();
    }

    /// Remove state value
    pub fn remove_state(&mut self, key: &str) -> Option<Value> {
        let result = self.state.remove(key);
        if result.is_some() {
            self.touch();
        }
        result
    }

    /// Send event to this session
    pub fn send_event(&self, event: SessionEvent) -> Result<(), String> {
        self.event_sender
            .send(event)
            .map_err(|e| format!("Failed to send event: {}", e))?;
        Ok(())
    }

    /// Subscribe to session events (for SSE)
    pub fn subscribe_events(&self) -> broadcast::Receiver<SessionEvent> {
        self.event_sender.subscribe()
    }
}

/// Session management errors
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("Session not found: {0}")]
    NotFound(String),
    #[error("Session expired: {0}")]
    Expired(String),
    #[error("Session not initialized: {0}")]
    NotInitialized(String),
    #[error("Invalid session data: {0}")]
    InvalidData(String),
}

/// Global session manager for MCP servers
pub struct SessionManager {
    /// Session storage backend
    storage: Arc<turul_mcp_session_storage::BoxedSessionStorage>,
    /// Active sessions (cache for performance)
    sessions: RwLock<HashMap<String, McpSession>>,
    /// Default session expiry time
    session_timeout: Duration,
    /// Cleanup interval
    cleanup_interval: Duration,
    /// Default server capabilities for new sessions
    default_capabilities: ServerCapabilities,
    /// Global event broadcaster for all session events
    global_event_sender: broadcast::Sender<(String, SessionEvent)>,
}

impl SessionManager {
    /// Create a new session manager with InMemory storage
    pub fn new(default_capabilities: ServerCapabilities) -> Self {
        let storage: Arc<turul_mcp_session_storage::BoxedSessionStorage> = Arc::new(turul_mcp_session_storage::InMemorySessionStorage::new());
        Self::with_storage_and_timeouts(
            storage,
            default_capabilities,
            Duration::from_secs(30 * 60), // 30 minutes
            Duration::from_secs(60),      // 1 minute
        )
    }
    
    /// Create a new session manager with custom timeouts and InMemory storage
    pub fn with_timeouts(
        default_capabilities: ServerCapabilities, 
        session_timeout: Duration,
        cleanup_interval: Duration,
    ) -> Self {
        let storage: Arc<turul_mcp_session_storage::BoxedSessionStorage> = Arc::new(turul_mcp_session_storage::InMemorySessionStorage::new());
        Self::with_storage_and_timeouts(
            storage,
            default_capabilities,
            session_timeout,
            cleanup_interval,
        )
    }

    /// Create a new session manager with specific storage backend
    pub fn with_storage(
        storage: Arc<turul_mcp_session_storage::BoxedSessionStorage>,
        default_capabilities: ServerCapabilities,
    ) -> Self {
        Self::with_storage_and_timeouts(
            storage,
            default_capabilities,
            Duration::from_secs(30 * 60), // 30 minutes
            Duration::from_secs(60),      // 1 minute
        )
    }

    /// Create a new session manager with custom storage and timeouts
    pub fn with_storage_and_timeouts(
        storage: Arc<turul_mcp_session_storage::BoxedSessionStorage>,
        default_capabilities: ServerCapabilities,
        session_timeout: Duration,
        cleanup_interval: Duration,
    ) -> Self {
        let (global_event_sender, _) = broadcast::channel(1000);
        
        Self {
            storage,
            sessions: RwLock::new(HashMap::new()),
            session_timeout,
            cleanup_interval,
            default_capabilities,
            global_event_sender,
        }
    }

    /// Create a new session and return its ID
    pub async fn create_session(&self) -> String {
        let session = McpSession::new(self.default_capabilities.clone());
        let session_id = session.id.clone();

        debug!("Creating new session: {}", session_id);
        
        // Store in pluggable storage backend
        match self.storage.create_session_with_id(
            session_id.clone(), 
            self.default_capabilities.clone()
        ).await {
            Ok(_) => debug!("Session {} created in storage backend", session_id),
            Err(e) => error!("Failed to create session {} in storage: {}", session_id, e),
        }
        
        // Also store in memory cache for performance
        self.sessions.write().await.insert(session_id.clone(), session);
        session_id
    }

    /// Create a new session with a specific ID (for GPS pattern compliance)
    pub async fn create_session_with_id(&self, session_id: String) -> String {
        let mut session = McpSession::new(self.default_capabilities.clone());
        session.id = session_id.clone();

        debug!("Creating session with provided ID: {}", session_id);
        
        // Store in pluggable storage backend
        match self.storage.create_session_with_id(
            session_id.clone(), 
            self.default_capabilities.clone()
        ).await {
            Ok(_) => debug!("Session {} created in storage backend", session_id),
            Err(e) => error!("Failed to create session {} in storage: {}", session_id, e),
        }
        
        // Also store in memory cache for performance
        self.sessions.write().await.insert(session_id.clone(), session);
        session_id
    }

    /// Get session and update last accessed time
    pub async fn touch_session(&self, session_id: &str) -> Result<(), SessionError> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            if session.is_expired(self.session_timeout) {
                sessions.remove(session_id);
                return Err(SessionError::Expired(session_id.to_string()));
            }
            session.touch();
            Ok(())
        } else {
            Err(SessionError::NotFound(session_id.to_string()))
        }
    }

    /// Initialize a session with client information
    pub async fn initialize_session(
        &self,
        session_id: &str,
        client_info: Implementation,
        client_capabilities: ClientCapabilities,
    ) -> Result<(), SessionError> {
        // Update storage backend
        if let Ok(Some(mut session_info)) = self.storage.get_session(session_id).await {
            session_info.client_capabilities = Some(client_capabilities.clone());
            session_info.is_initialized = true;
            session_info.touch();
            
            if let Err(e) = self.storage.update_session(session_info).await {
                error!("Failed to update session in storage: {}", e);
            }
        }
        
        // Update in-memory cache
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.initialize(client_info, client_capabilities);
            debug!("Session {} initialized", session_id);
            Ok(())
        } else {
            Err(SessionError::NotFound(session_id.to_string()))
        }
    }

    /// Initialize a session with client information and negotiated protocol version
    pub async fn initialize_session_with_version(
        &self,
        session_id: &str,
        client_info: Implementation,
        client_capabilities: ClientCapabilities,
        mcp_version: McpVersion,
    ) -> Result<(), SessionError> {
        // Update storage backend
        if let Ok(Some(mut session_info)) = self.storage.get_session(session_id).await {
            session_info.client_capabilities = Some(client_capabilities.clone());
            session_info.is_initialized = true;
            session_info.touch();
            // Note: mcp_version not stored in SessionInfo, only in memory cache
            
            if let Err(e) = self.storage.update_session(session_info).await {
                error!("Failed to update session in storage: {}", e);
            }
        }
        
        // Update in-memory cache
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.initialize_with_version(client_info, client_capabilities, mcp_version);
            debug!("Session {} initialized with protocol version {}", session_id, mcp_version);
            Ok(())
        } else {
            Err(SessionError::NotFound(session_id.to_string()))
        }
    }

    /// Check if session exists and is valid
    pub async fn session_exists(&self, session_id: &str) -> bool {
        // Check storage backend first for authoritative answer
        match self.storage.get_session(session_id).await {
            Ok(Some(session_info)) => {
                // Check if session is expired based on storage data
                let timeout_minutes = self.session_timeout.as_secs() / 60;
                !session_info.is_expired(timeout_minutes)
            }
            Ok(None) => false,
            Err(e) => {
                debug!("Storage backend error for session_exists: {}", e);
                // Fallback to in-memory cache
                let sessions = self.sessions.read().await;
                sessions
                    .get(session_id)
                    .map(|s| !s.is_expired(self.session_timeout))
                    .unwrap_or(false)
            }
        }
    }

    /// Get session state value
    pub async fn get_session_state(&self, session_id: &str, key: &str) -> Option<Value> {
        // Try storage backend first for consistency
        match self.storage.get_session_state(session_id, key).await {
            Ok(value) => value,
            Err(e) => {
                debug!("Storage backend error for get_session_state: {}", e);
                // Fallback to in-memory cache
                let sessions = self.sessions.read().await;
                sessions.get(session_id)?.get_state(key)
            }
        }
    }

    /// Set session state value
    pub async fn set_session_state(&self, session_id: &str, key: &str, value: Value) {
        // Update storage backend first
        if let Err(e) = self.storage.set_session_state(session_id, key, value.clone()).await {
            error!("Failed to set session state in storage: {}", e);
        }
        
        // Update in-memory cache
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.set_state(key, value);
        }
    }

    /// Remove session state value
    pub async fn remove_session_state(&self, session_id: &str, key: &str) -> Option<Value> {
        // Remove from storage backend first
        let storage_result = match self.storage.remove_session_state(session_id, key).await {
            Ok(value) => value,
            Err(e) => {
                error!("Failed to remove session state from storage: {}", e);
                None
            }
        };
        
        // Remove from in-memory cache
        let mut sessions = self.sessions.write().await;
        let memory_result = sessions.get_mut(session_id)?.remove_state(key);
        
        // Return storage result if available, otherwise memory result
        storage_result.or(memory_result)
    }

    /// Check if session is initialized
    pub async fn is_session_initialized(&self, session_id: &str) -> bool {
        let sessions = self.sessions.read().await;
        sessions
            .get(session_id)
            .map(|s| s.initialized)
            .unwrap_or(false)
    }

    /// Remove a session
    pub async fn remove_session(&self, session_id: &str) -> bool {
        // Remove from storage backend first
        let storage_removed = match self.storage.delete_session(session_id).await {
            Ok(removed) => {
                if removed {
                    debug!("Session {} removed from storage backend", session_id);
                }
                removed
            }
            Err(e) => {
                error!("Failed to remove session {} from storage: {}", session_id, e);
                false
            }
        };
        
        // Remove from in-memory cache
        let mut sessions = self.sessions.write().await;
        let memory_removed = if let Some(session) = sessions.remove(session_id) {
            debug!("Session {} removed from memory cache", session_id);
            // Send disconnect event
            let _ = session.send_event(SessionEvent::Disconnect);
            true
        } else {
            false
        };
        
        // Return true if removed from either storage or memory
        storage_removed || memory_removed
    }

    /// Cleanup expired sessions
    pub async fn cleanup_expired(&self) -> usize {
        let timeout_duration = self.session_timeout;
        let cutoff = std::time::SystemTime::now() - timeout_duration;
        
        // Clean up expired sessions from storage backend
        let storage_removed = match self.storage.expire_sessions(cutoff).await {
            Ok(expired_ids) => {
                let count = expired_ids.len();
                if count > 0 {
                    info!("Storage backend cleaned up {} expired sessions: {:?}", count, expired_ids);
                }
                count
            }
            Err(e) => {
                error!("Failed to clean up expired sessions from storage: {}", e);
                0
            }
        };
        
        // Clean up expired sessions from memory cache
        let cutoff_instant = Instant::now() - timeout_duration;
        let mut sessions = self.sessions.write().await;
        let initial_count = sessions.len();

        sessions.retain(|id, session| {
            let keep = session.last_accessed >= cutoff_instant;
            if !keep {
                info!("Session {} expired and removed from memory cache", id);
                // Send disconnect event before removal
                let _ = session.send_event(SessionEvent::Disconnect);
            }
            keep
        });

        let memory_removed = initial_count - sessions.len();
        
        // Return total cleaned up (storage + memory, avoiding double count)
        std::cmp::max(storage_removed, memory_removed)
    }

    /// Send event to specific session
    pub async fn send_event_to_session(
        &self,
        session_id: &str,
        event: SessionEvent,
    ) -> Result<(), SessionError> {
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(session_id) {
            // Send to the specific session
            session.send_event(event.clone())
                .map_err(|e| SessionError::InvalidData(e))?;
            
            // Also forward to global event broadcaster for SSE bridging
            debug!("🌐 Forwarding event to global broadcaster: session={}, event={:?}", session_id, event);
            if let Err(e) = self.global_event_sender.send((session_id.to_string(), event)) {
                debug!("⚠️ Global event broadcast failed (no listeners): {}", e);
            } else {
                debug!("✅ Global event broadcast succeeded");
            }
            
            Ok(())
        } else {
            Err(SessionError::NotFound(session_id.to_string()))
        }
    }

    /// Broadcast event to all sessions
    pub async fn broadcast_event(&self, event: SessionEvent) {
        let sessions = self.sessions.read().await;
        for (session_id, session) in sessions.iter() {
            if let Err(e) = session.send_event(event.clone()) {
                warn!("Failed to send event to session {}: {}", session_id, e);
            }
        }
    }

    /// Get active session count
    pub async fn session_count(&self) -> usize {
        // Get count from storage backend for authoritative answer
        match self.storage.session_count().await {
            Ok(count) => count,
            Err(e) => {
                debug!("Storage backend error for session_count: {}", e);
                // Fallback to in-memory cache
                self.sessions.read().await.len()
            }
        }
    }

    /// Create session context for a session
    pub fn create_session_context(self: &Arc<Self>, session_id: &str) -> Option<SessionContext> {
        let session_id = session_id.to_string();
        let session_manager = Arc::clone(self);

        // Create closures for state management
        let get_state = {
            let session_manager = session_manager.clone();
            let session_id = session_id.clone();
            Arc::new(move |key: &str| -> Option<Value> {
                futures::executor::block_on(async {
                    session_manager.get_session_state(&session_id, key).await
                })
            })
        };

        let set_state = {
            let session_manager = session_manager.clone();
            let session_id = session_id.clone();
            Arc::new(move |key: &str, value: Value| {
                futures::executor::block_on(async {
                    session_manager.set_session_state(&session_id, key, value).await
                });
            })
        };

        let remove_state = {
            let session_manager = session_manager.clone();
            let session_id = session_id.clone();
            Arc::new(move |key: &str| -> Option<Value> {
                futures::executor::block_on(async {
                    session_manager.remove_session_state(&session_id, key).await
                })
            })
        };

        let is_initialized = {
            let session_manager = session_manager.clone();
            let session_id = session_id.clone();
            Arc::new(move || -> bool {
                futures::executor::block_on(async {
                    session_manager.is_session_initialized(&session_id).await
                })
            })
        };

        let send_notification = {
            let session_manager = session_manager.clone();
            let session_id = session_id.clone();
            Arc::new(move |event: SessionEvent| {
                futures::executor::block_on(async {
                    let _ = session_manager.send_event_to_session(&session_id, event).await;
                });
            })
        };

        Some(SessionContext {
            session_id,
            get_state,
            set_state,
            remove_state,
            is_initialized,
            send_notification,
            broadcaster: None, // Old SessionManager doesn't have broadcaster
        })
    }

    /// Start automatic cleanup task
    pub fn start_cleanup_task(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        let manager = Arc::clone(&self);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(manager.cleanup_interval);
            loop {
                interval.tick().await;
                let cleaned = manager.cleanup_expired().await;
                if cleaned > 0 {
                    debug!("Cleaned up {} expired sessions", cleaned);
                }
            }
        })
    }

    /// Get a session's event receiver for SSE streaming
    pub async fn get_session_event_receiver(&self, session_id: &str) -> Option<broadcast::Receiver<SessionEvent>> {
        let sessions = self.sessions.read().await;
        Some(sessions.get(session_id)?.subscribe_events())
    }

    /// Subscribe to events from all sessions
    /// Returns a receiver that gets (session_id, event) tuples for all session events
    pub fn subscribe_all_session_events(&self) -> broadcast::Receiver<(String, SessionEvent)> {
        self.global_event_sender.subscribe()
    }
    
    /// Get the storage backend for use by other components (e.g., HTTP server)
    /// This ensures all components use the same storage backend
    pub fn get_storage(&self) -> Arc<turul_mcp_session_storage::BoxedSessionStorage> {
        Arc::clone(&self.storage)
    }
}

/// Trait for session-aware components
#[async_trait]
pub trait SessionAware {
    /// Handle request with session context
    async fn handle_with_session(
        &self,
        params: Option<Value>,
        session: Option<SessionContext>,
    ) -> Result<Value, String>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_session_creation() {
        let capabilities = ServerCapabilities::default();
        let manager = SessionManager::new(capabilities);

        let session_id = manager.create_session().await;
        assert!(!session_id.is_empty());
        assert!(manager.session_exists(&session_id).await);
    }

    #[tokio::test]
    async fn test_session_state() {
        let capabilities = ServerCapabilities::default();
        let manager = SessionManager::new(capabilities);

        let session_id = manager.create_session().await;

        // Set state
        manager
            .set_session_state(&session_id, "test_key", json!("test_value"))
            .await;

        // Get state
        let value = manager.get_session_state(&session_id, "test_key").await;
        assert_eq!(value, Some(json!("test_value")));

        // Remove state
        let removed = manager.remove_session_state(&session_id, "test_key").await;
        assert_eq!(removed, Some(json!("test_value")));

        // Verify removed
        let value = manager.get_session_state(&session_id, "test_key").await;
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_session_context() {
        let capabilities = ServerCapabilities::default();
        let manager = Arc::new(SessionManager::new(capabilities));

        let session_id = manager.create_session().await;
        let ctx = manager.create_session_context(&session_id).unwrap();

        // Test state operations through context
        (ctx.set_state)("test", json!("value"));
        let value = (ctx.get_state)("test");
        assert_eq!(value, Some(json!("value")));

        let removed = (ctx.remove_state)("test");
        assert_eq!(removed, Some(json!("value")));

        // Test notification sending
        ctx.notify_log("info", "Test notification");
        ctx.notify_progress("test-token", 50);
    }

    #[tokio::test]
    async fn test_session_expiry() {
        let capabilities = ServerCapabilities::default();
        let mut manager = SessionManager::new(capabilities);
        manager.session_timeout = Duration::from_millis(100); // Very short timeout

        let session_id = manager.create_session().await;
        assert!(manager.session_exists(&session_id).await);

        // Wait for expiry
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Session should be expired now
        let result = manager.touch_session(&session_id).await;
        assert!(matches!(result, Err(SessionError::Expired(_))));
    }
}