// ============================================================================
// DEMO ONLY — NOT FOR PRODUCTION USE
// ============================================================================
//
// A minimal OAuth 2.1 Authorization Server for local development, demos,
// and interoperability testing with Turul MCP Resource Servers.
//
// What this does:
//   - Serves AS metadata at /.well-known/oauth-authorization-server
//   - Serves JWKS at /.well-known/jwks.json
//   - Handles authorization code + PKCE flow (/authorize, /token)
//   - Issues JWTs signed with a static RSA key (survives restarts)
//   - Pre-registered client model
//
// What this does NOT do:
//   - No OIDC (no id_token, no /userinfo)
//   - No consent UI (returns JSON, not HTML)
//   - No rate limiting, no CSRF beyond PKCE
//   - No token revocation or introspection
//   - No persistent storage (in-memory only)
//
// Cargo.toml:
//   axum = "0.8"
//   jsonwebtoken = "9"
//   serde = { version = "1", features = ["derive"] }
//   serde_json = "1"
//   tokio = { version = "1", features = ["full"] }
//   uuid = { version = "1", features = ["v4"] }
//   base64 = "0.22"
//   sha2 = "0.10"
//   rsa = { version = "0.9", features = ["pkcs8"] }
//   rand = "0.8"
//
// Setup:
//   # Generate a static demo signing key (one-time):
//   openssl genrsa -out demo-key.pem 2048
//
//   # Run:
//   cargo run --bin demo-auth-server
//
// ============================================================================

use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use rsa::pkcs8::{DecodePrivateKey, EncodePublicKey};
use rsa::RsaPrivateKey;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

// ── Configuration ──────────────────────────────────────────────────────

const ISSUER: &str = "http://localhost:9000";
const ACCESS_TOKEN_TTL_SECS: u64 = 3600;
const REFRESH_TOKEN_TTL_SECS: u64 = 86400;
const AUTH_CODE_TTL_SECS: u64 = 300;

// ── Data Types ─────────────────────────────────────────────────────────

#[derive(Clone)]
struct ClientRecord {
    client_id: String,
    redirect_uris: Vec<String>,
    allowed_scopes: Vec<String>,
    token_endpoint_auth_method: String,
}

#[derive(Clone)]
struct AuthCode {
    code: String,
    client_id: String,
    redirect_uri: String,
    scope: String,
    resource: String,
    code_challenge: String,
    code_challenge_method: String,
    user_id: String,
    expires_at: u64,
}

#[derive(Clone)]
struct RefreshRecord {
    client_id: String,
    user_id: String,
    scope: String,
    resource: String,
    expires_at: u64,
}

#[derive(Clone)]
struct AppState {
    clients: HashMap<String, ClientRecord>,
    known_resources: Vec<String>,
    encoding_key: EncodingKey,
    jwks_json: serde_json::Value,
    kid: String,
    codes: Arc<RwLock<HashMap<String, AuthCode>>>,
    refresh_tokens: Arc<RwLock<HashMap<String, RefreshRecord>>>,
}

#[derive(Serialize)]
struct AccessTokenClaims {
    sub: String,
    iss: String,
    aud: String,
    scope: String,
    exp: u64,
    iat: u64,
    jti: String,
}

// ── Startup ────────────────────────────────────────────────────────────

fn load_signing_key() -> (RsaPrivateKey, String) {
    // Try loading static key; fall back to generation with warning
    match std::fs::read_to_string("demo-key.pem") {
        Ok(pem) => {
            let key = RsaPrivateKey::from_pkcs8_pem(&pem)
                .expect("demo-key.pem is not a valid PKCS#8 RSA key");
            eprintln!("[demo-as] Loaded signing key from demo-key.pem");
            (key, "demo-static-key-1".to_string())
        }
        Err(_) => {
            eprintln!("[demo-as] WARNING: demo-key.pem not found, generating ephemeral key.");
            eprintln!("[demo-as] Tokens will be invalidated when the server restarts.");
            eprintln!("[demo-as] To fix: openssl genrsa -out demo-key.pem 2048");
            let key = RsaPrivateKey::new(&mut rand::rngs::OsRng, 2048)
                .expect("Failed to generate RSA key");
            (key, format!("ephemeral-{}", &Uuid::new_v4().to_string()[..8]))
        }
    }
}

