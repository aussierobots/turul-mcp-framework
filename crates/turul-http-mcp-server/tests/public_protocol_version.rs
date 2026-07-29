//! The version type reachable from the crate root and the prelude must be the
//! same one the transport uses, and must know every version this build serves.
//! A second definition inside a transport module previously left the exported
//! type unable to parse the crate's own default spec version.

#[test]
fn root_export_parses_every_supported_version() {
    use turul_http_mcp_server::McpProtocolVersion;
    for v in [
        "2024-11-05",
        "2025-03-26",
        "2025-06-18",
        "2025-11-25",
        "2026-07-28",
    ] {
        let parsed = McpProtocolVersion::parse_version(v)
            .unwrap_or_else(|| panic!("root export cannot parse {v}"));
        assert_eq!(parsed.as_str(), v);
    }
    assert_eq!(McpProtocolVersion::parse_version("nonsense"), None);
}

#[test]
fn prelude_export_is_the_same_type_as_the_root_export() {
    use turul_http_mcp_server::prelude::McpProtocolVersion as FromPrelude;
    use turul_http_mcp_server::McpProtocolVersion as FromRoot;
    // Compiles only if both paths name one type; a second definition breaks it.
    let v: FromRoot = FromPrelude::parse_version("2026-07-28").expect("prelude parses 2026-07-28");
    assert_eq!(v.as_str(), "2026-07-28");
}

#[test]
fn latest_tracks_the_enabled_spec_lane() {
    use turul_http_mcp_server::McpProtocolVersion;
    #[cfg(feature = "protocol-2026-07-28")]
    assert_eq!(McpProtocolVersion::LATEST.as_str(), "2026-07-28");
    #[cfg(not(feature = "protocol-2026-07-28"))]
    assert_eq!(McpProtocolVersion::LATEST.as_str(), "2025-11-25");
}
