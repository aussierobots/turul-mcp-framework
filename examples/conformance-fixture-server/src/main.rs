//! # Conformance Fixture Server
//!
//! First bounded slice of a server exposing the fixtures required by
//! upstream's `@modelcontextprotocol/conformance@0.2.0-alpha.11` suite
//! (`docs/plans/2026-07-28-conformance-fixtures.md`). That suite lists 27
//! distinctly named fixtures; this crate implements 4, chosen to cover the
//! main shapes (plain-text tool result, error-flagged tool result, a
//! no-argument prompt, and the shared negative-capability fixture) so the
//! pattern can be replicated for the rest.
//!
//! Payloads here are asserted byte-for-byte by the conformance suite —
//! changing one without re-checking the corresponding scenario in
//! `docs/plans/2026-07-28-conformance-fixtures.md` will silently regress a
//! passing scenario to a failure.
//!
//!   cargo run -p conformance-fixture-server -- --port 8010

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;
use turul_mcp_derive::mcp_tool;
use turul_mcp_protocol::McpError;
use turul_mcp_protocol::content::ResourceContents;
use turul_mcp_protocol::prompts::PromptMessage;
use turul_mcp_protocol::resources::ResourceContent;
use turul_mcp_protocol::tools::{CallToolResult, ToolAnnotations, ToolResult, ToolSchema};
use turul_mcp_server::prelude::*;
use turul_mcp_server::{McpPrompt, McpResource, McpResult, McpServer, McpTool, SessionContext};

/// `tools-call-simple-text`: no arguments, fixed text content.
#[mcp_tool(
    name = "test_simple_text",
    description = "Returns a simple text response for conformance testing"
)]
async fn test_simple_text() -> McpResult<String> {
    Ok("This is a simple text response for testing.".to_string())
}

/// `tools-call-error`: no arguments, always answers with `isError: true`.
///
/// This is a *successful* JSON-RPC response — the error is carried in the
/// `CallToolResult` payload, not the transport — so it is implemented as a
/// manual `McpTool` rather than through the `#[mcp_tool]`/`#[derive(McpTool)]`
/// macros: both macros always wrap `Ok(_)` into a success result and have no
/// attribute to opt into `isError: true` (verified against
/// `turul-mcp-derive/src/tool_attr.rs` and `tool_derive.rs`).
#[derive(Clone)]
struct ErrorHandlingTool;

impl HasBaseMetadata for ErrorHandlingTool {
    fn name(&self) -> &str {
        "test_error_handling"
    }
}
impl HasDescription for ErrorHandlingTool {
    fn description(&self) -> Option<&str> {
        Some("Always returns an error result for conformance testing")
    }
}
impl HasInputSchema for ErrorHandlingTool {
    fn input_schema(&self) -> &ToolSchema {
        static SCHEMA: std::sync::OnceLock<ToolSchema> = std::sync::OnceLock::new();
        SCHEMA.get_or_init(ToolSchema::object)
    }
}
impl HasOutputSchema for ErrorHandlingTool {
    fn output_schema(&self) -> Option<&ToolSchema> {
        None
    }
}
impl HasAnnotations for ErrorHandlingTool {
    fn annotations(&self) -> Option<&ToolAnnotations> {
        None
    }
}
impl HasToolMeta for ErrorHandlingTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
        None
    }
}
impl HasIcons for ErrorHandlingTool {}
impl HasExecution for ErrorHandlingTool {}

#[async_trait]
impl McpTool for ErrorHandlingTool {
    async fn call(
        &self,
        _args: Value,
        _session: Option<SessionContext>,
    ) -> McpResult<CallToolResult> {
        Ok(CallToolResult::error(vec![ToolResult::text(
            "This tool intentionally returns an error for testing",
        )]))
    }
}

