//! CORS (Cross-Origin Resource Sharing) support

use hyper::HeaderMap;

/// CORS layer for adding appropriate headers
pub struct CorsLayer;

impl CorsLayer {
    /// Apply CORS headers to a response
    pub fn apply_cors_headers(headers: &mut HeaderMap) {
        headers.insert("Access-Control-Allow-Origin", "*".parse().unwrap());
        headers.insert(
            "Access-Control-Allow-Methods",
            "GET, POST, OPTIONS".parse().unwrap(),
        );
        headers.insert(
            "Access-Control-Allow-Headers",
            "Content-Type, Accept, Authorization".parse().unwrap(),
        );
        headers.insert("Access-Control-Max-Age", "86400".parse().unwrap());
    }

    /// Apply restrictive CORS headers for a specific origin
    pub fn apply_cors_headers_for_origin(headers: &mut HeaderMap, origin: &str) {
        headers.insert("Access-Control-Allow-Origin", origin.parse().unwrap());
        headers.insert(
            "Access-Control-Allow-Methods",
            "GET, POST, OPTIONS".parse().unwrap(),
        );
        headers.insert(
            "Access-Control-Allow-Headers",
            "Content-Type, Accept, Authorization".parse().unwrap(),
        );
        headers.insert("Access-Control-Allow-Credentials", "true".parse().unwrap());
        headers.insert("Access-Control-Max-Age", "86400".parse().unwrap());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_cors_headers() {
        let mut headers = HeaderMap::new();
        CorsLayer::apply_cors_headers(&mut headers);

        assert_eq!(headers.get("Access-Control-Allow-Origin").unwrap(), "*");
        assert!(headers.contains_key("Access-Control-Allow-Methods"));
        assert!(headers.contains_key("Access-Control-Allow-Headers"));
        assert!(headers.contains_key("Access-Control-Max-Age"));
    }

    #[test]
    fn test_apply_cors_headers_for_origin() {
        let mut headers = HeaderMap::new();
        CorsLayer::apply_cors_headers_for_origin(&mut headers, "https://example.com");

        assert_eq!(
            headers.get("Access-Control-Allow-Origin").unwrap(),
            "https://example.com"
        );
        assert_eq!(
            headers.get("Access-Control-Allow-Credentials").unwrap(),
            "true"
        );
    }
}
