//! Framework traits for MCP completion construction
//!
//! **IMPORTANT**: These are framework features, NOT part of the MCP specification.

use turul_mcp_protocol::completion::{
    CompleteArgument, CompleteRequest, CompletionContext, CompletionReference,
};

/// Trait for completion metadata (method, reference)
pub trait HasCompletionMetadata {
    /// The completion method name
    fn method(&self) -> &str;

    /// The reference being completed (prompt or resource)
    fn reference(&self) -> &CompletionReference;
}

/// Trait for completion context (argument, context)
pub trait HasCompletionContext {
    /// The argument being completed
    fn argument(&self) -> &CompleteArgument;

    /// Optional completion context
    fn context(&self) -> Option<&CompletionContext> {
        None
    }
}

/// Trait for completion validation and processing
pub trait HasCompletionHandling {
    /// Validate the completion request
    fn validate_request(&self, _request: &CompleteRequest) -> Result<(), String> {
        Ok(())
    }

    /// Filter completion values based on current input
    fn filter_completions(&self, values: Vec<String>, current_value: &str) -> Vec<String> {
        // Default: simple prefix matching
        values
            .into_iter()
            .filter(|v| v.to_lowercase().starts_with(&current_value.to_lowercase()))
            .collect()
    }
}

/// Complete MCP Completion Definition trait
///
/// This trait represents a complete, working MCP completion provider.
/// When you implement the required traits, you automatically get
/// `CompletionDefinition` for free via blanket implementation.
pub trait CompletionDefinition:
    HasCompletionMetadata + HasCompletionContext + HasCompletionHandling
{
    /// Convert this completion definition to a protocol CompleteRequest
    fn to_complete_request(&self) -> CompleteRequest {
        let mut request = CompleteRequest::new(self.reference().clone(), self.argument().clone());
        if let Some(context) = self.context() {
            request = request.with_context(context.clone());
        }
        request
    }
}

// Blanket implementation: any type implementing the fine-grained traits automatically gets CompletionDefinition
impl<T> CompletionDefinition for T where
    T: HasCompletionMetadata + HasCompletionContext + HasCompletionHandling
{
}
