//! Framework traits for MCP elicitation construction
//!
//! **IMPORTANT**: These are framework features, NOT part of the MCP specification.

use serde_json::Value;
use std::collections::HashMap;
use turul_mcp_protocol::elicitation::ElicitationSchema;

#[cfg(feature = "protocol-2025-11-25")]
use turul_mcp_protocol::elicitation::ElicitCreateRequest;

/// Supplies the identity and display fields of an elicitation request.
pub trait HasElicitationMetadata {
    /// Returns the message to present to the user.
    fn message(&self) -> &str;

    /// Returns the optional title for the elicitation dialog.
    fn title(&self) -> Option<&str> {
        None
    }

    /// Returns the optional `_meta` sent on the request.
    fn meta(&self) -> Option<&HashMap<String, Value>> {
        None
    }
}

/// Supplies the elicitation schema, restricted to primitive property types by the MCP spec.
pub trait HasElicitationSchema {
    /// Returns the restricted schema defining the structure of the input to collect.
    fn requested_schema(&self) -> &ElicitationSchema;

    /// Validates that the schema contains only primitive types.
    fn validate_schema(&self) -> Result<(), String> {
        // All schemas in ElicitationSchema are already primitive-only by design
        Ok(())
    }
}

/// Validates and processes content submitted in response to an elicitation.
pub trait HasElicitationHandling {
    /// Validates submitted content against the schema.
    fn validate_content(&self, _content: &HashMap<String, Value>) -> Result<(), String> {
        Ok(())
    }

    /// Processes accepted content, for example to transform or normalize it.
    fn process_content(
        &self,
        content: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, String> {
        Ok(content)
    }
}

/// Defines a complete MCP elicitation that prompts users for schema-validated structured input.
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
/// # use turul_mcp_protocol::elicitation::{ElicitationSchema, PrimitiveSchemaDefinition};
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
///         let schema = ElicitationSchema::new()
///             .with_property(
///                 "theme",
///                 PrimitiveSchemaDefinition::enum_values(vec!["dark".to_string(), "light".to_string()]),
///             )
///             .with_property("notifications", PrimitiveSchemaDefinition::boolean())
///             .with_property("max_items", PrimitiveSchemaDefinition::number())
///             .with_required(vec!["theme".to_string()]);
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
/// // The elicitation automatically implements ElicitationDefinition
/// fn assert_definition<T: ElicitationDefinition>(_: &T) {}
/// let form = UserPreferencesForm::new("project-setup".to_string());
/// assert_definition(&form);
/// assert_eq!(form.message(), "Please configure your preferences for this project");
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
    /// Converts this elicitation definition to a protocol `ElicitCreateRequest`.
    #[cfg(feature = "protocol-2025-11-25")]
    fn to_create_request(&self) -> ElicitCreateRequest {
        let request = ElicitCreateRequest::new(self.message(), self.requested_schema().clone());
        match self.meta() {
            Some(meta) => request.with_meta(meta.clone()),
            None => request,
        }
    }
}

// Blanket implementation: any type implementing the fine-grained traits automatically gets ElicitationDefinition
impl<T> ElicitationDefinition for T where
    T: HasElicitationMetadata + HasElicitationSchema + HasElicitationHandling
{
}
