//! MCP Server Builder
//!
//! This module provides a builder pattern for creating MCP servers.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

use crate::{McpTool, McpServer, Result, McpFrameworkError};
use crate::resource::McpResource;
use crate::{McpElicitation, McpPrompt, McpSampling, McpCompletion, McpLogger, McpRoot, McpNotification};
use crate::handlers::*;
use turul_mcp_protocol::{Implementation, ServerCapabilities};
use turul_mcp_protocol::initialize::*;





/// Builder for MCP servers
pub struct McpServerBuilder {
    /// Server implementation info
    name: String,
    version: String,
    title: Option<String>,
    
    /// Server capabilities
    capabilities: ServerCapabilities,
    
    /// Tools registered with the server
    tools: HashMap<String, Arc<dyn McpTool>>,
    
    /// Resources registered with the server
    resources: HashMap<String, Arc<dyn McpResource>>,
    
    /// Template resources (URI template -> resource)
    template_resources: Vec<(crate::uri_template::UriTemplate, Arc<dyn McpResource>)>,
    
    /// Prompts registered with the server
    prompts: HashMap<String, Arc<dyn McpPrompt>>,
    
    /// Elicitations registered with the server
    elicitations: HashMap<String, Arc<dyn McpElicitation>>,
    
    /// Sampling providers registered with the server
    sampling: HashMap<String, Arc<dyn McpSampling>>,
    
    /// Completion providers registered with the server
    completions: HashMap<String, Arc<dyn McpCompletion>>,
    
    /// Loggers registered with the server
    loggers: HashMap<String, Arc<dyn McpLogger>>,
    
    /// Root providers registered with the server
    root_providers: HashMap<String, Arc<dyn McpRoot>>,
    
    /// Notification providers registered with the server
    notifications: HashMap<String, Arc<dyn McpNotification>>,
    
    /// Handlers registered with the server
    handlers: HashMap<String, Arc<dyn McpHandler>>,
    
    /// Roots configured for the server
    roots: Vec<turul_mcp_protocol::roots::Root>,
    
    /// Optional instructions for clients
    instructions: Option<String>,
    
    /// Session configuration
    session_timeout_minutes: Option<u64>,
    session_cleanup_interval_seconds: Option<u64>,
    
    /// Session storage backend (defaults to InMemory if None)
    session_storage: Option<Arc<turul_mcp_session_storage::BoxedSessionStorage>>,
    
    /// MCP Lifecycle enforcement configuration
    strict_lifecycle: bool,
    
    /// HTTP configuration (if enabled)
    #[cfg(feature = "http")]
    bind_address: SocketAddr,
    #[cfg(feature = "http")]
    mcp_path: String,
    #[cfg(feature = "http")]
    enable_cors: bool,
    #[cfg(feature = "http")]
    enable_sse: bool,
    
    /// Validation errors collected during builder configuration
    validation_errors: Vec<String>,
}

