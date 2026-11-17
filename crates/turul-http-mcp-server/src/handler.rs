//! SSE stream body utilities for the MCP protocol.
//!
//! Contains `SseStreamBody`, a hyper body over a stream of SSE-formatted strings.

use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures::Stream;
use http_body::Body;

/// Wraps a stream of SSE-formatted strings as a hyper `Body`.
pub struct SseStreamBody {
    stream: Pin<
        Box<
            dyn Stream<Item = std::result::Result<String, tokio::sync::broadcast::error::RecvError>>
                + Send,
        >,
    >,
}

impl SseStreamBody {
    /// Wraps a stream of SSE-formatted strings as an HTTP response body; a stream error (closed or lagged broadcast) is surfaced as a body error.
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
