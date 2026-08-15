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
use hmac::{Hmac, Mac};
use serde_json::Value;
use sha2::Sha256;
use turul_mcp_derive::{McpTool, mcp_tool};
use turul_mcp_protocol::McpError;
use turul_mcp_protocol::content::ResourceContents;
use turul_mcp_protocol::elicitation::{
    ElicitRequest, ElicitationSchema, PrimitiveSchemaDefinition,
};
use turul_mcp_protocol::input_required::{InputRequest, InputRequests, InputResponse};
use turul_mcp_protocol::prompts::PromptMessage;
use turul_mcp_protocol::resources::ResourceContent;
// `roots/list` is deprecated under SEP-2577, but remains a valid `InputRequest`
// variant for the SEP-2322 migration window — which is precisely what the
// `input-required-result-basic-list-roots` scenario exercises. Suppressed here
// rather than avoided: dropping the fixture would drop the coverage.
#[allow(deprecated)]
use turul_mcp_protocol::roots::ListRootsRequest;
#[allow(deprecated)]
use turul_mcp_protocol::sampling::{CreateMessageRequest, SamplingMessage};
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

// ---------------------------------------------------------------------------
// MRTR (SEP-2322) fixtures — the `input-required-result-*` scenarios.
//
// Fixture NAMES here come from the harness itself, not from
// `docs/plans/2026-07-28-conformance-fixtures.md`: a scenario that cannot find
// its fixture reports `Unknown tool: <name>`, which is the authoritative
// spelling. The harvested plan disagrees with it in places.
//
// Shape of every one of these: the first leg returns
// `Err(McpError::InputRequired { .. })`, which the framework renders as an
// `InputRequiredResult` (`resultType: "input_required"`) — a SUCCESSFUL
// JSON-RPC response, not an error. The client retries the original call with
// `inputResponses` + the echoed `requestState`, and `session.input_responses()`
// surfaces them on the retry leg.
//
// This is why 0.4.2 deliberately did NOT convert `InputRequired` into
// `isError: true` when it made `ToolExecutionError` do so — a blanket
// conversion broke 7 MRTR wire tests. See `turul-mcp-derive/src/tool_attr.rs`.
// ---------------------------------------------------------------------------

/// One elicitation, then complete. Serves four scenarios:
/// `input-required-result-basic-elicitation`, `-ignore-extra-params`,
/// `-result-type` and `-validate-input`.
#[derive(McpTool, Clone, Default)]
#[tool(
    name = "test_input_required_result_elicitation",
    description = "Requires an elicitation before completing",
    output = String
)]
struct InputRequiredElicitationTool {}

impl InputRequiredElicitationTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
        if let Some(session) = &session
            && let Some(responses) = session.input_responses()
        {
            let answer = responses
                .get("user_name")
                .and_then(|r| match r {
                    InputResponse::Elicit(e) => e
                        .content
                        .as_ref()
                        .and_then(|c| c.get("name"))
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    _ => None,
                })
                .unwrap_or_else(|| "<no answer>".to_string());
            return Ok(format!("Received input: {answer}"));
        }
        // The harness asserts the inputRequests key is exactly "user_name" —
        // these keys are part of the fixture contract, not free choice.
        let schema = ElicitationSchema::new()
            .with_property("name".to_string(), PrimitiveSchemaDefinition::string());
        let mut requests = InputRequests::new();
        requests.insert(
            "user_name".to_string(),
            InputRequest::Elicit(ElicitRequest::new_form("What is your name?", schema)),
        );
        Err(McpError::InputRequired {
            input_requests: Some(requests),
            request_state: Some("elicitation-state-1".to_string()),
        })
    }
}

/// `input-required-result-basic-list-roots`: asks the client for its roots.
///
/// `roots/list` is deprecated under SEP-2577 but remains a valid
/// `InputRequest` variant for the SEP-2322 migration window, which is exactly
/// what this scenario exercises.
#[derive(McpTool, Clone, Default)]
#[tool(
    name = "test_input_required_result_list_roots",
    description = "Requires the client's roots before completing",
    output = String
)]
struct InputRequiredListRootsTool {}

impl InputRequiredListRootsTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
        if let Some(session) = &session
            && session.input_responses().is_some()
        {
            return Ok("Received roots".to_string());
        }
        let mut requests = InputRequests::new();
        #[allow(deprecated)]
        requests.insert(
            "roots1".to_string(),
            InputRequest::ListRoots(ListRootsRequest::new()),
        );
        Err(McpError::InputRequired {
            input_requests: Some(requests),
            request_state: Some("list-roots-state-1".to_string()),
        })
    }
}