impl McpServerBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        let tools = HashMap::new();
        let mut handlers: HashMap<String, Arc<dyn McpHandler>> = HashMap::new();
        
        // Add all standard MCP 2025-06-18 handlers by default
        handlers.insert("ping".to_string(), Arc::new(PingHandler));
        handlers.insert("completion/complete".to_string(), Arc::new(CompletionHandler));
        handlers.insert("resources/list".to_string(), Arc::new(ResourcesListHandler::new()));
        handlers.insert("resources/read".to_string(), Arc::new(ResourcesReadHandler::new()));
        handlers.insert("prompts/list".to_string(), Arc::new(PromptsListHandler::new()));
        handlers.insert("prompts/get".to_string(), Arc::new(PromptsGetHandler::new()));
        handlers.insert("logging/setLevel".to_string(), Arc::new(LoggingHandler));
        handlers.insert("roots/list".to_string(), Arc::new(RootsHandler::new()));
        handlers.insert("sampling/createMessage".to_string(), Arc::new(SamplingHandler));
        // Note: resources/templates/list is only registered if template resources are configured (see build method)
        handlers.insert("elicitation/create".to_string(), Arc::new(ElicitationHandler::with_mock_provider()));
        
        // Add all notification handlers (except notifications/initialized which is handled specially)
        let notifications_handler = Arc::new(NotificationsHandler);
        handlers.insert("notifications/message".to_string(), notifications_handler.clone());
        handlers.insert("notifications/progress".to_string(), notifications_handler.clone());
        handlers.insert("notifications/resources/listChanged".to_string(), notifications_handler.clone());
        handlers.insert("notifications/resources/updated".to_string(), notifications_handler.clone());
        handlers.insert("notifications/tools/listChanged".to_string(), notifications_handler.clone());
        handlers.insert("notifications/prompts/listChanged".to_string(), notifications_handler.clone());
        handlers.insert("notifications/roots/listChanged".to_string(), notifications_handler);
        
        // Note: notifications/initialized is handled by InitializedNotificationHandler in server.rs
        
        
        
        Self {
            name: "turul-mcp-server".to_string(),
            version: "1.0.0".to_string(),
            title: None,
            capabilities: ServerCapabilities::default(),
            tools,
            resources: HashMap::new(),
            template_resources: Vec::new(),
            prompts: HashMap::new(),
            elicitations: HashMap::new(),
            sampling: HashMap::new(),
            completions: HashMap::new(),
            loggers: HashMap::new(),
            root_providers: HashMap::new(),
            notifications: HashMap::new(),
            handlers,
            roots: Vec::new(),
            instructions: None,
            session_timeout_minutes: None,
            session_cleanup_interval_seconds: None,
            session_storage: None, // Default: InMemory storage
            strict_lifecycle: false, // Default: lenient mode for compatibility
            #[cfg(feature = "http")]
            bind_address: "127.0.0.1:8000".parse().unwrap(),
            #[cfg(feature = "http")]
            mcp_path: "/mcp".to_string(),
            #[cfg(feature = "http")]
            enable_cors: true,
            #[cfg(feature = "http")]
            enable_sse: cfg!(feature = "sse"),
            validation_errors: Vec::new(),
        }
    }

    /// Set the server name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Set the server version
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Set the server title (display name)
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Add instructions for clients
    pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    /// Register a tool with the server
    pub fn tool<T: McpTool + 'static>(mut self, tool: T) -> Self {
        let name = tool.name().to_string();
        self.tools.insert(name, Arc::new(tool));
        self
    }

    /// Register a function tool created with #[mcp_tool] macro
    /// 
    /// This method provides a more intuitive way to register function tools.
    /// The #[mcp_tool] macro generates a constructor function with the same name
    /// as your async function, so you can use the function name directly.
    /// 
    /// # Example
    /// ```rust,ignore
    /// use turul_mcp_derive::mcp_tool;
    /// use turul_mcp_server::McpServer;
    /// 
    /// #[mcp_tool(name = "add", description = "Add numbers")]
    /// async fn add_numbers(a: f64, b: f64) -> Result<f64, String> {
    ///     Ok(a + b)
    /// }
    /// 
    /// let server = McpServer::builder()
    ///     .name("math-server")
    ///     .tool_fn(add_numbers) // Use the function name directly!
    ///     .build()?;
    /// ```
    pub fn tool_fn<F, T>(self, func: F) -> Self 
    where
        F: Fn() -> T,
        T: McpTool + 'static,
    {
        // Call the helper function to get the tool instance
        self.tool(func())
    }

    /// Register multiple tools
    pub fn tools<T: McpTool + 'static, I: IntoIterator<Item = T>>(mut self, tools: I) -> Self {
        for tool in tools {
            self = self.tool(tool);
        }
        self
    }

    /// Register a resource with the server
    /// Note: URI validation is performed during build() to maintain builder pattern
    pub fn resource<R: McpResource + 'static>(mut self, resource: R) -> Self {
        let uri = resource.uri().to_string();
        
        // Validate URI and collect errors for later reporting
        if let Err(e) = self.validate_uri(&uri) {
            self.validation_errors.push(format!("Invalid resource URI '{}': {}", uri, e));
        }
        
        self.resources.insert(uri, Arc::new(resource));
        self
    }

    /// Register multiple resources
    pub fn resources<R: McpResource + 'static, I: IntoIterator<Item = R>>(mut self, resources: I) -> Self {
        for resource in resources {
            self = self.resource(resource);
        }
        self
    }

    /// Register a resource with URI template support
    pub fn template_resource<R: McpResource + 'static>(
        mut self, 
        template: crate::uri_template::UriTemplate,
        resource: R
    ) -> Self {
        // Validate template pattern is well-formed (MCP 2025-06-18 compliance)
        if let Err(e) = self.validate_uri_template(template.pattern()) {
            self.validation_errors.push(format!("Invalid resource template URI '{}': {}", template.pattern(), e));
        }
        
        self.template_resources.push((template, Arc::new(resource)));
        self
    }

    /// Register a prompt with the server
    pub fn prompt<P: McpPrompt + 'static>(mut self, prompt: P) -> Self {
        let name = prompt.name().to_string();
        self.prompts.insert(name, Arc::new(prompt));
        self
    }

    /// Register multiple prompts
    pub fn prompts<P: McpPrompt + 'static, I: IntoIterator<Item = P>>(mut self, prompts: I) -> Self {
        for prompt in prompts {
            self = self.prompt(prompt);
        }
        self
    }

    /// Register an elicitation provider with the server
    pub fn elicitation<E: McpElicitation + 'static>(mut self, elicitation: E) -> Self {
        let key = format!("elicitation_{}", self.elicitations.len());
        self.elicitations.insert(key, Arc::new(elicitation));
        self
    }

    /// Register multiple elicitation providers
    pub fn elicitations<E: McpElicitation + 'static, I: IntoIterator<Item = E>>(mut self, elicitations: I) -> Self {
        for elicitation in elicitations {
            self = self.elicitation(elicitation);
        }
        self
    }

    /// Register a sampling provider with the server
    pub fn sampling_provider<S: McpSampling + 'static>(mut self, sampling: S) -> Self {
        let key = format!("sampling_{}", self.sampling.len());
        self.sampling.insert(key, Arc::new(sampling));
        self
    }

    /// Register multiple sampling providers
    pub fn sampling_providers<S: McpSampling + 'static, I: IntoIterator<Item = S>>(mut self, sampling: I) -> Self {
        for s in sampling {
            self = self.sampling_provider(s);
        }
        self
    }

    /// Register a completion provider with the server
    pub fn completion_provider<C: McpCompletion + 'static>(mut self, completion: C) -> Self {
        let key = format!("completion_{}", self.completions.len());
        self.completions.insert(key, Arc::new(completion));
        self
    }

    /// Register multiple completion providers
    pub fn completion_providers<C: McpCompletion + 'static, I: IntoIterator<Item = C>>(mut self, completions: I) -> Self {
        for completion in completions {
            self = self.completion_provider(completion);
        }
        self
    }

    /// Register a logger with the server
    pub fn logger<L: McpLogger + 'static>(mut self, logger: L) -> Self {
        let key = format!("logger_{}", self.loggers.len());
        self.loggers.insert(key, Arc::new(logger));
        self
    }

    /// Register multiple loggers
    pub fn loggers<L: McpLogger + 'static, I: IntoIterator<Item = L>>(mut self, loggers: I) -> Self {
        for logger in loggers {
            self = self.logger(logger);
        }
        self
    }

    /// Register a root provider with the server
    pub fn root_provider<R: McpRoot + 'static>(mut self, root: R) -> Self {
        let key = format!("root_{}", self.root_providers.len());
        self.root_providers.insert(key, Arc::new(root));
        self
    }

    /// Register multiple root providers
    pub fn root_providers<R: McpRoot + 'static, I: IntoIterator<Item = R>>(mut self, roots: I) -> Self {
        for root in roots {
            self = self.root_provider(root);
        }
        self
    }

    /// Register a notification provider with the server
    pub fn notification_provider<N: McpNotification + 'static>(mut self, notification: N) -> Self {
        let key = format!("notification_{}", self.notifications.len());
        self.notifications.insert(key, Arc::new(notification));
        self
    }

    /// Register multiple notification providers
    pub fn notification_providers<N: McpNotification + 'static, I: IntoIterator<Item = N>>(mut self, notifications: I) -> Self {
        for notification in notifications {
            self = self.notification_provider(notification);
        }
        self
    }

    /// Validate URI is absolute and well-formed (reusing SecurityMiddleware logic)
    fn validate_uri(&self, uri: &str) -> std::result::Result<(), String> {
        // Check basic URI format - must have scheme
        if !uri.contains("://") {
            return Err("URI must be absolute with scheme (e.g., file://, http://, memory://)".to_string());
        }
        
        // Check for whitespace and control characters
        if uri.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err("URI must not contain whitespace or control characters".to_string());
        }
        
        // For file URIs, require absolute paths
        if uri.starts_with("file://") {
            let path_part = &uri[7..]; // Skip "file://"
            if !path_part.starts_with('/') {
                return Err("file:// URIs must use absolute paths".to_string());
            }
        }
        
        Ok(())
    }
    
    /// Validate URI template pattern (basic validation for template syntax)
    fn validate_uri_template(&self, template: &str) -> std::result::Result<(), String> {
        // First validate the base URI structure (ignoring template variables)
        let base_uri = template.replace(|c| c == '{' || c == '}', "");
        self.validate_uri(&base_uri)?;
        
        // Check template variable syntax is balanced
        let open_braces = template.matches('{').count();
        let close_braces = template.matches('}').count();
        if open_braces != close_braces {
            return Err("URI template has unbalanced braces".to_string());
        }
        
        Ok(())
    }

    // =============================================================================
    // ZERO-CONFIGURATION CONVENIENCE METHODS
    // These aliases make the API more intuitive and match the zero-config vision
    // =============================================================================

    /// Register a sampler - convenient alias for sampling_provider
    /// Automatically uses "sampling/createMessage" method
    pub fn sampler<S: McpSampling + 'static>(self, sampling: S) -> Self {
        self.sampling_provider(sampling)
    }

    /// Register a completer - convenient alias for completion_provider  
    /// Automatically uses "completion/complete" method
    pub fn completer<C: McpCompletion + 'static>(self, completion: C) -> Self {
        self.completion_provider(completion)
    }

    /// Register a notification by type - type determines method automatically
    /// This enables the `.notification::<T>()` pattern from universal-turul-mcp-server
    pub fn notification_type<N: McpNotification + 'static + Default>(self) -> Self {
        let notification = N::default();
        self.notification_provider(notification)  
    }

    /// Register a handler with the server
    pub fn handler<H: McpHandler + 'static>(mut self, handler: H) -> Self {
        let handler_arc = Arc::new(handler);
        for method in handler_arc.supported_methods() {
            self.handlers.insert(method, handler_arc.clone());
        }
        self
    }

    /// Register multiple handlers
    pub fn handlers<H: McpHandler + 'static, I: IntoIterator<Item = H>>(mut self, handlers: I) -> Self {
        for handler in handlers {
            self = self.handler(handler);
        }
        self
    }

    /// Add completion support
    pub fn with_completion(mut self) -> Self {
        self.capabilities.completions = Some(CompletionsCapabilities {
            enabled: Some(true),
        });
        self.handler(CompletionHandler)
    }

    /// Add prompts support
    pub fn with_prompts(mut self) -> Self {
        self.capabilities.prompts = Some(PromptsCapabilities {
            list_changed: Some(false),
        });
        
        // Prompts handlers are automatically registered when prompts are added via .prompt()
        // This method now just enables the capability
        self
    }

    /// Add resources support
    pub fn with_resources(mut self) -> Self {
        // Enable notifications if we have resources
        let has_resources = !self.resources.is_empty() || !self.template_resources.is_empty();
        
        self.capabilities.resources = Some(ResourcesCapabilities {
            subscribe: Some(false), // TODO: Implement resource subscriptions
            list_changed: Some(has_resources),
        });
        
        // Create ResourcesListHandler and add all registered resources
        let mut list_handler = ResourcesListHandler::new();
        for (_, resource) in &self.resources {
            list_handler = list_handler.add_resource_arc(resource.clone());
        }
        
        // Add template resources to list handler (using pattern as name)
        for (_template, resource) in &self.template_resources {
            list_handler = list_handler.add_resource_arc(resource.clone());
        }
        
        // Create ResourcesReadHandler and add all registered resources
        let mut read_handler = ResourcesReadHandler::new();
        for (_, resource) in &self.resources {
            read_handler = read_handler.add_resource_arc(resource.clone());
        }
        
        // Add template resources to read handler with template support
        for (template, resource) in &self.template_resources {
            read_handler = read_handler.add_template_resource_arc(template.clone(), resource.clone());
        }
        
        // Update notification manager
        
        // Register both handlers
        self.handler(list_handler)
            .handler(read_handler)
    }

    /// Add logging support
    pub fn with_logging(mut self) -> Self {
        self.capabilities.logging = Some(LoggingCapabilities::default());
        self.handler(LoggingHandler)
    }

    /// Add roots support
    pub fn with_roots(self) -> Self {
        // Note: roots is not part of standard server capabilities yet
        // Could be added to experimental if needed
        self.handler(RootsHandler::new())
    }

    /// Add a single root directory
    pub fn root(mut self, root: turul_mcp_protocol::roots::Root) -> Self {
        self.roots.push(root);
        self
    }

    /// Add sampling support
    pub fn with_sampling(self) -> Self {
        self.handler(SamplingHandler)
    }

    /// Add elicitation support with default mock provider
    pub fn with_elicitation(mut self) -> Self {
        self.capabilities.elicitation = Some(ElicitationCapabilities {
            enabled: Some(true),
        });
        self.handler(ElicitationHandler::with_mock_provider())
    }
    
    /// Add elicitation support with custom provider
    pub fn with_elicitation_provider<P: ElicitationProvider + 'static>(mut self, provider: P) -> Self {
        self.capabilities.elicitation = Some(ElicitationCapabilities {
            enabled: Some(true),
        });
        self.handler(ElicitationHandler::new(Arc::new(provider)))
    }


    /// Add notifications support
    pub fn with_notifications(self) -> Self {
        self.handler(NotificationsHandler)
    }

    /// Configure session timeout (in minutes, default: 30)
    pub fn session_timeout_minutes(mut self, minutes: u64) -> Self {
        self.session_timeout_minutes = Some(minutes);
        self
    }
    
    /// Configure session cleanup interval (in seconds, default: 60)
    pub fn session_cleanup_interval_seconds(mut self, seconds: u64) -> Self {
        self.session_cleanup_interval_seconds = Some(seconds);
        self
    }

    /// Enable strict MCP lifecycle enforcement
    /// 
    /// When enabled, the server will reject all operations (tools, resources, etc.) 
    /// until the client sends `notifications/initialized` after receiving the 
    /// initialize response.
    /// 
    /// **Default: false (lenient mode)** - for compatibility with existing clients
    /// **Production: consider true** - for strict MCP spec compliance
    /// 
    /// # Example
    /// ```rust,no_run
    /// use turul_mcp_server::McpServer;
    /// 
    /// let server = McpServer::builder()
    ///     .name("strict-server")
    ///     .version("1.0.0")
    ///     .strict_lifecycle(true)  // Enable strict enforcement
    ///     .build()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn strict_lifecycle(mut self, strict: bool) -> Self {
        self.strict_lifecycle = strict;
        self
    }

    /// Enable strict MCP lifecycle enforcement (convenience method)
    /// 
    /// Equivalent to `.strict_lifecycle(true)`. Enables strict enforcement where
    /// all operations are rejected until `notifications/initialized` is received.
    pub fn with_strict_lifecycle(self) -> Self {
        self.strict_lifecycle(true)
    }
    
    /// Configure sessions with recommended defaults for long-running sessions
    pub fn with_long_sessions(mut self) -> Self {
        self.session_timeout_minutes = Some(120); // 2 hours
        self.session_cleanup_interval_seconds = Some(300); // 5 minutes
        self
    }
    
    /// Configure sessions with recommended defaults for short-lived sessions
    pub fn with_short_sessions(mut self) -> Self {
        self.session_timeout_minutes = Some(15); // 15 minutes
        self.session_cleanup_interval_seconds = Some(30); // 30 seconds
        self
    }

    /// Configure session storage backend (defaults to InMemory if not specified)
    pub fn with_session_storage<S: turul_mcp_session_storage::SessionStorage<Error = turul_mcp_session_storage::SessionStorageError> + 'static>(
        mut self, 
        storage: Arc<S>
    ) -> Self {
        // Convert concrete storage type to trait object
        let boxed_storage: Arc<turul_mcp_session_storage::BoxedSessionStorage> = storage;
        self.session_storage = Some(boxed_storage);
        self
    }

    /// Set HTTP bind address (requires "http" feature)
    #[cfg(feature = "http")]
    pub fn bind_address(mut self, addr: SocketAddr) -> Self {
        self.bind_address = addr;
        self
    }

    /// Set MCP endpoint path (requires "http" feature)
    #[cfg(feature = "http")]
    pub fn mcp_path(mut self, path: impl Into<String>) -> Self {
        self.mcp_path = path.into();
        self
    }

    /// Enable/disable CORS (requires "http" feature)
    #[cfg(feature = "http")]
    pub fn cors(mut self, enable: bool) -> Self {
        self.enable_cors = enable;
        self
    }

    /// Enable/disable SSE (requires "sse" feature)
    #[cfg(feature = "http")]
    pub fn sse(mut self, enable: bool) -> Self {
        self.enable_sse = enable;
        self
    }



    /// Build the MCP server
    pub fn build(mut self) -> Result<McpServer> {
        // Validate configuration
        if self.name.is_empty() {
            return Err(McpFrameworkError::Config("Server name cannot be empty".to_string()));
        }
        if self.version.is_empty() {
            return Err(McpFrameworkError::Config("Server version cannot be empty".to_string()));
        }
        
        // Check for validation errors collected during registration
        if !self.validation_errors.is_empty() {
            return Err(McpFrameworkError::Config(
                format!("Resource validation errors:\n{}", self.validation_errors.join("\n"))
            ));
        }

        // Auto-detect and configure server capabilities based on registered components
        let has_resources = !self.resources.is_empty() || !self.template_resources.is_empty();
        let has_tools = !self.tools.is_empty();
        let has_prompts = !self.prompts.is_empty();
        let has_roots = !self.roots.is_empty();
        let has_elicitations = !self.elicitations.is_empty();
        let has_completions = !self.completions.is_empty();
        let _has_samplings = !self.sampling.is_empty();
        let has_logging = !self.loggers.is_empty();

        // Tools capabilities - support notifications only if tools are registered AND we have dynamic change sources
        // Note: In current static framework, tool set is fixed at build time and doesn't change
        // list_changed should only be true when dynamic change sources are wired, such as:
        // - Hot-reload configuration systems
        // - Admin APIs for runtime tool registration
        // - Plugin systems with dynamic tool loading
        if has_tools {
            self.capabilities.tools = Some(ToolsCapabilities {
                // Static framework: no dynamic change sources = no list changes
                list_changed: Some(false),
            });
        }

        // Prompts capabilities - support notifications only when dynamic change sources are wired
        // Note: In current static framework, prompt set is fixed at build time and doesn't change
        // list_changed should only be true when dynamic change sources are wired, such as:
        // - Hot-reload configuration systems
        // - Admin APIs for runtime prompt registration
        // - Plugin systems with dynamic prompt loading
        if has_prompts {
            self.capabilities.prompts = Some(PromptsCapabilities {
                // Static framework: no dynamic change sources = no list changes
                list_changed: Some(false),
            });
        }

        // Resources capabilities - support notifications only when dynamic change sources are wired
        // Note: In current static framework, resource set is fixed at build time and doesn't change
        // list_changed should only be true when dynamic change sources are wired, such as:
        // - Hot-reload configuration systems
        // - Admin APIs for runtime resource registration
        // - File system watchers that update resource availability
        if has_resources {
            self.capabilities.resources = Some(ResourcesCapabilities {
                subscribe: Some(false), // TODO: Implement resource subscriptions in Phase 5
                // Static framework: no dynamic change sources = no list changes
                list_changed: Some(false),
            });
        }

        // Elicitation capabilities - enable if elicitation handlers are registered
        if has_elicitations {
            self.capabilities.elicitation = Some(ElicitationCapabilities {
                enabled: Some(true),
            });
        }

        // Completion capabilities - enable if completion handlers are registered
        if has_completions {
            self.capabilities.completions = Some(CompletionsCapabilities {
                enabled: Some(true),
            });
        }

        // Logging capabilities - always enabled with comprehensive level support
        if has_logging || true { // Always enable logging for debugging/monitoring
            self.capabilities.logging = Some(LoggingCapabilities {
                enabled: Some(true),
                levels: Some(vec![
                    "debug".to_string(),
                    "info".to_string(), 
                    "warning".to_string(),
                    "error".to_string()
                ]),
            });
        }

        info!("🔧 Auto-configured server capabilities:");
        info!("   - Tools: {}", has_tools);
        info!("   - Resources: {}", has_resources);  
        info!("   - Prompts: {}", has_prompts);
        info!("   - Roots: {}", has_roots);
        info!("   - Elicitation: {}", has_elicitations);
        info!("   - Completions: {}", has_completions);
        info!("   - Logging: enabled");

        // Create implementation info
        let mut implementation = Implementation::new(&self.name, &self.version);
        if let Some(title) = self.title {
            implementation = implementation.with_title(title);
        }

        // Add RootsHandler if roots were configured
        let mut handlers = self.handlers;
        if !self.roots.is_empty() {
            let mut roots_handler = RootsHandler::new();
            for root in self.roots {
                roots_handler = roots_handler.add_root(root);
            }
            handlers.insert("roots/list".to_string(), Arc::new(roots_handler));
        }

        // Add PromptsHandlers if prompts were configured
        if !self.prompts.is_empty() {
            let mut prompts_list_handler = PromptsListHandler::new();
            let mut prompts_get_handler = PromptsGetHandler::new();
            
            for prompt in self.prompts.values() {
                prompts_list_handler = prompts_list_handler.add_prompt_arc(prompt.clone());
                prompts_get_handler = prompts_get_handler.add_prompt_arc(prompt.clone());
            }
            
            handlers.insert("prompts/list".to_string(), Arc::new(prompts_list_handler));
            handlers.insert("prompts/get".to_string(), Arc::new(prompts_get_handler));
        }

        // Add ResourceTemplatesHandler if template resources were configured
        if !self.template_resources.is_empty() {
            let resource_templates_handler = ResourceTemplatesHandler::new()
                .with_templates(self.template_resources.clone());
            handlers.insert("resources/templates/list".to_string(), Arc::new(resource_templates_handler));
        }

        // Create server
        Ok(McpServer::new(
            implementation,
            self.capabilities,
            self.tools,
            handlers,
            self.instructions,
            self.session_timeout_minutes,
            self.session_cleanup_interval_seconds,
            self.session_storage,
            self.strict_lifecycle,
            #[cfg(feature = "http")]
            self.bind_address,
            #[cfg(feature = "http")]
            self.mcp_path,
            #[cfg(feature = "http")]
            self.enable_cors,
            #[cfg(feature = "http")]
            self.enable_sse,
        ))
    }
}

