//! Bearer token extraction utilities (D5, D6)
//!
//! Provides hardened Bearer token parsing and scheme detection
//! shared across all transports.

/// Check if an Authorization header uses the Bearer scheme (case-insensitive).
///
/// Returns true for ANY "Bearer ..." header, even if the token value is malformed.
/// Used for D5 metadata exclusion — Bearer NEVER enters metadata.
pub fn is_bearer_scheme(auth_header: &str) -> bool {
    let trimmed = auth_header.trim();
    let scheme = trimmed.split_ascii_whitespace().next().unwrap_or("");
    scheme.eq_ignore_ascii_case("bearer")
}

/// Hardened Bearer token parser (D6).
///
/// Extracts a valid Bearer token from an Authorization header value.
/// Uses safe split-based parsing (no byte indexing), case-insensitive scheme
/// matching, OWS handling, and rejects malformed values including whitespace
/// and ASCII control characters.
pub fn extract_bearer_token(auth_header: &str) -> Option<String> {
    let trimmed = auth_header.trim();

    // Safe split on first whitespace — no byte indexing, no panic on non-ASCII
    let (scheme, rest) = trimmed.split_once([' ', '\t'])?;

    // Case-insensitive scheme check (RFC 7235 §2.1)
    if !scheme.eq_ignore_ascii_case("bearer") {
        return None;
    }

    // Trim OWS from token value (RFC 7235 §2.1 allows optional whitespace)
    let token = rest.trim();

    // Reject empty tokens
    if token.is_empty() {
        return None;
    }

    // Reject tokens containing ANY whitespace (multi-token)
    if token.contains(|c: char| c.is_ascii_whitespace()) {
        return None;
    }

    // Reject tokens containing ANY ASCII control characters (0x00-0x1F, 0x7F)
    if token.contains(|c: char| c.is_ascii_control()) {
        return None;
    }

    Some(token.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bearer_extraction_from_header() {
        assert_eq!(
            extract_bearer_token("Bearer abc123"),
            Some("abc123".to_string())
        );
        assert_eq!(
            extract_bearer_token("Bearer eyJhbGciOiJSUzI1NiJ9.eyJzdWIiOiIxIn0.sig"),
            Some("eyJhbGciOiJSUzI1NiJ9.eyJzdWIiOiIxIn0.sig".to_string())
        );
    }

    #[test]
    fn test_bearer_case_insensitive() {
        assert_eq!(
            extract_bearer_token("bearer abc123"),
            Some("abc123".to_string())
        );
        assert_eq!(
            extract_bearer_token("BEARER abc123"),
            Some("abc123".to_string())
        );
        assert_eq!(
            extract_bearer_token("BeArEr abc123"),
            Some("abc123".to_string())
        );
    }

    #[test]
    fn test_bearer_reject_empty_token() {
        assert_eq!(extract_bearer_token("Bearer "), None);
        assert_eq!(extract_bearer_token("Bearer  "), None);
    }

    #[test]
    fn test_bearer_reject_multi_token() {
        assert_eq!(extract_bearer_token("Bearer abc 123"), None);
        assert_eq!(extract_bearer_token("Bearer abc\t123"), None);
    }

    #[test]
    fn test_bearer_ows_handling() {
        assert_eq!(
            extract_bearer_token("Bearer  abc123"),
            Some("abc123".to_string())
        );
        assert_eq!(
            extract_bearer_token("Bearer\tabc123"),
            Some("abc123".to_string())
        );
        assert_eq!(
            extract_bearer_token("  Bearer abc123  "),
            Some("abc123".to_string())
        );
    }

    #[test]
    fn test_bearer_reject_control_chars() {
        assert_eq!(extract_bearer_token("Bearer abc\x00123"), None);
        assert_eq!(extract_bearer_token("Bearer abc\x01123"), None);
        assert_eq!(extract_bearer_token("Bearer abc\x7f123"), None);
    }

    #[test]
    fn test_non_bearer_not_extracted() {
        assert_eq!(extract_bearer_token("Basic dXNlcjpwYXNz"), None);
        assert_eq!(extract_bearer_token("Digest realm=\"mcp\""), None);
    }

    #[test]
    fn test_is_bearer_scheme() {
        assert!(is_bearer_scheme("Bearer abc123"));
        assert!(is_bearer_scheme("bearer abc123"));
        assert!(is_bearer_scheme("BEARER abc123"));
        assert!(is_bearer_scheme("Bearer ")); // Malformed but still Bearer scheme
        assert!(!is_bearer_scheme("Basic dXNlcjpwYXNz"));
        assert!(!is_bearer_scheme("Digest realm=\"mcp\""));
        assert!(!is_bearer_scheme(""));
    }
}
