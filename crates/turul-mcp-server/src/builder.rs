//! MCP Server Builder
//!
//! This module provides a builder pattern for creating MCP servers.

use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;

use crate::handlers::*;
use crate::resource::McpResource;
use crate::{
    McpCompletion, McpElicitation, McpLogger, McpNotification, McpPrompt, McpRoot, McpSampling,
};
use crate::{McpServer, McpTool, Result};
use turul_mcp_protocol::McpError;
use turul_mcp_protocol::initialize::*;
use turul_mcp_protocol::{Implementation, ServerCapabilities};

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

    /// Task storage / runtime (None = tasks not supported)
    task_runtime: Option<Arc<crate::task_runtime::TaskRuntime>>,

    /// Recovery timeout for stuck tasks (milliseconds), default 5 minutes
    task_recovery_timeout_ms: u64,

    /// MCP Lifecycle enforcement configuration
    strict_lifecycle: bool,

    /// Test mode - disables security middleware for test servers
    test_mode: bool,

    /// Middleware stack for request/response interception
    middleware_stack: crate::middleware::MiddlewareStack,

    /// HTTP configuration (if enabled)
    #[cfg(feature = "http")]
    bind_address: SocketAddr,
    #[cfg(feature = "http")]
    mcp_path: String,
    #[cfg(feature = "http")]
    enable_cors: bool,
    #[cfg(feature = "http")]
    enable_sse: bool,
    #[cfg(feature = "http")]
    allow_unauthenticated_ping: Option<bool>,

    /// Validation errors collected during builder configuration
    validation_errors: Vec<String>,
}

