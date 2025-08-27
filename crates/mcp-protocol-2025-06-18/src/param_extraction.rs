//! Parameter extraction utilities for MCP protocol
//!
//! This module provides unified parameter extraction from JSON-RPC RequestParams
//! to strongly-typed MCP parameter structures, matching the pattern from the other project.

use crate::traits::{Params, SerdeParamExtractor};

/// Macro to implement SerdeParamExtractor for any Params type that implements Deserialize
#[macro_export]
macro_rules! impl_serde_extractor {
    ($param_type:ty) => {
        impl $crate::traits::SerdeParamExtractor<$param_type> for $param_type {
            type Error = $crate::McpError;

            fn extract_serde(params: mcp_json_rpc_server::RequestParams) -> Result<$param_type, Self::Error> {
                // Convert RequestParams to Value
                let value = params.to_value();
                
                // Deserialize to the target type
                serde_json::from_value(value)
                    .map_err(|e| $crate::McpError::InvalidParameters(
                        format!("Failed to deserialize {}: {}", stringify!($param_type), e)
                    ))
            }
        }
    };
}

// Apply macro to all serde-compatible parameter types that exist
// Note: InitializeRequest doesn't follow the Request/Params pattern
impl_serde_extractor!(crate::tools::CallToolParams);
impl_serde_extractor!(crate::tools::ListToolsParams);
impl_serde_extractor!(crate::resources::ListResourcesParams);
impl_serde_extractor!(crate::resources::ListResourceTemplatesParams);
impl_serde_extractor!(crate::resources::ReadResourceParams);
// Note: Subscribe/Unsubscribe don't have separate params types
impl_serde_extractor!(crate::prompts::ListPromptsParams);
impl_serde_extractor!(crate::prompts::GetPromptParams);
impl_serde_extractor!(crate::completion::CompleteParams);
impl_serde_extractor!(crate::logging::SetLevelParams);
// Note: ListRootsRequest has no params per MCP spec
// Note: ResourceTemplate functionality is now in resources module
// ListResourceTemplatesParams is already handled above in resources section
// GetTemplateRequest and RenderTemplateRequest were template-specific, not part of MCP spec
impl_serde_extractor!(crate::elicitation::ElicitCreateParams);
impl_serde_extractor!(crate::sampling::CreateMessageParams);
impl_serde_extractor!(crate::ping::EmptyParams);
impl_serde_extractor!(crate::resources::SubscribeParams);
impl_serde_extractor!(crate::resources::UnsubscribeParams);

/// Generic parameter extractor function that works with any type implementing SerdeParamExtractor
pub fn extract_params<T>(params: mcp_json_rpc_server::RequestParams) -> Result<T, crate::McpError>
where
    T: Params + SerdeParamExtractor<T, Error = crate::McpError>,
{
    T::extract_serde(params)
}

/// Helper function to extract params from Option<RequestParams>
pub fn extract_optional_params<T>(params: Option<mcp_json_rpc_server::RequestParams>) -> Result<T, crate::McpError>
where
    T: Params + SerdeParamExtractor<T, Error = crate::McpError> + Default,
{
    match params {
        Some(p) => extract_params(p),
        None => Ok(T::default()),
    }
}