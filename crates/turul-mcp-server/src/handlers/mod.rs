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
use turul_mcp_protocol::{McpError, WithMeta};

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

/// Prompts list handler for prompts/list endpoint only
pub struct PromptsListHandler {
    prompts: HashMap<String, Arc<dyn McpPrompt>>,
}

impl PromptsListHandler {
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
impl McpHandler for PromptsListHandler {
    async fn handle(&self, params: Option<Value>) -> McpResult<Value> {
        // Handle prompts/list with pagination support
        use turul_mcp_protocol::meta::{Cursor, PaginatedResponse};
        use turul_mcp_protocol::prompts::{ListPromptsParams, ListPromptsResult, Prompt};

        // Parse typed parameters with proper error handling (MCP compliance)
        let list_params = if let Some(params_value) = params {
            serde_json::from_value::<ListPromptsParams>(params_value).map_err(|e| {
                McpError::InvalidParameters(format!("Invalid parameters for prompts/list: {}", e))
            })?
        } else {
            ListPromptsParams::new()
        };
        
        let cursor = list_params.cursor;

        debug!("Listing prompts with cursor: {:?}", cursor);

        // Convert all prompts and sort by name for stable ordering
        let mut all_prompts: Vec<Prompt> = self
            .prompts
            .values()
            .map(|p| {
                let mut prompt = Prompt::new(p.name());
                if let Some(desc) = p.description() {
                    prompt = prompt.with_description(desc);
                }
                // Include arguments from the prompt object
                if let Some(args) = p.arguments() {
                    prompt = prompt.with_arguments(args.clone());
                }
                prompt
            })
            .collect();
        
        // Sort by name to ensure stable pagination ordering (MCP 2025-06-18 requirement)
        all_prompts.sort_by(|a, b| a.name.cmp(&b.name));

        // Implement cursor-based pagination
        const DEFAULT_PAGE_SIZE: usize = 50; // MCP suggested default
        let page_size = DEFAULT_PAGE_SIZE;
        
        // Find starting index based on cursor
        let start_index = if let Some(cursor) = &cursor {
            // Cursor contains the last name from previous page
            let cursor_name = cursor.as_str();
            
            // Find the position after the cursor name
            all_prompts.iter()
                .position(|p| p.name.as_str() > cursor_name)
                .unwrap_or(all_prompts.len())
        } else {
            0 // No cursor = start from beginning
        };
        
        // Calculate end index for this page
        let end_index = std::cmp::min(start_index + page_size, all_prompts.len());
        
        // Extract page of prompts
        let page_prompts: Vec<Prompt> = all_prompts[start_index..end_index].to_vec();
        
        // Determine if there are more prompts after this page
        let has_more = end_index < all_prompts.len();
        
        // Generate next cursor if there are more prompts
        let next_cursor = if has_more {
            // Cursor should be the name of the last item in current page
            page_prompts.last().map(|p| Cursor::new(&p.name))
        } else {
            None
        };
        
        debug!(
            "Prompt pagination: start={}, end={}, page_size={}, has_more={}, next_cursor={:?}",
            start_index, end_index, page_prompts.len(), has_more, next_cursor
        );

        let mut base_response = ListPromptsResult::new(page_prompts);

        // Set top-level nextCursor field on the result before wrapping
        if let Some(ref cursor) = next_cursor {
            base_response = base_response.with_next_cursor(cursor.clone());
        }

        let total = Some(all_prompts.len() as u64);

        let mut paginated_response = PaginatedResponse::with_pagination(
            base_response,
            next_cursor,
            total,
            has_more,
        );

        // Propagate optional _meta from request to response (MCP 2025-06-18 compliance)
        if let Some(meta) = list_params.meta {
            let mut meta_obj = turul_mcp_protocol::meta::Meta::new();
            meta_obj.extra = meta;
            paginated_response = paginated_response.with_meta(meta_obj);
        }

        serde_json::to_value(paginated_response).map_err(McpError::from)
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["prompts/list".to_string()]
    }
}

/// Prompts get handler for prompts/get endpoint only
pub struct PromptsGetHandler {
    prompts: HashMap<String, Arc<dyn McpPrompt>>,
}

impl PromptsGetHandler {
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
impl McpHandler for PromptsGetHandler {
    async fn handle(&self, params: Option<Value>) -> McpResult<Value> {
        use turul_mcp_protocol::prompts::{GetPromptParams, GetPromptResult};
        use std::collections::HashMap as StdHashMap;

        // Parse get prompt parameters
        let params = params.ok_or_else(|| McpError::missing_param("GetPromptParams"))?;
        let get_params: GetPromptParams = serde_json::from_value(params)?;

        debug!("Getting prompt: {} with arguments: {:?}", get_params.name, get_params.arguments);

        // Find the prompt by name
        let prompt = self
            .prompts
            .get(&get_params.name)
            .ok_or_else(|| {
                McpError::invalid_param_type("name", "existing prompt name", &get_params.name)
            })?;

        // Validate required arguments against prompt definition (MCP 2025-06-18 compliance)
        if let Some(prompt_arguments) = prompt.arguments() {
            let empty_args = StdHashMap::new();
            let provided_args = get_params.arguments.as_ref().unwrap_or(&empty_args);
            
            for arg_def in prompt_arguments {
                let is_required = arg_def.required.unwrap_or(false);
                if is_required && !provided_args.contains_key(&arg_def.name) {
                    return Err(McpError::InvalidParameters(format!(
                        "Missing required argument '{}' for prompt '{}'", 
                        arg_def.name, 
                        get_params.name
                    )));
                }
            }
        }

        // Convert arguments from HashMap<String, String> to HashMap<String, Value> if needed
        let arguments = match get_params.arguments {
            Some(args) => {
                let mut value_args = StdHashMap::new();
                for (key, value) in args {
                    value_args.insert(key, serde_json::Value::String(value));
                }
                value_args
            }
            None => StdHashMap::new(),
        };

        // Generate prompt messages using the prompt implementation
        // Note: MCP 2025-06-18 spec enforces only 'user' and 'assistant' roles via Role enum - no 'system' role
        let messages = prompt.render(Some(arguments)).await?;

        // Create response with messages
        let mut response = GetPromptResult::new(messages);
        if let Some(desc) = prompt.description() {
            response = response.with_description(desc);
        }
        
        // Propagate optional _meta from request to response (MCP 2025-06-18 compliance)
        if let Some(meta) = get_params.meta {
            response = response.with_meta(meta);
        }

        serde_json::to_value(response).map_err(McpError::from)
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["prompts/get".to_string()]
    }
}

/// Legacy handler for backward compatibility - use PromptsListHandler instead
pub type PromptsHandler = PromptsListHandler;

/// Import the proper McpPrompt trait from the prompt module
pub use crate::prompt::McpPrompt;

/// Resources list handler for resources/list endpoint only
pub struct ResourcesListHandler {
    resources: HashMap<String, Arc<dyn McpResource>>,
}

impl ResourcesListHandler {
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
impl McpHandler for ResourcesListHandler {
    async fn handle(&self, params: Option<Value>) -> McpResult<Value> {
        use turul_mcp_protocol::meta::{Cursor, PaginatedResponse};
        use turul_mcp_protocol::resources::{ListResourcesParams, ListResourcesResult, Resource};

        // Parse typed parameters with proper error handling (MCP compliance)
        let list_params = if let Some(params_value) = params {
            serde_json::from_value::<ListResourcesParams>(params_value).map_err(|e| {
                McpError::InvalidParameters(format!("Invalid parameters for resources/list: {}", e))
            })?
        } else {
            ListResourcesParams::new()
        };
        
        let cursor = list_params.cursor;

        debug!("Listing resources with cursor: {:?}", cursor);

        // Convert all resources to descriptors and sort by URI for stable ordering
        let mut all_resources: Vec<Resource> = self
            .resources
            .values()
            .map(|r| resource_to_descriptor(r.as_ref()))
            .collect();
        
        // Sort by URI to ensure stable pagination ordering (MCP 2025-06-18 requirement)
        all_resources.sort_by(|a, b| a.uri.cmp(&b.uri));

        // Implement cursor-based pagination
        const DEFAULT_PAGE_SIZE: usize = 50; // MCP suggested default
        let page_size = DEFAULT_PAGE_SIZE;
        
        // Find starting index based on cursor
        let start_index = if let Some(cursor) = &cursor {
            // Cursor contains the last URI from previous page
            let cursor_uri = cursor.as_str();
            
            // Find the position after the cursor URI
            all_resources.iter()
                .position(|r| r.uri.as_str() > cursor_uri)
                .unwrap_or(all_resources.len())
        } else {
            0 // No cursor = start from beginning
        };
        
        // Calculate end index for this page
        let end_index = std::cmp::min(start_index + page_size, all_resources.len());
        
        // Extract page of resources
        let page_resources: Vec<Resource> = all_resources[start_index..end_index].to_vec();
        
        // Determine if there are more resources after this page
        let has_more = end_index < all_resources.len();
        
        // Generate next cursor if there are more resources
        let next_cursor = if has_more {
            // Cursor should be the URI of the last item in current page
            page_resources.last().map(|r| Cursor::new(&r.uri))
        } else {
            None
        };
        
        debug!(
            "Resource pagination: start={}, end={}, page_size={}, has_more={}, next_cursor={:?}",
            start_index, end_index, page_resources.len(), has_more, next_cursor
        );

        let mut base_response = ListResourcesResult::new(page_resources);

        // Set top-level nextCursor field on the result before wrapping
        if let Some(ref cursor) = next_cursor {
            base_response = base_response.with_next_cursor(cursor.clone());
        }

        let total = Some(all_resources.len() as u64);

        let mut paginated_response = PaginatedResponse::with_pagination(
            base_response,
            next_cursor,
            total,
            has_more,
        );

        // Propagate optional _meta from request to response (MCP 2025-06-18 compliance)
        if let Some(meta) = list_params.meta {
            let mut meta_obj = turul_mcp_protocol::meta::Meta::new();
            meta_obj.extra = meta;
            paginated_response = paginated_response.with_meta(meta_obj);
        }

        serde_json::to_value(paginated_response).map_err(McpError::from)
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["resources/list".to_string()]
    }
}

/// Resources read handler for resources/read endpoint only
pub struct ResourcesReadHandler {
    resources: HashMap<String, Arc<dyn McpResource>>,
    uri_registry: Arc<crate::uri_template::UriTemplateRegistry>,
    security_middleware: Option<Arc<crate::security::SecurityMiddleware>>,
}

impl ResourcesReadHandler {
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
            uri_registry: Arc::new(crate::uri_template::UriTemplateRegistry::new()),
            security_middleware: Some(Arc::new(crate::security::SecurityMiddleware::default())),
        }
    }

