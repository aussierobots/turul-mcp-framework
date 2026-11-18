//! OAuth Client Credentials extension (`io.modelcontextprotocol/oauth-client-credentials`):
//! machine-to-machine access tokens for the HTTP transport.
//!
//! Attach [`ClientCredentials`] with
//! [`McpClientBuilder::with_oauth_client_credentials`](crate::McpClientBuilder::with_oauth_client_credentials)
//! or [`HttpTransport::with_oauth_client_credentials`](crate::transport::http::HttpTransport::with_oauth_client_credentials).
//! On the first `401` the transport reads the server's Protected Resource
//! Metadata, fetches the authorization server's metadata, requests a token with
//! `grant_type=client_credentials`, and retries the request with it.
//!
//! The credentials are bound to one authorization server: they are sent only to
//! the issuer they were created with, and only when the resource server lists
//! that issuer.
//!
//! See <https://github.com/modelcontextprotocol/ext-auth/blob/main/specification/stable/oauth-client-credentials/2026-07-28/oauth-client-credentials.mdx>.
//!
//! # Examples
//!
//! ```no_run
//! use turul_mcp_client::McpClientBuilder;
//! use turul_mcp_client::oauth_client_credentials::ClientCredentials;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let secret = std::env::var("MCP_CLIENT_SECRET")?;
//! let client = McpClientBuilder::new()
//!     .with_url("https://mcp.example.com/mcp")?
//!     .with_oauth_client_credentials(
//!         ClientCredentials::client_secret("https://auth.example.com", "my-client", secret)
//!             .with_scope("mcp:read"),
//!     )
//!     .build();
//! # let _ = client;
//! # Ok(())
//! # }
//! ```

use std::fmt;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use url::Url;

use crate::error::{McpClientError, McpClientResult};

pub use jsonwebtoken::{Algorithm, EncodingKey};

/// The extension identifier.
pub const EXTENSION_ID: &str = "io.modelcontextprotocol/oauth-client-credentials";

/// Lifetime of a client assertion JWT.
const ASSERTION_LIFETIME: Duration = Duration::from_secs(60);
/// A token this close to expiry is refreshed before it is sent.
const REFRESH_SKEW: Duration = Duration::from_secs(30);
/// Timeout for each request to the resource or authorization server metadata
/// and token endpoints.
const AS_TIMEOUT: Duration = Duration::from_secs(30);

/// Largest metadata or token response read.
const MAX_BODY: usize = 64 * 1024;

const CLIENT_ASSERTION_TYPE: &str = "urn:ietf:params:oauth:client-assertion-type:jwt-bearer";

/// Pre-registered client credentials, bound to one authorization server.
#[derive(Clone)]
pub struct ClientCredentials {
    issuer: String,
    client_id: String,
    auth: ClientAuth,
    scope: Option<String>,
}

/// How the client authenticates to the token endpoint.
#[derive(Clone)]
pub enum ClientAuth {
    /// A client secret, sent as `client_secret_basic` or `client_secret_post`.
    Secret(String),
    /// A signed JWT assertion (`private_key_jwt`, RFC 7523 §2.2).
    PrivateKeyJwt {
        /// The private signing key.
        key: EncodingKey,
        /// The signing algorithm; must match the key.
        algorithm: Algorithm,
        /// The `kid` header, when the authorization server needs it to pick the key.
        kid: Option<String>,
        /// The assertion's `aud` claim.
        audience: AssertionAudience,
    },
}

/// The `aud` claim of a client assertion.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AssertionAudience {
    /// The authorization server's issuer identifier.
    #[default]
    Issuer,
    /// The token endpoint URL, for authorization servers that require it.
    TokenEndpoint,
}

impl ClientCredentials {
    /// Creates credentials that authenticate with a client secret.
    ///
    /// `issuer` is the authorization server's issuer identifier, exactly as its
    /// metadata states it.
    pub fn client_secret(
        issuer: impl Into<String>,
        client_id: impl Into<String>,
        secret: impl Into<String>,
    ) -> Self {
        Self {
            issuer: issuer.into(),
            client_id: client_id.into(),
            auth: ClientAuth::Secret(secret.into()),
            scope: None,
        }
    }

