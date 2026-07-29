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

use turul_mcp_derive::mcp_tool;
use turul_mcp_protocol::completion::{
    CompleteArgument, CompleteRequest, CompleteResult, CompletionReference, CompletionResult,
    PromptReference,
};
use turul_mcp_protocol::prompts::{PromptArgument, PromptMessage};
use turul_mcp_protocol::resources::ResourceContent;
use turul_mcp_server::prelude::*;
use turul_mcp_server::{McpCompletion, McpPrompt, McpResource};

const FIXTURE_URI: &str = "file:///fixture/readme.md";
const FIXTURE_TEXT: &str = "# Interop fixture\n\nStable text for cross-implementation probes.\n";

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
        Some("text/markdown")
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
        Ok(vec![ResourceContent::text(FIXTURE_URI, FIXTURE_TEXT)])
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
        Some(ARGS.get_or_init(|| {
            vec![PromptArgument::new("name").with_description("Who to greet")]
        }))
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
        .version("0.4.0")
        .tool_fn(echo)
        .tool_fn(add)
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
