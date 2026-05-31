//! Coverage table: one [`Case`] per upstream `schema/draft/examples/*` directory.
//!
//! Each [`Case`] either binds a fixture to a Rust type ([`Kind`] != `NotModeled`,
//! `parse_and_reserialize` parses the JSON into that type and re-serializes for
//! diffing) or is explicitly unmodeled (`NotModeled`, harness skips it but it
//! counts toward the "not yet covered" tally).
//!
//! The table is asserted in-sync with the upstream tree by
//! [`super::roundtrip::assert_table_matches_upstream`] — if upstream adds or
//! removes a directory the harness fails until the table is updated.

use serde_json::Value;

/// Tag classifying what level of the MCP wire the fixture represents.
/// Determines how the binary's report groups failures; does not affect the
/// round-trip itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Inner shape only — neither envelope nor params wrapper.
    Struct,
    /// `*Params` sub-shape inside a request envelope.
    Params,
    /// `*Request` — has `method` field, may have `params`.
    Request,
    /// `*Result` — inner result object (without JSON-RPC envelope).
    Result,
    /// `*Response` — full JSON-RPC response envelope (`{jsonrpc, id, result}`).
    Response,
    /// `*Notification` — full JSON-RPC notification envelope (`{jsonrpc, method, params?}`).
    Notification,
    /// JSON-RPC `error` object on its own (no envelope).
    ErrorEnvelope,
    /// JSON Schema 2020-12 fragment (boolean/string/number/enum schema).
    JsonSchemaFragment,
    /// Explicitly unmodeled. Harness skips and counts toward unmodeled tally.
    NotModeled,
}

/// One upstream example directory and its Rust binding.
#[derive(Debug, Clone, Copy)]
pub struct Case {
    /// Upstream PascalCase directory name (e.g. `"CallToolRequest"`).
    pub dir: &'static str,
    /// Classification.
    pub kind: Kind,
    /// Parse the upstream JSON into the bound Rust type, then re-serialize.
    /// Returns the re-serialized JSON for semantic diff against the upstream.
    /// For `NotModeled`, always returns `Err("not modeled")` and is never called
    /// by the harness (skipped by Kind check).
    pub parse_and_reserialize: fn(&str) -> Result<Value, String>,
}

fn not_modeled(_raw: &str) -> Result<Value, String> {
    Err("not modeled".to_string())
}

/// Generic helper: parse into `T`, then re-serialize. The error string is
/// prefixed with the type name so failures point straight at the binding.
fn roundtrip<T>(raw: &str) -> Result<Value, String>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let parsed: T = serde_json::from_str(raw)
        .map_err(|e| format!("parse as {}: {e}", std::any::type_name::<T>()))?;
    serde_json::to_value(&parsed)
        .map_err(|e| format!("re-serialize {}: {e}", std::any::type_name::<T>()))
}

// ============================================================
// CASES table — 86 entries, sorted lexicographically to match
// `list_example_dirs` output for trivial diffing.
// ============================================================

