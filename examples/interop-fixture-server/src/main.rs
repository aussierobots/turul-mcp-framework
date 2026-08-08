//! # Interop Fixture Server
//!
//! One fixed MCP 2026-07-28 surface for every cross-implementation probe —
//! FastMCP (Python), the MCP TypeScript SDK, the MCP Go SDK, and turul's own
//! client — so a difference between two runs is a difference between the
//! clients, not between the servers they happened to be pointed at.
//!
//! It exists because `minimal-server` exposes only a tool: the read surface
//! (resources, prompts, completion) cannot be exercised against it, which
//! capped interop coverage at 3 of 22 methods.
//!
//! Names and values here are part of the contract the probe scripts assert
//! against. Changing one means changing `scripts/interop-*.sh` in the same
//! slice.
//!
//!   cargo run -p interop-fixture-server -- --port 8700

use std::collections::HashMap;
use std::sync::OnceLock;

use turul_mcp_derive::{McpTool, mcp_tool};
use turul_mcp_protocol::completion::{
    CompleteArgument, CompleteRequest, CompleteResult, CompletionReference, CompletionResult,
    PromptReference,
};
use turul_mcp_protocol::elicitation::{
    ElicitRequest, ElicitationSchema, PrimitiveSchemaDefinition,
};
use turul_mcp_protocol::input_required::{InputRequest, InputRequests, InputResponse};
use turul_mcp_protocol::prompts::{PromptArgument, PromptMessage};
use turul_mcp_protocol::resources::ResourceContent;
use turul_mcp_server::prelude::*;
use turul_mcp_server::{McpCompletion, McpPrompt, McpResource};

const FIXTURE_URI: &str = "file:///fixture/readme.md";
const FIXTURE_TEXT: &str = "# Interop fixture\n\nStable text for cross-implementation probes.\n";
const FIXTURE_MIME: &str = "text/markdown";

#[mcp_tool(name = "echo", description = "Echo back the input text")]
async fn echo(#[param(description = "Text to echo back")] text: String) -> McpResult<String> {
    Ok(format!("Echo: {text}"))
}

/// Numeric arguments, so a peer's schema handling is exercised beyond strings.
#[mcp_tool(name = "add", description = "Add two numbers")]
async fn add(
    #[param(description = "First addend")] a: f64,
    #[param(description = "Second addend")] b: f64,
) -> McpResult<f64> {
    Ok(a + b)
}

/// The confirmation form the MRTR tool requests on leg 1 and validates on
/// leg 2. Re-derived each leg — nothing persists between them on the
/// stateless lane, which is the property J3 exists to prove.
fn confirm_schema() -> ElicitationSchema {
    let mut schema = ElicitationSchema::new()
        .with_property("proceed".to_string(), PrimitiveSchemaDefinition::boolean());
    schema.required = Some(vec!["proceed".to_string()]);
    schema
}

/// J3 fixture: the MRTR round trip (SEP-2322).
///
/// Leg 1 answers `resultType: "input_required"` with one elicit request and an
/// opaque `requestState`. The peer retries the ORIGINAL `tools/call` carrying
/// `inputResponses` plus that state verbatim, and leg 2 completes.
///
/// The probes assert there is no server-initiated `elicitation/create` and no
/// `notifications/elicitation/complete` anywhere in the capture — on 2026-07-28
/// the server never pushes a request to the client.
#[derive(McpTool, Default, Clone)]
#[tool(
    name = "confirm",
    description = "Ask for confirmation via MRTR, then report the answer",
    output = String
)]
struct ConfirmTool {
    #[param(description = "What is being confirmed")]
    subject: String,
}

impl ConfirmTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
        let session = session.ok_or_else(|| McpError::tool_execution("request context missing"))?;

        if let Some(responses) = session.input_responses() {
            // Leg 2. The opaque state must come back exactly as handed out.
            let state = session.mrtr_request_state().unwrap_or_default();
            let expected = format!("confirm:{}", self.subject);
            if state != expected {
                return Err(McpError::InvalidParameters(format!(
                    "requestState mismatch: got {state:?}, expected {expected:?}"
                )));
            }

            let elicit = responses
                .get("proceed")
                .and_then(|r| match r {
                    InputResponse::Elicit(e) => e.content.clone(),
                    _ => None,
                })
                .ok_or_else(|| McpError::InvalidParameters("proceed response missing".into()))?;

            let content = serde_json::to_value(&elicit)
                .map_err(|e| McpError::tool_execution(&e.to_string()))?;
            turul_mcp_builders::validate_elicit_content(&confirm_schema(), &content)
                .map_err(McpError::InvalidParameters)?;

            let proceed = content
                .get("proceed")
                .and_then(|v| v.as_bool())
                .ok_or_else(|| McpError::InvalidParameters("proceed must be a boolean".into()))?;

            return Ok(if proceed {
                format!("confirmed: {}", self.subject)
            } else {
                format!("declined: {}", self.subject)
            });
        }

        // Leg 1.
        let mut requests = InputRequests::new();
        requests.insert(
            "proceed".to_string(),
            InputRequest::Elicit(ElicitRequest::new_form(
                format!("Confirm {}?", self.subject),
                confirm_schema(),
            )),
        );
        Err(McpError::InputRequired {
            input_requests: Some(requests),
            request_state: Some(format!("confirm:{}", self.subject)),
        })
    }
}

/// J4 fixture: request-scoped `notifications/progress`.
///
/// Emits `steps` progress notifications carrying the client's own
/// `_meta.progressToken`, then returns. A peer that supplied no token gets no
/// notifications and a plain JSON answer — which is the ADR-006 content
/// negotiation rule, and is itself worth asserting.
#[derive(McpTool, Default, Clone)]
#[tool(
    name = "count",
    description = "Emit N progress notifications, then return the count",
    output = String
)]
struct CountTool {
    #[param(description = "How many progress notifications to emit (1-10)")]
    steps: f64,
}

