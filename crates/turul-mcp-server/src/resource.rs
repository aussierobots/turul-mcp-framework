//! MCP Resource Trait
//!
//! This module defines the high-level trait for implementing MCP resources.

use async_trait::async_trait;
use serde_json::Value;
use turul_mcp_builders::prelude::*;
use turul_mcp_protocol::{McpResult, resources::ResourceContent};
use crate::SessionContext;

/// High-level trait for implementing MCP resources
///
/// McpResource extends ResourceDefinition with execution capabilities.
/// All metadata is provided by the ResourceDefinition trait, ensuring
/// consistency between concrete Resource structs and dynamic implementations.
#[async_trait]
pub trait McpResource: ResourceDefinition + Send + Sync {
    /// Read the resource content
    ///
    /// The params parameter can contain read-specific parameters like file paths,
    /// query filters, or other resource-specific options. The session parameter
    /// provides access to session-specific data and state for personalized content.
    async fn read(&self, params: Option<Value>, session: Option<&SessionContext>) -> McpResult<Vec<ResourceContent>>;

    /// Optional: Subscribe to resource changes
    ///
    /// Resources that support real-time updates can override this method.
    /// By default, returns a "method not found" error.
    async fn subscribe(&self, _params: Option<Value>) -> McpResult<()> {
        Err(turul_mcp_protocol::McpError::tool_execution(
            "Resource does not support subscriptions",
        ))
    }

    /// Optional: Unsubscribe from resource changes
    async fn unsubscribe(&self, _params: Option<Value>) -> McpResult<()> {
        Err(turul_mcp_protocol::McpError::tool_execution(
            "Resource does not support subscriptions",
        ))
    }
}

/// Converts an McpResource trait object to a protocol Resource descriptor
///
/// This is now a thin wrapper around the ResourceDefinition::to_resource() method
/// for backward compatibility. New code should use resource.to_resource() directly.
pub fn resource_to_descriptor(
    resource: &dyn McpResource,
) -> turul_mcp_protocol::resources::Resource {
    resource.to_resource()
}

#[cfg(test)]
mod tests {
    use super::*;
    use turul_mcp_protocol::meta;
      // HasResourceMetadata, HasResourceDescription, etc.

    struct TestResource {
        uri: String,
        name: String,
        content: String,
    }

    // Implement fine-grained traits
    impl HasResourceMetadata for TestResource {
        fn name(&self) -> &str {
            &self.name
        }
    }

    impl HasResourceUri for TestResource {
        fn uri(&self) -> &str {
            &self.uri
        }
    }

    impl HasResourceDescription for TestResource {
        fn description(&self) -> Option<&str> {
            Some("A test resource")
        }
    }

    impl HasResourceMimeType for TestResource {
        fn mime_type(&self) -> Option<&str> {
            Some("text/plain")
        }
    }

    impl HasResourceSize for TestResource {
        fn size(&self) -> Option<u64> {
            Some(self.content.len() as u64)
        }
    }

    impl HasResourceMeta for TestResource {
        fn resource_meta(&self) -> Option<&std::collections::HashMap<String, Value>> {
            None
        }
    }

    impl HasResourceAnnotations for TestResource {
        fn annotations(&self) -> Option<&meta::Annotations> {
            None
        }
    }

    impl HasIcons for TestResource {}

    // ResourceDefinition automatically implemented via blanket impl!

    #[async_trait]
    impl McpResource for TestResource {
        async fn read(&self, _params: Option<Value>, _session: Option<&SessionContext>) -> McpResult<Vec<ResourceContent>> {
            Ok(vec![ResourceContent::text(&self.uri, &self.content)])
        }
    }

    #[tokio::test]
    async fn test_resource_trait() {
        let resource = TestResource {
            uri: "test://example".to_string(),
            name: "Test Resource".to_string(),
            content: "Test content".to_string(),
        };

        assert_eq!(resource.uri(), "test://example");
        assert_eq!(resource.name(), "Test Resource");
        assert_eq!(resource.description(), Some("A test resource"));
        assert_eq!(resource.mime_type(), Some("text/plain"));
        // Test that default subscription methods return errors
        let subscribe_result = resource.subscribe(None).await;
        assert!(subscribe_result.is_err());
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

        let result = resource.read(None, None).await.unwrap();
        assert_eq!(result.len(), 1);

        let ResourceContent::Text(text_content) = &result[0] else {
            panic!("Expected text content, got: {:?}", result[0]);
        };
        assert_eq!(text_content.text, "Hello, world!");
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

        let Err(turul_mcp_protocol::McpError::ToolExecutionError(message)) = result else {
            panic!("Expected ToolExecutionError, got: {:?}", result);
        };
        assert!(message.contains("subscriptions"));
    }
}
