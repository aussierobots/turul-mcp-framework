//! JSON-RPC 2.0 behavioural parity test through the shim's import paths.
//!
//! The compatibility-test pyramid for the shim is:
//!
//! 1. `shim_compat.rs` — type identity (`turul_mcp_json_rpc_server::T == turul_rpc::T`)
//! 2. `symbol_coverage.rs` — every v0.3.38 public path resolves
//! 3. **this file** — JSON-RPC 2.0 spec scenarios, executed exclusively via
//!    `turul_mcp_json_rpc_server::*` paths, asserting wire-correct responses
//!    per [JSON-RPC 2.0]
//! 4. Framework integration tests — end-to-end MCP transport
//!
//! Logically (1) + (2) + the upstream `turul-rpc-jsonrpc/tests/spec_conformance.rs`
//! suite are sufficient — same nominal type means same behaviour. This file
//! is the regression guard for the case where a future shim author migrates
//! from pure `pub use` to delegation/wrappers and silently changes wire
//! output. If that happens, this file still asserts the wire shape and
//! catches the regression.
//!
//! [JSON-RPC 2.0]: https://www.jsonrpc.org/specification

#![allow(dead_code)]

use serde_json::{Value, json};
use turul_mcp_json_rpc_server::dispatch::{
    JsonRpcMessage as IncomingMessage, parse_json_rpc_message,
};
use turul_mcp_json_rpc_server::error::JsonRpcError;
use turul_mcp_json_rpc_server::error_codes::*;
use turul_mcp_json_rpc_server::types::RequestId;

// -----------------------------------------------------------------------------
// §4.1 — version strictness
// -----------------------------------------------------------------------------

