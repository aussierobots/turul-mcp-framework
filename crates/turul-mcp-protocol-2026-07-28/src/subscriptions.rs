//! Unified subscription stream for MCP DRAFT-2026-v1.
//!
//! `subscriptions/listen` is the single opt-in channel for long-lived
//! change notifications (the spec's Subscriptions pattern; per the schema it
//! "replaces the former `resources/subscribe` RPC", and on Streamable HTTP it
//! is the only long-lived notification stream — the endpoint accepts POST
//! only).
//!
//! The client opens one long-lived channel with a [`SubscriptionFilter`]
//! declaring which notification types it wants; the server replies with a
//! [`SubscriptionsAcknowledgedNotification`] echoing the subset it agreed to
//! honor, then streams matching notifications inline.
//!
//! All notification types are **opt-in**: the server MUST NOT send any type
//! the client didn't request.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Wire method string for the subscription RPC.
pub const SUBSCRIPTIONS_LISTEN_METHOD: &str = "subscriptions/listen";

/// Wire method string for the acknowledgement notification.
pub const SUBSCRIPTIONS_ACKNOWLEDGED_METHOD: &str = "notifications/subscriptions/acknowledged";

/// Opt-in filter for which notification types the client wants on this stream.
///
/// Each field is independently optional; all-absent means the client wants
/// no notifications (a degenerate but valid case).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionFilter {
    /// Receive `notifications/tools/list_changed`?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools_list_changed: Option<bool>,

    /// Receive `notifications/prompts/list_changed`?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts_list_changed: Option<bool>,

    /// Receive `notifications/resources/list_changed`?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources_list_changed: Option<bool>,

    /// Subscribe to per-resource `notifications/resources/updated` for these
    /// URIs (the former `resources/subscribe` RPC's role; that method has no
    /// binding in this crate's pinned schema).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_subscriptions: Option<Vec<String>>,
}

impl SubscriptionFilter {
    /// Empty filter — server will not send any notifications on this stream.
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_tools_list_changed(mut self, enabled: bool) -> Self {
        self.tools_list_changed = Some(enabled);
        self
    }

    pub fn with_prompts_list_changed(mut self, enabled: bool) -> Self {
        self.prompts_list_changed = Some(enabled);
        self
    }

    pub fn with_resources_list_changed(mut self, enabled: bool) -> Self {
        self.resources_list_changed = Some(enabled);
        self
    }

    pub fn with_resource_subscriptions(mut self, uris: Vec<String>) -> Self {
        self.resource_subscriptions = Some(uris);
        self
    }
}

/// Params for `subscriptions/listen` — `SubscriptionsListenRequestParams
/// extends RequestParams`, so `_meta` is the typed [`crate::meta::RequestMetaObject`]
/// carrying the per-request capability negotiation (protocol version, client
/// info, client capabilities). Required by schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionsListenRequestParams {
    /// Notifications the client opts in to on this stream.
    pub notifications: SubscriptionFilter,

    /// Schema-typed `_meta` per `RequestMetaObject`. Required.
    #[serde(rename = "_meta")]
    pub meta: crate::meta::RequestMetaObject,
}

impl SubscriptionsListenRequestParams {
    /// Construct with the required filter and per-request meta.
    pub fn new(notifications: SubscriptionFilter, meta: crate::meta::RequestMetaObject) -> Self {
        Self {
            notifications,
            meta,
        }
    }

    /// Replace the per-request meta.
    pub fn with_meta(mut self, meta: crate::meta::RequestMetaObject) -> Self {
        self.meta = meta;
        self
    }
}

/// `subscriptions/listen` request.
///
/// The `jsonrpc`/`id` envelope is supplied by [`JsonRpcRequest`](crate::JsonRpcRequest)
/// when wrapped for the wire (existing crate convention).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionsListenRequest {
    /// Always `"subscriptions/listen"`.
    pub method: String,
    pub params: SubscriptionsListenRequestParams,
}

impl SubscriptionsListenRequest {
    /// Construct with a filter and the required per-request meta.
    pub fn new(filter: SubscriptionFilter, meta: crate::meta::RequestMetaObject) -> Self {
        Self {
            method: SUBSCRIPTIONS_LISTEN_METHOD.to_string(),
            params: SubscriptionsListenRequestParams::new(filter, meta),
        }
    }

    /// Attach a fully-constructed params struct.
    pub fn with_params(mut self, params: SubscriptionsListenRequestParams) -> Self {
        self.params = params;
        self
    }
}