/// `input-required-result-basic-sampling`: asks the client to sample.
#[derive(McpTool, Clone, Default)]
#[tool(
    name = "test_input_required_result_sampling",
    description = "Requires a sampling result before completing",
    output = String
)]
struct InputRequiredSamplingTool {}

impl InputRequiredSamplingTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
        if let Some(session) = &session
            && session.input_responses().is_some()
        {
            return Ok("Received sampling result".to_string());
        }
        #[allow(deprecated)]
        let request = CreateMessageRequest::new(vec![SamplingMessage::user_text("Say hello")], 64);
        let mut requests = InputRequests::new();
        requests.insert("s1".to_string(), InputRequest::CreateMessage(request));
        Err(McpError::InputRequired {
            input_requests: Some(requests),
            request_state: Some("sampling-state-1".to_string()),
        })
    }
}

/// `input-required-result-multiple-input-requests`: two distinct requests in
/// one `InputRequiredResult`, keyed independently.
#[derive(McpTool, Clone, Default)]
#[tool(
    name = "test_input_required_result_multiple_inputs",
    description = "Requires two inputs in a single round trip",
    output = String
)]
struct InputRequiredMultipleInputsTool {}

impl InputRequiredMultipleInputsTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
        if let Some(session) = &session
            && let Some(responses) = session.input_responses()
        {
            let mut keys: Vec<&String> = responses.keys().collect();
            keys.sort();
            let joined = keys
                .iter()
                .map(|k| k.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            return Ok(format!("Received inputs: {joined}"));
        }
        // The harness requires at least THREE requests spanning three distinct
        // method types — elicitation/create, sampling/createMessage and
        // roots/list — not merely three requests.
        let mut requests = InputRequests::new();
        let schema = ElicitationSchema::new()
            .with_property("name".to_string(), PrimitiveSchemaDefinition::string());
        requests.insert(
            "user_name".to_string(),
            InputRequest::Elicit(ElicitRequest::new_form("What is your name?", schema)),
        );
        #[allow(deprecated)]
        {
            let request =
                CreateMessageRequest::new(vec![SamplingMessage::user_text("Say hello")], 64);
            requests.insert("s1".to_string(), InputRequest::CreateMessage(request));
            requests.insert(
                "roots1".to_string(),
                InputRequest::ListRoots(ListRootsRequest::new()),
            );
        }
        Err(McpError::InputRequired {
            input_requests: Some(requests),
            request_state: Some("multiple-inputs-state-1".to_string()),
        })
    }
}

/// `input-required-result-request-state`: echoes the `requestState` the server
/// issued, proving it survived the round trip verbatim.
///
/// `input-required-result-tampered-state` also depends on this fixture class:
/// it first needs a real `InputRequiredResult` carrying `requestState` before
/// it can tamper with one.
#[derive(McpTool, Clone, Default)]
#[tool(
    name = "test_input_required_result_request_state",
    description = "Round-trips an opaque requestState",
    output = String
)]
struct InputRequiredRequestStateTool {}

impl InputRequiredRequestStateTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
        if let Some(session) = &session
            && session.input_responses().is_some()
        {
            let state = session
                .mrtr_request_state()
                .unwrap_or_else(|| "<none>".to_string());
            return Ok(format!("Received state: {state}"));
        }
        let schema = ElicitationSchema::new()
            .with_property("answer".to_string(), PrimitiveSchemaDefinition::string());
        let mut requests = InputRequests::new();
        requests.insert(
            "q1".to_string(),
            InputRequest::Elicit(ElicitRequest::new_form("Confirm?", schema)),
        );
        Err(McpError::InputRequired {
            input_requests: Some(requests),
            request_state: Some("request-state-token-1".to_string()),
        })
    }
}

