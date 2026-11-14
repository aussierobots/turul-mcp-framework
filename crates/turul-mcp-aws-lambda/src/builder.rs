//! High-level builder API for Lambda MCP servers
//!
//! `LambdaMcpServerBuilder` is a thin wrapper over
//! [`turul_mcp_server::McpServerBuilder`] (ADR-035): every registration and
//! capability method delegates to it, so a Lambda server serves exactly what an
//! HTTP `McpServer` built from the same configuration would. This crate adds only
//! Lambda transport: session storage defaults, the Lambda `ServerConfig` and
//! streaming settings, and CORS.

use std::sync::Arc;

use turul_http_mcp_server::{ServerConfig, StreamConfig};
#[cfg(feature = "protocol-2025-11-25")]
use turul_mcp_server::handlers::ElicitationProvider;
use turul_mcp_server::handlers::McpHandler;
use turul_mcp_server::{
    McpCompletion, McpNotification, McpPrompt, McpResource, McpServer, McpServerBuilder, McpTool,
};
#[cfg(feature = "protocol-2025-11-25")]
use turul_mcp_server::{McpElicitation, McpLogger, McpSampling};
use turul_mcp_session_storage::BoxedSessionStorage;

use crate::error::{LambdaError, Result};
use crate::server::LambdaMcpServer;

#[cfg(feature = "cors")]
use crate::cors::CorsConfig;

/// High-level builder for Lambda MCP servers.
///
/// Same registration API as [`McpServer::builder`]; Lambda-only settings are
/// [`storage`](Self::storage), [`server_config`](Self::server_config),
/// [`stream_config`](Self::stream_config), [`sse`](Self::sse),
/// [`origin_policy`](Self::origin_policy) and the CORS methods.
///
/// ```rust,no_run
/// use turul_mcp_aws_lambda::LambdaMcpServerBuilder;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let server = LambdaMcpServerBuilder::new()
///     .name("my-lambda-server")
///     .version("1.0.0")
///     .build()
///     .await?;
/// let handler = server.handler().await?;
/// # let _ = handler;
/// # Ok(())
/// # }
/// ```
pub struct LambdaMcpServerBuilder {
    /// The owner of handlers, capabilities and validation.
    inner: McpServerBuilder,
    /// Session storage (in-memory when not set).
    session_storage: Option<Arc<BoxedSessionStorage>>,
    /// Lambda HTTP-layer configuration.
    server_config: ServerConfig,
    /// Enable SSE responses.
    enable_sse: bool,
    /// Streaming configuration.
    stream_config: StreamConfig,
    /// Explicit Origin policy; otherwise derived from CORS at build (ADR-031).
    origin_policy: Option<turul_http_mcp_server::OriginPolicy>,
    /// CORS configuration.
    #[cfg(feature = "cors")]
    cors_config: Option<CorsConfig>,
}

impl LambdaMcpServerBuilder {
    /// Create a new Lambda MCP server builder.
    pub fn new() -> Self {
        let inner = McpServer::builder();
        // Buffered Lambda has no long-lived `subscriptions/listen` stream, so it
        // does not advertise resource subscriptions (ADR-035 §3).
        #[cfg(feature = "protocol-2026-07-28")]
        let inner = inner.resource_subscriptions(false);
        let mut server_config = ServerConfig::default();
        let enable_sse = cfg!(feature = "sse");
        #[allow(deprecated)] // 2025-lane SSE switches; harmless no-ops on 2026
        {
            server_config.enable_get_sse = enable_sse;
            server_config.enable_post_sse = enable_sse;
        }
        Self {
            inner,
            session_storage: None,
            server_config,
            enable_sse,
            stream_config: StreamConfig::default(),
            origin_policy: None,
            #[cfg(feature = "cors")]
            cors_config: None,
        }
    }

    /// Advertise an entry under `capabilities.extensions`; see
    /// [`McpServerBuilder::extension_capability`].
    #[cfg(feature = "protocol-2026-07-28")]
    pub fn extension_capability(
        mut self,
        identifier: impl Into<String>,
        settings: serde_json::Value,
    ) -> Self {
        self.inner = self.inner.extension_capability(identifier, settings);
        self
    }

    /// Serve the Skills extension; see [`McpServerBuilder::with_ext_skills`]
    /// (ADR-033). Build the catalog once per container (at cold start).
    #[cfg(feature = "ext-skills")]
    pub fn with_ext_skills(mut self, catalog: turul_mcp_server::ext_skills::SkillCatalog) -> Self {
        self.inner = self.inner.with_ext_skills(catalog);
        self
    }

