//! Multi-round-trip input requests for MCP DRAFT-2026-v1 (SEP-2322).
//!
//! In the stateless 2026 model, servers no longer hold persistent SSE streams
//! open for elicitation/sampling/roots prompts. Instead a server returns
//! [`InputRequiredResult`] from any normal request, the client gathers
//! responses and re-issues the original call with [`InputResponseRequestParams`]
//! carrying `inputResponses` and an echoed `requestState`. Any server instance
//! can process the retry.
//!
//! Maps directly to schema:
//! - `InputRequest`        → [`InputRequest`]
//! - `InputResponse`       → [`InputResponse`]
//! - `InputRequests`       → [`InputRequests`] (`HashMap<String, InputRequest>`)
//! - `InputResponses`      → [`InputResponses`] (`HashMap<String, InputResponse>`)
//! - `InputRequiredResult` → [`InputRequiredResult`]
//! - `InputResponseRequestParams` → [`InputResponseRequestParams`]

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::elicitation::ElicitRequest;
use crate::meta::MetaObject;
use crate::result_type::ResultType;
#[allow(deprecated)]
use crate::roots::{ListRootsRequest, ListRootsResult};
#[allow(deprecated)]
use crate::sampling::{CreateMessageRequest, CreateMessageResult};

/// Server → client request emitted as part of an [`InputRequiredResult`].
///
/// `type InputRequest = CreateMessageRequest | ListRootsRequest | ElicitRequest`.
///
/// Serialized untagged at the Rust level; the custom [`Deserialize`] impl
/// dispatches on the wire `method` string (`sampling/createMessage`,
/// `roots/list`, `elicitation/create`) and rejects any other value, so the
/// schema's union discriminator is enforced explicitly rather than relying
/// on first-variant-wins fallback.
///
/// **Note**: `CreateMessageRequest` (Sampling) and `ListRootsRequest` (Roots)
/// are themselves deprecated per SEP-2577 in DRAFT-2026-v1. They remain valid
/// variants of `InputRequest` during the 12-month migration window so servers
/// can continue to ask clients for sampling/roots input via the MRTR pattern
/// (SEP-2322). After the deprecated features are removed, the corresponding
/// variants will be removed from this enum.
#[allow(deprecated)]
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum InputRequest {
    /// `sampling/createMessage` request. **Deprecated** per SEP-2577.
    CreateMessage(CreateMessageRequest),
    /// `roots/list` request. **Deprecated** per SEP-2577.
    ListRoots(ListRootsRequest),
    /// `elicitation/create` request.
    Elicit(ElicitRequest),
}

#[allow(deprecated)]
impl<'de> Deserialize<'de> for InputRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        let method = value
            .get("method")
            .and_then(|m| m.as_str())
            .ok_or_else(|| {
                serde::de::Error::custom(
                    "InputRequest payload missing required `method` discriminator",
                )
            })?;
        match method {
            "sampling/createMessage" => {
                serde_json::from_value::<CreateMessageRequest>(value)
                    .map(InputRequest::CreateMessage)
                    .map_err(serde::de::Error::custom)
            }
            "roots/list" => serde_json::from_value::<ListRootsRequest>(value)
                .map(InputRequest::ListRoots)
                .map_err(serde::de::Error::custom),
            "elicitation/create" => serde_json::from_value::<ElicitRequest>(value)
                .map(InputRequest::Elicit)
                .map_err(serde::de::Error::custom),
            other => Err(serde::de::Error::custom(format!(
                "InputRequest method `{other}` is not one of `sampling/createMessage`, `roots/list`, `elicitation/create`"
            ))),
        }
    }
}

/// Client → server response for an [`InputRequest`], keyed by the server-assigned
/// identifier from the corresponding [`InputRequests`] entry.
///
/// `type InputResponse = CreateMessageResult | ListRootsResult | ElicitResult`.
///
/// **Note**: `CreateMessageResult` and `ListRootsResult` variants reference
/// types deprecated per SEP-2577. They remain valid during the deprecation
/// window so clients can respond to legacy server-initiated requests.
#[allow(deprecated)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InputResponse {
    /// Response to a `sampling/createMessage` request. **Deprecated** per SEP-2577.
    CreateMessage(CreateMessageResult),
    /// Response to a `roots/list` request. **Deprecated** per SEP-2577.
    ListRoots(ListRootsResult),
    /// Response to an `elicitation/create` request.
    Elicit(crate::elicitation::ElicitResult),
}

/// Map of server-assigned identifiers → server-initiated requests.
///
/// `interface InputRequests { [key: string]: InputRequest }`.
pub type InputRequests = HashMap<String, InputRequest>;

/// Map of identifiers → client responses, keyed identically to the originating [`InputRequests`].
///
/// `interface InputResponses { [key: string]: InputResponse }`.
pub type InputResponses = HashMap<String, InputResponse>;

