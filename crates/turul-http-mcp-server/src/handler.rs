//! SSE stream body utilities for MCP protocol
//!
//! McpHttpHandler has been removed in 0.2.0 - use SessionMcpHandler instead.
//! This module now only contains the SseStreamBody utility for compatibility.

use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures::Stream;
use http_body::Body;

/// SSE stream body that implements hyper's Body trait
pub struct SseStreamBody {
    stream: Pin<
        Box<
            dyn Stream<Item = std::result::Result<String, tokio::sync::broadcast::error::RecvError>>
                + Send,
        >,
    >,
}

impl SseStreamBody {
    pub fn new<S>(stream: S) -> Self
    where
        S: Stream<Item = std::result::Result<String, tokio::sync::broadcast::error::RecvError>>
            + Send
            + 'static,
    {
        Self {
            stream: Box::pin(stream),
        }
    }
}

impl Body for SseStreamBody {
    type Data = Bytes;
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<std::result::Result<http_body::Frame<Self::Data>, Self::Error>>> {
        match self.stream.as_mut().poll_next(cx) {
            Poll::Ready(Some(Ok(data))) => {
                let bytes = Bytes::from(data);
                Poll::Ready(Some(Ok(http_body::Frame::data(bytes))))
            }
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(Box::new(e)))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

// McpHttpHandler removed in 0.2.0 - use SessionMcpHandler instead
// All SSE functionality is now provided by SessionMcpHandler with proper streaming support

// McpHttpHandler implementation removed - use SessionMcpHandler instead

// Tests for McpHttpHandler removed - handler no longer exists
// Use SessionMcpHandler tests in session_handler.rs instead