    /// Create handler with custom security middleware
    pub fn with_security(mut self, middleware: Arc<crate::security::SecurityMiddleware>) -> Self {
        self.security_middleware = Some(middleware);
        self
    }

    /// Create handler without security middleware (for testing or trusted environments)
    pub fn without_security(mut self) -> Self {
        self.security_middleware = None;
        self
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

    /// Add a dynamic resource with URI template support
    pub fn add_template_resource<R: McpResource + 'static>(
        mut self, 
        template: crate::uri_template::UriTemplate,
        resource: R
    ) -> Self {
        // Register the template in the registry
        Arc::get_mut(&mut self.uri_registry)
            .expect("URI registry should not be shared yet")
            .register(template.clone());
        
        // Store the resource using the template pattern as key
        let pattern = template.pattern().to_string();
        self.resources.insert(pattern, Arc::new(resource));
        self
    }

    /// Add a dynamic resource with URI template support (Arc version)
    pub fn add_template_resource_arc(
        mut self, 
        template: crate::uri_template::UriTemplate,
        resource: Arc<dyn McpResource>
    ) -> Self {
        // Register the template in the registry
        Arc::get_mut(&mut self.uri_registry)
            .expect("URI registry should not be shared yet")
            .register(template.clone());
        
        // Store the resource using the template pattern as key
        let pattern = template.pattern().to_string();
        self.resources.insert(pattern, resource);
        self
    }
}

#[async_trait]
impl McpHandler for ResourcesReadHandler {
    async fn handle(&self, params: Option<Value>) -> McpResult<Value> {
        // Delegate to handle_with_session with no session
        self.handle_with_session(params, None).await
    }

