//! OAuth Client Credentials (ADR-039): the HTTP transport's token flow against a
//! wiremock authorization server, and end to end against a real resource server.
#![cfg(feature = "oauth-client-credentials")]

use std::sync::{Arc, LazyLock};

use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Validation};
use serde_json::{Value, json};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

use turul_mcp_client::McpClientError;
use turul_mcp_client::oauth_client_credentials::{AssertionAudience, ClientCredentials};
use turul_mcp_client::transport::Transport;
use turul_mcp_client::transport::http::HttpTransport;

const CLIENT_ID: &str = "svc client";
const SECRET: &str = "s3cr:et/é";
const KID: &str = "kid-1";

/// (signing key DER, JWKS n, JWKS e)
static RSA_KEY: LazyLock<(Vec<u8>, String, String)> = LazyLock::new(|| {
    use rsa::pkcs1::EncodeRsaPrivateKey;
    use rsa::traits::PublicKeyParts;
    let private = rsa::PrivateKey::new(&mut rand::rngs::ThreadRng::default(), 2048).unwrap();
    let der = private.to_pkcs1_der().unwrap().as_bytes().to_vec();
    let public = rsa::PublicKey::from(&private);
    let n = URL_SAFE_NO_PAD.encode(be_bytes(public.n().as_ref()));
    let e = URL_SAFE_NO_PAD.encode(be_bytes(public.e()));
    (der, n, e)
});

fn be_bytes(v: &crypto_bigint::BoxedUint) -> Vec<u8> {
    let bytes = v.to_be_bytes();
    let first = bytes
        .iter()
        .position(|b| *b != 0)
        .unwrap_or(bytes.len().saturating_sub(1));
    bytes[first..].to_vec()
}

fn signing_key() -> EncodingKey {
    EncodingKey::from_rsa_der(&RSA_KEY.0)
}

fn decoding_key() -> DecodingKey {
    DecodingKey::from_rsa_components(&RSA_KEY.1, &RSA_KEY.2).unwrap()
}

fn ping() -> Value {
    json!({"jsonrpc": "2.0", "id": "1", "method": "ping", "params": {}})
}

fn ok() -> ResponseTemplate {
    ResponseTemplate::new(200)
        .insert_header("Content-Type", "application/json")
        .set_body_json(json!({"jsonrpc": "2.0", "id": "1", "result": {}}))
}

/// A wiremock origin serving the MCP endpoint, its PRM and an authorization server.
struct World {
    server: MockServer,
}

impl World {
    async fn new() -> Self {
        Self {
            server: MockServer::start().await,
        }
    }
    fn uri(&self) -> String {
        self.server.uri()
    }
    fn endpoint(&self) -> String {
        format!("{}/mcp", self.uri())
    }
    fn prm_url(&self) -> String {
        format!("{}/.well-known/oauth-protected-resource/mcp", self.uri())
    }

    /// `/mcp`: 200 for `Bearer <token>`, else 401 with `challenge`.
    async fn mcp(&self, token: &str, challenge: &str) {
        Mock::given(method("POST"))
            .and(path("/mcp"))
            .and(header("Authorization", format!("Bearer {token}").as_str()))
            .respond_with(ok())
            .with_priority(1)
            .mount(&self.server)
            .await;
        Mock::given(method("POST"))
            .and(path("/mcp"))
            .respond_with(ResponseTemplate::new(401).insert_header("WWW-Authenticate", challenge))
            .with_priority(10)
            .mount(&self.server)
            .await;
    }

