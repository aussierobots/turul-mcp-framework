//! # MCP Prompts Test Server
//!
//! Comprehensive test server providing various types of prompts for E2E testing.
//! This server implements all MCP prompt patterns and edge cases to validate
//! framework compliance with the MCP 2025-06-18 specification.

use std::collections::HashMap;

use async_trait::async_trait;
use chrono::Utc;
use clap::Parser;
use serde_json::Value;
use tracing::info;
use turul_mcp_protocol::prompts::{PromptAnnotations, PromptArgument, PromptMessage};
use turul_mcp_protocol::McpError;
use turul_mcp_builders::prelude::*;  // HasPromptMetadata, HasPromptDescription, etc.
use turul_mcp_server::prompt::McpPrompt;
use turul_mcp_server::{McpResult, McpServer};

#[derive(Parser)]
#[command(name = "prompts-test-server")]
#[command(about = "MCP Prompts Test Server - Comprehensive test prompts for E2E validation")]
struct Args {
    /// Port to run the server on (0 = random port)
    #[arg(short, long, default_value = "0")]
    port: u16,
}

// =============================================================================
// Basic Prompts (Coverage)
// =============================================================================

/// Simple prompt with no arguments and fixed messages
#[derive(Clone, Default)]
struct SimplePrompt;

impl HasPromptMetadata for SimplePrompt {
    fn name(&self) -> &str {
        "simple_prompt"
    }
}

impl HasPromptDescription for SimplePrompt {
    fn description(&self) -> Option<&str> {
        Some("Simple prompt with no arguments and fixed messages")
    }
}

impl HasPromptArguments for SimplePrompt {
    fn arguments(&self) -> Option<&Vec<PromptArgument>> {
        None
    }
}

impl HasPromptAnnotations for SimplePrompt {
    fn annotations(&self) -> Option<&PromptAnnotations> {
        None
    }
}

impl HasPromptMeta for SimplePrompt {}
impl HasIcons for SimplePrompt {}

#[async_trait]
impl McpPrompt for SimplePrompt {
    async fn render(&self, _args: Option<HashMap<String, Value>>) -> McpResult<Vec<PromptMessage>> {
        Ok(vec![
            PromptMessage::user_text("This is a simple prompt with no arguments. Please respond as a helpful AI assistant for testing purposes."),
        ])
    }
}

/// String arguments prompt with required and optional string arguments
#[derive(Clone)]
struct StringArgsPrompt {
    arguments: Vec<PromptArgument>,
}

impl Default for StringArgsPrompt {
    fn default() -> Self {
        let arguments = vec![
            PromptArgument {
                name: "required_text".to_string(),
                title: None,
                description: Some("A required string argument".to_string()),
                required: Some(true),
            },
            PromptArgument {
                name: "optional_text".to_string(),
                title: None,
                description: Some("An optional string argument".to_string()),
                required: Some(false),
            },
        ];
        Self { arguments }
    }
}

impl HasPromptMetadata for StringArgsPrompt {
    fn name(&self) -> &str {
        "string_args_prompt"
    }
}

impl HasPromptDescription for StringArgsPrompt {
    fn description(&self) -> Option<&str> {
        Some("Prompt with required and optional string arguments")
    }
}

impl HasPromptArguments for StringArgsPrompt {
    fn arguments(&self) -> Option<&Vec<PromptArgument>> {
        Some(&self.arguments)
    }
}

impl HasPromptAnnotations for StringArgsPrompt {
    fn annotations(&self) -> Option<&PromptAnnotations> {
        None
    }
}

impl HasPromptMeta for StringArgsPrompt {}
impl HasIcons for StringArgsPrompt {}