/// `input-required-result-capability-check`: SEP-2322 requires a server to
/// include `inputRequests` ONLY for capabilities the client declared.
///
/// The harness calls this with `clientCapabilities: { sampling: {} }` and
/// fails the run if any `elicitation/create` request comes back. So the tool
/// must *adapt* what it asks for rather than asking unconditionally and
/// letting the framework's `-32021` gate fire — degrading gracefully is the
/// behaviour under test.
///
/// Reading the declaration needs `SessionContext::client_capabilities()`,
/// added 2026-08-15 for exactly this: the framework already enforced the
/// negative half, but a tool could not see what was declared, so this
/// requirement was unimplementable by any server built on the framework.
#[derive(McpTool, Clone, Default)]
#[tool(
    name = "test_input_required_result_capabilities",
    description = "Requests only the input kinds the client declared",
    output = String
)]
struct InputRequiredCapabilitiesTool {}

impl InputRequiredCapabilitiesTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
        let session = session.ok_or_else(|| McpError::tool_execution("session required"))?;
        if session.input_responses().is_some() {
            return Ok("Capability satisfied".to_string());
        }
        let caps = session.client_capabilities().unwrap_or_default();

        let mut requests = InputRequests::new();
        if caps.elicitation.is_some() {
            let schema = ElicitationSchema::new()
                .with_property("answer".to_string(), PrimitiveSchemaDefinition::string());
            requests.insert(
                "user_name".to_string(),
                InputRequest::Elicit(ElicitRequest::new_form("Capability gated", schema)),
            );
        }
        if caps.sampling.is_some() {
            #[allow(deprecated)]
            let request =
                CreateMessageRequest::new(vec![SamplingMessage::user_text("Say hello")], 64);
            requests.insert("s1".to_string(), InputRequest::CreateMessage(request));
        }
        if requests.is_empty() {
            return Ok("No supported input capability declared".to_string());
        }
        Err(McpError::InputRequired {
            input_requests: Some(requests),
            request_state: Some("capability-state-1".to_string()),
        })
    }
}

/// `input-required-result-multi-round`: three legs, not two.
///
/// Round 1 asks for a name; round 2 must return ANOTHER `InputRequiredResult`
/// with a *different* `requestState`; round 3 completes. The harness asserts
/// the state actually changes between rounds, so the fixture keys its
/// progress off the echoed state rather than a counter.
#[derive(McpTool, Clone, Default)]
#[tool(
    name = "test_input_required_result_multi_round",
    description = "Requires two rounds of input before completing",
    output = String
)]
struct InputRequiredMultiRoundTool {}

impl InputRequiredMultiRoundTool {
    fn ask(field: &str, prompt: &str, state: &str) -> McpError {
        let schema = ElicitationSchema::new()
            .with_property(field.to_string(), PrimitiveSchemaDefinition::string());
        let mut requests = InputRequests::new();
        requests.insert(
            field.to_string(),
            InputRequest::Elicit(ElicitRequest::new_form(prompt, schema)),
        );
        McpError::InputRequired {
            input_requests: Some(requests),
            request_state: Some(state.to_string()),
        }
    }

    async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
        let session = session.ok_or_else(|| McpError::tool_execution("session required"))?;
        match session.mrtr_request_state().as_deref() {
            // Leg 1: nothing echoed yet.
            None => Err(Self::ask("name", "What is your name?", "multi-round-1")),
            // Leg 2: round 1 answered — ask again under a NEW state.
            Some("multi-round-1") => Err(Self::ask(
                "color",
                "What is your favourite colour?",
                "multi-round-2",
            )),
            // Leg 3: both answered.
            Some("multi-round-2") => Ok("Multi-round complete".to_string()),
            Some(other) => Err(McpError::tool_execution(&format!(
                "unrecognised requestState: {other}"
            ))),
        }
    }
}

/// `input-required-result-tampered-state`: `requestState` is
/// attacker-controlled, so a server MUST detect modification rather than
/// trusting what comes back.
///
/// The harness takes the issued state, appends `-TAMPERED`, and requires a
/// JSON-RPC error. This fixture signs the state with an HMAC and rejects a
/// bad tag — the pattern a real server should copy, and the reason
/// `SessionContext::mrtr_request_state` documents the value as
/// attacker-controlled.
///
/// The key is process-local and random per boot: nothing here is persisted or
/// verified across restarts, and a fixture must never ship a fixed secret.
#[derive(McpTool, Clone, Default)]
#[tool(
    name = "test_input_required_result_tampered_state",
    description = "Rejects a modified requestState",
    output = String
)]
struct InputRequiredTamperedStateTool {}