    fn challenge(&self) -> String {
        format!(r#"Bearer resource_metadata="{}""#, self.prm_url())
    }

    async fn prm(&self, body: Value) {
        Mock::given(method("GET"))
            .and(path("/.well-known/oauth-protected-resource/mcp"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&self.server)
            .await;
    }

    fn default_prm(&self) -> Value {
        json!({
            "resource": self.endpoint(),
            "authorization_servers": [self.uri()],
            "scopes_supported": ["mcp:read", "mcp:write"],
        })
    }

    async fn as_metadata(&self, extra: Value) {
        let mut body = json!({
            "issuer": self.uri(),
            "token_endpoint": format!("{}/token", self.uri()),
        });
        body.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        Mock::given(method("GET"))
            .and(path("/.well-known/oauth-authorization-server"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&self.server)
            .await;
    }

    async fn token<R: Respond + 'static>(&self, responder: R) {
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(responder)
            .mount(&self.server)
            .await;
    }

    async fn token_requests(&self) -> Vec<Request> {
        self.server
            .received_requests()
            .await
            .unwrap()
            .into_iter()
            .filter(|r| r.url.path() == "/token")
            .collect()
    }

    async fn transport(&self, creds: ClientCredentials) -> HttpTransport {
        let t = HttpTransport::new(&self.endpoint())
            .unwrap()
            .with_oauth_client_credentials(creds)
            .unwrap();
        t.connect().await.unwrap();
        t
    }

    fn secret(&self) -> ClientCredentials {
        ClientCredentials::client_secret(self.uri(), CLIENT_ID, SECRET)
    }
}

fn token_ok(token: &str, expires_in: Option<u64>) -> ResponseTemplate {
    let mut body = json!({"access_token": token, "token_type": "Bearer"});
    if let Some(e) = expires_in {
        body["expires_in"] = json!(e);
    }
    ResponseTemplate::new(200).set_body_json(body)
}

fn form(req: &Request) -> std::collections::HashMap<String, String> {
    url::form_urlencoded::parse(&req.body)
        .into_owned()
        .collect()
}

/// Issues `tok-1`, `tok-2`, ... in order.
struct Counting(std::sync::atomic::AtomicUsize);
impl Respond for Counting {
    fn respond(&self, _: &Request) -> ResponseTemplate {
        let n = self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
        token_ok(&format!("tok-{n}"), Some(3600))
    }
}

#[tokio::test]
async fn client_secret_basic_full_flow() {
    let w = World::new().await;
    w.mcp("tok-1", &w.challenge()).await;
    w.prm(w.default_prm()).await;
    w.as_metadata(json!({"token_endpoint_auth_methods_supported": ["client_secret_basic"]}))
        .await;
    w.token(token_ok("tok-1", Some(3600))).await;

    let t = w.transport(w.secret().with_scope("mcp:read")).await;
    t.send_request(ping()).await.expect("authorized retry");

    let reqs = w.token_requests().await;
    assert_eq!(reqs.len(), 1);
    let basic = reqs[0]
        .headers
        .get("authorization")
        .unwrap()
        .to_str()
        .unwrap();
    let decoded = String::from_utf8(
        STANDARD
            .decode(basic.strip_prefix("Basic ").unwrap())
            .unwrap(),
    )
    .unwrap();
    // RFC 6749 §2.3.1: form-urlencoded before base64.
    assert_eq!(decoded, "svc+client:s3cr%3Aet%2F%C3%A9");
    let f = form(&reqs[0]);
    assert_eq!(f["grant_type"], "client_credentials");
    assert_eq!(f["resource"], w.endpoint());
    assert_eq!(f["scope"], "mcp:read");
    assert!(!f.contains_key("client_secret"));
    assert!(!f.contains_key("client_id"));
}

#[tokio::test]
async fn client_secret_post_when_basic_is_not_offered() {
    let w = World::new().await;
    w.mcp("tok-1", &w.challenge()).await;
    w.prm(w.default_prm()).await;
    w.as_metadata(json!({"token_endpoint_auth_methods_supported": ["client_secret_post"]}))
        .await;
    w.token(token_ok("tok-1", None)).await;

    let t = w.transport(w.secret()).await;
    t.send_request(ping()).await.unwrap();

    let reqs = w.token_requests().await;
    assert!(reqs[0].headers.get("authorization").is_none());
    let f = form(&reqs[0]);
    assert_eq!(f["client_id"], CLIENT_ID);
    assert_eq!(f["client_secret"], SECRET);
    // Scope falls back to the PRM's scopes_supported.
    assert_eq!(f["scope"], "mcp:read mcp:write");
}

#[tokio::test]
async fn private_key_jwt_assertion() {
    for (audience, expect_aud) in [
        (AssertionAudience::Issuer, None),
        (AssertionAudience::TokenEndpoint, Some("/token")),
    ] {
        let w = World::new().await;
        w.mcp("tok-1", &w.challenge()).await;
        w.prm(w.default_prm()).await;
        w.as_metadata(json!({
            "token_endpoint_auth_methods_supported": ["private_key_jwt"],
            "token_endpoint_auth_signing_alg_values_supported": ["RS256"],
        }))
        .await;
        w.token(token_ok("tok-1", None)).await;

        let creds =
            ClientCredentials::private_key_jwt(w.uri(), CLIENT_ID, signing_key(), Algorithm::RS256)
                .with_kid(KID)
                .with_assertion_audience(audience);
        let t = w.transport(creds).await;
        t.send_request(ping()).await.unwrap();

        let f = form(&w.token_requests().await[0]);
        assert!(!f.contains_key("client_id"), "RFC 7523 §3");
        assert_eq!(
            f["client_assertion_type"],
            "urn:ietf:params:oauth:client-assertion-type:jwt-bearer"
        );
        let aud = match expect_aud {
            None => w.uri(),
            Some(p) => format!("{}{p}", w.uri()),
        };
        let mut v = Validation::new(Algorithm::RS256);
        v.set_audience(&[&aud]);
        v.set_issuer(&[CLIENT_ID]);
        v.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
        let data =
            jsonwebtoken::decode::<Value>(&f["client_assertion"], &decoding_key(), &v).unwrap();
        assert_eq!(data.header.kid.as_deref(), Some(KID));
        assert_eq!(data.claims["sub"], CLIENT_ID);
        assert!(data.claims["jti"].as_str().is_some_and(|j| j.len() == 32));
        let (iat, exp) = (
            data.claims["iat"].as_u64().unwrap(),
            data.claims["exp"].as_u64().unwrap(),
        );
        assert_eq!(exp - iat, 60);
    }
}

#[tokio::test]
async fn private_key_jwt_refused_when_alg_is_not_accepted() {
    let w = World::new().await;
    w.mcp("tok-1", &w.challenge()).await;
    w.prm(w.default_prm()).await;
    w.as_metadata(json!({
        "token_endpoint_auth_methods_supported": ["private_key_jwt"],
        "token_endpoint_auth_signing_alg_values_supported": ["ES256"],
    }))
    .await;
    let creds =
        ClientCredentials::private_key_jwt(w.uri(), CLIENT_ID, signing_key(), Algorithm::RS256);
    let t = w.transport(creds).await;
    let err = t.send_request(ping()).await.unwrap_err();
    assert!(
        matches!(err, McpClientError::Auth(ref m) if m.contains("RS256")),
        "{err}"
    );
    assert!(w.token_requests().await.is_empty());
}

#[tokio::test]
async fn credentials_go_only_to_their_own_issuer() {
    // The PRM names another authorization server.
    let w = World::new().await;
    w.mcp("tok-1", &w.challenge()).await;
    w.prm(json!({
        "resource": w.endpoint(),
        "authorization_servers": ["https://attacker.example"],
    }))
    .await;
    w.as_metadata(json!({})).await;
    w.token(token_ok("tok-1", None)).await;
    let t = w.transport(w.secret()).await;
    let err = t.send_request(ping()).await.unwrap_err();
    assert!(
        matches!(err, McpClientError::Auth(ref m) if m.contains("does not list")),
        "{err}"
    );
    // The flow stopped before any authorization server request.
    assert!(
        w.server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .all(|r| !r.url.path().contains("authorization-server") && r.url.path() != "/token")
    );

    // AS metadata whose issuer differs from the credentials'.
    let w = World::new().await;
    w.mcp("tok-1", &w.challenge()).await;
    w.prm(w.default_prm()).await;
    w.as_metadata(json!({"issuer": "https://other.example"}))
        .await;
    w.token(token_ok("tok-1", None)).await;
    let t = w.transport(w.secret()).await;
    let err = t.send_request(ping()).await.unwrap_err();
    assert!(
        matches!(err, McpClientError::Auth(ref m) if m.contains("names issuer")),
        "{err}"
    );
    assert!(w.token_requests().await.is_empty());
}

#[tokio::test]
async fn prm_for_another_resource_is_refused() {
    let w = World::new().await;
    w.mcp("tok-1", &w.challenge()).await;
    w.prm(json!({
        "resource": "https://elsewhere.example/mcp",
        "authorization_servers": [w.uri()],
    }))
    .await;
    let t = w.transport(w.secret()).await;
    let err = t.send_request(ping()).await.unwrap_err();
    assert!(
        matches!(err, McpClientError::Auth(ref m) if m.contains("names resource")),
        "{err}"
    );
}

#[tokio::test]
async fn challenge_scope_and_well_known_fallback() {
    // No resource_metadata in the challenge: the path-form well-known PRM is used,
    // and the challenge's scope beats the PRM's scopes_supported.
    let w = World::new().await;
    w.mcp("tok-1", r#"Bearer scope="mcp:write""#).await;
    w.prm(w.default_prm()).await;
    w.as_metadata(json!({})).await;
    w.token(token_ok("tok-1", None)).await;
    let t = w.transport(w.secret()).await;
    t.send_request(ping()).await.unwrap();
    assert_eq!(form(&w.token_requests().await[0])["scope"], "mcp:write");
}

#[tokio::test]
async fn openid_configuration_is_the_fallback() {
    let w = World::new().await;
    w.mcp("tok-1", &w.challenge()).await;
    w.prm(w.default_prm()).await;
    Mock::given(method("GET"))
        .and(path("/.well-known/openid-configuration"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "issuer": w.uri(),
            "token_endpoint": format!("{}/token", w.uri()),
        })))
        .mount(&w.server)
        .await;
    w.token(token_ok("tok-1", None)).await;
    let t = w.transport(w.secret()).await;
    t.send_request(ping()).await.unwrap();
}

#[tokio::test]
async fn token_error_response_is_reported() {
    let w = World::new().await;
    w.mcp("tok-1", &w.challenge()).await;
    w.prm(w.default_prm()).await;
    w.as_metadata(json!({})).await;
    w.token(
        ResponseTemplate::new(400).set_body_json(
            json!({"error": "invalid_client", "error_description": "unknown client"}),
        ),
    )
    .await;
    let t = w.transport(w.secret()).await;
    let err = t.send_request(ping()).await.unwrap_err();
    assert!(
        matches!(err, McpClientError::Auth(ref m) if m.contains("invalid_client (unknown client)")),
        "{err}"
    );
}

#[tokio::test]
async fn tokens_are_cached_and_a_second_401_is_returned() {
    let w = World::new().await;
    w.mcp("tok-1", &w.challenge()).await;
    w.prm(w.default_prm()).await;
    w.as_metadata(json!({})).await;
    w.token(Counting(0.into())).await;
    let t = w.transport(w.secret()).await;

    t.send_request(ping()).await.unwrap();
    t.send_request(ping()).await.unwrap();
    assert_eq!(w.token_requests().await.len(), 1, "cached");

    // The server stops accepting tok-1: one refresh (tok-2) and one retry,
    // which the server also rejects; that 401 reaches the caller.
    w.server.reset().await;
    w.mcp("never", &w.challenge()).await;
    w.prm(w.default_prm()).await;
    w.as_metadata(json!({})).await;
    w.token(Counting(1.into())).await;
    let err = t.send_request(ping()).await.unwrap_err();
    assert!(err.to_string().contains("401"), "{err}");
    let tokens: Vec<_> = w
        .server
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .filter(|r| r.url.path() == "/mcp")
        .map(|r| {
            r.headers
                .get("authorization")
                .map(|v| v.to_str().unwrap().to_string())
        })
        .collect();
    assert_eq!(
        tokens,
        [Some("Bearer tok-1".into()), Some("Bearer tok-2".into())]
    );
}

#[tokio::test]
async fn an_expiring_token_is_refreshed_before_sending() {
    let w = World::new().await;
    w.mcp("tok-1", &w.challenge()).await;
    w.prm(w.default_prm()).await;
    w.as_metadata(json!({})).await;
    // tok-1 lives 1 s (refresh window 0.5 s); tok-2 lasts.
    struct Expiring(std::sync::atomic::AtomicUsize);
    impl Respond for Expiring {
        fn respond(&self, _: &Request) -> ResponseTemplate {
            match self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst) {
                0 => token_ok("tok-1", Some(1)),
                _ => token_ok("tok-2", Some(3600)),
            }
        }
    }
    w.token(Expiring(0.into())).await;
    let t = w.transport(w.secret()).await;
    t.send_request(ping()).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(600)).await;
    // Inside tok-1's refresh window the request goes out with tok-2. The server
    // rejects it, which is fine: the header sequence is the assertion.
    let _ = t.send_request(ping()).await;
    assert_eq!(
        mcp_auth_headers(&w).await[..3],
        [
            None,
            Some("Bearer tok-1".into()),
            Some("Bearer tok-2".into())
        ]
    );
}

