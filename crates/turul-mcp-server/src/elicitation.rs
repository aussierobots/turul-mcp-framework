//! MCP Elicitation Trait
//!
//! This module defines the high-level trait for implementing MCP elicitation.

use async_trait::async_trait;
use turul_mcp_builders::prelude::*;
use turul_mcp_protocol::{
    McpResult,
    elicitation::{ElicitCreateRequest, ElicitResult},
};

/// High-level trait for implementing MCP elicitation
///
/// McpElicitation extends ElicitationDefinition with execution capabilities.
/// All metadata is provided by the ElicitationDefinition trait, ensuring
/// consistency between concrete Elicitation structs and dynamic implementations.
#[async_trait]
pub trait McpElicitation: ElicitationDefinition + Send + Sync {
    /// Handle the elicitation request (per MCP spec)
    ///
    /// This method processes the elicitation/create request and returns the user's action and content.
    /// The implementation should present the elicitation to the user interface and
    /// wait for user response (accept, decline, or cancel).
    async fn elicit(&self, request: ElicitCreateRequest) -> McpResult<ElicitResult>;

    /// Check if this elicitation handler can handle the given request
    ///
    /// This allows for conditional elicitation handling based on request content,
    /// schema complexity, or other factors.
    fn can_handle(&self, _request: &ElicitCreateRequest) -> bool {
        true
    }

    /// Optional: Get elicitation priority for request routing
    ///
    /// Higher priority handlers are tried first when multiple handlers
    /// can handle the same request.
    fn priority(&self) -> u32 {
        0
    }

    /// Optional: Validate the elicitation result (per MCP spec)
    ///
    /// This method can perform additional validation beyond schema validation.
    async fn validate_result(&self, result: &ElicitResult) -> McpResult<()> {
        use turul_mcp_protocol::elicitation::ElicitAction;

        if matches!(result.action, ElicitAction::Accept) && result.content.is_none() {
            return Err(turul_mcp_protocol::McpError::validation(
                "Result marked as accept but contains no content",
            ));
        }
        Ok(())
    }

    /// Optional: Transform result data before returning
    ///
    /// This allows for post-processing of user input, such as formatting,
    /// normalization, or additional data enrichment.
    async fn transform_result(&self, result: ElicitResult) -> McpResult<ElicitResult> {
        Ok(result)
    }
}

/// Convert an McpElicitation trait object to an ElicitCreateRequest
///
/// This is a convenience function for converting elicitation definitions
/// to protocol requests (per MCP spec).
pub fn elicitation_to_create_request(elicitation: &dyn McpElicitation) -> ElicitCreateRequest {
    elicitation.to_create_request()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;
    use turul_mcp_protocol::elicitation::{ElicitationSchema, PrimitiveSchemaDefinition};
    use turul_mcp_builders::prelude::*;  // HasElicitationMetadata, etc.

    struct TestElicitation {
        message: String,
        schema: ElicitationSchema,
    }

    // Implement fine-grained traits (MCP spec compliant)
    impl HasElicitationMetadata for TestElicitation {
        fn message(&self) -> &str {
            &self.message
        }
    }

    impl HasElicitationSchema for TestElicitation {
        fn requested_schema(&self) -> &ElicitationSchema {
            &self.schema
        }
    }

    impl HasElicitationHandling for TestElicitation {}

    // ElicitationDefinition automatically implemented via blanket impl!

    #[async_trait]
    impl McpElicitation for TestElicitation {
        async fn elicit(&self, _request: ElicitCreateRequest) -> McpResult<ElicitResult> {
            // Simulate user input collection (MCP spec compliant)
            let mut content = HashMap::new();
            content.insert("name".to_string(), json!("Test User"));
            content.insert("email".to_string(), json!("test@example.com"));

            Ok(ElicitResult::accept(content))
        }
    }

    #[test]
    fn test_elicitation_trait() {
        let schema = ElicitationSchema::new().with_property(
            "name".to_string(),
            PrimitiveSchemaDefinition::string_with_description("Enter your name"),
        );

        let elicitation = TestElicitation {
            message: "Please provide your information".to_string(),
            schema,
        };

        assert_eq!(elicitation.message(), "Please provide your information");
    }
}
