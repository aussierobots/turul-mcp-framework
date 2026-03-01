// turul-mcp-server v0.3
// Rate limiting middleware: per-session counters with configurable window

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use turul_http_mcp_server::middleware::*;
use turul_mcp_session_storage::SessionView;

struct RateLimitMiddleware {
    max_requests: u64,
    window_seconds: u64,
    // Mutex is fine here — no .await while held
    counters: Mutex<HashMap<String, (u64, Instant)>>,
}

impl RateLimitMiddleware {
    fn new(max_requests: u64, window_seconds: u64) -> Self {
        Self {
            max_requests,
            window_seconds,
            counters: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl McpMiddleware for RateLimitMiddleware {
    async fn before_dispatch(
        &self,
        ctx: &mut RequestContext<'_>,
        session: Option<&dyn SessionView>,
        _injection: &mut SessionInjection,
    ) -> Result<(), MiddlewareError> {
        // Don't rate-limit initialize (session doesn't exist yet)
        if ctx.method() == "initialize" {
            return Ok(());
        }

        let session_id = session
            .and_then(|s| s.session_id())
            .unwrap_or("anonymous")
            .to_string();

        let mut counters = self.counters.lock().unwrap();
        let now = Instant::now();

        let (count, window_start) = counters.entry(session_id).or_insert((0, now));

        // Reset window if expired
        if now.duration_since(*window_start).as_secs() >= self.window_seconds {
            *count = 0;
            *window_start = now;
        }

        *count += 1;

        if *count > self.max_requests {
            return Err(MiddlewareError::rate_limit(
                format!(
                    "Rate limit exceeded: {} requests per {} seconds",
                    self.max_requests, self.window_seconds
                ),
                Some(self.window_seconds),
            ));
        }

        Ok(())
    }
}

// Registration:
// let server = McpServer::builder()
//     .middleware(Arc::new(RateLimitMiddleware::new(100, 60)))  // 100 req/min
//     .build()?;
