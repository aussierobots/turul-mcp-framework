//! Completion Builder for Runtime Completion Request Construction
//!
//! This module provides a builder pattern for creating completion requests at runtime
//! for autocomplete functionality in prompts and resources.

use std::collections::HashMap;
use serde_json::Value;

// Import from protocol via alias
use turul_mcp_protocol::completion::{
    CompleteRequest, CompleteParams, CompleteArgument, CompletionReference,
    CompletionContext, PromptReference, ResourceTemplateReference
};

/// Builder for creating completion requests at runtime
pub struct CompletionBuilder {
    reference: CompletionReference,
    argument: CompleteArgument,
    context: Option<CompletionContext>,
    meta: Option<HashMap<String, Value>>,
}

impl CompletionBuilder {
    /// Create a new completion builder for a prompt reference
    pub fn for_prompt(name: impl Into<String>) -> Self {
        Self {
            reference: CompletionReference::Prompt(PromptReference::new(name)),
            argument: CompleteArgument::new("", ""), // Will be set later
            context: None,
            meta: None,
        }
    }

    /// Create a new completion builder for a resource template reference  
    pub fn for_resource(uri: impl Into<String>) -> Self {
        Self {
            reference: CompletionReference::ResourceTemplate(ResourceTemplateReference::new(uri)),
            argument: CompleteArgument::new("", ""), // Will be set later
            context: None,
            meta: None,
        }
    }

    /// Set the argument being completed
    pub fn argument(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.argument = CompleteArgument::new(name, value);
        self
    }

    /// Set the argument name being completed
    pub fn argument_name(mut self, name: impl Into<String>) -> Self {
        self.argument.name = name.into();
        self
    }

    /// Set the current value being completed
    pub fn current_value(mut self, value: impl Into<String>) -> Self {
        self.argument.value = value.into();
        self
    }

    /// Set completion context
    pub fn context(mut self, context: CompletionContext) -> Self {
        self.context = Some(context);
        self
    }

    /// Set completion context with existing arguments
    pub fn with_context_arguments(mut self, arguments: HashMap<String, String>) -> Self {
        let mut context = CompletionContext::new();
        context.arguments = Some(arguments);
        self.context = Some(context);
        self
    }

    /// Add a context argument
    pub fn context_argument(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let context = self.context.get_or_insert_with(CompletionContext::new);
        let arguments = context.arguments.get_or_insert_with(HashMap::new);
        arguments.insert(key.into(), value.into());
        self
    }

    /// Set meta information
    pub fn meta(mut self, meta: HashMap<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }

    /// Build the completion request
    pub fn build(self) -> CompleteRequest {
        let mut params = CompleteParams::new(self.reference, self.argument);
        
        if let Some(context) = self.context {
            params = params.with_context(context);
        }
        
        if let Some(meta) = self.meta {
            params = params.with_meta(meta);
        }

        CompleteRequest {
            method: "completion/complete".to_string(),
            params,
        }
    }

    /// Build just the params (without the full request wrapper)
    pub fn build_params(self) -> CompleteParams {
        let mut params = CompleteParams::new(self.reference, self.argument);
        
        if let Some(context) = self.context {
            params = params.with_context(context);
        }
        
        if let Some(meta) = self.meta {
            params = params.with_meta(meta);
        }

        params
    }
}

/// Convenience methods for common completion scenarios
impl CompletionBuilder {
    /// Create completion for a prompt argument with no current value
    pub fn prompt_argument(prompt_name: impl Into<String>, arg_name: impl Into<String>) -> Self {
        Self::for_prompt(prompt_name)
            .argument_name(arg_name)
            .current_value("")
    }

    /// Create completion for a prompt argument with partial value
    pub fn prompt_argument_partial(
        prompt_name: impl Into<String>, 
        arg_name: impl Into<String>,
        partial_value: impl Into<String>
    ) -> Self {
        Self::for_prompt(prompt_name)
            .argument_name(arg_name)
            .current_value(partial_value)
    }

    /// Create completion for a resource template parameter
    pub fn resource_parameter(
        uri_template: impl Into<String>,
        param_name: impl Into<String>
    ) -> Self {
        Self::for_resource(uri_template)
            .argument_name(param_name)
            .current_value("")
    }

