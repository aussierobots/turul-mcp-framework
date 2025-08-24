//! MCP Server Builder
//!
//! This module provides a builder pattern for creating MCP servers.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use crate::{McpTool, McpServer, Result, McpFrameworkError};
use crate::resource::McpResource;
use crate::handlers::*;
use mcp_protocol::{Implementation, ServerCapabilities};
use mcp_protocol::initialize::*;



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
    
    /// Prompts registered with the server
    prompts: HashMap<String, Arc<dyn McpPrompt>>,
    
    /// Handlers registered with the server
    handlers: HashMap<String, Arc<dyn McpHandler>>,
    
    /// Roots configured for the server
    roots: Vec<mcp_protocol::roots::Root>,
    
    /// Optional instructions for clients
    instructions: Option<String>,
    
    /// Session configuration
    session_timeout_minutes: Option<u64>,
    session_cleanup_interval_seconds: Option<u64>,
    
    /// HTTP configuration (if enabled)
    #[cfg(feature = "http")]
    bind_address: SocketAddr,
    #[cfg(feature = "http")]
    mcp_path: String,
    #[cfg(feature = "http")]
    enable_cors: bool,
    #[cfg(feature = "http")]
    enable_sse: bool,
}

impl McpServerBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        let tools = HashMap::new();
        let mut handlers: HashMap<String, Arc<dyn McpHandler>> = HashMap::new();
        
        // Add all standard MCP 2025-06-18 handlers by default
        handlers.insert("ping".to_string(), Arc::new(PingHandler));
        handlers.insert("completion/complete".to_string(), Arc::new(CompletionHandler));
        handlers.insert("resources/list".to_string(), Arc::new(ResourcesHandler::new()));
        handlers.insert("prompts/list".to_string(), Arc::new(PromptsHandler::new()));
        handlers.insert("prompts/get".to_string(), Arc::new(PromptsHandler::new()));
        handlers.insert("logging/setLevel".to_string(), Arc::new(LoggingHandler));
        handlers.insert("roots/list".to_string(), Arc::new(RootsHandler::new()));
        handlers.insert("sampling/createMessage".to_string(), Arc::new(SamplingHandler));
        handlers.insert("resources/templates/list".to_string(), Arc::new(ResourceTemplatesHandler));
        handlers.insert("elicitation/create".to_string(), Arc::new(ElicitationHandler::with_mock_provider()));
        
        // Add all notification handlers
        let notifications_handler = Arc::new(NotificationsHandler);
        handlers.insert("notifications/message".to_string(), notifications_handler.clone());
        handlers.insert("notifications/initialized".to_string(), notifications_handler.clone());
        handlers.insert("notifications/progress".to_string(), notifications_handler.clone());
        handlers.insert("notifications/resources/listChanged".to_string(), notifications_handler.clone());
        handlers.insert("notifications/resources/updated".to_string(), notifications_handler.clone());
        handlers.insert("notifications/tools/listChanged".to_string(), notifications_handler.clone());
        handlers.insert("notifications/prompts/listChanged".to_string(), notifications_handler.clone());
        handlers.insert("notifications/roots/listChanged".to_string(), notifications_handler);
        
        
        
        Self {
            name: "mcp-server".to_string(),
            version: "1.0.0".to_string(),
            title: None,
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapabilities {
                    list_changed: Some(false),
                }),
                ..Default::default()
            },
            tools,
            resources: HashMap::new(),
            prompts: HashMap::new(),
            handlers,
            roots: Vec::new(),
            instructions: None,
            session_timeout_minutes: None,
            session_cleanup_interval_seconds: None,
            #[cfg(feature = "http")]
            bind_address: "127.0.0.1:8000".parse().unwrap(),
            #[cfg(feature = "http")]
            mcp_path: "/mcp".to_string(),
            #[cfg(feature = "http")]
            enable_cors: true,
            #[cfg(feature = "http")]
            enable_sse: cfg!(feature = "sse"),
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
    pub fn resources<R: McpResource + 'static, I: IntoIterator<Item = R>>(mut self, resources: I) -> Self {
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
    pub fn prompts<P: McpPrompt + 'static, I: IntoIterator<Item = P>>(mut self, prompts: I) -> Self {
        for prompt in prompts {
            self = self.prompt(prompt);
        }
        self
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
        
        // Create PromptsHandler and add all registered prompts
        let mut handler = PromptsHandler::new();
        for (_, prompt) in &self.prompts {
            handler = handler.add_prompt_arc(prompt.clone());
        }
        
        self.handler(handler)
    }

    /// Add resources support
    pub fn with_resources(mut self) -> Self {
        self.capabilities.resources = Some(ResourcesCapabilities {
            subscribe: Some(false),
            list_changed: Some(false),
        });
        
        // Create ResourcesHandler and add all registered resources
        let mut handler = ResourcesHandler::new();
        for (_, resource) in &self.resources {
            handler = handler.add_resource_arc(resource.clone());
        }
        
        self.handler(handler)
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
    pub fn root(mut self, root: mcp_protocol::roots::Root) -> Self {
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

    /// Add templates support
    pub fn with_templates(self) -> Self {
        self.handler(TemplatesHandler::new())
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
    pub fn build(self) -> Result<McpServer> {
        // Validate configuration
        if self.name.is_empty() {
            return Err(McpFrameworkError::Config("Server name cannot be empty".to_string()));
        }
        if self.version.is_empty() {
            return Err(McpFrameworkError::Config("Server version cannot be empty".to_string()));
        }

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

        // Create server
        Ok(McpServer::new(
            implementation,
            self.capabilities,
            self.tools,
            handlers,
            self.instructions,
            self.session_timeout_minutes,
            self.session_cleanup_interval_seconds,
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
    use mcp_protocol::{ToolSchema, ToolResult};
    use serde_json::Value;

    struct TestTool;

    #[async_trait]
    impl McpTool for TestTool {
        fn name(&self) -> &str { "test" }
        fn description(&self) -> &str { "Test tool" }
        fn input_schema(&self) -> ToolSchema { ToolSchema::object() }
        async fn call(&self, _args: Value, _session: Option<SessionContext>) -> crate::McpResult<Vec<ToolResult>> {
            Ok(vec![ToolResult::text("test")])
        }
    }

    #[test]
    fn test_builder_defaults() {
        let builder = McpServerBuilder::new();
        assert_eq!(builder.name, "mcp-server");
        assert_eq!(builder.version, "1.0.0");
        assert!(builder.title.is_none());
        assert!(builder.instructions.is_none());
        assert!(builder.tools.is_empty());
        assert_eq!(builder.handlers.len(), 18); // Default MCP 2025-06-18 handlers
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
            .tool(TestTool)
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