async fn mcp_auth_headers(w: &World) -> Vec<Option<String>> {
    w.server
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .filter(|r| r.url.path() == "/mcp")
        .map(|r| {
            r.headers
                .get("authorization")
                .map(|v| v.to_str().unwrap().to_string())
        })
        .collect()
}

#[tokio::test]
async fn concurrent_401s_on_a_stale_token_refresh_once() {
    let w = World::new().await;
    w.mcp("tok-1", &w.challenge()).await;
    w.prm(w.default_prm()).await;
    w.as_metadata(json!({})).await;
    w.token(Counting(0.into())).await;
    let t = Arc::new(w.transport(w.secret()).await);
    t.send_request(ping()).await.unwrap();

    // tok-1 is revoked and tok-2 accepted. Eight requests carry tok-1, all get
    // 401, and must share one refresh.
    w.server.reset().await;
    w.mcp("tok-2", &w.challenge()).await;
    w.prm(w.default_prm()).await;
    w.as_metadata(json!({})).await;
    w.token(Counting(1.into())).await;
    let calls: Vec<_> = (0..8)
        .map(|_| {
            let t = t.clone();
            tokio::spawn(async move { t.send_request(ping()).await })
        })
        .collect();
    for c in calls {
        c.await.unwrap().unwrap();
    }
    assert_eq!(w.token_requests().await.len(), 1);
}