impl InputRequiredTamperedStateTool {
    fn key() -> &'static [u8; 32] {
        static KEY: std::sync::OnceLock<[u8; 32]> = std::sync::OnceLock::new();
        KEY.get_or_init(|| {
            let mut k = [0u8; 32];
            getrandom::fill(&mut k).expect("OS RNG unavailable");
            k
        })
    }

    /// `payload.hex(HMAC-SHA256(key, payload))`
    fn sign(payload: &str) -> String {
        let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(Self::key()).expect("any key length");
        mac.update(payload.as_bytes());
        let tag = mac.finalize().into_bytes();
        format!("{payload}.{}", hex_lower(&tag))
    }

    fn verify(state: &str) -> bool {
        let Some((payload, tag)) = state.rsplit_once('.') else {
            return false;
        };
        let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(Self::key()).expect("any key length");
        mac.update(payload.as_bytes());
        // Constant-time compare via the MAC's own verify.
        match hex_decode(tag) {
            Some(bytes) => mac.verify_slice(&bytes).is_ok(),
            None => false,
        }
    }

    async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
        let session = session.ok_or_else(|| McpError::tool_execution("session required"))?;
        if let Some(state) = session.mrtr_request_state() {
            if !Self::verify(&state) {
                // -32602: the client sent a requestState we did not issue.
                return Err(McpError::param_out_of_range(
                    "requestState",
                    &state,
                    "a requestState issued by this server (integrity check failed)",
                ));
            }
            return Ok("State verified".to_string());
        }
        let schema = ElicitationSchema::new()
            .with_property("answer".to_string(), PrimitiveSchemaDefinition::string());
        let mut requests = InputRequests::new();
        requests.insert(
            "user_name".to_string(),
            InputRequest::Elicit(ElicitRequest::new_form("What is the answer?", schema)),
        );
        Err(McpError::InputRequired {
            input_requests: Some(requests),
            request_state: Some(Self::sign("tampered-state-payload")),
        })
    }
}

fn hex_lower(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

/// `input-required-result-non-tool-request`: MRTR is not tools-only — a
/// PROMPT must be able to return `InputRequiredResult` too. The harness asks
/// for a prompt of this name (`expected existing prompt name`), not a tool.
///
/// On the retry leg the responses arrive in the render args under the
/// reserved `io.modelcontextprotocol/inputResponses` key rather than through
/// `SessionContext`, because `render` takes no session.
struct InputRequiredPrompt;

impl HasPromptMetadata for InputRequiredPrompt {
    fn name(&self) -> &str {
        "test_input_required_result_prompt"
    }
}
impl HasPromptDescription for InputRequiredPrompt {
    fn description(&self) -> Option<&str> {
        Some("A prompt that requires input before rendering")
    }
}
impl HasPromptArguments for InputRequiredPrompt {}
impl HasPromptAnnotations for InputRequiredPrompt {}
impl HasPromptMeta for InputRequiredPrompt {}
impl HasIcons for InputRequiredPrompt {}

#[async_trait]
impl McpPrompt for InputRequiredPrompt {
    async fn render(
        &self,
        args: Option<HashMap<String, serde_json::Value>>,
    ) -> McpResult<Vec<PromptMessage>> {
        if let Some(responses) = args
            .as_ref()
            .and_then(|a| a.get("io.modelcontextprotocol/inputResponses"))
        {
            let answer = responses
                .pointer("/q1/content/answer")
                .and_then(|v| v.as_str())
                .unwrap_or("<no answer>");
            return Ok(vec![PromptMessage::user_text(format!(
                "Prompt received input: {answer}"
            ))]);
        }
        let schema = ElicitationSchema::new()
            .with_property("answer".to_string(), PrimitiveSchemaDefinition::string());
        let mut requests = InputRequests::new();
        requests.insert(
            "q1".to_string(),
            InputRequest::Elicit(ElicitRequest::new_form("What is the answer?", schema)),
        );
        Err(McpError::InputRequired {
            input_requests: Some(requests),
            request_state: Some("prompt-state-1".to_string()),
        })
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
        .tool(InputRequiredElicitationTool::default())
        .tool(InputRequiredListRootsTool::default())
        .tool(InputRequiredSamplingTool::default())
        .tool(InputRequiredMultipleInputsTool::default())
        .tool(InputRequiredRequestStateTool::default())
        .tool(InputRequiredCapabilitiesTool::default())
        .tool(InputRequiredMultiRoundTool::default())
        .tool(InputRequiredTamperedStateTool::default())
        .prompt(InputRequiredPrompt)
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