    async fn handle_with_session(
        &self,
        params: Option<Value>,
        session: Option<SessionContext>,
    ) -> McpResult<Value> {
        use turul_mcp_protocol::resources::{ReadResourceParams, ReadResourceResult};

        // Security validation
        if let Some(security_middleware) = &self.security_middleware {
            security_middleware.validate_request("resources/read", params.as_ref(), session.as_ref())?;
        }

        // Parse read resource parameters
        let params = params.ok_or_else(|| McpError::missing_param("ReadResourceParams"))?;
        let read_params: ReadResourceParams = serde_json::from_value(params)?;

        debug!("Reading resource with URI: {}", read_params.uri);

        // Additional security validation for the specific URI
        if let Some(security_middleware) = &self.security_middleware {
            // Re-validate the URI after parsing (defense in depth)
            let uri_params = serde_json::json!({"uri": read_params.uri});
            security_middleware.validate_request("resources/read", Some(&uri_params), session.as_ref())?;
        }

        // First try to match against URI templates
        if let Some(template) = self.uri_registry.find_matching(&read_params.uri) {
            debug!("Found matching URI template: {}", template.pattern());
            
            // Extract variables from the URI
            let template_vars = template.extract(&read_params.uri)?;
            debug!("Extracted template variables: {:?}", template_vars);
            
            // Find the resource by template pattern
            let resource = self
                .resources
                .get(template.pattern())
                .ok_or_else(|| {
                    McpError::invalid_param_type("template", "registered template pattern", template.pattern())
                })?;

            // Create enhanced params with template variables
            let mut enhanced_params = serde_json::to_value(&read_params)?;
            if let Some(params_obj) = enhanced_params.as_object_mut() {
                params_obj.insert("template_variables".to_string(), serde_json::to_value(template_vars)?);
            }

            let contents = resource.read(Some(enhanced_params)).await?;
            
            // Validate content before returning
            if let Some(security_middleware) = &self.security_middleware {
                for content in &contents {
                    match content {
                        turul_mcp_protocol::resources::ResourceContent::Text(text_content) => {
                            if let Some(mime_type) = &text_content.mime_type {
                                security_middleware.resource_access_control().validate_mime_type(mime_type)?;
                            }
                            let size = text_content.text.len() as u64;
                            security_middleware.resource_access_control().validate_size(size)?;
                        }
                        turul_mcp_protocol::resources::ResourceContent::Blob(blob_content) => {
                            if let Some(mime_type) = &blob_content.mime_type {
                                security_middleware.resource_access_control().validate_mime_type(mime_type)?;
                            }
                            let size = blob_content.blob.len() as u64;
                            security_middleware.resource_access_control().validate_size(size)?;
                        }
                    }
                }
            }

            let response = ReadResourceResult::new(contents);
            return serde_json::to_value(response).map_err(McpError::from);
        }

        // Fall back to exact URI matching
        let resource = self
            .resources
            .get(&read_params.uri)
            .ok_or_else(|| {
                McpError::invalid_param_type("uri", "existing resource URI or template pattern", &read_params.uri)
            })?;

        // Call the resource's read method with original params
        let params = Some(serde_json::to_value(&read_params)?);
        let contents = resource.read(params).await?;

        // Validate content before returning
        if let Some(security_middleware) = &self.security_middleware {
            for content in &contents {
                match content {
                    turul_mcp_protocol::resources::ResourceContent::Text(text_content) => {
                        if let Some(mime_type) = &text_content.mime_type {
                            security_middleware.resource_access_control().validate_mime_type(mime_type)?;
                        }
                        let size = text_content.text.len() as u64;
                        security_middleware.resource_access_control().validate_size(size)?;
                    }
                    turul_mcp_protocol::resources::ResourceContent::Blob(blob_content) => {
                        if let Some(mime_type) = &blob_content.mime_type {
                            security_middleware.resource_access_control().validate_mime_type(mime_type)?;
                        }
                        let size = blob_content.blob.len() as u64;
                        security_middleware.resource_access_control().validate_size(size)?;
                    }
                }
            }
        }

        let response = ReadResourceResult::new(contents);
        serde_json::to_value(response).map_err(McpError::from)
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["resources/read".to_string()]
    }
}

/// Legacy handler for backward compatibility - use ResourcesListHandler instead
pub type ResourcesHandler = ResourcesListHandler;

/// Logging handler for logging/setLevel endpoint
pub struct LoggingHandler;

#[async_trait]
impl McpHandler for LoggingHandler {
    async fn handle(&self, params: Option<Value>) -> McpResult<Value> {
        // Fallback for when no session is provided
        use turul_mcp_protocol::logging::SetLevelParams;

        if let Some(params) = params {
            let set_level_params: SetLevelParams = serde_json::from_value(params)?;

            // Without session context, just log the request but can't store per-session
            tracing::warn!("LoggingHandler.handle() called without session context - cannot store level per-session");
            tracing::info!("Would set log level to: {:?}", set_level_params.level);

            // MCP logging/setLevel doesn't return data, just success
            serde_json::to_value(json!({})).map_err(McpError::from)
        } else {
            Err(McpError::missing_param("SetLevelParams"))
        }
    }

