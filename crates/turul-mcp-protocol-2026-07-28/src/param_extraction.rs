//! Parameter extraction utilities for MCP protocol.
//!
//! This module provides unified parameter extraction from JSON-RPC RequestParams
//! to strongly-typed MCP parameter structures.

use crate::traits::{Params, SerdeParamExtractor};

/// Implements `SerdeParamExtractor` for a `Params` type that implements `Deserialize`.
#[macro_export]
macro_rules! impl_serde_extractor {
    ($param_type:ty) => {
        impl $crate::traits::SerdeParamExtractor<$param_type> for $param_type {
            type Error = $crate::McpError;

            fn extract_serde(params: turul_rpc::RequestParams) -> Result<$param_type, Self::Error> {
                let value = params.to_value();

                serde_json::from_value(value).map_err(|e| {
                    $crate::McpError::InvalidParameters(format!(
                        "Failed to deserialize {}: {}",
                        stringify!($param_type),
                        e
                    ))
                })
            }
        }
    };
}

// Apply macro to all serde-compatible parameter types that exist.
impl_serde_extractor!(crate::tools::CallToolRequestParams);
// `PaginatedRequestParams` covers all 4 `PaginatedRequest` extenders —
// `ListToolsRequest`, `ListResourcesRequest`, `ListResourceTemplatesRequest`,
// `ListPromptsRequest` — single shared shape.
impl_serde_extractor!(crate::json_rpc::PaginatedRequestParams);
impl_serde_extractor!(crate::resources::ReadResourceRequestParams);
// Subscribe/Unsubscribe have no params types in 2026-07-28; `subscriptions/listen` replaces them.
impl_serde_extractor!(crate::prompts::GetPromptRequestParams);
impl_serde_extractor!(crate::completion::CompleteRequestParams);
// `logging/setLevel` is not in the 2026-07-28 schema, so it has no extractor.
impl_serde_extractor!(crate::elicitation::ElicitRequestFormParams);
// SEP-2577-deprecated; extractor retained during the migration window.
mod sampling_extractor_impls {
    #![allow(deprecated)]
    impl_serde_extractor!(crate::sampling::CreateMessageRequestParams);
}
impl_serde_extractor!(crate::ping::EmptyParams);

/// Extracts a typed parameter struct from JSON-RPC request params.
///
/// Works with any type implementing [`SerdeParamExtractor`].
pub fn extract_params<T>(params: turul_rpc::RequestParams) -> Result<T, crate::McpError>
where
    T: Params + SerdeParamExtractor<T, Error = crate::McpError>,
{
    T::extract_serde(params)
}

/// Extracts typed params from an `Option<RequestParams>`.
///
/// Returns `T::default()` when `params` is `None`.
pub fn extract_optional_params<T>(
    params: Option<turul_rpc::RequestParams>,
) -> Result<T, crate::McpError>
where
    T: Params + SerdeParamExtractor<T, Error = crate::McpError> + Default,
{
    match params {
        Some(p) => extract_params(p),
        None => Ok(T::default()),
    }
}