#[async_trait]
impl McpPrompt for StringArgsPrompt {
    async fn render(&self, args: Option<HashMap<String, Value>>) -> McpResult<Vec<PromptMessage>> {
        let args = args.unwrap_or_default();

        // Validate required arguments
        let required_text = args
            .get("required_text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("required_text"))?;

        let optional_text = args
            .get("optional_text")
            .and_then(|v| v.as_str())
            .unwrap_or("(not provided)");

        Ok(vec![
            PromptMessage::user_text(format!(
                "You are processing a prompt with string arguments.\n\nRequired text: '{}'\nOptional text: '{}'\n\nPlease analyze these inputs.",
                required_text, optional_text
            )),
        ])
    }
}

/// Number arguments prompt with number validation
#[derive(Clone)]
struct NumberArgsPrompt {
    arguments: Vec<PromptArgument>,
}

impl Default for NumberArgsPrompt {
    fn default() -> Self {
        let arguments = vec![
            PromptArgument {
                name: "count".to_string(),
                title: None,
                description: Some("A number between 1 and 100".to_string()),
                required: Some(true),
            },
            PromptArgument {
                name: "multiplier".to_string(),
                title: None,
                description: Some("Optional multiplier (default: 1.0)".to_string()),
                required: Some(false),
            },
        ];
        Self { arguments }
    }
}

impl HasPromptMetadata for NumberArgsPrompt {
    fn name(&self) -> &str {
        "number_args_prompt"
    }
}

impl HasPromptDescription for NumberArgsPrompt {
    fn description(&self) -> Option<&str> {
        Some("Prompt with number argument validation")
    }
}

impl HasPromptArguments for NumberArgsPrompt {
    fn arguments(&self) -> Option<&Vec<PromptArgument>> {
        Some(&self.arguments)
    }
}

impl HasPromptAnnotations for NumberArgsPrompt {
    fn annotations(&self) -> Option<&PromptAnnotations> {
        None
    }
}

impl HasPromptMeta for NumberArgsPrompt {}
impl HasIcons for NumberArgsPrompt {}

#[async_trait]
impl McpPrompt for NumberArgsPrompt {
    async fn render(&self, args: Option<HashMap<String, Value>>) -> McpResult<Vec<PromptMessage>> {
        let args = args.unwrap_or_default();

        // Debug: Log received arguments
        tracing::info!("NumberArgsPrompt received arguments: {:?}", args);

        // Validate required number argument - MCP spec requires string arguments
        let count = args
            .get("count")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("count"))
            .and_then(|s| {
                s.parse::<f64>()
                    .map_err(|_| McpError::invalid_param_type("count", "number as string", s))
            })?;

        if !(1.0..=100.0).contains(&count) {
            return Err(McpError::param_out_of_range(
                "count",
                &count.to_string(),
                "1-100",
            ));
        }

        let multiplier = args
            .get("multiplier")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(1.0);

        let result = count * multiplier;

        Ok(vec![
            PromptMessage::user_text(format!(
                "You are processing a prompt with number validation.\n\nCount: {}\nMultiplier: {}\nResult: {}\nPlease analyze these numbers.",
                count, multiplier, result
            )),
        ])
    }
}

/// Boolean arguments prompt with boolean handling
#[derive(Clone)]
struct BooleanArgsPrompt {
    arguments: Vec<PromptArgument>,
}

impl Default for BooleanArgsPrompt {
    fn default() -> Self {
        let arguments = vec![
            PromptArgument {
                name: "enable_feature".to_string(),
                title: None,
                description: Some("Whether to enable the feature".to_string()),
                required: Some(true),
            },
            PromptArgument {
                name: "debug_mode".to_string(),
                title: None,
                description: Some("Optional debug mode flag".to_string()),
                required: Some(false),
            },
        ];
        Self { arguments }
    }
}

impl HasPromptMetadata for BooleanArgsPrompt {
    fn name(&self) -> &str {
        "boolean_args_prompt"
    }
}

impl HasPromptDescription for BooleanArgsPrompt {
    fn description(&self) -> Option<&str> {
        Some("Prompt with boolean argument handling")
    }
}