/// The shared negative-path fixture (appears in 27 scenarios per
/// `docs/plans/2026-07-28-conformance-fixtures.md`): a "structural test tool
/// requiring explicit capabilities in `_meta`". The harvested doc gives no
/// distinct success-path payload for it — only the failure contract
/// (`server-stateless` §4): calling it without the required client
/// capability declared answers `-32021 MissingRequiredClientCapabilityError`
/// with `error.data.requiredCapabilities` keyed by capability name, e.g.
/// `{ "sampling": {} }`.
///
/// KNOWN LIMITATION for a follow-up agent: this always returns that error.
/// The framework has no generic "tool declares a required client capability,
/// framework checks the per-request `_meta.clientCapabilities` and enforces
/// it" mechanism outside the Tasks-extension-specific check in
/// `turul-mcp-server/src/server.rs` (`ext_tasks::declared`) — the per-request
/// `client_capabilities` is read in `server.rs` but never surfaced into
/// `SessionContext` for a tool's own `call()` to inspect. Wiring the positive
/// branch (declared capability -> some success payload) would require
/// changing `crates/turul-mcp-server`, which is out of scope for an
/// examples-only slice. If a later scenario needs the positive branch,
/// that plumbing is the blocker to solve first.
#[derive(Clone)]
struct MissingCapabilityTool;

impl HasBaseMetadata for MissingCapabilityTool {
    fn name(&self) -> &str {
        "test_missing_capability"
    }
}
impl HasDescription for MissingCapabilityTool {
    fn description(&self) -> Option<&str> {
        Some("Requires a client capability that this fixture never declares")
    }
}
impl HasInputSchema for MissingCapabilityTool {
    fn input_schema(&self) -> &ToolSchema {
        static SCHEMA: std::sync::OnceLock<ToolSchema> = std::sync::OnceLock::new();
        SCHEMA.get_or_init(ToolSchema::object)
    }
}
impl HasOutputSchema for MissingCapabilityTool {
    fn output_schema(&self) -> Option<&ToolSchema> {
        None
    }
}
impl HasAnnotations for MissingCapabilityTool {
    fn annotations(&self) -> Option<&ToolAnnotations> {
        None
    }
}
impl HasToolMeta for MissingCapabilityTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
        None
    }
}
impl HasIcons for MissingCapabilityTool {}
impl HasExecution for MissingCapabilityTool {}

#[async_trait]
impl McpTool for MissingCapabilityTool {
    async fn call(
        &self,
        _args: Value,
        _session: Option<SessionContext>,
    ) -> McpResult<CallToolResult> {
        Err(McpError::MissingRequiredClientCapability {
            required: serde_json::json!({ "sampling": {} }),
        })
    }
}

/// `tools-call-image`: no arguments, returns image content.
#[derive(Clone)]
struct ImageContentTool;

impl HasBaseMetadata for ImageContentTool {
    fn name(&self) -> &str {
        "test_image_content"
    }
}
impl HasDescription for ImageContentTool {
    fn description(&self) -> Option<&str> {
        Some("Returns an image for conformance testing")
    }
}
impl HasInputSchema for ImageContentTool {
    fn input_schema(&self) -> &ToolSchema {
        static SCHEMA: std::sync::OnceLock<ToolSchema> = std::sync::OnceLock::new();
        SCHEMA.get_or_init(ToolSchema::object)
    }
}
impl HasOutputSchema for ImageContentTool {
    fn output_schema(&self) -> Option<&ToolSchema> {
        None
    }
}
impl HasAnnotations for ImageContentTool {
    fn annotations(&self) -> Option<&ToolAnnotations> {
        None
    }
}
impl HasToolMeta for ImageContentTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
        None
    }
}
impl HasIcons for ImageContentTool {}
impl HasExecution for ImageContentTool {}

#[async_trait]
impl McpTool for ImageContentTool {
    async fn call(
        &self,
        _args: Value,
        _session: Option<SessionContext>,
    ) -> McpResult<CallToolResult> {
        // 1x1 red pixel PNG in base64
        let base64_png = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFBQIAX8jx0gAAAABJRU5ErkJggg==";
        Ok(CallToolResult::success(vec![ToolResult::image(
            base64_png.to_string(),
            "image/png",
        )]))
    }
}

