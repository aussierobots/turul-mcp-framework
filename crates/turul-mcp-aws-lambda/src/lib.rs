//! AWS Lambda integration for turul-mcp-framework
//!
//! This crate provides seamless integration between the turul-mcp-framework and AWS Lambda,
//! enabling serverless deployment of MCP servers with proper session management, CORS handling,
//! and SSE streaming support.
//!
//! ## Architecture
//!
//! The crate bridges the gap between Lambda's HTTP execution model and the framework's
//! hyper-based architecture through:
//!
//! - **Type Conversion**: Clean conversion between `lambda_http` and `hyper` types
//! - **Handler Registration**: Direct tool registration with `JsonRpcDispatcher`
//! - **Session Management**: DynamoDB-backed session persistence across invocations
//! - **CORS Support**: Proper CORS header injection for browser clients
//! - **SSE Streaming**: Server-Sent Events adaptation through Lambda's streaming response
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use turul_mcp_aws_lambda::LambdaMcpServerBuilder;
//! use turul_mcp_derive::McpTool;
//! use turul_mcp_server::{McpResult, SessionContext};
//!
//! #[derive(McpTool, Clone, Default)]
//! #[tool(name = "example", description = "Example tool")]
//! struct ExampleTool {
//!     #[param(description = "Example parameter")]
//!     value: String,
//! }
//!
//! impl ExampleTool {
//!     async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
//!         Ok(format!("Got: {}", self.value))
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), lambda_http::Error> {
//!     let server = LambdaMcpServerBuilder::new()
//!         .tool(ExampleTool::default())
//!         .cors_allow_all_origins()
//!         .build()
//!         .await?;
//!
//!     let handler = server.handler().await?;
//!
//!     // run_streaming handles API Gateway completion invocations gracefully
//!     turul_mcp_aws_lambda::run_streaming(handler).await
//! }
//! ```
//!
//! ## Streaming Entry Points
//!
//! Two entry points replace `lambda_http::run_with_streaming_response()`:
//!
//! - [`run_streaming()`] — standard path: pass a [`LambdaMcpHandler`] directly
//! - [`run_streaming_with()`] — custom dispatch: provide your own closure for
//!   pre-dispatch logic (e.g., `.well-known` routing) while still getting
//!   completion-invocation handling for free

pub mod adapter;
pub mod builder;
pub mod error;
pub mod handler;
pub mod prelude;
pub mod server;

#[cfg(feature = "cors")]
pub mod cors;

#[cfg(feature = "sse")]
pub mod streaming;

// Re-exports for convenience
/// Builder for creating Lambda MCP servers with fluent configuration API
pub use builder::LambdaMcpServerBuilder;
/// Lambda-specific error types and result aliases
pub use error::{LambdaError, Result};
/// Lambda request handler with session management and protocol conversion
pub use handler::LambdaMcpHandler;
/// Core Lambda MCP server implementation with DynamoDB integration
pub use server::LambdaMcpServer;

#[cfg(feature = "cors")]
pub use cors::CorsConfig;

/// Classification of a raw Lambda runtime event payload.
///
/// Used by [`run_streaming()`] to distinguish API Gateway requests from
/// streaming completion invocations and unknown event shapes.
#[derive(Debug)]
enum RuntimeEventClassification {
    /// Valid API Gateway / ALB / Function URL event.
    ///
    /// Stores `Box<LambdaRequest>` to avoid a large enum variant (clippy
    /// `large_enum_variant`). Callers dereference with `(*lambda_request).into()`
    /// to move the inner `LambdaRequest` into an `http::Request`.
    ApiGatewayEvent(Box<lambda_http::request::LambdaRequest>),
    /// AWS streaming completion invocation (contains `invokeCompletionStatus`)
    StreamingCompletion,
    /// Unrecognized payload — not API Gateway, not completion
    UnrecognizedEvent,
}

/// Classify a raw JSON payload into one of three categories.
///
/// Order matters:
/// 1. Try API Gateway/ALB/WebSocket deserialization first (most common path)
/// 2. Check for streaming completion signature (`invokeCompletionStatus` at top level)
/// 3. Everything else is unrecognized
///
/// # Completion Detection Heuristic
///
/// Streaming completion payloads are identified by the presence of an
/// `invokeCompletionStatus` field at the top level. This is a compatibility
/// heuristic based on observed AWS behavior as of 2026-03 — AWS does not
/// officially document this payload shape.
///
/// As of this writing, no API Gateway v1/v2, ALB, or WebSocket event
/// produced by `lambda_http` contains this field at the top level.
/// The fixture corpus in `src/fixtures/` guards against drift.
///
/// **Precedence**: API Gateway deserialization is attempted first.
/// If a payload is both a valid API Gateway event AND contains
/// `invokeCompletionStatus`, it will be classified as `ApiGatewayEvent`.
/// The completion heuristic only applies to payloads that fail API
/// Gateway parsing. This means false-positive completion detection is
/// preferred over retry storms — an intentional design choice.
fn classify_runtime_event(payload: serde_json::Value) -> RuntimeEventClassification {
    // Fast path: try API Gateway event deserialization
    if let Ok(request) =
        serde_json::from_value::<lambda_http::request::LambdaRequest>(payload.clone())
    {
        return RuntimeEventClassification::ApiGatewayEvent(Box::new(request));
    }

    // Check for streaming completion signature
    if payload.get("invokeCompletionStatus").is_some() {
        return RuntimeEventClassification::StreamingCompletion;
    }

    // Unknown payload shape
    RuntimeEventClassification::UnrecognizedEvent
}

type StreamBody = http_body_util::combinators::UnsyncBoxBody<bytes::Bytes, hyper::Error>;
type StreamResult = lambda_runtime::StreamResponse<http_body_util::BodyDataStream<StreamBody>>;

/// Result of [`handle_runtime_payload()`], carrying both the Lambda response
/// and a static string identifying the event type for logging/observability.
struct HandleResult {
    response: StreamResult,
    /// One of `"api_gateway_event"`, `"streaming_completion"`, or
    /// `"unrecognized_lambda_payload"`.
    event_type: &'static str,
}

/// Process a raw Lambda runtime payload into a streaming response.
///
/// Classifies the payload via [`classify_runtime_event()`], dispatches API
/// Gateway events through `dispatch`, and acknowledges non-API payloads with
/// an empty 200 response. Returns a [`HandleResult`] so the caller can
/// inspect `event_type` for logging decisions.
async fn handle_runtime_payload<F, Fut>(
    payload: serde_json::Value,
    context: lambda_runtime::Context,
    dispatch: F,
) -> std::result::Result<HandleResult, lambda_http::Error>
where
    F: FnOnce(lambda_http::Request) -> Fut,
    Fut: std::future::Future<
            Output = std::result::Result<http::Response<StreamBody>, lambda_http::Error>,
        >,
{
    match classify_runtime_event(payload) {
        RuntimeEventClassification::ApiGatewayEvent(lambda_request) => {
            use lambda_http::RequestExt;
            let request: lambda_http::Request = (*lambda_request).into();
            let request = request.with_lambda_context(context);
            let response = dispatch(request).await?;
            Ok(HandleResult {
                response: into_lambda_stream_response(response),
                event_type: "api_gateway_event",
            })
        }
        RuntimeEventClassification::StreamingCompletion => Ok(HandleResult {
            response: into_lambda_stream_response(empty_streaming_response()),
            event_type: "streaming_completion",
        }),
        RuntimeEventClassification::UnrecognizedEvent => Ok(HandleResult {
            response: into_lambda_stream_response(empty_streaming_response()),
            event_type: "unrecognized_lambda_payload",
        }),
    }
}

/// Map an event type string to the appropriate tracing log level.
///
/// Returns `Some(Level::WARN)` for unrecognized payloads (surfaced in
/// CloudWatch), `Some(Level::DEBUG)` for completion acks (normally silent),
/// and `None` for API Gateway events (no extra logging needed).
fn event_log_level(event_type: &str) -> Option<tracing::Level> {
    match event_type {
        "streaming_completion" => Some(tracing::Level::DEBUG),
        "unrecognized_lambda_payload" => Some(tracing::Level::WARN),
        _ => None,
    }
}

/// Run the Lambda MCP handler with streaming response support.
///
/// This replaces `lambda_http::run_with_streaming_response(service_fn(...))` and
/// gracefully handles API Gateway streaming completion invocations that would
/// otherwise cause deserialization errors in the Lambda runtime.
///
/// ## Problem
///
/// When API Gateway uses `response-streaming-invocations`, it sends a secondary
/// "completion" invocation after the streaming response finishes. This invocation
/// is NOT an API Gateway proxy event — `lambda_http` cannot deserialize it, causing
/// ERROR logs and CloudWatch Lambda Error metrics for every streaming response.
///
/// ## Solution
///
/// This function uses `lambda_runtime::run()` directly with `serde_json::Value`
/// (which always deserializes), then classifies the payload three ways via
/// [`classify_runtime_event()`]:
///
/// - **`ApiGatewayEvent`** — dispatched to the handler normally
/// - **`StreamingCompletion`** — acknowledged silently (`debug` log)
/// - **`UnrecognizedEvent`** — acknowledged with a `warn` log to surface
///   unexpected payload shapes in CloudWatch without triggering Lambda retries
///
/// ## Usage
///
/// ```rust,ignore
/// let handler = server.handler().await?;
/// turul_mcp_aws_lambda::run_streaming(handler).await
/// ```
pub async fn run_streaming(
    handler: LambdaMcpHandler,
) -> std::result::Result<(), lambda_http::Error> {
    use lambda_runtime::{LambdaEvent, service_fn};

    lambda_runtime::run(service_fn(move |event: LambdaEvent<serde_json::Value>| {
        let handler = handler.clone();
        async move {
            let result = handle_runtime_payload(event.payload, event.context, |req| {
                handler.handle_streaming(req)
            })
            .await?;

            match event_log_level(result.event_type) {
                Some(level) if level == tracing::Level::WARN => {
                    tracing::warn!(
                        event_type = result.event_type,
                        "Received unrecognized Lambda invocation payload"
                    );
                }
                Some(_) => {
                    tracing::debug!(
                        event_type = result.event_type,
                        "Acknowledging streaming completion"
                    );
                }
                None => {}
            }

            Ok::<_, lambda_http::Error>(result.response)
        }
    }))
    .await
}

/// Run a custom dispatch function with streaming response support.
///
/// Like [`run_streaming()`], but accepts a custom dispatch closure instead of
/// a [`LambdaMcpHandler`]. Use this when you need pre-dispatch logic
/// (e.g., `.well-known` routing) that runs before the MCP handler.
///
/// The dispatch closure is called once per API Gateway invocation. Streaming
/// completion invocations and unrecognized payloads are acknowledged
/// automatically without invoking the closure.
///
/// ## Usage
///
/// ```rust,ignore
/// async fn lambda_handler(request: Request) -> Result<Response, Error> {
///     // pre-dispatch logic here (e.g., .well-known short-circuit)
///     let handler = HANDLER.get_or_try_init(|| async { ... }).await?;
///     handler.handle_streaming(request).await
/// }
///
/// turul_mcp_aws_lambda::run_streaming_with(lambda_handler).await
/// ```
pub async fn run_streaming_with<F, Fut>(dispatch: F) -> std::result::Result<(), lambda_http::Error>
where
    F: Fn(lambda_http::Request) -> Fut + Clone + Send + 'static,
    Fut: std::future::Future<Output = std::result::Result<http::Response<StreamBody>, lambda_http::Error>>
        + Send,
{
    use lambda_runtime::{LambdaEvent, service_fn};

    lambda_runtime::run(service_fn(move |event: LambdaEvent<serde_json::Value>| {
        let dispatch = dispatch.clone();
        async move {
            let result =
                handle_runtime_payload(event.payload, event.context, dispatch).await?;

            match event_log_level(result.event_type) {
                Some(level) if level == tracing::Level::WARN => {
                    tracing::warn!(
                        event_type = result.event_type,
                        "Received unrecognized Lambda invocation payload"
                    );
                }
                Some(_) => {
                    tracing::debug!(
                        event_type = result.event_type,
                        "Acknowledging streaming completion"
                    );
                }
                None => {}
            }

            Ok::<_, lambda_http::Error>(result.response)
        }
    }))
    .await
}

/// Convert an HTTP response into a Lambda `StreamResponse`.
///
/// Replicates `lambda_http::streaming::into_stream_response` (which is private)
/// by extracting status/headers/cookies into `MetadataPrelude` and converting
/// the body into a `Stream`.
fn into_lambda_stream_response<B>(
    response: http::Response<B>,
) -> lambda_runtime::StreamResponse<http_body_util::BodyDataStream<B>>
where
    B: http_body::Body + Unpin + Send + 'static,
{
    let (parts, body) = response.into_parts();
    let mut headers = parts.headers;

    // Extract Set-Cookie headers into the cookies vec (Lambda streaming protocol)
    let cookies = headers
        .get_all(http::header::SET_COOKIE)
        .iter()
        .map(|c| String::from_utf8_lossy(c.as_bytes()).to_string())
        .collect::<Vec<_>>();
    headers.remove(http::header::SET_COOKIE);

    lambda_runtime::StreamResponse {
        metadata_prelude: lambda_runtime::MetadataPrelude {
            headers,
            status_code: parts.status,
            cookies,
        },
        stream: http_body_util::BodyDataStream::new(body),
    }
}

/// Build an empty 200 response for acknowledging completion invocations.
fn empty_streaming_response()
-> http::Response<http_body_util::combinators::UnsyncBoxBody<bytes::Bytes, hyper::Error>> {
    use http_body_util::{BodyExt, Full};
    let body = Full::new(bytes::Bytes::new())
        .map_err(|e: std::convert::Infallible| match e {})
        .boxed_unsync();
    http::Response::builder().status(200).body(body).unwrap()
}

#[cfg(test)]
mod streaming_completion_tests {
    use super::*;
    use serde_json::json;

    /// Load a test fixture from `src/fixtures/` via `include_str!`.
    /// Compile-time verified — missing files cause a build error.
    fn load_fixture(name: &str) -> serde_json::Value {
        let json_str = match name {
            "apigw_v1" => include_str!("fixtures/apigw_v1_proxy_event.json"),
            "apigw_v2" => include_str!("fixtures/apigw_v2_http_api_event.json"),
            "completion_success" => include_str!("fixtures/streaming_completion_success.json"),
            "completion_failure" => include_str!("fixtures/streaming_completion_failure.json"),
            "completion_extra" => include_str!("fixtures/streaming_completion_extra_fields.json"),
            "completion_api_like" => {
                include_str!("fixtures/completion_with_api_like_fields.json")
            }
            other => panic!("Unknown fixture: {other}"),
        };
        serde_json::from_str(json_str).unwrap_or_else(|e| panic!("Bad fixture {name}: {e}"))
    }

    // ── Fixture tests: API Gateway events → ApiGatewayEvent ──

    #[test]
    fn test_classify_api_gateway_v1_event() {
        let payload = load_fixture("apigw_v1");
        assert!(
            matches!(
                classify_runtime_event(payload),
                RuntimeEventClassification::ApiGatewayEvent(_)
            ),
            "API Gateway v1 proxy event must classify as ApiGatewayEvent"
        );
    }

    #[test]
    fn test_classify_api_gateway_v2_event() {
        let payload = load_fixture("apigw_v2");
        assert!(
            matches!(
                classify_runtime_event(payload),
                RuntimeEventClassification::ApiGatewayEvent(_)
            ),
            "API Gateway v2 HTTP API event must classify as ApiGatewayEvent"
        );
    }

    // ── Fixture tests: Streaming completion → StreamingCompletion ──

    #[test]
    fn test_classify_streaming_completion() {
        let payload = load_fixture("completion_success");
        assert!(matches!(
            classify_runtime_event(payload),
            RuntimeEventClassification::StreamingCompletion
        ));
    }

    #[test]
    fn test_classify_completion_failure_status() {
        let payload = load_fixture("completion_failure");
        assert!(matches!(
            classify_runtime_event(payload),
            RuntimeEventClassification::StreamingCompletion
        ));
    }

    #[test]
    fn test_classify_completion_extra_fields() {
        let payload = load_fixture("completion_extra");
        assert!(matches!(
            classify_runtime_event(payload),
            RuntimeEventClassification::StreamingCompletion
        ));
    }

    // ── R5: Precedence edge case ──

    #[test]
    fn test_classify_completion_with_api_like_fields() {
        // Intentional: prefer false-positive ack over retries.
        // A payload with invokeCompletionStatus + partial API Gateway fields
        // is classified as StreamingCompletion (not UnrecognizedEvent),
        // because invokeCompletionStatus is the discriminator.
        //
        // NOTE: This fixture is intentionally NOT a valid API Gateway event.
        // Do not "fix" it into one — that would change the expected classification.
        let payload = load_fixture("completion_api_like");
        assert!(matches!(
            classify_runtime_event(payload),
            RuntimeEventClassification::StreamingCompletion
        ));
    }

    // ── Inline tests: Unrecognized payloads → UnrecognizedEvent ──

    #[test]
    fn test_classify_empty_object() {
        assert!(matches!(
            classify_runtime_event(json!({})),
            RuntimeEventClassification::UnrecognizedEvent
        ));
    }

    #[test]
    fn test_classify_random_object() {
        assert!(matches!(
            classify_runtime_event(json!({"foo": "bar", "baz": 123})),
            RuntimeEventClassification::UnrecognizedEvent
        ));
    }

    #[test]
    fn test_classify_null_payload() {
        assert!(matches!(
            classify_runtime_event(json!(null)),
            RuntimeEventClassification::UnrecognizedEvent
        ));
    }

    #[test]
    fn test_classify_string_payload() {
        assert!(matches!(
            classify_runtime_event(json!("hello")),
            RuntimeEventClassification::UnrecognizedEvent
        ));
    }

    #[test]
    fn test_classify_array_payload() {
        assert!(matches!(
            classify_runtime_event(json!([1, 2, 3])),
            RuntimeEventClassification::UnrecognizedEvent
        ));
    }

    #[test]
    fn test_classify_nested_invoke_status() {
        // invokeCompletionStatus must be at top level to match
        let payload = json!({
            "data": {"invokeCompletionStatus": "Success"}
        });
        assert!(matches!(
            classify_runtime_event(payload),
            RuntimeEventClassification::UnrecognizedEvent
        ));
    }

    // ── Property-style tests ──

    #[test]
    fn test_classify_never_panics_on_arbitrary_json() {
        // R4: Only assert no panics — no brittle !ApiGatewayEvent assertion.
        let payloads = vec![
            json!(null),
            json!(true),
            json!(false),
            json!(42),
            json!(-1.5),
            json!(""),
            json!("some string"),
            json!([]),
            json!([1, "two", null, false]),
            json!({}),
            json!({"a": 1}),
            json!({"requestContext": null}),
            json!({"requestContext": "not-an-object"}),
            json!({"httpMethod": "POST"}),
            json!({"version": "2.0"}),
            json!({"version": "2.0", "routeKey": "GET /"}),
            json!({"resource": "/", "httpMethod": "GET"}),
            json!({"deeply": {"nested": {"invokeCompletionStatus": "Success"}}}),
            // Large payload
            serde_json::Value::Object((0..100).map(|i| (format!("key_{i}"), json!(i))).collect()),
        ];

        for payload in payloads {
            let _result = classify_runtime_event(payload);
        }
    }

    #[test]
    fn test_classify_invoke_completion_status_always_wins() {
        // Any object with top-level invokeCompletionStatus that doesn't parse as API GW
        // should be classified as StreamingCompletion
        let payloads = vec![
            json!({"invokeCompletionStatus": "Success"}),
            json!({"invokeCompletionStatus": "Failure"}),
            json!({"invokeCompletionStatus": "Unknown"}),
            json!({"invokeCompletionStatus": 42}),
            json!({"invokeCompletionStatus": null}),
            json!({"invokeCompletionStatus": "Success", "requestId": "abc-123"}),
            json!({"invokeCompletionStatus": "Success", "extra": "field", "nested": {"a": 1}}),
        ];

        for payload in payloads {
            let result = classify_runtime_event(payload.clone());
            assert!(
                matches!(result, RuntimeEventClassification::StreamingCompletion),
                "Payload with top-level invokeCompletionStatus must be StreamingCompletion: {payload}"
            );
        }
    }

    // ── event_log_level contract tests (R1) ──

    #[test]
    fn test_unrecognized_logs_at_warn_level() {
        assert_eq!(
            event_log_level("unrecognized_lambda_payload"),
            Some(tracing::Level::WARN)
        );
    }

    #[test]
    fn test_completion_logs_at_debug_level() {
        assert_eq!(
            event_log_level("streaming_completion"),
            Some(tracing::Level::DEBUG)
        );
    }

    #[test]
    fn test_api_gateway_has_no_extra_logging() {
        assert_eq!(event_log_level("api_gateway_event"), None);
    }

    // ── handle_runtime_payload action-path tests (R3) ──

    #[tokio::test]
    async fn test_handle_completion_does_not_dispatch() {
        let dispatched = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let dispatched_clone = dispatched.clone();

        let result = handle_runtime_payload(
            load_fixture("completion_success"),
            lambda_runtime::Context::default(),
            |_req| {
                let d = dispatched_clone.clone();
                async move {
                    d.store(true, std::sync::atomic::Ordering::SeqCst);
                    Ok(empty_streaming_response())
                }
            },
        )
        .await
        .expect("handle should succeed");

        assert!(
            !dispatched.load(std::sync::atomic::Ordering::SeqCst),
            "Completion events must not dispatch to handler"
        );
        assert_eq!(result.event_type, "streaming_completion");
        assert_eq!(result.response.metadata_prelude.status_code, 200);
    }

    #[tokio::test]
    async fn test_handle_unrecognized_does_not_dispatch() {
        let dispatched = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let dispatched_clone = dispatched.clone();

        let result = handle_runtime_payload(
            json!({"foo": "bar"}),
            lambda_runtime::Context::default(),
            |_req| {
                let d = dispatched_clone.clone();
                async move {
                    d.store(true, std::sync::atomic::Ordering::SeqCst);
                    Ok(empty_streaming_response())
                }
            },
        )
        .await
        .expect("handle should succeed");

        assert!(
            !dispatched.load(std::sync::atomic::Ordering::SeqCst),
            "Unrecognized events must not dispatch to handler"
        );
        assert_eq!(result.event_type, "unrecognized_lambda_payload");
        assert_eq!(result.response.metadata_prelude.status_code, 200);
    }

    #[tokio::test]
    async fn test_handle_apigw_v1_dispatches() {
        let dispatched = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let dispatched_clone = dispatched.clone();

        let result = handle_runtime_payload(
            load_fixture("apigw_v1"),
            lambda_runtime::Context::default(),
            |_req| {
                let d = dispatched_clone.clone();
                async move {
                    d.store(true, std::sync::atomic::Ordering::SeqCst);
                    Ok(empty_streaming_response())
                }
            },
        )
        .await
        .expect("handle should succeed");

        assert!(
            dispatched.load(std::sync::atomic::Ordering::SeqCst),
            "API Gateway v1 events must dispatch to handler"
        );
        assert_eq!(result.event_type, "api_gateway_event");
    }

    #[tokio::test]
    async fn test_handle_apigw_v2_dispatches() {
        let dispatched = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let dispatched_clone = dispatched.clone();

        let result = handle_runtime_payload(
            load_fixture("apigw_v2"),
            lambda_runtime::Context::default(),
            |_req| {
                let d = dispatched_clone.clone();
                async move {
                    d.store(true, std::sync::atomic::Ordering::SeqCst);
                    Ok(empty_streaming_response())
                }
            },
        )
        .await
        .expect("handle should succeed");

        assert!(
            dispatched.load(std::sync::atomic::Ordering::SeqCst),
            "API Gateway v2 events must dispatch to handler"
        );
        assert_eq!(result.event_type, "api_gateway_event");
    }

    #[tokio::test]
    async fn test_handle_unrecognized_surfaces_distinct_event_type() {
        let result = handle_runtime_payload(
            json!({"unknown": true}),
            lambda_runtime::Context::default(),
            |_req| async { Ok(empty_streaming_response()) },
        )
        .await
        .expect("handle should succeed");

        assert_eq!(result.event_type, "unrecognized_lambda_payload");
    }

    // ── Existing response conversion tests ──

    #[test]
    fn test_empty_streaming_response() {
        let resp = empty_streaming_response();
        assert_eq!(resp.status(), 200);
    }

    #[test]
    fn test_into_lambda_stream_response_preserves_metadata() {
        use http_body_util::{BodyExt, Full};

        let response = http::Response::builder()
            .status(401)
            .header("WWW-Authenticate", "Bearer realm=\"mcp\"")
            .header("X-Custom", "test")
            .body(
                Full::new(bytes::Bytes::from("Unauthorized"))
                    .map_err(|e: std::convert::Infallible| match e {})
                    .boxed_unsync(),
            )
            .unwrap();

        let stream_resp = into_lambda_stream_response(response);
        assert_eq!(stream_resp.metadata_prelude.status_code, 401);
        assert_eq!(
            stream_resp
                .metadata_prelude
                .headers
                .get("WWW-Authenticate")
                .unwrap(),
            "Bearer realm=\"mcp\""
        );
        assert_eq!(
            stream_resp
                .metadata_prelude
                .headers
                .get("X-Custom")
                .unwrap(),
            "test"
        );
    }

    #[test]
    fn test_into_lambda_stream_response_extracts_cookies() {
        use http_body_util::{BodyExt, Full};

        let response = http::Response::builder()
            .status(200)
            .header("Set-Cookie", "session=abc; Path=/")
            .header("Set-Cookie", "theme=dark")
            .body(
                Full::new(bytes::Bytes::new())
                    .map_err(|e: std::convert::Infallible| match e {})
                    .boxed_unsync(),
            )
            .unwrap();

        let stream_resp = into_lambda_stream_response(response);
        assert_eq!(stream_resp.metadata_prelude.cookies.len(), 2);
        assert!(
            stream_resp
                .metadata_prelude
                .cookies
                .contains(&"session=abc; Path=/".to_string())
        );
        assert!(
            stream_resp
                .metadata_prelude
                .cookies
                .contains(&"theme=dark".to_string())
        );
        assert!(
            stream_resp
                .metadata_prelude
                .headers
                .get("Set-Cookie")
                .is_none()
        );
    }

    // ── run_streaming_with dispatch tests ──

    #[tokio::test]
    async fn test_run_streaming_with_dispatches_apigw_events() {
        let dispatched = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let dispatched_clone = dispatched.clone();

        let dispatch = move |_req: lambda_http::Request| {
            let d = dispatched_clone.clone();
            async move {
                d.store(true, std::sync::atomic::Ordering::SeqCst);
                Ok(empty_streaming_response())
            }
        };

        let result = handle_runtime_payload(
            load_fixture("apigw_v1"),
            lambda_runtime::Context::default(),
            dispatch,
        )
        .await
        .expect("handle should succeed");

        assert!(
            dispatched.load(std::sync::atomic::Ordering::SeqCst),
            "run_streaming_with dispatch must be called for API Gateway events"
        );
        assert_eq!(result.event_type, "api_gateway_event");
    }

    #[tokio::test]
    async fn test_run_streaming_with_acks_completion_without_dispatch() {
        let dispatched = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let dispatched_clone = dispatched.clone();

        let dispatch = move |_req: lambda_http::Request| {
            let d = dispatched_clone.clone();
            async move {
                d.store(true, std::sync::atomic::Ordering::SeqCst);
                Ok(empty_streaming_response())
            }
        };

        let result = handle_runtime_payload(
            load_fixture("completion_success"),
            lambda_runtime::Context::default(),
            dispatch,
        )
        .await
        .expect("handle should succeed");

        assert!(
            !dispatched.load(std::sync::atomic::Ordering::SeqCst),
            "run_streaming_with dispatch must NOT be called for completion events"
        );
        assert_eq!(result.event_type, "streaming_completion");
        assert_eq!(result.response.metadata_prelude.status_code, 200);
    }
}