/// Params for the acknowledgement notification.
///
/// `notifications` is the subset of the client's requested filter that the
/// server agreed to honor — types the server doesn't support are omitted
/// (e.g. `promptsListChanged` if the server has no prompts).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionsAcknowledgedNotificationParams {
    pub notifications: SubscriptionFilter,

    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

impl SubscriptionsAcknowledgedNotificationParams {
    pub fn new(notifications: SubscriptionFilter) -> Self {
        Self {
            notifications,
            meta: None,
        }
    }
}

/// `notifications/subscriptions/acknowledged` — sent by the server as the first
/// message on a `subscriptions/listen` stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionsAcknowledgedNotification {
    /// Always `"notifications/subscriptions/acknowledged"`.
    pub method: String,
    pub params: SubscriptionsAcknowledgedNotificationParams,
}

impl SubscriptionsAcknowledgedNotification {
    pub fn new(notifications: SubscriptionFilter) -> Self {
        Self {
            method: SUBSCRIPTIONS_ACKNOWLEDGED_METHOD.to_string(),
            params: SubscriptionsAcknowledgedNotificationParams::new(notifications),
        }
    }
}

// Trait impls: `SubscriptionsListenRequest` satisfies
// `RpcRequest + SubscriptionsListenRequestTrait`.
impl crate::traits::Params for SubscriptionsListenRequestParams {}
impl crate::traits::HasSubscriptionsListenParams for SubscriptionsListenRequestParams {
    fn notifications(&self) -> &SubscriptionFilter {
        &self.notifications
    }
}
impl crate::traits::HasMethod for SubscriptionsListenRequest {
    fn method(&self) -> &str {
        &self.method
    }
}
impl crate::traits::HasParams for SubscriptionsListenRequest {
    fn params(&self) -> Option<&dyn crate::traits::Params> {
        Some(&self.params as &dyn crate::traits::Params)
    }
}
impl crate::traits::RpcRequest for SubscriptionsListenRequest {}
impl crate::traits::SubscriptionsListenRequestTrait for SubscriptionsListenRequest {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_request_meta() -> crate::meta::RequestMetaObject {
        crate::meta::RequestMetaObject::new(
            "DRAFT-2026-v1",
            crate::initialize::Implementation::new("test-client", "1.0.0"),
            crate::initialize::ClientCapabilities::default(),
        )
    }

    #[test]
    fn listen_method_constant_matches_schema() {
        assert_eq!(SUBSCRIPTIONS_LISTEN_METHOD, "subscriptions/listen");
    }

    #[test]
    fn acknowledged_method_constant_matches_schema() {
        assert_eq!(
            SUBSCRIPTIONS_ACKNOWLEDGED_METHOD,
            "notifications/subscriptions/acknowledged"
        );
    }

    #[test]
    fn listen_request_serializes_method() {
        let req = SubscriptionsListenRequest::new(
            SubscriptionFilter::new().with_tools_list_changed(true),
            test_request_meta(),
        );
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(v["method"], "subscriptions/listen");
        assert_eq!(v["params"]["notifications"]["toolsListChanged"], true);
        assert_eq!(
            v["params"]["_meta"]["io.modelcontextprotocol/protocolVersion"],
            "DRAFT-2026-v1"
        );
    }

    #[test]
    fn acknowledged_notification_serializes_method() {
        let n = SubscriptionsAcknowledgedNotification::new(
            SubscriptionFilter::new().with_prompts_list_changed(true),
        );
        let v = serde_json::to_value(&n).unwrap();
        assert_eq!(v["method"], "notifications/subscriptions/acknowledged");
        assert_eq!(v["params"]["notifications"]["promptsListChanged"], true);
    }

    #[test]
    fn filter_camelcase_field_names() {
        let f = SubscriptionFilter::new()
            .with_tools_list_changed(true)
            .with_prompts_list_changed(false)
            .with_resources_list_changed(true)
            .with_resource_subscriptions(vec!["file:///a.txt".to_string()]);
        let v = serde_json::to_value(&f).unwrap();
        assert_eq!(v["toolsListChanged"], true);
        assert_eq!(v["promptsListChanged"], false);
        assert_eq!(v["resourcesListChanged"], true);
        assert_eq!(v["resourceSubscriptions"][0], "file:///a.txt");
    }