impl HasPromptArguments for BooleanArgsPrompt {
    fn arguments(&self) -> Option<&Vec<PromptArgument>> {
        Some(&self.arguments)
    }
}

impl HasPromptAnnotations for BooleanArgsPrompt {
    fn annotations(&self) -> Option<&PromptAnnotations> {
        None
    }
}

impl HasPromptMeta for BooleanArgsPrompt {}
impl HasIcons for BooleanArgsPrompt {}

#[async_trait]
impl McpPrompt for BooleanArgsPrompt {
    async fn render(&self, args: Option<HashMap<String, Value>>) -> McpResult<Vec<PromptMessage>> {
        let args = args.unwrap_or_default();

        // Validate required boolean argument - MCP spec requires string arguments
        let enable_feature = args
            .get("enable_feature")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("enable_feature"))
            .and_then(|s| {
                s.parse::<bool>().map_err(|_| {
                    McpError::invalid_param_type("enable_feature", "boolean as string", s)
                })
            })?;

        let debug_mode = args
            .get("debug_mode")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<bool>().ok())
            .unwrap_or(false);

        let status = if enable_feature {
            "ENABLED"
        } else {
            "DISABLED"
        };
        let debug_status = if debug_mode { "ON" } else { "OFF" };

        Ok(vec![
            PromptMessage::user_text(format!(
                "You are processing a prompt with boolean flags.\n\nFeature Status: {}\nDebug Mode: {}\n\nPlease provide guidance based on these settings.",
                status, debug_status
            )),
        ])
    }
}

/// Template prompt with variable substitution
#[derive(Clone)]
struct TemplatePrompt {
    arguments: Vec<PromptArgument>,
}

impl Default for TemplatePrompt {
    fn default() -> Self {
        let arguments = vec![
            PromptArgument {
                name: "name".to_string(),
                title: None,
                description: Some("Name to substitute in template".to_string()),
                required: Some(true),
            },
            PromptArgument {
                name: "topic".to_string(),
                title: None,
                description: Some("Topic to discuss".to_string()),
                required: Some(true),
            },
            PromptArgument {
                name: "style".to_string(),
                title: None,
                description: Some("Communication style (formal/casual)".to_string()),
                required: Some(false),
            },
        ];
        Self { arguments }
    }
}

impl HasPromptMetadata for TemplatePrompt {
    fn name(&self) -> &str {
        "template_prompt"
    }
}

impl HasPromptDescription for TemplatePrompt {
    fn description(&self) -> Option<&str> {
        Some("Prompt with template substitution using variables")
    }
}

impl HasPromptArguments for TemplatePrompt {
    fn arguments(&self) -> Option<&Vec<PromptArgument>> {
        Some(&self.arguments)
    }
}

impl HasPromptAnnotations for TemplatePrompt {
    fn annotations(&self) -> Option<&PromptAnnotations> {
        None
    }
}

impl HasPromptMeta for TemplatePrompt {}
impl HasIcons for TemplatePrompt {}

#[async_trait]
impl McpPrompt for TemplatePrompt {
    async fn render(&self, args: Option<HashMap<String, Value>>) -> McpResult<Vec<PromptMessage>> {
        let args = args.unwrap_or_default();

        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("name"))?;

        let topic = args
            .get("topic")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("topic"))?;

        let style = args
            .get("style")
            .and_then(|v| v.as_str())
            .unwrap_or("casual");

        let greeting = if style == "formal" {
            format!("Dear {},", name)
        } else {
            format!("Hi {},", name)
        };

        let tone = if style == "formal" {
            "Please provide a comprehensive analysis of"
        } else {
            "Let's talk about"
        };

        Ok(vec![
            PromptMessage::user_text(format!(
                "You are communicating in a {} style with {}. Adapt your response accordingly.\n\n{}\n\n{} {}. Please share your thoughts and insights.",
                style, name, greeting, tone, topic
            )),
        ])
    }
}