/// `tools-call-audio`: no arguments, returns audio content.
#[derive(Clone)]
struct AudioContentTool;

impl HasBaseMetadata for AudioContentTool {
    fn name(&self) -> &str {
        "test_audio_content"
    }
}
impl HasDescription for AudioContentTool {
    fn description(&self) -> Option<&str> {
        Some("Returns audio content for conformance testing")
    }
}
impl HasInputSchema for AudioContentTool {
    fn input_schema(&self) -> &ToolSchema {
        static SCHEMA: std::sync::OnceLock<ToolSchema> = std::sync::OnceLock::new();
        SCHEMA.get_or_init(ToolSchema::object)
    }
}
impl HasOutputSchema for AudioContentTool {
    fn output_schema(&self) -> Option<&ToolSchema> {
        None
    }
}
impl HasAnnotations for AudioContentTool {
    fn annotations(&self) -> Option<&ToolAnnotations> {
        None
    }
}
impl HasToolMeta for AudioContentTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
        None
    }
}
impl HasIcons for AudioContentTool {}
impl HasExecution for AudioContentTool {}

#[async_trait]
impl McpTool for AudioContentTool {
    async fn call(
        &self,
        _args: Value,
        _session: Option<SessionContext>,
    ) -> McpResult<CallToolResult> {
        // Minimal WAV file (silence, 1 second, 16-bit, 44.1kHz) in base64
        let base64_wav = "UklGRiYAAABXQVZFZm10IBAAAAABAAEAQB8AAAB9AAACABAAZGF0YQIAAAAAAA==";
        Ok(CallToolResult::success(vec![ToolResult::audio(
            base64_wav.to_string(),
            "audio/wav",
        )]))
    }
}

/// `tools-call-embedded-resource`: no arguments, returns resource content.
#[derive(Clone)]
struct EmbeddedResourceTool;

impl HasBaseMetadata for EmbeddedResourceTool {
    fn name(&self) -> &str {
        "test_embedded_resource"
    }
}
impl HasDescription for EmbeddedResourceTool {
    fn description(&self) -> Option<&str> {
        Some("Returns an embedded resource for conformance testing")
    }
}
impl HasInputSchema for EmbeddedResourceTool {
    fn input_schema(&self) -> &ToolSchema {
        static SCHEMA: std::sync::OnceLock<ToolSchema> = std::sync::OnceLock::new();
        SCHEMA.get_or_init(ToolSchema::object)
    }
}
impl HasOutputSchema for EmbeddedResourceTool {
    fn output_schema(&self) -> Option<&ToolSchema> {
        None
    }
}
impl HasAnnotations for EmbeddedResourceTool {
    fn annotations(&self) -> Option<&ToolAnnotations> {
        None
    }
}
impl HasToolMeta for EmbeddedResourceTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
        None
    }
}
impl HasIcons for EmbeddedResourceTool {}
impl HasExecution for EmbeddedResourceTool {}

#[async_trait]
impl McpTool for EmbeddedResourceTool {
    async fn call(
        &self,
        _args: Value,
        _session: Option<SessionContext>,
    ) -> McpResult<CallToolResult> {
        Ok(CallToolResult::success(vec![ToolResult::resource(
            ResourceContents::text_with_mime(
                "test://embedded-resource",
                "This is an embedded resource content.",
                "text/plain",
            ),
        )]))
    }
}

/// `tools-call-mixed-content`: no arguments, returns multiple content types.
#[derive(Clone)]
struct MultipleContentTypesTool;

