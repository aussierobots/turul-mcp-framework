//! Wire-shape tests against the vendored SEP-1865 spec
//! (`schema/apps-2026-01-26.mdx`, with `schema/spec.types.ts` as the
//! machine-readable companion).

use serde_json::json;
use turul_mcp_protocol_2026_07_28::initialize::ClientCapabilities;

use turul_mcp_protocol_2026_07_28::initialize::ServerCapabilities;

use super::capability::{
    EXTENSION_IDENTIFIER, capability, client_supports_html_views, declared_by_client,
    declared_by_server,
};
use super::types::{
    EmptyObject, MCP_APP_HTML_MIME, META_KEY_UI, UI_URI_SCHEME, UiClientCapabilities,
    UiResourceCsp, UiResourceMeta, UiResourcePermissions, UiToolMeta, UiToolVisibility,
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

/// The server's extension entry is `{}` (core schema: "an empty object
/// indicates support with no settings"), and `declared_by_server` reads it back.
#[test]
fn server_capability_is_empty_object() {
    assert_eq!(capability(), json!({}));
    let caps: ServerCapabilities = serde_json::from_value(json!({
        "extensions": { EXTENSION_IDENTIFIER: capability() }
    }))
    .unwrap();
    assert!(declared_by_server(&caps));
    let caps: ServerCapabilities = serde_json::from_value(json!({})).unwrap();
    assert!(!declared_by_server(&caps));
}

/// `UiToolMeta::to_meta` reproduces both tool `_meta` examples in the spec
/// (apps-2026-01-26.mdx, "Example (tool visible to both model and app)" and
/// "Example (app-only tool, hidden from model)"), and never emits the
/// deprecated flat `ui/resourceUri` key.
#[test]
fn tool_to_meta_matches_spec_examples() {
    let both = UiToolMeta {
        resource_uri: Some("ui://weather-server/dashboard-template".into()),
        visibility: Some(vec![UiToolVisibility::Model, UiToolVisibility::App]),
    }
    .to_meta();
    assert_eq!(
        serde_json::to_value(&both).unwrap(),
        json!({ "ui": {
            "resourceUri": "ui://weather-server/dashboard-template",
            "visibility": ["model", "app"]
        }})
    );

    let app_only = UiToolMeta {
        resource_uri: Some("ui://weather-server/dashboard-template".into()),
        visibility: Some(vec![UiToolVisibility::App]),
    }
    .to_meta();
    assert_eq!(
        serde_json::to_value(&app_only).unwrap(),
        json!({ "ui": {
            "resourceUri": "ui://weather-server/dashboard-template",
            "visibility": ["app"]
        }})
    );

    assert_eq!(both.len(), 1);
    assert!(both.contains_key(META_KEY_UI));
    assert!(!both.contains_key("ui/resourceUri"));
}

/// `UiResourceMeta::to_meta` reproduces the `_meta` of the spec's
/// "Resource content with metadata" example.
#[test]
fn resource_to_meta_matches_spec_example() {
    let meta = UiResourceMeta {
        csp: Some(UiResourceCsp {
            connect_domains: Some(vec!["https://api.openweathermap.org".into()]),
            resource_domains: Some(vec!["https://cdn.jsdelivr.net".into()]),
            ..Default::default()
        }),
        prefers_border: Some(true),
        ..Default::default()
    }
    .to_meta();
    assert_eq!(
        serde_json::to_value(&meta).unwrap(),
        json!({ "ui": {
            "csp": {
                "connectDomains": ["https://api.openweathermap.org"],
                "resourceDomains": ["https://cdn.jsdelivr.net"]
            },
            "prefersBorder": true
        }})
    );
}

/// The URI scheme constant matches the spec's `ui://` requirement and its examples.
#[test]
fn ui_uri_scheme_literal() {
    assert_eq!(UI_URI_SCHEME, "ui://");
    assert!("ui://weather-server/dashboard-template".starts_with(UI_URI_SCHEME));
}

/// `from_meta` is the inverse of `to_meta`: absent `ui` is `Ok(None)`, a
/// malformed one is an error rather than a silent `None`.
#[test]
fn ui_meta_from_meta_round_trips_and_rejects_malformed() {
    let tool = UiToolMeta {
        resource_uri: Some("ui://weather-server/dashboard-template".into()),
        visibility: Some(vec![UiToolVisibility::App]),
    };
    assert_eq!(UiToolMeta::from_meta(&tool.to_meta()).unwrap(), Some(tool));

    let resource = UiResourceMeta {
        prefers_border: Some(true),
        ..Default::default()
    };
    assert_eq!(
        UiResourceMeta::from_meta(&resource.to_meta()).unwrap(),
        Some(resource)
    );

    let empty = std::collections::HashMap::new();
    assert_eq!(UiToolMeta::from_meta(&empty).unwrap(), None);

    let bad: std::collections::HashMap<String, serde_json::Value> = [(
        META_KEY_UI.to_string(),
        json!({ "visibility": ["everyone"] }),
    )]
    .into();
    assert!(UiToolMeta::from_meta(&bad).is_err());
}

/// Apps §Visibility: "`visibility` defaults to `["model", "app"]` if omitted";
/// `"model"` = visible to the agent, `"app"` = callable by the app.
#[test]
fn apps_visibility_defaults_to_model_and_app_when_omitted() {
    let omitted = UiToolMeta::default();
    assert!(omitted.is_model_visible() && omitted.is_app_callable());

    let app_only = UiToolMeta {
        visibility: Some(vec![UiToolVisibility::App]),
        ..Default::default()
    };
    assert!(!app_only.is_model_visible() && app_only.is_app_callable());

    let model_only = UiToolMeta {
        visibility: Some(vec![UiToolVisibility::Model]),
        ..Default::default()
    };
    assert!(model_only.is_model_visible() && !model_only.is_app_callable());
}