/// Multi-message prompt returning user and assistant messages
#[derive(Clone)]
struct MultiMessagePrompt {
    arguments: Vec<PromptArgument>,
}

impl Default for MultiMessagePrompt {
    fn default() -> Self {
        let arguments = vec![PromptArgument {
            name: "scenario".to_string(),
            title: None,
            description: Some("Scenario to create multi-turn conversation for".to_string()),
            required: Some(true),
        }];
        Self { arguments }
    }
}

impl HasPromptMetadata for MultiMessagePrompt {
    fn name(&self) -> &str {
        "multi_message_prompt"
    }
}

impl HasPromptDescription for MultiMessagePrompt {
    fn description(&self) -> Option<&str> {
        Some("Prompt returning user and assistant messages")
    }
}

impl HasPromptArguments for MultiMessagePrompt {
    fn arguments(&self) -> Option<&Vec<PromptArgument>> {
        Some(&self.arguments)
    }
}

impl HasPromptAnnotations for MultiMessagePrompt {
    fn annotations(&self) -> Option<&PromptAnnotations> {
        None
    }
}

impl HasPromptMeta for MultiMessagePrompt {}
impl HasIcons for MultiMessagePrompt {}

#[async_trait]
impl McpPrompt for MultiMessagePrompt {
    async fn render(&self, args: Option<HashMap<String, Value>>) -> McpResult<Vec<PromptMessage>> {
        let args = args.unwrap_or_default();

        let scenario = args
            .get("scenario")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("scenario"))?;

        Ok(vec![
            PromptMessage::user_text(format!(
                "I'm interested in learning about {}. Can you give me an overview?",
                scenario
            )),
            PromptMessage::assistant_text(format!(
                "I'd be happy to help you explore {}! This is a fascinating topic with many aspects to consider. Let me start with a foundational overview, and then we can dive deeper into specific areas that interest you most.",
                scenario
            )),
            PromptMessage::user_text("That sounds great! What are the key concepts I should understand first?"),
        ])
    }
}

// =============================================================================
// Advanced Prompts (Features)
// =============================================================================

/// Session-aware prompt that uses session context
#[derive(Clone, Default)]
struct SessionAwarePrompt;

impl HasPromptMetadata for SessionAwarePrompt {
    fn name(&self) -> &str {
        "session_aware_prompt"
    }
}

impl HasPromptDescription for SessionAwarePrompt {
    fn description(&self) -> Option<&str> {
        Some("Session-aware prompt that uses session context in messages")
    }
}

impl HasPromptArguments for SessionAwarePrompt {
    fn arguments(&self) -> Option<&Vec<PromptArgument>> {
        None
    }
}

impl HasPromptAnnotations for SessionAwarePrompt {
    fn annotations(&self) -> Option<&PromptAnnotations> {
        None
    }
}

impl HasPromptMeta for SessionAwarePrompt {}
impl HasIcons for SessionAwarePrompt {}

#[async_trait]
impl McpPrompt for SessionAwarePrompt {
    async fn render(&self, _args: Option<HashMap<String, Value>>) -> McpResult<Vec<PromptMessage>> {
        // In a real implementation, this would access session context
        // For testing, we'll simulate session awareness
        let session_info = format!(
            "Session ID: example-session-{}\nTimestamp: {}",
            "12345",
            Utc::now().to_rfc3339()
        );

        Ok(vec![
            PromptMessage::user_text(format!(
                "You are a session-aware AI assistant. You have access to session information for personalized responses.\n\nThis prompt is aware of the current session:\n{}\n\nPlease acknowledge the session context.",
                session_info
            )),
        ])
    }
}

/// Validation prompt with strict argument validation
#[derive(Clone)]
struct ValidationPrompt {
    arguments: Vec<PromptArgument>,
}

