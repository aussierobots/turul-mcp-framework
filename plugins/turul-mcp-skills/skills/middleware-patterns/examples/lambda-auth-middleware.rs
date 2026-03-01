// turul-mcp-server v0.3
// Lambda auth middleware: extract identity from API Gateway authorizer headers

use async_trait::async_trait;
use serde_json::json;
use turul_http_mcp_server::middleware::*;
use turul_mcp_session_storage::SessionView;

struct LambdaAuthMiddleware;

#[async_trait]
impl McpMiddleware for LambdaAuthMiddleware {
    async fn before_dispatch(
        &self,
        ctx: &mut RequestContext<'_>,
        _session: Option<&dyn SessionView>,
        injection: &mut SessionInjection,
    ) -> Result<(), MiddlewareError> {
        // Skip auth for initialize
        if ctx.method() == "initialize" {
            return Ok(());
        }

        // API Gateway Lambda authorizer populates x-authorizer-* headers.
        // These are pre-validated by the authorizer — middleware just extracts them.
        let principal_id = ctx
            .metadata()
            .get("x-authorizer-principalid")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                MiddlewareError::unauthenticated(
                    "Missing authorizer principal — is the API Gateway authorizer configured?",
                )
            })?;

        // Optional: extract additional authorizer context
        let scope = ctx
            .metadata()
            .get("x-authorizer-scope")
            .and_then(|v| v.as_str())
            .unwrap_or("default");

        // Inject into session for tool access
        injection.set_state("user_id", json!(principal_id));
        injection.set_state("scope", json!(scope));
        injection.set_metadata("auth_source", json!("api-gateway-authorizer"));

        Ok(())
    }
}

// Registration with LambdaMcpServerBuilder:
// let server = LambdaMcpServerBuilder::new()
//     .name("lambda-auth-server")
//     .middleware(Arc::new(LambdaAuthMiddleware))
//     .build()?;
