// turul-mcp-server v0.3
// Logging/timing middleware: request duration tracking with tracing

use async_trait::async_trait;
use std::sync::Arc;
use turul_http_mcp_server::middleware::*;
use turul_mcp_session_storage::SessionView;

struct LoggingMiddleware;

#[async_trait]
impl McpMiddleware for LoggingMiddleware {
    async fn before_dispatch(
        &self,
        ctx: &mut RequestContext<'_>,
        session: Option<&dyn SessionView>,
        _injection: &mut SessionInjection,
    ) -> Result<(), MiddlewareError> {
        let session_id = session
            .and_then(|s| s.session_id())
            .unwrap_or("none");

        tracing::info!(
            method = %ctx.method(),
            session_id = %session_id,
            "MCP request started"
        );
        Ok(())
    }

    async fn after_dispatch(
        &self,
        ctx: &RequestContext<'_>,
        result: &mut DispatcherResult,
    ) -> Result<(), MiddlewareError> {
        match result {
            DispatcherResult::Success(_) => {
                tracing::info!(method = %ctx.method(), "MCP request succeeded");
            }
            DispatcherResult::Error(ref msg) => {
                tracing::warn!(method = %ctx.method(), error = %msg, "MCP request failed");
            }
        }
        Ok(())
    }
}

// Registration — logging should be first (outermost):
// let server = McpServer::builder()
//     .middleware(Arc::new(LoggingMiddleware))     // 1st before, last after
//     .middleware(Arc::new(AuthMiddleware))        // 2nd before, 2nd-last after
//     .middleware(Arc::new(RateLimitMiddleware))   // 3rd before, 1st after
//     .build()?;
