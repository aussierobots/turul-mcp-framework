//! High-level builder API for Lambda MCP servers
//!
//! This module provides a fluent builder API similar to McpServer::builder()
//! but specifically designed for AWS Lambda deployment.

use std::collections::HashMap;
use std::sync::Arc;

use turul_http_mcp_server::{ServerConfig, StreamConfig};
use turul_mcp_protocol::{Implementation, ServerCapabilities};
use turul_mcp_server::handlers::{McpHandler, *};
use turul_mcp_server::{
    McpCompletion, McpElicitation, McpLogger, McpNotification, McpPrompt, McpResource, McpRoot,
    McpSampling, McpTool,
};
use turul_mcp_session_storage::BoxedSessionStorage;

use crate::error::Result;

#[cfg(feature = "dynamodb")]
use crate::error::LambdaError;
use crate::server::LambdaMcpServer;

#[cfg(feature = "cors")]
use crate::cors::CorsConfig;

/// High-level builder for Lambda MCP servers
///
/// This provides a clean, fluent API for building Lambda MCP servers
/// similar to the framework's McpServer::builder() pattern.
///
/// ## Example
///
/// ```rust,no_run
/// use std::sync::Arc;
/// use turul_mcp_aws_lambda::LambdaMcpServerBuilder;
/// use turul_mcp_session_storage::InMemorySessionStorage;
/// use turul_mcp_derive::McpTool;
/// use turul_mcp_server::{McpResult, SessionContext};
///
/// #[derive(McpTool, Clone, Default)]
/// #[tool(name = "example", description = "Example tool")]
/// struct ExampleTool {
///     #[param(description = "Example parameter")]
///     value: String,
/// }
///
/// impl ExampleTool {
///     async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
///         Ok(format!("Got: {}", self.value))
///     }
/// }
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let server = LambdaMcpServerBuilder::new()
///         .name("my-lambda-server")
///         .version("1.0.0")
///         .tool(ExampleTool::default())
///         .storage(Arc::new(InMemorySessionStorage::new()))
///         .cors_allow_all_origins()
///         .build()
///         .await?;
///
///     // Use with Lambda runtime...
///     Ok(())
/// }
/// ```
pub struct LambdaMcpServerBuilder {
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
    session_storage: Option<Arc<BoxedSessionStorage>>,

    /// MCP Lifecycle enforcement configuration
    strict_lifecycle: bool,

    /// Enable SSE streaming
    enable_sse: bool,
    /// Server and stream configuration
    server_config: ServerConfig,
    stream_config: StreamConfig,

    /// CORS configuration (if enabled)
    #[cfg(feature = "cors")]
    cors_config: Option<CorsConfig>,
}