fn build_jwks(private_key: &RsaPrivateKey, kid: &str) -> serde_json::Value {
    let public_key = private_key.to_public_key();
    let public_key_der = public_key
        .to_public_key_der()
        .expect("Failed to encode public key");

    // Extract RSA components for JWK
    let n = URL_SAFE_NO_PAD.encode(public_key.n().to_bytes_be());
    let e = URL_SAFE_NO_PAD.encode(public_key.e().to_bytes_be());

    serde_json::json!({
        "keys": [{
            "kty": "RSA",
            "use": "sig",
            "alg": "RS256",
            "kid": kid,
            "n": n,
            "e": e,
        }]
    })
}

fn build_clients() -> HashMap<String, ClientRecord> {
    // Pre-registered demo client — add more as needed
    HashMap::from([(
        "demo-mcp-client".to_string(),
        ClientRecord {
            client_id: "demo-mcp-client".to_string(),
            redirect_uris: vec![
                "http://localhost:3000/callback".to_string(),
                "http://localhost:8080/callback".to_string(),
            ],
            allowed_scopes: vec!["mcp:read".to_string(), "mcp:write".to_string()],
            token_endpoint_auth_method: "none".to_string(),
        },
    )])
}

// ── Handlers ───────────────────────────────────────────────────────────

// GET /.well-known/oauth-authorization-server
async fn as_metadata(State(state): State<AppState>) -> impl IntoResponse {
    Json(serde_json::json!({
        "issuer": ISSUER,
        "authorization_endpoint": format!("{}/authorize", ISSUER),
        "token_endpoint": format!("{}/token", ISSUER),
        "jwks_uri": format!("{}/.well-known/jwks.json", ISSUER),
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code", "refresh_token"],
        "code_challenge_methods_supported": ["S256"],
        "scopes_supported": ["mcp:read", "mcp:write"],
        "token_endpoint_auth_methods_supported": ["none"],
    }))
}

// GET /.well-known/jwks.json
async fn jwks(State(state): State<AppState>) -> impl IntoResponse {
    Json(state.jwks_json)
}

// GET /authorize
#[derive(Deserialize)]
struct AuthorizeParams {
    response_type: String,
    client_id: String,
    redirect_uri: String,
    code_challenge: String,
    code_challenge_method: Option<String>,
    scope: Option<String>,
    resource: Option<String>,
    state: Option<String>,
}