impl HasBaseMetadata for MultipleContentTypesTool {
    fn name(&self) -> &str {
        "test_multiple_content_types"
    }
}
impl HasDescription for MultipleContentTypesTool {
    fn description(&self) -> Option<&str> {
        Some("Returns multiple content types for conformance testing")
    }
}
impl HasInputSchema for MultipleContentTypesTool {
    fn input_schema(&self) -> &ToolSchema {
        static SCHEMA: std::sync::OnceLock<ToolSchema> = std::sync::OnceLock::new();
        SCHEMA.get_or_init(ToolSchema::object)
    }
}
impl HasOutputSchema for MultipleContentTypesTool {
    fn output_schema(&self) -> Option<&ToolSchema> {
        None
    }
}
impl HasAnnotations for MultipleContentTypesTool {
    fn annotations(&self) -> Option<&ToolAnnotations> {
        None
    }
}
impl HasToolMeta for MultipleContentTypesTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
        None
    }
}
impl HasIcons for MultipleContentTypesTool {}
impl HasExecution for MultipleContentTypesTool {}

#[async_trait]
impl McpTool for MultipleContentTypesTool {
    async fn call(
        &self,
        _args: Value,
        _session: Option<SessionContext>,
    ) -> McpResult<CallToolResult> {
        let base64_png = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFBQIAX8jx0gAAAABJRU5ErkJggg==";
        let json_text = r#"{"test":"data","value":123}"#;
        Ok(CallToolResult::success(vec![
            ToolResult::text("Multiple content types test:"),
            ToolResult::image(base64_png.to_string(), "image/png"),
            ToolResult::resource(ResourceContents::text_with_mime(
                "test://mixed-content-resource",
                json_text,
                "application/json",
            )),
        ]))
    }
}

/// `prompts-get-simple`: no arguments, fixed single user-text message.
struct SimplePrompt;

impl HasPromptMetadata for SimplePrompt {
    fn name(&self) -> &str {
        "test_simple_prompt"
    }
}
impl HasPromptDescription for SimplePrompt {
    fn description(&self) -> Option<&str> {
        Some("A simple prompt for conformance testing")
    }
}
impl HasPromptArguments for SimplePrompt {}
impl HasPromptAnnotations for SimplePrompt {}
impl HasPromptMeta for SimplePrompt {}
impl HasIcons for SimplePrompt {}

#[async_trait]
impl McpPrompt for SimplePrompt {
    async fn render(
        &self,
        _args: Option<HashMap<String, serde_json::Value>>,
    ) -> McpResult<Vec<PromptMessage>> {
        Ok(vec![PromptMessage::user_text(
            "This is a simple prompt for testing.",
        )])
    }
}

/// `prompts-get-with-args`: takes two required arguments and returns a prompt
/// that includes their values in the message.
struct PromptWithArguments;

impl HasPromptMetadata for PromptWithArguments {
    fn name(&self) -> &str {
        "test_prompt_with_arguments"
    }
}
impl HasPromptDescription for PromptWithArguments {
    fn description(&self) -> Option<&str> {
        Some("A prompt that accepts arguments")
    }
}
impl HasPromptArguments for PromptWithArguments {
    fn arguments(&self) -> Option<&Vec<turul_mcp_protocol::prompts::PromptArgument>> {
        static ARGS: std::sync::OnceLock<Vec<turul_mcp_protocol::prompts::PromptArgument>> =
            std::sync::OnceLock::new();
        Some(ARGS.get_or_init(|| {
            vec![
                turul_mcp_protocol::prompts::PromptArgument {
                    name: "arg1".to_string(),
                    title: None,
                    description: Some("First test argument".to_string()),
                    required: Some(true),
                },
                turul_mcp_protocol::prompts::PromptArgument {
                    name: "arg2".to_string(),
                    title: None,
                    description: Some("Second test argument".to_string()),
                    required: Some(true),
                },
            ]
        }))
    }
}
impl HasPromptAnnotations for PromptWithArguments {}
impl HasPromptMeta for PromptWithArguments {}
impl HasIcons for PromptWithArguments {}

