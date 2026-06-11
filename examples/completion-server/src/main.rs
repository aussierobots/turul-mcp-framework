//! # IDE Auto-Completion Server
//!
//! Demonstrates the REAL MCP completion protocol (`completion/complete`):
//! an `McpCompletion` provider registered via `.completion_provider()`
//! serves argument suggestions for the `code_review` prompt's `language`
//! argument — reference-matched, prefix-filtered, and capped per the spec.
//! A plain tool (`ide_completion`) is kept alongside to contrast the two
//! surfaces: `completion/complete` is for argument autocomplete while
//! editing a prompt/template; tools are for model-invoked actions.

use serde::Deserialize;
use serde_json::{Value, json};
use tracing::info;
use turul_mcp_derive::McpTool;
use turul_mcp_server::prelude::*;

/// IDE Auto-Completion Tool that provides intelligent suggestions
#[derive(McpTool, Clone, Default, Deserialize)]
#[tool(
    name = "ide_completion",
    description = "Provides intelligent auto-completion suggestions for programming languages, frameworks, commands, and file extensions"
)]
pub struct IdeCompletionTool {
    #[param(
        description = "Category of completions to get (language, framework, command, extension, all)"
    )]
    pub category: String,

    #[param(description = "Prefix to filter completions", optional)]
    pub prefix: Option<String>,
}

impl IdeCompletionTool {
    fn get_completions(&self, category: &str, prefix: &str) -> Vec<String> {
        let languages = vec![
            "rust",
            "python",
            "javascript",
            "typescript",
            "java",
            "go",
            "cpp",
            "c",
            "kotlin",
            "swift",
            "php",
            "ruby",
            "csharp",
        ];
        let frameworks = vec![
            "react", "vue", "angular", "express", "django", "flask", "spring", "rails", "laravel",
            "tokio", "actix", "axum",
        ];
        let commands = vec![
            "build", "test", "run", "deploy", "install", "update", "lint", "format", "check",
            "clean", "serve",
        ];
        let file_extensions = vec![
            ".rs", ".py", ".js", ".ts", ".java", ".go", ".cpp", ".c", ".json", ".yaml", ".toml",
            ".md",
        ];

        let prefix = prefix.to_lowercase();

        let source: Vec<&str> = match category {
            "language" => languages,
            "framework" => frameworks,
            "command" => commands,
            "extension" => file_extensions,
            "all" => {
                let mut all = Vec::new();
                all.extend(languages);
                all.extend(frameworks);
                all.extend(commands);
                all.extend(file_extensions);
                return all
                    .into_iter()
                    .filter(|item| item.to_lowercase().starts_with(&prefix))
                    .take(20)
                    .map(|s| s.to_string())
                    .collect();
            }
            _ => return vec![],
        };

        source
            .iter()
            .filter(|item| prefix.is_empty() || item.to_lowercase().starts_with(&prefix))
            .take(10)
            .map(|s| s.to_string())
            .collect()
    }

    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<Value> {
        let category = &self.category;
        let prefix = self.prefix.as_deref().unwrap_or("");

        let completions = self.get_completions(category, prefix);
        let count = completions.len();

        Ok(json!({
            "category": category,
            "prefix": prefix,
            "completions": completions,
            "count": count
        }))
    }
}

/// Real `completion/complete` provider: completes the `language` argument
/// of the `code_review` prompt (reference-matched by the routing handler).
struct LanguageCompleter;

impl turul_mcp_server::prelude::HasCompletionMetadata for LanguageCompleter {
    fn method(&self) -> &str {
        "completion/complete"
    }
    fn reference(&self) -> &turul_mcp_protocol::completion::CompletionReference {
        use std::sync::OnceLock;
        use turul_mcp_protocol::completion::{CompletionReference, PromptReference};
        static R: OnceLock<CompletionReference> = OnceLock::new();
        R.get_or_init(|| CompletionReference::Prompt(PromptReference::new("code_review")))
    }
}
impl turul_mcp_server::prelude::HasCompletionContext for LanguageCompleter {
    fn argument(&self) -> &turul_mcp_protocol::completion::CompleteArgument {
        use std::sync::OnceLock;
        use turul_mcp_protocol::completion::CompleteArgument;
        static A: OnceLock<CompleteArgument> = OnceLock::new();
        A.get_or_init(|| CompleteArgument::new("language", ""))
    }
}
impl turul_mcp_server::prelude::HasCompletionHandling for LanguageCompleter {}

