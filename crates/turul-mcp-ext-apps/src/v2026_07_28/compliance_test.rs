//! Wire-shape tests against the vendored SEP-1865 spec types
//! (`schema/spec.types.ts` / `schema/apps-draft.mdx`).

use serde_json::json;
use turul_mcp_protocol_2026_07_28::initialize::ClientCapabilities;

use super::capability::{EXTENSION_IDENTIFIER, client_supports_html_views, declared_by_client};
use super::types::{
    EmptyObject, MCP_APP_HTML_MIME, UiClientCapabilities, UiResourceCsp, UiResourceMeta,
    UiResourcePermissions, UiToolMeta, UiToolVisibility,
};

/// The capability rides `capabilities.extensions["io.modelcontextprotocol/ui"]`
/// with a `mimeTypes` array that must include the mcp-app HTML profile.
#[test]
fn capability_negotiation_shape() {
    let caps: ClientCapabilities = serde_json::from_value(json!({
        "extensions": {
            EXTENSION_IDENTIFIER: { "mimeTypes": ["text/html;profile=mcp-app"] }
        }
    }))
    .unwrap();
    let ui = declared_by_client(&caps).expect("declared");
    assert!(ui.supports_html_views());
    assert!(client_supports_html_views(&caps));

    // Declared but without the HTML profile → no HTML views.
    let caps: ClientCapabilities = serde_json::from_value(json!({
        "extensions": { EXTENSION_IDENTIFIER: { "mimeTypes": ["image/png"] } }
    }))
    .unwrap();
    assert!(!client_supports_html_views(&caps));

    // Not declared at all.
    let caps: ClientCapabilities = serde_json::from_value(json!({})).unwrap();
    assert!(declared_by_client(&caps).is_none());
}

/// Tool `_meta.ui`: `resourceUri` + `visibility` with lowercase wire values.
#[test]
fn tool_meta_wire_shape() {
    let meta = UiToolMeta {
        resource_uri: Some("ui://weather/view.html".to_string()),
        visibility: Some(vec![UiToolVisibility::Model, UiToolVisibility::App]),
    };
    assert_eq!(
        serde_json::to_value(&meta).unwrap(),
        json!({
            "resourceUri": "ui://weather/view.html",
            "visibility": ["model", "app"]
        })
    );
}

/// Resource `_meta.ui`: CSP domain lists keep their camelCase names; the
/// permission keys serialize as empty objects.
#[test]
fn resource_meta_wire_shape() {
    let meta = UiResourceMeta {
        csp: Some(UiResourceCsp {
            connect_domains: Some(vec!["https://api.weather.com".to_string()]),
            resource_domains: Some(vec!["https://cdn.jsdelivr.net".to_string()]),
            frame_domains: None,
            base_uri_domains: None,
        }),
        permissions: Some(UiResourcePermissions {
            camera: None,
            microphone: None,
            geolocation: Some(EmptyObject {}),
            clipboard_write: Some(EmptyObject {}),
        }),
        domain: Some("a904794854a047f6.claudemcpcontent.com".to_string()),
        prefers_border: Some(true),
    };
    assert_eq!(
        serde_json::to_value(&meta).unwrap(),
        json!({
            "csp": {
                "connectDomains": ["https://api.weather.com"],
                "resourceDomains": ["https://cdn.jsdelivr.net"]
            },
            "permissions": { "geolocation": {}, "clipboardWrite": {} },
            "domain": "a904794854a047f6.claudemcpcontent.com",
            "prefersBorder": true
        })
    );
}

/// Round trip through the spec doc's own tool example shape.
#[test]
fn tool_meta_round_trip_from_spec_example() {
    let meta: UiToolMeta = serde_json::from_value(json!({
        "resourceUri": "ui://weather/view.html",
        "visibility": ["model"]
    }))
    .unwrap();
    assert_eq!(meta.resource_uri.as_deref(), Some("ui://weather/view.html"));
    assert_eq!(meta.visibility, Some(vec![UiToolVisibility::Model]));
}

/// The HTML profile constant matches the spec literal.
#[test]
fn html_mime_profile_literal() {
    assert_eq!(MCP_APP_HTML_MIME, "text/html;profile=mcp-app");
    let caps = UiClientCapabilities {
        mime_types: Some(vec![MCP_APP_HTML_MIME.to_string()]),
    };
    assert!(caps.supports_html_views());
}

/// Permission values are STRICTLY empty objects: the upstream generated
/// schema declares `additionalProperties: false` per key, so non-objects and
/// populated objects must be rejected.
#[test]
fn permission_values_must_be_empty_objects() {
    // {} parses
    let ok: UiResourcePermissions =
        serde_json::from_value(json!({ "camera": {} })).expect("empty object accepted");
    assert!(ok.camera.is_some());

    // non-object → rejected
    assert!(serde_json::from_value::<UiResourcePermissions>(json!({ "camera": true })).is_err());
    assert!(
        serde_json::from_value::<UiResourcePermissions>(json!({ "geolocation": "yes" })).is_err()
    );

    // populated object → rejected (additionalProperties: false)
    assert!(
        serde_json::from_value::<UiResourcePermissions>(json!({ "camera": { "hd": true } }))
            .is_err()
    );
}
