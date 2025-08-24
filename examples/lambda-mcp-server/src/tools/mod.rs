//! MCP Tools Module
//!
//! Comprehensive tool suite for AWS Lambda MCP server with real-time notifications

pub mod aws_tools;
pub mod lambda_tools;
pub mod session_tools;

use async_trait::async_trait;
use lambda_runtime::Context as LambdaContext;
use mcp_protocol::{ToolResult, ToolSchema};
use mcp_server::{McpTool, SessionContext, McpResult};
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, info};

/// Tool execution context with Lambda and session information
#[derive(Debug, Clone)]
pub struct ToolExecutionContext {
    /// MCP session ID
    pub session_id: String,
    /// Lambda execution context
    pub lambda_context: LambdaEventContext,
    /// Request correlation ID
    pub request_id: String,
}

/// Lambda event context wrapper
#[derive(Debug, Clone)]
pub struct LambdaEventContext {
    /// AWS Lambda context
    pub lambda_context: LambdaContext,
    /// Optional session ID from headers
    pub session_id: Option<String>,
}

impl LambdaEventContext {
    /// Create new Lambda event context
    pub fn new(lambda_context: LambdaContext) -> Self {
        Self {
            lambda_context,
            session_id: None,
        }
    }

    /// Create with session ID
    pub fn with_session_id(lambda_context: LambdaContext, session_id: String) -> Self {
        Self {
            lambda_context,
            session_id: Some(session_id),
        }
    }

    /// Get session ID
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// Get request ID
    pub fn request_id(&self) -> &str {
        &self.lambda_context.request_id
    }

    /// Get function name
    pub fn function_name(&self) -> &str {
        &self.lambda_context.env_config.function_name
    }

    /// Get remaining time in milliseconds
    pub fn remaining_time_millis(&self) -> i64 {
        // Note: lambda_http::Context doesn't have get_remaining_time_in_millis
        // Return a default value for now
        300000 // 5 minutes in milliseconds
    }
}

impl From<LambdaContext> for LambdaEventContext {
    fn from(context: LambdaContext) -> Self {
        Self::new(context)
    }
}

/// Tool registry for managing available MCP tools
pub struct ToolRegistry {
    /// Registered tools by name
    tools: HashMap<String, Box<dyn McpTool + Send + Sync>>,
}

impl ToolRegistry {
    /// Create new tool registry with all available tools
    pub async fn new() -> Result<Self, lambda_runtime::Error> {
        let mut registry = Self {
            tools: HashMap::new(),
        };

        // Register AWS tools
        registry.register_tool(Box::new(aws_tools::AwsRealTimeMonitor)).await?;

        // Register Lambda tools  
        registry.register_tool(Box::new(lambda_tools::LambdaDiagnostics)).await?;

        // Register session tools
        registry.register_tool(Box::new(session_tools::SessionInfo)).await?;
        registry.register_tool(Box::new(session_tools::ListActiveSessions)).await?;
        registry.register_tool(Box::new(session_tools::SessionCleanup)).await?;
        registry.register_tool(Box::new(session_tools::ServerNotificationTool::default())).await?;
        registry.register_tool(Box::new(session_tools::ProgressUpdateTool::default())).await?;

        info!("Registered {} tools", registry.tools.len());
        Ok(registry)
    }

    /// Create tool registry for testing
    #[cfg(test)]
    pub async fn new_for_test() -> Result<Self, lambda_runtime::Error> {
        // For tests, register minimal tools
        let mut registry = Self {
            tools: HashMap::new(),
        };

        registry.register_tool(Box::new(session_tools::SessionInfo)).await?;
        Ok(registry)
    }

    /// Register a new tool
    async fn register_tool(&mut self, tool: Box<dyn McpTool + Send + Sync>) -> Result<(), lambda_runtime::Error> {
        let name = tool.name().to_string();
        debug!("Registering tool: {}", name);
        self.tools.insert(name, tool);
        Ok(())
    }

    /// List all available tools
    pub async fn list_tools(&self) -> Result<Vec<Value>, lambda_runtime::Error> {
        let mut tools = Vec::new();

        for tool in self.tools.values() {
            let tool_def = serde_json::json!({
                "name": tool.name(),
                "description": tool.description(),
                "inputSchema": tool.input_schema()
            });
            tools.push(tool_def);
        }

        debug!("Listed {} tools", tools.len());
        Ok(tools)
    }

    /// Execute a tool by name
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        arguments: Value,
        context: ToolExecutionContext,
    ) -> Result<ToolResult, lambda_runtime::Error> {
        debug!("Executing tool: {} with context: {:?}", tool_name, context);

        let tool = self.tools.get(tool_name)
            .ok_or_else(|| lambda_runtime::Error::from(format!("Tool not found: {}", tool_name)))?;

        // Create session context (limited in Lambda environment)
        let session_context = None; // SessionContext creation will be handled by mcp-server framework

        // Execute the tool
        match tool.call(arguments, session_context).await {
            Ok(results) => {
                if let Some(result) = results.into_iter().next() {
                    debug!("Tool {} executed successfully", tool_name);
                    Ok(result)
                } else {
                    Ok(ToolResult::text("Tool executed but returned no results"))
                }
            }
            Err(e) => {
                let error_msg = format!("Tool execution failed: {:?}", e);
                debug!("{}", error_msg);
                Err(lambda_runtime::Error::from(error_msg))
            }
        }
    }

    /// Get tool count
    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }

    /// Check if tool exists
    pub fn has_tool(&self, tool_name: &str) -> bool {
        self.tools.contains_key(tool_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lambda_event_context() {
        let lambda_ctx = LambdaContext::default();
        let event_ctx = LambdaEventContext::new(lambda_ctx);
        
        assert!(event_ctx.session_id().is_none());
        assert!(!event_ctx.request_id().is_empty());
    }

    #[tokio::test]
    async fn test_tool_registry_creation() {
        let registry = ToolRegistry::new_for_test().await.unwrap();
        assert!(registry.tool_count() > 0);
        assert!(registry.has_tool("session_info"));
    }
}