/// One [`Case`] per upstream example directory at the pinned SHA. Sorted.
pub const CASES: &[Case] = &[
    Case { dir: "AudioContent",                        kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "BlobResourceContents",                kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "BooleanSchema",                       kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "CallToolRequest",                     kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "CallToolRequestParams",               kind: Kind::Params,     parse_and_reserialize: roundtrip::<crate::tools::CallToolRequestParams> },
    Case { dir: "CallToolResult",                      kind: Kind::Result,     parse_and_reserialize: roundtrip::<crate::tools::CallToolResult> },
    Case { dir: "CallToolResultResponse",              kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "CancelledNotification",               kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "CancelledNotificationParams",         kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "ClientCapabilities",                  kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "CompleteRequest",                     kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "CompleteRequestParams",               kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "CompleteResult",                      kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "CompleteResultResponse",              kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "CreateMessageRequest",                kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "CreateMessageRequestParams",          kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "CreateMessageResult",                 kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "DiscoverRequest",                     kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "DiscoverResult",                      kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "DiscoverResultResponse",              kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "ElicitRequest",                       kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "ElicitRequestFormParams",             kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "ElicitRequestURLParams",              kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "ElicitResult",                        kind: Kind::Result,     parse_and_reserialize: roundtrip::<crate::elicitation::ElicitResult> },
    Case { dir: "ElicitationCompleteNotification",     kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "EmbeddedResource",                    kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "GetPromptRequest",                    kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "GetPromptRequestParams",              kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "GetPromptResult",                     kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "GetPromptResultResponse",             kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "ImageContent",                        kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "InputRequests",                       kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "InputRequiredResult",                 kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "InputResponses",                      kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "InternalError",                       kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "InvalidParamsError",                  kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "ListPromptsRequest",                  kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "ListPromptsResult",                   kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "ListPromptsResultResponse",           kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "ListResourceTemplatesRequest",        kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "ListResourceTemplatesResult",         kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "ListResourceTemplatesResultResponse", kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "ListResourcesRequest",                kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "ListResourcesResult",                 kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "ListResourcesResultResponse",         kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "ListRootsRequest",                    kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    // SEP-2577-deprecated; binding retained during the migration window so
    // upstream fixtures continue to round-trip.
    #[allow(deprecated)]
    Case { dir: "ListRootsResult",                     kind: Kind::Result,     parse_and_reserialize: roundtrip::<crate::roots::ListRootsResult> },
    // ListToolsRequest upstream fixture is the FULL JSON-RPC envelope
    // (`{jsonrpc, id, method, params}`). Our `tools::ListToolsRequest`
    // models only the inner `{method, params?}` — binding requires
    // `JsonRpcRequest<...>` and a `_meta` that preserves the namespaced
    // negotiation keys. Tracked for a later slice.
    Case { dir: "ListToolsRequest",                    kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "ListToolsResult",                     kind: Kind::Result,     parse_and_reserialize: roundtrip::<crate::tools::ListToolsResult> },
    Case { dir: "ListToolsResultResponse",             kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "LoggingMessageNotification",          kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "LoggingMessageNotificationParams",    kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "MethodNotFoundError",                 kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "MissingRequiredClientCapabilityError",kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "ModelPreferences",                    kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "NumberSchema",                        kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "PaginatedRequestParams",              kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "ParseError",                          kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "ProgressNotification",                kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "ProgressNotificationParams",          kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "PromptListChangedNotification",       kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "ReadResourceRequest",                 kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "ReadResourceResult",                  kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "ReadResourceResultResponse",          kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "Resource",                            kind: Kind::Struct,     parse_and_reserialize: roundtrip::<crate::resources::Resource> },
    Case { dir: "ResourceLink",                        kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "ResourceListChangedNotification",     kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "ResourceUpdatedNotification",         kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "ResourceUpdatedNotificationParams",   kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    // SEP-2577-deprecated; binding retained during the migration window.
    #[allow(deprecated)]
    Case { dir: "Root",                                kind: Kind::Struct,     parse_and_reserialize: roundtrip::<crate::roots::Root> },
    Case { dir: "SamplingMessage",                     kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "ServerCapabilities",                  kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "StringSchema",                        kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "SubscriptionsAcknowledgedNotification", kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "SubscriptionsListenRequest",          kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "TextContent",                         kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "TextResourceContents",                kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "TitledMultiSelectEnumSchema",         kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "TitledSingleSelectEnumSchema",        kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "Tool",                                kind: Kind::Struct,     parse_and_reserialize: roundtrip::<crate::tools::Tool> },
    Case { dir: "ToolListChangedNotification",         kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "ToolResultContent",                   kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "ToolUseContent",                      kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "UnsupportedProtocolVersionError",     kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "UntitledMultiSelectEnumSchema",       kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
    Case { dir: "UntitledSingleSelectEnumSchema",      kind: Kind::NotModeled, parse_and_reserialize: not_modeled },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cases_table_is_sorted_and_unique() {
        let names: Vec<&str> = CASES.iter().map(|c| c.dir).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted, "CASES must be sorted lexicographically");

        let set: std::collections::BTreeSet<_> = names.iter().collect();
        assert_eq!(set.len(), names.len(), "CASES must have unique dirs");
    }

    #[test]
    fn cases_table_has_86_entries() {
        // Hard count — matches the upstream tree at the pinned SHA.
        assert_eq!(CASES.len(), 86);
    }
}
