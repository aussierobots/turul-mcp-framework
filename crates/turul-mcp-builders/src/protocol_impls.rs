//! Framework trait implementations for protocol types
//!
//! This module provides framework trait implementations for concrete protocol types
//! (Resource, Prompt, Tool, Root, etc.) from turul_mcp_protocol, enabling them to be used
//! with framework features like ResourceDefinition, PromptDefinition, ToolDefinition, etc.
//!
//! **CRITICAL**: Every protocol type that has corresponding framework traits MUST have
//! implementations here. Missing implementations break the trait hierarchy and cause
//! compilation failures in user code.

use crate::traits::*;
use turul_mcp_protocol::completion::{
    CompleteArgument, CompleteRequest, CompletionContext, CompletionReference,
};
use turul_mcp_protocol::elicitation::{ElicitCreateRequest, ElicitationSchema};
use turul_mcp_protocol::logging::{LoggingLevel, LoggingMessageNotification};
use turul_mcp_protocol::notifications::{
    CancelledNotification, InitializedNotification, Notification, ProgressNotification,
    PromptListChangedNotification, ResourceListChangedNotification, ResourceUpdatedNotification,
    RootsListChangedNotification, ToolListChangedNotification,
};
use turul_mcp_protocol::roots::Root;
use turul_mcp_protocol::sampling::{CreateMessageParams, ModelPreferences, SamplingMessage};
use turul_mcp_protocol::tools::ToolExecution;
use turul_mcp_protocol::{Prompt, Resource, Tool, ToolSchema};

// ============================================================================
// Resource trait implementations
// ============================================================================

impl HasResourceMetadata for Resource {
    fn name(&self) -> &str {
        &self.name
    }

    fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }
}

impl HasResourceDescription for Resource {
    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

impl HasResourceUri for Resource {
    fn uri(&self) -> &str {
        &self.uri
    }
}

impl HasResourceMimeType for Resource {
    fn mime_type(&self) -> Option<&str> {
        self.mime_type.as_deref()
    }
}

impl HasResourceSize for Resource {
    fn size(&self) -> Option<u64> {
        self.size
    }
}

impl HasResourceAnnotations for Resource {
    fn annotations(&self) -> Option<&turul_mcp_protocol::meta::Annotations> {
        self.annotations.as_ref()
    }
}

impl HasResourceMeta for Resource {
    fn resource_meta(&self) -> Option<&std::collections::HashMap<String, serde_json::Value>> {
        self.meta.as_ref()
    }
}

impl HasIcons for Resource {
    fn icons(&self) -> Option<&Vec<turul_mcp_protocol::icons::Icon>> {
        self.icons.as_ref()
    }
}

// ResourceDefinition is automatically implemented via blanket impl

// ============================================================================
// Prompt trait implementations
// ============================================================================

impl HasPromptMetadata for Prompt {
    fn name(&self) -> &str {
        &self.name
    }

    fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }
}

impl HasPromptDescription for Prompt {
    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

impl HasPromptArguments for Prompt {
    fn arguments(&self) -> Option<&Vec<turul_mcp_protocol::prompts::PromptArgument>> {
        self.arguments.as_ref()
    }
}

impl HasPromptAnnotations for Prompt {
    fn annotations(&self) -> Option<&turul_mcp_protocol::prompts::PromptAnnotations> {
        // Prompt struct doesn't have annotations field in current protocol
        None
    }
}

impl HasPromptMeta for Prompt {
    fn prompt_meta(&self) -> Option<&std::collections::HashMap<String, serde_json::Value>> {
        self.meta.as_ref()
    }
}

impl HasIcons for Prompt {
    fn icons(&self) -> Option<&Vec<turul_mcp_protocol::icons::Icon>> {
        self.icons.as_ref()
    }
}

// PromptDefinition is automatically implemented via blanket impl

// ============================================================================
// Tool trait implementations
// ============================================================================

impl HasBaseMetadata for Tool {
    fn name(&self) -> &str {
        &self.name
    }

    fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }
}

