//! MCP Handler System
//!
//! This module provides a standardized handler system for MCP endpoints.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};
use tracing::debug;

use crate::resource::{McpResource, resource_to_descriptor};

use crate::{McpResult, SessionContext};
use turul_mcp_protocol::McpError;

//pub mod response;
//pub use response::*;

/// Generic MCP handler trait
#[async_trait]
pub trait McpHandler: Send + Sync {
    /// Handle an MCP request
    async fn handle(&self, params: Option<Value>) -> McpResult<Value>;

    /// Handle an MCP request with session context (default implementation calls handle)
    async fn handle_with_session(
        &self,
        params: Option<Value>,
        _session: Option<SessionContext>,
    ) -> McpResult<Value> {
        self.handle(params).await
    }

    /// Get the methods this handler supports
    fn supported_methods(&self) -> Vec<String>;
}

/// Ping handler for ping endpoint
pub struct PingHandler;

#[async_trait]
impl McpHandler for PingHandler {
    async fn handle(&self, _params: Option<Value>) -> McpResult<Value> {
        Ok(serde_json::json!({}))
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["ping".to_string()]
    }
}

/// Completion handler for completion/complete endpoint
pub struct CompletionHandler;

#[async_trait]
impl McpHandler for CompletionHandler {
    async fn handle(&self, _params: Option<Value>) -> McpResult<Value> {
        use turul_mcp_protocol::completion::{CompleteResult, CompletionResult};

        // Default implementation - can be overridden by users
        let values = vec!["example1".to_string(), "example2".to_string()];

        let completion_result = CompletionResult::new(values);
        let response = CompleteResult::new(completion_result);
        serde_json::to_value(response).map_err(McpError::from)
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["completion/complete".to_string()]
    }
}

/// Prompts handler for prompts/list and prompts/get endpoints
pub struct PromptsHandler {
    prompts: HashMap<String, Arc<dyn McpPrompt>>,
}

impl PromptsHandler {
    pub fn new() -> Self {
        Self {
            prompts: HashMap::new(),
        }
    }

    pub fn add_prompt<P: McpPrompt + 'static>(mut self, prompt: P) -> Self {
        self.prompts
            .insert(prompt.name().to_string(), Arc::new(prompt));
        self
    }

    pub fn add_prompt_arc(mut self, prompt: Arc<dyn McpPrompt>) -> Self {
        self.prompts.insert(prompt.name().to_string(), prompt);
        self
    }
}

#[async_trait]
impl McpHandler for PromptsHandler {
    async fn handle(&self, params: Option<Value>) -> McpResult<Value> {
        // Handle prompts/list with pagination support
        use turul_mcp_protocol::meta::{Cursor, PaginatedResponse};
        use turul_mcp_protocol::prompts::{ListPromptsResult, Prompt};

        // Parse cursor from params if provided
        let cursor = params
            .as_ref()
            .and_then(|p| p.get("cursor"))
            .and_then(|c| c.as_str())
            .map(Cursor::from);

        debug!("Listing prompts with cursor: {:?}", cursor);

        let prompts: Vec<Prompt> = self
            .prompts
            .values()
            .map(|p| Prompt::new(p.name()).with_description(p.description()))
            .collect();

        let base_response = ListPromptsResult::new(prompts.clone());

        // Add pagination metadata
        let has_more = false; // In a real implementation, this would depend on the actual data
        let total = Some(prompts.len() as u64);

        let paginated_response = PaginatedResponse::with_pagination(
            base_response,
            None, // next_cursor - would be calculated based on current page
            total,
            has_more,
        );

        serde_json::to_value(paginated_response).map_err(McpError::from)
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["prompts/list".to_string(), "prompts/get".to_string()]
    }
}

/// Trait for MCP prompts
#[async_trait]
pub trait McpPrompt: Send + Sync {
    /// The name of the prompt
    fn name(&self) -> &str;

    /// Description of the prompt
    fn description(&self) -> &str;

    /// Generate the prompt content
    async fn generate(
        &self,
        args: HashMap<String, Value>,
    ) -> McpResult<Vec<turul_mcp_protocol::prompts::PromptMessage>>;
}