#[test]
fn shim_rejects_jsonrpc_1_0_with_minus_32600() {
    let r = parse_json_rpc_message(r#"{"jsonrpc":"1.0","method":"x","id":1}"#).unwrap_err();
    assert_eq!(r.error.code, INVALID_REQUEST);
}

#[test]
fn shim_rejects_jsonrpc_as_number_with_minus_32600() {
    let r = parse_json_rpc_message(r#"{"jsonrpc":2.0,"method":"x","id":1}"#).unwrap_err();
    assert_eq!(r.error.code, INVALID_REQUEST);
}

// -----------------------------------------------------------------------------
// §4.2 — id rules (strict posture per ADR-002)
// -----------------------------------------------------------------------------

#[test]
fn shim_accepts_string_id() {
    let m = parse_json_rpc_message(r#"{"jsonrpc":"2.0","method":"x","id":"abc"}"#).unwrap();
    match m {
        IncomingMessage::Request(_) => {}
        _ => panic!("expected request"),
    }
}

#[test]
fn shim_accepts_number_id() {
    let m = parse_json_rpc_message(r#"{"jsonrpc":"2.0","method":"x","id":42}"#).unwrap();
    assert!(matches!(m, IncomingMessage::Request(_)));
}

#[test]
fn shim_rejects_null_id_per_strict_posture() {
    // ADR-002 strict departure: null id rejected at parser. Inherited from v0.3.38.
    let r = parse_json_rpc_message(r#"{"jsonrpc":"2.0","method":"x","id":null}"#).unwrap_err();
    assert_eq!(r.error.code, INVALID_REQUEST);
}

#[test]
fn shim_rejects_fractional_id() {
    let r = parse_json_rpc_message(r#"{"jsonrpc":"2.0","method":"x","id":1.5}"#).unwrap_err();
    assert_eq!(r.error.code, INVALID_REQUEST);
}

// -----------------------------------------------------------------------------
// §4.4 — Notification (no id)
// -----------------------------------------------------------------------------

#[test]
fn shim_accepts_notification_no_id() {
    let m = parse_json_rpc_message(r#"{"jsonrpc":"2.0","method":"ping"}"#).unwrap();
    match m {
        IncomingMessage::Notification(n) => assert_eq!(n.method, "ping"),
        _ => panic!("expected notification"),
    }
}

// -----------------------------------------------------------------------------
// §5 / §5.1 — Error responses
// -----------------------------------------------------------------------------

#[test]
fn shim_parse_error_returns_minus_32700_with_null_id() {
    let r = parse_json_rpc_message(r#"{garbage}"#).unwrap_err();
    assert_eq!(r.error.code, PARSE_ERROR);
    assert!(r.id.is_none());

    // Wire-format check: id MUST serialize as JSON null per spec §5.1.
    let v = serde_json::to_value(&r).unwrap();
    assert_eq!(v["id"], Value::Null);
    assert_eq!(v["error"]["code"], json!(PARSE_ERROR));
    assert_eq!(v["jsonrpc"], "2.0");
}

#[test]
fn shim_empty_body_is_parse_error() {
    let r = parse_json_rpc_message("").unwrap_err();
    assert_eq!(r.error.code, PARSE_ERROR);
}

#[test]
fn shim_primitive_body_is_invalid_request() {
    let r = parse_json_rpc_message("42").unwrap_err();
    assert_eq!(r.error.code, INVALID_REQUEST);
}

#[test]
fn shim_invalid_request_echoes_id_when_parseable() {
    let r = parse_json_rpc_message(r#"{"jsonrpc":"2.0","id":7}"#).unwrap_err();
    assert_eq!(r.error.code, INVALID_REQUEST);
    assert_eq!(r.id, Some(RequestId::Number(7)));
}

// -----------------------------------------------------------------------------
// §5.1 — Server error range (constants reachable through the shim)
// -----------------------------------------------------------------------------

#[test]
fn shim_server_error_range_constants() {
    assert_eq!(SERVER_ERROR_START, -32099);
    assert_eq!(SERVER_ERROR_END, -32000);
}

#[test]
fn shim_method_not_found_echoes_id() {
    let e = JsonRpcError::method_not_found(RequestId::String("abc".into()), "missing");
    let v = serde_json::to_value(&e).unwrap();
    assert_eq!(v["id"], "abc");
    assert_eq!(v["error"]["code"], json!(METHOD_NOT_FOUND));
}

// -----------------------------------------------------------------------------
// Async dispatcher round-trip through the shim's import paths
// -----------------------------------------------------------------------------

#[cfg(feature = "async")]
mod dispatcher {
    use async_trait::async_trait;
    use serde_json::{Value, json};
    use turul_mcp_json_rpc_server::r#async::ToJsonRpcError;
    use turul_mcp_json_rpc_server::error::JsonRpcErrorObject;
    use turul_mcp_json_rpc_server::{
        JsonRpcDispatcher, JsonRpcHandler, JsonRpcRequest, RequestId, RequestParams, SessionContext,
    };

    #[derive(thiserror::Error, Debug)]
    enum E {
        #[error("oops")]
        Oops,
    }

    impl ToJsonRpcError for E {
        fn to_error_object(&self) -> JsonRpcErrorObject {
            JsonRpcErrorObject::internal_error(Some(self.to_string()))
        }
    }

    struct H;

    #[async_trait]
    impl JsonRpcHandler for H {
        type Error = E;
        async fn handle(
            &self,
            method: &str,
            _: Option<RequestParams>,
            _: Option<SessionContext>,
        ) -> Result<Value, E> {
            match method {
                "echo" => Ok(json!({"ok": true})),
                "fail" => Err(E::Oops),
                _ => unreachable!(),
            }
        }
    }

    #[tokio::test]
    async fn shim_dispatcher_success_round_trip() {
        let mut d: JsonRpcDispatcher<E> = JsonRpcDispatcher::new();
        d.register_method("echo".into(), H);

        let req = JsonRpcRequest::new_no_params(RequestId::Number(1), "echo".into());
        let resp = d.handle_request(req).await;

        let v = serde_json::to_value(&resp).unwrap();
        assert_eq!(v["jsonrpc"], "2.0");
        assert_eq!(v["id"], 1);
        assert_eq!(v["result"]["ok"], true);
    }

    #[tokio::test]
    async fn shim_dispatcher_method_not_found_echoes_id() {
        let d: JsonRpcDispatcher<E> = JsonRpcDispatcher::new();
        let req = JsonRpcRequest::new_no_params(RequestId::String("xyz".into()), "missing".into());
        let resp = d.handle_request(req).await;

        let v = serde_json::to_value(&resp).unwrap();
        assert_eq!(v["id"], "xyz");
        assert_eq!(v["error"]["code"], -32601);
    }

    #[tokio::test]
    async fn shim_dispatcher_handler_error_maps_via_to_error_object() {
        let mut d: JsonRpcDispatcher<E> = JsonRpcDispatcher::new();
        d.register_method("fail".into(), H);

        let req = JsonRpcRequest::new_no_params(RequestId::Number(99), "fail".into());
        let resp = d.handle_request(req).await;

        let v = serde_json::to_value(&resp).unwrap();
        assert_eq!(v["id"], 99);
        assert_eq!(v["error"]["code"], -32603); // INTERNAL_ERROR per H's ToJsonRpcError impl
        assert_eq!(v["error"]["message"], "oops");
    }

    #[tokio::test]
    async fn shim_handle_batch_two_requests_via_shim_path() {
        // Dispatcher reached through shim path; handle_batch is a method on
        // the re-exported type (additive item per ADR-003 — listed in
        // CHANGELOG [0.3.39] / Added).
        let mut d: JsonRpcDispatcher<E> = JsonRpcDispatcher::new();
        d.register_method("echo".into(), H);

        let body = r#"[
            {"jsonrpc":"2.0","method":"echo","id":1},
            {"jsonrpc":"2.0","method":"echo","id":2}
        ]"#;
        let resp = d.handle_batch(body).await.expect("response body");
        let v: Value = serde_json::from_str(&resp).unwrap();
        let arr = v.as_array().expect("batch response should be JSON array");
        assert_eq!(arr.len(), 2);
        let ids: Vec<&Value> = arr.iter().map(|e| &e["id"]).collect();
        assert!(ids.contains(&&json!(1)));
        assert!(ids.contains(&&json!(2)));
    }

    #[tokio::test]
    async fn shim_handle_batch_all_notifications_returns_no_body() {
        let mut d: JsonRpcDispatcher<E> = JsonRpcDispatcher::new();
        d.register_method("echo".into(), H);

        let body = r#"[
            {"jsonrpc":"2.0","method":"echo"},
            {"jsonrpc":"2.0","method":"echo"}
        ]"#;
        assert!(d.handle_batch(body).await.is_none());
    }
}
