//! Framework traits for MCP completion construction.
//!
//! These are framework features, not part of the MCP specification.

use turul_mcp_protocol::completion::{
    CompleteArgument, CompleteRequest, CompletionContext, CompletionReference,
};

/// Completion metadata: the method name and the reference being completed.
pub trait HasCompletionMetadata {
    /// Returns the completion method name.
    fn method(&self) -> &str;

    /// Returns the reference being completed (prompt or resource).
    fn reference(&self) -> &CompletionReference;
}

/// Completion context: the argument being completed and optional extra context.
pub trait HasCompletionContext {
    /// Returns the argument being completed.
    fn argument(&self) -> &CompleteArgument;

    /// Returns the optional completion context; the default is `None`.
    fn context(&self) -> Option<&CompletionContext> {
        None
    }
}

/// Completion request validation and value filtering.
pub trait HasCompletionHandling {
    /// Validates the completion request; the default accepts every request.
    fn validate_request(&self, _request: &CompleteRequest) -> Result<(), String> {
        Ok(())
    }

    /// Keeps the values that start with the current input, compared case-insensitively.
    fn filter_completions(&self, values: Vec<String>, current_value: &str) -> Vec<String> {
        values
            .into_iter()
            .filter(|v| v.to_lowercase().starts_with(&current_value.to_lowercase()))
            .collect()
    }
}

/// Complete MCP completion definition.
///
/// This trait represents a complete, working MCP completion provider.
/// When you implement the required traits, you automatically get
/// `CompletionDefinition` for free via blanket implementation.
pub trait CompletionDefinition:
    HasCompletionMetadata + HasCompletionContext + HasCompletionHandling
{
    /// Converts this completion definition to a protocol `CompleteRequest`.
    #[cfg(feature = "protocol-2025-11-25")]
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