    /// Creates credentials that authenticate with a JWT signed by `key`.
    ///
    /// The assertion's `aud` is the issuer; change it with
    /// [`with_assertion_audience`](Self::with_assertion_audience).
    pub fn private_key_jwt(
        issuer: impl Into<String>,
        client_id: impl Into<String>,
        key: EncodingKey,
        algorithm: Algorithm,
    ) -> Self {
        Self {
            issuer: issuer.into(),
            client_id: client_id.into(),
            auth: ClientAuth::PrivateKeyJwt {
                key,
                algorithm,
                kid: None,
                audience: AssertionAudience::Issuer,
            },
            scope: None,
        }
    }

    /// Sets the `scope` to request, overriding the server's challenge and metadata.
    pub fn with_scope(mut self, scope: impl Into<String>) -> Self {
        self.scope = Some(scope.into());
        self
    }

    /// Sets the assertion's `kid` header. Has no effect on secret credentials.
    pub fn with_kid(mut self, kid: impl Into<String>) -> Self {
        if let ClientAuth::PrivateKeyJwt { kid: k, .. } = &mut self.auth {
            *k = Some(kid.into());
        }
        self
    }

    /// Sets the assertion's `aud` claim. Has no effect on secret credentials.
    pub fn with_assertion_audience(mut self, audience: AssertionAudience) -> Self {
        if let ClientAuth::PrivateKeyJwt { audience: a, .. } = &mut self.auth {
            *a = audience;
        }
        self
    }

    /// Returns the issuer these credentials are bound to.
    pub fn issuer(&self) -> &str {
        &self.issuer
    }

    /// Returns the client identifier.
    pub fn client_id(&self) -> &str {
        &self.client_id
    }
}

impl fmt::Debug for ClientCredentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClientCredentials")
            .field("issuer", &self.issuer)
            .field("client_id", &self.client_id)
            .field("auth", &self.auth)
            .field("scope", &self.scope)
            .finish()
    }
}

impl fmt::Debug for ClientAuth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Secret(_) => f.write_str("Secret(<redacted>)"),
            Self::PrivateKeyJwt {
                algorithm,
                kid,
                audience,
                ..
            } => f
                .debug_struct("PrivateKeyJwt")
                .field("key", &"<redacted>")
                .field("algorithm", algorithm)
                .field("kid", kid)
                .field("audience", audience)
                .finish(),
        }
    }
}

/// The parameters of a `Bearer` `WWW-Authenticate` challenge this flow uses.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Challenge {
    pub(crate) resource_metadata: Option<String>,
    pub(crate) scope: Option<String>,
}

/// Parses the `Bearer` challenge of a `WWW-Authenticate` header (RFC 6750 §3,
/// RFC 9110 §11.2). Returns the default when there is no `Bearer` challenge.
pub(crate) fn parse_challenge(header: &str) -> Challenge {
    let mut out = Challenge::default();
    let mut rest = header.trim_start();
    // Find the Bearer scheme token.
    loop {
        let Some(pos) = find_scheme(rest, "bearer") else {
            return out;
        };
        rest = &rest[pos + "bearer".len()..];
        if rest.is_empty() || rest.starts_with(' ') || rest.starts_with(',') {
            break;
        }
    }
    // Parse auth-params until the next scheme (a token not followed by '=').
    loop {
        rest = rest.trim_start_matches([' ', ',']);
        let Some(eq) = rest.find('=') else { break };
        let name = rest[..eq].trim();
        if name.is_empty() || name.contains(' ') {
            break;
        }
        rest = rest[eq + 1..].trim_start();
        let value;
        if let Some(stripped) = rest.strip_prefix('"') {
            let mut v = String::new();
            let mut chars = stripped.char_indices();
            let mut end = stripped.len();
            while let Some((i, c)) = chars.next() {
                match c {
                    '\\' => {
                        if let Some((_, n)) = chars.next() {
                            v.push(n);
                        }
                    }
                    '"' => {
                        end = i + 1;
                        break;
                    }
                    _ => v.push(c),
                }
            }
            value = v;
            rest = &stripped[end.min(stripped.len())..];
        } else {
            let end = rest.find([',', ' ']).unwrap_or(rest.len());
            value = rest[..end].to_string();
            rest = &rest[end..];
        }
        match name.to_ascii_lowercase().as_str() {
            "resource_metadata" => out.resource_metadata = Some(value),
            "scope" => out.scope = Some(value),
            _ => {}
        }
    }
    out
}

