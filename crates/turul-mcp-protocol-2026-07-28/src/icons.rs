//! MCP Icon Types
//!
//! Icons are display hints for tools, resources, prompts, and implementations.
//! Most implementations do not need icons — they are optional visual enhancements.
//!
//! See [MCP spec](https://modelcontextprotocol.io/specification/2025-11-25)

use serde::{Deserialize, Serialize};

/// Theme preference for an icon (light or dark mode).
/// See [MCP spec](https://modelcontextprotocol.io/specification/2025-11-25)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum IconTheme {
    Light,
    Dark,
}

/// Icon for tools, resources, prompts, and implementations.
/// Icons are display hints — most implementations do not need icons.
/// See [MCP spec](https://modelcontextprotocol.io/specification/2025-11-25)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Icon {
    /// Icon source URL (data: URI or https:// URL)
    pub src: String,
    /// MIME type of the icon (e.g., "image/png", "image/svg+xml")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// Icon sizes (e.g., ["16x16", "32x32"])
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sizes: Option<Vec<String>>,
    /// Theme preference for this icon
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<IconTheme>,
}

impl Icon {
    /// Create an icon from a URL
    pub fn new(src: impl Into<String>) -> Self {
        Self {
            src: src.into(),
            mime_type: None,
            sizes: None,
            theme: None,
        }
    }

    /// Create an icon from a data: URI
    pub fn data_uri(mime_type: &str, base64_data: &str) -> Self {
        Self {
            src: format!("data:{};base64,{}", mime_type, base64_data),
            mime_type: Some(mime_type.to_string()),
            sizes: None,
            theme: None,
        }
    }

    pub fn with_mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = Some(mime_type.into());
        self
    }

    pub fn with_sizes(mut self, sizes: Vec<String>) -> Self {
        self.sizes = Some(sizes);
        self
    }

    pub fn with_theme(mut self, theme: IconTheme) -> Self {
        self.theme = Some(theme);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_icon_new() {
        let icon = Icon::new("https://example.com/icon.png");
        assert_eq!(icon.src, "https://example.com/icon.png");
        assert!(icon.mime_type.is_none());
        assert!(icon.sizes.is_none());
        assert!(icon.theme.is_none());
    }

    #[test]
    fn test_icon_data_uri() {
        let icon = Icon::data_uri("image/png", "iVBORw0KGgo=");
        assert_eq!(icon.src, "data:image/png;base64,iVBORw0KGgo=");
        assert_eq!(icon.mime_type, Some("image/png".to_string()));
    }

    #[test]
    fn test_icon_serialization_camel_case() {
        let icon = Icon::new("https://example.com/icon.png")
            .with_mime_type("image/png")
            .with_sizes(vec!["16x16".to_string(), "32x32".to_string()])
            .with_theme(IconTheme::Dark);

        let json = serde_json::to_value(&icon).unwrap();
        // Verify camelCase field names
        assert_eq!(json["src"], "https://example.com/icon.png");
        assert_eq!(json["mimeType"], "image/png");
        assert_eq!(json["sizes"], json!(["16x16", "32x32"]));
        assert_eq!(json["theme"], "dark");
        // Verify NO snake_case leak
        assert!(json.get("mime_type").is_none());
    }

    #[test]
    fn test_icon_round_trip() {
        let icon = Icon::new("https://example.com/icon.png")
            .with_mime_type("image/svg+xml")
            .with_theme(IconTheme::Light);

        let json = serde_json::to_string(&icon).unwrap();
        let parsed: Icon = serde_json::from_str(&json).unwrap();
        assert_eq!(icon, parsed);
    }

    #[test]
    fn test_icon_minimal_serialization() {
        let icon = Icon::new("https://example.com/icon.png");
        let json = serde_json::to_value(&icon).unwrap();
        // Only src should be present, optional fields skipped
        assert_eq!(json, json!({"src": "https://example.com/icon.png"}));
    }

    #[test]
    fn test_icon_theme_serialization() {
        assert_eq!(
            serde_json::to_value(IconTheme::Light).unwrap(),
            json!("light")
        );
        assert_eq!(
            serde_json::to_value(IconTheme::Dark).unwrap(),
            json!("dark")
        );
    }

    #[test]
    fn test_icons_array_serialization() {
        let icons = vec![
            Icon::new("https://example.com/light.png").with_theme(IconTheme::Light),
            Icon::new("https://example.com/dark.png").with_theme(IconTheme::Dark),
        ];
        let json = serde_json::to_value(&icons).unwrap();
        assert!(json.is_array());
        assert_eq!(json.as_array().unwrap().len(), 2);
        assert_eq!(json[0]["theme"], "light");
        assert_eq!(json[1]["theme"], "dark");
    }
}
