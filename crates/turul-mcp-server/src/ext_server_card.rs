//! Server Cards (experimental, ADR-038): reconciling a card with the server's
//! `server/discover` data, and serving it at `<mcp_path>/server-card`.

use async_trait::async_trait;
use bytes::Bytes;
use http_body_util::{BodyExt, Empty, Full};
use hyper::{Method, Request, Response, StatusCode};
use turul_http_mcp_server::{RouteBody, RouteHandler};
use turul_mcp_ext_server_card::v1::{MEDIA_TYPE, ServerCard, ServerCardDocument};
use turul_mcp_protocol::initialize::Implementation;

/// Fill the fields `server/discover` also serves from the server, and refuse a
/// card that contradicts them (ADR-038 §2). `supported_versions` is the list
/// `server/discover` answers with.
pub(crate) fn reconcile(
    mut card: ServerCard,
    implementation: &Implementation,
    supported_versions: &[String],
) -> Result<ServerCardDocument, String> {
    fn contradiction(field: &str, card: &str, server: &str) -> String {
        format!(
            "server card {field} {card:?} contradicts server/discover's {server:?}; \
             leave it unset to copy the server's (ADR-038 §2)"
        )
    }
    if card.version.is_empty() {
        card.version = implementation.version.clone();
    } else if card.version != implementation.version {
        return Err(contradiction(
            "version",
            &card.version,
            &implementation.version,
        ));
    }
    if card.description.is_empty() {
        card.description = implementation.description.clone().unwrap_or_default();
    } else if let Some(server) = &implementation.description
        && &card.description != server
    {
        return Err(contradiction("description", &card.description, server));
    }
    fill_or_match("title", &mut card.title, &implementation.title)?;
    fill_or_match(
        "websiteUrl",
        &mut card.website_url,
        &implementation.website_url,
    )?;
    match (&card.icons, &implementation.icons) {
        (None, Some(server)) => card.icons = Some(server.clone()),
        (Some(c), Some(s)) if c != s => {
            return Err(
                "server card icons contradict server/discover's; leave them unset to copy \
                 the server's (ADR-038 §2)"
                    .into(),
            );
        }
        _ => {}
    }
    for remote in card.remotes.iter_mut().flatten() {
        match &remote.supported_protocol_versions {
            None => remote.supported_protocol_versions = Some(supported_versions.to_vec()),
            Some(v) if v.as_slice() != supported_versions => {
                return Err(format!(
                    "server card remote {} supportedProtocolVersions {v:?} contradicts \
                     server/discover's {supported_versions:?} (ADR-038 §2)",
                    remote.url
                ));
            }
            Some(_) => {}
        }
    }
    ServerCardDocument::new(&card).map_err(|e| e.to_string())
}

fn fill_or_match(
    field: &str,
    card: &mut Option<String>,
    server: &Option<String>,
) -> Result<(), String> {
    match (card.as_ref(), server) {
        (None, Some(s)) => *card = Some(s.clone()),
        (Some(c), Some(s)) if c != s => {
            return Err(format!(
                "server card {field} {c:?} contradicts server/discover's {s:?}; \
                 leave it unset to copy the server's (ADR-038 §2)"
            ));
        }
        _ => {}
    }
    Ok(())
}

/// The hosted card. Body and `ETag` are fixed at build.
pub(crate) struct ServerCardRoute {
    pub(crate) document: ServerCardDocument,
}

impl ServerCardRoute {
    fn base(&self, status: StatusCode) -> hyper::http::response::Builder {
        // The four CORS headers docs/discovery.md §CORS Requirements requires.
        Response::builder()
            .status(status)
            .header("Access-Control-Allow-Origin", "*")
            .header("Access-Control-Allow-Methods", "GET")
            .header(
                "Access-Control-Allow-Headers",
                "Content-Type, If-None-Match",
            )
            .header("Access-Control-Expose-Headers", "ETag")
    }

    fn cached(&self, status: StatusCode) -> hyper::http::response::Builder {
        self.base(status)
            .header("ETag", self.document.etag())
            .header("Cache-Control", "public, max-age=3600")
    }

    /// RFC 9110 §13.1.2: `*`, or any listed tag equal under weak comparison.
    fn not_modified(&self, if_none_match: &str) -> bool {
        let ours = self.document.etag();
        if_none_match
            .split(',')
            .map(str::trim)
            .any(|tag| tag == "*" || tag.strip_prefix("W/").unwrap_or(tag) == ours)
    }
}

fn empty() -> RouteBody {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed_unsync()
}

#[async_trait]
impl RouteHandler for ServerCardRoute {
    async fn handle(&self, req: Request<RouteBody>) -> Response<RouteBody> {
        let method = req.method();
        let response = if method == Method::OPTIONS {
            self.base(StatusCode::NO_CONTENT).body(empty())
        } else if method == Method::GET || method == Method::HEAD {
            let matched = req
                .headers()
                .get(hyper::header::IF_NONE_MATCH)
                .and_then(|v| v.to_str().ok())
                .is_some_and(|v| self.not_modified(v));
            if matched {
                self.cached(StatusCode::NOT_MODIFIED).body(empty())
            } else {
                let body = if method == Method::HEAD {
                    empty()
                } else {
                    Full::new(Bytes::copy_from_slice(self.document.body()))
                        .map_err(|never| match never {})
                        .boxed_unsync()
                };
                self.cached(StatusCode::OK)
                    .header("Content-Type", MEDIA_TYPE)
                    .body(body)
            }
        } else {
            self.base(StatusCode::METHOD_NOT_ALLOWED)
                .header("Allow", "GET, HEAD, OPTIONS")
                .body(empty())
        };
        response.expect("static header values are valid")
    }
}