#[async_trait::async_trait]
impl turul_mcp_server::McpCompletion for LanguageCompleter {
    async fn complete(
        &self,
        request: turul_mcp_protocol::completion::CompleteRequest,
    ) -> McpResult<turul_mcp_protocol::completion::CompleteResult> {
        use turul_mcp_protocol::completion::{CompleteResult, CompletionResult};
        const LANGUAGES: &[&str] = &[
            "c",
            "cpp",
            "csharp",
            "go",
            "java",
            "javascript",
            "kotlin",
            "python",
            "ruby",
            "rust",
            "scala",
            "swift",
            "typescript",
            "zig",
        ];
        let prefix = request.params.argument.value.to_lowercase();
        let values: Vec<String> = LANGUAGES
            .iter()
            .filter(|l| l.starts_with(&prefix))
            .map(|l| l.to_string())
            .collect();
        Ok(CompleteResult::new(CompletionResult::new(values)))
    }
}

/// The prompt whose `language` argument the completer serves.
struct CodeReviewPrompt;

impl turul_mcp_server::prelude::HasPromptMetadata for CodeReviewPrompt {
    fn name(&self) -> &str {
        "code_review"
    }
    fn title(&self) -> Option<&str> {
        Some("Code Review")
    }
}
impl turul_mcp_server::prelude::HasPromptDescription for CodeReviewPrompt {
    fn description(&self) -> Option<&str> {
        Some("Review code in a given language")
    }
}
impl turul_mcp_server::prelude::HasPromptArguments for CodeReviewPrompt {
    fn arguments(&self) -> Option<&Vec<turul_mcp_protocol::prompts::PromptArgument>> {
        use std::sync::OnceLock;
        use turul_mcp_protocol::prompts::PromptArgument;
        static ARGS: OnceLock<Vec<PromptArgument>> = OnceLock::new();
        Some(ARGS.get_or_init(|| {
            vec![PromptArgument::new("language").with_description("Programming language")]
        }))
    }
}
impl turul_mcp_server::prelude::HasPromptAnnotations for CodeReviewPrompt {}
impl turul_mcp_server::prelude::HasPromptMeta for CodeReviewPrompt {}
impl turul_mcp_server::prelude::HasIcons for CodeReviewPrompt {}

#[async_trait::async_trait]
impl turul_mcp_server::McpPrompt for CodeReviewPrompt {
    async fn render(
        &self,
        args: Option<std::collections::HashMap<String, Value>>,
    ) -> McpResult<Vec<turul_mcp_protocol::prompts::PromptMessage>> {
        let language = args
            .as_ref()
            .and_then(|a| a.get("language"))
            .and_then(|v| v.as_str())
            .unwrap_or("rust")
            .to_string();
        Ok(vec![turul_mcp_protocol::prompts::PromptMessage::user_text(
            format!("Please review the following {language} code for quality and safety."),
        )])
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting IDE Auto-Completion Server Example");

    let completion_tool = IdeCompletionTool::default();

    let server = McpServer::builder()
        .name("ide-completion-server")
        .version("1.0.0")
        .title("IDE Auto-Completion Server")
        .instructions("Provides intelligent auto-completion suggestions for developers. Use the ide_completion tool with category (language/framework/command/extension/all) and optional prefix parameters.")
        .tool(completion_tool)
        .prompt(CodeReviewPrompt)
        .completion_provider(LanguageCompleter)
        .bind_address("127.0.0.1:8042".parse()?)
        .build()?;

    info!("IDE completion server running at: http://127.0.0.1:8042/mcp");
    info!("Available completion categories:");
    info!("  - language: Programming language suggestions");
    info!("  - framework: Web and application framework suggestions");
    info!("  - command: Development command suggestions");
    info!("  - extension: File extension suggestions");
    info!("  - all: Combined suggestions from all categories");
    info!("Use prefix parameter to filter results");

    server.run().await?;
    Ok(())
}