#[tokio::test]
async fn an_overflowing_expires_in_is_treated_as_unknown() {
    let w = World::new().await;
    w.mcp("tok-1", &w.challenge()).await;
    w.prm(w.default_prm()).await;
    w.as_metadata(json!({})).await;
    w.token(token_ok("tok-1", Some(u64::MAX))).await;
    let t = w.transport(w.secret()).await;
    t.send_request(ping()).await.unwrap();
    t.send_request(ping()).await.unwrap();
    assert_eq!(w.token_requests().await.len(), 1);
}

#[tokio::test]
async fn root_prm_and_path_issuer_discovery_order() {
    // No resource_metadata and no path-form PRM: the root form is used. The
    // issuer has a path, and only its third metadata location exists.
    let w = World::new().await;
    let issuer = format!("{}/tenant", w.uri());
    w.mcp("tok-1", "Bearer").await;
    Mock::given(method("GET"))
        .and(path("/.well-known/oauth-protected-resource"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "resource": w.endpoint(), "authorization_servers": [issuer],
        })))
        .mount(&w.server)
        .await;
    Mock::given(method("GET"))
        .and(path("/tenant/.well-known/openid-configuration"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "issuer": issuer, "token_endpoint": format!("{}/token", w.uri()),
        })))
        .mount(&w.server)
        .await;
    w.token(token_ok("tok-1", None)).await;
    let t = w
        .transport(ClientCredentials::client_secret(&issuer, CLIENT_ID, SECRET))
        .await;
    t.send_request(ping()).await.unwrap();
    let gets: Vec<String> = w
        .server
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .filter(|r| r.method.as_str() == "GET")
        .map(|r| r.url.path().to_string())
        .collect();
    assert_eq!(
        gets,
        [
            "/.well-known/oauth-protected-resource/mcp",
            "/.well-known/oauth-protected-resource",
            "/.well-known/oauth-authorization-server/tenant",
            "/.well-known/openid-configuration/tenant",
            "/tenant/.well-known/openid-configuration",
        ]
    );
}

