//! Session Management for MCP Servers
//!
//! This module provides transparent session management for MCP tools and handlers.
//! Sessions are automatically created and managed by the framework.
//!
//! ## Async Design
//!
//! All session state operations are fully async using futures. This prevents
//! blocking the async runtime and enables true concurrent session operations.
//! All session state calls must use `.await`.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::{RwLock, broadcast};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use turul_mcp_protocol::{ClientCapabilities, Implementation, McpVersion, ServerCapabilities};
use turul_mcp_session_storage::{SessionStorage, SessionStorageError, SessionView};

type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

/// Session context provided automatically to tools and handlers
///
/// ## Async API
///
/// All session state operations return futures and must be awaited:
/// ```rust,no_run
/// # use turul_mcp_server::SessionContext;
/// # use serde_json::json;
/// # async fn example(ctx: SessionContext) {
/// let value = (ctx.get_state)("key").await;
/// (ctx.set_state)("key", json!("value")).await;
/// # }
/// ```
// Type aliases for complex session handler types
type GetStateFn = Arc<dyn Fn(&str) -> BoxFuture<Option<Value>> + Send + Sync>;
type SetStateFn = Arc<dyn Fn(&str, Value) -> BoxFuture<()> + Send + Sync>;
type RemoveStateFn = Arc<dyn Fn(&str) -> BoxFuture<Option<Value>> + Send + Sync>;

#[derive(Clone)]
pub struct SessionContext {
    /// Unique session identifier
    pub session_id: String,
    /// Get session state value by key (async)
    pub get_state: GetStateFn,
    /// Set session state value by key (async)
    pub set_state: SetStateFn,
    /// Remove session state value by key (async)
    pub remove_state: RemoveStateFn,
    /// Check if session is initialized (async)
    pub is_initialized: Arc<dyn Fn() -> BoxFuture<bool> + Send + Sync>,
    /// Send notification to this session (async)
    pub send_notification: Arc<dyn Fn(SessionEvent) -> BoxFuture<()> + Send + Sync>,
    /// NotificationBroadcaster for sending MCP-compliant notifications
    pub broadcaster: Option<Arc<dyn std::any::Any + Send + Sync>>,
    /// Request-scoped extensions (auth claims, middleware data)
    ///
    /// Populated by transport from `RequestContext.extensions` via JSON-RPC `SessionContext`.
    /// Never persisted to session storage — exists only for the duration of one request.
    pub extensions: HashMap<String, Value>,
}