    /// Create completion for a resource template parameter with partial value
    pub fn resource_parameter_partial(
        uri_template: impl Into<String>,
        param_name: impl Into<String>,
        partial_value: impl Into<String>
    ) -> Self {
        Self::for_resource(uri_template)
            .argument_name(param_name)
            .current_value(partial_value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_completion_builder_prompt() {
        let request = CompletionBuilder::for_prompt("greeting_prompt")
            .argument("name", "Al")
            .build();

        assert_eq!(request.method, "completion/complete");
        
        match &request.params.reference {
            CompletionReference::Prompt(prompt_ref) => {
                assert_eq!(prompt_ref.name, "greeting_prompt");
            },
            _ => panic!("Expected prompt reference"),
        }
        
        assert_eq!(request.params.argument.name, "name");
        assert_eq!(request.params.argument.value, "Al");
    }

    #[test]
    fn test_completion_builder_resource() {
        let request = CompletionBuilder::for_resource("file:///users/{user_id}/profile.json")
            .argument("user_id", "123")
            .build();

        assert_eq!(request.method, "completion/complete");
        
        match &request.params.reference {
            CompletionReference::ResourceTemplate(resource_ref) => {
                assert_eq!(resource_ref.uri, "file:///users/{user_id}/profile.json");
            },
            _ => panic!("Expected resource template reference"),
        }
        
        assert_eq!(request.params.argument.name, "user_id");
        assert_eq!(request.params.argument.value, "123");
    }

    #[test]
    fn test_completion_builder_with_context() {
        let mut context_args = HashMap::new();
        context_args.insert("current_user".to_string(), "alice".to_string());
        context_args.insert("session_id".to_string(), "abc123".to_string());

        let request = CompletionBuilder::for_prompt("search_prompt")
            .argument("query", "rust prog")
            .with_context_arguments(context_args)
            .build();

        let context = request.params.context.expect("Expected completion context");
        let args = context.arguments.expect("Expected context arguments");
        assert_eq!(args.get("current_user").unwrap(), "alice");
        assert_eq!(args.get("session_id").unwrap(), "abc123");
    }

    #[test]
    fn test_completion_builder_context_arguments() {
        let request = CompletionBuilder::for_prompt("template_prompt")
            .argument("field", "val")
            .context_argument("user_id", "456")
            .context_argument("role", "admin")
            .build();

        let context = request.params.context.expect("Expected completion context");
        let args = context.arguments.expect("Expected context arguments");
        assert_eq!(args.len(), 2);
        assert_eq!(args.get("user_id").unwrap(), "456");
        assert_eq!(args.get("role").unwrap(), "admin");
    }

    #[test]
    fn test_completion_builder_convenience_methods() {
        // Test prompt argument completion
        let request1 = CompletionBuilder::prompt_argument("greeting", "name")
            .build();
        
        assert_eq!(request1.params.argument.name, "name");
        assert_eq!(request1.params.argument.value, "");

        // Test prompt argument with partial value
        let request2 = CompletionBuilder::prompt_argument_partial("greeting", "name", "Al")
            .build();
        
        assert_eq!(request2.params.argument.name, "name");
        assert_eq!(request2.params.argument.value, "Al");

        // Test resource parameter completion
        let request3 = CompletionBuilder::resource_parameter("file:///{path}.txt", "path")
            .build();
        
        assert_eq!(request3.params.argument.name, "path");
        assert_eq!(request3.params.argument.value, "");

        // Test resource parameter with partial value
        let request4 = CompletionBuilder::resource_parameter_partial("file:///{path}.txt", "path", "doc")
            .build();
        
        assert_eq!(request4.params.argument.name, "path");
        assert_eq!(request4.params.argument.value, "doc");
    }

    #[test]
    fn test_completion_builder_meta() {
        let meta = HashMap::from([
            ("request_id".to_string(), json!("req-123")),
            ("priority".to_string(), json!(1)),
        ]);

        let request = CompletionBuilder::for_prompt("test_prompt")
            .argument("arg", "value")
            .meta(meta.clone())
            .build();

        assert_eq!(request.params.meta, Some(meta));
    }

    #[test]
    fn test_completion_builder_step_by_step() {
        let request = CompletionBuilder::for_prompt("step_prompt")
            .argument_name("step")
            .current_value("ste")
            .context_argument("workflow_id", "wf-456")
            .build();

        assert_eq!(request.params.argument.name, "step");
        assert_eq!(request.params.argument.value, "ste");
        
        let context = request.params.context.expect("Expected context");
        let args = context.arguments.expect("Expected context arguments");
        assert_eq!(args.get("workflow_id").unwrap(), "wf-456");
    }

    #[test]
    fn test_completion_builder_build_params_only() {
        let params = CompletionBuilder::for_prompt("test")
            .argument("arg", "val")
            .build_params();

        match params.reference {
            CompletionReference::Prompt(prompt_ref) => {
                assert_eq!(prompt_ref.name, "test");
            },
            _ => panic!("Expected prompt reference"),
        }
        
        assert_eq!(params.argument.name, "arg");
        assert_eq!(params.argument.value, "val");
    }
}