fn find_scheme(s: &str, scheme: &str) -> Option<usize> {
    let lower = s.to_ascii_lowercase();
    let mut from = 0;
    while let Some(i) = lower[from..].find(scheme) {
        let at = from + i;
        let boundary = at == 0 || matches!(lower.as_bytes()[at - 1], b' ' | b',');
        if boundary {
            return Some(at);
        }
        from = at + scheme.len();
    }
    None
}

/// Requires `https`, or `http` on a loopback host (OAuth 2.1 §1.5).
pub(crate) fn require_secure(url: &Url) -> McpClientResult<()> {
    let loopback = match url.host() {
        Some(url::Host::Domain(d)) => d.eq_ignore_ascii_case("localhost"),
        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        None => false,
    };
    if url.scheme() == "https" || (url.scheme() == "http" && loopback) {
        Ok(())
    } else {
        Err(auth_error(format!(
            "{url} is not https; authorization server URLs must be https except on a loopback host"
        )))
    }
}

fn auth_error(msg: impl Into<String>) -> McpClientError {
    McpClientError::Auth(msg.into())
}

/// The form-urlencoding RFC 6749 §2.3.1 applies to the id and secret before
/// HTTP Basic encoding.
pub(crate) fn form_encode(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

#[derive(Deserialize)]
struct ProtectedResourceMetadata {
    resource: String,
    #[serde(default)]
    authorization_servers: Vec<String>,
    #[serde(default)]
    scopes_supported: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct AuthorizationServerMetadata {
    issuer: String,
    token_endpoint: Option<String>,
    #[serde(default)]
    token_endpoint_auth_methods_supported: Option<Vec<String>>,
    #[serde(default)]
    token_endpoint_auth_signing_alg_values_supported: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    token_type: String,
    #[serde(default, deserialize_with = "lenient_seconds")]
    expires_in: Option<u64>,
}

/// `expires_in` as a JSON number or, as some servers send it, a numeric string.
fn lenient_seconds<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<u64>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Seconds {
        Number(u64),
        Text(String),
    }
    Ok(match Option::<Seconds>::deserialize(d)? {
        None => None,
        Some(Seconds::Number(n)) => Some(n),
        Some(Seconds::Text(t)) => t.trim().parse().ok(),
    })
}

#[derive(Deserialize)]
struct TokenErrorResponse {
    error: String,
    #[serde(default)]
    error_description: Option<String>,
}

#[derive(Serialize)]
struct AssertionClaims<'a> {
    iss: &'a str,
    sub: &'a str,
    aud: &'a str,
    iat: u64,
    exp: u64,
    jti: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Method {
    Basic,
    Post,
    PrivateKeyJwt,
}

struct Discovered {
    /// Where the PRM was read; a challenge naming another location re-discovers.
    prm_url: Url,
    resource: String,
    token_endpoint: Url,
    method: Method,
    scope: Option<String>,
}

struct Token {
    value: String,
    expires_at: Option<Instant>,
    lifetime: Option<Duration>,
}

#[derive(Default)]
struct State {
    discovered: Option<Discovered>,
    token: Option<Token>,
}

/// Obtains and caches access tokens for one MCP endpoint.
pub(crate) struct TokenSource {
    credentials: ClientCredentials,
    endpoint: Url,
    /// A client of its own: the MCP client's default headers may carry a
    /// bearer for the resource server, which must not reach the authorization
    /// server; and redirects are not followed with credentials attached.
    http: Client,
    state: Mutex<State>,
}

impl fmt::Debug for TokenSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TokenSource")
            .field("credentials", &self.credentials)
            .field("endpoint", &self.endpoint.as_str())
            .finish_non_exhaustive()
    }
}