#[tokio::test]
async fn mcp_default_headers_never_reach_the_authorization_server() {
    let w = World::new().await;
    w.mcp("tok-1", &w.challenge()).await;
    w.prm(w.default_prm()).await;
    w.as_metadata(json!({})).await;
    w.token(token_ok("tok-1", None)).await;
    let config = turul_mcp_client::config::ConnectionConfig {
        headers: Some(
            [
                ("X-Leak".to_string(), "rs-only".to_string()),
                ("Authorization".to_string(), "Bearer rs-static".to_string()),
            ]
            .into(),
        ),
        ..Default::default()
    };
    let t = HttpTransport::with_config(&w.endpoint(), &config)
        .unwrap()
        .with_oauth_client_credentials(w.secret())
        .unwrap();
    t.connect().await.unwrap();
    t.send_request(ping()).await.unwrap();
    for r in w
        .server
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .filter(|r| r.url.path() != "/mcp")
    {
        assert!(r.headers.get("x-leak").is_none(), "{}", r.url);
        let auth = r
            .headers
            .get("authorization")
            .map(|v| v.to_str().unwrap().to_string());
        assert!(
            auth.as_deref().is_none_or(|a| a.starts_with("Basic ")),
            "{}: {auth:?}",
            r.url
        );
    }
}

