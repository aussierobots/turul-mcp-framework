//! Integration tests for URI template functionality

#[cfg(test)]
mod tests {
    use crate::{McpResult, McpServer};
    use crate::handlers::McpHandler;
    use crate::uri_template::{UriTemplate, VariableValidator};
    use async_trait::async_trait;
    use serde_json::{Value, json};
    use std::collections::HashMap;
    use turul_mcp_protocol::meta;
    use turul_mcp_protocol::resources::ResourceContent;
    use turul_mcp_builders::prelude::*;  // HasResourceMetadata, HasResourceDescription, etc.

    /// Test resource that supports URI templates
    #[derive(Clone)]
    struct UserProfileResource {
        base_pattern: String,
    }

    impl UserProfileResource {
        fn new() -> Self {
            Self {
                base_pattern: "file:///user/{user_id}.json".to_string(),
            }
        }
    }

    // Implement fine-grained traits for ResourceDefinition
    impl HasResourceMetadata for UserProfileResource {
        fn name(&self) -> &str {
            "user_profile"
        }
    }

    impl HasResourceUri for UserProfileResource {
        fn uri(&self) -> &str {
            &self.base_pattern
        }
    }

    impl HasResourceDescription for UserProfileResource {
        fn description(&self) -> Option<&str> {
            Some("User profile data with dynamic user ID")
        }
    }

    impl HasResourceMimeType for UserProfileResource {
        fn mime_type(&self) -> Option<&str> {
            Some("application/json")
        }
    }

    impl HasResourceSize for UserProfileResource {
        fn size(&self) -> Option<u64> {
            None // Dynamic size
        }
    }

    impl HasResourceAnnotations for UserProfileResource {
        fn annotations(&self) -> Option<&meta::Annotations> {
            None
        }
    }

    impl HasResourceMeta for UserProfileResource {
        fn resource_meta(&self) -> Option<&HashMap<String, Value>> {
            None
        }
    }

    impl HasIcons for UserProfileResource {}

    // ResourceDefinition automatically implemented via blanket impl

    #[async_trait]
    impl crate::McpResource for UserProfileResource {
        async fn read(&self, params: Option<Value>, _session: Option<&crate::SessionContext>) -> McpResult<Vec<ResourceContent>> {
            // Extract template variables from params
            let params = params.unwrap_or(json!({}));

            if let Some(template_vars) = params.get("template_variables")
                && let Some(user_id) = template_vars.get("user_id").and_then(|v| v.as_str())
            {
                // Generate dynamic content based on user_id
                let user_data = json!({
                    "user_id": user_id,
                    "name": format!("User {}", user_id),
                    "email": format!("{}@example.com", user_id),
                    "profile": {
                        "created": "2024-01-01",
                        "active": true
                    }
                });

                let uri = format!("file:///user/{}.json", user_id);
                return Ok(vec![ResourceContent::text(&uri, user_data.to_string())]);
            }

            // Fallback for static access
            Ok(vec![ResourceContent::text(
                &self.base_pattern,
                r#"{"error": "Template variables required"}"#,
            )])
        }
    }

    #[tokio::test]
    async fn test_uri_template_resource_handler_integration() {
        use crate::handlers::ResourcesReadHandler;

        // Create URI template with validation
        let template = UriTemplate::new("file:///user/{user_id}.json")
            .unwrap()
            .with_validator("user_id", VariableValidator::user_id());

        // Create resource
        let resource = UserProfileResource::new();

        // Create handler with template resource (disable security for testing)
        let read_handler = ResourcesReadHandler::new()
            .without_security()
            .add_template_resource(template, resource);

        // Test with dynamic URI
        let read_params = json!({
            "uri": "file:///user/alice123.json"
        });

        let result = read_handler.handle(Some(read_params)).await.unwrap();

        // Verify the response structure
        assert!(result.is_object());

        let contents = result.get("contents").unwrap().as_array().unwrap();
        assert_eq!(contents.len(), 1);

        let content = &contents[0];
        assert_eq!(
            content.get("mimeType").unwrap().as_str().unwrap(),
            "text/plain"
        );
        assert_eq!(
            content.get("uri").unwrap().as_str().unwrap(),
            "file:///user/alice123.json"
        );

        let text = content.get("text").unwrap().as_str().unwrap();
        let parsed: Value = serde_json::from_str(text).unwrap();

        // Verify dynamic content was generated
        assert_eq!(parsed.get("user_id").unwrap().as_str().unwrap(), "alice123");
        assert_eq!(
            parsed.get("name").unwrap().as_str().unwrap(),
            "User alice123"
        );
        assert_eq!(
            parsed.get("email").unwrap().as_str().unwrap(),
            "alice123@example.com"
        );
    }