impl Default for McpServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{McpTool, SessionContext};
    use async_trait::async_trait;
    use turul_mcp_protocol::{ToolSchema, CallToolResult};
    use turul_mcp_protocol::tools::{HasBaseMetadata, HasDescription, HasInputSchema, HasOutputSchema, HasAnnotations, HasToolMeta, ToolAnnotations};
    use serde_json::Value;
    use std::collections::HashMap;

    struct TestTool {
        input_schema: ToolSchema,
    }

    impl TestTool {
        fn new() -> Self {
            Self {
                input_schema: ToolSchema::object(),
            }
        }
    }

    // Implement fine-grained traits
    impl HasBaseMetadata for TestTool {
        fn name(&self) -> &str { "test" }
        fn title(&self) -> Option<&str> { None }
    }

    impl HasDescription for TestTool {
        fn description(&self) -> Option<&str> { Some("Test tool") }
    }

    impl HasInputSchema for TestTool {
        fn input_schema(&self) -> &ToolSchema { &self.input_schema }
    }

    impl HasOutputSchema for TestTool {
        fn output_schema(&self) -> Option<&ToolSchema> { None }
    }

    impl HasAnnotations for TestTool {
        fn annotations(&self) -> Option<&ToolAnnotations> { None }
    }

    impl HasToolMeta for TestTool {
        fn tool_meta(&self) -> Option<&HashMap<String, Value>> { None }
    }

    #[async_trait]
    impl McpTool for TestTool {
        async fn call(&self, _args: Value, _session: Option<SessionContext>) -> crate::McpResult<CallToolResult> {
            Ok(CallToolResult::success(vec![turul_mcp_protocol::ToolResult::text("test")]))
        }
    }

    #[test]
    fn test_builder_defaults() {
        let builder = McpServerBuilder::new();
        assert_eq!(builder.name, "turul-mcp-server");
        assert_eq!(builder.version, "1.0.0");
        assert!(builder.title.is_none());
        assert!(builder.instructions.is_none());
        assert!(builder.tools.is_empty());
        assert_eq!(builder.handlers.len(), 17); // Default MCP 2025-06-18 handlers
        assert!(builder.handlers.contains_key("ping"));
    }

    #[test]
    fn test_builder_configuration() {
        let builder = McpServerBuilder::new()
            .name("test-server")
            .version("2.0.0")
            .title("Test Server")
            .instructions("This is a test server");

        assert_eq!(builder.name, "test-server");
        assert_eq!(builder.version, "2.0.0");
        assert_eq!(builder.title, Some("Test Server".to_string()));
        assert_eq!(builder.instructions, Some("This is a test server".to_string()));
    }

    

    #[test]
    fn test_builder_build() {
        let server = McpServerBuilder::new()
            .name("test-server")
            .version("1.0.0")
            .tool(TestTool::new())
            .build()
            .unwrap();

        assert_eq!(server.implementation.name, "test-server");
        assert_eq!(server.implementation.version, "1.0.0");
    }

    #[test]
    fn test_builder_validation() {
        let result = McpServerBuilder::new()
            .name("")
            .build();

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), McpFrameworkError::Config(_)));
    }
}