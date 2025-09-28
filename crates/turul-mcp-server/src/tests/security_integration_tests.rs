//! Security Integration Tests
//!
//! Tests for MCP security middleware integration.

use crate::handlers::{McpHandler, ResourcesReadHandler};
use crate::resource::McpResource;
use crate::security::{ResourceAccessControl, SecurityMiddleware};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::Arc;
use turul_mcp_protocol::resources::ResourceContent;

// Simple test resource
#[derive(Clone)]
struct SimpleTestResource {
    content: String,
}

impl SimpleTestResource {
    fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
        }
    }
}

#[async_trait]
impl crate::McpResource for SimpleTestResource {
    async fn read(&self, _params: Option<Value>, _session: Option<&crate::SessionContext>) -> crate::McpResult<Vec<ResourceContent>> {
        Ok(vec![ResourceContent::text(
            "file:///tmp/test.txt",
            &self.content,
        )])
    }
}

// Required trait implementations
use turul_mcp_protocol::resources::{
    HasResourceAnnotations, HasResourceDescription, HasResourceMeta, HasResourceMetadata,
    HasResourceMimeType, HasResourceSize, HasResourceUri,
};

impl HasResourceMetadata for SimpleTestResource {
    fn name(&self) -> &str {
        "simple_test"
    }
}

impl HasResourceDescription for SimpleTestResource {
    fn description(&self) -> Option<&str> {
        Some("Simple test resource")
    }
}

impl HasResourceUri for SimpleTestResource {
    fn uri(&self) -> &str {
        "file:///tmp/test.txt"
    }
}

impl HasResourceMimeType for SimpleTestResource {}
impl HasResourceSize for SimpleTestResource {}
impl HasResourceAnnotations for SimpleTestResource {}
impl HasResourceMeta for SimpleTestResource {}

#[tokio::test]
async fn test_security_middleware_setup() {
    // Create security middleware (just test that it can be created)
    let access_control = ResourceAccessControl::default();
    let security_middleware =
        SecurityMiddleware::new().with_resource_access_control(access_control);

    // Create handler with security (just test that it can be created)
    let resource = SimpleTestResource::new("Small test content");
    let _handler = ResourcesReadHandler::new()
        .with_security(Arc::new(security_middleware))
        .add_resource(resource);

    // Test passes if handler can be created with security middleware
}

#[tokio::test]
async fn test_security_middleware_validates_parameters() {
    // Create restrictive security middleware
    let security_middleware = SecurityMiddleware::default();

    let resource = SimpleTestResource::new("Test content");
    let handler = ResourcesReadHandler::new()
        .with_security(Arc::new(security_middleware))
        .add_resource(resource);

    // Test missing URI parameter
    let result = handler.handle(None).await;
    assert!(result.is_err(), "Missing parameters should be rejected");

    // Test empty parameters
    let empty_params = json!({});
    let result = handler.handle(Some(empty_params)).await;
    assert!(result.is_err(), "Empty parameters should be rejected");
}

#[tokio::test]
async fn test_handler_without_security() {
    // Test that handler works without security middleware
    let resource = SimpleTestResource::new("Unsecured content");
    let handler = ResourcesReadHandler::new()
        .without_security()
        .add_resource(resource);

    let params = json!({
        "uri": "file:///tmp/test.txt"
    });

    let result = handler.handle(Some(params)).await;
    assert!(
        result.is_ok(),
        "Handler should work without security middleware"
    );
}