impl Default for ValidationPrompt {
    fn default() -> Self {
        let arguments = vec![
            PromptArgument {
                name: "email".to_string(),
                title: None,
                description: Some("Valid email address".to_string()),
                required: Some(true),
            },
            PromptArgument {
                name: "age".to_string(),
                title: None,
                description: Some("Age between 18 and 120".to_string()),
                required: Some(true),
            },
        ];
        Self { arguments }
    }
}

impl HasPromptMetadata for ValidationPrompt {
    fn name(&self) -> &str {
        "validation_prompt"
    }
}

impl HasPromptDescription for ValidationPrompt {
    fn description(&self) -> Option<&str> {
        Some("Prompt with strict argument validation and detailed errors")
    }
}

impl HasPromptArguments for ValidationPrompt {
    fn arguments(&self) -> Option<&Vec<PromptArgument>> {
        Some(&self.arguments)
    }
}

impl HasPromptAnnotations for ValidationPrompt {
    fn annotations(&self) -> Option<&PromptAnnotations> {
        None
    }
}

impl HasPromptMeta for ValidationPrompt {}
impl HasIcons for ValidationPrompt {}

#[async_trait]
impl McpPrompt for ValidationPrompt {
    async fn render(&self, args: Option<HashMap<String, Value>>) -> McpResult<Vec<PromptMessage>> {
        let args = args.unwrap_or_default();

        // Strict email validation
        let email = args
            .get("email")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("email"))?;

        if !email.contains('@') || !email.contains('.') {
            return Err(McpError::invalid_param_type(
                "email",
                "valid email address",
                email,
            ));
        }

        // Strict age validation - MCP spec requires string arguments
        let age = args
            .get("age")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("age"))
            .and_then(|s| {
                s.parse::<f64>()
                    .map_err(|_| McpError::invalid_param_type("age", "number as string", s))
            })?;

        if !(18.0..=120.0).contains(&age) {
            return Err(McpError::param_out_of_range(
                "age",
                &age.to_string(),
                "18-120",
            ));
        }

        Ok(vec![
            PromptMessage::user_text(format!(
                "You are processing validated user information with strict validation.\n\nValidated Information:\nEmail: {}\nAge: {}\n\nThis information has passed strict validation checks.",
                email, age as u32
            )),
        ])
    }
}

/// Dynamic prompt that changes behavior based on arguments
#[derive(Clone)]
struct DynamicPrompt {
    arguments: Vec<PromptArgument>,
}

impl Default for DynamicPrompt {
    fn default() -> Self {
        let arguments = vec![
            PromptArgument {
                name: "mode".to_string(),
                title: None,
                description: Some("Prompt mode: creative, analytical, or supportive".to_string()),
                required: Some(true),
            },
            PromptArgument {
                name: "content".to_string(),
                title: None,
                description: Some("Content to process".to_string()),
                required: Some(true),
            },
        ];
        Self { arguments }
    }
}

impl HasPromptMetadata for DynamicPrompt {
    fn name(&self) -> &str {
        "dynamic_prompt"
    }
}

impl HasPromptDescription for DynamicPrompt {
    fn description(&self) -> Option<&str> {
        Some("Dynamic prompt that changes behavior based on mode argument")
    }
}

impl HasPromptArguments for DynamicPrompt {
    fn arguments(&self) -> Option<&Vec<PromptArgument>> {
        Some(&self.arguments)
    }
}

impl HasPromptAnnotations for DynamicPrompt {
    fn annotations(&self) -> Option<&PromptAnnotations> {
        None
    }
}

impl HasPromptMeta for DynamicPrompt {}
impl HasIcons for DynamicPrompt {}