/// Resources handler for resources/list and resources/read endpoints
pub struct ResourcesHandler {
    resources: HashMap<String, Arc<dyn McpResource>>,
}

impl ResourcesHandler {
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
        }
    }

    pub fn add_resource<R: McpResource + 'static>(mut self, resource: R) -> Self {
        self.resources
            .insert(resource.uri().to_string(), Arc::new(resource));
        self
    }

    pub fn add_resource_arc(mut self, resource: Arc<dyn McpResource>) -> Self {
        self.resources.insert(resource.uri().to_string(), resource);
        self
    }
}

#[async_trait]
impl McpHandler for ResourcesHandler {
    async fn handle(&self, params: Option<Value>) -> McpResult<Value> {
        use turul_mcp_protocol::meta::{Cursor, PaginatedResponse};
        use turul_mcp_protocol::resources::{ListResourcesResult, Resource};

        // Parse cursor from params if provided
        let cursor = params
            .as_ref()
            .and_then(|p| p.get("cursor"))
            .and_then(|c| c.as_str())
            .map(Cursor::from);

        debug!("Listing resources with cursor: {:?}", cursor);

        let resources: Vec<Resource> = self
            .resources
            .values()
            .map(|r| resource_to_descriptor(r.as_ref()))
            .collect();

        let base_response = ListResourcesResult::new(resources.clone());

        // Add pagination metadata if applicable
        let has_more = false; // In a real implementation, this would depend on the actual data
        let total = Some(resources.len() as u64);

        let paginated_response = PaginatedResponse::with_pagination(
            base_response,
            None, // next_cursor - would be calculated based on current page
            total,
            has_more,
        );

        serde_json::to_value(paginated_response).map_err(McpError::from)
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["resources/list".to_string(), "resources/read".to_string()]
    }
}

/// Logging handler for logging/setLevel endpoint
pub struct LoggingHandler;

#[async_trait]
impl McpHandler for LoggingHandler {
    async fn handle(&self, params: Option<Value>) -> McpResult<Value> {
        use turul_mcp_protocol::logging::SetLevelParams;

        if let Some(params) = params {
            let set_level_params: SetLevelParams = serde_json::from_value(params)?;

            // In a real implementation, you'd set the actual log level
            tracing::info!("Setting log level to: {:?}", set_level_params.level);

            // MCP logging/setLevel doesn't return data, just success
            serde_json::to_value(json!({})).map_err(McpError::from)
        } else {
            Err(McpError::missing_param("SetLevelParams"))
        }
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["logging/setLevel".to_string()]
    }
}

/// Roots handler for roots/list endpoint
pub struct RootsHandler {
    roots: Vec<turul_mcp_protocol::roots::Root>,
}

impl RootsHandler {
    pub fn new() -> Self {
        Self { roots: Vec::new() }
    }

    pub fn add_root(mut self, root: turul_mcp_protocol::roots::Root) -> Self {
        self.roots.push(root);
        self
    }
}

#[async_trait]
impl McpHandler for RootsHandler {
    async fn handle(&self, params: Option<Value>) -> McpResult<Value> {
        use turul_mcp_protocol::meta::{Cursor, PaginatedResponse};
        use turul_mcp_protocol::roots::ListRootsResult;

        // Parse cursor from params if provided
        let cursor = params
            .as_ref()
            .and_then(|p| p.get("cursor"))
            .and_then(|c| c.as_str())
            .map(Cursor::from);

        debug!("Listing roots with cursor: {:?}", cursor);

        let base_response = ListRootsResult::new(self.roots.clone());

        // Add pagination metadata
        let has_more = false; // In a real implementation, this would depend on the actual data
        let total = Some(self.roots.len() as u64);

        let paginated_response = PaginatedResponse::with_pagination(
            base_response,
            None, // next_cursor - would be calculated based on current page
            total,
            has_more,
        );

        serde_json::to_value(paginated_response).map_err(McpError::from)
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["roots/list".to_string()]
    }
}

/// Sampling handler for sampling/createMessage endpoint
pub struct SamplingHandler;

