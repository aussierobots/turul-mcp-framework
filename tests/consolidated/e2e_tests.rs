//! Consolidated end-to-end test suite.
//!
//! Groups: streamable HTTP, SSE notification roundtrip, SSE progress,
//! client drop/streaming, basic session

#[path = "../streamable_http_e2e.rs"]
mod streamable_http_e2e;

#[path = "../streamable_http_client_test.rs"]
mod streamable_http_client_test;

#[path = "../e2e_sse_notification_roundtrip.rs"]
mod e2e_sse_notification_roundtrip;

#[path = "../sse_progress_delivery.rs"]
mod sse_progress_delivery;

#[path = "../client_drop_test.rs"]
mod client_drop_test;

#[path = "../client_streaming_test.rs"]
mod client_streaming_test;

#[path = "../basic_session_test.rs"]
mod basic_session_test;