impl McpServerBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        let tools = HashMap::new();
        let mut handlers: HashMap<String, Arc<dyn McpHandler>> = HashMap::new();

        // Add all standard MCP 2025-11-25 handlers by default
        handlers.insert("ping".to_string(), Arc::new(PingHandler));
        handlers.insert(
            "completion/complete".to_string(),
            Arc::new(CompletionHandler),
        );
        handlers.insert(
            "resources/list".to_string(),
            Arc::new(ResourcesListHandler::new()),
        );
        handlers.insert(
            "resources/read".to_string(),
            Arc::new(ResourcesReadHandler::new()),
        );
        handlers.insert(
            "prompts/list".to_string(),
            Arc::new(PromptsListHandler::new()),
        );
        handlers.insert(
            "prompts/get".to_string(),
            Arc::new(PromptsGetHandler::new()),
        );
        handlers.insert("logging/setLevel".to_string(), Arc::new(LoggingHandler));
        handlers.insert("roots/list".to_string(), Arc::new(RootsHandler::new()));
        handlers.insert(
            "sampling/createMessage".to_string(),
            Arc::new(SamplingHandler),
        );
        // Note: resources/templates/list is only registered if template resources are configured (see build method)
        handlers.insert(
            "elicitation/create".to_string(),
            Arc::new(ElicitationHandler::with_mock_provider()),
        );

        // Add all notification handlers (except notifications/initialized which is handled specially)
        let notifications_handler = Arc::new(NotificationsHandler);
        handlers.insert(
            "notifications/message".to_string(),
            notifications_handler.clone(),
        );
        handlers.insert(
            "notifications/progress".to_string(),
            notifications_handler.clone(),
        );
        // MCP 2025-11-25 spec-correct underscore form
        handlers.insert(
            "notifications/resources/list_changed".to_string(),
            notifications_handler.clone(),
        );
        handlers.insert(
            "notifications/resources/updated".to_string(),
            notifications_handler.clone(),
        );
        handlers.insert(
            "notifications/tools/list_changed".to_string(),
            notifications_handler.clone(),
        );
        handlers.insert(
            "notifications/prompts/list_changed".to_string(),
            notifications_handler.clone(),
        );
        handlers.insert(
            "notifications/roots/list_changed".to_string(),
            notifications_handler.clone(),
        );
        // Legacy compat: accept camelCase from older clients
        handlers.insert(
            "notifications/resources/listChanged".to_string(),
            notifications_handler.clone(),
        );
        handlers.insert(
            "notifications/tools/listChanged".to_string(),
            notifications_handler.clone(),
        );
        handlers.insert(
            "notifications/prompts/listChanged".to_string(),
            notifications_handler.clone(),
        );
        handlers.insert(
            "notifications/roots/listChanged".to_string(),
            notifications_handler,
        );

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
            session_storage: None,             // Default: InMemory storage
            task_runtime: None,                // Default: tasks not supported
            task_recovery_timeout_ms: 300_000, // Default: 5 minutes
            strict_lifecycle: false,           // Default: lenient mode for compatibility
            test_mode: false,                  // Default: production mode with security
            middleware_stack: crate::middleware::MiddlewareStack::new(), // Default: empty middleware stack
            #[cfg(feature = "http")]
            bind_address: "127.0.0.1:8000".parse().unwrap(),
            #[cfg(feature = "http")]
            mcp_path: "/mcp".to_string(),
            #[cfg(feature = "http")]
            enable_cors: true,
            #[cfg(feature = "http")]
            enable_sse: cfg!(feature = "sse"),
            #[cfg(feature = "http")]
            allow_unauthenticated_ping: None, // Default: use ServerConfig default (true)
            validation_errors: Vec::new(),
        }
    }

    /// Sets the server name for identification
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Sets the server version string
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Sets the human-readable server title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets usage instructions for MCP clients
    pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    /// Registers a tool that clients can execute
    pub fn tool<T: McpTool + 'static>(mut self, tool: T) -> Self {
        let name = tool.name().to_string();
        self.tools.insert(name, Arc::new(tool));
        self
    }

    /// Register a function tool created with `#[mcp_tool]` macro
    ///
    /// This method provides a more intuitive way to register function tools.
    /// The `#[mcp_tool]` macro generates a constructor function with the same name
    /// as your async function, so you can use the function name directly.
    ///
    /// # Example
    /// ```rust,no_run
    /// use turul_mcp_server::prelude::*;
    /// use std::collections::HashMap;
    ///
    /// // Manual tool implementation without derive macros
    /// #[derive(Clone, Default)]
    /// struct AddTool;
    ///
    /// // Implement all required traits for ToolDefinition
    /// impl turul_mcp_builders::traits::HasBaseMetadata for AddTool {
    ///     fn name(&self) -> &str { "add" }
    ///     fn title(&self) -> Option<&str> { Some("Add Numbers") }
    /// }
    ///
    /// impl turul_mcp_builders::traits::HasDescription for AddTool {
    ///     fn description(&self) -> Option<&str> {
    ///         Some("Add two numbers together")
    ///     }
    /// }
    ///
    /// impl turul_mcp_builders::traits::HasInputSchema for AddTool {
    ///     fn input_schema(&self) -> &turul_mcp_protocol::ToolSchema {
    ///         use turul_mcp_protocol::schema::JsonSchema;
    ///         static SCHEMA: std::sync::OnceLock<turul_mcp_protocol::ToolSchema> = std::sync::OnceLock::new();
    ///         SCHEMA.get_or_init(|| {
    ///             let mut props = HashMap::new();
    ///             props.insert("a".to_string(), JsonSchema::number().with_description("First number"));
    ///             props.insert("b".to_string(), JsonSchema::number().with_description("Second number"));
    ///             turul_mcp_protocol::ToolSchema::object()
    ///                 .with_properties(props)
    ///                 .with_required(vec!["a".to_string(), "b".to_string()])
    ///         })
    ///     }
    /// }
    ///
    /// impl turul_mcp_builders::traits::HasOutputSchema for AddTool {
    ///     fn output_schema(&self) -> Option<&turul_mcp_protocol::ToolSchema> { None }
    /// }
    ///
    /// impl turul_mcp_builders::traits::HasAnnotations for AddTool {
    ///     fn annotations(&self) -> Option<&turul_mcp_protocol::tools::ToolAnnotations> { None }
    /// }
    ///
    /// impl turul_mcp_builders::traits::HasToolMeta for AddTool {
    ///     fn tool_meta(&self) -> Option<&HashMap<String, serde_json::Value>> { None }
    /// }
    ///
    /// impl turul_mcp_builders::traits::HasIcons for AddTool {}
    ///
    /// #[async_trait]
    /// impl McpTool for AddTool {
    ///     async fn call(&self, args: serde_json::Value, _session: Option<SessionContext>)
    ///         -> McpResult<turul_mcp_protocol::tools::CallToolResult> {
    ///         let a = args.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
    ///         let b = args.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
    ///         let result = a + b;
    ///
    ///         Ok(turul_mcp_protocol::tools::CallToolResult::success(vec![
    ///             turul_mcp_protocol::ToolResult::text(format!("{} + {} = {}", a, b, result))
    ///         ]))
    ///     }
    /// }
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let server = McpServer::builder()
    ///     .name("math-server")
    ///     .tool_fn(|| AddTool::default()) // Function returns working tool instance
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn tool_fn<F, T>(self, func: F) -> Self
    where
        F: Fn() -> T,
        T: McpTool + 'static,
    {
        // Call the helper function to get the tool instance
        self.tool(func())
    }

    /// Registers multiple tools in a batch
    pub fn tools<T: McpTool + 'static, I: IntoIterator<Item = T>>(mut self, tools: I) -> Self {
        for tool in tools {
            self = self.tool(tool);
        }
        self
    }

    /// Add middleware to the request/response processing chain
    ///
    /// **This method is additive** - each call adds a new middleware to the stack.
    /// Middleware execute in the order they are registered (FIFO):
    /// - **Before dispatch**: First registered executes first (FIFO order)
    /// - **After dispatch**: First registered executes last (LIFO/reverse order)
    ///
    /// Multiple middleware can be composed by calling this method multiple times.
    /// Middleware works identically across all transports (HTTP, Lambda, etc.).
    ///
    /// # Behavior with Other Builder Methods
    ///
    /// - **`.test_mode()`**: Does NOT affect middleware - middleware always executes
    /// - **Non-HTTP builds**: Middleware is available but requires manual wiring
    ///
    /// # Examples
    ///
    /// ## Single Middleware
    ///
    /// ```rust,no_run
    /// use turul_mcp_server::prelude::*;
    /// use async_trait::async_trait;
    /// use std::sync::Arc;
    ///
    /// struct LoggingMiddleware;
    ///
    /// #[async_trait]
    /// impl McpMiddleware for LoggingMiddleware {
    ///     async fn before_dispatch(
    ///         &self,
    ///         ctx: &mut RequestContext<'_>,
    ///         _session: Option<&dyn turul_mcp_session_storage::SessionView>,
    ///         _injection: &mut SessionInjection,
    ///     ) -> Result<(), MiddlewareError> {
    ///         println!("Request: {}", ctx.method());
    ///         Ok(())
    ///     }
    /// }
    ///
    /// # async fn example() -> McpResult<()> {
    /// let server = McpServer::builder()
    ///     .name("my-server")
    ///     .middleware(Arc::new(LoggingMiddleware))
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Multiple Middleware Composition
    ///
    /// ```rust,no_run
    /// use turul_mcp_server::prelude::*;
    /// use async_trait::async_trait;
    /// use std::sync::Arc;
    /// use serde_json::json;
    /// # struct AuthMiddleware;
    /// # struct LoggingMiddleware;
    /// # struct RateLimitMiddleware;
    /// # #[async_trait]
    /// # impl McpMiddleware for AuthMiddleware {
    /// #     async fn before_dispatch(&self, _ctx: &mut RequestContext<'_>, _session: Option<&dyn turul_mcp_session_storage::SessionView>, _injection: &mut SessionInjection) -> Result<(), MiddlewareError> { Ok(()) }
    /// # }
    /// # #[async_trait]
    /// # impl McpMiddleware for LoggingMiddleware {
    /// #     async fn before_dispatch(&self, _ctx: &mut RequestContext<'_>, _session: Option<&dyn turul_mcp_session_storage::SessionView>, _injection: &mut SessionInjection) -> Result<(), MiddlewareError> { Ok(()) }
    /// # }
    /// # #[async_trait]
    /// # impl McpMiddleware for RateLimitMiddleware {
    /// #     async fn before_dispatch(&self, _ctx: &mut RequestContext<'_>, _session: Option<&dyn turul_mcp_session_storage::SessionView>, _injection: &mut SessionInjection) -> Result<(), MiddlewareError> { Ok(()) }
    /// # }
    ///
    /// # async fn example() -> McpResult<()> {
    /// // Execution order:
    /// // Before dispatch: Auth → Logging → RateLimit
    /// // After dispatch: RateLimit → Logging → Auth (reverse)
    /// let server = McpServer::builder()
    ///     .name("my-server")
    ///     .middleware(Arc::new(AuthMiddleware))      // 1st before, 3rd after
    ///     .middleware(Arc::new(LoggingMiddleware))   // 2nd before, 2nd after
    ///     .middleware(Arc::new(RateLimitMiddleware)) // 3rd before, 1st after
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn middleware(mut self, middleware: Arc<dyn crate::middleware::McpMiddleware>) -> Self {
        self.middleware_stack.push(middleware);
        self
    }

    /// Register a resource with the server
    ///
    /// Automatically detects if the resource URI contains template variables (e.g., `{ticker}`, `{id}`)
    /// and registers it as either a static resource or template resource accordingly.
    /// This eliminates the need to manually call `.template_resource()` for templated URIs.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use turul_mcp_server::prelude::*;
    /// use std::collections::HashMap;
    ///
    /// // Manual resource implementation without derive macros
    /// #[derive(Clone)]
    /// struct ConfigResource {
    ///     data: String,
    /// }
    ///
    /// // Implement all required traits for ResourceDefinition
    /// impl turul_mcp_builders::traits::HasResourceMetadata for ConfigResource {
    ///     fn name(&self) -> &str { "config" }
    ///     fn title(&self) -> Option<&str> { Some("Configuration") }
    /// }
    ///
    /// impl turul_mcp_builders::traits::HasResourceDescription for ConfigResource {
    ///     fn description(&self) -> Option<&str> {
    ///         Some("Application configuration file")
    ///     }
    /// }
    ///
    /// impl turul_mcp_builders::traits::HasResourceUri for ConfigResource {
    ///     fn uri(&self) -> &str { "file:///config.json" }
    /// }
    ///
    /// impl turul_mcp_builders::traits::HasResourceMimeType for ConfigResource {
    ///     fn mime_type(&self) -> Option<&str> { Some("application/json") }
    /// }
    ///
    /// impl turul_mcp_builders::traits::HasResourceSize for ConfigResource {
    ///     fn size(&self) -> Option<u64> { Some(self.data.len() as u64) }
    /// }
    ///
    /// impl turul_mcp_builders::traits::HasResourceAnnotations for ConfigResource {
    ///     fn annotations(&self) -> Option<&turul_mcp_protocol::meta::Annotations> { None }
    /// }
    ///
    /// impl turul_mcp_builders::traits::HasResourceMeta for ConfigResource {
    ///     fn resource_meta(&self) -> Option<&HashMap<String, serde_json::Value>> { None }
    /// }
    ///
    /// impl turul_mcp_builders::traits::HasIcons for ConfigResource {}
    ///
    /// #[async_trait]
    /// impl McpResource for ConfigResource {
    ///     async fn read(&self, _params: Option<serde_json::Value>, _session: Option<&SessionContext>)
    ///         -> McpResult<Vec<turul_mcp_protocol::ResourceContent>> {
    ///         Ok(vec![turul_mcp_protocol::ResourceContent::text(
    ///             self.uri(),
    ///             &self.data
    ///         )])
    ///     }
    /// }
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ConfigResource {
    ///     data: r#"{"debug": true, "port": 8080}"#.to_string(),
    /// };
    ///
    /// let server = McpServer::builder()
    ///     .name("resource-server")
    ///     .resource(config) // Working resource with actual data
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn resource<R: McpResource + 'static>(mut self, resource: R) -> Self {
        let uri = resource.uri().to_string();

        // Auto-detect if URI contains template variables
        if self.is_template_uri(&uri) {
            // It's a template resource - parse as UriTemplate
            tracing::debug!("Detected template resource: {}", uri);
            match crate::uri_template::UriTemplate::new(&uri) {
                Ok(template) => {
                    // Validate template pattern
                    if let Err(e) = self.validate_uri_template(template.pattern()) {
                        self.validation_errors
                            .push(format!("Invalid template resource URI '{}': {}", uri, e));
                    }
                    self.template_resources.push((template, Arc::new(resource)));
                }
                Err(e) => {
                    self.validation_errors.push(format!(
                        "Failed to parse template resource URI '{}': {}",
                        uri, e
                    ));
                }
            }
        } else {
            // It's a static resource
            tracing::debug!("Detected static resource: {}", uri);
            if let Err(e) = self.validate_uri(&uri) {
                tracing::warn!("Static resource validation failed for '{}': {}", uri, e);
                self.validation_errors
                    .push(format!("Invalid resource URI '{}': {}", uri, e));
            } else {
                tracing::debug!("Successfully added static resource: {}", uri);
                self.resources.insert(uri, Arc::new(resource));
            }
        }

        self
    }

    /// Register a function resource created with `#[mcp_resource]` macro
    ///
    /// This method provides a more intuitive way to register function resources.
    /// The `#[mcp_resource]` macro generates a constructor function with the same name
    /// as your async function, so you can use the function name directly.
    ///
    /// # Example
    /// ```rust,no_run
    /// use turul_mcp_server::prelude::*;
    /// use std::collections::HashMap;
    ///
    /// // Manual resource implementation without derive macros
    /// #[derive(Clone)]
    /// struct DataResource {
    ///     content: String,
    /// }
    ///
    /// // Implement all required traits for ResourceDefinition (same as resource() example)
    /// impl turul_mcp_builders::traits::HasResourceMetadata for DataResource {
    ///     fn name(&self) -> &str { "data" }
    ///     fn title(&self) -> Option<&str> { Some("Data File") }
    /// }
    ///
    /// impl turul_mcp_builders::traits::HasResourceDescription for DataResource {
    ///     fn description(&self) -> Option<&str> { Some("Sample data file") }
    /// }
    ///
    /// impl turul_mcp_builders::traits::HasResourceUri for DataResource {
    ///     fn uri(&self) -> &str { "file:///data/sample.json" }
    /// }
    ///
    /// impl turul_mcp_builders::traits::HasResourceMimeType for DataResource {
    ///     fn mime_type(&self) -> Option<&str> { Some("application/json") }
    /// }
    ///
    /// impl turul_mcp_builders::traits::HasResourceSize for DataResource {
    ///     fn size(&self) -> Option<u64> { Some(self.content.len() as u64) }
    /// }
    ///
    /// impl turul_mcp_builders::traits::HasResourceAnnotations for DataResource {
    ///     fn annotations(&self) -> Option<&turul_mcp_protocol::meta::Annotations> { None }
    /// }
    ///
    /// impl turul_mcp_builders::traits::HasResourceMeta for DataResource {
    ///     fn resource_meta(&self) -> Option<&HashMap<String, serde_json::Value>> { None }
    /// }
    ///
    /// impl turul_mcp_builders::traits::HasIcons for DataResource {}
    ///
    /// #[async_trait]
    /// impl McpResource for DataResource {
    ///     async fn read(&self, _params: Option<serde_json::Value>, _session: Option<&SessionContext>)
    ///         -> McpResult<Vec<turul_mcp_protocol::ResourceContent>> {
    ///         Ok(vec![turul_mcp_protocol::ResourceContent::text(
    ///             self.uri(),
    ///             &self.content
    ///         )])
    ///     }
    /// }
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let server = McpServer::builder()
    ///     .name("data-server")
    ///     .resource_fn(|| DataResource {
    ///         content: r#"{"items": [1, 2, 3]}"#.to_string()
    ///     }) // Function returns working resource instance
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn resource_fn<F, R>(self, func: F) -> Self
    where
        F: Fn() -> R,
        R: McpResource + 'static,
    {
        // Call the helper function to get the resource instance
        self.resource(func())
    }

    /// Register multiple resources
    pub fn resources<R: McpResource + 'static, I: IntoIterator<Item = R>>(
        mut self,
        resources: I,
    ) -> Self {
        for resource in resources {
            self = self.resource(resource);
        }
        self
    }

    /// Register a resource with explicit URI template support
    ///
    /// **Note**: This method is now optional. The `.resource()` method automatically detects
    /// template URIs and handles them appropriately. Use this method only when you need
    /// explicit control over template parsing or want to add custom validators.
    ///
    /// # Example
    /// ```rust,no_run
    /// use turul_mcp_server::prelude::*;
    /// use turul_mcp_server::uri_template::{UriTemplate, VariableValidator};
    /// use std::collections::HashMap;
    ///
    /// // Manual template resource implementation
    /// #[derive(Clone)]
    /// struct TemplateResource {
    ///     base_path: String,
    /// }
    ///
    /// // Implement all required traits for ResourceDefinition
    /// impl turul_mcp_builders::traits::HasResourceMetadata for TemplateResource {
    ///     fn name(&self) -> &str { "template-data" }
    ///     fn title(&self) -> Option<&str> { Some("Template Data") }
    /// }
    ///
    /// impl turul_mcp_builders::traits::HasResourceDescription for TemplateResource {
    ///     fn description(&self) -> Option<&str> { Some("Template-based data resource") }
    /// }
    ///
    /// impl turul_mcp_builders::traits::HasResourceUri for TemplateResource {
    ///     fn uri(&self) -> &str { "file:///data/{id}.json" }
    /// }
    ///
    /// impl turul_mcp_builders::traits::HasResourceMimeType for TemplateResource {
    ///     fn mime_type(&self) -> Option<&str> { Some("application/json") }
    /// }
    ///
    /// impl turul_mcp_builders::traits::HasResourceSize for TemplateResource {
    ///     fn size(&self) -> Option<u64> { None } // Size varies by template
    /// }
    ///
    /// impl turul_mcp_builders::traits::HasResourceAnnotations for TemplateResource {
    ///     fn annotations(&self) -> Option<&turul_mcp_protocol::meta::Annotations> { None }
    /// }
    ///
    /// impl turul_mcp_builders::traits::HasResourceMeta for TemplateResource {
    ///     fn resource_meta(&self) -> Option<&HashMap<String, serde_json::Value>> { None }
    /// }
    ///
    /// impl turul_mcp_builders::traits::HasIcons for TemplateResource {}
    ///
    /// #[async_trait]
    /// impl McpResource for TemplateResource {
    ///     async fn read(&self, params: Option<serde_json::Value>, _session: Option<&SessionContext>)
    ///         -> McpResult<Vec<turul_mcp_protocol::ResourceContent>> {
    ///         let id = params
    ///             .as_ref()
    ///             .and_then(|p| p.get("id"))
    ///             .and_then(|v| v.as_str())
    ///             .unwrap_or("default");
    ///
    ///         let content = format!(r#"{{"id": "{}", "data": "sample content for {}"}}"#, id, id);
    ///         Ok(vec![turul_mcp_protocol::ResourceContent::text(
    ///             &format!("file:///data/{}.json", id),
    ///             &content
    ///         )])
    ///     }
    /// }
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let template = UriTemplate::new("file:///data/{id}.json")?
    ///     .with_validator("id", VariableValidator::user_id());
    ///
    /// let resource = TemplateResource {
    ///     base_path: "/data".to_string(),
    /// };
    ///
    /// let server = McpServer::builder()
    ///     .name("template-server")
    ///     .template_resource(template, resource) // Working template resource
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn template_resource<R: McpResource + 'static>(
        mut self,
        template: crate::uri_template::UriTemplate,
        resource: R,
    ) -> Self {
        // Validate template pattern is well-formed (MCP 2025-11-25 compliance)
        if let Err(e) = self.validate_uri_template(template.pattern()) {
            self.validation_errors.push(format!(
                "Invalid resource template URI '{}': {}",
                template.pattern(),
                e
            ));
        }

        self.template_resources.push((template, Arc::new(resource)));
        self
    }

    /// Registers a prompt template for conversation generation
    pub fn prompt<P: McpPrompt + 'static>(mut self, prompt: P) -> Self {
        let name = prompt.name().to_string();
        self.prompts.insert(name, Arc::new(prompt));
        self
    }

    /// Register multiple prompts
    pub fn prompts<P: McpPrompt + 'static, I: IntoIterator<Item = P>>(
        mut self,
        prompts: I,
    ) -> Self {
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
    pub fn elicitations<E: McpElicitation + 'static, I: IntoIterator<Item = E>>(
        mut self,
        elicitations: I,
    ) -> Self {
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
    pub fn sampling_providers<S: McpSampling + 'static, I: IntoIterator<Item = S>>(
        mut self,
        sampling: I,
    ) -> Self {
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
    pub fn completion_providers<C: McpCompletion + 'static, I: IntoIterator<Item = C>>(
        mut self,
        completions: I,
    ) -> Self {
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
    pub fn loggers<L: McpLogger + 'static, I: IntoIterator<Item = L>>(
        mut self,
        loggers: I,
    ) -> Self {
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
    pub fn root_providers<R: McpRoot + 'static, I: IntoIterator<Item = R>>(
        mut self,
        roots: I,
    ) -> Self {
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
    pub fn notification_providers<N: McpNotification + 'static, I: IntoIterator<Item = N>>(
        mut self,
        notifications: I,
    ) -> Self {
        for notification in notifications {
            self = self.notification_provider(notification);
        }
        self
    }

    /// Check if URI contains template variables (e.g., {ticker}, {id})
    fn is_template_uri(&self, uri: &str) -> bool {
        uri.contains('{') && uri.contains('}')
    }

    /// Validate URI is absolute and well-formed (reusing SecurityMiddleware logic)
    fn validate_uri(&self, uri: &str) -> std::result::Result<(), String> {
        // Check basic URI format - must have scheme
        if !uri.contains("://") {
            return Err(
                "URI must be absolute with scheme (e.g., file://, http://, memory://)".to_string(),
            );
        }

        // Check for whitespace and control characters
        if uri.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err("URI must not contain whitespace or control characters".to_string());
        }

        // For file URIs, require absolute paths
        if let Some(path_part) = uri.strip_prefix("file://") {
            // Skip "file://"
            if !path_part.starts_with('/') {
                return Err("file:// URIs must use absolute paths".to_string());
            }
        }

        Ok(())
    }

    /// Validate URI template pattern (basic validation for template syntax)
    fn validate_uri_template(&self, template: &str) -> std::result::Result<(), String> {
        // First validate the base URI structure (ignoring template variables)
        let base_uri = template.replace(['{', '}'], "");
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
    pub fn handlers<H: McpHandler + 'static, I: IntoIterator<Item = H>>(
        mut self,
        handlers: I,
    ) -> Self {
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
    ///
    /// **Note**: This method is now optional. The framework automatically calls this
    /// when resources are registered via `.resource()` or `.template_resource()`.
    /// You only need to call this explicitly if you want to enable resource capabilities
    /// without registering any resources.
    pub fn with_resources(mut self) -> Self {
        // Enable notifications if we have resources
        let has_resources = !self.resources.is_empty() || !self.template_resources.is_empty();

        self.capabilities.resources = Some(ResourcesCapabilities {
            subscribe: Some(false), // TODO: Implement resource subscriptions
            list_changed: Some(has_resources),
        });

        // Create ResourcesListHandler and add all registered resources
        let mut list_handler = ResourcesListHandler::new();
        tracing::debug!(
            "with_resources() - adding {} static resources to list handler",
            self.resources.len()
        );
        for (uri, resource) in &self.resources {
            tracing::debug!("Adding static resource to list handler: {}", uri);
            list_handler = list_handler.add_resource_arc(resource.clone());
        }

        // Template resources should NOT be added to ResourcesListHandler
        // They only appear in ResourceTemplatesHandler (resources/templates/list)
        // NOT in resources/list

        // Create ResourcesReadHandler and add all registered resources
        // Auto-configure security based on registered resources
        let mut read_handler = if self.test_mode {
            ResourcesReadHandler::new().without_security()
        } else if has_resources {
            // Auto-generate security configuration from registered resources
            let security_middleware = self.build_resource_security();
            ResourcesReadHandler::new().with_security(Arc::new(security_middleware))
        } else {
            ResourcesReadHandler::new()
        };
        for resource in self.resources.values() {
            read_handler = read_handler.add_resource_arc(resource.clone());
        }

        // Add template resources to read handler with template support
        for (template, resource) in &self.template_resources {
            read_handler =
                read_handler.add_template_resource_arc(template.clone(), resource.clone());
        }

        // Update notification manager

        // Register both handlers
        self.handler(list_handler).handler(read_handler)
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
    ///
    /// Note: Elicitation is a client-side capability per MCP 2025-11-25.
    /// The server requests elicitation from the client; it doesn't advertise it.
    pub fn with_elicitation(self) -> Self {
        self.handler(ElicitationHandler::with_mock_provider())
    }

    /// Add elicitation support with custom provider
    ///
    /// Note: Elicitation is a client-side capability per MCP 2025-11-25.
    /// The server requests elicitation from the client; it doesn't advertise it.
    pub fn with_elicitation_provider<P: ElicitationProvider + 'static>(self, provider: P) -> Self {
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

    /// Enable test mode - disables security middleware for test servers
    ///
    /// In test mode, ResourcesReadHandler is created without security middleware,
    /// allowing custom URI schemes for testing (binary://, memory://, error://, etc.).
    /// Production servers should NOT use test mode as it bypasses security controls.
    ///
    /// # Example
    /// ```rust,no_run
    /// use turul_mcp_server::McpServer;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let server = McpServer::builder()
    ///     .name("test-server")
    ///     .version("1.0.0")
    ///     .test_mode()  // Disable security for testing
    ///     .with_resources()
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn test_mode(mut self) -> Self {
        self.test_mode = true;
        self
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
    pub fn with_session_storage<
        S: turul_mcp_session_storage::SessionStorage<
                Error = turul_mcp_session_storage::SessionStorageError,
            > + 'static,
    >(
        mut self,
        storage: Arc<S>,
    ) -> Self {
        // Convert concrete storage type to trait object
        let boxed_storage: Arc<turul_mcp_session_storage::BoxedSessionStorage> = storage;
        self.session_storage = Some(boxed_storage);
        self
    }

    /// Configure task storage to enable MCP task support for long-running operations.
    ///
    /// When task storage is configured, the server will:
    /// - Advertise `tasks` capabilities in the initialize response
    /// - Register handlers for `tasks/get`, `tasks/list`, `tasks/cancel`, `tasks/result`
    /// - Recover stuck tasks (in `Working`/`InputRequired` state) on startup
    ///
    /// # Example
    /// ```rust,no_run
    /// # use turul_mcp_server::prelude::*;
    /// # use turul_mcp_task_storage::InMemoryTaskStorage;
    /// # use std::sync::Arc;
    /// #
    /// # fn example() -> McpResult<()> {
    /// let server = McpServer::builder()
    ///     .name("task-server")
    ///     .with_task_storage(Arc::new(InMemoryTaskStorage::new()))
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_task_storage(
        mut self,
        storage: Arc<dyn turul_mcp_task_storage::TaskStorage>,
    ) -> Self {
        let runtime = crate::task_runtime::TaskRuntime::with_default_executor(storage)
            .with_recovery_timeout(self.task_recovery_timeout_ms);
        self.task_runtime = Some(Arc::new(runtime));
        self
    }

    /// Configure task support with a pre-built `TaskRuntime`.
    ///
    /// Use this when you need fine-grained control over the task runtime configuration.
    pub fn with_task_runtime(mut self, runtime: Arc<crate::task_runtime::TaskRuntime>) -> Self {
        self.task_runtime = Some(runtime);
        self
    }

    /// Set the recovery timeout for stuck tasks (in milliseconds).
    ///
    /// On server startup, tasks in non-terminal states older than this timeout
    /// will be marked as `Failed`. Default: 300,000 ms (5 minutes).
    pub fn task_recovery_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.task_recovery_timeout_ms = timeout_ms;
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

    /// Allow or disallow ping requests without Mcp-Session-Id header (requires "http" feature)
    ///
    /// Default: `true` (sessionless pings allowed per MCP spec).
    /// Set to `false` for hardened deployments requiring session for all methods.
    #[cfg(feature = "http")]
    pub fn allow_unauthenticated_ping(mut self, allow: bool) -> Self {
        self.allow_unauthenticated_ping = Some(allow);
        self
    }

    /// Auto-generate security configuration based on registered resources
    fn build_resource_security(&self) -> crate::security::SecurityMiddleware {
        use crate::security::{AccessLevel, ResourceAccessControl, SecurityMiddleware};
        use regex::Regex;
        use std::collections::HashSet;

        let mut allowed_patterns = Vec::new();
        let mut allowed_extensions = HashSet::new();

        // Extract patterns from static resources
        for uri in self.resources.keys() {
            // Extract file extension
            if let Some(extension) = Self::extract_extension(uri) {
                allowed_extensions.insert(extension);
            }

            // Generate regex pattern for this URI's base path
            if let Some(base_pattern) = Self::uri_to_base_pattern(uri) {
                allowed_patterns.push(base_pattern);
            }
        }

        // Extract patterns from template resources
        for (template, _) in &self.template_resources {
            if let Some(pattern) = Self::template_to_regex_pattern(template.pattern()) {
                allowed_patterns.push(pattern);
            }

            // Extract extension from template if present
            if let Some(extension) = Self::extract_extension(template.pattern()) {
                allowed_extensions.insert(extension);
            }
        }

        // Build allowed MIME types from file extensions
        let allowed_mime_types = Self::extensions_to_mime_types(&allowed_extensions);

        // Convert pattern strings to Regex objects
        let regex_patterns: Vec<Regex> = allowed_patterns
            .into_iter()
            .filter_map(|pattern| Regex::new(&pattern).ok())
            .collect();

        tracing::debug!(
            "Auto-generated resource security: {} patterns, {} mime types",
            regex_patterns.len(),
            allowed_mime_types.len()
        );

        SecurityMiddleware::new().with_resource_access_control(ResourceAccessControl {
            access_level: AccessLevel::Public, // Allow access without session for auto-detected resources
            allowed_patterns: regex_patterns,
            blocked_patterns: vec![
                Regex::new(r"\.\.").unwrap(),  // Still prevent directory traversal
                Regex::new(r"/etc/").unwrap(), // Block system directories
                Regex::new(r"/proc/").unwrap(),
            ],
            max_size: Some(50 * 1024 * 1024), // 50MB limit for auto-detected resources
            allowed_mime_types: Some(allowed_mime_types),
        })
    }

    /// Extract file extension from URI
    fn extract_extension(uri: &str) -> Option<String> {
        uri.split('.')
            .next_back()
            .filter(|ext| !ext.is_empty() && ext.len() <= 10)
            .map(|ext| ext.to_lowercase())
    }

    /// Convert URI to base regex pattern that allows files in the same directory
    fn uri_to_base_pattern(uri: &str) -> Option<String> {
        if let Some(last_slash) = uri.rfind('/') {
            let base_path = &uri[..last_slash];
            Some(format!("^{}/[^/]+$", regex::escape(base_path)))
        } else {
            None
        }
    }

    /// Convert URI template to regex pattern
    fn template_to_regex_pattern(template: &str) -> Option<String> {
        use regex::Regex;

        // Create a regex to find template variables in the original template
        let template_var_regex = Regex::new(r"\{[^}]+\}").ok()?;

        let mut result = String::new();
        let mut last_end = 0;

        // Process each template variable
        for mat in template_var_regex.find_iter(template) {
            // Add the escaped literal part before this match
            result.push_str(&regex::escape(&template[last_end..mat.start()]));

            // Add the regex pattern for the template variable
            result.push_str("[a-zA-Z0-9_.-]+"); // Allow dots for IDs like announcement_id

            last_end = mat.end();
        }

        // Add any remaining literal part
        result.push_str(&regex::escape(&template[last_end..]));

        Some(format!("^{}$", result))
    }

    /// Map file extensions to MIME types
    fn extensions_to_mime_types(extensions: &HashSet<String>) -> Vec<String> {
        let mut mime_types = Vec::new();

        for ext in extensions {
            match ext.as_str() {
                "json" => mime_types.push("application/json".to_string()),
                "csv" => mime_types.push("text/csv".to_string()),
                "txt" => mime_types.push("text/plain".to_string()),
                "html" => mime_types.push("text/html".to_string()),
                "md" => mime_types.push("text/markdown".to_string()),
                "xml" => mime_types.push("application/xml".to_string()),
                "pdf" => mime_types.push("application/pdf".to_string()),
                "png" => mime_types.push("image/png".to_string()),
                "jpg" | "jpeg" => mime_types.push("image/jpeg".to_string()),
                _ => {} // Unknown extensions not explicitly allowed
            }
        }

        // Always allow basic text types
        mime_types.extend_from_slice(&["text/plain".to_string(), "application/json".to_string()]);

        mime_types.sort();
        mime_types.dedup();
        mime_types
    }

    /// Build the MCP server
    pub fn build(mut self) -> Result<McpServer> {
        // Validate configuration
        if self.name.is_empty() {
            return Err(McpError::configuration("Server name cannot be empty"));
        }
        if self.version.is_empty() {
            return Err(McpError::configuration("Server version cannot be empty"));
        }

        // Check for validation errors collected during registration
        if !self.validation_errors.is_empty() {
            return Err(McpError::configuration(&format!(
                "Resource validation errors:\n{}",
                self.validation_errors.join("\n")
            )));
        }

        // Auto-register resource handlers if resources were registered
        // This eliminates the need for manual .with_resources() calls
        let has_resources = !self.resources.is_empty() || !self.template_resources.is_empty();
        if has_resources {
            // Automatically configure resource handlers - this will replace the empty defaults
            self = self.with_resources();
        }

        // Auto-detect and configure server capabilities based on registered components
        let has_tools = !self.tools.is_empty();
        let has_prompts = !self.prompts.is_empty();
        let has_roots = !self.roots.is_empty();
        let has_elicitations = !self.elicitations.is_empty();
        let has_completions = !self.completions.is_empty();
        let has_samplings = !self.sampling.is_empty();
        tracing::debug!("🔧 Has sampling configured: {}", has_samplings);
        let has_logging = !self.loggers.is_empty();
        tracing::debug!("🔧 Has logging configured: {}", has_logging);

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

        // Elicitation is a client-side capability per MCP 2025-11-25.
        // The server doesn't advertise elicitation support; it requests it from the client.
        let _ = has_elicitations; // suppress unused warning

        // Completion capabilities - enable if completion handlers are registered
        if has_completions {
            self.capabilities.completions = Some(CompletionsCapabilities {
                enabled: Some(true),
            });
        }

        // Logging capabilities - always enabled with comprehensive level support
        // Always enable logging for debugging/monitoring
        self.capabilities.logging = Some(LoggingCapabilities {
            enabled: Some(true),
            levels: Some(vec![
                "debug".to_string(),
                "info".to_string(),
                "warning".to_string(),
                "error".to_string(),
            ]),
        });

        // Tasks capabilities — auto-configure when task runtime is set
        let has_tasks = self.task_runtime.is_some();
        if has_tasks {
            use turul_mcp_protocol::initialize::*;

            // Advertise full task support: list, cancel, and task-augmented tools/call
            self.capabilities.tasks = Some(TasksCapabilities {
                list: Some(TasksListCapabilities::default()),
                cancel: Some(TasksCancelCapabilities::default()),
                requests: Some(TasksRequestCapabilities {
                    tools: Some(TasksToolCapabilities {
                        call: Some(TasksToolCallCapabilities::default()),
                        extra: Default::default(),
                    }),
                    extra: Default::default(),
                }),
                extra: Default::default(),
            });
        }

        tracing::debug!("🔧 Auto-configured server capabilities:");
        tracing::debug!("   - Tools: {}", has_tools);
        tracing::debug!("   - Resources: {}", has_resources);
        tracing::debug!("   - Prompts: {}", has_prompts);
        tracing::debug!("   - Roots: {}", has_roots);
        tracing::debug!("   - Elicitation: {}", has_elicitations);
        tracing::debug!("   - Completions: {}", has_completions);
        tracing::debug!("   - Tasks: {}", has_tasks);
        tracing::debug!("   - Logging: enabled");

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
            let resource_templates_handler =
                ResourceTemplatesHandler::new().with_templates(self.template_resources.clone());
            handlers.insert(
                "resources/templates/list".to_string(),
                Arc::new(resource_templates_handler),
            );
        }

        // Add ProvidedSamplingHandler if sampling providers were configured
        // This replaces the default SamplingHandler with one that actually calls
        // the registered providers' validate_request() and sample() methods
        if !self.sampling.is_empty() {
            handlers.insert(
                "sampling/createMessage".to_string(),
                Arc::new(ProvidedSamplingHandler::new(self.sampling)),
            );
        }

        // Add task handlers if task runtime is configured
        if let Some(ref runtime) = self.task_runtime {
            handlers.insert(
                "tasks/get".to_string(),
                Arc::new(crate::task_handlers::TasksGetHandler::new(Arc::clone(
                    runtime,
                ))),
            );
            handlers.insert(
                "tasks/list".to_string(),
                Arc::new(crate::task_handlers::TasksListHandler::new(Arc::clone(
                    runtime,
                ))),
            );
            handlers.insert(
                "tasks/cancel".to_string(),
                Arc::new(crate::task_handlers::TasksCancelHandler::new(Arc::clone(
                    runtime,
                ))),
            );
            handlers.insert(
                "tasks/result".to_string(),
                Arc::new(crate::task_handlers::TasksResultHandler::new(Arc::clone(
                    runtime,
                ))),
            );
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
            self.task_runtime,
            self.strict_lifecycle,
            self.middleware_stack,
            #[cfg(feature = "http")]
            self.bind_address,
            #[cfg(feature = "http")]
            self.mcp_path,
            #[cfg(feature = "http")]
            self.enable_cors,
            #[cfg(feature = "http")]
            self.enable_sse,
            #[cfg(feature = "http")]
            self.allow_unauthenticated_ping,
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
    use serde_json::Value;
    use std::collections::HashMap;
    use turul_mcp_builders::prelude::*; // HasBaseMetadata, HasDescription, etc.
    use turul_mcp_protocol::tools::ToolAnnotations;
    use turul_mcp_protocol::{CallToolResult, ToolSchema};

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
        fn name(&self) -> &str {
            "test"
        }
        fn title(&self) -> Option<&str> {
            None
        }
    }

    impl HasDescription for TestTool {
        fn description(&self) -> Option<&str> {
            Some("Test tool")
        }
    }

    impl HasInputSchema for TestTool {
        fn input_schema(&self) -> &ToolSchema {
            &self.input_schema
        }
    }

    impl HasOutputSchema for TestTool {
        fn output_schema(&self) -> Option<&ToolSchema> {
            None
        }
    }

    impl HasAnnotations for TestTool {
        fn annotations(&self) -> Option<&ToolAnnotations> {
            None
        }
    }

    impl HasToolMeta for TestTool {
        fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
            None
        }
    }

    impl HasIcons for TestTool {}

    #[async_trait]
    impl McpTool for TestTool {
        async fn call(
            &self,
            _args: Value,
            _session: Option<SessionContext>,
        ) -> crate::McpResult<CallToolResult> {
            Ok(CallToolResult::success(vec![
                turul_mcp_protocol::ToolResult::text("test"),
            ]))
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
        assert_eq!(builder.handlers.len(), 21); // MCP 2025-11-25 handlers (spec + legacy compat)
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
        assert_eq!(
            builder.instructions,
            Some("This is a test server".to_string())
        );
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
        let result = McpServerBuilder::new().name("").build();

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            McpError::ConfigurationError(_)
        ));
    }

    // Test resources for auto-detection testing
    use turul_mcp_protocol::resources::ResourceContent;
    // Resource traits now in builders crate (already imported via prelude above)

    struct StaticTestResource;

    impl HasResourceMetadata for StaticTestResource {
        fn name(&self) -> &str {
            "static_test"
        }
    }

    impl HasResourceDescription for StaticTestResource {
        fn description(&self) -> Option<&str> {
            Some("Static test resource")
        }
    }

    impl HasResourceUri for StaticTestResource {
        fn uri(&self) -> &str {
            "file:///test.txt"
        }
    }

    impl HasResourceMimeType for StaticTestResource {
        fn mime_type(&self) -> Option<&str> {
            Some("text/plain")
        }
    }

    impl HasResourceSize for StaticTestResource {
        fn size(&self) -> Option<u64> {
            None
        }
    }

    impl HasResourceAnnotations for StaticTestResource {
        fn annotations(&self) -> Option<&turul_mcp_protocol::meta::Annotations> {
            None
        }
    }

    impl HasResourceMeta for StaticTestResource {
        fn resource_meta(&self) -> Option<&HashMap<String, Value>> {
            None
        }
    }

    impl HasIcons for StaticTestResource {}

    #[async_trait]
    impl crate::McpResource for StaticTestResource {
        async fn read(
            &self,
            _params: Option<Value>,
            _session: Option<&crate::SessionContext>,
        ) -> crate::McpResult<Vec<ResourceContent>> {
            Ok(vec![ResourceContent::text(
                "file:///test.txt",
                "test content",
            )])
        }
    }

    struct TemplateTestResource;

    impl HasResourceMetadata for TemplateTestResource {
        fn name(&self) -> &str {
            "template_test"
        }
    }

    impl HasResourceDescription for TemplateTestResource {
        fn description(&self) -> Option<&str> {
            Some("Template test resource")
        }
    }

    impl HasResourceUri for TemplateTestResource {
        fn uri(&self) -> &str {
            "template://data/{id}.json"
        }
    }

    impl HasResourceMimeType for TemplateTestResource {
        fn mime_type(&self) -> Option<&str> {
            Some("application/json")
        }
    }

    impl HasResourceSize for TemplateTestResource {
        fn size(&self) -> Option<u64> {
            None
        }
    }

    impl HasResourceAnnotations for TemplateTestResource {
        fn annotations(&self) -> Option<&turul_mcp_protocol::meta::Annotations> {
            None
        }
    }

    impl HasResourceMeta for TemplateTestResource {
        fn resource_meta(&self) -> Option<&HashMap<String, Value>> {
            None
        }
    }

    impl HasIcons for TemplateTestResource {}

    #[async_trait]
    impl crate::McpResource for TemplateTestResource {
        async fn read(
            &self,
            _params: Option<Value>,
            _session: Option<&crate::SessionContext>,
        ) -> crate::McpResult<Vec<ResourceContent>> {
            Ok(vec![ResourceContent::text(
                "template://data/123.json",
                "test content",
            )])
        }
    }

    #[test]
    fn test_is_template_uri() {
        let builder = McpServerBuilder::new();

        // Test static URIs
        assert!(!builder.is_template_uri("file:///test.txt"));
        assert!(!builder.is_template_uri("http://example.com/api"));
        assert!(!builder.is_template_uri("memory://cache"));

        // Test template URIs
        assert!(builder.is_template_uri("file:///data/{id}.json"));
        assert!(builder.is_template_uri("template://users/{user_id}"));
        assert!(builder.is_template_uri("api://v1/{resource}/{id}"));

        // Test edge cases
        assert!(!builder.is_template_uri("file:///no-braces.txt"));
        assert!(!builder.is_template_uri("file:///missing-close.txt{"));
        assert!(!builder.is_template_uri("file:///missing-open.txt}"));
        assert!(builder.is_template_uri("file:///multiple/{id}/{type}.json"));
    }

    #[test]
    fn test_auto_detection_static_resource() {
        let builder = McpServerBuilder::new()
            .name("test-server")
            .resource(StaticTestResource);

        // Verify static resource was added to resources collection
        assert_eq!(builder.resources.len(), 1);
        assert!(builder.resources.contains_key("file:///test.txt"));
        assert_eq!(builder.template_resources.len(), 0);
        assert_eq!(builder.validation_errors.len(), 0);
    }

    #[test]
    fn test_auto_detection_template_resource() {
        let builder = McpServerBuilder::new()
            .name("test-server")
            .resource(TemplateTestResource);

        // Verify template resource was added to template_resources collection
        assert_eq!(builder.resources.len(), 0);
        assert_eq!(builder.template_resources.len(), 1);
        assert_eq!(builder.validation_errors.len(), 0);

        // Verify the template pattern is correct
        let (template, _) = &builder.template_resources[0];
        assert_eq!(template.pattern(), "template://data/{id}.json");
    }

    #[test]
    fn test_auto_detection_mixed_resources() {
        let builder = McpServerBuilder::new()
            .name("test-server")
            .resource(StaticTestResource)
            .resource(TemplateTestResource);

        // Verify both resources were categorized correctly
        assert_eq!(builder.resources.len(), 1);
        assert!(builder.resources.contains_key("file:///test.txt"));
        assert_eq!(builder.template_resources.len(), 1);

        let (template, _) = &builder.template_resources[0];
        assert_eq!(template.pattern(), "template://data/{id}.json");
    }

    #[test]
    fn test_template_resource_explicit_still_works() {
        let template = crate::uri_template::UriTemplate::new("template://explicit/{id}")
            .expect("Failed to create template");

        let builder = McpServerBuilder::new()
            .name("test-server")
            .template_resource(template, TemplateTestResource);

        // Verify explicit template_resource still works
        assert_eq!(builder.resources.len(), 0);
        assert_eq!(builder.template_resources.len(), 1);

        let (template, _) = &builder.template_resources[0];
        assert_eq!(template.pattern(), "template://explicit/{id}");
    }

    #[test]
    fn test_invalid_template_uri_error_handling() {
        struct InvalidTemplateResource;

        impl HasResourceMetadata for InvalidTemplateResource {
            fn name(&self) -> &str {
                "invalid_template"
            }
        }

        impl HasResourceDescription for InvalidTemplateResource {
            fn description(&self) -> Option<&str> {
                Some("Invalid template resource")
            }
        }

        impl HasResourceUri for InvalidTemplateResource {
            fn uri(&self) -> &str {
                "not-a-valid-uri-{id}"
            } // Invalid base URI without scheme
        }

        impl HasResourceMimeType for InvalidTemplateResource {
            fn mime_type(&self) -> Option<&str> {
                None
            }
        }

        impl HasResourceSize for InvalidTemplateResource {
            fn size(&self) -> Option<u64> {
                None
            }
        }

        impl HasResourceAnnotations for InvalidTemplateResource {
            fn annotations(&self) -> Option<&turul_mcp_protocol::meta::Annotations> {
                None
            }
        }

        impl HasResourceMeta for InvalidTemplateResource {
            fn resource_meta(&self) -> Option<&HashMap<String, Value>> {
                None
            }
        }

        impl HasIcons for InvalidTemplateResource {}

        #[async_trait]
        impl crate::McpResource for InvalidTemplateResource {
            async fn read(
                &self,
                _params: Option<Value>,
                _session: Option<&crate::SessionContext>,
            ) -> crate::McpResult<Vec<ResourceContent>> {
                Ok(vec![])
            }
        }

        let builder = McpServerBuilder::new()
            .name("test-server")
            .resource(InvalidTemplateResource);

        // The URI "not-a-valid-uri-{id}" has braces but lacks a scheme
        // So it will be detected as a template but fail validation during template creation
        assert!(!builder.validation_errors.is_empty());
        assert!(builder.validation_errors[0].contains("URI must be absolute with scheme"));

        // Resource is still added to template collection but validation error is captured
        // The error will be reported during build() to prevent invalid servers
        assert_eq!(builder.resources.len(), 0);
        assert_eq!(builder.template_resources.len(), 1);
    }

    #[test]
    fn test_automatic_resource_handler_registration() {
        // Test that resources automatically register handlers without needing .with_resources()
        let server_result = McpServerBuilder::new()
            .name("auto-resources-server")
            .resource(StaticTestResource)
            .resource(TemplateTestResource)
            .build();

        // Server should build successfully with automatic resource handler registration
        assert!(server_result.is_ok());
    }

    #[test]
    fn test_no_resources_builds_successfully() {
        // Test that servers without resources build successfully
        let server_result = McpServerBuilder::new().name("no-resources-server").build();

        assert!(server_result.is_ok());
    }

    #[test]
    fn test_explicit_with_resources_still_works() {
        // Test that explicit .with_resources() still works (no double registration)
        let server_result = McpServerBuilder::new()
            .name("explicit-resources-server")
            .resource(StaticTestResource)
            .with_resources() // Explicit call should not cause issues
            .build();

        // Should build successfully even with explicit .with_resources() call
        assert!(server_result.is_ok());
    }

    // Function resource constructor for testing resource_fn method
    fn create_static_test_resource() -> StaticTestResource {
        StaticTestResource
    }

    fn create_template_test_resource() -> TemplateTestResource {
        TemplateTestResource
    }

    #[test]
    fn test_resource_fn_static_resource() {
        // Test resource_fn with static resource (no template variables)
        let builder = McpServerBuilder::new()
            .name("resource-fn-static-server")
            .resource_fn(create_static_test_resource);

        // Verify static resource was registered correctly via resource_fn
        assert_eq!(builder.resources.len(), 1);
        assert!(builder.resources.contains_key("file:///test.txt"));
        assert_eq!(builder.template_resources.len(), 0);
        assert_eq!(builder.validation_errors.len(), 0);
    }

    #[test]
    fn test_resource_fn_template_resource() {
        // Test resource_fn with template resource (has template variables)
        let builder = McpServerBuilder::new()
            .name("resource-fn-template-server")
            .resource_fn(create_template_test_resource);

        // Verify template resource was auto-detected and registered correctly via resource_fn
        assert_eq!(builder.resources.len(), 0);
        assert_eq!(builder.template_resources.len(), 1);
        assert_eq!(builder.validation_errors.len(), 0);

        // Verify the template pattern is correct
        let (template, _) = &builder.template_resources[0];
        assert_eq!(template.pattern(), "template://data/{id}.json");
    }

    #[test]
    fn test_resource_fn_mixed_with_direct_registration() {
        // Test that resource_fn works alongside direct .resource() calls
        let builder = McpServerBuilder::new()
            .name("mixed-registration-server")
            .resource(StaticTestResource) // Direct registration
            .resource_fn(create_template_test_resource); // Function registration

        // Verify both registration methods work together
        assert_eq!(builder.resources.len(), 1);
        assert!(builder.resources.contains_key("file:///test.txt"));
        assert_eq!(builder.template_resources.len(), 1);

        let (template, _) = &builder.template_resources[0];
        assert_eq!(template.pattern(), "template://data/{id}.json");
    }

    #[test]
    fn test_resource_fn_multiple_resources() {
        // Test registering multiple resources via resource_fn
        let builder = McpServerBuilder::new()
            .name("multi-resource-fn-server")
            .resource_fn(create_static_test_resource)
            .resource_fn(create_template_test_resource);

        // Verify both resources were registered correctly
        assert_eq!(builder.resources.len(), 1);
        assert!(builder.resources.contains_key("file:///test.txt"));
        assert_eq!(builder.template_resources.len(), 1);

        let (template, _) = &builder.template_resources[0];
        assert_eq!(template.pattern(), "template://data/{id}.json");
    }

    #[test]
    fn test_resource_fn_builds_successfully() {
        // Test that server builds successfully with resource_fn registrations
        let server_result = McpServerBuilder::new()
            .name("resource-fn-build-server")
            .resource_fn(create_static_test_resource)
            .resource_fn(create_template_test_resource)
            .build();

        // Server should build successfully with automatic resource handler registration
        assert!(server_result.is_ok());

        let server = server_result.unwrap();
        assert_eq!(server.implementation.name, "resource-fn-build-server");

        // Verify capabilities were auto-configured for resources
        assert!(server.capabilities.resources.is_some());
        let resources_caps = server.capabilities.resources.as_ref().unwrap();
        assert_eq!(resources_caps.list_changed, Some(false)); // Static framework
    }
}