#[async_trait]
impl McpHandler for SamplingHandler {
    async fn handle(&self, params: Option<Value>) -> McpResult<Value> {
        use turul_mcp_protocol::meta::{ProgressResponse, ProgressToken};
        use turul_mcp_protocol::sampling::{CreateMessageResult, SamplingMessage};

        // Parse progress token if provided
        let progress_token = params
            .as_ref()
            .and_then(|p| p.get("progressToken"))
            .and_then(|t| t.as_str())
            .map(ProgressToken::from);

        // Default implementation - return a simple message
        let message = SamplingMessage {
            role: turul_mcp_protocol::sampling::Role::Assistant,
            content: turul_mcp_protocol::prompts::ContentBlock::Text {
                text: "This is a sample message generated by the MCP server".to_string(),
            },
        };

        let base_response = CreateMessageResult {
            message,
            model: "mock-model-v1".to_string(),
            stop_reason: Some("stop".to_string()),
            meta: None,
        };

        // Add progress metadata for message generation operations
        // In a real implementation, progress would track token generation steps
        let progress_response = ProgressResponse::with_progress(
            base_response,
            progress_token.or_else(|| Some(ProgressToken::new("sampling-default"))),
            1.0, // Complete since we're returning immediately
            Some(1),
            Some(1),
        );

        serde_json::to_value(progress_response).map_err(McpError::from)
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["sampling/createMessage".to_string()]
    }
}

/// Templates handler for templates/list and templates/get endpoints
pub struct TemplatesHandler {
    templates: HashMap<String, Arc<dyn McpTemplate>>,
}

impl TemplatesHandler {
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
        }
    }

    pub fn add_template<T: McpTemplate + 'static>(mut self, template: T) -> Self {
        self.templates
            .insert(template.name().to_string(), Arc::new(template));
        self
    }
}

#[async_trait]
impl McpHandler for TemplatesHandler {
    async fn handle(&self, _params: Option<Value>) -> McpResult<Value> {
        // Templates functionality has been integrated into resources
        debug!(
            "Templates handler called - returning empty list as templates are now integrated into resources"
        );

        // Return empty templates list since functionality moved to resources
        let templates: Vec<turul_mcp_protocol::resources::ResourceTemplate> = Vec::new();
        let total = Some(templates.len() as u64);
        let base_response =
            turul_mcp_protocol::resources::ListResourceTemplatesResult::new(templates);

        // Add pagination metadata if applicable
        let has_more = false; // In a real implementation, this would depend on the actual data

        let paginated_response = turul_mcp_protocol::meta::PaginatedResponse::with_pagination(
            base_response,
            None, // next_cursor - would be calculated based on current page
            total,
            has_more,
        );

        serde_json::to_value(paginated_response).map_err(McpError::from)
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["templates/list".to_string(), "templates/get".to_string()]
    }
}

/// Trait for MCP templates
#[async_trait]
pub trait McpTemplate: Send + Sync {
    /// The name of the template
    fn name(&self) -> &str;

    /// Description of the template
    fn description(&self) -> &str;

    /// Generate the template content
    async fn generate(&self, args: HashMap<String, Value>) -> McpResult<String>;
}

/// Resource templates handler for resources/templates/list endpoint
pub struct ResourceTemplatesHandler;

#[async_trait]
impl McpHandler for ResourceTemplatesHandler {
    async fn handle(&self, params: Option<Value>) -> McpResult<Value> {
        use turul_mcp_protocol::meta::{Cursor, PaginatedResponse};

        // Parse cursor from params if provided
        let cursor = params
            .as_ref()
            .and_then(|p| p.get("cursor"))
            .and_then(|c| c.as_str())
            .map(Cursor::from);

        debug!("Listing resource templates with cursor: {:?}", cursor);

        // In a real implementation, this would return actual resource templates
        // For demonstration purposes, we return an empty list
        tracing::info!("Resource templates list requested");

        let base_response = serde_json::json!({
            "resourceTemplates": []
        });

        // Add pagination metadata for consistency
        let has_more = false;
        let total = Some(0u64);

        let paginated_response = PaginatedResponse::with_pagination(
            base_response,
            None, // next_cursor
            total,
            has_more,
        );

        serde_json::to_value(paginated_response).map_err(McpError::from)
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["resources/templates/list".to_string()]
    }
}