impl HasDescription for Tool {
    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

impl HasInputSchema for Tool {
    fn input_schema(&self) -> &ToolSchema {
        &self.input_schema
    }
}

impl HasOutputSchema for Tool {
    fn output_schema(&self) -> Option<&ToolSchema> {
        self.output_schema.as_ref()
    }
}

impl HasAnnotations for Tool {
    fn annotations(&self) -> Option<&turul_mcp_protocol::tools::ToolAnnotations> {
        self.annotations.as_ref()
    }
}

impl HasToolMeta for Tool {
    fn tool_meta(&self) -> Option<&std::collections::HashMap<String, serde_json::Value>> {
        self.meta.as_ref()
    }
}

impl HasIcons for Tool {
    fn icons(&self) -> Option<&Vec<turul_mcp_protocol::icons::Icon>> {
        self.icons.as_ref()
    }
}

impl HasExecution for Tool {
    fn execution(&self) -> Option<ToolExecution> {
        self.execution.clone()
    }
}

// ToolDefinition is automatically implemented via blanket impl

// ============================================================================
// Root trait implementations
// ============================================================================

impl HasRootMetadata for Root {
    fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    fn uri(&self) -> &str {
        &self.uri
    }
}

impl HasRootPermissions for Root {
    // Root struct doesn't have permissions field - framework can add later if needed
}

impl HasRootFiltering for Root {
    // Root struct doesn't have filtering field - framework can add later if needed
}

impl HasRootAnnotations for Root {
    fn annotations(&self) -> Option<&std::collections::HashMap<String, serde_json::Value>> {
        self.meta.as_ref()
    }
}

// RootDefinition is automatically implemented via blanket impl

// ============================================================================
// Sampling trait implementations
// ============================================================================

impl HasSamplingConfig for CreateMessageParams {
    fn max_tokens(&self) -> u32 {
        self.max_tokens
    }

    fn temperature(&self) -> Option<f64> {
        self.temperature
    }

    fn stop_sequences(&self) -> Option<&Vec<String>> {
        self.stop_sequences.as_ref()
    }
}

impl HasSamplingContext for CreateMessageParams {
    fn messages(&self) -> &[SamplingMessage] {
        &self.messages
    }

    fn system_prompt(&self) -> Option<&str> {
        self.system_prompt.as_deref()
    }

    fn include_context(&self) -> Option<&str> {
        self.include_context.as_deref()
    }
}

impl HasModelPreferences for CreateMessageParams {
    fn model_preferences(&self) -> Option<&ModelPreferences> {
        self.model_preferences.as_ref()
    }

    fn metadata(&self) -> Option<&serde_json::Value> {
        self.metadata.as_ref()
    }
}

impl HasSamplingTools for CreateMessageParams {
    fn tools(&self) -> Option<&Vec<turul_mcp_protocol::Tool>> {
        self.tools.as_ref()
    }
}

// SamplingDefinition is automatically implemented via blanket impl

impl HasSamplingMessageMetadata for SamplingMessage {
    fn role(&self) -> &turul_mcp_protocol::sampling::Role {
        &self.role
    }

    fn content(&self) -> &turul_mcp_protocol::prompts::ContentBlock {
        &self.content
    }
}

// ============================================================================
// Logging trait implementations
// ============================================================================

impl HasLoggingMetadata for LoggingMessageNotification {
    fn method(&self) -> &str {
        &self.method
    }

    fn logger_name(&self) -> Option<&str> {
        self.params.logger.as_deref()
    }
}

impl HasLogLevel for LoggingMessageNotification {
    fn level(&self) -> LoggingLevel {
        self.params.level
    }
}

impl HasLogFormat for LoggingMessageNotification {
    fn data(&self) -> &serde_json::Value {
        &self.params.data
    }
}

impl HasLogTransport for LoggingMessageNotification {
    // Use default implementations
}

// LoggerDefinition is automatically implemented via blanket impl

// ============================================================================
// Completion trait implementations
// ============================================================================

impl HasCompletionMetadata for CompleteRequest {
    fn method(&self) -> &str {
        &self.method
    }

    fn reference(&self) -> &CompletionReference {
        &self.params.reference
    }
}

impl HasCompletionContext for CompleteRequest {
    fn argument(&self) -> &CompleteArgument {
        &self.params.argument
    }

    fn context(&self) -> Option<&CompletionContext> {
        self.params.context.as_ref()
    }
}

impl HasCompletionHandling for CompleteRequest {
    // Use default implementations
}

// CompletionDefinition is automatically implemented via blanket impl

// ============================================================================
// Elicitation trait implementations
// ============================================================================

impl HasElicitationMetadata for ElicitCreateRequest {
    fn message(&self) -> &str {
        &self.params.message
    }