/// A `Result` discriminated as `"input_required"` — server signals that
/// additional input must be gathered and the original request re-issued.
///
/// Invariant: at least one of `inputRequests` or `requestState` MUST be
/// present. Enforced by both the public constructors and the custom
/// `Deserialize` impl — any JSON missing both fields is rejected at parse time.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputRequiredResult {
    /// Discriminator — always [`ResultType::InputRequired`].
    ///
    /// Serialized as `"resultType": "input_required"`.
    #[serde(default)]
    pub result_type: ResultType,

    /// Requests the client must fulfill before retrying the original call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_requests: Option<InputRequests>,

    /// Opaque blob the client must echo verbatim in the retry's
    /// [`InputResponseRequestParams::request_state`].
    ///
    /// > The client must treat this as an opaque blob; it must not interpret
    /// > it in any way. — schema comment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_state: Option<String>,

    /// Optional `_meta` per `Result` schema.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaObject>,
}

impl InputRequiredResult {
    /// Construct with only `inputRequests`; `requestState` absent.
    pub fn with_requests(requests: InputRequests) -> Self {
        Self {
            result_type: ResultType::InputRequired,
            input_requests: Some(requests),
            request_state: None,
            meta: None,
        }
    }

    /// Construct with only `requestState` (load-shedding / pure-state pattern,
    /// per schema example "input-required-result-with-request-state-only").
    pub fn with_state(state: impl Into<String>) -> Self {
        Self {
            result_type: ResultType::InputRequired,
            input_requests: None,
            request_state: Some(state.into()),
            meta: None,
        }
    }

    /// Construct with both `inputRequests` and `requestState`.
    pub fn with_requests_and_state(requests: InputRequests, state: impl Into<String>) -> Self {
        Self {
            result_type: ResultType::InputRequired,
            input_requests: Some(requests),
            request_state: Some(state.into()),
            meta: None,
        }
    }

    /// Attach `_meta`.
    pub fn with_meta(mut self, meta: MetaObject) -> Self {
        self.meta = Some(meta);
        self
    }

    /// Per schema: "At least one of `inputRequests` or `requestState`
    /// MUST be present." Verify this invariant.
    pub fn is_well_formed(&self) -> bool {
        self.input_requests.is_some() || self.request_state.is_some()
    }
}

// Trait impls: `InputRequiredResult` satisfies `HasResultType + HasMeta +
// HasInputRequiredResult` so consumers can dispatch on `resultType` and pull
// the structured fields generically.
impl crate::traits::HasResultType for InputRequiredResult {
    fn result_type(&self) -> ResultType {
        self.result_type
    }
}
impl crate::traits::HasMeta for InputRequiredResult {
    fn meta(&self) -> Option<&MetaObject> {
        self.meta.as_ref()
    }
}
impl crate::traits::HasInputRequiredResult for InputRequiredResult {
    fn input_requests(&self) -> Option<&InputRequests> {
        self.input_requests.as_ref()
    }
    fn request_state(&self) -> Option<&str> {
        self.request_state.as_deref()
    }
    fn meta(&self) -> Option<&MetaObject> {
        self.meta.as_ref()
    }
}

impl<'de> Deserialize<'de> for InputRequiredResult {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Raw {
            #[serde(default)]
            result_type: ResultType,
            #[serde(default)]
            input_requests: Option<InputRequests>,
            #[serde(default)]
            request_state: Option<String>,
            #[serde(rename = "_meta", default)]
            meta: Option<MetaObject>,
        }

        let r = Raw::deserialize(deserializer)?;
        if r.input_requests.is_none() && r.request_state.is_none() {
            return Err(serde::de::Error::custom(
                "InputRequiredResult must include at least one of `inputRequests` or `requestState`",
            ));
        }
        Ok(Self {
            result_type: r.result_type,
            input_requests: r.input_requests,
            request_state: r.request_state,
            meta: r.meta,
        })
    }
}

/// Mixin shape that may appear on the `params` of any client-initiated request,
/// allowing the client to attach prior input responses + echoed request state.
///
/// `InputResponseRequestParams extends RequestParams`. Extended by
/// `CallToolRequestParams`, `ReadResourceRequestParams`, `GetPromptRequestParams`.
///
/// **NOTE**: this struct only models the **mixin fields** (`inputResponses?`,
/// `requestState?`). It does NOT carry the required `_meta: RequestMetaObject`
/// of [`RequestParams`](crate::json_rpc::RequestParams) — embed it alongside
/// `RequestParams` (the schema's extension model maps to Rust composition).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputResponseRequestParams {
    /// Responses to a prior [`InputRequiredResult`]. Keys match the original
    /// `inputRequests` map.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_responses: Option<InputResponses>,

    /// Verbatim echo of [`InputRequiredResult::request_state`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_state: Option<String>,
}

impl InputResponseRequestParams {
    /// Construct with only responses.
    pub fn with_responses(responses: InputResponses) -> Self {
        Self {
            input_responses: Some(responses),
            request_state: None,
        }
    }

