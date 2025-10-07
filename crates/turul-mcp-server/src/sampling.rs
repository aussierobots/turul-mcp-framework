//! MCP Sampling Trait
//!
//! This module defines the high-level trait for implementing MCP sampling.

use async_trait::async_trait;
use turul_mcp_builders::prelude::*;
use turul_mcp_protocol::{
    McpResult,
    sampling::{CreateMessageRequest, CreateMessageResult},
};

/// High-level trait for implementing MCP sampling
///
/// McpSampling extends SamplingDefinition with execution capabilities.
/// All metadata is provided by the SamplingDefinition trait, ensuring
/// consistency between concrete Sampling structs and dynamic implementations.
#[async_trait]
pub trait McpSampling: SamplingDefinition + Send + Sync {
    /// Create a message using the sampling model (per MCP spec)
    ///
    /// This method processes the sampling/createMessage request and returns
    /// the generated message response.
    async fn sample(&self, request: CreateMessageRequest) -> McpResult<CreateMessageResult>;

    /// Optional: Check if this sampling handler can handle the given request
    ///
    /// This allows for conditional sampling based on model preferences,
    /// context size, or other factors.
    fn can_handle(&self, _request: &CreateMessageRequest) -> bool {
        true
    }

    /// Optional: Get sampling priority for request routing
    ///
    /// Higher priority handlers are tried first when multiple handlers
    /// can handle the same request.
    fn priority(&self) -> u32 {
        0
    }

    /// Optional: Validate the sampling request
    ///
    /// This method can perform additional validation beyond basic parameter checks.
    async fn validate_request(&self, request: &CreateMessageRequest) -> McpResult<()> {
        // Basic validation - ensure max_tokens is reasonable
        if request.params.max_tokens == 0 {
            return Err(turul_mcp_protocol::McpError::validation(
                "max_tokens must be greater than 0",
            ));
        }
        if request.params.max_tokens > 1000000 {
            return Err(turul_mcp_protocol::McpError::validation(
                "max_tokens exceeds maximum allowed value",
            ));
        }
        Ok(())
    }
}

/// Convert an McpSampling trait object to CreateMessageParams
///
/// This is a convenience function for converting sampling definitions
/// to protocol parameters.
pub fn sampling_to_params(
    sampling: &dyn McpSampling,
) -> turul_mcp_protocol::sampling::CreateMessageParams {
    sampling.to_create_params()
}

#[cfg(test)]
mod tests {
    use super::*;
    use turul_mcp_protocol::sampling::SamplingMessage;
    use turul_mcp_builders::prelude::*;  // HasSamplingConfig, HasSamplingContext, etc.

    struct TestSampling {
        messages: Vec<SamplingMessage>,
        max_tokens: u32,
        temperature: Option<f64>,
    }

    // Implement fine-grained traits (MCP spec compliant)
    impl HasSamplingConfig for TestSampling {
        fn max_tokens(&self) -> u32 {
            self.max_tokens
        }

        fn temperature(&self) -> Option<f64> {
            self.temperature
        }
    }

    impl HasSamplingContext for TestSampling {
        fn messages(&self) -> &[SamplingMessage] {
            &self.messages
        }
    }

    impl HasModelPreferences for TestSampling {}

    // SamplingDefinition automatically implemented via blanket impl!

    #[async_trait]
    impl McpSampling for TestSampling {
        async fn sample(&self, _request: CreateMessageRequest) -> McpResult<CreateMessageResult> {
            // Simulate message generation
            let response_message = SamplingMessage {
                role: turul_mcp_protocol::sampling::Role::Assistant,
                content: turul_mcp_protocol::prompts::ContentBlock::Text {
                    text: "Generated response".to_string(),
                    annotations: None,
                    meta: None,
                },
            };

            Ok(CreateMessageResult::new(response_message, "test-model"))
        }
    }

    #[test]
    fn test_sampling_trait() {
        let sampling = TestSampling {
            messages: vec![],
            max_tokens: 100,
            temperature: Some(0.7),
        };

        assert_eq!(sampling.max_tokens(), 100);
        assert_eq!(sampling.temperature(), Some(0.7));
    }

    #[tokio::test]
    async fn test_sampling_validation() {
        let sampling = TestSampling {
            messages: vec![],
            max_tokens: 0,
            temperature: None,
        };

        let params = sampling.to_create_params();
        let request = CreateMessageRequest {
            method: "sampling/createMessage".to_string(),
            params,
        };

        let result = sampling.validate_request(&request).await;
        assert!(result.is_err());
    }
}
