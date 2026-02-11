//! # IDE Auto-Completion Server Example
//!
//! This example demonstrates a simple MCP tool that provides intelligent
//! auto-completion suggestions for developers working in IDEs and code editors.
//! The server provides context-aware suggestions for programming languages,
//! frameworks, file extensions, and development commands.

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
