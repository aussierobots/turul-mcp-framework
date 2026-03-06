//! `.well-known/oauth-protected-resource` route handler
//!
//! Serves the RFC 9728 Protected Resource Metadata document at the
//! well-known path, enabling clients to discover the authorization server.

use async_trait::async_trait;
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{Request, Response, StatusCode};

use turul_http_mcp_server::routes::{RouteBody, RouteHandler};

use crate::metadata::ProtectedResourceMetadata;

/// Route handler that serves the Protected Resource Metadata document
pub struct WellKnownOAuthHandler {
    metadata_json: String,
}

impl WellKnownOAuthHandler {
    /// Create a new handler from metadata
    pub fn new(metadata: &ProtectedResourceMetadata) -> Self {
        Self {
            metadata_json: serde_json::to_string(metadata)
                .expect("ProtectedResourceMetadata must be serializable"),
        }
    }
}

#[async_trait]
impl RouteHandler for WellKnownOAuthHandler {
    async fn handle(&self, _req: Request<RouteBody>) -> Response<RouteBody> {
        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .header("Cache-Control", "public, max-age=3600")
            .body(
                Full::new(Bytes::from(self.metadata_json.clone()))
                    .map_err(|never| match never {})
                    .boxed_unsync(),
            )
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // T28: Well-known endpoint returns metadata
    #[tokio::test]
    async fn test_well_known_endpoint_returns_metadata() {
        let metadata = ProtectedResourceMetadata::new(
            "https://example.com/mcp",
            vec!["https://auth.example.com".to_string()],
        );

        let handler = WellKnownOAuthHandler::new(&metadata);

        // Create a dummy request
        let req = Request::builder()
            .uri("/.well-known/oauth-protected-resource")
            .body(
                Full::new(Bytes::new())
                    .map_err(|never| match never {})
                    .boxed_unsync(),
            )
            .unwrap();

        let resp = handler.handle(req).await;

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get("Content-Type").unwrap(),
            "application/json"
        );

        // Parse the body
        let body_bytes = http_body_util::BodyExt::collect(resp.into_body())
            .await
            .unwrap()
            .to_bytes();
        let parsed: ProtectedResourceMetadata = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(parsed.resource, "https://example.com/mcp");
    }

    // Verify path-form endpoint serves same metadata as root-form
    #[tokio::test]
    async fn test_path_form_endpoint_returns_same_metadata() {
        let metadata = ProtectedResourceMetadata::new(
            "https://example.com/mcp",
            vec!["https://auth.example.com".to_string()],
        );

        // Same handler instance serves both forms (shared via Arc)
        let handler = WellKnownOAuthHandler::new(&metadata);

        // Request to path-form endpoint
        let req = Request::builder()
            .uri("/.well-known/oauth-protected-resource/mcp")
            .body(
                Full::new(Bytes::new())
                    .map_err(|never| match never {})
                    .boxed_unsync(),
            )
            .unwrap();

        let resp = handler.handle(req).await;

        assert_eq!(resp.status(), StatusCode::OK);

        let body_bytes = http_body_util::BodyExt::collect(resp.into_body())
            .await
            .unwrap()
            .to_bytes();
        let parsed: ProtectedResourceMetadata = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(parsed.resource, "https://example.com/mcp");
        assert_eq!(
            parsed.authorization_servers,
            vec!["https://auth.example.com".to_string()]
        );
    }
}
