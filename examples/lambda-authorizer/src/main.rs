//! API Gateway REQUEST Authorizer for MCP Streamable HTTP
//!
//! A standalone Lambda authorizer that validates API keys and returns IAM policy
//! responses with **wildcarded methodArn**. This is essential for MCP Streamable
//! HTTP, which uses POST (requests), GET (SSE streaming), and DELETE (session
//! termination) on the same endpoint.
//!
//! ## Why Wildcard the methodArn?
//!
//! API Gateway REST API (v1) caches authorizer responses keyed by the full
//! `methodArn` (including HTTP method). Without wildcarding, the cached policy
//! from the first request (e.g., `Allow POST/`) blocks subsequent `GET` and
//! `DELETE` requests with 403.
//!
//! ## Event Format Support
//!
//! - **REST API v1**: Returns IAM policy response (`principalId`, `policyDocument`, `context`)
//! - **HTTP API v2**: Returns simple response (`isAuthorized`, `context`)
//!
//! ## Demo API Keys
//!
//! - `secret-key-123` → user-alice (admin)
//! - `secret-key-456` → user-bob (reader)
//!
//! ## Deployment
//!
//! ```bash
//! cargo lambda build --release --package lambda-authorizer
//! cargo lambda deploy lambda-authorizer
//! ```

use aws_lambda_events::apigw::{
    ApiGatewayCustomAuthorizerPolicy, ApiGatewayCustomAuthorizerResponse,
    ApiGatewayV2CustomAuthorizerSimpleResponse,
};
use aws_lambda_events::iam::{IamPolicyEffect, IamPolicyStatement};
use lambda_runtime::{Error, LambdaEvent, run, service_fn};
use serde_json::{Value, json};
use std::collections::HashMap;
use tracing::{Level, debug, error, info, warn};

// ---------------------------------------------------------------------------
// Demo API key store
// ---------------------------------------------------------------------------

/// User info associated with a valid API key
struct UserInfo {
    user_id: &'static str,
    role: &'static str,
}

