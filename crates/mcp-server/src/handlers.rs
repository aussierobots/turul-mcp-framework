//! MCP Handler System
//!
//! This module provides a standardized handler system for MCP endpoints.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::{McpResult, SessionContext};
use mcp_protocol::McpError;

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
            },
            CompletionSuggestion {
                value: "example2".to_string(),
                label: Some("Example completion 2".to_string()),
                description: Some("Another sample completion".to_string()),
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
    async fn handle(&self, _params: Option<Value>) -> McpResult<Value> {
        // This is a simplified implementation - in practice you'd parse the method
        // and handle prompts/list vs prompts/get differently
        use mcp_protocol::prompts::{ListPromptsResponse, Prompt};
        
        let prompts: Vec<Prompt> = self.prompts.values()
            .map(|p| Prompt::new(p.name()).with_description(p.description()))
            .collect();
        
        let response = ListPromptsResponse::new(prompts);
        serde_json::to_value(response).map_err(McpError::from)
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
    async fn handle(&self, _params: Option<Value>) -> McpResult<Value> {
        use mcp_protocol::resources::{ListResourcesResponse, Resource};
        
        let resources: Vec<Resource> = self.resources.values()
            .map(|r| Resource::new(r.uri(), r.name()).with_description(r.description()))
            .collect();
        
        let response = ListResourcesResponse::new(resources);
        serde_json::to_value(response).map_err(McpError::from)
    }
    
    fn supported_methods(&self) -> Vec<String> {
        vec!["resources/list".to_string(), "resources/read".to_string()]
    }
}

/// Trait for MCP resources
#[async_trait]
pub trait McpResource: Send + Sync {
    /// The URI of the resource
    fn uri(&self) -> &str;
    
    /// Human-readable name
    fn name(&self) -> &str;
    
    /// Description of the resource
    fn description(&self) -> &str;
    
    /// Read the resource content
    async fn read(&self) -> McpResult<Vec<mcp_protocol::resources::ResourceContent>>;
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
            tracing::info!("Setting log level to: {:?}", request.level);
            
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
    async fn handle(&self, _params: Option<Value>) -> McpResult<Value> {
        use mcp_protocol::roots::ListRootsResponse;
        
        let response = ListRootsResponse::new(self.roots.clone());
        serde_json::to_value(response).map_err(McpError::from)
    }
    
    fn supported_methods(&self) -> Vec<String> {
        vec!["roots/list".to_string()]
    }
}

/// Sampling handler for sampling/createMessage endpoint
pub struct SamplingHandler;

#[async_trait]
impl McpHandler for SamplingHandler {
    async fn handle(&self, _params: Option<Value>) -> McpResult<Value> {
        use mcp_protocol::sampling::{CreateMessageResponse, SamplingMessage};
        
        // Default implementation - return a simple message
        let message = SamplingMessage {
            role: "assistant".to_string(),
            content: mcp_protocol::sampling::MessageContent::Text {
                text: "This is a sample message generated by the MCP server".to_string(),
            },
        };
        
        let response = CreateMessageResponse {
            message,
            stop_reason: Some("stop".to_string()),
        };
        
        serde_json::to_value(response).map_err(McpError::from)
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
    async fn handle(&self, _params: Option<Value>) -> McpResult<Value> {
        use mcp_protocol::templates::{ListTemplatesResponse, Template};
        
        let templates: Vec<Template> = self.templates.values()
            .map(|t| Template::new(t.name()).with_description(t.description()))
            .collect();
        
        let response = ListTemplatesResponse::new(templates);
        serde_json::to_value(response).map_err(McpError::from)
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

/// Elicitation handler for elicitation/request endpoint
pub struct ElicitationHandler;

#[async_trait]
impl McpHandler for ElicitationHandler {
    async fn handle(&self, params: Option<Value>) -> McpResult<Value> {
        use mcp_protocol::elicitation::{ElicitationRequestParams, ElicitationResponse, ElicitationRequestResult};
        
        if let Some(params) = params {
            let request: ElicitationRequestParams = serde_json::from_value(params)?;
            
            // In a real implementation, this would present the elicitation UI to the user
            // For demonstration purposes, we return a simulated cancelled response
            tracing::info!("Elicitation request: {} - {}", 
                request.request.title.as_deref().unwrap_or("Untitled"), 
                request.request.prompt
            );
            
            let response = ElicitationResponse::cancelled()
                .with_message("Elicitation not implemented in this server - this is a placeholder response");
                
            let result = ElicitationRequestResult::new(response);
            serde_json::to_value(result).map_err(McpError::from)
        } else {
            Err(McpError::missing_param("ElicitationRequestParams"))
        }
    }
    
    fn supported_methods(&self) -> Vec<String> {
        vec!["elicitation/request".to_string()]
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
            "notifications/resources/list_changed".to_string(),
            "notifications/resources/updated".to_string(),
            "notifications/tools/list_changed".to_string(),
            "notifications/prompts/list_changed".to_string(),
        ]
    }
}