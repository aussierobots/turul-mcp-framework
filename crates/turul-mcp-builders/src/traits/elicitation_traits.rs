//! Framework traits for MCP elicitation construction
//!
//! **IMPORTANT**: These are framework features, NOT part of the MCP specification.

use turul_mcp_protocol::elicitation::{ElicitCreateRequest, ElicitationSchema};
use serde_json::Value;
use std::collections::HashMap;

pub trait HasElicitationMetadata {
    /// The message to present to the user
    fn message(&self) -> &str;

    /// Optional title for the elicitation dialog
    fn title(&self) -> Option<&str> {
        None
    }
}

/// Trait for elicitation schema definition (restricted to primitive types per MCP spec)
pub trait HasElicitationSchema {
    /// Restricted schema defining structure of input to collect (primitives only)
    fn requested_schema(&self) -> &ElicitationSchema;

    /// Validate that schema only contains primitive types (per MCP spec)
    fn validate_schema(&self) -> Result<(), String> {
        // All schemas in ElicitationSchema are already primitive-only by design
        Ok(())
    }
}

/// Trait for elicitation validation and handling
pub trait HasElicitationHandling {
    /// Validate submitted content against the schema
    fn validate_content(&self, _content: &HashMap<String, Value>) -> Result<(), String> {
        // Basic validation - can be extended
        Ok(())
    }

    /// Process accepted content (transform, normalize, etc.)
    fn process_content(
        &self,
        content: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, String> {
        Ok(content)
    }
}

/// **Complete MCP Elicitation Creation** - Build schema-validated user input collection systems.
///
/// This trait represents a **complete, working MCP elicitation** that can prompt users
/// for structured input and validate their responses against JSON schemas. When you implement
/// the required metadata traits, you automatically get `ElicitationDefinition` for free
/// via blanket implementation.
///
/// # What You're Building
///
/// An elicitation is a sophisticated user input system that:
/// - Presents a clear message/prompt to the user
/// - Defines a JSON schema for expected input structure
/// - Validates user responses against that schema
/// - Processes the validated data for your application
///
/// # How to Create an Elicitation
///
/// Implement these three traits on your struct:
///
/// ```rust
/// # use turul_mcp_protocol::elicitation::*;
/// # use turul_mcp_builders::prelude::*;
/// # use serde_json::Value;
/// # use std::collections::HashMap;
///
/// // This struct will automatically implement ElicitationDefinition!
/// struct UserPreferencesForm {
///     context: String,
///     schema: ElicitationSchema,
/// }
///
/// impl UserPreferencesForm {
///     fn new(context: String) -> Self {
///         let mut properties = HashMap::new();
///         properties.insert("theme".to_string(), PrimitiveSchemaDefinition::Enum(EnumSchema {
///             schema_type: "string".to_string(),
///             title: None,
///             description: Some("UI theme preference".to_string()),
///             enum_values: vec!["dark".to_string(), "light".to_string()],
///             enum_names: None,
///         }));
///         properties.insert("notifications".to_string(), PrimitiveSchemaDefinition::Boolean(BooleanSchema {
///             schema_type: "boolean".to_string(),
///             title: None,
///             description: Some("Enable notifications".to_string()),
///             default: Some(true),
///         }));
///         properties.insert("max_items".to_string(), PrimitiveSchemaDefinition::Number(NumberSchema {
///             schema_type: "number".to_string(),
///             title: None,
///             description: Some("Maximum items to display".to_string()),
///             default: None,
///             minimum: Some(1.0),
///             maximum: Some(100.0),
///         }));
///
///         let schema = ElicitationSchema {
///             schema_type: "object".to_string(),
///             properties,
///             required: Some(vec!["theme".to_string()]),
///         };
///
///         Self { context, schema }
///     }
/// }
///
/// impl HasElicitationMetadata for UserPreferencesForm {
///     fn message(&self) -> &str {
///         "Please configure your preferences for this project"
///     }
/// }
///
/// impl HasElicitationSchema for UserPreferencesForm {
///     fn requested_schema(&self) -> &ElicitationSchema {
///         &self.schema
///     }
/// }
///
/// impl HasElicitationHandling for UserPreferencesForm {
///     fn process_content(&self, content: HashMap<String, Value>) -> Result<HashMap<String, Value>, String> {
///         // Validate that theme is acceptable
///         if let Some(theme) = content.get("theme") {
///             if !["dark", "light"].contains(&theme.as_str().unwrap_or("")) {
///                 return Err("Theme must be 'dark' or 'light'".to_string());
///             }
///         }
///
///         // Process and potentially transform the data
///         let mut processed = content.clone();
///         processed.insert("processed_at".to_string(), Value::String("2024-01-01T00:00:00Z".to_string()));
///         Ok(processed)
///     }
/// }
///
/// // Now you can use it with the server:
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let form = UserPreferencesForm::new("project-setup".to_string());
///
/// // The elicitation automatically implements ElicitationDefinition
/// let create_request = form.to_create_request();
/// # Ok(())
/// # }
/// ```
///
/// # Key Benefits
///
/// - **Type Safety**: Schema validation happens at the protocol level
/// - **Automatic Implementation**: Just implement the three component traits
/// - **Flexible Processing**: Handle and transform user input as needed
/// - **MCP Compliant**: Fully compatible with MCP 2025-11-25 specification
///
/// # Common Use Cases
///
/// - Configuration forms with validation
/// - User preference collection
/// - Survey and feedback systems
/// - Structured data entry workflows
/// - Multi-step input wizards
pub trait ElicitationDefinition:
    HasElicitationMetadata + HasElicitationSchema + HasElicitationHandling
{
    /// Convert this elicitation definition to a protocol ElicitCreateRequest
    fn to_create_request(&self) -> ElicitCreateRequest {
        ElicitCreateRequest::new(self.message(), self.requested_schema().clone())
    }
}

// Blanket implementation: any type implementing the fine-grained traits automatically gets ElicitationDefinition
impl<T> ElicitationDefinition for T where
    T: HasElicitationMetadata + HasElicitationSchema + HasElicitationHandling
{
}