impl CountTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
        let session = session.ok_or_else(|| McpError::tool_execution("request context missing"))?;
        let steps = self.steps.clamp(1.0, 10.0) as u32;

        let mut emitted = 0u32;
        for i in 1..=steps {
            // Returns false when the request carried no progressToken. Counting
            // the trues means the tool reports what it actually sent, not what
            // it attempted.
            if session
                .notify_request_progress_with_message(
                    f64::from(i),
                    Some(f64::from(steps)),
                    format!("step {i} of {steps}"),
                )
                .await
            {
                emitted += 1;
            }
        }
        Ok(format!("counted {steps}, emitted {emitted}"))
    }
}

struct FixtureResource;

impl HasResourceMetadata for FixtureResource {
    fn name(&self) -> &str {
        "Interop fixture document"
    }
}
impl HasResourceUri for FixtureResource {
    fn uri(&self) -> &str {
        FIXTURE_URI
    }
}
impl HasResourceDescription for FixtureResource {
    fn description(&self) -> Option<&str> {
        Some("A small text resource with stable contents")
    }
}
impl HasResourceMimeType for FixtureResource {
    fn mime_type(&self) -> Option<&str> {
        Some(FIXTURE_MIME)
    }
}
impl HasResourceSize for FixtureResource {}
impl HasResourceAnnotations for FixtureResource {}
impl HasResourceMeta for FixtureResource {}
impl HasIcons for FixtureResource {}

#[async_trait::async_trait]
impl McpResource for FixtureResource {
    async fn read(
        &self,
        _params: Option<serde_json::Value>,
        _session: Option<&SessionContext>,
    ) -> McpResult<Vec<ResourceContent>> {
        Ok(vec![
            ResourceContent::text(FIXTURE_URI, FIXTURE_TEXT).with_mime_type(FIXTURE_MIME),
        ])
    }
}

struct GreetingPrompt;

impl HasPromptMetadata for GreetingPrompt {
    fn name(&self) -> &str {
        "greeting"
    }
}
impl HasPromptDescription for GreetingPrompt {
    fn description(&self) -> Option<&str> {
        Some("Greet someone by name")
    }
}
impl HasPromptArguments for GreetingPrompt {
    fn arguments(&self) -> Option<&Vec<PromptArgument>> {
        static ARGS: OnceLock<Vec<PromptArgument>> = OnceLock::new();
        Some(
            ARGS.get_or_init(|| vec![PromptArgument::new("name").with_description("Who to greet")]),
        )
    }
}
impl HasPromptAnnotations for GreetingPrompt {}
impl HasPromptMeta for GreetingPrompt {}
impl HasIcons for GreetingPrompt {}

#[async_trait::async_trait]
impl McpPrompt for GreetingPrompt {
    async fn render(
        &self,
        args: Option<HashMap<String, serde_json::Value>>,
    ) -> McpResult<Vec<PromptMessage>> {
        let name = args
            .as_ref()
            .and_then(|a| a.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("world");
        Ok(vec![PromptMessage::user_text(format!("Hello, {name}!"))])
    }
}

/// Completes the `greeting` prompt's `name` argument.
struct GreetingNameCompleter;

impl HasCompletionMetadata for GreetingNameCompleter {
    fn method(&self) -> &str {
        "completion/complete"
    }
    fn reference(&self) -> &CompletionReference {
        static REF: OnceLock<CompletionReference> = OnceLock::new();
        REF.get_or_init(|| CompletionReference::Prompt(PromptReference::new("greeting")))
    }
}
impl HasCompletionContext for GreetingNameCompleter {
    fn argument(&self) -> &CompleteArgument {
        static ARG: OnceLock<CompleteArgument> = OnceLock::new();
        ARG.get_or_init(|| CompleteArgument::new("name", ""))
    }
}
impl HasCompletionHandling for GreetingNameCompleter {}

#[async_trait::async_trait]
impl McpCompletion for GreetingNameCompleter {
    async fn complete(&self, request: CompleteRequest) -> McpResult<CompleteResult> {
        let prefix = request.params.argument.value.to_lowercase();
        let values: Vec<String> = ["ada", "alan", "grace"]
            .iter()
            .filter(|v| v.starts_with(&prefix))
            .map(|v| v.to_string())
            .collect();
        Ok(CompleteResult::new(CompletionResult::new(values)))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let mut port: u16 = 8700;
    let args: Vec<String> = std::env::args().collect();
    for i in 0..args.len() {
        if args[i] == "--port"
            && let Some(p) = args.get(i + 1)
        {
            port = p.parse()?;
        }
    }
    let bind_address: std::net::SocketAddr = format!("127.0.0.1:{port}").parse()?;

    let server = McpServer::builder()
        .name("interop-fixture-server")
        .version(env!("CARGO_PKG_VERSION"))
        .tool_fn(echo)
        .tool_fn(add)
        .tool(ConfirmTool::default())
        .tool(CountTool::default())
        .resource(FixtureResource)
        .prompt(GreetingPrompt)
        .completion_provider(GreetingNameCompleter)
        .bind_address(bind_address)
        .build()?;

    println!("Interop fixture server running at: http://{bind_address}/mcp");
    println!("  tools:     echo, add");
    println!("  resources: {FIXTURE_URI}");
    println!("  prompts:   greeting(name)");
    println!("  completion: greeting/name");

    server.run().await?;
    Ok(())
}