#[tokio::test]
async fn a_token_endpoint_redirect_is_not_followed() {
    let w = World::new().await;
    w.mcp("tok-1", &w.challenge()).await;
    w.prm(w.default_prm()).await;
    w.as_metadata(json!({})).await;
    w.token(ResponseTemplate::new(307).insert_header("Location", format!("{}/elsewhere", w.uri())))
        .await;
    Mock::given(method("POST"))
        .and(path("/elsewhere"))
        .respond_with(token_ok("tok-1", None))
        .expect(0)
        .mount(&w.server)
        .await;
    let t = w.transport(w.secret()).await;
    assert!(t.send_request(ping()).await.is_err());
}

#[tokio::test]
async fn an_hmac_key_is_not_private_key_jwt() {
    let w = World::new().await;
    w.mcp("tok-1", &w.challenge()).await;
    w.prm(w.default_prm()).await;
    w.as_metadata(json!({"token_endpoint_auth_methods_supported": ["private_key_jwt"]}))
        .await;
    let creds = ClientCredentials::private_key_jwt(
        w.uri(),
        CLIENT_ID,
        EncodingKey::from_secret(b"shared"),
        Algorithm::HS256,
    );
    let t = w.transport(creds).await;
    let err = t.send_request(ping()).await.unwrap_err();
    assert!(
        matches!(err, McpClientError::Auth(ref m) if m.contains("asymmetric")),
        "{err}"
    );
    assert!(w.token_requests().await.is_empty());
}

