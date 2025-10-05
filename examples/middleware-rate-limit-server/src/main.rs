//! Middleware Rate Limiting Example
//!
//! Demonstrates rate limiting middleware that:
//! 1. Tracks request counts per session
//! 2. Returns MiddlewareError::RateLimitExceeded when limit hit
//! 3. Maps to -32003 JSON-RPC error with retryAfter data

use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use turul_mcp_server::prelude::*;

/// Rate limiting middleware with per-session counters
struct RateLimitMiddleware {
    max_requests: u32,
    window_secs: u64,
    /// Session ID -> request count
    counters: Mutex<HashMap<String, u32>>,
}

impl RateLimitMiddleware {
    fn new(max_requests: u32, window_secs: u64) -> Self {
        Self {
            max_requests,
            window_secs,
            counters: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl McpMiddleware for RateLimitMiddleware {
    async fn before_dispatch(
        &self,
        ctx: &mut RequestContext<'_>,
        session: Option<&dyn turul_mcp_session_storage::SessionView>,
        injection: &mut SessionInjection,
    ) -> Result<(), MiddlewareError> {
        // Skip rate limiting for initialize (needed for session creation)
        if ctx.method() == "initialize" {
            return Ok(());
        }

        if let Some(session_view) = session {
            let session_id = session_view.session_id().to_string();
            let mut counters = self.counters.lock().unwrap();
            let count = counters.entry(session_id.clone()).or_insert(0);

            if *count >= self.max_requests {
                tracing::warn!(
                    "Rate limit exceeded for session {}: {} >= {}",
                    session_id,
                    count,
                    self.max_requests
                );

                return Err(MiddlewareError::RateLimitExceeded {
                    message: format!(
                        "Rate limit exceeded: {} requests per {} seconds",
                        self.max_requests, self.window_secs
                    ),
                    retry_after: Some(self.window_secs),
                });
            }

            *count += 1;
            injection.set_metadata("request_count", json!(count));

            tracing::info!(
                "Session {} request count: {}/{}",
                session_id,
                count,
                self.max_requests
            );
        }

        Ok(())
    }

    async fn after_dispatch(
        &self,
        _ctx: &RequestContext<'_>,
        _result: &mut DispatcherResult,
    ) -> Result<(), MiddlewareError> {
        Ok(())
    }
}

#[tokio::main]
async fn main() -> McpResult<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("middleware_rate_limit_server=info,turul_mcp_server=info")
        .init();

    tracing::info!("Starting middleware-rate-limit-server example");
    tracing::info!("Rate limit: 5 requests per session");
    tracing::info!("After 5 requests, you'll receive error -32003 (RateLimitExceeded)");

    let server = McpServer::builder()
        .name("middleware-rate-limit-server")
        .version("1.0.0")
        .title("Rate Limiting Middleware Example")
        .instructions("Demonstrates rate limiting. Max 5 requests per session before hitting rate limit.")
        // Register rate limiting middleware (5 requests per session)
        .middleware(Arc::new(RateLimitMiddleware::new(5, 60)))
        .build()?;

    tracing::info!("Server listening on http://localhost:8080/mcp");
    tracing::info!("Try sending multiple requests to see rate limiting in action");

    server.run().await
}
