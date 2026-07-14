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
        elicitation: (declared.elicitation || declared.elicitation_url).then(|| {
            p::initialize::ElicitationCapabilities {
                // Explicit form marker keeps form support declared even when
                // url is also present (an explicit url-only object would NOT
                // imply form-mode support).
                form: Some(Default::default()),
                url: declared.elicitation_url.then(Default::default),
                ..Default::default()
            }
        }),
        sampling: (declared.sampling || declared.sampling_tools || declared.sampling_context).then(
            || p::initialize::SamplingCapabilities {
                tools: declared.sampling_tools.then(Default::default),
                context: declared.sampling_context.then(Default::default),
                ..Default::default()
            },
        ),
        roots: declared.roots.then(Default::default),
        extensions: declared.ext_tasks.then(|| {
            std::collections::HashMap::from([(
                "io.modelcontextprotocol/tasks".to_string(),
                serde_json::json!({}),
            )])
        }),
        ..Default::default()
    };
    p::meta::RequestMetaObject::new(
        p::MCP_VERSION,
        p::initialize::Implementation::new(client_name.to_string(), client_version.to_string()),
        capabilities,
    )
}

/// `resultType` discriminator check (SEP-2322 / basic §Responses).
///
/// `Ok(())` means a complete result (absent or `"complete"`). `"input_required"`
/// surfaces as [`McpClientError::InputRequired`] so the caller can gather inputs
/// and retry. Any other discriminator is invalid: "A resultType of any value
/// unrecognized by the client MUST be considered invalid."
pub(crate) fn check_result_type(result: &Value) -> Result<(), crate::error::McpClientError> {
    match result.get("resultType").and_then(|v| v.as_str()) {
        None | Some("complete") => Ok(()),
        Some("input_required") => Err(crate::error::McpClientError::InputRequired {
            input_requests: result.get("inputRequests").cloned(),
            request_state: result
                .get("requestState")
                .and_then(|v| v.as_str())
                .map(String::from),
        }),
        Some(other) => Err(crate::error::ProtocolError::InvalidResponse(format!(
            "unrecognized resultType {other:?}"
        ))
        .into()),
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

/// A tool whose `inputSchema` fails JSON Schema 2020-12 dialect validation is
/// excluded from `tools/list`.
fn schema_is_valid(schema: &Value) -> Result<(), String> {
    turul_mcp_schema_validation::validate_tool_input_schema(schema).map_err(|e| e.to_string())
}

pub(crate) fn parse_list_tools(
    result: &Value,
) -> McpClientResult<Vec<turul_mcp_protocol_2025_11_25::Tool>> {
    check_result_type(result)?;
    let r: p::tools::ListToolsResult = serde_json::from_value(result.clone())?;
    r.tools
        .iter()
        .filter(|tool| {
            let schema = serde_json::to_value(&tool.input_schema).unwrap_or_default();

            // SEP-2243: clients on Streamable HTTP MUST reject tool
            // definitions whose x-mcp-header annotations violate the
            // constraints, INCLUDING an annotation reachable only through
            // `items`/composition/`$ref` rather than a plain `properties`
            // chain — exclude the tool and log a warning so one malformed
            // definition doesn't block the rest.
            if let Err(reason) = p::headers::scan_x_mcp_headers(&schema) {
                tracing::warn!(
                    tool = %tool.name,
                    %reason,
                    "excluding tool from tools/list: invalid x-mcp-header annotation"
                );
                return false;
            }
            if let Some(pointer) = p::headers::find_misplaced_x_mcp_header(&schema) {
                tracing::warn!(
                    tool = %tool.name,
                    %pointer,
                    "excluding tool from tools/list: x-mcp-header annotation not reachable via a properties chain"
                );
                return false;
            }

            // Reject an invalid advertised inputSchema (JSON Schema 2020-12
            // dialect/bounds).
            if let Err(reason) = schema_is_valid(&schema) {
                tracing::warn!(
                    tool = %tool.name,
                    %reason,
                    "excluding tool from tools/list: invalid inputSchema"
                );
                return false;
            }

            true
        })
        .map(remap)
        .collect()
}

/// SEP-2243: per-tool `Mcp-Param-*` bindings from a raw `tools/list` result —
/// `(header name, argument JSON pointer, declared type)` per annotation.
/// Tools with invalid annotations yield no entry (they are excluded from the
/// list result anyway).
pub(crate) fn collect_param_bindings(
    result: &Value,
) -> std::collections::HashMap<String, Vec<(String, String, String)>> {
    let mut map = std::collections::HashMap::new();
    let Some(tools) = result.get("tools").and_then(|t| t.as_array()) else {
        return map;
    };
    for tool in tools {
        let (Some(name), Some(schema)) = (
            tool.get("name").and_then(|n| n.as_str()),
            tool.get("inputSchema"),
        ) else {
            continue;
        };
        if let Ok(bindings) = p::headers::scan_x_mcp_headers(schema)
            && !bindings.is_empty()
        {
            map.insert(
                name.to_string(),
                bindings
                    .into_iter()
                    .map(|b| (b.header_name, b.argument_pointer, b.schema_type))
                    .collect(),
            );
        }
    }
    map
}

/// Encode the `Mcp-Param-*` headers for a `tools/call`, per the cached
/// bindings: absent/null arguments omit their header (per the SEP-2243
/// client-behavior table); unencodable values also omit (the server rejects
/// the call, which is the correct surfacing of a schema-violating argument).
pub(crate) fn encode_param_headers(
    bindings: &[(String, String, String)],
    arguments: &Value,
) -> Vec<(String, String)> {
    let mut headers = Vec::new();
    for (header_name, pointer, _schema_type) in bindings {
        let Some(value) = arguments.pointer(pointer).filter(|v| !v.is_null()) else {
            continue;
        };
        if let Some(encoded) = p::headers::encode_param_value(value) {
            headers.push((format!("Mcp-Param-{header_name}"), encoded));
        }
    }
    headers
}

pub(crate) fn parse_call_tool(
    result: &Value,
) -> McpClientResult<turul_mcp_protocol_2025_11_25::CallToolResult> {
    // MRTR (SEP-2322): an input_required result is not a CallToolResult —
    // surface it so the caller can gather inputs and retry with
    // `call_tool_with_input_responses`.
    check_result_type(result)?;
    let r: p::tools::CallToolResult = serde_json::from_value(result.clone())?;
    remap(&r)
}

pub(crate) fn parse_list_resources(
    result: &Value,
) -> McpClientResult<Vec<turul_mcp_protocol_2025_11_25::Resource>> {
    check_result_type(result)?;
    let r: p::resources::ListResourcesResult = serde_json::from_value(result.clone())?;
    r.resources.iter().map(remap).collect()
}

pub(crate) fn parse_list_resource_templates(
    result: &Value,
) -> McpClientResult<Vec<turul_mcp_protocol_2025_11_25::resources::ResourceTemplate>> {
    check_result_type(result)?;
    let r: p::resources::ListResourceTemplatesResult = serde_json::from_value(result.clone())?;
    r.resource_templates.iter().map(remap).collect()
}

pub(crate) fn parse_read_resource(
    result: &Value,
) -> McpClientResult<turul_mcp_protocol_2025_11_25::ReadResourceResult> {
    // Servers MAY answer resources/read with InputRequiredResult (MRTR) —
    // surface it; retry with `read_resource_with_input_responses`.
    check_result_type(result)?;
    let r: p::resources::ReadResourceResult = serde_json::from_value(result.clone())?;
    remap(&r)
}

pub(crate) fn parse_list_prompts(
    result: &Value,
) -> McpClientResult<Vec<turul_mcp_protocol_2025_11_25::Prompt>> {
    check_result_type(result)?;
    let r: p::prompts::ListPromptsResult = serde_json::from_value(result.clone())?;
    r.prompts.iter().map(remap).collect()
}

pub(crate) fn parse_get_prompt(
    result: &Value,
) -> McpClientResult<turul_mcp_protocol_2025_11_25::GetPromptResult> {
    // Servers MAY answer prompts/get with InputRequiredResult (MRTR) —
    // surface it; retry with `get_prompt_with_input_responses`.
    check_result_type(result)?;
    let r: p::prompts::GetPromptResult = serde_json::from_value(result.clone())?;
    remap(&r)
}

// Full-result parsers for the paginated list ops (preserve `nextCursor`). The
// 2026 result's `resultType`/`ttlMs`/`cacheScope` are dropped by the remap into
// the public (alias) result type.
pub(crate) fn parse_list_tools_result(
    result: &Value,
) -> McpClientResult<turul_mcp_protocol_2025_11_25::ListToolsResult> {
    check_result_type(result)?;
    let r: p::tools::ListToolsResult = serde_json::from_value(result.clone())?;
    remap(&r)
}

pub(crate) fn parse_list_resources_result(
    result: &Value,
) -> McpClientResult<turul_mcp_protocol_2025_11_25::ListResourcesResult> {
    check_result_type(result)?;
    let r: p::resources::ListResourcesResult = serde_json::from_value(result.clone())?;
    remap(&r)
}

pub(crate) fn parse_list_resource_templates_result(
    result: &Value,
) -> McpClientResult<turul_mcp_protocol_2025_11_25::resources::ListResourceTemplatesResult> {
    check_result_type(result)?;
    let r: p::resources::ListResourceTemplatesResult = serde_json::from_value(result.clone())?;
    remap(&r)
}

pub(crate) fn parse_list_prompts_result(
    result: &Value,
) -> McpClientResult<turul_mcp_protocol_2025_11_25::ListPromptsResult> {
    check_result_type(result)?;
    let r: p::prompts::ListPromptsResult = serde_json::from_value(result.clone())?;
    remap(&r)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// basic §Responses: "A resultType of any value unrecognized by the
    /// client MUST be considered invalid."
    #[test]
    fn unrecognized_result_type_is_invalid() {
        assert!(check_result_type(&json!({"resultType": "complete"})).is_ok());
        assert!(check_result_type(&json!({"content": []})).is_ok());
        match check_result_type(&json!({"resultType": "partial"})) {
            Err(crate::error::McpClientError::Protocol(_)) => {}
            other => panic!("unrecognized resultType must be invalid, got: {other:?}"),
        }
    }

    /// Sub-capability declarations reach the wire shapes the gating servers
    /// check: elicitation.url and sampling.tools/.context.
    #[test]
    fn ext_tasks_declaration_rides_request_meta_extensions() {
        let declared = crate::config::DeclaredCapabilities {
            ext_tasks: true,
            ..Default::default()
        };
        let meta = request_meta("t", "1", &declared);
        let v = serde_json::to_value(&meta).unwrap();
        assert_eq!(
            v["io.modelcontextprotocol/clientCapabilities"]["extensions"]["io.modelcontextprotocol/tasks"],
            serde_json::json!({})
        );

        let none = request_meta("t", "1", &Default::default());
        let v = serde_json::to_value(&none).unwrap();
        assert!(
            v["io.modelcontextprotocol/clientCapabilities"]
                .get("extensions")
                .is_none(),
            "no declaration without the opt-in: {v}"
        );
    }

    #[test]
    fn sub_capabilities_map_into_request_meta() {
        let declared = crate::config::DeclaredCapabilities {
            elicitation: true,
            elicitation_url: true,
            sampling: true,
            sampling_tools: true,
            sampling_context: false,
            roots: false,
            ext_tasks: false,
        };
        let meta = request_meta("t", "1", &declared);
        let v = serde_json::to_value(&meta).unwrap();
        let caps = &v["io.modelcontextprotocol/clientCapabilities"];
        assert!(caps["elicitation"]["form"].is_object(), "{caps}");
        assert!(caps["elicitation"]["url"].is_object(), "{caps}");
        assert!(caps["sampling"]["tools"].is_object(), "{caps}");
        assert!(caps["sampling"].get("context").is_none(), "{caps}");

        // url-only still implies the elicitation object with form marker.
        let url_only = crate::config::DeclaredCapabilities {
            elicitation_url: true,
            ..Default::default()
        };
        let v = serde_json::to_value(&request_meta("t", "1", &url_only)).unwrap();
        assert!(v["io.modelcontextprotocol/clientCapabilities"]["elicitation"]["url"].is_object());
    }

    /// "Clients that support sampling MUST declare the sampling capability in
    /// `_meta.io.modelcontextprotocol/clientCapabilities` on each request"
    /// (client/sampling). `sub_capabilities_map_into_request_meta` above only
    /// exercises `sampling: true` combined with `sampling_tools: true`, so the
    /// bare declaration (no sub-capabilities) was never asserted on its own —
    /// this proves declaring plain `sampling` alone still emits the object.
    #[test]
    fn bare_sampling_capability_alone_is_declared_in_request_meta() {
        let declared = crate::config::DeclaredCapabilities {
            sampling: true,
            ..Default::default()
        };
        let meta = request_meta("t", "1", &declared);
        let v = serde_json::to_value(&meta).unwrap();
        let caps = &v["io.modelcontextprotocol/clientCapabilities"];
        assert!(
            caps["sampling"].is_object(),
            "declaring bare `sampling: true` must still emit a sampling capability object: {caps}"
        );
        assert!(caps["sampling"].get("tools").is_none(), "{caps}");
        assert!(caps["sampling"].get("context").is_none(), "{caps}");

        // Without the declaration, no sampling capability rides at all.
        let none = request_meta("t", "1", &Default::default());
        let v = serde_json::to_value(&none).unwrap();
        assert!(
            v["io.modelcontextprotocol/clientCapabilities"]
                .get("sampling")
                .is_none(),
            "no declaration without the opt-in: {v}"
        );
    }

    #[test]
    fn input_required_surfaces_requests_and_state() {
        match check_result_type(&json!({
            "resultType": "input_required",
            "inputRequests": {"q1": {"method": "elicitation/create"}},
            "requestState": "st-1"
        })) {
            Err(crate::error::McpClientError::InputRequired {
                input_requests,
                request_state,
            }) => {
                assert!(input_requests.is_some());
                assert_eq!(request_state.as_deref(), Some("st-1"));
            }
            other => panic!("expected InputRequired, got: {other:?}"),
        }
    }
}