    // title() uses default implementation which returns None
}

impl HasElicitationSchema for ElicitCreateRequest {
    fn requested_schema(&self) -> &ElicitationSchema {
        &self.params.requested_schema
    }
}

impl HasElicitationHandling for ElicitCreateRequest {
    // Use default implementations
}

// ElicitationDefinition is automatically implemented via blanket impl

// ============================================================================
// Notification trait implementations
// ============================================================================

// Base Notification type
impl HasNotificationMetadata for Notification {
    fn method(&self) -> &str {
        &self.method
    }
}

impl HasNotificationPayload for Notification {
    fn payload(&self) -> Option<serde_json::Value> {
        self.params.as_ref().map(|params| {
            let mut map = serde_json::Map::new();

            // Add all params.other fields
            for (key, value) in &params.other {
                map.insert(key.clone(), value.clone());
            }

            // Add _meta if present
            if let Some(meta) = &params.meta
                && let Ok(meta_value) = serde_json::to_value(meta)
            {
                map.insert("_meta".to_string(), meta_value);
            }

            serde_json::Value::Object(map)
        })
    }
}

impl HasNotificationRules for Notification {}

// ResourceListChangedNotification
impl HasNotificationMetadata for ResourceListChangedNotification {
    fn method(&self) -> &str {
        &self.method
    }
}

impl HasNotificationPayload for ResourceListChangedNotification {
    fn payload(&self) -> Option<serde_json::Value> {
        // Serialize params if present (includes _meta)
        self.params
            .as_ref()
            .and_then(|p| serde_json::to_value(p).ok())
    }
}

impl HasNotificationRules for ResourceListChangedNotification {}

// ToolListChangedNotification
impl HasNotificationMetadata for ToolListChangedNotification {
    fn method(&self) -> &str {
        &self.method
    }
}

impl HasNotificationPayload for ToolListChangedNotification {
    fn payload(&self) -> Option<serde_json::Value> {
        // Serialize params if present (includes _meta)
        self.params
            .as_ref()
            .and_then(|p| serde_json::to_value(p).ok())
    }
}

impl HasNotificationRules for ToolListChangedNotification {}

// PromptListChangedNotification
impl HasNotificationMetadata for PromptListChangedNotification {
    fn method(&self) -> &str {
        &self.method
    }
}

impl HasNotificationPayload for PromptListChangedNotification {
    fn payload(&self) -> Option<serde_json::Value> {
        // Serialize params if present (includes _meta)
        self.params
            .as_ref()
            .and_then(|p| serde_json::to_value(p).ok())
    }
}

impl HasNotificationRules for PromptListChangedNotification {}

// RootsListChangedNotification
impl HasNotificationMetadata for RootsListChangedNotification {
    fn method(&self) -> &str {
        &self.method
    }
}

impl HasNotificationPayload for RootsListChangedNotification {
    fn payload(&self) -> Option<serde_json::Value> {
        // Serialize params if present (includes _meta)
        self.params
            .as_ref()
            .and_then(|p| serde_json::to_value(p).ok())
    }
}

impl HasNotificationRules for RootsListChangedNotification {}

// ProgressNotification
impl HasNotificationMetadata for ProgressNotification {
    fn method(&self) -> &str {
        &self.method
    }
}

impl HasNotificationPayload for ProgressNotification {
    fn payload(&self) -> Option<serde_json::Value> {
        // Serialize the entire params struct (includes progressToken, progress, total, message, _meta)
        serde_json::to_value(&self.params).ok()
    }
}

impl HasNotificationRules for ProgressNotification {
    fn priority(&self) -> u32 {
        2 // Progress notifications have higher priority
    }
}

// ResourceUpdatedNotification
impl HasNotificationMetadata for ResourceUpdatedNotification {
    fn method(&self) -> &str {
        &self.method
    }
}

impl HasNotificationPayload for ResourceUpdatedNotification {
    fn payload(&self) -> Option<serde_json::Value> {
        // Serialize params (includes uri, _meta)
        serde_json::to_value(&self.params).ok()
    }
}

impl HasNotificationRules for ResourceUpdatedNotification {}

// CancelledNotification
impl HasNotificationMetadata for CancelledNotification {
    fn method(&self) -> &str {
        &self.method
    }
}

impl HasNotificationPayload for CancelledNotification {
    fn payload(&self) -> Option<serde_json::Value> {
        // Serialize params (includes requestId, reason, _meta)
        serde_json::to_value(&self.params).ok()
    }
}

impl HasNotificationRules for CancelledNotification {
    fn priority(&self) -> u32 {
        3 // Cancellation has highest priority
    }
}

// InitializedNotification
impl HasNotificationMetadata for InitializedNotification {
    fn method(&self) -> &str {
        &self.method
    }
}

impl HasNotificationPayload for InitializedNotification {
    fn payload(&self) -> Option<serde_json::Value> {
        // Serialize params if present (includes _meta)
        self.params
            .as_ref()
            .and_then(|p| serde_json::to_value(p).ok())
    }
}

impl HasNotificationRules for InitializedNotification {
    fn priority(&self) -> u32 {
        3 // Initialization has highest priority
    }
}

// NotificationDefinition is automatically implemented via blanket impl for all notification types