#[async_trait]
impl McpPrompt for PromptWithArguments {
    async fn render(
        &self,
        args: Option<HashMap<String, serde_json::Value>>,
    ) -> McpResult<Vec<PromptMessage>> {
        let arg1 = args
            .as_ref()
            .and_then(|a| a.get("arg1"))
            .and_then(|v| v.as_str())
            .unwrap_or("missing");
        let arg2 = args
            .as_ref()
            .and_then(|a| a.get("arg2"))
            .and_then(|v| v.as_str())
            .unwrap_or("missing");

        Ok(vec![PromptMessage::user_text(format!(
            "Prompt with arguments: arg1='{}', arg2='{}'",
            arg1, arg2
        ))])
    }
}

/// `prompts-get-with-image`: no arguments, returns a prompt with an image and text content.
struct PromptWithImage;

impl HasPromptMetadata for PromptWithImage {
    fn name(&self) -> &str {
        "test_prompt_with_image"
    }
}
impl HasPromptDescription for PromptWithImage {
    fn description(&self) -> Option<&str> {
        Some("A prompt that includes an image")
    }
}
impl HasPromptArguments for PromptWithImage {}
impl HasPromptAnnotations for PromptWithImage {}
impl HasPromptMeta for PromptWithImage {}
impl HasIcons for PromptWithImage {}

#[async_trait]
impl McpPrompt for PromptWithImage {
    async fn render(
        &self,
        _args: Option<HashMap<String, serde_json::Value>>,
    ) -> McpResult<Vec<PromptMessage>> {
        // 1x1 red pixel PNG in base64
        let base64_png = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFBQIAX8jx0gAAAABJRU5ErkJggg==";
        Ok(vec![
            PromptMessage::user_image(base64_png.to_string(), "image/png"),
            PromptMessage::user_text("Please analyze the image above."),
        ])
    }
}

/// `prompts-get-embedded-resource`: takes a resourceUri argument and returns
/// a prompt that embeds the resource.
struct PromptWithEmbeddedResource;

impl HasPromptMetadata for PromptWithEmbeddedResource {
    fn name(&self) -> &str {
        "test_prompt_with_embedded_resource"
    }
}
impl HasPromptDescription for PromptWithEmbeddedResource {
    fn description(&self) -> Option<&str> {
        Some("A prompt that embeds a resource")
    }
}
impl HasPromptArguments for PromptWithEmbeddedResource {
    fn arguments(&self) -> Option<&Vec<turul_mcp_protocol::prompts::PromptArgument>> {
        static ARGS: std::sync::OnceLock<Vec<turul_mcp_protocol::prompts::PromptArgument>> =
            std::sync::OnceLock::new();
        Some(ARGS.get_or_init(|| {
            vec![turul_mcp_protocol::prompts::PromptArgument {
                name: "resourceUri".to_string(),
                title: None,
                description: Some("URI of the resource to embed".to_string()),
                required: Some(true),
            }]
        }))
    }
}
impl HasPromptAnnotations for PromptWithEmbeddedResource {}
impl HasPromptMeta for PromptWithEmbeddedResource {}
impl HasIcons for PromptWithEmbeddedResource {}

#[async_trait]
impl McpPrompt for PromptWithEmbeddedResource {
    async fn render(
        &self,
        args: Option<HashMap<String, serde_json::Value>>,
    ) -> McpResult<Vec<PromptMessage>> {
        use turul_mcp_protocol::content::ContentBlock;
        use turul_mcp_protocol::prompts::Role;

        let resource_uri = args
            .as_ref()
            .and_then(|a| a.get("resourceUri"))
            .and_then(|v| v.as_str())
            .unwrap_or("test://default-resource");

        Ok(vec![
            PromptMessage {
                role: Role::User,
                content: ContentBlock::resource(ResourceContents::text_with_mime(
                    resource_uri,
                    "Embedded resource content for testing.",
                    "text/plain",
                )),
            },
            PromptMessage::user_text("Please process the embedded resource above."),
        ])
    }
}

/// `resources-read-text`: resource at test://static-text that returns plain text content.
struct StaticTextResource;