impl TokenSource {
    pub(crate) fn new(credentials: ClientCredentials, endpoint: Url) -> McpClientResult<Self> {
        let http = Client::builder()
            .timeout(AS_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| McpClientError::Config(format!("OAuth HTTP client: {e}")))?;
        Ok(Self {
            credentials,
            endpoint,
            http,
            state: Mutex::new(State::default()),
        })
    }

    /// Returns the token to send, refreshing one near expiry. `None` before the
    /// first `401` has told the client where to get one.
    pub(crate) async fn current(&self) -> McpClientResult<Option<String>> {
        let mut state = self.state.lock().await;
        if let Some(t) = &state.token
            && !near_expiry(t)
        {
            return Ok(Some(t.value.clone()));
        }
        let Some(discovered) = &state.discovered else {
            return Ok(None);
        };
        match self.fetch_token(discovered).await {
            Ok(token) => {
                let value = token.value.clone();
                state.token = Some(token);
                Ok(Some(value))
            }
            // An early refresh failed but the token has not expired: send it.
            Err(e) => match &state.token {
                Some(t) if !expired(t) => {
                    tracing::warn!("OAuth token refresh failed; using the unexpired token: {e}");
                    Ok(Some(t.value.clone()))
                }
                _ => Err(e),
            },
        }
    }

    /// Handles a `401`: returns a token to retry with. `sent` is the token the
    /// rejected request carried; a different cached token means another request
    /// already refreshed, and that one is returned.
    pub(crate) async fn after_unauthorized(
        &self,
        www_authenticate: Option<&str>,
        sent: Option<&str>,
    ) -> McpClientResult<String> {
        let mut state = self.state.lock().await;
        if let Some(t) = &state.token
            && Some(t.value.as_str()) != sent
            && !near_expiry(t)
        {
            return Ok(t.value.clone());
        }
        state.token = None;
        let challenge = www_authenticate.map(parse_challenge).unwrap_or_default();
        let moved = matches!(
            (&state.discovered, &challenge.resource_metadata),
            (Some(d), Some(url)) if Url::parse(url).map_or(true, |u| u != d.prm_url)
        );
        if state.discovered.is_none() || moved {
            state.discovered = Some(self.discover(&challenge).await?);
        } else if self.credentials.scope.is_none()
            && let Some(scope) = challenge.scope
        {
            state.discovered.as_mut().expect("checked").scope = Some(scope);
        }
        let token = self
            .fetch_token(state.discovered.as_ref().expect("set"))
            .await?;
        let value = token.value.clone();
        state.token = Some(token);
        Ok(value)
    }

    async fn discover(&self, challenge: &Challenge) -> McpClientResult<Discovered> {
        let (prm_url, prm) = self.protected_resource_metadata(challenge).await?;
        if !same_url(&prm.resource, &self.endpoint) {
            return Err(auth_error(format!(
                "protected resource metadata names resource {:?}, not this endpoint {}",
                prm.resource, self.endpoint
            )));
        }
        let issuer = &self.credentials.issuer;
        if !prm.authorization_servers.iter().any(|a| a == issuer) {
            return Err(auth_error(format!(
                "the resource server does not list authorization server {issuer:?} \
                 (it lists {:?}); credentials are sent only to their own issuer",
                prm.authorization_servers
            )));
        }
        let asm = self.authorization_server_metadata(issuer).await?;
        let token_endpoint = asm
            .token_endpoint
            .as_deref()
            .ok_or_else(|| auth_error(format!("{issuer} metadata has no token_endpoint")))?;
        let token_endpoint = Url::parse(token_endpoint)
            .map_err(|e| auth_error(format!("token_endpoint {token_endpoint:?}: {e}")))?;
        require_secure(&token_endpoint)?;
        let method = self.choose_method(&asm)?;
        let scope = self
            .credentials
            .scope
            .clone()
            .or_else(|| challenge.scope.clone())
            .or_else(|| {
                prm.scopes_supported
                    .filter(|s| !s.is_empty())
                    .map(|s| s.join(" "))
            });
        Ok(Discovered {
            prm_url,
            resource: prm.resource,
            token_endpoint,
            method,
            scope,
        })
    }