async fn authorize(
    State(app): State<AppState>,
    Query(params): Query<AuthorizeParams>,
) -> impl IntoResponse {
    // Validate response_type
    if params.response_type != "code" {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "unsupported_response_type"})),
        );
    }

    // Validate client
    let client = match app.clients.get(&params.client_id) {
        Some(c) => c,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "invalid_client", "error_description": "Unknown client_id"})),
            );
        }
    };

    // Exact-match redirect URI — NEVER use prefix/wildcard matching
    if !client.redirect_uris.iter().any(|u| u == &params.redirect_uri) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "invalid_redirect_uri", "error_description": "Redirect URI not registered for this client"})),
        );
    }

    // PKCE is required for public clients
    let method = params.code_challenge_method.as_deref().unwrap_or("S256");
    if method != "S256" {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "invalid_request", "error_description": "Only S256 code_challenge_method is supported"})),
        );
    }

    // Validate scopes
    let requested_scopes: Vec<String> = params
        .scope
        .as_deref()
        .unwrap_or("")
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    if !requested_scopes.iter().all(|s| client.allowed_scopes.contains(s)) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "invalid_scope", "error_description": "Requested scope not allowed for this client"})),
        );
    }

    // Validate resource (audience) — must be a known RS
    let resource = params.resource.unwrap_or_default();
    if !resource.is_empty() && !app.known_resources.contains(&resource) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "invalid_target", "error_description": "Unknown resource/audience"})),
        );
    }

    // Issue authorization code
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let code = Uuid::new_v4().to_string();
    let auth_code = AuthCode {
        code: code.clone(),
        client_id: params.client_id.clone(),
        redirect_uri: params.redirect_uri.clone(),
        scope: requested_scopes.join(" "),
        resource: resource.clone(),
        code_challenge: params.code_challenge.clone(),
        code_challenge_method: method.to_string(),
        user_id: "demo-user".to_string(), // Demo: auto-approve, no consent UI
        expires_at: now + AUTH_CODE_TTL_SECS,
    };

    app.codes.write().unwrap().insert(code.clone(), auth_code);

    // In a real AS, this would redirect to a consent page.
    // For the demo, we return the code directly as JSON.
    let mut redirect = format!("{}?code={}", params.redirect_uri, code);
    if let Some(state) = params.state {
        redirect.push_str(&format!("&state={}", state));
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "redirect_to": redirect,
            "code": code,
            "note": "Demo AS: auto-approved. A real AS would show a consent page and redirect."
        })),
    )
}

// POST /token
#[derive(Deserialize)]
struct TokenParams {
    grant_type: String,
    code: Option<String>,
    redirect_uri: Option<String>,
    client_id: Option<String>,
    code_verifier: Option<String>,
    refresh_token: Option<String>,
    /// MCP spec requires resource on token requests (must match /authorize)
    resource: Option<String>,
}

async fn token(
    State(app): State<AppState>,
    axum::Form(params): axum::Form<TokenParams>,
) -> impl IntoResponse {
    match params.grant_type.as_str() {
        "authorization_code" => handle_code_exchange(app, params).await,
        "refresh_token" => handle_refresh(app, params).await,
        _ => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "unsupported_grant_type"})),
        ),
    }
}

async fn handle_code_exchange(
    app: AppState,
    params: TokenParams,
) -> (StatusCode, Json<serde_json::Value>) {
    let code_str = match &params.code {
        Some(c) => c,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "invalid_request", "error_description": "Missing code"})),
            );
        }
    };

    // Retrieve and consume the authorization code (single-use)
    let auth_code = match app.codes.write().unwrap().remove(code_str) {
        Some(c) => c,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "invalid_grant", "error_description": "Invalid or expired authorization code"})),
            );
        }
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Check expiration
    if now > auth_code.expires_at {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "invalid_grant", "error_description": "Authorization code expired"})),
        );
    }

    // Validate client_id matches
    if params.client_id.as_deref() != Some(&auth_code.client_id) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "invalid_client"})),
        );
    }

    // Validate redirect_uri matches
    if params.redirect_uri.as_deref() != Some(&auth_code.redirect_uri) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "invalid_grant", "error_description": "Redirect URI mismatch"})),
        );
    }

    // Validate resource matches what was authorized (MCP spec: MUST include resource at /token)
    if let Some(ref resource) = params.resource {
        if resource != &auth_code.resource {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "invalid_target", "error_description": "Resource does not match authorized resource"})),
            );
        }
    } else if !auth_code.resource.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "invalid_request", "error_description": "Missing resource parameter (required by MCP spec)"})),
        );
    }

    // Verify PKCE: SHA256(code_verifier) must match stored code_challenge
    let verifier = match &params.code_verifier {
        Some(v) => v,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "invalid_request", "error_description": "Missing code_verifier"})),
            );
        }
    };

    let computed_challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    if computed_challenge != auth_code.code_challenge {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "invalid_grant", "error_description": "PKCE verification failed"})),
        );
    }

    // Issue tokens
    issue_tokens(&app, &auth_code.user_id, &auth_code.scope, &auth_code.resource, &auth_code.client_id, now)
}