    /// Set the server name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.inner = self.inner.name(name);
        self
    }

    /// Set the server version
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.inner = self.inner.version(version);
        self
    }

    /// Set the server title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.inner = self.inner.title(title);
        self
    }

    /// Set icons for the server (displayed by MCP clients like Claude Desktop)
    pub fn icons(mut self, icons: Vec<turul_mcp_protocol::Icon>) -> Self {
        self.inner = self.inner.icons(icons);
        self
    }

    /// Set optional instructions for clients
    pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
        self.inner = self.inner.instructions(instructions);
        self
    }

    /// Register a tool with the server
    ///
    /// Tools can be created using any of the framework's 4 creation levels:
    /// - Function macros: `#[mcp_tool]`
    /// - Derive macros: `#[derive(McpTool)]`
    /// - Builder pattern: `ToolBuilder::new(...).build()`
    /// - Manual implementation: Custom struct implementing `McpTool`
    pub fn tool<T: McpTool + 'static>(mut self, tool: T) -> Self {
        self.inner = self.inner.tool(tool);
        self
    }

    /// Register a function tool created with `#[mcp_tool]` macro
    pub fn tool_fn<F, T>(mut self, func: F) -> Self
    where
        F: Fn() -> T,
        T: McpTool + 'static,
    {
        self.inner = self.inner.tool_fn(func);
        self
    }

    /// Register multiple tools
    pub fn tools<T: McpTool + 'static, I: IntoIterator<Item = T>>(mut self, tools: I) -> Self {
        self.inner = self.inner.tools(tools);
        self
    }

    /// Register a resource with the server
    ///
    /// Automatically detects template resources (URIs containing `{variables}`)
    /// and routes them to the template resource list. Template resources appear
    /// in `resources/templates/list`, not `resources/list`.
    pub fn resource<R: McpResource + 'static>(mut self, resource: R) -> Self {
        self.inner = self.inner.resource(resource);
        self
    }

    /// Register multiple resources
    pub fn resources<R: McpResource + 'static, I: IntoIterator<Item = R>>(
        mut self,
        resources: I,
    ) -> Self {
        self.inner = self.inner.resources(resources);
        self
    }

    /// Register a prompt with the server
    pub fn prompt<P: McpPrompt + 'static>(mut self, prompt: P) -> Self {
        self.inner = self.inner.prompt(prompt);
        self
    }

    /// Register multiple prompts
    pub fn prompts<P: McpPrompt + 'static, I: IntoIterator<Item = P>>(
        mut self,
        prompts: I,
    ) -> Self {
        self.inner = self.inner.prompts(prompts);
        self
    }

    /// Register an elicitation provider with the server
    #[cfg(feature = "protocol-2025-11-25")]
    pub fn elicitation<E: McpElicitation + 'static>(mut self, elicitation: E) -> Self {
        self.inner = self.inner.elicitation(elicitation);
        self
    }

    /// Register multiple elicitation providers
    #[cfg(feature = "protocol-2025-11-25")]
    pub fn elicitations<E: McpElicitation + 'static, I: IntoIterator<Item = E>>(
        mut self,
        elicitations: I,
    ) -> Self {
        self.inner = self.inner.elicitations(elicitations);
        self
    }

    /// Register a sampling provider with the server
    #[cfg(feature = "protocol-2025-11-25")]
    pub fn sampling_provider<S: McpSampling + 'static>(mut self, sampling: S) -> Self {
        self.inner = self.inner.sampling_provider(sampling);
        self
    }

    /// Register multiple sampling providers
    #[cfg(feature = "protocol-2025-11-25")]
    pub fn sampling_providers<S: McpSampling + 'static, I: IntoIterator<Item = S>>(
        mut self,
        sampling: I,
    ) -> Self {
        self.inner = self.inner.sampling_providers(sampling);
        self
    }

    /// Register a completion provider with the server
    pub fn completion_provider<C: McpCompletion + 'static>(mut self, completion: C) -> Self {
        self.inner = self.inner.completion_provider(completion);
        self
    }

    /// Register multiple completion providers
    pub fn completion_providers<C: McpCompletion + 'static, I: IntoIterator<Item = C>>(
        mut self,
        completions: I,
    ) -> Self {
        self.inner = self.inner.completion_providers(completions);
        self
    }

    /// Register a logger with the server
    #[cfg(feature = "protocol-2025-11-25")]
    pub fn logger<L: McpLogger + 'static>(mut self, logger: L) -> Self {
        self.inner = self.inner.logger(logger);
        self
    }

    /// Register multiple loggers
    #[cfg(feature = "protocol-2025-11-25")]
    pub fn loggers<L: McpLogger + 'static, I: IntoIterator<Item = L>>(
        mut self,
        loggers: I,
    ) -> Self {
        self.inner = self.inner.loggers(loggers);
        self
    }

    /// Register a notification provider with the server
    pub fn notification_provider<N: McpNotification + 'static>(mut self, notification: N) -> Self {
        self.inner = self.inner.notification_provider(notification);
        self
    }

    /// Register multiple notification providers
    pub fn notification_providers<N: McpNotification + 'static, I: IntoIterator<Item = N>>(
        mut self,
        notifications: I,
    ) -> Self {
        self.inner = self.inner.notification_providers(notifications);
        self
    }

    /// Register a sampler - convenient alias for sampling_provider
    #[cfg(feature = "protocol-2025-11-25")]
    pub fn sampler<S: McpSampling + 'static>(mut self, sampling: S) -> Self {
        self.inner = self.inner.sampler(sampling);
        self
    }

    /// Register a completer - convenient alias for completion_provider
    pub fn completer<C: McpCompletion + 'static>(mut self, completion: C) -> Self {
        self.inner = self.inner.completer(completion);
        self
    }

    /// Register a notification by type - type determines method automatically
    pub fn notification_type<N: McpNotification + 'static + Default>(mut self) -> Self {
        self.inner = self.inner.notification_type::<N>();
        self
    }

    /// Register a handler with the server
    pub fn handler<H: McpHandler + 'static>(mut self, handler: H) -> Self {
        self.inner = self.inner.handler(handler);
        self
    }

    /// Register multiple handlers
    pub fn handlers<H: McpHandler + 'static, I: IntoIterator<Item = H>>(
        mut self,
        handlers: I,
    ) -> Self {
        self.inner = self.inner.handlers(handlers);
        self
    }

    /// Add a single root directory
    #[allow(deprecated)]
    #[cfg(feature = "protocol-2025-11-25")]
    pub fn root(mut self, root: turul_mcp_protocol::roots::Root) -> Self {
        self.inner = self.inner.root(root);
        self
    }

    /// Add completion support
    pub fn with_completion(mut self) -> Self {
        self.inner = self.inner.with_completion();
        self
    }

    /// Add prompts support
    pub fn with_prompts(mut self) -> Self {
        self.inner = self.inner.with_prompts();
        self
    }

    /// Add resources support
    pub fn with_resources(mut self) -> Self {
        self.inner = self.inner.with_resources();
        self
    }

    /// Add logging support
    #[cfg(feature = "protocol-2025-11-25")]
    pub fn with_logging(mut self) -> Self {
        self.inner = self.inner.with_logging();
        self
    }

    /// Add roots support
    #[cfg(feature = "protocol-2025-11-25")]
    pub fn with_roots(mut self) -> Self {
        self.inner = self.inner.with_roots();
        self
    }

    /// Add sampling support
    #[cfg(feature = "protocol-2025-11-25")]
    pub fn with_sampling(mut self) -> Self {
        self.inner = self.inner.with_sampling();
        self
    }

    /// Add elicitation support with default mock provider
    ///
    /// Note: Elicitation is a client-side capability per MCP 2025-11-25.
    /// The server requests elicitation from the client; it doesn't advertise it.
    #[cfg(feature = "protocol-2025-11-25")]
    pub fn with_elicitation(mut self) -> Self {
        self.inner = self.inner.with_elicitation();
        self
    }

    /// Add elicitation support with custom provider
    ///
    /// Note: Elicitation is a client-side capability per MCP 2025-11-25.
    /// The server requests elicitation from the client; it doesn't advertise it.
    #[cfg(feature = "protocol-2025-11-25")]
    pub fn with_elicitation_provider<P: ElicitationProvider + 'static>(
        mut self,
        provider: P,
    ) -> Self {
        self.inner = self.inner.with_elicitation_provider(provider);
        self
    }

    /// Add notifications support
    pub fn with_notifications(mut self) -> Self {
        self.inner = self.inner.with_notifications();
        self
    }

    /// Configure task storage to enable MCP task support for long-running operations.
    ///
    /// When task storage is configured, the server will:
    /// - Advertise `tasks` capabilities in the initialize response
    /// - Register handlers for `tasks/get`, `tasks/list`, `tasks/cancel`, `tasks/result`
    /// - Wire task-augmented `tools/call` for `CreateTaskResult` returns
    /// - Recover stuck tasks on cold start
    ///
    /// **Lambda note**: Use a durable backend (DynamoDB recommended) since Lambda
    /// invocations are stateless. `InMemoryTaskStorage` will lose state between invocations.
    #[cfg(feature = "protocol-2025-11-25")]
    pub fn with_task_storage(
        mut self,
        storage: Arc<dyn turul_mcp_server::task_storage::TaskStorage>,
    ) -> Self {
        self.inner = self.inner.with_task_storage(storage);
        self
    }

    /// Configure task support with a pre-built `TaskRuntime`.
    ///
    /// Use this when you need fine-grained control over the task runtime configuration.
    #[cfg(feature = "protocol-2025-11-25")]
    pub fn with_task_runtime(mut self, runtime: Arc<turul_mcp_server::TaskRuntime>) -> Self {
        self.inner = self.inner.with_task_runtime(runtime);
        self
    }

    /// Set the recovery timeout for stuck tasks (in milliseconds).
    ///
    /// On Lambda cold start, tasks in non-terminal states older than this timeout
    /// will be marked as `Failed`. Default: 300,000 ms (5 minutes).
    #[cfg(feature = "protocol-2025-11-25")]
    pub fn task_recovery_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.inner = self.inner.task_recovery_timeout_ms(timeout_ms);
        self
    }

    /// Set the tool change detection and notification mode.
    ///
    /// - `Static` (default): No change detection, no fingerprint, no notifications. `listChanged=false`.
    /// - `Dynamic` (requires `dynamic-tools` feature): Runtime tool activation/deactivation
    ///   with live `notifications/tools/list_changed`. `listChanged=true`.
    ///   Optionally pair with `.server_state_storage()` for cross-instance coordination.
    pub fn tool_change_mode(mut self, mode: turul_mcp_server::ToolChangeMode) -> Self {
        self.inner = self.inner.tool_change_mode(mode);
        self
    }

    /// Set the server state storage backend for cross-instance coordination.
    ///
    /// When provided with `ToolChangeMode::Dynamic`, tool activation state is
    /// persisted to this backend so multiple server instances share the same
    /// view of which tools are active. Without this, an in-memory backend is
    /// used automatically (suitable for single-process deployments).
    #[cfg(feature = "dynamic-tools")]
    pub fn server_state_storage(
        mut self,
        storage: Arc<dyn turul_mcp_server_state_storage::ServerStateStorage>,
    ) -> Self {
        self.inner = self.inner.server_state_storage(storage);
        self
    }

    /// Configure session timeout (in minutes, default: 30)
    pub fn session_timeout_minutes(mut self, minutes: u64) -> Self {
        self.inner = self.inner.session_timeout_minutes(minutes);
        self
    }

    /// Configure session cleanup interval (in seconds, default: 60)
    pub fn session_cleanup_interval_seconds(mut self, seconds: u64) -> Self {
        self.inner = self.inner.session_cleanup_interval_seconds(seconds);
        self
    }

    /// Enable strict MCP lifecycle enforcement
    pub fn strict_lifecycle(mut self, strict: bool) -> Self {
        self.inner = self.inner.strict_lifecycle(strict);
        self
    }

    /// Enable strict MCP lifecycle enforcement (convenience method)
    pub fn with_strict_lifecycle(mut self) -> Self {
        self.inner = self.inner.with_strict_lifecycle();
        self
    }

    /// Enable or disable SSE streaming support
    pub fn sse(mut self, enable: bool) -> Self {
        self.enable_sse = enable;

        // Update SSE endpoints in ServerConfig based on enable flag
        // (enable_get_sse is 2026-lane-deprecated; the setter remains the
        // 2025-lane control and a harmless no-op on 2026)
        #[allow(deprecated)]
        if enable {
            self.server_config.enable_get_sse = true;
            self.server_config.enable_post_sse = true;
        } else {
            // When SSE is disabled, also disable SSE endpoints in ServerConfig
            // This prevents GET /mcp from hanging by returning 405 instead
            self.server_config.enable_get_sse = false;
            self.server_config.enable_post_sse = false;
        }

        self
    }

    /// Configure sessions with recommended defaults for long-running sessions
    pub fn with_long_sessions(mut self) -> Self {
        self.inner = self.inner.with_long_sessions();
        self
    }

    /// Configure sessions with recommended defaults for short-lived sessions
    pub fn with_short_sessions(mut self) -> Self {
        self.inner = self.inner.with_short_sessions();
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
            LambdaError::Configuration(format!("Failed to create DynamoDB storage: {}", e))
        })?;

        Ok(self.storage(Arc::new(storage)))
    }

    /// Register middleware for request/response interception
    ///
    /// Middleware can inspect and modify requests before they reach handlers,
    /// inject data into sessions, and transform responses. Multiple middleware
    /// can be registered and will execute in FIFO order for before_dispatch
    /// and LIFO order for after_dispatch.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use std::sync::Arc;
    /// use turul_mcp_aws_lambda::LambdaMcpServerBuilder;
    /// use turul_http_mcp_server::middleware::McpMiddleware;
    /// # use turul_mcp_session_storage::SessionView;
    /// # use turul_http_mcp_server::middleware::{RequestContext, SessionInjection, MiddlewareError};
    /// # use async_trait::async_trait;
    /// # struct AuthMiddleware;
    /// # #[async_trait]
    /// # impl McpMiddleware for AuthMiddleware {
    /// #     async fn before_dispatch(&self, _: &mut RequestContext<'_>, _: Option<&dyn SessionView>, _: &mut SessionInjection) -> Result<(), MiddlewareError> { Ok(()) }
    /// # }
    /// # struct RateLimitMiddleware;
    /// # #[async_trait]
    /// # impl McpMiddleware for RateLimitMiddleware {
    /// #     async fn before_dispatch(&self, _: &mut RequestContext<'_>, _: Option<&dyn SessionView>, _: &mut SessionInjection) -> Result<(), MiddlewareError> { Ok(()) }
    /// # }
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let builder = LambdaMcpServerBuilder::new()
    ///     .name("my-server")
    ///     .middleware(Arc::new(AuthMiddleware))
    ///     .middleware(Arc::new(RateLimitMiddleware));
    /// # Ok(())
    /// # }
    /// ```
    pub fn middleware(
        mut self,
        middleware: Arc<dyn turul_http_mcp_server::middleware::McpMiddleware>,
    ) -> Self {
        self.inner = self.inner.middleware(middleware);
        self
    }

    /// Register a custom HTTP route (e.g., `.well-known` endpoints)
    pub fn route(
        mut self,
        path: &str,
        handler: Arc<dyn turul_http_mcp_server::RouteHandler>,
    ) -> Self {
        self.inner = self.inner.route(path, handler);
        self
    }

    /// Configure server settings
    /// Set the Origin-header validation policy explicitly (DNS-rebinding
    /// protection). When not set, the policy is derived from the CORS
    /// configuration at build time (ADR-031): `cors_allow_all_origins()` →
    /// `Disabled`, an explicit origin list → `AllowList`, no CORS config →
    /// the `SameOriginOrLoopback` default.
    pub fn origin_policy(mut self, policy: turul_http_mcp_server::OriginPolicy) -> Self {
        self.origin_policy = Some(policy);
        self
    }

    pub fn server_config(mut self, config: ServerConfig) -> Self {
        self.server_config = config;
        self
    }

    /// Configure streaming/SSE settings
    pub fn stream_config(mut self, config: StreamConfig) -> Self {
        self.stream_config = config;
        self
    }

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

    /// Build the Lambda MCP server.
    ///
    /// Validation, handlers and capabilities come from
    /// [`McpServerBuilder::build`]; its configuration errors are returned as
    /// [`LambdaError::Configuration`].
    pub async fn build(mut self) -> Result<LambdaMcpServer> {
        use turul_mcp_session_storage::InMemorySessionStorage;

        // Origin validation policy (ADR-031): an explicit `.origin_policy()`
        // wins; otherwise an explicit CORS configuration is the operator's
        // declaration of allowed origins and the policy follows it. With no
        // CORS config, the `SameOriginOrLoopback` default stands.
        if let Some(policy) = self.origin_policy.take() {
            self.server_config.origin_policy = policy;
        } else {
            #[cfg(feature = "cors")]
            if let Some(cors) = &self.cors_config {
                self.server_config.origin_policy = if cors.allowed_origins.iter().any(|o| o == "*")
                {
                    turul_http_mcp_server::OriginPolicy::Disabled
                } else {
                    turul_http_mcp_server::OriginPolicy::AllowList(cors.allowed_origins.clone())
                };
            }
        }

        let session_storage = self
            .session_storage
            .unwrap_or_else(|| Arc::new(InMemorySessionStorage::new()));
        let core = self
            .inner
            .with_session_storage(session_storage)
            .build()
            .map_err(|e| LambdaError::Configuration(e.to_string()))?;

        Ok(LambdaMcpServer::new(
            core,
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

    builder.sse(false).build().await
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
    use turul_mcp_builders::prelude::*;
    use turul_mcp_session_storage::InMemorySessionStorage; // HasBaseMetadata, HasDescription, etc.

    // Mock tool for testing
    #[derive(Clone, Default)]
    struct TestTool;

    impl HasBaseMetadata for TestTool {
        fn name(&self) -> &str {
            "test_tool"
        }
    }

    impl HasDescription for TestTool {
        fn description(&self) -> Option<&str> {
            Some("Test tool")
        }
    }

    impl HasInputSchema for TestTool {
        fn input_schema(&self) -> &turul_mcp_protocol::ToolSchema {
            use turul_mcp_protocol::ToolSchema;
            static SCHEMA: std::sync::OnceLock<ToolSchema> = std::sync::OnceLock::new();
            SCHEMA.get_or_init(ToolSchema::object)
        }
    }

    impl HasOutputSchema for TestTool {
        fn output_schema(&self) -> Option<&turul_mcp_protocol::ToolSchema> {
            None
        }
    }

    impl HasAnnotations for TestTool {
        fn annotations(&self) -> Option<&turul_mcp_protocol::tools::ToolAnnotations> {
            None
        }
    }

    impl HasToolMeta for TestTool {
        fn tool_meta(&self) -> Option<&std::collections::HashMap<String, serde_json::Value>> {
            None
        }
    }

    impl HasIcons for TestTool {}
    impl HasExecution for TestTool {}

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
            .sse(false) // Disable SSE for tests since streaming feature not enabled
            .build()
            .await
            .unwrap();

        // Create handler from server and verify it has stream_manager
        let handler = server.handler().await.unwrap();
        // Verify handler has stream_manager (critical invariant)
        assert!(
            handler.get_stream_manager().as_ref() as *const _ as usize > 0,
            "Stream manager must be initialized"
        );
    }

    #[tokio::test]
    async fn test_simple_lambda_server() {
        let tools = vec![TestTool];
        let server = simple_lambda_server(tools).await.unwrap();

        // Create handler and verify it was created with default configuration
        let handler = server.handler().await.unwrap();
        // Verify handler has stream_manager
        // Verify handler has stream_manager (critical invariant)
        assert!(
            handler.get_stream_manager().as_ref() as *const _ as usize > 0,
            "Stream manager must be initialized"
        );
    }

    #[tokio::test]
    async fn test_builder_extension_trait() {
        let tools = vec![TestTool, TestTool];

        let server = LambdaMcpServerBuilder::new()
            .tools(tools)
            .storage(Arc::new(InMemorySessionStorage::new()))
            .sse(false) // Disable SSE for tests since streaming feature not enabled
            .build()
            .await
            .unwrap();

        let handler = server.handler().await.unwrap();
        // Verify handler has stream_manager
        // Verify handler has stream_manager (critical invariant)
        assert!(
            handler.get_stream_manager().as_ref() as *const _ as usize > 0,
            "Stream manager must be initialized"
        );
    }

    // ── Registered-method parity with the non-Lambda McpServerBuilder gates ──
    //
    // Builds a server through the production path (builder → build() →
    // handler()) and asserts the dispatcher's registered method set matches
    // an explicit, spec-derived literal — not a diff against the non-Lambda
    // builder, which could share a bug.

    #[cfg(feature = "protocol-2026-07-28")]
    #[tokio::test]
    async fn test_registered_methods_parity_2026_07_28() {
        use std::collections::BTreeSet;

        let server = LambdaMcpServerBuilder::new()
            .name("parity-test")
            .version("1.0.0")
            .tool(TestTool)
            .storage(Arc::new(InMemorySessionStorage::new()))
            .sse(false)
            .build()
            .await
            .unwrap();
        let handler = server.handler().await.unwrap();

        let expected: BTreeSet<String> = [
            "tools/list",
            "tools/call",
            "server/discover",
            "resources/list",
            "resources/read",
            "resources/templates/list",
            "prompts/list",
            "prompts/get",
            "notifications/cancelled",
            "notifications/resources/list_changed",
            "notifications/resources/updated",
            "notifications/tools/list_changed",
            "notifications/prompts/list_changed",
            "notifications/resources/listChanged",
            "notifications/tools/listChanged",
            "notifications/prompts/listChanged",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let actual: BTreeSet<String> = handler.registered_methods().into_iter().collect();
        assert_eq!(actual, expected);
    }

    #[cfg(feature = "protocol-2025-11-25")]
    #[tokio::test]
    async fn test_registered_methods_parity_2025_11_25() {
        use std::collections::BTreeSet;

        let server = LambdaMcpServerBuilder::new()
            .name("parity-test")
            .version("1.0.0")
            .tool(TestTool)
            .storage(Arc::new(InMemorySessionStorage::new()))
            .sse(false)
            .build()
            .await
            .unwrap();
        let handler = server.handler().await.unwrap();

        let expected: BTreeSet<String> = [
            "initialize",
            "tools/list",
            "tools/call",
            "ping",
            "resources/list",
            "resources/read",
            "resources/templates/list",
            "prompts/list",
            "prompts/get",
            "logging/setLevel",
            "roots/list",
            "sampling/createMessage",
            "elicitation/create",
            "notifications/message",
            "notifications/progress",
            "notifications/cancelled",
            "notifications/resources/list_changed",
            "notifications/resources/updated",
            "notifications/tools/list_changed",
            "notifications/prompts/list_changed",
            "notifications/roots/list_changed",
            "notifications/resources/listChanged",
            "notifications/tools/listChanged",
            "notifications/prompts/listChanged",
            "notifications/roots/listChanged",
            "notifications/initialized",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let actual: BTreeSet<String> = handler.registered_methods().into_iter().collect();
        assert_eq!(actual, expected);
    }

    #[cfg(feature = "cors")]
    #[tokio::test]
    async fn test_cors_configuration() {
        let server = LambdaMcpServerBuilder::new()
            .cors_allow_all_origins()
            .storage(Arc::new(InMemorySessionStorage::new()))
            .sse(false) // Disable SSE for tests since streaming feature not enabled
            .build()
            .await
            .unwrap();

        let handler = server.handler().await.unwrap();
        // Verify handler has stream_manager (critical invariant — pre-existing smoke check)
        assert!(
            handler.get_stream_manager().as_ref() as *const _ as usize > 0,
            "Stream manager must be initialized"
        );
    }

    // ── Builder-path CORS propagation regressions ──
    //
    // `LambdaMcpServerBuilder::cors(...)` populates `LambdaMcpServer.cors_config`,
    // but for ~7 months `LambdaMcpServer::handler()` constructed the
    // `LambdaMcpHandler` via `with_middleware_and_fingerprint(...)` (which
    // initializes `cors_config: None`) and never chained `.with_cors(...)`
    // onto the result. Every `if let Some(ref cors_config) = self.cors_config`
    // branch inside the handler was therefore unreachable through the
    // documented builder entry point. The smoke test above didn't catch it
    // because it only asserts `stream_manager` exists.
    //
    // These tests probe the actual contract: configure CORS on the builder,
    // pull a real response off the handler, assert the headers appear.

    #[cfg(feature = "cors")]
    mod cors_propagation {
        use super::*;
        use async_trait::async_trait;
        use http::Request;
        use lambda_http::Body as LambdaBody;
        use turul_http_mcp_server::middleware::{
            DispatcherResult, McpMiddleware, MiddlewareError, RequestContext, SessionInjection,
        };
        use turul_mcp_session_storage::SessionView;

        fn preflight_request() -> lambda_http::Request {
            Request::builder()
                .method("OPTIONS")
                .uri("/mcp")
                .header("Origin", "https://client.example.test")
                .header("Access-Control-Request-Method", "POST")
                .body(LambdaBody::Empty)
                .unwrap()
        }

        fn post_request() -> lambda_http::Request {
            Request::builder()
                .method("POST")
                .uri("/mcp")
                .header("Content-Type", "application/json")
                .header("Accept", "application/json, text/event-stream")
                .header("MCP-Protocol-Version", "2025-11-25")
                .header("Origin", "https://client.example.test")
                .body(LambdaBody::Text(
                    r#"{"jsonrpc":"2.0","method":"initialize","id":1}"#.to_string(),
                ))
                .unwrap()
        }

        /// Builder-path streaming preflight must carry CORS headers.
        ///
        /// Repros the original drop-on-the-floor bug: before the fix,
        /// `server.handler().await.handle_streaming(OPTIONS)` returned a
        /// preflight response with no `Access-Control-Allow-Origin`.
        #[tokio::test]
        async fn builder_path_streaming_preflight_has_cors() {
            let server = LambdaMcpServerBuilder::new()
                .cors_allow_all_origins()
                .storage(Arc::new(InMemorySessionStorage::new()))
                .sse(false)
                .build()
                .await
                .unwrap();
            let handler = server.handler().await.unwrap();

            let resp = handler.handle_streaming(preflight_request()).await.unwrap();
            assert_eq!(resp.status(), 200);
            assert!(
                resp.headers().contains_key("access-control-allow-origin"),
                "builder-configured CORS must reach the streaming preflight response; \
                 got headers: {:?}",
                resp.headers(),
            );
            assert!(
                resp.headers().contains_key("access-control-allow-methods"),
                "preflight must advertise methods",
            );
        }

        /// CORS origin list derives the origin-validation AllowList
        /// (ADR-031): the listed origin reaches dispatch; an unlisted
        /// cross-origin request is rejected 403 before anything else runs.
        #[tokio::test]
        async fn cors_origin_list_derives_origin_allowlist() {
            let server = LambdaMcpServerBuilder::new()
                .cors_allow_origins(vec!["https://client.example.test".to_string()])
                .storage(Arc::new(InMemorySessionStorage::new()))
                .sse(false)
                .build()
                .await
                .unwrap();
            let handler = server.handler().await.unwrap();

            // Listed origin: passes the origin gate (reaches protocol handling).
            let resp = handler.handle_streaming(post_request()).await.unwrap();
            assert_ne!(
                resp.status(),
                403,
                "allowlisted origin must pass the origin gate"
            );

            // Unlisted origin: rejected by the origin gate.
            let req = Request::builder()
                .method("POST")
                .uri("/mcp")
                .header("Content-Type", "application/json")
                .header("Accept", "application/json, text/event-stream")
                .header("MCP-Protocol-Version", "2025-11-25")
                .header("Origin", "https://other.example.test")
                .body(LambdaBody::Text(
                    r#"{"jsonrpc":"2.0","method":"initialize","id":1}"#.to_string(),
                ))
                .unwrap();
            let resp = handler.handle_streaming(req).await.unwrap();
            assert_eq!(
                resp.status(),
                403,
                "unlisted cross-origin request must get 403 Forbidden"
            );
        }

        /// Non-preflight: a middleware-emitted 401 through the builder-path
        /// streaming handler must also carry CORS — this is the production
        /// failure mode that previously masquerade-ed as a CORS bug.
        #[tokio::test]
        async fn builder_path_streaming_401_has_cors_and_exposes_www_authenticate() {
            struct ForceChallenge;

            #[async_trait]
            impl McpMiddleware for ForceChallenge {
                fn runs_before_session(&self) -> bool {
                    true
                }
                async fn before_dispatch(
                    &self,
                    _ctx: &mut RequestContext<'_>,
                    _session: Option<&dyn SessionView>,
                    _injection: &mut SessionInjection,
                ) -> std::result::Result<(), MiddlewareError> {
                    Err(MiddlewareError::http_challenge(
                        401,
                        "Bearer realm=\"mcp\", \
                         resource_metadata=\"https://example.test/.well-known/oauth-protected-resource\"",
                    ))
                }
                async fn after_dispatch(
                    &self,
                    _ctx: &RequestContext<'_>,
                    _result: &mut DispatcherResult,
                ) -> std::result::Result<(), MiddlewareError> {
                    Ok(())
                }
            }

            let server = LambdaMcpServerBuilder::new()
                .cors_allow_all_origins()
                .middleware(Arc::new(ForceChallenge))
                .storage(Arc::new(InMemorySessionStorage::new()))
                .sse(false)
                .build()
                .await
                .unwrap();
            let handler = server.handler().await.unwrap();

            let resp = handler.handle_streaming(post_request()).await.unwrap();
            let headers = resp.headers();

            assert_eq!(resp.status(), 401);
            assert!(
                headers.contains_key("www-authenticate"),
                "WWW-Authenticate must survive the builder-path streaming transport",
            );
            assert!(
                headers.contains_key("access-control-allow-origin"),
                "401 must carry CORS through the builder path — was the source of the v0.3.40 production gap",
            );
            let expose = headers
                .get("access-control-expose-headers")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            assert!(
                expose
                    .split(',')
                    .map(str::trim)
                    .any(|h| h.eq_ignore_ascii_case("WWW-Authenticate")),
                "expose-headers must include WWW-Authenticate; got {expose:?}",
            );
        }

        /// Negative: builder with no CORS configured → no CORS headers.
        /// Guards against accidentally injecting defaults for consumers who
        /// opted out (or never opted in).
        #[tokio::test]
        async fn builder_path_without_cors_emits_no_cors_headers() {
            let server = LambdaMcpServerBuilder::new()
                .storage(Arc::new(InMemorySessionStorage::new()))
                .sse(false)
                .build()
                .await
                .unwrap();
            let handler = server.handler().await.unwrap();

            let resp = handler.handle_streaming(preflight_request()).await.unwrap();
            assert!(
                !resp.headers().contains_key("access-control-allow-origin"),
                "no builder CORS → no CORS headers; got {:?}",
                resp.headers(),
            );
        }
    }

    #[tokio::test]
    #[allow(deprecated)] // asserts the 2025-lane enable_get_sse plumbing
    async fn test_sse_toggle_functionality() {
        // Test that SSE can be toggled on/off/on correctly
        let mut builder =
            LambdaMcpServerBuilder::new().storage(Arc::new(InMemorySessionStorage::new()));

        // Initially enable SSE
        builder = builder.sse(true);
        assert!(builder.enable_sse, "SSE should be enabled");
        assert!(
            builder.server_config.enable_get_sse,
            "GET SSE endpoint should be enabled"
        );
        assert!(
            builder.server_config.enable_post_sse,
            "POST SSE endpoint should be enabled"
        );

        // Disable SSE
        builder = builder.sse(false);
        assert!(!builder.enable_sse, "SSE should be disabled");
        assert!(
            !builder.server_config.enable_get_sse,
            "GET SSE endpoint should be disabled"
        );
        assert!(
            !builder.server_config.enable_post_sse,
            "POST SSE endpoint should be disabled"
        );

        // Re-enable SSE (this was broken before the fix)
        builder = builder.sse(true);
        assert!(builder.enable_sse, "SSE should be re-enabled");
        assert!(
            builder.server_config.enable_get_sse,
            "GET SSE endpoint should be re-enabled"
        );
        assert!(
            builder.server_config.enable_post_sse,
            "POST SSE endpoint should be re-enabled"
        );

        // Verify the server can be built with SSE enabled
        let server = builder.build().await.unwrap();
        let handler = server.handler().await.unwrap();
        assert!(
            handler.get_stream_manager().as_ref() as *const _ as usize > 0,
            "Stream manager must be initialized"
        );
    }

    // =========================================================================
    // Task support tests
    // =========================================================================

    // Tasks moved to the turul-mcp-ext-tasks extension in 2026-07-28; the task
    // runtime/storage builder surface and the capabilities.tasks field are 2025-only.
    #[cfg(feature = "protocol-2025-11-25")]
    #[tokio::test]
    async fn test_builder_without_tasks_no_capability() {
        let server = LambdaMcpServerBuilder::new()
            .name("no-tasks")
            .tool(TestTool)
            .storage(Arc::new(InMemorySessionStorage::new()))
            .sse(false)
            .build()
            .await
            .unwrap();

        assert!(
            server.capabilities().tasks.is_none(),
            "Tasks capability should not be advertised without task storage"
        );
    }

    #[cfg(feature = "protocol-2025-11-25")]
    #[tokio::test]
    async fn test_builder_with_task_storage_advertises_capability() {
        use turul_mcp_server::task_storage::InMemoryTaskStorage;

        let server = LambdaMcpServerBuilder::new()
            .name("with-tasks")
            .tool(TestTool)
            .storage(Arc::new(InMemorySessionStorage::new()))
            .with_task_storage(Arc::new(InMemoryTaskStorage::new()))
            .sse(false)
            .build()
            .await
            .unwrap();

        let tasks_cap = server
            .capabilities()
            .tasks
            .as_ref()
            .expect("Tasks capability should be advertised");
        assert!(tasks_cap.list.is_some(), "list capability should be set");
        assert!(
            tasks_cap.cancel.is_some(),
            "cancel capability should be set"
        );
        let requests = tasks_cap
            .requests
            .as_ref()
            .expect("requests capability should be set");
        let tools = requests
            .tools
            .as_ref()
            .expect("tools capability should be set");
        assert!(tools.call.is_some(), "tools.call capability should be set");
    }

    #[cfg(feature = "protocol-2025-11-25")]
    #[tokio::test]
    async fn test_builder_with_task_runtime_advertises_capability() {
        let runtime = Arc::new(turul_mcp_server::TaskRuntime::in_memory());

        let server = LambdaMcpServerBuilder::new()
            .name("with-runtime")
            .tool(TestTool)
            .storage(Arc::new(InMemorySessionStorage::new()))
            .with_task_runtime(runtime)
            .sse(false)
            .build()
            .await
            .unwrap();

        assert!(
            server.capabilities().tasks.is_some(),
            "Tasks capability should be advertised with task runtime"
        );
    }

    #[cfg(feature = "protocol-2025-11-25")]
    #[tokio::test]
    async fn test_task_recovery_timeout_configuration() {
        use turul_mcp_server::task_storage::InMemoryTaskStorage;

        let server = LambdaMcpServerBuilder::new()
            .name("custom-timeout")
            .tool(TestTool)
            .storage(Arc::new(InMemorySessionStorage::new()))
            .task_recovery_timeout_ms(60_000)
            .with_task_storage(Arc::new(InMemoryTaskStorage::new()))
            .sse(false)
            .build()
            .await
            .unwrap();

        assert!(
            server.capabilities().tasks.is_some(),
            "Tasks should be enabled with custom timeout"
        );
    }

    #[cfg(feature = "protocol-2025-11-25")]
    #[tokio::test]
    async fn test_backward_compatibility_no_tasks() {
        // Existing builder pattern still works unchanged
        let server = LambdaMcpServerBuilder::new()
            .name("backward-compat")
            .version("1.0.0")
            .tool(TestTool)
            .storage(Arc::new(InMemorySessionStorage::new()))
            .sse(false)
            .build()
            .await
            .unwrap();

        let handler = server.handler().await.unwrap();
        assert!(
            handler.get_stream_manager().as_ref() as *const _ as usize > 0,
            "Stream manager must be initialized"
        );
        assert!(server.capabilities().tasks.is_none());
    }

    /// Slow tool that sleeps for 2 seconds — used to prove non-blocking behavior.
    /// Declares task support, which exists only in the 2025-11-25 spec.
    #[cfg(feature = "protocol-2025-11-25")]
    #[derive(Clone, Default)]
    struct SlowTool;

    #[cfg(feature = "protocol-2025-11-25")]
    impl HasBaseMetadata for SlowTool {
        fn name(&self) -> &str {
            "slow_tool"
        }
    }

    #[cfg(feature = "protocol-2025-11-25")]
    impl HasDescription for SlowTool {
        fn description(&self) -> Option<&str> {
            Some("A slow tool for testing")
        }
    }

    #[cfg(feature = "protocol-2025-11-25")]
    impl HasInputSchema for SlowTool {
        fn input_schema(&self) -> &turul_mcp_protocol::ToolSchema {
            use turul_mcp_protocol::ToolSchema;
            static SCHEMA: std::sync::OnceLock<ToolSchema> = std::sync::OnceLock::new();
            SCHEMA.get_or_init(ToolSchema::object)
        }
    }

    #[cfg(feature = "protocol-2025-11-25")]
    impl HasOutputSchema for SlowTool {
        fn output_schema(&self) -> Option<&turul_mcp_protocol::ToolSchema> {
            None
        }
    }

    #[cfg(feature = "protocol-2025-11-25")]
    impl HasAnnotations for SlowTool {
        fn annotations(&self) -> Option<&turul_mcp_protocol::tools::ToolAnnotations> {
            None
        }
    }

    #[cfg(feature = "protocol-2025-11-25")]
    impl HasToolMeta for SlowTool {
        fn tool_meta(&self) -> Option<&std::collections::HashMap<String, serde_json::Value>> {
            None
        }
    }

    #[cfg(feature = "protocol-2025-11-25")]
    impl HasIcons for SlowTool {}
    #[cfg(feature = "protocol-2025-11-25")]
    impl HasExecution for SlowTool {
        fn execution(&self) -> Option<turul_mcp_protocol::tools::ToolExecution> {
            Some(turul_mcp_protocol::tools::ToolExecution {
                task_support: Some(turul_mcp_protocol::tools::TaskSupport::Optional),
            })
        }
    }

    #[cfg(feature = "protocol-2025-11-25")]
    #[async_trait::async_trait]
    impl McpTool for SlowTool {
        async fn call(
            &self,
            _args: serde_json::Value,
            _session: Option<turul_mcp_server::SessionContext>,
        ) -> turul_mcp_server::McpResult<turul_mcp_protocol::tools::CallToolResult> {
            use turul_mcp_protocol::tools::{CallToolResult, ToolResult};
            // Sleep 2 seconds to prove the task path is non-blocking
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            Ok(CallToolResult::success(vec![ToolResult::text("slow done")]))
        }
    }

    #[cfg(feature = "protocol-2025-11-25")]
    #[tokio::test]
    async fn test_nonblocking_tools_call_with_task() {
        use turul_mcp_server::SessionAwareToolHandler;
        use turul_mcp_server::task_storage::InMemoryTaskStorage;
        use turul_rpc::r#async::JsonRpcHandler;

        let task_storage = Arc::new(InMemoryTaskStorage::new());
        let runtime = Arc::new(turul_mcp_server::TaskRuntime::with_default_executor(
            task_storage,
        ));

        // Build tools map
        let mut tools: HashMap<String, Arc<dyn McpTool>> = HashMap::new();
        tools.insert("slow_tool".to_string(), Arc::new(SlowTool));

        // Create session manager
        let session_storage: Arc<turul_mcp_session_storage::BoxedSessionStorage> =
            Arc::new(InMemorySessionStorage::new());
        let session_manager = Arc::new(turul_mcp_server::session::SessionManager::with_storage(
            session_storage,
            turul_mcp_protocol::ServerCapabilities::default(),
        ));

        // Create tool handler with task runtime
        let tool_handler = SessionAwareToolHandler::new(tools, session_manager, false)
            .with_task_runtime(Arc::clone(&runtime));

        // Build a tools/call request with task parameter
        let params = serde_json::json!({
            "name": "slow_tool",
            "arguments": {},
            "task": {}
        });
        let request_params = turul_rpc::RequestParams::Object(
            params
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        );

        // Time the call
        let start = std::time::Instant::now();
        let result = tool_handler
            .handle("tools/call", Some(request_params), None)
            .await;
        let elapsed = start.elapsed();

        // Should succeed with CreateTaskResult
        let value = result.expect("tools/call with task should succeed");
        assert!(
            value.get("task").is_some(),
            "Response should contain 'task' field (CreateTaskResult shape)"
        );
        let task = value.get("task").unwrap();
        assert!(
            task.get("taskId").is_some(),
            "Task should have taskId field"
        );
        assert_eq!(
            task.get("status")
                .and_then(|v| v.as_str())
                .unwrap_or_default(),
            "working",
            "Task status should be 'working'"
        );

        // Non-blocking proof: should return well under the 2s tool sleep.
        // Threshold is 1s (not 500ms) to avoid flakes on slow CI runners —
        // the 2s tool sleep vs 1s threshold still proves a clear 2x gap.
        assert!(
            elapsed < std::time::Duration::from_secs(1),
            "tools/call with task should return immediately (took {:?}, expected < 1s)",
            elapsed
        );
    }

    // =========================================================================
    // Resource and template resource tests
    // =========================================================================

    // Mock static resource for testing
    #[derive(Clone)]
    struct StaticTestResource;

    impl turul_mcp_builders::prelude::HasResourceMetadata for StaticTestResource {
        fn name(&self) -> &str {
            "static_test"
        }
    }

    impl turul_mcp_builders::prelude::HasResourceDescription for StaticTestResource {
        fn description(&self) -> Option<&str> {
            Some("Static test resource")
        }
    }

    impl turul_mcp_builders::prelude::HasResourceUri for StaticTestResource {
        fn uri(&self) -> &str {
            "file:///test.txt"
        }
    }

    impl turul_mcp_builders::prelude::HasResourceMimeType for StaticTestResource {
        fn mime_type(&self) -> Option<&str> {
            Some("text/plain")
        }
    }

    impl turul_mcp_builders::prelude::HasResourceSize for StaticTestResource {
        fn size(&self) -> Option<u64> {
            None
        }
    }

    impl turul_mcp_builders::prelude::HasResourceAnnotations for StaticTestResource {
        fn annotations(&self) -> Option<&turul_mcp_protocol::meta::Annotations> {
            None
        }
    }

    impl turul_mcp_builders::prelude::HasResourceMeta for StaticTestResource {
        fn resource_meta(&self) -> Option<&std::collections::HashMap<String, serde_json::Value>> {
            None
        }
    }

    impl HasIcons for StaticTestResource {}

    #[async_trait::async_trait]
    impl McpResource for StaticTestResource {
        async fn read(
            &self,
            _params: Option<serde_json::Value>,
            _session: Option<&turul_mcp_server::SessionContext>,
        ) -> turul_mcp_server::McpResult<Vec<turul_mcp_protocol::resources::ResourceContent>>
        {
            use turul_mcp_protocol::resources::ResourceContent;
            Ok(vec![ResourceContent::text("file:///test.txt", "test")])
        }
    }

    // Mock template resource for testing
    #[derive(Clone)]
    struct TemplateTestResource;

    impl turul_mcp_builders::prelude::HasResourceMetadata for TemplateTestResource {
        fn name(&self) -> &str {
            "template_test"
        }
    }

    impl turul_mcp_builders::prelude::HasResourceDescription for TemplateTestResource {
        fn description(&self) -> Option<&str> {
            Some("Template test resource")
        }
    }

    impl turul_mcp_builders::prelude::HasResourceUri for TemplateTestResource {
        fn uri(&self) -> &str {
            "agent://agents/{agent_id}"
        }
    }

    impl turul_mcp_builders::prelude::HasResourceMimeType for TemplateTestResource {
        fn mime_type(&self) -> Option<&str> {
            Some("application/json")
        }
    }

    impl turul_mcp_builders::prelude::HasResourceSize for TemplateTestResource {
        fn size(&self) -> Option<u64> {
            None
        }
    }

    impl turul_mcp_builders::prelude::HasResourceAnnotations for TemplateTestResource {
        fn annotations(&self) -> Option<&turul_mcp_protocol::meta::Annotations> {
            None
        }
    }

    impl turul_mcp_builders::prelude::HasResourceMeta for TemplateTestResource {
        fn resource_meta(&self) -> Option<&std::collections::HashMap<String, serde_json::Value>> {
            None
        }
    }

    impl HasIcons for TemplateTestResource {}

    #[async_trait::async_trait]
    impl McpResource for TemplateTestResource {
        async fn read(
            &self,
            _params: Option<serde_json::Value>,
            _session: Option<&turul_mcp_server::SessionContext>,
        ) -> turul_mcp_server::McpResult<Vec<turul_mcp_protocol::resources::ResourceContent>>
        {
            use turul_mcp_protocol::resources::ResourceContent;
            Ok(vec![ResourceContent::text("agent://agents/test", "{}")])
        }
    }

    /// Call one method on the built Lambda handler over the 2026-07-28 wire
    /// (ADR-035: resource handling is the owner's, so tests go through it).
    #[cfg(feature = "protocol-2026-07-28")]
    async fn wire_2026(builder: LambdaMcpServerBuilder, method: &str) -> serde_json::Value {
        use http_body_util::BodyExt;
        let handler = builder
            .storage(Arc::new(InMemorySessionStorage::new()))
            .sse(false)
            .build()
            .await
            .expect("build")
            .handler()
            .await
            .expect("handler");
        let body = serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": method,
            "params": { "_meta": {
                "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                "io.modelcontextprotocol/clientInfo": { "name": "t", "version": "1" },
                "io.modelcontextprotocol/clientCapabilities": {}
            } }
        });
        let req = http::Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("MCP-Protocol-Version", "2026-07-28")
            .header("Mcp-Method", method)
            .body(lambda_http::Body::from(body.to_string()))
            .unwrap();
        let resp = handler
            .handle_streaming(req)
            .await
            .expect("handle_streaming");
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice::<serde_json::Value>(&bytes).expect("JSON")["result"].clone()
    }

    #[cfg(feature = "protocol-2026-07-28")]
    #[tokio::test]
    async fn test_resource_auto_detection_static() {
        let builder = LambdaMcpServerBuilder::new()
            .name("test")
            .resource(StaticTestResource);
        let listed = wire_2026(builder, "resources/list").await;
        let resources = listed["resources"].as_array().unwrap();
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0]["uri"], "file:///test.txt");
        let builder = LambdaMcpServerBuilder::new()
            .name("test")
            .resource(StaticTestResource);
        let templates = wire_2026(builder, "resources/templates/list").await;
        assert_eq!(templates["resourceTemplates"].as_array().unwrap().len(), 0);
    }

    #[cfg(feature = "protocol-2026-07-28")]
    #[tokio::test]
    async fn test_resource_auto_detection_template() {
        let builder = LambdaMcpServerBuilder::new()
            .name("test")
            .resource(TemplateTestResource);
        let listed = wire_2026(builder, "resources/list").await;
        assert_eq!(listed["resources"].as_array().unwrap().len(), 0);
        let builder = LambdaMcpServerBuilder::new()
            .name("test")
            .resource(TemplateTestResource);
        let templates = wire_2026(builder, "resources/templates/list").await;
        let templates = templates["resourceTemplates"].as_array().unwrap();
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0]["uriTemplate"], "agent://agents/{agent_id}");
    }

    #[cfg(feature = "protocol-2026-07-28")]
    #[tokio::test]
    async fn test_resource_auto_detection_mixed() {
        let mixed = || {
            LambdaMcpServerBuilder::new()
                .name("test")
                .resource(StaticTestResource)
                .resource(TemplateTestResource)
        };
        let listed = wire_2026(mixed(), "resources/list").await;
        let resources = listed["resources"].as_array().unwrap();
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0]["uri"], "file:///test.txt");
        let templates = wire_2026(mixed(), "resources/templates/list").await;
        let templates = templates["resourceTemplates"].as_array().unwrap();
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0]["uriTemplate"], "agent://agents/{agent_id}");
    }

    #[tokio::test]
    async fn test_build_advertises_resources_capability_for_templates_only() {
        let server = LambdaMcpServerBuilder::new()
            .name("template-only")
            .resource(TemplateTestResource)
            .storage(Arc::new(InMemorySessionStorage::new()))
            .sse(false)
            .build()
            .await
            .unwrap();

        assert!(
            server.capabilities().resources.is_some(),
            "Resources capability should be advertised when template resources are registered"
        );
    }

    #[tokio::test]
    async fn test_build_advertises_resources_capability_for_static_only() {
        let server = LambdaMcpServerBuilder::new()
            .name("static-only")
            .resource(StaticTestResource)
            .storage(Arc::new(InMemorySessionStorage::new()))
            .sse(false)
            .build()
            .await
            .unwrap();

        assert!(
            server.capabilities().resources.is_some(),
            "Resources capability should be advertised when static resources are registered"
        );
    }

    #[tokio::test]
    async fn test_build_no_resources_no_capability() {
        let server = LambdaMcpServerBuilder::new()
            .name("no-resources")
            .tool(TestTool)
            .storage(Arc::new(InMemorySessionStorage::new()))
            .sse(false)
            .build()
            .await
            .unwrap();

        assert!(
            server.capabilities().resources.is_none(),
            "Resources capability should NOT be advertised when no resources are registered"
        );
    }

    #[cfg(feature = "protocol-2026-07-28")]
    #[tokio::test]
    async fn test_lambda_builder_templates_list_returns_template() {
        let builder = LambdaMcpServerBuilder::new()
            .name("template-test")
            .resource(TemplateTestResource);
        let result = wire_2026(builder, "resources/templates/list").await;

        let templates = result["resourceTemplates"]
            .as_array()
            .expect("resourceTemplates should be an array");
        assert_eq!(
            templates.len(),
            1,
            "Should have exactly 1 template resource"
        );
        assert_eq!(
            templates[0]["uriTemplate"], "agent://agents/{agent_id}",
            "Template URI should match"
        );
        assert_eq!(templates[0]["name"], "template_test");
    }

    #[cfg(feature = "protocol-2026-07-28")]
    #[tokio::test]
    async fn test_lambda_builder_resources_list_returns_static() {
        let builder = LambdaMcpServerBuilder::new()
            .name("static-test")
            .resource(StaticTestResource);
        let result = wire_2026(builder, "resources/list").await;

        let resources = result["resources"]
            .as_array()
            .expect("resources should be an array");
        assert_eq!(resources.len(), 1, "Should have exactly 1 static resource");
        assert_eq!(resources[0]["uri"], "file:///test.txt");
        assert_eq!(resources[0]["name"], "static_test");
    }

    #[cfg(feature = "protocol-2026-07-28")]
    #[tokio::test]
    async fn test_lambda_builder_mixed_resources_separation() {
        // Build with both static and template resources
        let mixed = || {
            LambdaMcpServerBuilder::new()
                .name("mixed-test")
                .resource(StaticTestResource)
                .resource(TemplateTestResource)
        };

        // resources/list should return only the static resource
        let list_result = wire_2026(mixed(), "resources/list").await;
        let resources = list_result["resources"]
            .as_array()
            .expect("resources should be an array");
        assert_eq!(resources.len(), 1, "Only static resource in resources/list");
        assert_eq!(resources[0]["uri"], "file:///test.txt");

        // resources/templates/list should return only the template resource
        let templates_result = wire_2026(mixed(), "resources/templates/list").await;
        let templates = templates_result["resourceTemplates"]
            .as_array()
            .expect("resourceTemplates should be an array");
        assert_eq!(
            templates.len(),
            1,
            "Only template resource in resources/templates/list"
        );
        assert_eq!(templates[0]["uriTemplate"], "agent://agents/{agent_id}");
    }

    #[cfg(feature = "protocol-2025-11-25")]
    #[tokio::test]
    async fn test_tasks_get_route_registered() {
        use turul_mcp_server::TasksGetHandler;
        use turul_mcp_server::handlers::McpHandler;
        use turul_mcp_server::task_storage::InMemoryTaskStorage;

        let runtime = Arc::new(turul_mcp_server::TaskRuntime::with_default_executor(
            Arc::new(InMemoryTaskStorage::new()),
        ));
        let handler = TasksGetHandler::new(runtime);

        // Dispatch tasks/get with a non-existent task_id — should return MCP error
        // (not "method not found"), proving the route is registered and responds
        let params = serde_json::json!({ "taskId": "nonexistent-task-id" });

        let result = handler.handle(Some(params)).await;

        // Should be an error (task not found) — NOT a "method not found" error
        assert!(
            result.is_err(),
            "tasks/get with unknown task should return error"
        );
        let err = result.unwrap_err();
        let err_str = err.to_string();
        assert!(
            !err_str.contains("method not found"),
            "Error should not be 'method not found' — handler should respond to tasks/get"
        );
    }

    // ── Handler registration parity tests ─────────────────────────────

    /// Verify resources/read is registered by default even with no resources.
    /// HTTP server registers it unconditionally — Lambda must match.
    /// We test by sending a resources/read request through handle() and
    /// verifying we get an MCP error (not "method not found").
    // Drives the 2025-11-25 initialize handshake to obtain an Mcp-Session-Id;
    // the 2026-07-28 stateless core has no initialize and returns no session id.
    #[cfg(feature = "protocol-2025-11-25")]
    #[tokio::test]
    async fn test_resources_read_registered_by_default() {
        use lambda_http::Body as LambdaBody;

        let server = LambdaMcpServerBuilder::new()
            .name("parity-test")
            .version("1.0.0")
            .tool(TestTool) // tools only, no resources
            .storage(Arc::new(InMemorySessionStorage::new()))
            .strict_lifecycle(false) // skip handshake for this test
            .sse(false)
            .build()
            .await
            .unwrap();

        let handler = server.handler().await.unwrap();

        // Initialize to get session
        let init_req = http::Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("Content-Type", "application/json")
            .header("MCP-Protocol-Version", "2025-11-25")
            .body(LambdaBody::Text(
                serde_json::json!({
                    "jsonrpc": "2.0", "method": "initialize", "id": 1,
                    "params": {
                        "protocolVersion": "2025-11-25",
                        "capabilities": {},
                        "clientInfo": { "name": "test", "version": "1.0.0" }
                    }
                })
                .to_string(),
            ))
            .unwrap();
        let init_resp = handler.handle(init_req).await.unwrap();
        let session_id = init_resp
            .headers()
            .get("Mcp-Session-Id")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        // Send resources/read — should get a JSON-RPC error (handler registered),
        // NOT a "method not found" error (handler missing)
        let read_req = http::Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("Content-Type", "application/json")
            .header("MCP-Protocol-Version", "2025-11-25")
            .header("Mcp-Session-Id", &session_id)
            .body(LambdaBody::Text(
                serde_json::json!({
                    "jsonrpc": "2.0", "method": "resources/read", "id": 2,
                    "params": { "uri": "file:///nonexistent" }
                })
                .to_string(),
            ))
            .unwrap();
        let read_resp = handler.handle(read_req).await.unwrap();
        let body = String::from_utf8_lossy(read_resp.body().as_ref()).to_string();
        let json: serde_json::Value = serde_json::from_str(&body)
            .unwrap_or_else(|e| panic!("Response must be valid JSON: {e}\nBody: {body}"));

        // Must be a JSON-RPC error response with an error object
        assert!(
            json["error"].is_object(),
            "resources/read must return JSON-RPC error, got: {json}"
        );
        // The error code must NOT be -32601 (method not found) — that would mean
        // the handler isn't registered. Any other error code (e.g., resource not found)
        // proves the handler IS registered and executed.
        let error_code = json["error"]["code"].as_i64().unwrap();
        assert_ne!(
            error_code, -32601,
            "resources/read must be registered (got method-not-found -32601): {json}"
        );
    }

    /// resources/templates/list is registered unconditionally, matching the
    /// local builder: a server with no templates reports an empty list.
    #[cfg(feature = "protocol-2025-11-25")]
    #[tokio::test]
    async fn test_resources_templates_list_answers_empty_without_templates() {
        use lambda_http::Body as LambdaBody;

        let server = LambdaMcpServerBuilder::new()
            .name("parity-test")
            .version("1.0.0")
            .tool(TestTool) // tools only, no templates
            .storage(Arc::new(InMemorySessionStorage::new()))
            .strict_lifecycle(false) // skip handshake for this test
            .sse(false)
            .build()
            .await
            .unwrap();

        let handler = server.handler().await.unwrap();

        // Initialize to get session
        let init_req = http::Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("Content-Type", "application/json")
            .header("MCP-Protocol-Version", "2025-11-25")
            .body(LambdaBody::Text(
                serde_json::json!({
                    "jsonrpc": "2.0", "method": "initialize", "id": 1,
                    "params": {
                        "protocolVersion": "2025-11-25",
                        "capabilities": {},
                        "clientInfo": { "name": "test", "version": "1.0.0" }
                    }
                })
                .to_string(),
            ))
            .unwrap();
        let init_resp = handler.handle(init_req).await.unwrap();
        let session_id = init_resp
            .headers()
            .get("Mcp-Session-Id")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        // resources/templates/list with no templates registered
        let tmpl_req = http::Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("Content-Type", "application/json")
            .header("MCP-Protocol-Version", "2025-11-25")
            .header("Mcp-Session-Id", &session_id)
            .body(LambdaBody::Text(
                serde_json::json!({
                    "jsonrpc": "2.0", "method": "resources/templates/list", "id": 2
                })
                .to_string(),
            ))
            .unwrap();
        let tmpl_resp = handler.handle(tmpl_req).await.unwrap();
        let body = String::from_utf8_lossy(tmpl_resp.body().as_ref()).to_string();
        let json: serde_json::Value = serde_json::from_str(&body)
            .unwrap_or_else(|e| panic!("Response must be valid JSON: {e}\nBody: {body}"));

        // An empty list, not -32601. A server that declares the resources
        // capability and answers "method not found" tells a client the method
        // does not exist, which is a different claim from "there are none" —
        // and it is the one a capability-driven client acts on.
        assert!(
            json.get("error").is_none(),
            "resources/templates/list must not error without templates: {json}"
        );
        assert_eq!(
            json["result"]["resourceTemplates"],
            serde_json::json!([]),
            "a server with no templates reports an empty list: {json}"
        );
    }
}