    fn choose_method(&self, asm: &AuthorizationServerMetadata) -> McpClientResult<Method> {
        let listed = |m: &str| {
            asm.token_endpoint_auth_methods_supported
                .as_ref()
                .is_some_and(|l| l.iter().any(|x| x == m))
        };
        match &self.credentials.auth {
            ClientAuth::Secret(_) => {
                if asm.token_endpoint_auth_methods_supported.is_none()
                    || listed("client_secret_basic")
                {
                    Ok(Method::Basic)
                } else if listed("client_secret_post") {
                    Ok(Method::Post)
                } else {
                    Err(auth_error(format!(
                        "{} supports neither client_secret_basic nor client_secret_post",
                        asm.issuer
                    )))
                }
            }
            ClientAuth::PrivateKeyJwt { algorithm, .. } => {
                // An HMAC "key" is a shared secret: that is client_secret_jwt,
                // which this extension does not name.
                if matches!(
                    algorithm,
                    Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512
                ) {
                    return Err(auth_error(format!(
                        "private_key_jwt needs an asymmetric algorithm, not {algorithm:?}"
                    )));
                }
                if !listed("private_key_jwt") {
                    return Err(auth_error(format!(
                        "{} does not support private_key_jwt",
                        asm.issuer
                    )));
                }
                let alg = format!("{algorithm:?}");
                if let Some(algs) = &asm.token_endpoint_auth_signing_alg_values_supported
                    && !algs.contains(&alg)
                {
                    return Err(auth_error(format!(
                        "{} does not accept {alg} client assertions (it accepts {algs:?})",
                        asm.issuer
                    )));
                }
                Ok(Method::PrivateKeyJwt)
            }
        }
    }

    async fn protected_resource_metadata(
        &self,
        challenge: &Challenge,
    ) -> McpClientResult<(Url, ProtectedResourceMetadata)> {
        let candidates: Vec<Url> = match &challenge.resource_metadata {
            Some(u) => {
                let url = Url::parse(u)
                    .map_err(|e| auth_error(format!("resource_metadata {u:?}: {e}")))?;
                // The challenge is the resource server's word: fetch only from
                // its own origin, so it cannot point the client at arbitrary hosts.
                if url.origin() != self.endpoint.origin() {
                    return Err(auth_error(format!(
                        "resource_metadata {url} is not on the endpoint's origin {}",
                        self.endpoint.origin().ascii_serialization()
                    )));
                }
                vec![url]
            }
            None => well_known_prm(&self.endpoint),
        };
        let mut last = String::new();
        for url in candidates {
            match self.get_json::<ProtectedResourceMetadata>(&url).await {
                Ok(Some(m)) => return Ok((url, m)),
                Ok(None) => last = format!("{url}: not found"),
                Err(e) => return Err(e),
            }
        }
        Err(auth_error(format!(
            "no protected resource metadata ({last})"
        )))
    }

    async fn authorization_server_metadata(
        &self,
        issuer: &str,
    ) -> McpClientResult<AuthorizationServerMetadata> {
        let issuer_url =
            Url::parse(issuer).map_err(|e| auth_error(format!("issuer {issuer:?}: {e}")))?;
        let mut last = String::new();
        for url in authorization_server_metadata_urls(&issuer_url) {
            require_secure(&url)?;
            match self.get_json::<AuthorizationServerMetadata>(&url).await {
                Ok(Some(m)) => {
                    if m.issuer != issuer {
                        return Err(auth_error(format!(
                            "{url} names issuer {:?}, not {issuer:?}",
                            m.issuer
                        )));
                    }
                    return Ok(m);
                }
                Ok(None) => last = format!("{url}: not found"),
                Err(e) => return Err(e),
            }
        }
        Err(auth_error(format!(
            "no authorization server metadata for {issuer} ({last})"
        )))
    }