    async fn handle_with_session(
        &self,
        params: Option<Value>,
        session: Option<SessionContext>,
    ) -> McpResult<Value> {
        use turul_mcp_protocol::logging::SetLevelParams;

        // Parse params - returns InvalidParams (-32602) if fails  
        let params = params.ok_or_else(|| 
            McpError::missing_param("params"))?;
        
        let set_level_params: SetLevelParams = serde_json::from_value(params)?;

        // Require session - returns configuration error if missing
        let session_ctx = session.ok_or_else(|| 
            McpError::configuration("Session required for logging/setLevel"))?;
        
        // Check initialization - returns configuration error if not initialized
        if !(session_ctx.is_initialized)().await {
            return Err(McpError::configuration(
                "Session must be initialized before setting logging level"
            ));
        }
        
        // Set the level
        session_ctx.set_logging_level(set_level_params.level).await;
        
        tracing::info!("🎯 Set logging level for session {}: {:?}", 
            session_ctx.session_id, set_level_params.level);
        
        // Verify persistence - returns configuration error if fails
        let stored_level = session_ctx.get_logging_level().await;
        if stored_level != set_level_params.level {
            return Err(McpError::configuration(
                "Failed to persist logging level in session storage"
            ));
        }
        
        // Send confirmation notification
        session_ctx.notify_log(
            turul_mcp_protocol::logging::LoggingLevel::Info,
            serde_json::json!(format!("Logging level changed to: {:?}", set_level_params.level)),
            None,
            None
        ).await;
        
        // Success returns empty object per MCP spec
        Ok(json!({}))
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

        // Sort roots by URI for stable ordering
        let mut all_roots = self.roots.clone();
        all_roots.sort_by(|a, b| a.uri.cmp(&b.uri));

        // Implement cursor-based pagination
        const DEFAULT_PAGE_SIZE: usize = 50; // MCP suggested default
        let page_size = DEFAULT_PAGE_SIZE;
        
        // Find starting index based on cursor
        let start_index = if let Some(cursor) = &cursor {
            // Cursor contains the last URI from previous page
            let cursor_uri = cursor.as_str();
            
            // Find the position after the cursor URI
            all_roots.iter()
                .position(|r| r.uri.as_str() > cursor_uri)
                .unwrap_or(all_roots.len())
        } else {
            0 // No cursor = start from beginning
        };
        
        // Calculate end index for this page
        let end_index = std::cmp::min(start_index + page_size, all_roots.len());
        
        // Extract page of roots
        let page_roots = all_roots[start_index..end_index].to_vec();
        
        // Determine if there are more roots after this page
        let has_more = end_index < all_roots.len();
        
        // Generate next cursor if there are more roots
        let next_cursor = if has_more {
            // Cursor should be the URI of the last item in current page
            page_roots.last().map(|r| Cursor::new(&r.uri))
        } else {
            None
        };
        
        debug!(
            "Root pagination: start={}, end={}, page_size={}, has_more={}, next_cursor={:?}",
            start_index, end_index, page_roots.len(), has_more, next_cursor
        );

        let base_response = ListRootsResult::new(page_roots);

        // Note: ListRootsResult doesn't have next_cursor field - roots may not be paginatable per MCP spec

        let total = Some(all_roots.len() as u64);

        let paginated_response = PaginatedResponse::with_pagination(
            base_response,
            next_cursor,
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


/// Resource templates handler for resources/templates/list endpoint
pub struct ResourceTemplatesHandler {
    templates: Vec<(crate::uri_template::UriTemplate, Arc<dyn McpResource>)>,
}

impl ResourceTemplatesHandler {
    pub fn new() -> Self {
        Self {
            templates: Vec::new(),
        }
    }

    pub fn with_templates(mut self, templates: Vec<(crate::uri_template::UriTemplate, Arc<dyn McpResource>)>) -> Self {
        self.templates = templates;
        self
    }
}

#[async_trait]
impl McpHandler for ResourceTemplatesHandler {
    async fn handle(&self, params: Option<Value>) -> McpResult<Value> {
        use turul_mcp_protocol::meta::Cursor;

        // Parse typed parameters with proper error handling (MCP compliance)
        use turul_mcp_protocol::resources::ListResourceTemplatesParams;
        let list_params = if let Some(params_value) = params {
            serde_json::from_value::<ListResourceTemplatesParams>(params_value).map_err(|e| {
                McpError::InvalidParameters(format!("Invalid parameters for resources/templates/list: {}", e))
            })?
        } else {
            ListResourceTemplatesParams::new()
        };

        let cursor = list_params.cursor;
        debug!("Listing resource templates with cursor: {:?}", cursor);

        tracing::info!("Resource templates list requested - {} templates registered", self.templates.len());

        use turul_mcp_protocol::resources::{ListResourceTemplatesResult, ResourceTemplate};
        
        // Convert registered templates to ResourceTemplate objects and sort by template for stable ordering
        let mut all_templates: Vec<ResourceTemplate> = self
            .templates
            .iter()
            .map(|(uri_template, resource)| {
                let template_name = resource.name();
                let mut template = ResourceTemplate::new(template_name, uri_template.pattern());
                if let Some(desc) = resource.description() {
                    template = template.with_description(desc);
                }
                // Add MIME type if the resource provides it
                if let Some(mime_type) = resource.mime_type() {
                    template = template.with_mime_type(mime_type);
                }
                template
            })
            .collect();
        
        // Sort by uri_template to ensure stable pagination ordering (MCP 2025-06-18 requirement)
        all_templates.sort_by(|a, b| a.uri_template.cmp(&b.uri_template));

        // Implement cursor-based pagination
        const DEFAULT_PAGE_SIZE: usize = 50; // MCP suggested default
        let page_size = DEFAULT_PAGE_SIZE;
        
        // Find starting index based on cursor
        let start_index = if let Some(cursor) = &cursor {
            // Cursor contains the last uri_template from previous page
            let cursor_template = cursor.as_str();
            
            // Find the position after the cursor template
            all_templates.iter()
                .position(|t| t.uri_template.as_str() > cursor_template)
                .unwrap_or(all_templates.len())
        } else {
            0 // No cursor = start from beginning
        };
        
        // Calculate end index for this page
        let end_index = std::cmp::min(start_index + page_size, all_templates.len());
        
        // Extract the page
        let page_templates = all_templates[start_index..end_index].to_vec();
        
        // Calculate pagination metadata
        let total = Some(all_templates.len() as u64);
        let has_more = end_index < all_templates.len();
        let next_cursor = if has_more {
            // Next cursor is the last template name in this page
            page_templates.last().map(|t| Cursor::new(&t.uri_template))
        } else {
            None // No more pages
        };
        
        debug!(
            "Resource template pagination: page_size={}, has_more={}, next_cursor={:?}",
            page_templates.len(), has_more, next_cursor
        );
        
        let mut base_response = ListResourceTemplatesResult::new(page_templates);

        // Set top-level nextCursor field on the result before wrapping
        if let Some(ref cursor) = next_cursor {
            base_response = base_response.with_next_cursor(cursor.clone());
        }

        use turul_mcp_protocol::meta::PaginatedResponse;
        let mut paginated_response = PaginatedResponse::with_pagination(
            base_response,
            next_cursor,
            total,
            has_more,
        );

        // Propagate optional _meta from request to response (MCP 2025-06-18 compliance)
        if let Some(meta) = list_params.meta {
            let mut meta_obj = turul_mcp_protocol::meta::Meta::new();
            meta_obj.extra = meta;
            paginated_response = paginated_response.with_meta(meta_obj);
        }

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
