//! # MRTR Elicitation Server (2026-07-28)
//!
//! On the 2026-07-28 stateless core, servers never push requests to clients —
//! a tool that needs user input returns an `InputRequiredResult`
//! (`resultType: "input_required"`) and the client retries the ORIGINAL
//! request carrying `inputResponses` plus the echoed opaque `requestState`.
//! This Multi-Round-Trip-Request (MRTR) pattern replaces 2025's
//! server-initiated `elicitation/create` over a session stream.
//!
//! The `deploy_service` tool here asks for confirmation before "deploying":
//! the first call answers input-required with an elicitation form; the retry
//! completes (or aborts) based on the user's answer.
//!
//! Capability gate: clients that do not declare `elicitation` in their
//! per-request `_meta` `clientCapabilities` get JSON-RPC `-32003` instead of
//! an input request — servers MUST NOT demand inputs a client can't answer.
//!
//! Pair with the client leg:
//!
//! ```bash
//! cargo run -p mrtr-elicitation-server
//! cargo run -p mrtr-elicitation-server --bin mrtr-elicitation-client
//! ```

use turul_mcp_derive::McpTool;
use turul_mcp_protocol::elicitation::{
    ElicitRequest, ElicitationSchema, PrimitiveSchemaDefinition,
};
use turul_mcp_protocol::input_required::{InputRequest, InputRequests, InputResponse};
use turul_mcp_server::prelude::*;

/// Pretend deploy that demands an elicited confirmation first.
#[derive(McpTool, Clone, Default)]
#[tool(
    name = "deploy_service",
    description = "Deploy a service — asks for confirmation via MRTR elicitation",
    output = String
)]
struct DeployServiceTool {
    #[param(description = "Service name to deploy")]
    service: String,
}

impl DeployServiceTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
        let session = session.ok_or_else(|| McpError::tool_execution("request context missing"))?;

        // Retry leg: the dispatcher surfaces the retry's inputResponses.
        if let Some(responses) = session.input_responses() {
            // The opaque state we handed out on the first leg comes back verbatim.
            let state = session.mrtr_request_state().unwrap_or_default();
            if state != format!("deploy:{}", self.service) {
                return Err(McpError::InvalidParameters(format!(
                    "requestState mismatch: got {state:?}"
                )));
            }

            let confirmed = responses
                .get("confirm")
                .and_then(|r| match r {
                    InputResponse::Elicit(e) => e
                        .content
                        .as_ref()
                        .and_then(|c| c.get("proceed"))
                        .and_then(|v| v.as_bool()),
                    _ => None,
                })
                .ok_or_else(|| {
                    McpError::InvalidParameters("confirm elicit response missing".into())
                })?;

            return if confirmed {
                Ok(format!("deployed {} ✅", self.service))
            } else {
                Ok(format!("deploy of {} aborted by user", self.service))
            };
        }

        // First leg: demand a confirmation form via MRTR.
        let schema = ElicitationSchema::new()
            .with_property("proceed".to_string(), PrimitiveSchemaDefinition::boolean());
        let mut requests = InputRequests::new();
        requests.insert(
            "confirm".to_string(),
            InputRequest::Elicit(ElicitRequest::new_form(
                format!("Really deploy {}?", self.service),
                schema,
            )),
        );
        Err(McpError::InputRequired {
            input_requests: Some(requests),
            request_state: Some(format!("deploy:{}", self.service)),
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let server = McpServer::builder()
        .name("mrtr-elicitation-server")
        .version("0.4.0")
        .title("MRTR Elicitation Example")
        .instructions(
            "deploy_service asks for confirmation via MRTR: the first call \
             returns input_required, the retry carries inputResponses.",
        )
        .tool(DeployServiceTool::default())
        .bind_address("127.0.0.1:8642".parse()?)
        .build()?;

    tracing::info!("MRTR elicitation server running at http://127.0.0.1:8642/mcp");
    tracing::info!("Walk the round trip with the paired client:");
    tracing::info!("  cargo run -p mrtr-elicitation-server --bin mrtr-elicitation-client");

    server.run().await?;
    Ok(())
}
