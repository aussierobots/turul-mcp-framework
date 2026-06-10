//! 2026-07-28 request building and response parsing for client operations.
//!
//! Stateless core: every request carries `_meta` (per-request capability
//! negotiation — `protocolVersion` + `clientInfo` + `clientCapabilities`), and
//! list/read results carry `resultType` plus the `CacheableResult` mixin
//! (`ttlMs`/`cacheScope`). Results are mapped back to the version-neutral public
//! types `McpClient` exposes (which are the 2025-11-25 alias types); the core
//! fields overlap, and 2026-only fields the public type lacks are dropped here.

use crate::error::McpClientResult;
use serde_json::Value;
use turul_mcp_protocol_2026_07_28 as p;

/// Build the per-request `_meta` object required by the 2026 stateless core.
/// Capabilities are declared per request (servers MUST NOT infer them from
/// prior requests), translated from the config-level declarations.
pub(crate) fn request_meta(
    client_name: &str,
    client_version: &str,
    declared: &crate::config::DeclaredCapabilities,
) -> p::meta::RequestMetaObject {
    #[allow(deprecated)]
    let capabilities = p::initialize::ClientCapabilities {
        elicitation: declared
            .elicitation
            .then(p::initialize::ElicitationCapabilities::default),
        sampling: declared
            .sampling
            .then(p::initialize::SamplingCapabilities::default),
        roots: declared.roots.then(Default::default),
        ..Default::default()
    };
    p::meta::RequestMetaObject::new(
        p::MCP_VERSION,
        p::initialize::Implementation::new(client_name.to_string(), client_version.to_string()),
        capabilities,
    )
}

/// `resultType` discriminator check for MRTR-capable methods.
pub(crate) fn input_required_outcome(result: &Value) -> Option<crate::error::McpClientError> {
    if result.get("resultType").and_then(|v| v.as_str()) == Some("input_required") {
        Some(crate::error::McpClientError::InputRequired {
            input_requests: result.get("inputRequests").cloned(),
            request_state: result
                .get("requestState")
                .and_then(|v| v.as_str())
                .map(String::from),
        })
    } else {
        None
    }
}

/// Build an operation's params object with the required `_meta` merged in. `extra`
/// carries the op-specific params (e.g. `{ "name": ..., "arguments": ... }`).
pub(crate) fn params_with_meta(meta: &p::meta::RequestMetaObject, extra: Value) -> Value {
    let mut map = match extra {
        Value::Object(m) => m,
        _ => serde_json::Map::new(),
    };
    map.insert(
        "_meta".to_string(),
        serde_json::to_value(meta).unwrap_or(Value::Null),
    );
    Value::Object(map)
}

/// Map a 2026 typed value to the public (alias) type via JSON.
fn remap<A, B>(v: &A) -> McpClientResult<B>
where
    A: serde::Serialize,
    B: serde::de::DeserializeOwned,
{
    Ok(serde_json::from_value(serde_json::to_value(v)?)?)
}

pub(crate) fn parse_list_tools(
    result: &Value,
) -> McpClientResult<Vec<turul_mcp_protocol_2025_11_25::Tool>> {
    let r: p::tools::ListToolsResult = serde_json::from_value(result.clone())?;
    r.tools.iter().map(remap).collect()
}

pub(crate) fn parse_call_tool(
    result: &Value,
) -> McpClientResult<turul_mcp_protocol_2025_11_25::CallToolResult> {
    // MRTR (SEP-2322): an input_required result is not a CallToolResult —
    // surface it so the caller can gather inputs and retry with
    // `call_tool_with_input_responses`.
    if let Some(input_required) = input_required_outcome(result) {
        return Err(input_required);
    }
    let r: p::tools::CallToolResult = serde_json::from_value(result.clone())?;
    remap(&r)
}

pub(crate) fn parse_list_resources(
    result: &Value,
) -> McpClientResult<Vec<turul_mcp_protocol_2025_11_25::Resource>> {
    let r: p::resources::ListResourcesResult = serde_json::from_value(result.clone())?;
    r.resources.iter().map(remap).collect()
}

pub(crate) fn parse_list_resource_templates(
    result: &Value,
) -> McpClientResult<Vec<turul_mcp_protocol_2025_11_25::resources::ResourceTemplate>> {
    let r: p::resources::ListResourceTemplatesResult = serde_json::from_value(result.clone())?;
    r.resource_templates.iter().map(remap).collect()
}

pub(crate) fn parse_read_resource(
    result: &Value,
) -> McpClientResult<turul_mcp_protocol_2025_11_25::ReadResourceResult> {
    let r: p::resources::ReadResourceResult = serde_json::from_value(result.clone())?;
    remap(&r)
}

pub(crate) fn parse_list_prompts(
    result: &Value,
) -> McpClientResult<Vec<turul_mcp_protocol_2025_11_25::Prompt>> {
    let r: p::prompts::ListPromptsResult = serde_json::from_value(result.clone())?;
    r.prompts.iter().map(remap).collect()
}

pub(crate) fn parse_get_prompt(
    result: &Value,
) -> McpClientResult<turul_mcp_protocol_2025_11_25::GetPromptResult> {
    let r: p::prompts::GetPromptResult = serde_json::from_value(result.clone())?;
    remap(&r)
}

// Full-result parsers for the paginated list ops (preserve `nextCursor`). The
// 2026 result's `resultType`/`ttlMs`/`cacheScope` are dropped by the remap into
// the public (alias) result type.
pub(crate) fn parse_list_tools_result(
    result: &Value,
) -> McpClientResult<turul_mcp_protocol_2025_11_25::ListToolsResult> {
    let r: p::tools::ListToolsResult = serde_json::from_value(result.clone())?;
    remap(&r)
}

pub(crate) fn parse_list_resources_result(
    result: &Value,
) -> McpClientResult<turul_mcp_protocol_2025_11_25::ListResourcesResult> {
    let r: p::resources::ListResourcesResult = serde_json::from_value(result.clone())?;
    remap(&r)
}

pub(crate) fn parse_list_resource_templates_result(
    result: &Value,
) -> McpClientResult<turul_mcp_protocol_2025_11_25::resources::ListResourceTemplatesResult> {
    let r: p::resources::ListResourceTemplatesResult = serde_json::from_value(result.clone())?;
    remap(&r)
}

pub(crate) fn parse_list_prompts_result(
    result: &Value,
) -> McpClientResult<turul_mcp_protocol_2025_11_25::ListPromptsResult> {
    let r: p::prompts::ListPromptsResult = serde_json::from_value(result.clone())?;
    remap(&r)
}
