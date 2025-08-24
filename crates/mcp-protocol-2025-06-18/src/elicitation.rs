//! MCP Elicitation Protocol Types
//!
//! This module defines the types used for MCP elicitation functionality,
//! which enables structured user input collection via JSON Schema.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::schema::JsonSchema;
use crate::meta::{Meta, ProgressToken, WithMeta};

/// Request for structured user input elicitation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElicitationRequest {
    /// JSON Schema defining the structure of input to collect
    pub schema: JsonSchema,
    /// Human-readable prompt describing what input is needed
    pub prompt: String,
    /// Optional title for the elicitation dialog
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Optional description with additional context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional default values for the schema fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defaults: Option<Value>,
    /// Whether this elicitation is required (cannot be cancelled)
    #[serde(default)]
    pub required: bool,
}

impl ElicitationRequest {
    pub fn new(schema: JsonSchema, prompt: impl Into<String>) -> Self {
        Self {
            schema,
            prompt: prompt.into(),
            title: None,
            description: None,
            defaults: None,
            required: false,
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_defaults(mut self, defaults: Value) -> Self {
        self.defaults = Some(defaults);
        self
    }

    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }
}

/// Response containing the collected user input
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElicitationResponse {
    /// The collected input data matching the requested schema
    pub data: Value,
    /// Whether the elicitation was completed successfully
    pub completed: bool,
    /// Optional message from the user interface
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl ElicitationResponse {
    pub fn completed(data: Value) -> Self {
        Self {
            data,
            completed: true,
            message: None,
        }
    }

    pub fn cancelled() -> Self {
        Self {
            data: Value::Null,
            completed: false,
            message: Some("Elicitation cancelled by user".to_string()),
        }
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

/// Parameters for elicitation/request method
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElicitationRequestParams {
    /// The elicitation request details
    #[serde(flatten)]
    pub request: ElicitationRequest,
    /// Optional progress token for tracking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress_token: Option<ProgressToken>,
}

impl crate::traits::Params for ElicitationRequestParams {}

impl ElicitationRequestParams {
    pub fn new(request: ElicitationRequest) -> Self {
        Self {
            request,
            progress_token: None,
        }
    }

    pub fn with_progress_token(mut self, token: ProgressToken) -> Self {
        self.progress_token = Some(token);
        self
    }
}

/// Result of elicitation/request method
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElicitationRequestResult {
    /// The elicitation response
    #[serde(flatten)]
    pub response: ElicitationResponse,
    /// Optional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Meta>,
}

impl ElicitationRequestResult {
    pub fn new(response: ElicitationResponse) -> Self {
        Self {
            response,
            _meta: None,
        }
    }

    pub fn with_meta(mut self, meta: Meta) -> Self {
        self._meta = Some(meta);
        self
    }
}

impl WithMeta for ElicitationRequestResult {
    fn meta(&self) -> Option<&Meta> {
        self._meta.as_ref()
    }

    fn set_meta(&mut self, meta: Option<Meta>) {
        self._meta = meta;
    }
}

/// Notification when elicitation is cancelled
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElicitationCancelledNotification {
    /// Optional reason for cancellation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Progress token if this was for a tracked elicitation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress_token: Option<ProgressToken>,
}

impl ElicitationCancelledNotification {
    pub fn new() -> Self {
        Self {
            reason: None,
            progress_token: None,
        }
    }

    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    pub fn with_progress_token(mut self, token: ProgressToken) -> Self {
        self.progress_token = Some(token);
        self
    }
}

impl Default for ElicitationCancelledNotification {
    fn default() -> Self {
        Self::new()
    }
}

/// Utility for building common elicitation schemas
pub struct ElicitationBuilder;

impl ElicitationBuilder {
    /// Create a simple text input elicitation
    pub fn text_input(prompt: impl Into<String>, field_name: impl Into<String>, field_description: impl Into<String>) -> ElicitationRequest {
        use std::collections::HashMap;
        
        let field_name = field_name.into();
        let field_description = field_description.into();
        
        let mut properties = HashMap::new();
        properties.insert(field_name.clone(), JsonSchema::string().with_description(&field_description));
        
        let schema = JsonSchema::object()
            .with_properties(properties)
            .with_required(vec![field_name]);
            
        ElicitationRequest::new(schema, prompt)
    }

    /// Create a number input elicitation
    pub fn number_input(
        prompt: impl Into<String>, 
        field_name: impl Into<String>, 
        field_description: impl Into<String>,
        min: Option<f64>,
        max: Option<f64>
    ) -> ElicitationRequest {
        use std::collections::HashMap;
        
        let field_name = field_name.into();
        let field_description = field_description.into();
        
        let mut number_schema = JsonSchema::number().with_description(&field_description);
        if let Some(min) = min {
            number_schema = number_schema.with_minimum(min);
        }
        if let Some(max) = max {
            number_schema = number_schema.with_maximum(max);
        }
        
        let mut properties = HashMap::new();
        properties.insert(field_name.clone(), number_schema);
        
        let schema = JsonSchema::object()
            .with_properties(properties)
            .with_required(vec![field_name]);
            
        ElicitationRequest::new(schema, prompt)
    }

    /// Create a choice/enum elicitation
    pub fn choice_input(
        prompt: impl Into<String>, 
        field_name: impl Into<String>, 
        field_description: impl Into<String>,
        choices: Vec<String>
    ) -> ElicitationRequest {
        use std::collections::HashMap;
        
        let field_name = field_name.into();
        let field_description = field_description.into();
        
        let choice_schema = JsonSchema::string_enum(choices)
            .with_description(&field_description);
        
        let mut properties = HashMap::new();
        properties.insert(field_name.clone(), choice_schema);
        
        let schema = JsonSchema::object()
            .with_properties(properties)
            .with_required(vec![field_name]);
            
        ElicitationRequest::new(schema, prompt)
    }

    /// Create a boolean confirmation elicitation
    pub fn confirm(prompt: impl Into<String>) -> ElicitationRequest {
        use std::collections::HashMap;
        
        let mut properties = HashMap::new();
        properties.insert("confirmed".to_string(), JsonSchema::boolean()
            .with_description("User confirmation"));
        
        let schema = JsonSchema::object()
            .with_properties(properties)
            .with_required(vec!["confirmed".to_string()]);
            
        ElicitationRequest::new(schema, prompt)
    }

    /// Create a multi-field form elicitation
    pub fn form(prompt: impl Into<String>, fields: Vec<(String, JsonSchema)>, required: Vec<String>) -> ElicitationRequest {
        use std::collections::HashMap;
        
        let mut properties = HashMap::new();
        for (field_name, field_schema) in fields {
            properties.insert(field_name, field_schema);
        }
        
        let schema = JsonSchema::object()
            .with_properties(properties)
            .with_required(required);
            
        ElicitationRequest::new(schema, prompt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_elicitation_request_creation() {
        use std::collections::HashMap;
        
        let schema = JsonSchema::object()
            .with_properties(HashMap::from([
                ("name".to_string(), JsonSchema::string().with_description("Enter your name"))
            ]));
        
        let request = ElicitationRequest::new(schema, "Please provide your information")
            .with_title("User Information")
            .with_description("We need some basic information to continue")
            .required(true);
        
        assert_eq!(request.prompt, "Please provide your information");
        assert_eq!(request.title, Some("User Information".to_string()));
        assert!(request.required);
    }

    #[test]
    fn test_elicitation_response() {
        let data = json!({"name": "John Doe", "age": 30});
        let response = ElicitationResponse::completed(data.clone())
            .with_message("Thank you!");
        
        assert!(response.completed);
        assert_eq!(response.data, data);
        assert_eq!(response.message, Some("Thank you!".to_string()));
    }

    #[test]
    fn test_elicitation_cancelled() {
        let response = ElicitationResponse::cancelled();
        
        assert!(!response.completed);
        assert_eq!(response.data, Value::Null);
        assert!(response.message.is_some());
    }

    #[test]
    fn test_elicitation_builder_text_input() {
        let request = ElicitationBuilder::text_input(
            "Enter your username",
            "username",
            "Your unique username"
        );
        
        assert_eq!(request.prompt, "Enter your username");
        // Schema structure should be valid
        assert!(matches!(request.schema, JsonSchema::Object { .. }));
    }

    #[test]
    fn test_elicitation_builder_number_input() {
        let request = ElicitationBuilder::number_input(
            "Enter your age",
            "age",
            "Your age in years",
            Some(0.0),
            Some(120.0)
        );
        
        assert_eq!(request.prompt, "Enter your age");
        assert!(matches!(request.schema, JsonSchema::Object { .. }));
    }

    #[test]
    fn test_elicitation_builder_choice() {
        let choices = vec!["red".to_string(), "green".to_string(), "blue".to_string()];
        let request = ElicitationBuilder::choice_input(
            "Choose your favorite color",
            "color",
            "Your preferred color",
            choices
        );
        
        assert_eq!(request.prompt, "Choose your favorite color");
        assert!(matches!(request.schema, JsonSchema::Object { .. }));
    }

    #[test]
    fn test_elicitation_builder_confirm() {
        let request = ElicitationBuilder::confirm("Do you want to continue?");
        
        assert_eq!(request.prompt, "Do you want to continue?");
        assert!(matches!(request.schema, JsonSchema::Object { .. }));
    }

    #[test]
    fn test_serialization() {
        let schema = JsonSchema::string().with_description("Test field");
        let request = ElicitationRequest::new(schema, "Test prompt");
        
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("Test prompt"));
        
        let parsed: ElicitationRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.prompt, "Test prompt");
    }
}