#[async_trait]
impl McpPrompt for DynamicPrompt {
    async fn render(&self, args: Option<HashMap<String, Value>>) -> McpResult<Vec<PromptMessage>> {
        let args = args.unwrap_or_default();

        let mode = args
            .get("mode")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("mode"))?;

        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("content"))?;

        let (persona, approach) = match mode {
            "creative" => (
                "You are a creative AI assistant. Approach tasks with imagination, originality, and artistic flair.",
                "Think outside the box and explore creative possibilities"
            ),
            "analytical" => (
                "You are an analytical AI assistant. Approach tasks with logic, data-driven reasoning, and systematic analysis.",
                "Break down the problem systematically and provide evidence-based insights"
            ),
            "supportive" => (
                "You are a supportive AI assistant. Approach tasks with empathy, encouragement, and understanding.",
                "Provide emotional support and constructive guidance"
            ),
            _ => {
                return Err(McpError::invalid_param_type(
                    "mode",
                    "creative, analytical, or supportive",
                    mode
                ));
            }
        };

        Ok(vec![
            PromptMessage::user_text(format!(
                "{}\n\nMode: {} - {}\n\nContent to process:\n{}\n\nPlease respond according to the selected mode.",
                persona, mode.to_uppercase(), approach, content
            )),
        ])
    }
}

// =============================================================================
// Edge Case Prompts (simplified for brevity)
// =============================================================================

/// Prompt that returns empty messages array
#[derive(Clone, Default)]
struct EmptyMessagesPrompt;

impl HasPromptMetadata for EmptyMessagesPrompt {
    fn name(&self) -> &str {
        "empty_messages_prompt"
    }
}

impl HasPromptDescription for EmptyMessagesPrompt {
    fn description(&self) -> Option<&str> {
        Some("Edge case prompt that returns empty messages array")
    }
}

impl HasPromptArguments for EmptyMessagesPrompt {
    fn arguments(&self) -> Option<&Vec<PromptArgument>> {
        None
    }
}

impl HasPromptAnnotations for EmptyMessagesPrompt {
    fn annotations(&self) -> Option<&PromptAnnotations> {
        None
    }
}

impl HasPromptMeta for EmptyMessagesPrompt {}
impl HasIcons for EmptyMessagesPrompt {}

#[async_trait]
impl McpPrompt for EmptyMessagesPrompt {
    async fn render(&self, _args: Option<HashMap<String, Value>>) -> McpResult<Vec<PromptMessage>> {
        Ok(vec![]) // Return empty messages array for edge case testing
    }
}

/// Prompt that always fails validation
#[derive(Clone)]
struct ValidationFailurePrompt {
    arguments: Vec<PromptArgument>,
}

impl Default for ValidationFailurePrompt {
    fn default() -> Self {
        let arguments = vec![PromptArgument {
            name: "impossible_param".to_string(),
            title: None,
            description: Some("This parameter can never be satisfied".to_string()),
            required: Some(true),
        }];
        Self { arguments }
    }
}

impl HasPromptMetadata for ValidationFailurePrompt {
    fn name(&self) -> &str {
        "validation_failure_prompt"
    }
}

impl HasPromptDescription for ValidationFailurePrompt {
    fn description(&self) -> Option<&str> {
        Some("Edge case prompt that always fails validation with specific errors")
    }
}

impl HasPromptArguments for ValidationFailurePrompt {
    fn arguments(&self) -> Option<&Vec<PromptArgument>> {
        Some(&self.arguments)
    }
}

impl HasPromptAnnotations for ValidationFailurePrompt {
    fn annotations(&self) -> Option<&PromptAnnotations> {
        None
    }
}

impl HasPromptMeta for ValidationFailurePrompt {}
impl HasIcons for ValidationFailurePrompt {}

#[async_trait]
impl McpPrompt for ValidationFailurePrompt {
    async fn render(&self, args: Option<HashMap<String, Value>>) -> McpResult<Vec<PromptMessage>> {
        let _args = args.unwrap_or_default();

        // This prompt always fails for testing error handling
        Err(McpError::invalid_param_type(
            "impossible_param",
            "a value that cannot exist",
            "any provided value",
        ))
    }
}