async fn handle_refresh(
    app: AppState,
    params: TokenParams,
) -> (StatusCode, Json<serde_json::Value>) {
    let refresh_str = match &params.refresh_token {
        Some(r) => r.clone(),
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "invalid_request", "error_description": "Missing refresh_token"})),
            );
        }
    };

    // Retrieve and consume refresh token (rotate on use)
    let record = match app.refresh_tokens.write().unwrap().remove(&refresh_str) {
        Some(r) => r,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "invalid_grant", "error_description": "Invalid refresh token"})),
            );
        }
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    if now > record.expires_at {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "invalid_grant", "error_description": "Refresh token expired"})),
        );
    }

    issue_tokens(&app, &record.user_id, &record.scope, &record.resource, &record.client_id, now)
}

fn issue_tokens(
    app: &AppState,
    user_id: &str,
    scope: &str,
    resource: &str,
    client_id: &str,
    now: u64,
) -> (StatusCode, Json<serde_json::Value>) {
    let claims = AccessTokenClaims {
        sub: user_id.to_string(),
        iss: ISSUER.to_string(),
        aud: resource.to_string(),
        scope: scope.to_string(),
        exp: now + ACCESS_TOKEN_TTL_SECS,
        iat: now,
        jti: Uuid::new_v4().to_string(),
    };

    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(app.kid.clone());

    let access_token = match encode(&header, &claims, &app.encoding_key) {
        Ok(t) => t,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "server_error", "error_description": e.to_string()})),
            );
        }
    };

    // Issue opaque refresh token
    let refresh_token = Uuid::new_v4().to_string();
    app.refresh_tokens.write().unwrap().insert(
        refresh_token.clone(),
        RefreshRecord {
            client_id: client_id.to_string(),
            user_id: user_id.to_string(),
            scope: scope.to_string(),
            resource: resource.to_string(),
            expires_at: now + REFRESH_TOKEN_TTL_SECS,
        },
    );

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "access_token": access_token,
            "token_type": "Bearer",
            "expires_in": ACCESS_TOKEN_TTL_SECS,
            "refresh_token": refresh_token,
            "scope": scope,
        })),
    )
}

// ── Main ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    eprintln!("============================================");
    eprintln!("  DEMO OAuth Authorization Server");
    eprintln!("  NOT FOR PRODUCTION USE");
    eprintln!("============================================");

    let (private_key, kid) = load_signing_key();
    let jwks_json = build_jwks(&private_key, &kid);

    let private_key_der = rsa::pkcs8::EncodePrivateKey::to_pkcs8_der(&private_key)
        .expect("Failed to encode private key");
    let encoding_key = EncodingKey::from_rsa_der(private_key_der.as_bytes());

    let state = AppState {
        clients: build_clients(),
        known_resources: vec![
            "https://example.com/mcp".to_string(),
            "http://localhost:8080/mcp".to_string(),
        ],
        encoding_key,
        jwks_json,
        kid,
        codes: Arc::new(RwLock::new(HashMap::new())),
        refresh_tokens: Arc::new(RwLock::new(HashMap::new())),
    };

    let app = Router::new()
        .route("/.well-known/oauth-authorization-server", get(as_metadata))
        .route("/.well-known/jwks.json", get(jwks))
        .route("/authorize", get(authorize))
        .route("/token", post(token))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:9000")
        .await
        .expect("Failed to bind to port 9000");

    eprintln!("[demo-as] Listening on http://localhost:9000");
    eprintln!("[demo-as] AS metadata: http://localhost:9000/.well-known/oauth-authorization-server");
    eprintln!("[demo-as] JWKS:        http://localhost:9000/.well-known/jwks.json");
    eprintln!("[demo-as] Pre-registered client: demo-mcp-client");

    axum::serve(listener, app).await.unwrap();
}