/// Trait for custom elicitation UI implementations
///
/// This trait enables server implementations to provide custom user interfaces
/// for collecting structured input from users via JSON Schema-defined forms.
#[async_trait]
pub trait ElicitationProvider: Send + Sync {
    /// Present an elicitation request to the user and return their response
    async fn elicit(
        &self,
        request: &turul_mcp_protocol_2025_06_18::elicitation::ElicitCreateRequest,
    ) -> McpResult<turul_mcp_protocol_2025_06_18::elicitation::ElicitResult>;

    /// Check if this provider can handle a specific elicitation schema
    fn can_handle(
        &self,
        _request: &turul_mcp_protocol_2025_06_18::elicitation::ElicitCreateRequest,
    ) -> bool {
        // Default implementation accepts all requests
        true
    }
}

/// Default console-based elicitation provider for demonstration
pub struct MockElicitationProvider;

#[async_trait]
impl ElicitationProvider for MockElicitationProvider {
    async fn elicit(
        &self,
        request: &turul_mcp_protocol_2025_06_18::elicitation::ElicitCreateRequest,
    ) -> McpResult<turul_mcp_protocol_2025_06_18::elicitation::ElicitResult> {
        use turul_mcp_protocol::elicitation::ElicitResult;

        // Mock implementation based on message content
        let mut mock_data = std::collections::HashMap::new();
        mock_data.insert("mock_response".to_string(), serde_json::json!(true));
        mock_data.insert(
            "message".to_string(),
            serde_json::json!(&request.params.message),
        );
        mock_data.insert(
            "note".to_string(),
            serde_json::json!("This is a mock elicitation response for testing"),
        );

        // Simple mock logic based on message content
        match request.params.message.as_str() {
            msg if msg.contains("cancel") => Ok(ElicitResult::cancel()),
            msg if msg.contains("decline") => Ok(ElicitResult::decline()),
            _ => Ok(ElicitResult::accept(mock_data)),
        }
    }

    fn can_handle(
        &self,
        _request: &turul_mcp_protocol_2025_06_18::elicitation::ElicitCreateRequest,
    ) -> bool {
        true // Mock provider handles all requests
    }
}

/// Elicitation handler for elicitation/create endpoint
pub struct ElicitationHandler {
    provider: Arc<dyn ElicitationProvider>,
}

impl ElicitationHandler {
    pub fn new(provider: Arc<dyn ElicitationProvider>) -> Self {
        Self { provider }
    }

    pub fn with_mock_provider() -> Self {
        Self::new(Arc::new(MockElicitationProvider))
    }
}

#[async_trait]
impl McpHandler for ElicitationHandler {
    async fn handle(&self, params: Option<Value>) -> McpResult<Value> {
        use turul_mcp_protocol::elicitation::ElicitCreateParams;

        if let Some(params) = params {
            let request_params: ElicitCreateParams = serde_json::from_value(params)?;

            tracing::info!("Processing elicitation request: {}", request_params.message);

            // Create full request object from parameters
            use turul_mcp_protocol::elicitation::ElicitCreateRequest;
            let create_request = ElicitCreateRequest {
                method: "elicitation/create".to_string(),
                params: request_params.clone(),
            };

            // Check if provider can handle this request
            if !self.provider.can_handle(&create_request) {
                let error_response =
                    turul_mcp_protocol_2025_06_18::elicitation::ElicitResult::cancel();
                return serde_json::to_value(error_response).map_err(McpError::from);
            }

            // Delegate to the elicitation provider
            let result = self.provider.elicit(&create_request).await?;

            serde_json::to_value(result).map_err(McpError::from)
        } else {
            Err(McpError::missing_param("ElicitCreateParams"))
        }
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["elicitation/create".to_string()]
    }
}

use crate::session::SessionManager;

/// Generic notifications handler for most notification endpoints
pub struct NotificationsHandler;

#[async_trait]
impl McpHandler for NotificationsHandler {
    async fn handle(&self, params: Option<Value>) -> McpResult<Value> {
        // Notifications are typically one-way, so we just log them
        tracing::info!("Received notification: {:?}", params);
        Ok(Value::Null)
    }