#[tokio::test]
async fn the_sse_get_refreshes_on_401_and_keeps_the_session() {
    // 2025-11-25 lane, where the GET listener runs. Unauthenticated → 401 with
    // the challenge; with tok-1 → 405 (no stream), which keeps the session.
    let w = World::new().await;
    Mock::given(method("GET"))
        .and(path("/mcp"))
        .and(header("Authorization", "Bearer tok-1"))
        .respond_with(ResponseTemplate::new(405))
        .with_priority(1)
        .expect(1)
        .mount(&w.server)
        .await;
    Mock::given(method("GET"))
        .and(path("/mcp"))
        .respond_with(ResponseTemplate::new(401).insert_header("WWW-Authenticate", w.challenge()))
        .with_priority(10)
        .expect(1)
        .mount(&w.server)
        .await;
    Mock::given(method("POST"))
        .and(path("/mcp"))
        .and(header("Mcp-Session-Id", "sess-1"))
        .and(header("Authorization", "Bearer tok-1"))
        .respond_with(ok())
        .expect(1)
        .mount(&w.server)
        .await;
    w.prm(w.default_prm()).await;
    w.as_metadata(json!({})).await;
    w.token(token_ok("tok-1", None)).await;
    let t = w.transport(w.secret()).await;
    t.set_session_id("sess-1".into());
    let _rx = t.start_event_listener().await.unwrap();
    for _ in 0..100 {
        let gets = w
            .server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .filter(|r| r.method.as_str() == "GET" && r.url.path() == "/mcp")
            .count();
        if gets >= 2 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    // The session survived the 401, and the POST carries the token.
    t.send_request(ping()).await.unwrap();
}

#[tokio::test]
async fn concurrent_first_requests_fetch_one_token() {
    let w = World::new().await;
    w.mcp("tok-1", &w.challenge()).await;
    w.prm(w.default_prm()).await;
    w.as_metadata(json!({})).await;
    w.token(Counting(0.into())).await;
    let t = Arc::new(w.transport(w.secret()).await);
    let calls = (0..8).map(|_| {
        let t = t.clone();
        tokio::spawn(async move { t.send_request(ping()).await })
    });
    for c in calls {
        c.await.unwrap().unwrap();
    }
    assert_eq!(w.token_requests().await.len(), 1);
}

#[tokio::test]
async fn set_bearer_does_not_override_the_token_source() {
    let w = World::new().await;
    w.mcp("tok-1", &w.challenge()).await;
    w.prm(w.default_prm()).await;
    w.as_metadata(json!({})).await;
    w.token(token_ok("tok-1", None)).await;
    let t = w.transport(w.secret()).await;
    t.update_auth_header(Some("Bearer manual".into())).await;
    t.send_request(ping()).await.unwrap();
    let mcp: Vec<_> = w
        .server
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .filter(|r| r.url.path() == "/mcp")
        .collect();
    for r in &mcp {
        let auths: Vec<_> = r.headers.get_all("authorization").iter().collect();
        assert!(auths.len() <= 1, "one Authorization header");
        assert!(
            auths.iter().all(|v| v.to_str().unwrap() != "Bearer manual"),
            "{auths:?}"
        );
    }
}

#[test]
#[should_panic(expected = "with_oauth_client_credentials needs with_url()")]
fn builder_refuses_credentials_with_a_caller_built_transport() {
    let transport = Box::new(HttpTransport::new("http://127.0.0.1:1/mcp").unwrap());
    let _ = turul_mcp_client::McpClientBuilder::new()
        .with_transport(transport)
        .with_oauth_client_credentials(ClientCredentials::client_secret("https://as", "id", "s"))
        .build();
}

// ---- end to end: a real resource server ----

mod e2e {
    use super::*;
    use turul_mcp_oauth::{
        OAuthResourceMiddleware, ProtectedResourceMetadata, WellKnownOAuthHandler,
        hardened_validator,
    };
    use turul_mcp_server::prelude::*;

    #[derive(turul_mcp_derive::McpTool, Clone, Default)]
    #[tool(name = "echo", description = "Echo")]
    struct Echo {
        message: String,
    }
    impl Echo {
        async fn execute(&self, _s: Option<SessionContext>) -> McpResult<String> {
            Ok(self.message.clone())
        }
    }

    /// An authorization server that checks the client secret and issues RS256
    /// tokens for `resource`, served from `as_server`.
    struct Issuer {
        issuer: String,
        resource: String,
    }
    impl Respond for Issuer {
        fn respond(&self, req: &Request) -> ResponseTemplate {
            let expected = format!(
                "Basic {}",
                STANDARD.encode(format!(
                    "{}:{}",
                    url::form_urlencoded::byte_serialize(CLIENT_ID.as_bytes()).collect::<String>(),
                    url::form_urlencoded::byte_serialize(SECRET.as_bytes()).collect::<String>()
                ))
            );
            let auth = req
                .headers
                .get("authorization")
                .map(|v| v.to_str().unwrap());
            let f = form(req);
            if auth != Some(expected.as_str())
                || f.get("grant_type").map(String::as_str) != Some("client_credentials")
                || f.get("resource") != Some(&self.resource)
            {
                return ResponseTemplate::new(401)
                    .set_body_json(json!({"error": "invalid_client"}));
            }
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let claims = json!({
                "iss": self.issuer, "sub": CLIENT_ID, "aud": self.resource,
                "iat": now, "exp": now + 600, "scope": f.get("scope").cloned().unwrap_or_default(),
            });
            let mut h = jsonwebtoken::Header::new(Algorithm::RS256);
            h.kid = Some(KID.into());
            let token = jsonwebtoken::encode(&h, &claims, &signing_key()).unwrap();
            token_ok(&token, Some(600))
        }
    }

    #[tokio::test]
    async fn list_tools_through_the_full_flow_on_a_real_resource_server() {
        let as_server = MockServer::start().await;
        let issuer = as_server.uri();
        Mock::given(method("GET"))
            .and(path("/.well-known/oauth-authorization-server"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "issuer": issuer,
                "token_endpoint": format!("{issuer}/token"),
                "jwks_uri": format!("{issuer}/jwks.json"),
                "token_endpoint_auth_methods_supported": ["client_secret_basic"],
            })))
            .mount(&as_server)
            .await;
        Mock::given(method("GET"))
            .and(path("/jwks.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"keys": [{
                "kty": "RSA", "use": "sig", "kid": KID, "alg": "RS256",
                "n": RSA_KEY.1, "e": RSA_KEY.2,
            }]})))
            .mount(&as_server)
            .await;

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let resource = format!(
            "http://127.0.0.1:{}/mcp",
            listener.local_addr().unwrap().port()
        );
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(Issuer {
                issuer: issuer.clone(),
                resource: resource.clone(),
            })
            .mount(&as_server)
            .await;

        let metadata = ProtectedResourceMetadata::new(&resource, vec![issuer.clone()])
            .unwrap()
            .with_scopes(vec!["mcp:read".into()]);
        let validator = Arc::new(
            hardened_validator(&format!("{issuer}/jwks.json"), &resource)
                .unwrap()
                .with_issuer(&issuer),
        );
        let middleware = Arc::new(
            OAuthResourceMiddleware::new(validator, metadata.clone())
                .with_required_scopes(vec!["mcp:read".into()]),
        );
        let well_known = Arc::new(WellKnownOAuthHandler::new(&metadata));
        let mut builder = McpServer::builder()
            .name("oauth-cc-e2e")
            .version("1.0.0")
            .tool(Echo::default())
            .middleware(middleware)
            .listener(listener);
        for p in metadata.well_known_paths() {
            builder = builder.route(&p, well_known.clone());
        }
        let server = builder.build().unwrap();
        tokio::spawn(async move {
            server.run().await.ok();
        });
        let probe = reqwest::Client::new();
        for _ in 0..50 {
            if probe.get(&resource).send().await.is_ok() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }

        let client = turul_mcp_client::McpClientBuilder::new()
            .with_url(&resource)
            .unwrap()
            .with_oauth_client_credentials(ClientCredentials::client_secret(
                &issuer, CLIENT_ID, SECRET,
            ))
            .build();
        client
            .connect()
            .await
            .expect("connect through the OAuth flow");
        let tools = client.list_tools().await.expect("list_tools");
        assert!(tools.iter().any(|t| t.name == "echo"), "{tools:?}");
        let token_calls = as_server
            .received_requests()
            .await
            .unwrap()
            .into_iter()
            .filter(|r| r.url.path() == "/token")
            .count();
        assert_eq!(token_calls, 1);
    }
}