    /// GETs a JSON document; `Ok(None)` on any 4xx, so the caller tries the
    /// next location.
    async fn get_json<T: for<'de> Deserialize<'de>>(
        &self,
        url: &Url,
    ) -> McpClientResult<Option<T>> {
        let resp = self
            .http
            .get(url.clone())
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| auth_error(format!("GET {url}: {e}")))?;
        let status = resp.status();
        if status.is_client_error() {
            return Ok(None);
        }
        if !status.is_success() {
            return Err(auth_error(format!("GET {url}: HTTP {status}")));
        }
        let body = read_capped(resp)
            .await
            .map_err(|e| auth_error(format!("GET {url}: {e}")))?;
        serde_json::from_slice::<T>(&body)
            .map(Some)
            .map_err(|e| auth_error(format!("GET {url}: {e}")))
    }

    async fn fetch_token(&self, d: &Discovered) -> McpClientResult<Token> {
        let creds = &self.credentials;
        // The serializer is not Send, so the body is finished before any await.
        let (req, body) = {
            let mut form = url::form_urlencoded::Serializer::new(String::new());
            form.append_pair("grant_type", "client_credentials");
            let mut req = self.http.post(d.token_endpoint.clone());
            match (&creds.auth, d.method) {
                (ClientAuth::Secret(secret), Method::Basic) => {
                    req = req.basic_auth(form_encode(&creds.client_id), Some(form_encode(secret)));
                }
                (ClientAuth::Secret(secret), Method::Post) => {
                    form.append_pair("client_id", &creds.client_id);
                    form.append_pair("client_secret", secret);
                }
                (
                    ClientAuth::PrivateKeyJwt {
                        key,
                        algorithm,
                        kid,
                        audience,
                    },
                    Method::PrivateKeyJwt,
                ) => {
                    let aud = match audience {
                        AssertionAudience::Issuer => creds.issuer.as_str(),
                        AssertionAudience::TokenEndpoint => d.token_endpoint.as_str(),
                    };
                    let assertion = sign_assertion(&creds.client_id, aud, key, *algorithm, kid)?;
                    form.append_pair("client_assertion_type", CLIENT_ASSERTION_TYPE);
                    form.append_pair("client_assertion", &assertion);
                }
                _ => unreachable!("choose_method pairs the method with the credential kind"),
            }
            form.append_pair("resource", &d.resource);
            if let Some(scope) = &d.scope {
                form.append_pair("scope", scope);
            }
            (req, form.finish())
        };
        let sent_at = Instant::now();
        let resp = req
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Accept", "application/json")
            .body(body)
            .send()
            .await
            .map_err(|e| auth_error(format!("token request to {}: {e}", d.token_endpoint)))?;
        let status = resp.status();
        let body = read_capped(resp)
            .await
            .map_err(|e| auth_error(format!("token response: {e}")))?;
        if status != StatusCode::OK {
            return Err(match serde_json::from_slice::<TokenErrorResponse>(&body) {
                Ok(e) => auth_error(format!(
                    "token request rejected: {}{}",
                    e.error,
                    e.error_description
                        .map(|d| format!(" ({d})"))
                        .unwrap_or_default()
                )),
                Err(_) => auth_error(format!("token request failed: HTTP {status}")),
            });
        }
        let tr: TokenResponse = serde_json::from_slice(&body)
            .map_err(|e| auth_error(format!("token response: {e}")))?;
        if !tr.token_type.eq_ignore_ascii_case("bearer") {
            return Err(auth_error(format!(
                "token_type {:?} is not Bearer",
                tr.token_type
            )));
        }
        Ok(Token {
            value: tr.access_token,
            // An absurd lifetime is treated as unknown rather than overflowing.
            expires_at: tr
                .expires_in
                .and_then(|secs| sent_at.checked_add(Duration::from_secs(secs))),
            lifetime: tr.expires_in.map(Duration::from_secs),
        })
    }
}