    fn supported_methods(&self) -> Vec<String> {
        vec![
            "notifications/message".to_string(),
            "notifications/progress".to_string(),
            "notifications/resources/listChanged".to_string(),
            "notifications/resources/updated".to_string(),
            "notifications/tools/listChanged".to_string(),
            "notifications/prompts/listChanged".to_string(),
            "notifications/roots/listChanged".to_string(),
        ]
    }
}

/// Special handler for notifications/initialized that manages session lifecycle
pub struct InitializedNotificationHandler {
    session_manager: Arc<SessionManager>,
}

impl InitializedNotificationHandler {
    pub fn new(session_manager: Arc<SessionManager>) -> Self {
        Self { session_manager }
    }
}

#[async_trait]
impl McpHandler for InitializedNotificationHandler {
    async fn handle(&self, _params: Option<Value>) -> McpResult<Value> {
        // This should not be called directly without session context
        tracing::warn!("notifications/initialized received without session context");
        Ok(Value::Null)
    }

    async fn handle_with_session(
        &self,
        params: Option<Value>,
        session: Option<SessionContext>,
    ) -> McpResult<Value> {
        tracing::info!("📨 Received notifications/initialized: {:?}", params);

        if let Some(session_ctx) = session {
            tracing::info!(
                "🔄 Processing notifications/initialized for session: {}",
                session_ctx.session_id
            );

            // Check if session is already initialized
            if self
                .session_manager
                .is_session_initialized(&session_ctx.session_id)
                .await
            {
                tracing::info!(
                    "ℹ️ Session {} already initialized, ignoring duplicate notifications/initialized",
                    session_ctx.session_id
                );
                return Ok(Value::Null);
            }

            // Get client info from session state (it should have been stored during the initialize request)
            let client_info_value = self
                .session_manager
                .get_session_state(&session_ctx.session_id, "client_info")
                .await;
            let capabilities_value = self
                .session_manager
                .get_session_state(&session_ctx.session_id, "client_capabilities")
                .await;
            let negotiated_version_value = self
                .session_manager
                .get_session_state(&session_ctx.session_id, "negotiated_version")
                .await;

            if let (
                Some(client_info_value),
                Some(capabilities_value),
                Some(negotiated_version_value),
            ) = (
                client_info_value,
                capabilities_value,
                negotiated_version_value,
            ) {
                // Deserialize the stored values
                use turul_mcp_protocol::{ClientCapabilities, Implementation, McpVersion};

                if let (Ok(client_info), Ok(client_capabilities), Ok(negotiated_version)) = (
                    serde_json::from_value::<Implementation>(client_info_value),
                    serde_json::from_value::<ClientCapabilities>(capabilities_value),
                    serde_json::from_value::<McpVersion>(negotiated_version_value),
                ) {
                    // Mark session as initialized now that we received the notification
                    if let Err(e) = self
                        .session_manager
                        .initialize_session_with_version(
                            &session_ctx.session_id,
                            client_info,
                            client_capabilities,
                            negotiated_version,
                        )
                        .await
                    {
                        tracing::error!(
                            "❌ Failed to initialize session {}: {}",
                            session_ctx.session_id,
                            e
                        );
                        return Err(turul_mcp_protocol::McpError::configuration(&format!(
                            "Failed to initialize session: {}",
                            e
                        )));
                    }

                    tracing::info!(
                        "✅ Session {} successfully initialized after receiving notifications/initialized",
                        session_ctx.session_id
                    );
                } else {
                    tracing::error!(
                        "❌ Failed to deserialize stored client info/capabilities/version for session {}",
                        session_ctx.session_id
                    );
                    return Err(turul_mcp_protocol::McpError::configuration(
                        "Failed to deserialize stored client info",
                    ));
                }
            } else {
                tracing::error!(
                    "❌ Missing stored client info/capabilities/version for session {}",
                    session_ctx.session_id
                );
                return Err(turul_mcp_protocol::McpError::configuration(
                    "Missing stored client info - session must call initialize first",
                ));
            }
        } else {
            tracing::warn!("⚠️ notifications/initialized received without session context");
        }

        Ok(Value::Null)
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["notifications/initialized".to_string()]
    }
}
