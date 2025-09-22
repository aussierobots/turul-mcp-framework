//! MCP Completion Trait
//!
//! This module defines the high-level trait for implementing MCP completion.

use async_trait::async_trait;
use turul_mcp_protocol::completion::CompletionDefinition;
use turul_mcp_protocol::{
    McpResult,
    completion::{CompleteRequest, CompleteResult},
};

/// High-level trait for implementing MCP completion
///
/// McpCompletion extends CompletionDefinition with execution capabilities.
/// All metadata is provided by the CompletionDefinition trait, ensuring
/// consistency between concrete Completion structs and dynamic implementations.
#[async_trait]
pub trait McpCompletion: CompletionDefinition + Send + Sync {
    /// Provide completion suggestions (per MCP spec)
    ///
    /// This method processes the completion/complete request and returns
    /// completion values that match the current input.
    async fn complete(&self, request: CompleteRequest) -> McpResult<CompleteResult>;

    /// Optional: Check if this completion handler can handle the given request
    ///
    /// This allows for conditional completion based on reference type,
    /// argument name, or other factors.
    fn can_handle(&self, _request: &CompleteRequest) -> bool {
        true
    }

    /// Optional: Get completion priority for request routing
    ///
    /// Higher priority handlers are tried first when multiple handlers
    /// can handle the same request.
    fn priority(&self) -> u32 {
        0
    }

    /// Optional: Validate the completion request
    ///
    /// This method can perform additional validation beyond basic parameter checks.
    async fn validate_request(&self, request: &CompleteRequest) -> McpResult<()> {
        use turul_mcp_protocol::completion::CompletionReference;

        // Basic validation - ensure reference and argument are present
        match &request.params.reference {
            CompletionReference::ResourceTemplate(template_ref) => {
                if template_ref.uri.is_empty() {
                    return Err(turul_mcp_protocol::McpError::validation(
                        "Resource template URI cannot be empty",
                    ));
                }
            }
            CompletionReference::Prompt(prompt_ref) => {
                if prompt_ref.name.is_empty() {
                    return Err(turul_mcp_protocol::McpError::validation(
                        "Prompt name cannot be empty",
                    ));
                }
            }
        }

        if request.params.argument.name.is_empty() {
            return Err(turul_mcp_protocol::McpError::validation(
                "Argument name cannot be empty",
            ));
        }

        Ok(())
    }

    /// Optional: Get maximum number of completions to return
    ///
    /// This helps limit response size for large completion sets.
    fn max_completions(&self) -> Option<usize> {
        Some(100) // Default reasonable limit
    }
}

/// Convert an McpCompletion trait object to a CompleteRequest
///
/// This is a convenience function for converting completion definitions
/// to protocol requests.
pub fn completion_to_request(completion: &dyn McpCompletion) -> CompleteRequest {
    completion.to_complete_request()
}

#[cfg(test)]
mod tests {
    use super::*;
    use turul_mcp_protocol::completion::{
        CompleteArgument, CompletionReference, CompletionResult, HasCompletionContext,
        HasCompletionHandling, HasCompletionMetadata,
    };

    struct TestCompletion {
        reference: CompletionReference,
        argument: CompleteArgument,
    }

    // Implement fine-grained traits (MCP spec compliant)
    impl HasCompletionMetadata for TestCompletion {
        fn method(&self) -> &str {
            "completion/complete"
        }

        fn reference(&self) -> &CompletionReference {
            &self.reference
        }
    }

    impl HasCompletionContext for TestCompletion {
        fn argument(&self) -> &CompleteArgument {
            &self.argument
        }
    }

    impl HasCompletionHandling for TestCompletion {}

    // CompletionDefinition automatically implemented via blanket impl!

    #[async_trait]
    impl McpCompletion for TestCompletion {
        async fn complete(&self, _request: CompleteRequest) -> McpResult<CompleteResult> {
            // Simulate completion generation
            let completion_result = CompletionResult::new(vec![
                "suggestion1".to_string(),
                "suggestion2".to_string(),
                "suggestion3".to_string(),
            ]);

            Ok(CompleteResult::new(completion_result))
        }
    }

    #[test]
    fn test_completion_trait() {
        let completion = TestCompletion {
            reference: CompletionReference::prompt("test-prompt"),
            argument: CompleteArgument::new("param", "test"),
        };

        assert_eq!(completion.method(), "completion/complete");
        assert_eq!(completion.argument().name, "param");
        assert_eq!(completion.argument().value, "test");
    }

    #[tokio::test]
    async fn test_completion_validation() {
        let completion = TestCompletion {
            reference: CompletionReference::resource("file:///tmp/test.txt"),
            argument: CompleteArgument::new("filename", ""),
        };

        let request = completion.to_complete_request();
        let result = McpCompletion::validate_request(&completion, &request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_empty_reference_validation() {
        let completion = TestCompletion {
            reference: CompletionReference::resource(""),
            argument: CompleteArgument::new("param", "value"),
        };

        let request = completion.to_complete_request();
        let result = McpCompletion::validate_request(&completion, &request).await;
        assert!(result.is_err());
    }
}