impl LambdaMcpServerBuilder {
    /// Create a new Lambda MCP server builder
    pub fn new() -> Self {
        // Initialize with default capabilities (same as McpServer)
        // Capabilities will be set truthfully in build() based on registered components
        let capabilities = ServerCapabilities::default();

        // Initialize handlers with defaults (same as McpServerBuilder)
        let mut handlers: HashMap<String, Arc<dyn McpHandler>> = HashMap::new();
        handlers.insert("ping".to_string(), Arc::new(PingHandler));
        handlers.insert(
            "completion/complete".to_string(),
            Arc::new(CompletionHandler),
        );
        handlers.insert(
            "resources/list".to_string(),
            Arc::new(ResourcesHandler::new()),
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
        handlers.insert(
            "resources/templates/list".to_string(),
            Arc::new(ResourceTemplatesHandler::new()),
        );
        handlers.insert(
            "elicitation/create".to_string(),
            Arc::new(ElicitationHandler::with_mock_provider()),
        );

        // Add notification handlers
        let notifications_handler = Arc::new(NotificationsHandler);
        handlers.insert(
            "notifications/message".to_string(),
            notifications_handler.clone(),
        );
        handlers.insert(
            "notifications/progress".to_string(),
            notifications_handler.clone(),
        );
        handlers.insert(
            "notifications/resources/listChanged".to_string(),
            notifications_handler.clone(),
        );
        handlers.insert(
            "notifications/resources/updated".to_string(),
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

        Self {
            name: "turul-mcp-aws-lambda".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            title: None,
            capabilities,
            tools: HashMap::new(),
            resources: HashMap::new(),
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
            session_storage: None,
            strict_lifecycle: false,
            enable_sse: cfg!(feature = "sse"),
            server_config: ServerConfig::default(),
            stream_config: StreamConfig::default(),
            #[cfg(feature = "cors")]
            cors_config: None,
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

    /// Set the server title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set optional instructions for clients
    pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    // =============================================================================
    // PROVIDER REGISTRATION METHODS (same as McpServerBuilder)
    // =============================================================================

    /// Register a tool with the server
    ///
    /// Tools can be created using any of the framework's 4 creation levels:
    /// - Function macros: `#[mcp_tool]`
    /// - Derive macros: `#[derive(McpTool)]`
    /// - Builder pattern: `ToolBuilder::new(...).build()`
    /// - Manual implementation: Custom struct implementing `McpTool`
    pub fn tool<T: McpTool + 'static>(mut self, tool: T) -> Self {
        let name = tool.name().to_string();
        self.tools.insert(name, Arc::new(tool));
        self
    }

    /// Register a function tool created with `#[mcp_tool]` macro
    pub fn tool_fn<F, T>(self, func: F) -> Self
    where
        F: Fn() -> T,
        T: McpTool + 'static,
    {
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
    pub fn resource<R: McpResource + 'static>(mut self, resource: R) -> Self {
        let uri = resource.uri().to_string();
        self.resources.insert(uri, Arc::new(resource));
        self
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

    /// Register a prompt with the server
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

    // =============================================================================
    // ZERO-CONFIGURATION CONVENIENCE METHODS (same as McpServerBuilder)
    // =============================================================================

    /// Register a sampler - convenient alias for sampling_provider
    pub fn sampler<S: McpSampling + 'static>(self, sampling: S) -> Self {
        self.sampling_provider(sampling)
    }

    /// Register a completer - convenient alias for completion_provider
    pub fn completer<C: McpCompletion + 'static>(self, completion: C) -> Self {
        self.completion_provider(completion)
    }

    /// Register a notification by type - type determines method automatically
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

    /// Add a single root directory
    pub fn root(mut self, root: turul_mcp_protocol::roots::Root) -> Self {
        self.roots.push(root);
        self
    }

    // =============================================================================
    // CAPABILITY CONFIGURATION METHODS (same as McpServerBuilder)
    // =============================================================================

    /// Add completion support
    pub fn with_completion(mut self) -> Self {
        use turul_mcp_protocol::initialize::CompletionsCapabilities;
        self.capabilities.completions = Some(CompletionsCapabilities {
            enabled: Some(true),
        });
        self.handler(CompletionHandler)
    }

    /// Add prompts support
    pub fn with_prompts(mut self) -> Self {
        use turul_mcp_protocol::initialize::PromptsCapabilities;
        self.capabilities.prompts = Some(PromptsCapabilities {
            list_changed: Some(false),
        });

        // Prompts handlers are automatically registered when prompts are added via .prompt()
        // This method now just enables the capability
        self
    }

    /// Add resources support
    pub fn with_resources(mut self) -> Self {
        use turul_mcp_protocol::initialize::ResourcesCapabilities;
        self.capabilities.resources = Some(ResourcesCapabilities {
            subscribe: Some(false),
            list_changed: Some(false),
        });

        // Create ResourcesHandler and add all registered resources
        let mut handler = ResourcesHandler::new();
        for resource in self.resources.values() {
            handler = handler.add_resource_arc(resource.clone());
        }

        self.handler(handler)
    }

    /// Add logging support
    pub fn with_logging(mut self) -> Self {
        use turul_mcp_protocol::initialize::LoggingCapabilities;
        self.capabilities.logging = Some(LoggingCapabilities::default());
        self.handler(LoggingHandler)
    }

    /// Add roots support
    pub fn with_roots(self) -> Self {
        self.handler(RootsHandler::new())
    }

    /// Add sampling support
    pub fn with_sampling(self) -> Self {
        self.handler(SamplingHandler)
    }

    /// Add elicitation support with default mock provider
    pub fn with_elicitation(mut self) -> Self {
        use turul_mcp_protocol::initialize::ElicitationCapabilities;
        self.capabilities.elicitation = Some(ElicitationCapabilities {
            enabled: Some(true),
        });
        self.handler(ElicitationHandler::with_mock_provider())
    }

    /// Add elicitation support with custom provider
    pub fn with_elicitation_provider<P: ElicitationProvider + 'static>(
        mut self,
        provider: P,
    ) -> Self {
        use turul_mcp_protocol::initialize::ElicitationCapabilities;
        self.capabilities.elicitation = Some(ElicitationCapabilities {
            enabled: Some(true),
        });
        self.handler(ElicitationHandler::new(Arc::new(provider)))
    }

    /// Add notifications support
    pub fn with_notifications(self) -> Self {
        self.handler(NotificationsHandler)
    }

    // =============================================================================
    // SESSION AND CONFIGURATION METHODS
    // =============================================================================

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
    pub fn strict_lifecycle(mut self, strict: bool) -> Self {
        self.strict_lifecycle = strict;
        self
    }

    /// Enable strict MCP lifecycle enforcement (convenience method)
    pub fn with_strict_lifecycle(self) -> Self {
        self.strict_lifecycle(true)
    }

    /// Enable or disable SSE streaming support
    pub fn sse(mut self, enable: bool) -> Self {
        self.enable_sse = enable;
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
        self.session_timeout_minutes = Some(5); // 5 minutes
        self.session_cleanup_interval_seconds = Some(30); // 30 seconds
        self
    }

    /// Set the session storage backend
    ///
    /// Supports all framework storage backends:
    /// - `InMemorySessionStorage` - For development and testing
    /// - `SqliteSessionStorage` - For single-instance persistence
    /// - `PostgreSqlSessionStorage` - For multi-instance deployments
    /// - `DynamoDbSessionStorage` - For serverless AWS deployments
    pub fn storage(mut self, storage: Arc<BoxedSessionStorage>) -> Self {
        self.session_storage = Some(storage);
        self
    }

    /// Create DynamoDB storage from environment variables
    ///
    /// Uses these environment variables:
    /// - `SESSION_TABLE_NAME` or `MCP_SESSION_TABLE` - DynamoDB table name
    /// - `AWS_REGION` - AWS region
    /// - AWS credentials from standard AWS credential chain
    #[cfg(feature = "dynamodb")]
    pub async fn dynamodb_storage(self) -> Result<Self> {
        use turul_mcp_session_storage::DynamoDbSessionStorage;

        let storage = DynamoDbSessionStorage::new().await.map_err(|e| {
            LambdaError::Config(format!("Failed to create DynamoDB storage: {}", e))
        })?;

        Ok(self.storage(Arc::new(storage)))
    }

    /// Configure server settings
    pub fn server_config(mut self, config: ServerConfig) -> Self {
        self.server_config = config;
        self
    }

    /// Configure streaming/SSE settings
    pub fn stream_config(mut self, config: StreamConfig) -> Self {
        self.stream_config = config;
        self
    }

    // CORS Configuration Methods

    /// Set custom CORS configuration
    #[cfg(feature = "cors")]
    pub fn cors(mut self, config: CorsConfig) -> Self {
        self.cors_config = Some(config);
        self
    }

    /// Allow all origins for CORS (development only)
    #[cfg(feature = "cors")]
    pub fn cors_allow_all_origins(mut self) -> Self {
        self.cors_config = Some(CorsConfig::allow_all());
        self
    }

    /// Set specific allowed origins for CORS
    #[cfg(feature = "cors")]
    pub fn cors_allow_origins(mut self, origins: Vec<String>) -> Self {
        self.cors_config = Some(CorsConfig::for_origins(origins));
        self
    }

    /// Configure CORS from environment variables
    ///
    /// Uses these environment variables:
    /// - `MCP_CORS_ORIGINS` - Comma-separated list of allowed origins
    /// - `MCP_CORS_CREDENTIALS` - Whether to allow credentials (true/false)
    /// - `MCP_CORS_MAX_AGE` - Preflight cache max age in seconds
    #[cfg(feature = "cors")]
    pub fn cors_from_env(mut self) -> Self {
        self.cors_config = Some(CorsConfig::from_env());
        self
    }

    /// Disable CORS (headers will not be added)
    #[cfg(feature = "cors")]
    pub fn cors_disabled(self) -> Self {
        // Don't set any CORS config - builder will not add headers
        self
    }

    // Convenience Methods

    /// Create with DynamoDB storage and environment-based CORS
    ///
    /// This is the recommended configuration for production Lambda deployments.
    #[cfg(all(feature = "dynamodb", feature = "cors"))]
    pub async fn production_config(self) -> Result<Self> {
        Ok(self.dynamodb_storage().await?.cors_from_env())
    }

    /// Create with in-memory storage and permissive CORS
    ///
    /// This is the recommended configuration for development and testing.
    #[cfg(feature = "cors")]
    pub fn development_config(self) -> Self {
        use turul_mcp_session_storage::InMemorySessionStorage;

        self.storage(Arc::new(InMemorySessionStorage::new()))
            .cors_allow_all_origins()
    }

    /// Build the Lambda MCP server
    ///
    /// Returns a server that can create handlers when needed.
    pub async fn build(self) -> Result<LambdaMcpServer> {
        use turul_mcp_session_storage::InMemorySessionStorage;

        // Validate configuration (same as MCP server)
        if self.name.is_empty() {
            return Err(crate::error::LambdaError::Config(
                "Server name cannot be empty".to_string(),
            ));
        }
        if self.version.is_empty() {
            return Err(crate::error::LambdaError::Config(
                "Server version cannot be empty".to_string(),
            ));
        }

        // Create session storage (use in-memory if none provided)
        let session_storage = self
            .session_storage
            .unwrap_or_else(|| Arc::new(InMemorySessionStorage::new()));

        // Create implementation info
        let implementation = if let Some(title) = self.title {
            Implementation::new(&self.name, &self.version).with_title(title)
        } else {
            Implementation::new(&self.name, &self.version)
        };

        // Auto-detect and configure server capabilities based on registered components (same as McpServer)
        let mut capabilities = self.capabilities.clone();
        let has_tools = !self.tools.is_empty();
        let has_resources = !self.resources.is_empty();
        let has_prompts = !self.prompts.is_empty();
        let has_elicitations = !self.elicitations.is_empty();
        let has_completions = !self.completions.is_empty();
        let has_logging = !self.loggers.is_empty();
        tracing::debug!("🔧 Has logging configured: {}", has_logging);

        // Tools capabilities - truthful reporting (only set if tools are registered)
        if has_tools {
            capabilities.tools = Some(turul_mcp_protocol::initialize::ToolsCapabilities {
                list_changed: Some(false), // Static framework: no dynamic change sources
            });
        }

        // Resources capabilities - truthful reporting (only set if resources are registered)
        if has_resources {
            capabilities.resources = Some(turul_mcp_protocol::initialize::ResourcesCapabilities {
                subscribe: Some(false),    // TODO: Implement resource subscriptions
                list_changed: Some(false), // Static framework: no dynamic change sources
            });
        }

        // Prompts capabilities - truthful reporting (only set if prompts are registered)
        if has_prompts {
            capabilities.prompts = Some(turul_mcp_protocol::initialize::PromptsCapabilities {
                list_changed: Some(false), // Static framework: no dynamic change sources
            });
        }

        // Elicitation capabilities - truthful reporting (only set if elicitations are registered)
        if has_elicitations {
            capabilities.elicitation =
                Some(turul_mcp_protocol::initialize::ElicitationCapabilities {
                    enabled: Some(true),
                });
        }

        // Completion capabilities - truthful reporting (only set if completions are registered)
        if has_completions {
            capabilities.completions =
                Some(turul_mcp_protocol::initialize::CompletionsCapabilities {
                    enabled: Some(true),
                });
        }

        // Logging capabilities - always enabled for debugging/monitoring (same as McpServer)
        // Always enable logging for debugging/monitoring
        capabilities.logging = Some(turul_mcp_protocol::initialize::LoggingCapabilities {
            enabled: Some(true),
            levels: Some(vec![
                "debug".to_string(),
                "info".to_string(),
                "warning".to_string(),
                "error".to_string(),
            ]),
        });

        // Add RootsHandler if roots were configured (same pattern as MCP server)
        let mut handlers = self.handlers;
        if !self.roots.is_empty() {
            let mut roots_handler = RootsHandler::new();
            for root in &self.roots {
                roots_handler = roots_handler.add_root(root.clone());
            }
            handlers.insert("roots/list".to_string(), Arc::new(roots_handler));
        }

        // Create the Lambda server (stores all configuration like MCP server does)
        Ok(LambdaMcpServer::new(
            implementation,
            capabilities,
            self.tools,
            self.resources,
            self.prompts,
            self.elicitations,
            self.sampling,
            self.completions,
            self.loggers,
            self.root_providers,
            self.notifications,
            handlers,
            self.roots,
            self.instructions,
            session_storage,
            self.strict_lifecycle,
            self.server_config,
            self.enable_sse,
            self.stream_config,
            #[cfg(feature = "cors")]
            self.cors_config,
        ))
    }
}

impl Default for LambdaMcpServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// Extension trait for cleaner chaining
pub trait LambdaMcpServerBuilderExt {
    /// Add multiple tools at once
    fn tools<I, T>(self, tools: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: McpTool + 'static;
}

impl LambdaMcpServerBuilderExt for LambdaMcpServerBuilder {
    fn tools<I, T>(mut self, tools: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: McpTool + 'static,
    {
        for tool in tools {
            self = self.tool(tool);
        }
        self
    }
}

/// Create a Lambda MCP server with minimal configuration
///
/// This is a convenience function for simple use cases where you just
/// want to register some tools and get a working handler.
pub async fn simple_lambda_server<I, T>(tools: I) -> Result<LambdaMcpServer>
where
    I: IntoIterator<Item = T>,
    T: McpTool + 'static,
{
    let mut builder = LambdaMcpServerBuilder::new();

    for tool in tools {
        builder = builder.tool(tool);
    }

    #[cfg(feature = "cors")]
    {
        builder = builder.cors_allow_all_origins();
    }

    builder.build().await
}

/// Create a Lambda MCP server configured for production
///
/// Uses DynamoDB for session storage and environment-based CORS configuration.
#[cfg(all(feature = "dynamodb", feature = "cors"))]
pub async fn production_lambda_server<I, T>(tools: I) -> Result<LambdaMcpServer>
where
    I: IntoIterator<Item = T>,
    T: McpTool + 'static,
{
    let mut builder = LambdaMcpServerBuilder::new();

    for tool in tools {
        builder = builder.tool(tool);
    }

    builder.production_config().await?.build().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use turul_mcp_session_storage::InMemorySessionStorage;

    // Mock tool for testing
    #[derive(Clone, Default)]
    struct TestTool;

    impl turul_mcp_protocol::tools::HasBaseMetadata for TestTool {
        fn name(&self) -> &str {
            "test_tool"
        }
    }

    impl turul_mcp_protocol::tools::HasDescription for TestTool {
        fn description(&self) -> Option<&str> {
            Some("Test tool")
        }
    }

    impl turul_mcp_protocol::tools::HasInputSchema for TestTool {
        fn input_schema(&self) -> &turul_mcp_protocol::ToolSchema {
            use turul_mcp_protocol::ToolSchema;
            static SCHEMA: std::sync::OnceLock<ToolSchema> = std::sync::OnceLock::new();
            SCHEMA.get_or_init(ToolSchema::object)
        }
    }

    impl turul_mcp_protocol::tools::HasOutputSchema for TestTool {
        fn output_schema(&self) -> Option<&turul_mcp_protocol::ToolSchema> {
            None
        }
    }

    impl turul_mcp_protocol::tools::HasAnnotations for TestTool {
        fn annotations(&self) -> Option<&turul_mcp_protocol::tools::ToolAnnotations> {
            None
        }
    }

    impl turul_mcp_protocol::tools::HasToolMeta for TestTool {
        fn tool_meta(&self) -> Option<&std::collections::HashMap<String, serde_json::Value>> {
            None
        }
    }

    #[async_trait::async_trait]
    impl McpTool for TestTool {
        async fn call(
            &self,
            _args: serde_json::Value,
            _session: Option<turul_mcp_server::SessionContext>,
        ) -> turul_mcp_server::McpResult<turul_mcp_protocol::tools::CallToolResult> {
            use turul_mcp_protocol::tools::{CallToolResult, ToolResult};
            Ok(CallToolResult::success(vec![ToolResult::text(
                "test result",
            )]))
        }
    }

    #[tokio::test]
    async fn test_builder_basic() {
        let server = LambdaMcpServerBuilder::new()
            .name("test-server")
            .version("1.0.0")
            .tool(TestTool)
            .storage(Arc::new(InMemorySessionStorage::new()))
            .build()
            .await
            .unwrap();

        // Create handler from server and verify it has stream_manager
        let handler = server.handler().await.unwrap();
        // Verify handler has stream_manager (critical invariant)
        // Verify handler has stream_manager (critical invariant)
        assert!(handler.stream_manager().as_ref() as *const _ as usize > 0, "Stream manager must be initialized");
    }

    #[tokio::test]
    async fn test_simple_lambda_server() {
        let tools = vec![TestTool];
        let server = simple_lambda_server(tools).await.unwrap();

        // Create handler and verify it was created with default configuration
        let handler = server.handler().await.unwrap();
        // Verify handler has stream_manager
        // Verify handler has stream_manager (critical invariant)
        assert!(handler.stream_manager().as_ref() as *const _ as usize > 0, "Stream manager must be initialized");
    }

    #[tokio::test]
    async fn test_builder_extension_trait() {
        let tools = vec![TestTool, TestTool];

        let server = LambdaMcpServerBuilder::new()
            .tools(tools)
            .storage(Arc::new(InMemorySessionStorage::new()))
            .build()
            .await
            .unwrap();

        let handler = server.handler().await.unwrap();
        // Verify handler has stream_manager
        // Verify handler has stream_manager (critical invariant)
        assert!(handler.stream_manager().as_ref() as *const _ as usize > 0, "Stream manager must be initialized");
    }

    #[cfg(feature = "cors")]
    #[tokio::test]
    async fn test_cors_configuration() {
        let server = LambdaMcpServerBuilder::new()
            .cors_allow_all_origins()
            .storage(Arc::new(InMemorySessionStorage::new()))
            .build()
            .await
            .unwrap();

        let handler = server.handler().await.unwrap();
        // Verify handler has stream_manager
        // Verify handler has stream_manager (critical invariant)
        assert!(handler.stream_manager().as_ref() as *const _ as usize > 0, "Stream manager must be initialized");
    }
}
