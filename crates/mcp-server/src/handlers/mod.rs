//! MCP Handler System
//!
//! This module provides a standardized handler system for MCP endpoints.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tracing::debug;

use crate::resource::{McpResource, resource_to_descriptor};

use crate::{McpResult, SessionContext};
use mcp_protocol::McpError;

//pub mod response;
//pub use response::*;

/// Generic MCP handler trait
#[async_trait]
pub trait McpHandler: Send + Sync {
    /// Handle an MCP request
    async fn handle(&self, params: Option<Value>) -> McpResult<Value>;
    
    /// Handle an MCP request with session context (default implementation calls handle)
    async fn handle_with_session(&self, params: Option<Value>, _session: Option<SessionContext>) -> McpResult<Value> {
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
        use mcp_protocol::completion::{CompletionResponse, CompletionSuggestion};
        
        // Default implementation - can be overridden by users
        let suggestions = vec![
            CompletionSuggestion {
                value: "example1".to_string(),
                label: Some("Example completion 1".to_string()),
                description: Some("A sample completion".to_string()),
                annotations: None,
            },
            CompletionSuggestion {
                value: "example2".to_string(),
                label: Some("Example completion 2".to_string()),
                description: Some("Another sample completion".to_string()),
                annotations: None,
            },
        ];
        
        let response = CompletionResponse::new(suggestions);
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
        self.prompts.insert(prompt.name().to_string(), Arc::new(prompt));
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
        use mcp_protocol::prompts::{ListPromptsResponse, Prompt};
        use mcp_protocol_2025_06_18::meta::{PaginatedResponse, Cursor};
        
        // Parse cursor from params if provided
        let cursor = params.as_ref()
            .and_then(|p| p.get("cursor"))
            .and_then(|c| c.as_str())
            .map(Cursor::from);
        
        debug!("Listing prompts with cursor: {:?}", cursor);
        
        let prompts: Vec<Prompt> = self.prompts.values()
            .map(|p| Prompt::new(p.name()).with_description(p.description()))
            .collect();
        
        let base_response = ListPromptsResponse::new(prompts.clone());
        
        // Add pagination metadata
        let has_more = false; // In a real implementation, this would depend on the actual data
        let total = Some(prompts.len() as u64);
        
        let paginated_response = PaginatedResponse::with_pagination(
            base_response,
            None, // next_cursor - would be calculated based on current page
            total,
            has_more
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
    async fn generate(&self, args: HashMap<String, Value>) -> McpResult<Vec<mcp_protocol::prompts::PromptMessage>>;
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
        self.resources.insert(resource.uri().to_string(), Arc::new(resource));
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
        use mcp_protocol::resources::{ListResourcesResponse, Resource};
        use mcp_protocol_2025_06_18::meta::{PaginatedResponse, Cursor};
        
        // Parse cursor from params if provided
        let cursor = params.as_ref()
            .and_then(|p| p.get("cursor"))
            .and_then(|c| c.as_str())
            .map(Cursor::from);
        
        debug!("Listing resources with cursor: {:?}", cursor);
        
        let resources: Vec<Resource> = self.resources.values()
            .map(|r| resource_to_descriptor(r.as_ref()))
            .collect();
        
        let base_response = ListResourcesResponse::new(resources.clone());
        
        // Add pagination metadata if applicable
        let has_more = false; // In a real implementation, this would depend on the actual data
        let total = Some(resources.len() as u64);
        
        let paginated_response = PaginatedResponse::with_pagination(
            base_response,
            None, // next_cursor - would be calculated based on current page
            total,
            has_more
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
        use mcp_protocol::logging::SetLevelRequest;
        
        if let Some(params) = params {
            let request: SetLevelRequest = serde_json::from_value(params)?;
            
            // In a real implementation, you'd set the actual log level
            tracing::info!("Setting log level to: {:?}", request.params.level);
            
            // MCP logging/setLevel doesn't return data, just success
            Ok(Value::Null)
        } else {
            Err(McpError::missing_param("SetLevelRequest"))
        }
    }
    
    fn supported_methods(&self) -> Vec<String> {
        vec!["logging/setLevel".to_string()]
    }
}

/// Roots handler for roots/list endpoint
pub struct RootsHandler {
    roots: Vec<mcp_protocol::roots::Root>,
}

impl RootsHandler {
    pub fn new() -> Self {
        Self {
            roots: Vec::new(),
        }
    }
    
    pub fn add_root(mut self, root: mcp_protocol::roots::Root) -> Self {
        self.roots.push(root);
        self
    }
}

#[async_trait]
impl McpHandler for RootsHandler {
    async fn handle(&self, params: Option<Value>) -> McpResult<Value> {
        use mcp_protocol::roots::ListRootsResponse;
        use mcp_protocol_2025_06_18::meta::{PaginatedResponse, Cursor};
        
        // Parse cursor from params if provided
        let cursor = params.as_ref()
            .and_then(|p| p.get("cursor"))
            .and_then(|c| c.as_str())
            .map(Cursor::from);
        
        debug!("Listing roots with cursor: {:?}", cursor);
        
        let base_response = ListRootsResponse::new(self.roots.clone());
        
        // Add pagination metadata
        let has_more = false; // In a real implementation, this would depend on the actual data
        let total = Some(self.roots.len() as u64);
        
        let paginated_response = PaginatedResponse::with_pagination(
            base_response,
            None, // next_cursor - would be calculated based on current page
            total,
            has_more
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
        use mcp_protocol::sampling::{CreateMessageResponse, SamplingMessage};
        use mcp_protocol_2025_06_18::meta::{ProgressResponse, ProgressToken};
        
        // Parse progress token if provided
        let progress_token = params.as_ref()
            .and_then(|p| p.get("progressToken"))
            .and_then(|t| t.as_str())
            .map(ProgressToken::from);
        
        // Default implementation - return a simple message
        let message = SamplingMessage {
            role: "assistant".to_string(),
            content: mcp_protocol::sampling::MessageContent::Text {
                text: "This is a sample message generated by the MCP server".to_string(),
            },
        };
        
        let base_response = CreateMessageResponse {
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
            Some(1)
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
        self.templates.insert(template.name().to_string(), Arc::new(template));
        self
    }
}

#[async_trait]
impl McpHandler for TemplatesHandler {
    async fn handle(&self, params: Option<Value>) -> McpResult<Value> {
        use mcp_protocol::templates::{ListTemplatesResponse, Template};
        use mcp_protocol_2025_06_18::meta::{PaginatedResponse, Cursor};
        
        // Parse cursor from params if provided
        let cursor = params.as_ref()
            .and_then(|p| p.get("cursor"))
            .and_then(|c| c.as_str())
            .map(Cursor::from);
        
        debug!("Listing templates with cursor: {:?}", cursor);
        
        let templates: Vec<Template> = self.templates.values()
            .map(|t| Template::new(t.name()).with_description(t.description()))
            .collect();
        
        let base_response = ListTemplatesResponse::new(templates.clone());
        
        // Add pagination metadata if applicable
        let has_more = false; // In a real implementation, this would depend on the actual data
        let total = Some(templates.len() as u64);
        
        let paginated_response = PaginatedResponse::with_pagination(
            base_response,
            None, // next_cursor - would be calculated based on current page
            total,
            has_more
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
        use mcp_protocol_2025_06_18::meta::{PaginatedResponse, Cursor};
        
        // Parse cursor from params if provided
        let cursor = params.as_ref()
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
            has_more
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
    async fn elicit(&self, request: &mcp_protocol_2025_06_18::elicitation::ElicitationRequest) -> McpResult<mcp_protocol_2025_06_18::elicitation::ElicitationResponse>;
    
    /// Check if this provider can handle a specific elicitation schema
    fn can_handle(&self, _request: &mcp_protocol_2025_06_18::elicitation::ElicitationRequest) -> bool {
        // Default implementation accepts all requests
        true
    }
}

/// Default console-based elicitation provider for demonstration
pub struct MockElicitationProvider;

#[async_trait]
impl ElicitationProvider for MockElicitationProvider {
    async fn elicit(&self, request: &mcp_protocol_2025_06_18::elicitation::ElicitationRequest) -> McpResult<mcp_protocol_2025_06_18::elicitation::ElicitationResponse> {
        use mcp_protocol_2025_06_18::elicitation::ElicitationResponse;
        
        if request.required {
            // For required elicitations, provide mock structured data
            let mock_data = serde_json::json!({
                "mock_response": true,
                "schema_description": request.description.as_deref().unwrap_or("No description"),
                "prompt": &request.prompt,
                "note": "This is a mock elicitation response for testing"
            });
            
            Ok(ElicitationResponse::completed(mock_data)
                .with_message("Mock elicitation completed"))
        } else {
            Ok(ElicitationResponse::cancelled()
                .with_message("Mock provider cancelled optional elicitation"))
        }
    }
    
    fn can_handle(&self, _request: &mcp_protocol_2025_06_18::elicitation::ElicitationRequest) -> bool {
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
        use mcp_protocol_2025_06_18::elicitation::{ElicitationRequestParams, ElicitationRequestResult};
        use mcp_protocol_2025_06_18::meta::{ProgressToken, Meta};
        
        if let Some(params) = params {
            let request_params: ElicitationRequestParams = serde_json::from_value(params)?;
            
            tracing::info!("Processing elicitation request: {} - {}", 
                request_params.request.title.as_deref().unwrap_or("Untitled"), 
                request_params.request.prompt
            );
            
            // Check if provider can handle this request
            if !self.provider.can_handle(&request_params.request) {
                let error_response = mcp_protocol_2025_06_18::elicitation::ElicitationResponse::cancelled()
                    .with_message("This elicitation request cannot be handled by the current provider");
                let result = ElicitationRequestResult::new(error_response);
                return serde_json::to_value(result).map_err(McpError::from);
            }
            
            // Delegate to the elicitation provider
            let response = self.provider.elicit(&request_params.request).await?;
            let mut result = ElicitationRequestResult::new(response);
            
            // Add comprehensive metadata for elicitation tracking
            let progress_meta = Meta::with_progress(1.0, Some(1), Some(1))
                .set_progress_token(request_params.progress_token.unwrap_or_else(|| 
                    ProgressToken::new(format!("elicit-{}", uuid::Uuid::new_v4().to_string()[..8].to_string()))))
                .add_extra("provider_type", "configurable")
                .add_extra("request_required", request_params.request.required)
                .add_extra("has_defaults", request_params.request.defaults.is_some())
                .add_extra("schema_type", match request_params.request.schema {
                    mcp_protocol_2025_06_18::schema::JsonSchema::Object { .. } => "object",
                    mcp_protocol_2025_06_18::schema::JsonSchema::Array { .. } => "array", 
                    mcp_protocol_2025_06_18::schema::JsonSchema::String { .. } => "string",
                    mcp_protocol_2025_06_18::schema::JsonSchema::Number { .. } => "number",
                    mcp_protocol_2025_06_18::schema::JsonSchema::Integer { .. } => "integer",
                    mcp_protocol_2025_06_18::schema::JsonSchema::Boolean { .. } => "boolean",
                });
            
            result = result.with_meta(progress_meta);
            serde_json::to_value(result).map_err(McpError::from)
        } else {
            Err(McpError::missing_param("ElicitationRequestParams"))
        }
    }
    
    fn supported_methods(&self) -> Vec<String> {
        vec!["elicitation/create".to_string()]
    }
}

/// Notifications handler for various notification endpoints
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
            "notifications/initialized".to_string(),
            "notifications/progress".to_string(),
            "notifications/resources/listChanged".to_string(),
            "notifications/resources/updated".to_string(),
            "notifications/tools/listChanged".to_string(),
            "notifications/prompts/listChanged".to_string(),
            "notifications/roots/listChanged".to_string(),
        ]
    }
}