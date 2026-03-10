// ============================================================================
// DEMO ONLY — NOT FOR PRODUCTION USE
// ============================================================================
//
// Extends demo-auth-server.rs with Dynamic Client Registration (RFC 7591).
//
// This file shows ONLY the DCR additions. In practice, you would add these
// to the base demo-auth-server.rs example.
//
// Cargo.toml: same as demo-auth-server.rs
//
// ============================================================================

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse, routing::post};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

// ── Client Record (same as base example) ───────────────────────────────

#[derive(Clone)]
struct ClientRecord {
    client_id: String,
    redirect_uris: Vec<String>,
    allowed_scopes: Vec<String>,
    token_endpoint_auth_method: String,
}

// ── DCR Request / Response ─────────────────────────────────────────────

#[derive(Deserialize)]
struct RegistrationRequest {
    redirect_uris: Vec<String>,
    /// Optional: requested scopes. AS may grant a subset.
    scope: Option<String>,
    /// Must be "none" for public clients (PKCE flow).
    token_endpoint_auth_method: Option<String>,
    client_name: Option<String>,
}

#[derive(Serialize)]
struct RegistrationResponse {
    client_id: String,
    redirect_uris: Vec<String>,
    scope: String,
    token_endpoint_auth_method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    client_name: Option<String>,
}

// ── Shared State (add dynamic_clients alongside pre-registered ones) ──

#[derive(Clone)]
struct DcrState {
    /// Pre-registered clients (immutable)
    static_clients: HashMap<String, ClientRecord>,
    /// DCR-registered clients (mutable)
    dynamic_clients: Arc<RwLock<HashMap<String, ClientRecord>>>,
    /// Scopes the AS is willing to grant
    allowed_scopes: Vec<String>,
}

impl DcrState {
    fn lookup_client(&self, client_id: &str) -> Option<ClientRecord> {
        self.static_clients
            .get(client_id)
            .cloned()
            .or_else(|| self.dynamic_clients.read().unwrap().get(client_id).cloned())
    }
}

// ── POST /register ─────────────────────────────────────────────────────

async fn register(
    State(state): State<DcrState>,
    Json(req): Json<RegistrationRequest>,
) -> impl IntoResponse {
    // Validate redirect URIs are present
    if req.redirect_uris.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "invalid_client_metadata",
                "error_description": "At least one redirect_uri is required"
            })),
        );
    }

    // Validate redirect URIs are HTTP(S) — reject javascript:, data:, etc.
    for uri in &req.redirect_uris {
        if !uri.starts_with("http://") && !uri.starts_with("https://") {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "invalid_redirect_uri",
                    "error_description": format!("Redirect URI must be HTTP(S): {}", uri)
                })),
            );
        }
    }

    // NOTE: This registration policy is intentionally permissive for demo use.
    // A production DCR endpoint would authenticate the registrant, restrict
    // allowed redirect URIs to known domains, and enforce stricter scope grants.

    // Only public clients (token_endpoint_auth_method = "none") supported in demo
    let auth_method = req
        .token_endpoint_auth_method
        .unwrap_or_else(|| "none".to_string());
    if auth_method != "none" {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "invalid_client_metadata",
                "error_description": "Only public clients (token_endpoint_auth_method=none) are supported"
            })),
        );
    }

    // Grant scopes: intersection of requested and AS-allowed
    let requested_scopes: Vec<String> = req
        .scope
        .as_deref()
        .unwrap_or("")
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    let granted_scopes: Vec<String> = if requested_scopes.is_empty() {
        state.allowed_scopes.clone() // Grant all if none requested
    } else {
        requested_scopes
            .into_iter()
            .filter(|s| state.allowed_scopes.contains(s))
            .collect()
    };

    // Generate client_id
    let client_id = format!("dcr-{}", Uuid::new_v4());

    let record = ClientRecord {
        client_id: client_id.clone(),
        redirect_uris: req.redirect_uris.clone(),
        allowed_scopes: granted_scopes.clone(),
        token_endpoint_auth_method: auth_method.clone(),
    };

    state
        .dynamic_clients
        .write()
        .unwrap()
        .insert(client_id.clone(), record);

    eprintln!("[demo-as] DCR: registered client {}", client_id);

    (
        StatusCode::CREATED,
        Json(serde_json::json!({
            "client_id": client_id,
            "redirect_uris": req.redirect_uris,
            "scope": granted_scopes.join(" "),
            "token_endpoint_auth_method": auth_method,
            "client_name": req.client_name,
        })),
    )
}

// ── Integration Notes ──────────────────────────────────────────────────

// To add DCR to the base demo-auth-server.rs:
//
// 1. Add DcrState (or extend AppState with dynamic_clients)
//
// 2. Add route:
//    .route("/register", post(register))
//
// 3. Update AS metadata to advertise:
//    "registration_endpoint": "http://localhost:9000/register"
//
// 4. Update client lookup in /authorize and /token to check both
//    static_clients and dynamic_clients (see DcrState::lookup_client)
//
// Usage:
//   curl -X POST http://localhost:9000/register \
//     -H "Content-Type: application/json" \
//     -d '{
//       "redirect_uris": ["http://localhost:4000/callback"],
//       "scope": "mcp:read",
//       "token_endpoint_auth_method": "none",
//       "client_name": "My Test Client"
//     }'
//
// Response:
//   {
//     "client_id": "dcr-a1b2c3d4-...",
//     "redirect_uris": ["http://localhost:4000/callback"],
//     "scope": "mcp:read",
//     "token_endpoint_auth_method": "none",
//     "client_name": "My Test Client"
//   }
