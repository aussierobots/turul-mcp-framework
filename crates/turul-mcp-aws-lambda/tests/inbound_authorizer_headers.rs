//! `x-authorizer-*` request headers are set only by the adapter, from the API
//! Gateway authorizer context. A header with that prefix arriving from the
//! client must never reach middleware, which trusts it as authorizer output.

use serde_json::{Value, json};
use turul_mcp_aws_lambda::adapter::lambda_to_hyper_request;

fn event(client_headers: Value, authorizer: Option<Value>) -> lambda_http::Request {
    let mut ctx = json!({
        "accountId": "123456789",
        "apiId": "api-123",
        "domainName": "api-123.execute-api.us-east-1.amazonaws.com",
        "http": {
            "method": "POST", "path": "/mcp", "protocol": "HTTP/1.1",
            "sourceIp": "1.2.3.4", "userAgent": "test"
        },
        "requestId": "req-456",
        "stage": "$default",
        "time": "01/Jan/2025:00:00:00 +0000",
        "timeEpoch": 1735689600000u64
    });
    if let Some(a) = authorizer {
        ctx["authorizer"] = json!({ "lambda": a });
    }
    let event = json!({
        "version": "2.0",
        "routeKey": "POST /mcp",
        "rawPath": "/mcp",
        "rawQueryString": "",
        "headers": client_headers,
        "requestContext": ctx,
        "body": "{}",
        "isBase64Encoded": false
    });
    lambda_http::request::from_str(&event.to_string()).expect("valid API Gateway v2 event")
}

fn header(req: &hyper::Request<impl Sized>, name: &str) -> Option<String> {
    req.headers()
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(String::from)
}

#[test]
fn client_sent_authorizer_headers_are_dropped_when_an_authorizer_ran() {
    let req = event(
        json!({
            "content-type": "application/json",
            "x-authorizer-account_id": "spoofed",
            "x-authorizer-extra": "spoofed"
        }),
        Some(json!({ "account_id": "real" })),
    );
    let out = lambda_to_hyper_request(req).expect("convert");

    assert_eq!(
        header(&out, "x-authorizer-account_id").as_deref(),
        Some("real")
    );
    assert_eq!(
        header(&out, "x-authorizer-extra"),
        None,
        "a field the authorizer did not emit must not survive from the client"
    );
    assert_eq!(
        header(&out, "content-type").as_deref(),
        Some("application/json"),
        "unrelated headers are preserved"
    );
}

#[test]
fn client_sent_authorizer_headers_are_dropped_when_no_authorizer_ran() {
    let req = event(
        json!({
            "content-type": "application/json",
            "x-authorizer-account_id": "spoofed"
        }),
        None,
    );
    let out = lambda_to_hyper_request(req).expect("convert");

    assert_eq!(
        header(&out, "x-authorizer-account_id"),
        None,
        "with no authorizer context there is nothing to inject and nothing to trust"
    );
    assert_eq!(
        header(&out, "content-type").as_deref(),
        Some("application/json")
    );
}