/// Within the refresh window: [`REFRESH_SKEW`], or half the lifetime for a
/// token issued for less than twice that, so a short-lived token is not
/// re-fetched on every request.
/// Reads a response body of at most [`MAX_BODY`] bytes.
async fn read_capped(mut resp: reqwest::Response) -> Result<Vec<u8>, String> {
    let mut body = Vec::new();
    while let Some(chunk) = resp.chunk().await.map_err(|e| e.to_string())? {
        if body.len() + chunk.len() > MAX_BODY {
            return Err(format!("response larger than {MAX_BODY} bytes"));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

fn near_expiry(t: &Token) -> bool {
    let skew = t.lifetime.map_or(REFRESH_SKEW, |l| REFRESH_SKEW.min(l / 2));
    t.expires_at
        .is_some_and(|at| at.saturating_duration_since(Instant::now()) <= skew)
}

fn expired(t: &Token) -> bool {
    t.expires_at.is_some_and(|at| at <= Instant::now())
}

fn sign_assertion(
    client_id: &str,
    aud: &str,
    key: &EncodingKey,
    algorithm: Algorithm,
    kid: &Option<String>,
) -> McpClientResult<String> {
    let iat = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| auth_error(format!("system clock: {e}")))?
        .as_secs();
    let mut header = jsonwebtoken::Header::new(algorithm);
    header.kid = kid.clone();
    let jti: String = (0..16)
        .map(|_| format!("{:02x}", rand::random::<u8>()))
        .collect();
    let claims = AssertionClaims {
        iss: client_id,
        sub: client_id,
        aud,
        iat,
        exp: iat + ASSERTION_LIFETIME.as_secs(),
        jti,
    };
    jsonwebtoken::encode(&header, &claims, key)
        .map_err(|e| auth_error(format!("signing the client assertion: {e}")))
}

fn same_url(a: &str, b: &Url) -> bool {
    Url::parse(a).is_ok_and(|a| a == *b)
}

/// The RFC 9728 §3.1 locations: the well-known suffix inserted before the
/// endpoint's path and query, then the root form.
fn well_known_prm(endpoint: &Url) -> Vec<Url> {
    let path = endpoint.path().trim_end_matches('/');
    let mut out = Vec::new();
    let mut at = |p: &str, keep_query: bool| {
        let mut u = endpoint.clone();
        u.set_path(p);
        if !keep_query {
            u.set_query(None);
        }
        u.set_fragment(None);
        out.push(u);
    };
    if !path.is_empty() || endpoint.query().is_some() {
        at(
            &format!("/.well-known/oauth-protected-resource{path}"),
            true,
        );
    }
    at("/.well-known/oauth-protected-resource", false);
    out
}

/// The authorization server metadata locations, in the core spec's order.
fn authorization_server_metadata_urls(issuer: &Url) -> Vec<Url> {
    let path = issuer.path().trim_end_matches('/');
    let paths: Vec<String> = if path.is_empty() {
        vec![
            "/.well-known/oauth-authorization-server".into(),
            "/.well-known/openid-configuration".into(),
        ]
    } else {
        vec![
            format!("/.well-known/oauth-authorization-server{path}"),
            format!("/.well-known/openid-configuration{path}"),
            format!("{path}/.well-known/openid-configuration"),
        ]
    };
    paths
        .into_iter()
        .map(|p| {
            let mut u = issuer.clone();
            u.set_path(&p);
            u.set_query(None);
            u.set_fragment(None);
            u
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_bearer_challenge() {
        let c = parse_challenge(
            r#"Bearer error="invalid_token", resource_metadata="https://h/.well-known/oauth-protected-resource/mcp", scope="mcp:read mcp:write""#,
        );
        assert_eq!(
            c.resource_metadata.as_deref(),
            Some("https://h/.well-known/oauth-protected-resource/mcp")
        );
        assert_eq!(c.scope.as_deref(), Some("mcp:read mcp:write"));
    }

    #[test]
    fn finds_bearer_among_other_schemes_and_handles_escapes() {
        let c = parse_challenge(
            r#"Basic realm="x", Bearer realm="a\"b", scope=read, resource_metadata="https://h/m""#,
        );
        assert_eq!(c.scope.as_deref(), Some("read"));
        assert_eq!(c.resource_metadata.as_deref(), Some("https://h/m"));
        assert_eq!(parse_challenge(r#"Basic realm="x""#), Challenge::default());
        assert_eq!(parse_challenge("Bearer"), Challenge::default());
    }

    #[test]
    fn stops_at_the_next_scheme() {
        let c = parse_challenge(r#"Bearer scope="a", DPoP algs="ES256""#);
        assert_eq!(c.scope.as_deref(), Some("a"));
    }

    #[test]
    fn requires_https_except_on_loopback() {
        for ok in [
            "https://as.example.com/token",
            "http://localhost:8080/token",
            "http://127.0.0.1/token",
            "http://[::1]/token",
        ] {
            assert!(require_secure(&Url::parse(ok).unwrap()).is_ok(), "{ok}");
        }
        for bad in ["http://as.example.com/token", "http://10.0.0.1/token"] {
            assert!(require_secure(&Url::parse(bad).unwrap()).is_err(), "{bad}");
        }
    }

    #[test]
    fn form_encodes_before_basic() {
        assert_eq!(form_encode("a b:c/é"), "a+b%3Ac%2F%C3%A9");
    }

    #[test]
    fn well_known_locations() {
        let prm = well_known_prm(&Url::parse("https://h/mcp?x=1").unwrap());
        assert_eq!(
            prm.iter().map(Url::as_str).collect::<Vec<_>>(),
            [
                "https://h/.well-known/oauth-protected-resource/mcp?x=1",
                "https://h/.well-known/oauth-protected-resource"
            ]
        );
        let asm = authorization_server_metadata_urls(&Url::parse("https://as/tenant1").unwrap());
        assert_eq!(
            asm.iter().map(Url::as_str).collect::<Vec<_>>(),
            [
                "https://as/.well-known/oauth-authorization-server/tenant1",
                "https://as/.well-known/openid-configuration/tenant1",
                "https://as/tenant1/.well-known/openid-configuration"
            ]
        );
        let root = authorization_server_metadata_urls(&Url::parse("https://as").unwrap());
        assert_eq!(
            root.iter().map(Url::as_str).collect::<Vec<_>>(),
            [
                "https://as/.well-known/oauth-authorization-server",
                "https://as/.well-known/openid-configuration"
            ]
        );
    }

    #[test]
    fn debug_redacts_secrets() {
        let c = ClientCredentials::client_secret("https://as", "id", "s3cr3t-value");
        let d = format!("{c:?}");
        assert!(!d.contains("s3cr3t-value"), "{d}");
        assert!(d.contains("<redacted>"), "{d}");
        let k = ClientCredentials::private_key_jwt(
            "https://as",
            "id",
            EncodingKey::from_secret(b"k3y-bytes"),
            Algorithm::RS256,
        );
        let d = format!("{k:?}");
        assert!(d.contains(r#"key: "<redacted>""#), "{d}");
    }

    #[test]
    fn expires_in_accepts_strings_and_survives_overflow() {
        let t: TokenResponse = serde_json::from_str(
            r#"{"access_token":"a","token_type":"Bearer","expires_in":"3600"}"#,
        )
        .unwrap();
        assert_eq!(t.expires_in, Some(3600));
        assert!(
            Instant::now()
                .checked_add(Duration::from_secs(u64::MAX))
                .is_none()
        );
    }

    #[test]
    fn short_lived_tokens_get_a_proportional_refresh_window() {
        let now = Instant::now();
        let t = |secs: u64, life: u64| Token {
            value: String::new(),
            expires_at: Some(now + Duration::from_secs(secs)),
            lifetime: Some(Duration::from_secs(life)),
        };
        assert!(!near_expiry(&t(20, 20)), "a fresh 20 s token is usable");
        assert!(near_expiry(&t(9, 20)));
        assert!(near_expiry(&t(29, 3600)));
        assert!(!near_expiry(&t(31, 3600)));
    }
}