// =============================================================================
// Main Server
// =============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let args = Args::parse();

    // Use specified port or OS ephemeral allocation if 0
    let port = if args.port == 0 {
        // Use OS ephemeral port allocation - more reliable than portpicker
        let listener = std::net::TcpListener::bind("127.0.0.1:0")
            .map_err(|e| format!("Failed to bind to ephemeral port: {}", e))?;
        let port = listener.local_addr()?.port();
        drop(listener); // Release immediately so server can bind to it
        port
    } else {
        args.port
    };

    info!("🚀 Starting MCP Prompts Test Server on port {}", port);

    let server = McpServer::builder()
        .name("prompts-test-server")
        .version("0.2.0")
        .title("MCP Prompts Test Server")
        .instructions(
            "Comprehensive test server providing various types of prompts for E2E testing.\n\
            This server implements all MCP prompt patterns and edge cases to validate\n\
            framework compliance with the MCP 2025-06-18 specification.\n\n\
            Available test prompts:\n\
            • Basic: simple, string_args, number_args, boolean_args, template, multi_message\n\
            • Advanced: session_aware, validation, dynamic\n\
            • Edge cases: empty_messages, validation_failure",
        )
        // Basic Prompts (Coverage)
        .prompt(SimplePrompt)
        .prompt(StringArgsPrompt::default())
        .prompt(NumberArgsPrompt::default())
        .prompt(BooleanArgsPrompt::default())
        .prompt(TemplatePrompt::default())
        .prompt(MultiMessagePrompt::default())
        // Advanced Prompts (Features)
        .prompt(SessionAwarePrompt)
        .prompt(ValidationPrompt::default())
        .prompt(DynamicPrompt::default())
        // Edge Case Prompts
        .prompt(EmptyMessagesPrompt)
        .prompt(ValidationFailurePrompt::default())
        .with_prompts()
        .bind_address(format!("127.0.0.1:{}", port).parse()?)
        .build()?;

    info!("📡 Server URL: http://127.0.0.1:{}/mcp", port);
    info!("");
    info!("🧪 Test Prompts Available:");
    info!("   📝 Basic Prompts (Coverage):");
    info!("      • simple_prompt - No arguments, fixed messages");
    info!("      • string_args_prompt - Required and optional string arguments");
    info!("      • number_args_prompt - Number validation (1-100)");
    info!("      • boolean_args_prompt - Boolean flag handling");
    info!("      • template_prompt - Variable substitution in messages");
    info!("      • multi_message_prompt - User and assistant messages");
    info!("");
    info!("   🚀 Advanced Prompts (Features):");
    info!("      • session_aware_prompt - Uses session context in messages");
    info!("      • validation_prompt - Strict email/age validation with detailed errors");
    info!(
        "      • dynamic_prompt - Behavior changes based on mode (creative/analytical/supportive)"
    );
    info!("");
    info!("   ⚠️  Edge Case Prompts:");
    info!("      • empty_messages_prompt - Returns empty messages array");
    info!("      • validation_failure_prompt - Always fails validation for error testing");
    info!("");
    info!("💡 Quick Test Commands:");
    info!("   curl -X POST http://127.0.0.1:{}/mcp \\", port);
    info!("     -H 'Content-Type: application/json' \\");
    info!("     -d '{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{{\"protocolVersion\":\"2025-06-18\",\"capabilities\":{{}},\"clientInfo\":{{\"name\":\"test\",\"version\":\"1.0\"}}}}}}'");
    info!("");
    info!("   curl -X POST http://127.0.0.1:{}/mcp \\", port);
    info!("     -H 'Content-Type: application/json' \\");
    info!("     -H 'Mcp-Session-Id: SESSION_ID' \\");
    info!("     -d '{{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"prompts/list\",\"params\":{{}}}}'");
    info!("");

    server.run().await?;
    Ok(())
}