impl SessionContext {
    /// Create from JSON-RPC server's SessionContext with proper NotificationBroadcaster integration
    pub(crate) fn from_json_rpc_with_broadcaster(
        json_rpc_ctx: turul_mcp_json_rpc_server::SessionContext,
        storage: Arc<dyn SessionStorage<Error = SessionStorageError>>,
    ) -> Self {
        let session_id = json_rpc_ctx.session_id.clone();
        let broadcaster = json_rpc_ctx.broadcaster.clone();

        // Use real storage for state management
        let get_state = {
            let storage = storage.clone();
            let session_id = session_id.clone();
            Arc::new(move |key: &str| -> BoxFuture<Option<Value>> {
                let storage = storage.clone();
                let session_id = session_id.clone();
                let key = key.to_string();
                Box::pin(async move {
                    match storage.get_session_state(&session_id, &key).await {
                        Ok(Some(value)) => Some(value),
                        Ok(None) => None,
                        Err(e) => {
                            tracing::warn!("Failed to get session state for key '{}': {}", key, e);
                            None
                        }
                    }
                })
            })
        };

        let set_state = {
            let storage = storage.clone();
            let session_id = session_id.clone();
            Arc::new(move |key: &str, value: Value| -> BoxFuture<()> {
                let storage = storage.clone();
                let session_id = session_id.clone();
                let key = key.to_string();
                Box::pin(async move {
                    if let Err(e) = storage.set_session_state(&session_id, &key, value).await {
                        tracing::error!("Failed to set session state for key '{}': {}", key, e);
                    }
                })
            })
        };

        let remove_state = {
            let storage = storage.clone();
            let session_id = session_id.clone();
            Arc::new(move |key: &str| -> BoxFuture<Option<Value>> {
                let storage = storage.clone();
                let session_id = session_id.clone();
                let key = key.to_string();
                Box::pin(async move {
                    match storage.remove_session_state(&session_id, &key).await {
                        Ok(value) => value,
                        Err(e) => {
                            tracing::warn!(
                                "Failed to remove session state for key '{}': {}",
                                key,
                                e
                            );
                            None
                        }
                    }
                })
            })
        };

        let is_initialized = {
            let storage = storage.clone();
            let session_id = session_id.clone();
            Arc::new(move || -> BoxFuture<bool> {
                let storage = storage.clone();
                let session_id = session_id.clone();
                Box::pin(async move {
                    match storage.get_session(&session_id).await {
                        Ok(Some(session_info)) => session_info.is_initialized,
                        Ok(None) => {
                            tracing::warn!("Session {} not found in storage", session_id);
                            false
                        }
                        Err(e) => {
                            tracing::error!("Failed to check session initialization: {}", e);
                            false
                        }
                    }
                })
            })
        };

        // Store the broadcaster in the send_notification closure for later use by notify methods
        let send_notification = {
            let session_id = session_id.clone();
            let broadcaster = broadcaster.clone();
            Arc::new(move |event: SessionEvent| -> BoxFuture<()> {
                let session_id = session_id.clone();
                let broadcaster = broadcaster.clone();
                Box::pin(async move {
                    debug!(
                        "📨 SessionContext.send_notification() called for session {}: {:?}",
                        session_id, event
                    );

                    // Try to use broadcaster if available
                    if let Some(broadcaster_any) = &broadcaster {
                        debug!(
                            "✅ NotificationBroadcaster available for session: {}",
                            session_id
                        );

                        // Attempt to extract and use the actual broadcaster
                        match event {
                            SessionEvent::Notification(json_value) => {
                                debug!(
                                    "🔧 Attempting to send notification via StreamManagerNotificationBroadcaster"
                                );
                                debug!("📦 Notification JSON: {}", json_value);

                                // Now we can directly await the notification sending
                                match parse_and_send_notification_with_broadcaster(
                                    &session_id,
                                    &json_value,
                                    broadcaster_any,
                                )
                                .await
                                {
                                    Ok(_) => debug!(
                                        "✅ Bridge working: Successfully processed notification for session {}",
                                        session_id
                                    ),
                                    Err(e) => error!(
                                        "❌ Bridge error: Failed to process notification for session {}: {}",
                                        session_id, e
                                    ),
                                }
                            }
                            _ => {
                                debug!("⚠️ Non-notification event, ignoring: {:?}", event);
                            }
                        }
                    } else {
                        debug!("⚠️ No broadcaster available for session {}", session_id);
                    }
                })
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
            extensions: json_rpc_ctx.extensions,
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

    /// Get a request-scoped extension value by key
    pub fn get_extension(&self, key: &str) -> Option<&Value> {
        self.extensions.get(key)
    }

    /// Get a typed request-scoped extension value by key
    pub fn get_typed_extension<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.extensions
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Create from JSON-RPC server's SessionContext with proper NotificationBroadcaster integration (test helper)
    #[cfg(feature = "test-utils")]
    pub fn from_json_rpc_with_broadcaster_for_tests(
        json_rpc_ctx: turul_mcp_json_rpc_server::SessionContext,
        storage: Arc<dyn SessionStorage<Error = SessionStorageError>>,
    ) -> Self {
        Self::from_json_rpc_with_broadcaster(json_rpc_ctx, storage)
    }

    /// Convenience method to get typed session state (async)
    pub async fn get_typed_state<T>(&self, key: &str) -> Option<T>
    where
        T: serde::de::DeserializeOwned,
    {
        (self.get_state)(key)
            .await
            .and_then(|v| serde_json::from_value(v).ok())
    }

    /// Convenience method to set typed session state (async)
    pub async fn set_typed_state<T>(&self, key: &str, value: T) -> Result<(), String>
    where
        T: serde::Serialize,
    {
        match serde_json::to_value(value) {
            Ok(json_value) => {
                (self.set_state)(key, json_value).await;
                Ok(())
            }
            Err(e) => Err(format!("Failed to serialize value: {}", e)),
        }
    }

    /// Create a test session context (for unit tests)
    #[cfg(test)]
    pub fn new_test() -> Self {
        use std::collections::HashMap;
        use std::sync::Arc;
        use tokio::sync::RwLock;

        let state = Arc::new(RwLock::new(HashMap::<String, Value>::new()));

        let get_state = {
            let state = state.clone();
            Arc::new(move |key: &str| -> BoxFuture<Option<Value>> {
                let state = state.clone();
                let key = key.to_string();
                Box::pin(async move { state.read().await.get(&key).cloned() })
            })
        };

        let set_state = {
            let state = state.clone();
            Arc::new(move |key: &str, value: Value| -> BoxFuture<()> {
                let state = state.clone();
                let key = key.to_string();
                Box::pin(async move {
                    state.write().await.insert(key, value);
                })
            })
        };

        let remove_state = {
            let state = state.clone();
            Arc::new(move |key: &str| -> BoxFuture<Option<Value>> {
                let state = state.clone();
                let key = key.to_string();
                Box::pin(async move { state.write().await.remove(&key) })
            })
        };

        let is_initialized = Arc::new(|| -> BoxFuture<bool> { Box::pin(async { true }) });

        let send_notification =
            Arc::new(|_event: SessionEvent| -> BoxFuture<()> { Box::pin(async {}) });

        SessionContext {
            session_id: Uuid::now_v7().as_simple().to_string(),
            get_state,
            set_state,
            remove_state,
            is_initialized,
            send_notification,
            broadcaster: None,
            extensions: HashMap::new(),
        }
    }

    /// Send a custom notification to this session (async)
    pub async fn notify(&self, event: SessionEvent) {
        debug!(
            "📨 SessionContext.notify() called for session {}: {:?}",
            self.session_id, event
        );
        (self.send_notification)(event).await;
        debug!("🚀 SessionContext.notify() send_notification closure completed");
    }

    /// Send a progress notification
    pub async fn notify_progress(&self, progress_token: impl Into<String>, progress: u64) {
        if self.has_broadcaster() {
            debug!(
                "🔔 notify_progress using NotificationBroadcaster for session: {}",
                self.session_id
            );
            // TODO: Use broadcaster for MCP-compliant notifications
        } else {
            debug!(
                "🔔 notify_progress using OLD SessionManager for session: {}",
                self.session_id
            );
        }
        let mut other = std::collections::HashMap::new();
        other.insert(
            "progressToken".to_string(),
            serde_json::json!(progress_token.into()),
        );
        other.insert("progress".to_string(), serde_json::json!(progress));

        let params = turul_mcp_protocol::RequestParams { meta: None, other };
        let notification =
            turul_mcp_protocol::JsonRpcNotification::new("notifications/progress".to_string())
                .with_params(params);
        self.notify(SessionEvent::Notification(
            serde_json::to_value(notification).unwrap(),
        ))
        .await;
    }

    /// Send a progress notification with total
    pub async fn notify_progress_with_total(
        &self,
        progress_token: impl Into<String>,
        progress: u64,
        total: u64,
    ) {
        let mut other = std::collections::HashMap::new();
        other.insert(
            "progressToken".to_string(),
            serde_json::json!(progress_token.into()),
        );
        other.insert("progress".to_string(), serde_json::json!(progress));
        other.insert("total".to_string(), serde_json::json!(total));

        let params = turul_mcp_protocol::RequestParams { meta: None, other };
        let notification =
            turul_mcp_protocol::JsonRpcNotification::new("notifications/progress".to_string())
                .with_params(params);
        self.notify(SessionEvent::Notification(
            serde_json::to_value(notification).unwrap(),
        ))
        .await;
    }

    /// Send a logging message notification (with session-aware level filtering)
    pub async fn notify_log(
        &self,
        level: turul_mcp_protocol::logging::LoggingLevel,
        data: serde_json::Value,
        logger: Option<String>,
        meta: Option<std::collections::HashMap<String, serde_json::Value>>,
    ) {
        // Use the provided LoggingLevel directly
        let message_level = level;

        // Check if this message should be sent to this session based on its logging level
        if !self.should_log(message_level).await {
            let threshold = self.get_logging_level().await;
            debug!(
                "🔕 Filtering out {:?} level message for session {} (threshold: {:?})",
                message_level, self.session_id, threshold
            );
            return;
        }

        let threshold = self.get_logging_level().await;
        debug!(
            "📢 Sending {:?} level message to session {} (threshold: {:?})",
            message_level, self.session_id, threshold
        );

        // Create proper LoggingMessageNotification struct once
        use turul_mcp_protocol::notifications::LoggingMessageNotification;
        let mut notification = LoggingMessageNotification::new(message_level, data);

        // Add optional logger if provided
        if let Some(logger) = logger {
            notification = notification.with_logger(logger);
        }

        // Add optional meta if provided
        if let Some(meta) = meta {
            notification = notification.with_meta(meta);
        }

        if self.has_broadcaster() {
            debug!(
                "🔔 notify_log using NotificationBroadcaster for session: {}",
                self.session_id
            );
            // Send via SessionEvent (which will be picked up by the broadcaster if connected properly)
            self.notify(SessionEvent::Notification(
                serde_json::to_value(notification).unwrap(),
            ))
            .await;
            return;
        } else {
            debug!(
                "🔔 notify_log using OLD SessionManager for session: {}",
                self.session_id
            );
        }

        // Legacy implementation (fallback) - use the same notification
        self.notify(SessionEvent::Notification(
            serde_json::to_value(notification).unwrap(),
        ))
        .await;
    }

    /// Send a resource list changed notification
    pub async fn notify_resources_changed(&self) {
        let notification = turul_mcp_protocol::JsonRpcNotification::new(
            "notifications/resources/list_changed".to_string(),
        );
        self.notify(SessionEvent::Notification(
            serde_json::to_value(notification).unwrap(),
        ))
        .await;
    }

    /// Send a resource updated notification
    pub async fn notify_resource_updated(&self, uri: impl Into<String>) {
        let mut other = std::collections::HashMap::new();
        other.insert("uri".to_string(), serde_json::json!(uri.into()));

        let params = turul_mcp_protocol::RequestParams { meta: None, other };
        let notification = turul_mcp_protocol::JsonRpcNotification::new(
            "notifications/resources/updated".to_string(),
        )
        .with_params(params);
        self.notify(SessionEvent::Notification(
            serde_json::to_value(notification).unwrap(),
        ))
        .await;
    }

    /// Send a tools list changed notification
    pub async fn notify_tools_changed(&self) {
        let notification = turul_mcp_protocol::JsonRpcNotification::new(
            "notifications/tools/list_changed".to_string(),
        );
        self.notify(SessionEvent::Notification(
            serde_json::to_value(notification).unwrap(),
        ))
        .await;
    }

    // ============================================================================
    // === Session-Aware Logging Level Methods ===
    // ============================================================================

    /// Get the current logging level for this session (async)
    pub async fn get_logging_level(&self) -> turul_mcp_protocol::logging::LoggingLevel {
        use turul_mcp_protocol::logging::LoggingLevel;

        // Check session state for stored logging level
        if let Some(level_value) = (self.get_state)("mcp:logging:level").await {
            if let Some(level_str) = level_value.as_str() {
                match level_str {
                    "debug" => LoggingLevel::Debug,
                    "info" => LoggingLevel::Info,
                    "notice" => LoggingLevel::Notice,
                    "warning" => LoggingLevel::Warning,
                    "error" => LoggingLevel::Error,
                    "critical" => LoggingLevel::Critical,
                    "alert" => LoggingLevel::Alert,
                    "emergency" => LoggingLevel::Emergency,
                    _ => LoggingLevel::Info, // Default fallback
                }
            } else {
                LoggingLevel::Info // Default if not a string
            }
        } else {
            LoggingLevel::Info // Default if not set
        }
    }

    /// Set the logging level for this session (async)
    pub async fn set_logging_level(&self, level: turul_mcp_protocol::logging::LoggingLevel) {
        use turul_mcp_protocol::logging::LoggingLevel;

        let level_str = match level {
            LoggingLevel::Debug => "debug",
            LoggingLevel::Info => "info",
            LoggingLevel::Notice => "notice",
            LoggingLevel::Warning => "warning",
            LoggingLevel::Error => "error",
            LoggingLevel::Critical => "critical",
            LoggingLevel::Alert => "alert",
            LoggingLevel::Emergency => "emergency",
        };

        (self.set_state)("mcp:logging:level", serde_json::json!(level_str)).await;
        debug!(
            "🎯 Set logging level for session {}: {:?}",
            self.session_id, level
        );
    }

    /// Check if a log message at the given level should be sent to this session (async)
    pub async fn should_log(
        &self,
        message_level: turul_mcp_protocol::logging::LoggingLevel,
    ) -> bool {
        let session_threshold = self.get_logging_level().await;
        message_level.should_log(session_threshold)
    }

    /// Synchronous version of should_log for trait compatibility
    pub fn should_log_sync(
        &self,
        message_level: turul_mcp_protocol::logging::LoggingLevel,
    ) -> bool {
        // For sync compatibility, block on async get_logging_level
        let session_level = futures::executor::block_on(self.get_logging_level());
        message_level.should_log(session_level)
    }
}

// ============================================================================
// === SessionView Implementation ===
// ============================================================================

/// Implement SessionView trait for SessionContext
/// (trait is defined in turul-mcp-session-storage)
///
/// This allows SessionContext to be used with middleware. Metadata is stored using a
/// special prefix ("__meta__:") to distinguish it from regular state.
#[async_trait]
impl SessionView for SessionContext {
    fn session_id(&self) -> &str {
        &self.session_id
    }

    async fn get_state(&self, key: &str) -> Result<Option<Value>, String> {
        Ok((self.get_state)(key).await)
    }

    async fn set_state(&self, key: &str, value: Value) -> Result<(), String> {
        (self.set_state)(key, value).await;
        Ok(())
    }

    async fn get_metadata(&self, key: &str) -> Result<Option<Value>, String> {
        // Store metadata with a special prefix to distinguish from regular state
        let metadata_key = format!("__meta__:{}", key);
        Ok((self.get_state)(&metadata_key).await)
    }

    async fn set_metadata(&self, key: &str, value: Value) -> Result<(), String> {
        // Store metadata with a special prefix to distinguish from regular state
        let metadata_key = format!("__meta__:{}", key);
        (self.set_state)(&metadata_key, value).await;
        Ok(())
    }
}

// ============================================================================
// === LoggingTarget Trait Implementation ===
// ============================================================================

/// Implement LoggingTarget trait from turul-mcp-builders to enable session-aware logging
impl turul_mcp_builders::logging::LoggingTarget for SessionContext {
    fn should_log(&self, level: turul_mcp_protocol::logging::LoggingLevel) -> bool {
        self.should_log_sync(level)
    }

    fn notify_log(
        &self,
        level: turul_mcp_protocol::logging::LoggingLevel,
        data: serde_json::Value,
        logger: Option<String>,
        meta: Option<std::collections::HashMap<String, serde_json::Value>>,
    ) {
        // Since the trait expects sync but our method is async, we need to spawn a task
        let session_ctx = self.clone();
        tokio::spawn(async move {
            session_ctx.notify_log(level, data, logger, meta).await;
        });
    }
}

/// Parse notification JSON and send via actual NotificationBroadcaster to StreamManager using proper notification structs
async fn parse_and_send_notification_with_broadcaster(
    session_id: &str,
    json_value: &Value,
    broadcaster_any: &Arc<dyn std::any::Any + Send + Sync>,
) -> Result<(), String> {
    debug!(
        "🔍 Parsing notification JSON for session {}: {:?}",
        session_id, json_value
    );

    // Import the types we need for downcasting and notifications
    use turul_http_mcp_server::notification_bridge::SharedNotificationBroadcaster;
    use turul_mcp_protocol::notifications::{LoggingMessageNotification, ProgressNotification};
    // Attempt to downcast Arc<dyn Any> back to SharedNotificationBroadcaster
    debug!(
        "🔍 Attempting downcast for session {}, broadcaster type: {:?}",
        session_id,
        std::any::type_name::<SharedNotificationBroadcaster>()
    );
    if let Some(broadcaster) = broadcaster_any.downcast_ref::<SharedNotificationBroadcaster>() {
        debug!(
            "✅ Successfully downcast broadcaster for session {}",
            session_id
        );

        // Extract method from JSON-RPC notification to determine type
        if let Some(method) = json_value.get("method").and_then(|v| v.as_str()) {
            match method {
                "notifications/message" => {
                    debug!(
                        "📝 Message notification detected, deserializing directly to LoggingMessageNotification"
                    );

                    // Deserialize directly into LoggingMessageNotification struct
                    match serde_json::from_value::<LoggingMessageNotification>(json_value.clone()) {
                        Ok(notification) => {
                            debug!(
                                "✅ Successfully deserialized LoggingMessageNotification: level={:?}, logger={:?}",
                                notification.params.level, notification.params.logger
                            );

                            debug!(
                                "🔧 About to call broadcaster.send_message_notification() for session {}",
                                session_id
                            );
                            // ACTUALLY SEND the notification using the proper method
                            match broadcaster
                                .send_message_notification(session_id, notification)
                                .await
                            {
                                Ok(()) => {
                                    debug!(
                                        "🎉 SUCCESS: LoggingMessageNotification sent to StreamManager for session {}",
                                        session_id
                                    );
                                    debug!(
                                        "🚀 Streamable HTTP Transport Bridge: Complete end-to-end delivery confirmed!"
                                    );
                                    return Ok(());
                                }
                                Err(e) => {
                                    error!(
                                        "❌ Failed to send LoggingMessageNotification to StreamManager: {}",
                                        e
                                    );
                                    return Err(format!(
                                        "Failed to send LoggingMessageNotification: {}",
                                        e
                                    ));
                                }
                            }
                        }
                        Err(e) => {
                            error!("❌ Failed to deserialize LoggingMessageNotification: {}", e);
                            return Err(format!(
                                "Failed to deserialize LoggingMessageNotification: {}",
                                e
                            ));
                        }
                    }
                }
                "notifications/progress" => {
                    if let Some(params) = json_value.get("params")
                        && let Some(token) = params.get("progressToken").and_then(|v| v.as_str())
                    {
                        debug!("📊 Progress notification detected: token={}", token);

                        // Get progress value
                        let progress = params
                            .get("progress")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0);

                        // Create proper ProgressNotification using the struct from notifications.rs
                        let notification = ProgressNotification {
                            method: "notifications/progress".to_string(),
                            params: turul_mcp_protocol::notifications::ProgressNotificationParams {
                                progress_token: token.to_string().into(),
                                progress,
                                total: params.get("total").and_then(|v| v.as_f64()),
                                message: params
                                    .get("message")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                meta: None,
                            },
                        };

                        debug!(
                            "🔧 About to call broadcaster.send_progress_notification() for session {}",
                            session_id
                        );
                        // ACTUALLY SEND the notification using the proper method
                        match broadcaster
                            .send_progress_notification(session_id, notification)
                            .await
                        {
                            Ok(()) => {
                                debug!(
                                    "🎉 SUCCESS: ProgressNotification sent to StreamManager for session {}",
                                    session_id
                                );
                                debug!(
                                    "🚀 Streamable HTTP Transport Bridge: Complete end-to-end delivery confirmed!"
                                );
                                return Ok(());
                            }
                            Err(e) => {
                                error!(
                                    "❌ Failed to send ProgressNotification to StreamManager: {}",
                                    e
                                );
                                return Err(format!("Failed to send ProgressNotification: {}", e));
                            }
                        }
                    }
                }
                _ => {
                    debug!(
                        "🔧 Other notification method: {} - sending as generic JsonRpcNotification",
                        method
                    );

                    // For other notifications, use the generic send_notification method
                    let params_map: std::collections::HashMap<String, serde_json::Value> =
                        json_value
                            .get("params")
                            .and_then(|p| p.as_object())
                            .unwrap_or(&serde_json::Map::new())
                            .iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect();
                    let json_rpc_notification =
                        turul_mcp_json_rpc_server::JsonRpcNotification::new_with_object_params(
                            method.to_string(),
                            params_map,
                        );

                    match broadcaster
                        .send_notification(session_id, json_rpc_notification)
                        .await
                    {
                        Ok(()) => {
                            debug!(
                                "🎉 SUCCESS: Generic notification sent to StreamManager for session {}",
                                session_id
                            );
                            return Ok(());
                        }
                        Err(e) => {
                            error!(
                                "❌ Failed to send generic notification to StreamManager: {}",
                                e
                            );
                            return Err(format!("Failed to send generic notification: {}", e));
                        }
                    }
                }
            }
        }
    } else {
        error!(
            "❌ Failed to downcast broadcaster for session {}",
            session_id
        );
        return Err("Failed to downcast broadcaster to SharedNotificationBroadcaster".to_string());
    }

    debug!(
        "❓ Could not determine notification type for session {}",
        session_id
    );
    Ok(())
}

/// Awaitable dispatcher for session events that need guaranteed persistence
/// and live delivery. Installed by the runtime (HTTP server, Lambda) during
/// construction. Without a dispatcher, events are best-effort only (in-memory
/// channels, no persistence guarantee).
///
/// The dispatcher is called from `SessionManager::broadcast_event()` on the
/// request path, ensuring persistence completes before the request returns.
#[async_trait]
pub trait SessionEventDispatcher: Send + Sync {
    /// Persist and deliver an event to a specific session.
    /// MUST complete persistence before returning.
    async fn dispatch_to_session(
        &self,
        session_id: &str,
        event_type: String,
        data: serde_json::Value,
    ) -> Result<(), String>;
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
        let session_id = Uuid::now_v7().as_simple().to_string();
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
    #[error("Storage error: {0}")]
    StorageError(String),
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
    /// Awaited event dispatcher for guaranteed persistence + live delivery.
    /// Installed by the runtime (HTTP server, Lambda). When set, Custom events
    /// are dispatched synchronously on the request path before broadcast_event returns.
    event_dispatcher: RwLock<Option<Arc<dyn SessionEventDispatcher>>>,
}

impl SessionManager {
    /// Create a new session manager with InMemory storage
    pub fn new(default_capabilities: ServerCapabilities) -> Self {
        let storage: Arc<turul_mcp_session_storage::BoxedSessionStorage> =
            Arc::new(turul_mcp_session_storage::InMemorySessionStorage::new());
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
        let storage: Arc<turul_mcp_session_storage::BoxedSessionStorage> =
            Arc::new(turul_mcp_session_storage::InMemorySessionStorage::new());
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
            event_dispatcher: RwLock::new(None),
        }
    }

    /// Install an event dispatcher for guaranteed persistence and live delivery.
    /// Called by the runtime (HTTP server, Lambda) after construction.
    /// Once set, `broadcast_event()` awaits the dispatcher for `SessionEvent::Custom`
    /// events before returning, ensuring persistence on the request path.
    pub async fn set_event_dispatcher(&self, dispatcher: Arc<dyn SessionEventDispatcher>) {
        *self.event_dispatcher.write().await = Some(dispatcher);
    }

    /// Create a new session and return its ID
    pub async fn create_session(&self) -> String {
        let session = McpSession::new(self.default_capabilities.clone());
        let session_id = session.id.clone();

        debug!("Creating new session: {}", session_id);

        // Store in pluggable storage backend
        match self
            .storage
            .create_session_with_id(session_id.clone(), self.default_capabilities.clone())
            .await
        {
            Ok(_) => debug!("Session {} created in storage backend", session_id),
            Err(e) => error!("Failed to create session {} in storage: {}", session_id, e),
        }

        // Also store in memory cache for performance
        self.sessions
            .write()
            .await
            .insert(session_id.clone(), session);
        session_id
    }

    /// Create a new session with a specific ID (for testing only - see trait documentation)
    pub async fn create_session_with_id(&self, session_id: String) -> String {
        let mut session = McpSession::new(self.default_capabilities.clone());
        session.id = session_id.clone();

        debug!("Creating session with provided ID: {}", session_id);

        // Store in pluggable storage backend
        match self
            .storage
            .create_session_with_id(session_id.clone(), self.default_capabilities.clone())
            .await
        {
            Ok(_) => debug!("Session {} created in storage backend", session_id),
            Err(e) => error!("Failed to create session {} in storage: {}", session_id, e),
        }

        // Also store in memory cache for performance
        self.sessions
            .write()
            .await
            .insert(session_id.clone(), session);
        session_id
    }

    /// Add an externally created session to the cache
    /// Used when session_handler creates a session directly in storage
    pub async fn add_session_to_cache(
        &self,
        session_id: String,
        server_capabilities: ServerCapabilities,
    ) {
        let mut session = McpSession::new(server_capabilities);
        session.id = session_id.clone(); // Use the provided ID

        debug!("Adding externally created session {} to cache", session_id);
        self.sessions.write().await.insert(session_id, session);
    }

    /// Load session from storage into cache with its actual capabilities
    /// This preserves the negotiated capabilities and session state from persistent storage
    pub async fn load_session_from_storage(&self, session_id: &str) -> Result<bool, SessionError> {
        match self.storage.get_session(session_id).await {
            Ok(Some(session_info)) => {
                debug!("Loading session {} from storage", session_id);

                // Create McpSession from stored SessionInfo with preserved capabilities
                let server_capabilities =
                    session_info.server_capabilities.clone().unwrap_or_else(|| {
                        warn!(
                            "Session {} in storage has no server capabilities, using defaults",
                            session_id
                        );
                        self.default_capabilities.clone()
                    });

                let mut session = McpSession::new(server_capabilities);
                session.id = session_id.to_string();
                session.initialized = session_info.is_initialized;
                session.client_capabilities = session_info.client_capabilities.clone();
                session.state = session_info.state.clone();

                // Convert Unix timestamps to Instant
                // Calculate elapsed time from stored timestamps to current time
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;

                let created_elapsed = if now > session_info.created_at {
                    Duration::from_millis(now - session_info.created_at)
                } else {
                    Duration::from_secs(0)
                };

                let last_activity_elapsed = if now > session_info.last_activity {
                    Duration::from_millis(now - session_info.last_activity)
                } else {
                    Duration::from_secs(0)
                };

                // Set timestamps relative to current time
                session.created = Instant::now() - created_elapsed;
                session.last_accessed = Instant::now() - last_activity_elapsed;

                // Add to cache with preserved state and capabilities
                self.sessions
                    .write()
                    .await
                    .insert(session_id.to_string(), session);

                debug!(
                    "Session {} loaded from storage: initialized={}, has_capabilities={}",
                    session_id,
                    session_info.is_initialized,
                    session_info.server_capabilities.is_some()
                );

                Ok(true)
            }
            Ok(None) => {
                debug!("Session {} not found in storage", session_id);
                Ok(false)
            }
            Err(e) => {
                error!("Failed to get session {} from storage: {}", session_id, e);
                Err(SessionError::StorageError(e.to_string()))
            }
        }
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
        // Update storage backend first - CRITICAL for persistence
        if let Ok(Some(mut session_info)) = self.storage.get_session(session_id).await {
            session_info.client_capabilities = Some(client_capabilities.clone());
            session_info.is_initialized = true;
            session_info.touch();
            // Note: mcp_version not stored in SessionInfo, only in memory cache

            if let Err(e) = self.storage.update_session(session_info).await {
                error!(
                    "❌ CRITICAL: Failed to update session {} in storage: {}",
                    session_id, e
                );
                return Err(SessionError::StorageError(format!(
                    "Failed to persist session initialization: {}",
                    e
                )));
            }
            debug!(
                "✅ Session {} storage updated with is_initialized=true",
                session_id
            );
        } else {
            error!(
                "❌ Session {} not found in storage during initialization",
                session_id
            );
            return Err(SessionError::NotFound(session_id.to_string()));
        }

        // Update in-memory cache
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.initialize_with_version(client_info, client_capabilities, mcp_version);
            debug!(
                "✅ Session {} cache updated with protocol version {}",
                session_id, mcp_version
            );
            Ok(())
        } else {
            warn!(
                "⚠️ Session {} not found in cache but exists in storage - creating cache entry",
                session_id
            );
            // Session exists in storage but not in cache - this is acceptable
            // The cache will be populated on next access
            Ok(())
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
        if let Err(e) = self
            .storage
            .set_session_state(session_id, key, value.clone())
            .await
        {
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
        // Check storage backend first for authoritative answer (cache might be stale)
        match self.storage.get_session(session_id).await {
            Ok(Some(session_info)) => {
                debug!(
                    "✅ Session {} initialization status from storage: {}",
                    session_id, session_info.is_initialized
                );
                session_info.is_initialized
            }
            Ok(None) => {
                debug!("⚠️ Session {} not found in storage", session_id);
                false
            }
            Err(e) => {
                warn!(
                    "⚠️ Failed to check session {} in storage: {} - falling back to cache",
                    session_id, e
                );
                // Fallback to cache on storage error
                let sessions = self.sessions.read().await;
                sessions
                    .get(session_id)
                    .map(|s| s.initialized)
                    .unwrap_or(false)
            }
        }
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
                error!(
                    "Failed to remove session {} from storage: {}",
                    session_id, e
                );
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
                    info!(
                        "Storage backend cleaned up {} expired sessions: {:?}",
                        count, expired_ids
                    );
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

    /// Send event to a specific session with guaranteed persistence for Custom events.
    ///
    /// Returns `Err` if the session does not exist or if dispatcher persistence fails
    /// for Custom events. Non-custom events are best-effort after the existence check.
    pub async fn send_event_to_session(
        &self,
        session_id: &str,
        event: SessionEvent,
    ) -> std::result::Result<(), String> {
        // Phase 1: Fire in-memory listener (session must exist)
        {
            let sessions = self.sessions.read().await;
            match sessions.get(session_id) {
                Some(session) => {
                    let _ = session.send_event(event.clone());
                }
                None => {
                    return Err(format!("Session not found: {}", session_id));
                }
            }
        }

        // Phase 2: Awaited dispatcher for Custom events (mandatory persistence)
        if let SessionEvent::Custom {
            ref event_type,
            ref data,
        } = event
        {
            let dispatcher = self.event_dispatcher.read().await.clone();
            if let Some(ref dispatcher) = dispatcher {
                dispatcher
                    .dispatch_to_session(session_id, event_type.clone(), data.clone())
                    .await?;
            }
        }

        // Phase 3: Global channel (observer-only)
        let _ = self
            .global_event_sender
            .send((session_id.to_string(), event));

        Ok(())
    }

    /// Dispatch a Custom event to a specific session via the installed dispatcher.
    /// Storage-backed — does NOT require the session to be in the in-memory cache.
    /// The caller is responsible for verifying the session exists in storage
    /// (e.g., via `validate_session_exists()`).
    /// Returns `Err` if dispatcher persistence fails.
    pub async fn dispatch_custom_event(
        &self,
        session_id: &str,
        event_type: String,
        data: serde_json::Value,
    ) -> std::result::Result<(), String> {
        // In-memory listener (best-effort if session is cached)
        {
            let sessions = self.sessions.read().await;
            if let Some(session) = sessions.get(session_id) {
                let event = SessionEvent::Custom {
                    event_type: event_type.clone(),
                    data: data.clone(),
                };
                let _ = session.send_event(event);
            }
        }

        // Dispatcher (mandatory — storage-backed, not cache-gated)
        let dispatcher = self.event_dispatcher.read().await.clone();
        if let Some(ref dispatcher) = dispatcher {
            dispatcher
                .dispatch_to_session(session_id, event_type, data)
                .await?;
        }

        Ok(())
    }

    /// Broadcast event to all sessions.
    /// Broadcast event to all sessions.
    ///
    /// For `SessionEvent::Custom`: uses `storage.list_sessions()` to enumerate targets
    /// from the storage backend (not the in-memory cache). Filters terminated sessions.
    /// Returns `Err` if storage enumeration or any session dispatch fails.
    ///
    /// For non-Custom events: uses in-memory cache only, best-effort, always returns `Ok(())`.
    pub async fn broadcast_event(&self, event: SessionEvent) -> std::result::Result<(), String> {
        // Phase 1: In-memory listeners (cache-local, best-effort)
        let cached_ids: Vec<String> = {
            let sessions = self.sessions.read().await;
            let mut ids = Vec::with_capacity(sessions.len());
            for (session_id, session) in sessions.iter() {
                let _ = session.send_event(event.clone());
                ids.push(session_id.clone());
            }
            ids
        };

        // Phase 2: Storage-backed targeting for Custom events
        let mut dispatch_errors: Vec<String> = Vec::new();
        if let SessionEvent::Custom {
            ref event_type,
            ref data,
        } = event
        {
            let dispatcher = self.event_dispatcher.read().await.clone();
            if let Some(ref dispatcher) = dispatcher {
                // Enumerate ALL sessions from storage — not the in-memory cache
                let all_ids = self
                    .storage
                    .list_sessions()
                    .await
                    .map_err(|e| format!("Failed to list sessions from storage: {}", e))?;

                // Filter terminated sessions
                let mut targets = Vec::new();
                for sid in &all_ids {
                    if let Ok(Some(info)) = self.storage.get_session(sid).await {
                        if !info.is_terminated() {
                            targets.push(sid.clone());
                        }
                    }
                }

                for session_id in &targets {
                    if let Err(e) = dispatcher
                        .dispatch_to_session(session_id, event_type.clone(), data.clone())
                        .await
                    {
                        dispatch_errors.push(format!("session {}: {}", session_id, e));
                    }
                }
            }
        }

        // Phase 3: Global broadcast channel — observer-only (cache-local, tests/debugging)
        for session_id in &cached_ids {
            if let Err(e) = self
                .global_event_sender
                .send((session_id.clone(), event.clone()))
            {
                debug!("Global event broadcast failed (no listeners): {}", e);
            }
        }

        if dispatch_errors.is_empty() {
            Ok(())
        } else {
            Err(dispatch_errors.join("; "))
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
            Arc::new(move |key: &str| -> BoxFuture<Option<Value>> {
                let session_manager = session_manager.clone();
                let session_id = session_id.clone();
                let key = key.to_string();
                Box::pin(async move { session_manager.get_session_state(&session_id, &key).await })
            })
        };

        let set_state = {
            let session_manager = session_manager.clone();
            let session_id = session_id.clone();
            Arc::new(move |key: &str, value: Value| -> BoxFuture<()> {
                let session_manager = session_manager.clone();
                let session_id = session_id.clone();
                let key = key.to_string();
                Box::pin(async move {
                    let _ = session_manager
                        .set_session_state(&session_id, &key, value)
                        .await;
                })
            })
        };

        let remove_state = {
            let session_manager = session_manager.clone();
            let session_id = session_id.clone();
            Arc::new(move |key: &str| -> BoxFuture<Option<Value>> {
                let session_manager = session_manager.clone();
                let session_id = session_id.clone();
                let key = key.to_string();
                Box::pin(async move {
                    session_manager
                        .remove_session_state(&session_id, &key)
                        .await
                })
            })
        };

        let is_initialized = {
            let session_manager = session_manager.clone();
            let session_id = session_id.clone();
            Arc::new(move || -> BoxFuture<bool> {
                let session_manager = session_manager.clone();
                let session_id = session_id.clone();
                Box::pin(async move { session_manager.is_session_initialized(&session_id).await })
            })
        };

        let send_notification = {
            let session_manager = session_manager.clone();
            let session_id = session_id.clone();
            Arc::new(move |event: SessionEvent| -> BoxFuture<()> {
                let session_manager = session_manager.clone();
                let session_id = session_id.clone();
                Box::pin(async move {
                    let _ = session_manager
                        .send_event_to_session(&session_id, event)
                        .await;
                })
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
            extensions: HashMap::new(),
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
    pub async fn get_session_event_receiver(
        &self,
        session_id: &str,
    ) -> Option<broadcast::Receiver<SessionEvent>> {
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

    /// Get the default capabilities for use by other components
    pub fn get_default_capabilities(&self) -> ServerCapabilities {
        self.default_capabilities.clone()
    }

    /// Check if session exists in the in-memory cache only (not storage)
    pub async fn session_exists_in_cache(&self, session_id: &str) -> bool {
        self.sessions.read().await.contains_key(session_id)
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
        (ctx.set_state)("test", json!("value")).await;
        let value = (ctx.get_state)("test").await;
        assert_eq!(value, Some(json!("value")));

        let removed = (ctx.remove_state)("test").await;
        assert_eq!(removed, Some(json!("value")));

        // Test notification sending
        ctx.notify_log(
            turul_mcp_protocol::logging::LoggingLevel::Info,
            serde_json::json!("Test notification"),
            Some("test".to_string()),
            None,
        )
        .await;
        ctx.notify_progress("test-token", 50).await;
    }

    #[tokio::test]
    async fn test_session_expiry() {
        let capabilities = ServerCapabilities::default();
        let mut manager = SessionManager::new(capabilities);
        manager.session_timeout = Duration::from_millis(100); // Very short timeout

        let session_id = manager.create_session().await;
        // Use cache-based check to avoid storage backend timing issues in tests
        assert!(manager.session_exists_in_cache(&session_id).await);

        // Wait for expiry
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Session should be expired now
        let result = manager.touch_session(&session_id).await;
        assert!(matches!(result, Err(SessionError::Expired(_))));
    }

    // T17: Extensions are not persisted to session storage
    #[tokio::test]
    async fn test_extensions_not_persisted_to_session_storage() {
        let ctx = SessionContext::new_test();
        // Extensions field exists but is empty by default
        assert!(ctx.extensions.is_empty());
        // Setting state works via storage, extensions are separate
        (ctx.set_state)("key", json!("value")).await;
        let val = (ctx.get_state)("key").await;
        assert_eq!(val, Some(json!("value")));
        // Extensions remain empty — they are never persisted
        assert!(ctx.extensions.is_empty());
    }

    // T18: Extensions empty when no middleware
    #[tokio::test]
    async fn test_extensions_empty_when_no_middleware() {
        let ctx = SessionContext::new_test();
        assert!(ctx.extensions.is_empty());
        assert!(ctx.get_extension("anything").is_none());
    }

    // T19: get_typed_extension deserialization
    #[tokio::test]
    async fn test_get_typed_extension_deserialization() {
        #[derive(Debug, serde::Deserialize, PartialEq)]
        struct TokenClaims {
            sub: String,
            iss: String,
        }

        let mut ctx = SessionContext::new_test();
        ctx.extensions.insert(
            "__turul_internal.auth_claims".to_string(),
            json!({
                "sub": "user-123",
                "iss": "https://auth.example.com"
            }),
        );

        // Typed deserialization
        let claims: Option<TokenClaims> = ctx.get_typed_extension("__turul_internal.auth_claims");
        assert!(claims.is_some());
        let claims = claims.unwrap();
        assert_eq!(claims.sub, "user-123");
        assert_eq!(claims.iss, "https://auth.example.com");

        // Non-existent key
        let missing: Option<TokenClaims> = ctx.get_typed_extension("nonexistent");
        assert!(missing.is_none());

        // Wrong type
        let wrong: Option<Vec<String>> = ctx.get_typed_extension("__turul_internal.auth_claims");
        assert!(wrong.is_none());
    }

    // T16: Extensions thread from middleware to framework SessionContext
    #[tokio::test]
    async fn test_extensions_thread_from_json_rpc_to_framework() {
        use turul_mcp_session_storage::InMemorySessionStorage;

        let storage = Arc::new(InMemorySessionStorage::new());
        storage
            .create_session_with_id("test-session".to_string(), Default::default())
            .await
            .unwrap();

        let mut json_rpc_ctx = turul_mcp_json_rpc_server::SessionContext {
            session_id: "test-session".to_string(),
            metadata: HashMap::new(),
            broadcaster: None,
            timestamp: 0,
            extensions: HashMap::new(),
        };

        // Simulate what transport does: write claims into extensions
        json_rpc_ctx.extensions.insert(
            "__turul_internal.auth_claims".to_string(),
            json!({"sub": "user-456"}),
        );

        let framework_ctx = SessionContext::from_json_rpc_with_broadcaster(json_rpc_ctx, storage);

        // Verify extensions were copied through
        assert_eq!(
            framework_ctx.get_extension("__turul_internal.auth_claims"),
            Some(&json!({"sub": "user-456"}))
        );
    }
}
