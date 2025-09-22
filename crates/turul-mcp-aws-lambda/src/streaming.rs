//! SSE (Server-Sent Events) streaming adaptation for AWS Lambda
//!
//! This module handles the conversion of the framework's SSE streams to
//! Lambda's streaming response format, enabling real-time notifications
//! in serverless environments.

use bytes::Bytes;
use futures::{Stream, StreamExt};
use lambda_http::Body as LambdaBody;
use tracing::debug;

use crate::error::{LambdaError, Result};

/// Adapt a framework SSE stream to Lambda's streaming body format
///
/// This function converts the framework's UnifiedMcpBody SSE stream into
/// a format that Lambda's streaming response can handle properly.
pub async fn adapt_sse_stream(
    body: http_body_util::combinators::UnsyncBoxBody<bytes::Bytes, hyper::Error>,
) -> Result<LambdaBody> {
    debug!("Adapting body for Lambda streaming response");

    // Convert the boxed body to a byte stream
    use futures::TryStreamExt;
    use http_body_util::BodyExt;

    let byte_stream = body.into_data_stream().map_err(|e| {
        debug!("Body stream error: {}", e);
        std::io::Error::other(e.to_string())
    });

    // Create Lambda streaming body by collecting all bytes
    // Note: Lambda doesn't support true streaming like hyper, so we collect everything
    let mut all_bytes = Vec::new();

    let mut stream = std::pin::pin!(byte_stream);
    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result
            .map_err(|e| LambdaError::Sse(format!("Failed to read stream chunk: {}", e)))?;
        all_bytes.extend_from_slice(&chunk);
    }

    let lambda_body = if all_bytes.is_empty() {
        LambdaBody::Empty
    } else {
        match String::from_utf8(all_bytes.clone()) {
            Ok(text) => LambdaBody::Text(text),
            Err(_) => LambdaBody::Binary(all_bytes),
        }
    };

    debug!("Successfully adapted body to Lambda streaming body");
    Ok(lambda_body)
}

/// Create an SSE event string from structured data
///
/// This helper function formats data as proper SSE events with optional
/// event type and ID fields.
pub fn format_sse_event(data: &str, event_type: Option<&str>, event_id: Option<&str>) -> String {
    let mut event = String::new();

    if let Some(id) = event_id {
        event.push_str(&format!("id: {}\n", id));
    }

    if let Some(event_type) = event_type {
        event.push_str(&format!("event: {}\n", event_type));
    }

    // Split data on newlines and prefix each with "data: "
    for line in data.lines() {
        event.push_str(&format!("data: {}\n", line));
    }

    event.push('\n'); // End with blank line
    event
}

/// Create a stream of SSE events from a vector of data
///
/// Utility function for testing and simple use cases where you have
/// a known set of events to stream.
pub fn create_sse_stream<T>(
    events: Vec<T>,
    formatter: impl Fn(&T) -> String + Send + 'static,
) -> impl Stream<Item = Result<Bytes>> + Send + 'static
where
    T: Send + 'static,
{
    async_stream::stream! {
        for event in events {
            let sse_data = formatter(&event);
            let bytes = Bytes::from(sse_data);
            yield Ok(bytes);

            // Small delay to simulate real-time events
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    }
}

/// Stream heartbeat events to keep SSE connections alive
///
/// Lambda has timeout limits, so sending periodic heartbeat events
/// can help maintain long-running SSE connections.
pub fn create_heartbeat_stream(
    interval_secs: u64,
) -> impl Stream<Item = Result<Bytes>> + Send + 'static {
    async_stream::stream! {
        let mut interval = tokio::time::interval(
            tokio::time::Duration::from_secs(interval_secs)
        );

        loop {
            interval.tick().await;

            let heartbeat = format_sse_event(
                "heartbeat",
                Some("heartbeat"),
                Some(&chrono::Utc::now().timestamp().to_string())
            );

            yield Ok(Bytes::from(heartbeat));
        }
    }
}

/// Merge multiple SSE streams into a single stream
///
/// This is useful when you want to combine different types of events
/// (like notifications and heartbeats) into a single SSE stream.
pub fn merge_sse_streams<S1, S2>(
    stream1: S1,
    stream2: S2,
) -> impl Stream<Item = Result<Bytes>> + Send + 'static
where
    S1: Stream<Item = Result<Bytes>> + Send + 'static,
    S2: Stream<Item = Result<Bytes>> + Send + 'static,
{
    use futures::stream::select;

    select(stream1.map(|item| (1, item)), stream2.map(|item| (2, item))).map(|(_, result)| result)
}

/// Validate SSE event format
///
/// Ensures that SSE events conform to the standard format and don't
/// contain invalid characters that could break the stream.
pub fn validate_sse_event(event: &str) -> Result<()> {
    // Check for null bytes which can break SSE
    if event.contains('\0') {
        return Err(LambdaError::Sse(
            "SSE events cannot contain null bytes".to_string(),
        ));
    }

    // Warn about very large events (browsers may have limits)
    if event.len() > 1_048_576 {
        // 1MB
        debug!("Warning: SSE event is very large ({} bytes)", event.len());
    }

    // Check for proper line endings
    if event.contains('\r') && !event.contains("\r\n") {
        return Err(LambdaError::Sse(
            "SSE events should use LF or CRLF line endings, not standalone CR".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;

    #[test]
    fn test_format_sse_event() {
        let event = format_sse_event("Hello, World!", Some("message"), Some("123"));

        assert!(event.contains("id: 123\n"));
        assert!(event.contains("event: message\n"));
        assert!(event.contains("data: Hello, World!\n"));
        assert!(event.ends_with("\n\n"));
    }

    #[test]
    fn test_format_multiline_event() {
        let data = "Line 1\nLine 2\nLine 3";
        let event = format_sse_event(data, None, None);

        assert!(event.contains("data: Line 1\n"));
        assert!(event.contains("data: Line 2\n"));
        assert!(event.contains("data: Line 3\n"));
    }

    #[tokio::test]
    async fn test_create_sse_stream() {
        use futures::StreamExt;
        use futures::pin_mut;

        let events = vec!["event1", "event2", "event3"];
        let stream = create_sse_stream(events, |s| format_sse_event(s, Some("test"), None));
        pin_mut!(stream);

        let first_event = stream.next().await.unwrap().unwrap();
        let event_str = String::from_utf8(first_event.to_vec()).unwrap();

        assert!(event_str.contains("event: test\n"));
        assert!(event_str.contains("data: event1\n"));
    }

    #[test]
    fn test_validate_sse_event() {
        assert!(validate_sse_event("Normal event").is_ok());
        assert!(validate_sse_event("Event\nwith\nnewlines").is_ok());
        assert!(validate_sse_event("Event with\0null byte").is_err());
        assert!(validate_sse_event("Event with\rstandalone CR").is_err());
        assert!(validate_sse_event("Event with\r\nCRLF").is_ok());
    }

    #[tokio::test]
    async fn test_merge_streams() {
        let stream1 = stream::iter(vec![
            Ok(Bytes::from("stream1-1")),
            Ok(Bytes::from("stream1-2")),
        ]);

        let stream2 = stream::iter(vec![
            Ok(Bytes::from("stream2-1")),
            Ok(Bytes::from("stream2-2")),
        ]);

        let merged = merge_sse_streams(stream1, stream2);
        let results: Vec<_> = merged.collect().await;

        assert_eq!(results.len(), 4);
        // Note: Order may vary due to select() behavior
    }
}
