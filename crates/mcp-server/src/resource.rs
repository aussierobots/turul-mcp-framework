//! MCP Resource Trait
//!
//! This module defines the high-level trait for implementing MCP resources.

use async_trait::async_trait;
use mcp_protocol::{McpResult, resources::ResourceContent};
use serde_json::Value;


/// High-level trait for implementing MCP resources
#[async_trait]
pub trait McpResource: Send + Sync {
    /// The URI identifier for this resource
    fn uri(&self) -> &str;

    /// Human-readable name
    fn name(&self) -> &str;

    /// Description of the resource
    fn description(&self) -> &str;

    /// Optional MIME type hint
    fn mime_type(&self) -> Option<&str> {
        None
    }

    /// Optional annotations for client hints
    fn annotations(&self) -> Option<Value> {
        None
    }

    /// Read the resource content
    /// 
    /// The params parameter can contain read-specific parameters like file paths,
    /// query filters, or other resource-specific options.
    async fn read(&self, params: Option<Value>) -> McpResult<Vec<ResourceContent>>;

    /// Optional: Check if resource supports subscriptions for real-time updates
    fn supports_subscriptions(&self) -> bool {
        false
    }

    /// Optional: Subscribe to resource changes
    /// 
    /// Resources that support real-time updates can override this method.
    /// By default, returns a "method not found" error.
    async fn subscribe(&self, _params: Option<Value>) -> McpResult<()> {
        Err(mcp_protocol::McpError::tool_execution("Resource does not support subscriptions"))
    }

    /// Optional: Unsubscribe from resource changes
    async fn unsubscribe(&self, _params: Option<Value>) -> McpResult<()> {
        Err(mcp_protocol::McpError::tool_execution("Resource does not support subscriptions"))
    }
}

/// Convert an McpResource trait object to a Resource descriptor
pub fn resource_to_descriptor(resource: &dyn McpResource) -> mcp_protocol::resources::Resource {
    let mut mcp_resource = mcp_protocol::resources::Resource::new(resource.uri(), resource.name())
        .with_description(resource.description());
        
    if let Some(mime_type) = resource.mime_type() {
        mcp_resource = mcp_resource.with_mime_type(mime_type);
    }
    
    if let Some(annotations) = resource.annotations() {
        mcp_resource = mcp_resource.with_annotations(annotations);
    }
    
    mcp_resource
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct TestResource {
        uri: String,
        name: String,
        content: String,
    }

    #[async_trait]
    impl McpResource for TestResource {
        fn uri(&self) -> &str {
            &self.uri
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            "A test resource"
        }

        fn mime_type(&self) -> Option<&str> {
            Some("text/plain")
        }

        async fn read(&self, _params: Option<Value>) -> McpResult<Vec<ResourceContent>> {
            Ok(vec![ResourceContent::text(&self.content)])
        }
    }

    #[test]
    fn test_resource_trait() {
        let resource = TestResource {
            uri: "test://example".to_string(),
            name: "Test Resource".to_string(),
            content: "Test content".to_string(),
        };
        
        assert_eq!(resource.uri(), "test://example");
        assert_eq!(resource.name(), "Test Resource");
        assert_eq!(resource.description(), "A test resource");
        assert_eq!(resource.mime_type(), Some("text/plain"));
        assert!(!resource.supports_subscriptions());
    }

    #[test]
    fn test_resource_conversion() {
        let resource = TestResource {
            uri: "test://example".to_string(),
            name: "Test Resource".to_string(),
            content: "Test content".to_string(),
        };
        
        let descriptor = resource_to_descriptor(&resource);
        
        assert_eq!(descriptor.uri, "test://example");
        assert_eq!(descriptor.name, "Test Resource");
        assert_eq!(descriptor.description, Some("A test resource".to_string()));
        assert_eq!(descriptor.mime_type, Some("text/plain".to_string()));
    }

    #[tokio::test]
    async fn test_resource_read() {
        let resource = TestResource {
            uri: "test://example".to_string(),
            name: "Test Resource".to_string(),
            content: "Hello, world!".to_string(),
        };
        
        let result = resource.read(None).await.unwrap();
        assert_eq!(result.len(), 1);
        
        if let ResourceContent::Text { text } = &result[0] {
            assert_eq!(text, "Hello, world!");
        } else {
            panic!("Expected text content");
        }
    }

    #[tokio::test]
    async fn test_resource_subscribe_default() {
        let resource = TestResource {
            uri: "test://example".to_string(),
            name: "Test Resource".to_string(),
            content: "Test content".to_string(),
        };
        
        let result = resource.subscribe(None).await;
        assert!(result.is_err());
        
        if let Err(mcp_protocol::McpError::ToolExecutionError(message)) = result {
            assert!(message.contains("subscriptions"));
        } else {
            panic!("Expected ToolExecutionError");
        }
    }
}