/// Look up an API key and return associated user info.
/// In production, replace this with a database or secrets manager lookup.
fn lookup_api_key(api_key: &str) -> Option<UserInfo> {
    match api_key {
        "secret-key-123" => Some(UserInfo {
            user_id: "user-alice",
            role: "admin",
        }),
        "secret-key-456" => Some(UserInfo {
            user_id: "user-bob",
            role: "reader",
        }),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Event detection
// ---------------------------------------------------------------------------

/// Detected authorizer event type
#[derive(Debug)]
enum AuthorizerEventType {
    V1Rest,
    V2Http,
}

/// Inspect raw JSON to determine REST v1 vs HTTP API v2 event.
///
/// - v1: has `type: "REQUEST"` and `methodArn`
/// - v2: has `type: "REQUEST"`, `version: "2.0"`, and `routeArn`
fn detect_event_type(payload: &Value) -> Result<AuthorizerEventType, String> {
    let type_field = payload.get("type").and_then(|v| v.as_str());

    if type_field != Some("REQUEST") {
        return Err(format!(
            "Unsupported authorizer type: {:?} (expected REQUEST)",
            type_field
        ));
    }

    let has_version_2 = payload.get("version").and_then(|v| v.as_str()) == Some("2.0");
    let has_route_arn = payload.get("routeArn").is_some();
    let has_method_arn = payload.get("methodArn").is_some();

    if has_version_2 && has_route_arn {
        Ok(AuthorizerEventType::V2Http)
    } else if has_method_arn {
        Ok(AuthorizerEventType::V1Rest)
    } else {
        Err("Unknown authorizer event: no routeArn or methodArn".to_string())
    }
}

// ---------------------------------------------------------------------------
// API key extraction
// ---------------------------------------------------------------------------

/// Extract x-api-key from raw event headers, case-insensitive.
/// Falls back to multiValueHeaders (REST v1 events may only populate that field).
fn extract_api_key(payload: &Value) -> Option<String> {
    // Try headers first (present on both v1 and v2)
    if let Some(key) = extract_api_key_from_object(payload.get("headers")) {
        return Some(key);
    }
    // Fallback: REST v1 multiValueHeaders (values are arrays)
    if let Some(headers) = payload.get("multiValueHeaders").and_then(|v| v.as_object()) {
        for (key, value) in headers {
            if key.eq_ignore_ascii_case("x-api-key") {
                if let Some(first) = value.as_array().and_then(|arr| arr.first()) {
                    return first.as_str().map(String::from);
                }
            }
        }
    }
    None
}

/// Search a JSON object for x-api-key (case-insensitive)
fn extract_api_key_from_object(headers: Option<&Value>) -> Option<String> {
    let headers = headers?.as_object()?;
    for (key, value) in headers {
        if key.eq_ignore_ascii_case("x-api-key") {
            return value.as_str().map(String::from);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// methodArn wildcarding
// ---------------------------------------------------------------------------

/// Wildcard the HTTP method and resource path in a methodArn.
///
/// methodArn format: `arn:aws:execute-api:{region}:{account}:{api-id}/{stage}/{method}/{resource...}`
///
/// This replaces `{stage}/{method}/{resource...}` with `{stage}/*/*`, ensuring
/// the cached IAM policy applies to all methods and paths. Required for MCP
/// Streamable HTTP which uses POST + GET + DELETE on the same endpoint.
fn wildcard_method_arn(method_arn: &str) -> String {
    // Split on '/' to find the stage, then wildcard everything after it
    if let Some(slash_pos) = method_arn.find('/') {
        if let Some(second_slash) = method_arn[slash_pos + 1..].find('/') {
            let prefix = &method_arn[..slash_pos + 1 + second_slash];
            return format!("{prefix}/*/*");
        }
    }
    method_arn.to_string()
}

// ---------------------------------------------------------------------------
// IAM policy builders (v1 REST API)
// ---------------------------------------------------------------------------

/// Build an IAM policy statement
fn build_iam_statement(effect: IamPolicyEffect, resource: &str) -> IamPolicyStatement {
    let mut stmt = IamPolicyStatement::default();
    stmt.action = vec!["execute-api:Invoke".to_string()];
    stmt.effect = effect;
    stmt.resource = vec![resource.to_string()];
    stmt
}

/// Build an IAM policy document
fn build_policy_document(
    effect: IamPolicyEffect,
    resource: &str,
) -> ApiGatewayCustomAuthorizerPolicy {
    let mut policy = ApiGatewayCustomAuthorizerPolicy::default();
    policy.version = Some("2012-10-17".to_string());
    policy.statement = vec![build_iam_statement(effect, resource)];
    policy
}

/// Build IAM policy Allow response for REST API v1 authorizer.
/// Wildcards the methodArn before building the policy.
fn build_v1_allow_response(
    principal_id: &str,
    method_arn: &str,
    context: HashMap<String, String>,
) -> ApiGatewayCustomAuthorizerResponse<HashMap<String, String>> {
    let wildcard_arn = wildcard_method_arn(method_arn);
    let mut resp = ApiGatewayCustomAuthorizerResponse::default();
    resp.principal_id = Some(principal_id.to_string());
    resp.policy_document = build_policy_document(IamPolicyEffect::Allow, &wildcard_arn);
    resp.context = context;
    resp
}

/// Build IAM policy Deny response for REST API v1 authorizer.
/// Uses original (non-wildcarded) ARN for deny.
fn build_v1_deny_response(
    method_arn: &str,
    error_message: &str,
) -> ApiGatewayCustomAuthorizerResponse<HashMap<String, String>> {
    warn!("Authorization failed (v1): {}", error_message);
    let mut context = HashMap::new();
    context.insert("error".to_string(), error_message.to_string());
    let mut resp = ApiGatewayCustomAuthorizerResponse::default();
    resp.principal_id = Some("unauthorized".to_string());
    resp.policy_document = build_policy_document(IamPolicyEffect::Deny, method_arn);
    resp.context = context;
    resp
}

// ---------------------------------------------------------------------------
// Simple response builders (v2 HTTP API)
// ---------------------------------------------------------------------------

/// Build simple Allow response for HTTP API v2 authorizer.
fn build_v2_allow_response(context: Value) -> ApiGatewayV2CustomAuthorizerSimpleResponse {
    let mut resp = ApiGatewayV2CustomAuthorizerSimpleResponse::default();
    resp.is_authorized = true;
    resp.context = context;
    resp
}

/// Build simple Deny response for HTTP API v2 authorizer.
fn build_v2_deny_response(error_message: &str) -> ApiGatewayV2CustomAuthorizerSimpleResponse {
    warn!("Authorization failed (v2): {}", error_message);
    let mut resp = ApiGatewayV2CustomAuthorizerSimpleResponse::default();
    resp.is_authorized = false;
    resp.context = json!({"error": error_message});
    resp
}

// ---------------------------------------------------------------------------
// Handler dispatch
// ---------------------------------------------------------------------------

/// Main handler — accepts raw JSON, dispatches to v1 or v2 response format.
async fn function_handler(event: LambdaEvent<Value>) -> Result<Value, Error> {
    let payload = &event.payload;

    // Detect event format (v1 REST or v2 HTTP API)
    let event_type = match detect_event_type(payload) {
        Ok(t) => t,
        Err(e) => {
            error!("Unrecognized authorizer event: {} — returning deny", e);
            if let Some(arn) = payload.get("methodArn").and_then(|v| v.as_str()) {
                let resp = build_v1_deny_response(arn, "Unrecognized authorizer event");
                return Ok(serde_json::to_value(resp)?);
            }
            let resp = build_v2_deny_response("Unrecognized authorizer event");
            return Ok(serde_json::to_value(resp)?);
        }
    };

    // Extract API key from headers
    let api_key = match extract_api_key(payload) {
        Some(key) => key,
        None => {
            error!("Missing x-api-key header");
            return match event_type {
                AuthorizerEventType::V1Rest => {
                    let method_arn = payload
                        .get("methodArn")
                        .and_then(|v| v.as_str())
                        .unwrap_or("*");
                    Ok(serde_json::to_value(build_v1_deny_response(
                        method_arn,
                        "Missing API key",
                    ))?)
                }
                AuthorizerEventType::V2Http => Ok(serde_json::to_value(build_v2_deny_response(
                    "Missing API key",
                ))?),
            };
        }
    };

    // Look up the API key
    let user_info = match lookup_api_key(&api_key) {
        Some(info) => {
            info!(
                "Authorized: userId={}, role={}",
                info.user_id, info.role
            );
            info
        }
        None => {
            error!("Invalid API key");
            return match event_type {
                AuthorizerEventType::V1Rest => {
                    let method_arn = payload
                        .get("methodArn")
                        .and_then(|v| v.as_str())
                        .unwrap_or("*");
                    Ok(serde_json::to_value(build_v1_deny_response(
                        method_arn,
                        "Invalid API key",
                    ))?)
                }
                AuthorizerEventType::V2Http => Ok(serde_json::to_value(build_v2_deny_response(
                    "Invalid API key",
                ))?),
            };
        }
    };

    // Build response based on event type
    match event_type {
        AuthorizerEventType::V1Rest => {
            let method_arn = match payload.get("methodArn").and_then(|v| v.as_str()) {
                Some(arn) => arn,
                None => {
                    error!("v1 event missing methodArn — returning Deny");
                    let resp = build_v1_deny_response("*", "Malformed authorizer event");
                    return Ok(serde_json::to_value(resp)?);
                }
            };

            let mut context = HashMap::new();
            context.insert("userId".to_string(), user_info.user_id.to_string());
            context.insert("role".to_string(), user_info.role.to_string());

            let resp = build_v1_allow_response(user_info.user_id, method_arn, context);
            debug!("v1 Allow response for principal={}", user_info.user_id);
            Ok(serde_json::to_value(resp)?)
        }
        AuthorizerEventType::V2Http => {
            let context = json!({
                "userId": user_info.user_id,
                "role": user_info.role,
            });
            let resp = build_v2_allow_response(context);
            debug!("v2 Allow response for userId={}", user_info.user_id);
            Ok(serde_json::to_value(resp)?)
        }
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<(), Error> {
    if std::env::var("AWS_EXECUTION_ENV").is_ok() {
        tracing_subscriber::fmt()
            .with_max_level(Level::INFO)
            .with_target(false)
            .without_time()
            .json()
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_max_level(Level::INFO)
            .with_target(false)
            .init();
    }

    info!("Lambda Authorizer starting (MCP Streamable HTTP)");
    info!("Demo keys: secret-key-123 (alice/admin), secret-key-456 (bob/reader)");

    run(service_fn(function_handler)).await
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Event detection tests --

    #[test]
    fn test_detect_v1_event() {
        let payload = json!({
            "type": "REQUEST",
            "methodArn": "arn:aws:execute-api:us-west-2:111222333444:r8knz0x1b9/staging/POST/",
            "headers": {"x-api-key": "test-key"}
        });
        assert!(matches!(
            detect_event_type(&payload).unwrap(),
            AuthorizerEventType::V1Rest
        ));
    }

    #[test]
    fn test_detect_v2_event() {
        let payload = json!({
            "type": "REQUEST",
            "version": "2.0",
            "routeArn": "arn:aws:execute-api:us-west-2:111222333444:r8knz0x1b9/$default",
            "headers": {"x-api-key": "test-key"}
        });
        assert!(matches!(
            detect_event_type(&payload).unwrap(),
            AuthorizerEventType::V2Http
        ));
    }

    #[test]
    fn test_detect_unknown_event() {
        let payload = json!({
            "type": "REQUEST",
            "headers": {"x-api-key": "test-key"}
        });
        assert!(detect_event_type(&payload).is_err());
    }

    // -- API key extraction tests --

    #[test]
    fn test_extract_api_key_lowercase() {
        let payload = json!({"headers": {"x-api-key": "my-secret-key"}});
        assert_eq!(extract_api_key(&payload), Some("my-secret-key".to_string()));
    }

    #[test]
    fn test_extract_api_key_mixed_case() {
        let payload = json!({"headers": {"X-Api-Key": "my-secret-key"}});
        assert_eq!(extract_api_key(&payload), Some("my-secret-key".to_string()));
    }

    #[test]
    fn test_extract_api_key_from_multi_value_headers() {
        let payload = json!({
            "headers": {"content-type": "application/json"},
            "multiValueHeaders": {"x-api-key": ["my-secret-key"]}
        });
        assert_eq!(extract_api_key(&payload), Some("my-secret-key".to_string()));
    }

    #[test]
    fn test_extract_api_key_missing() {
        let payload = json!({"headers": {"content-type": "application/json"}});
        assert_eq!(extract_api_key(&payload), None);
    }

    // -- methodArn wildcarding tests --

    #[test]
    fn test_wildcard_method_arn() {
        // POST on root (us-west-2)
        assert_eq!(
            wildcard_method_arn(
                "arn:aws:execute-api:us-west-2:111222333444:r8knz0x1b9/staging/POST/"
            ),
            "arn:aws:execute-api:us-west-2:111222333444:r8knz0x1b9/staging/*/*"
        );

        // GET with resource path (eu-central-1)
        assert_eq!(
            wildcard_method_arn(
                "arn:aws:execute-api:eu-central-1:998877665544:m4pqrs7tuv/v1/GET/items"
            ),
            "arn:aws:execute-api:eu-central-1:998877665544:m4pqrs7tuv/v1/*/*"
        );

        // DELETE with deep path (us-east-1)
        assert_eq!(
            wildcard_method_arn(
                "arn:aws:execute-api:us-east-1:000111222333:j5abc2def1/test/DELETE/sessions/abc"
            ),
            "arn:aws:execute-api:us-east-1:000111222333:j5abc2def1/test/*/*"
        );

        // Malformed ARN without slashes returns as-is
        assert_eq!(wildcard_method_arn("no-slashes"), "no-slashes");
    }

    // -- v1 response shape tests --

    #[test]
    fn test_v1_allow_response_wildcards_method_arn() {
        let mut context = HashMap::new();
        context.insert("userId".to_string(), "user-alice".to_string());
        context.insert("role".to_string(), "admin".to_string());

        let resp = build_v1_allow_response(
            "user-alice",
            "arn:aws:execute-api:us-west-2:111222333444:r8knz0x1b9/staging/POST/",
            context,
        );
        let json = serde_json::to_value(&resp).unwrap();

        assert_eq!(json["principalId"], "user-alice");
        assert_eq!(json["policyDocument"]["Version"], "2012-10-17");
        assert_eq!(json["policyDocument"]["Statement"][0]["Effect"], "Allow");
        assert_eq!(
            json["policyDocument"]["Statement"][0]["Action"][0],
            "execute-api:Invoke"
        );
        // Resource must be wildcarded — not the original staging/POST/
        assert_eq!(
            json["policyDocument"]["Statement"][0]["Resource"][0],
            "arn:aws:execute-api:us-west-2:111222333444:r8knz0x1b9/staging/*/*"
        );
        assert_eq!(json["context"]["userId"], "user-alice");
        assert_eq!(json["context"]["role"], "admin");
    }

    #[test]
    fn test_v1_deny_response_shape() {
        let resp = build_v1_deny_response(
            "arn:aws:execute-api:eu-central-1:998877665544:m4pqrs7tuv/v1/GET/items",
            "Invalid API key",
        );
        let json = serde_json::to_value(&resp).unwrap();

        assert_eq!(json["principalId"], "unauthorized");
        assert_eq!(json["policyDocument"]["Statement"][0]["Effect"], "Deny");
        // Deny uses original ARN (not wildcarded)
        assert_eq!(
            json["policyDocument"]["Statement"][0]["Resource"][0],
            "arn:aws:execute-api:eu-central-1:998877665544:m4pqrs7tuv/v1/GET/items"
        );
        assert_eq!(json["context"]["error"], "Invalid API key");
    }

    // -- v2 response shape tests --

    #[test]
    fn test_v2_allow_response_shape() {
        let context = json!({"userId": "user-alice", "role": "admin"});
        let resp = build_v2_allow_response(context);
        let json = serde_json::to_value(&resp).unwrap();

        assert_eq!(json["isAuthorized"], true);
        assert_eq!(json["context"]["userId"], "user-alice");
        assert_eq!(json["context"]["role"], "admin");
    }

    #[test]
    fn test_v2_deny_response_shape() {
        let resp = build_v2_deny_response("Invalid API key");
        let json = serde_json::to_value(&resp).unwrap();

        assert_eq!(json["isAuthorized"], false);
        assert_eq!(json["context"]["error"], "Invalid API key");
    }
}