impl HasResourceMetadata for StaticTextResource {
    fn name(&self) -> &str {
        "Static Text Resource"
    }
}

impl HasResourceUri for StaticTextResource {
    fn uri(&self) -> &str {
        "test://static-text"
    }
}

impl HasResourceDescription for StaticTextResource {
    fn description(&self) -> Option<&str> {
        Some("A static text resource for conformance testing")
    }
}

impl HasResourceMimeType for StaticTextResource {
    fn mime_type(&self) -> Option<&str> {
        Some("text/plain")
    }
}

impl HasResourceSize for StaticTextResource {}
impl HasResourceAnnotations for StaticTextResource {
    fn annotations(&self) -> Option<&turul_mcp_protocol::meta::Annotations> {
        None
    }
}
impl HasResourceMeta for StaticTextResource {}
impl HasIcons for StaticTextResource {}

#[async_trait]
impl McpResource for StaticTextResource {
    async fn read(
        &self,
        _params: Option<serde_json::Value>,
        _session: Option<&SessionContext>,
    ) -> McpResult<Vec<ResourceContent>> {
        Ok(vec![ResourceContent::text(
            "test://static-text",
            "This is the content of the static text resource.",
        )])
    }
}

/// `resources-read-binary`: resource at test://static-binary that returns binary content.
struct StaticBinaryResource;

impl HasResourceMetadata for StaticBinaryResource {
    fn name(&self) -> &str {
        "Static Binary Resource"
    }
}

impl HasResourceUri for StaticBinaryResource {
    fn uri(&self) -> &str {
        "test://static-binary"
    }
}

impl HasResourceDescription for StaticBinaryResource {
    fn description(&self) -> Option<&str> {
        Some("A static binary resource for conformance testing")
    }
}

impl HasResourceMimeType for StaticBinaryResource {
    fn mime_type(&self) -> Option<&str> {
        Some("image/png")
    }
}

impl HasResourceSize for StaticBinaryResource {}
impl HasResourceAnnotations for StaticBinaryResource {
    fn annotations(&self) -> Option<&turul_mcp_protocol::meta::Annotations> {
        None
    }
}
impl HasResourceMeta for StaticBinaryResource {}
impl HasIcons for StaticBinaryResource {}

#[async_trait]
impl McpResource for StaticBinaryResource {
    async fn read(
        &self,
        _params: Option<serde_json::Value>,
        _session: Option<&SessionContext>,
    ) -> McpResult<Vec<ResourceContent>> {
        // 1x1 red pixel PNG in base64
        let base64_png = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFBQIAX8jx0gAAAABJRU5ErkJggg==";
        Ok(vec![ResourceContent::blob(
            "test://static-binary",
            base64_png.to_string(),
            "image/png",
        )])
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let mut port: u16 = 8010;
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
        .name("conformance-fixture-server")
        .version(env!("CARGO_PKG_VERSION"))
        .tool_fn(test_simple_text)
        .tool(ErrorHandlingTool)
        .tool(MissingCapabilityTool)
        .tool(ImageContentTool)
        .tool(AudioContentTool)
        .tool(EmbeddedResourceTool)
        .tool(MultipleContentTypesTool)
        .prompt(SimplePrompt)
        .prompt(PromptWithArguments)
        .prompt(PromptWithImage)
        .prompt(PromptWithEmbeddedResource)
        .resource(StaticTextResource)
        .resource(StaticBinaryResource)
        .bind_address(bind_address)
        .build()?;

    println!("Conformance fixture server running at: http://{bind_address}/mcp");
    println!("  tools:    test_simple_text, test_error_handling, test_missing_capability,");
    println!("            test_image_content, test_audio_content, test_embedded_resource,");
    println!("            test_multiple_content_types");
    println!("  prompts:  test_simple_prompt, test_prompt_with_arguments,");
    println!("            test_prompt_with_image, test_prompt_with_embedded_resource");
    println!("  resources: test://static-text, test://static-binary");

    server.run().await?;
    Ok(())
}