    #[test]
    fn filter_omits_absent_fields() {
        let f = SubscriptionFilter::new();
        let v = serde_json::to_value(&f).unwrap();
        let obj = v.as_object().unwrap();
        assert!(obj.is_empty(), "empty filter serializes to empty object");
    }

    #[test]
    fn filter_round_trips() {
        let f = SubscriptionFilter::new()
            .with_resource_subscriptions(vec!["file:///x".to_string(), "file:///y".to_string()]);
        let s = serde_json::to_string(&f).unwrap();
        let parsed: SubscriptionFilter = serde_json::from_str(&s).unwrap();
        assert_eq!(
            parsed.resource_subscriptions.unwrap(),
            vec!["file:///x".to_string(), "file:///y".to_string()]
        );
        assert!(parsed.tools_list_changed.is_none());
    }

    #[test]
    fn ack_filter_can_be_subset_of_request_filter() {
        // Server may honor a strict subset of the requested filter.
        let _request_filter = SubscriptionFilter::new()
            .with_tools_list_changed(true)
            .with_prompts_list_changed(true)
            .with_resources_list_changed(true);

        // Server response — drops the unsupported prompts type:
        let ack_filter = SubscriptionFilter::new()
            .with_tools_list_changed(true)
            .with_resources_list_changed(true);

        let n = SubscriptionsAcknowledgedNotification::new(ack_filter);
        let v = serde_json::to_value(&n).unwrap();
        assert_eq!(v["params"]["notifications"]["toolsListChanged"], true);
        assert_eq!(v["params"]["notifications"]["resourcesListChanged"], true);
        assert!(
            !v["params"]["notifications"]
                .as_object()
                .unwrap()
                .contains_key("promptsListChanged"),
            "unsupported type omitted in acknowledgement"
        );
    }

    #[test]
    fn listen_params_meta_required_on_wire() {
        // _meta is a required typed field on the wire — no longer omittable.
        let p =
            SubscriptionsListenRequestParams::new(SubscriptionFilter::new(), test_request_meta());
        let v = serde_json::to_value(&p).unwrap();
        assert!(v.as_object().unwrap().contains_key("_meta"));
        assert_eq!(
            v["_meta"]["io.modelcontextprotocol/protocolVersion"],
            "DRAFT-2026-v1"
        );
    }

    #[test]
    fn listen_request_round_trips_from_wire_example() {
        // Schema-conformant example with the required `_meta`.
        let wire = json!({
            "method": "subscriptions/listen",
            "params": {
                "notifications": {
                    "toolsListChanged": true,
                    "resourcesListChanged": true
                },
                "_meta": {
                    "io.modelcontextprotocol/protocolVersion": "DRAFT-2026-v1",
                    "io.modelcontextprotocol/clientInfo": {
                        "name": "test-client",
                        "version": "1.0.0"
                    },
                    "io.modelcontextprotocol/clientCapabilities": {}
                }
            }
        });
        let r: SubscriptionsListenRequest = serde_json::from_value(wire).unwrap();
        assert_eq!(r.method, "subscriptions/listen");
        assert_eq!(r.params.notifications.tools_list_changed, Some(true));
        assert_eq!(r.params.notifications.resources_list_changed, Some(true));
        assert!(r.params.notifications.prompts_list_changed.is_none());
        assert_eq!(r.params.meta.protocol_version, "DRAFT-2026-v1");
    }

    #[test]
    fn listen_request_satisfies_new_rpc_trait() {
        // Generic function over the trait abstraction (A8).
        fn method_via_trait<R: crate::traits::SubscriptionsListenRequestTrait>(r: &R) -> &str {
            r.method_string()
        }
        let req = SubscriptionsListenRequest::new(
            SubscriptionFilter::new().with_tools_list_changed(true),
            test_request_meta(),
        );
        assert_eq!(method_via_trait(&req), "subscriptions/listen");

        // Field-getter via HasSubscriptionsListenParams on the params struct.
        let n: &SubscriptionFilter =
            crate::traits::HasSubscriptionsListenParams::notifications(&req.params);
        assert_eq!(n.tools_list_changed, Some(true));
    }

    #[test]
    fn listen_request_rejects_missing_meta() {
        // Pre-fix shape (no `_meta`) must now fail to deserialize.
        let wire = json!({
            "method": "subscriptions/listen",
            "params": {
                "notifications": { "toolsListChanged": true }
            }
        });
        assert!(serde_json::from_value::<SubscriptionsListenRequest>(wire).is_err());
    }
}