    #[tokio::test]
    async fn test_template_validation() {
        use crate::handlers::ResourcesReadHandler;

        let template = UriTemplate::new("file:///user/{user_id}.json")
            .unwrap()
            .with_validator("user_id", VariableValidator::user_id());

        let resource = UserProfileResource::new();

        let read_handler = ResourcesReadHandler::new()
            .without_security()
            .add_template_resource(template, resource);

        // Test with invalid user_id (contains @)
        let read_params = json!({
            "uri": "file:///user/invalid@user.json"
        });

        let result = read_handler.handle(Some(read_params)).await;

        // Should return error due to validation failure
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_template_fallback_to_exact_uri() {
        use crate::handlers::ResourcesReadHandler;

        let template = UriTemplate::new("file:///user/{user_id}.json")
            .unwrap()
            .with_validator("user_id", VariableValidator::user_id());

        let template_resource = UserProfileResource::new();

        // Also add a static resource
        let static_resource = UserProfileResource {
            base_pattern: "file:///static.json".to_string(),
        };

        let read_handler = ResourcesReadHandler::new()
            .without_security()
            .add_template_resource(template, template_resource)
            .add_resource(static_resource);

        // Test static resource access (should fall back to exact matching)
        let read_params = json!({
            "uri": "file:///static.json"
        });

        let result = read_handler.handle(Some(read_params)).await.unwrap();

        // Should succeed without template processing
        assert!(result.is_object());
        assert!(result.get("contents").is_some());
    }

    #[test]
    fn test_multiple_variable_template() {
        let template = UriTemplate::new("file:///user/{user_id}/avatar.{format}")
            .unwrap()
            .with_validator("user_id", VariableValidator::user_id())
            .with_validator("format", VariableValidator::image_format());

        // Test successful extraction
        let vars = template
            .extract("file:///user/alice123/avatar.png")
            .unwrap();
        assert_eq!(vars.get("user_id"), Some(&"alice123".to_string()));
        assert_eq!(vars.get("format"), Some(&"png".to_string()));

        // Test resolution
        let mut test_vars = HashMap::new();
        test_vars.insert("user_id".to_string(), "bob456".to_string());
        test_vars.insert("format".to_string(), "jpg".to_string());

        let resolved = template.resolve(&test_vars).unwrap();
        assert_eq!(resolved, "file:///user/bob456/avatar.jpg");
    }

    #[test]
    fn test_mime_type_detection() {
        let json_template = UriTemplate::new("file:///data/{id}.json").unwrap();
        assert_eq!(json_template.mime_type(), Some("application/json"));

        let image_template = UriTemplate::new("file:///images/{id}.{format}").unwrap();
        assert_eq!(image_template.mime_type(), None); // Variable extension

        let pdf_template = UriTemplate::new("file:///docs/{id}.pdf").unwrap();
        assert_eq!(pdf_template.mime_type(), Some("application/pdf"));
    }

    #[test]
    fn test_builder_template_validation_error_collection() {
        // Test that builder collects validation errors instead of panicking
        let valid_template = UriTemplate::new("file:///valid/{id}.json").unwrap();
        let invalid_template_pattern = "invalid template pattern {"; // Missing closing brace

        // This should succeed - valid template
        let builder = McpServer::builder()
            .name("test")
            .version("1.0.0")
            .bind_address("127.0.0.1:0".parse().unwrap())
            .template_resource(valid_template, UserProfileResource::new());

        // This should collect an error but not panic - invalid template
        let invalid_template = match UriTemplate::new(invalid_template_pattern) {
            Ok(t) => t,
            Err(_) => {
                // Template creation itself fails - test passes (this is expected)
                return;
            }
        };

        let builder_with_invalid =
            builder.template_resource(invalid_template, UserProfileResource::new());
        let result = builder_with_invalid.build();

        // Should fail with validation error, not panic
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("validation errors")
                || error_msg.contains("Invalid resource template")
        );
    }
}