    /// Construct with only echoed state.
    pub fn with_state(state: impl Into<String>) -> Self {
        Self {
            input_responses: None,
            request_state: Some(state.into()),
        }
    }

    /// Construct with both.
    pub fn with_responses_and_state(responses: InputResponses, state: impl Into<String>) -> Self {
        Self {
            input_responses: Some(responses),
            request_state: Some(state.into()),
        }
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn input_required_result_with_requests_only_is_well_formed() {
        let mut reqs = HashMap::new();
        reqs.insert(
            "rq-1".to_string(),
            InputRequest::ListRoots(ListRootsRequest::new()),
        );
        let r = InputRequiredResult::with_requests(reqs);
        assert!(r.is_well_formed());
        assert_eq!(r.result_type, ResultType::InputRequired);
    }

    #[test]
    fn input_required_result_with_state_only_is_well_formed() {
        let r = InputRequiredResult::with_state("opaque-blob");
        assert!(r.is_well_formed());
        assert_eq!(r.result_type, ResultType::InputRequired);
        assert_eq!(r.request_state.as_deref(), Some("opaque-blob"));
        assert!(r.input_requests.is_none());
    }

    #[test]
    fn input_required_result_field_getters_via_trait() {
        // Drive through the `HasInputRequiredResult` trait (A8). Returns the
        // same field bodies as direct struct access.
        use crate::traits::HasInputRequiredResult;
        let r = InputRequiredResult::with_state("opaque-blob");
        assert_eq!(
            crate::traits::HasResultType::result_type(&r),
            ResultType::InputRequired
        );
        assert_eq!(HasInputRequiredResult::request_state(&r), Some("opaque-blob"));
        assert!(HasInputRequiredResult::input_requests(&r).is_none());
        assert!(HasInputRequiredResult::meta(&r).is_none());
    }

    #[test]
    fn input_required_result_serializes_result_type_field() {
        let r = InputRequiredResult::with_state("s");
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["resultType"], "input_required", "discriminator must be on wire");
        assert_eq!(v["requestState"], "s");
        assert!(
            !v.as_object().unwrap().contains_key("inputRequests"),
            "absent inputRequests omitted"
        );
        assert!(
            !v.as_object().unwrap().contains_key("_meta"),
            "absent _meta omitted"
        );
    }

    #[test]
    fn input_required_result_round_trips() {
        let r = InputRequiredResult::with_state("opaque").with_meta(MetaObject::new());
        let s = serde_json::to_string(&r).unwrap();
        let parsed: InputRequiredResult = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed.result_type, ResultType::InputRequired);
        assert_eq!(parsed.request_state.as_deref(), Some("opaque"));
    }

    #[test]
    fn input_response_request_params_omits_none_fields() {
        let p = InputResponseRequestParams::with_state("state-x");
        let v = serde_json::to_value(&p).unwrap();
        assert_eq!(v["requestState"], "state-x");
        assert!(!v.as_object().unwrap().contains_key("inputResponses"));
    }

    #[test]
    fn input_response_request_params_round_trips_with_responses() {
        let mut responses = HashMap::new();
        responses.insert(
            "rq-1".to_string(),
            InputResponse::ListRoots(ListRootsResult::new(vec![])),
        );
        let p = InputResponseRequestParams::with_responses_and_state(responses, "s");
        let v = serde_json::to_value(&p).unwrap();
        assert_eq!(v["requestState"], "s");
        assert!(v["inputResponses"]["rq-1"].is_object());

        // Round-trip
        let s = serde_json::to_string(&p).unwrap();
        let parsed: InputResponseRequestParams = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed.request_state.as_deref(), Some("s"));
        assert!(parsed.input_responses.is_some());
        assert_eq!(parsed.input_responses.unwrap().len(), 1);
    }

    #[test]
    fn input_request_list_roots_serializes_with_method_string() {
        let req = InputRequest::ListRoots(ListRootsRequest::new());
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(
            v["method"], "roots/list",
            "ListRoots variant must serialize with the underlying request's method"
        );
    }

    #[test]
    fn input_requests_map_keyed_by_server_assigned_id() {
        let mut reqs: InputRequests = HashMap::new();
        reqs.insert(
            "rq-1".to_string(),
            InputRequest::ListRoots(ListRootsRequest::new()),
        );
        let v = serde_json::to_value(&reqs).unwrap();
        assert!(v["rq-1"].is_object());
        assert_eq!(v["rq-1"]["method"], "roots/list");
    }

    #[test]
    fn input_required_result_parses_minimum_shape() {
        // Wire JSON example from schema "input-required-result-with-request-state-only":
        let wire = json!({
            "resultType": "input_required",
            "requestState": "opaque-server-state"
        });
        let r: InputRequiredResult = serde_json::from_value(wire).unwrap();
        assert_eq!(r.result_type, ResultType::InputRequired);
        assert_eq!(r.request_state.as_deref(), Some("opaque-server-state"));
        assert!(r.input_requests.is_none());
    }
}
