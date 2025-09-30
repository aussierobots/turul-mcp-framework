//! # IDE Auto-Completion Server Example
//!
//! This example demonstrates a simple MCP tool that provides intelligent
//! auto-completion suggestions for developers working in IDEs and code editors.
//! The server provides context-aware suggestions for programming languages,
//! frameworks, file extensions, and development commands.

use async_trait::async_trait;
use serde_json::{Value, json};
use std::collections::HashMap;
use tracing::info;
use turul_mcp_protocol::{
    McpError,
    schema::JsonSchema,
    tools::{
        CallToolResult, HasAnnotations, HasBaseMetadata, HasDescription, HasInputSchema,
        HasOutputSchema, HasToolMeta, ToolResult, ToolSchema,
    },
};
use turul_mcp_server::{McpResult, McpServer, McpTool, SessionContext};

/// IDE Auto-Completion Tool that provides intelligent suggestions
#[derive(Clone)]
pub struct IdeCompletionTool {
    input_schema: ToolSchema,
    languages: Vec<String>,
    frameworks: Vec<String>,
    commands: Vec<String>,
    file_extensions: Vec<String>,
}

impl Default for IdeCompletionTool {
    fn default() -> Self {
        Self::new()
    }
}

impl IdeCompletionTool {
    /// Create a new IDE completion tool with predefined data
    pub fn new() -> Self {
        let languages = vec![
            "rust".to_string(),
            "python".to_string(),
            "javascript".to_string(),
            "typescript".to_string(),
            "java".to_string(),
            "go".to_string(),
            "cpp".to_string(),
            "c".to_string(),
            "kotlin".to_string(),
            "swift".to_string(),
            "php".to_string(),
            "ruby".to_string(),
            "csharp".to_string(),
        ];

        let frameworks = vec![
            "react".to_string(),
            "vue".to_string(),
            "angular".to_string(),
            "express".to_string(),
            "django".to_string(),
            "flask".to_string(),
            "spring".to_string(),
            "rails".to_string(),
            "laravel".to_string(),
            "tokio".to_string(),
            "actix".to_string(),
            "axum".to_string(),
        ];

        let commands = vec![
            "build".to_string(),
            "test".to_string(),
            "run".to_string(),
            "deploy".to_string(),
            "install".to_string(),
            "update".to_string(),
            "lint".to_string(),
            "format".to_string(),
            "check".to_string(),
            "clean".to_string(),
            "serve".to_string(),
        ];

        let file_extensions = vec![
            ".rs".to_string(),
            ".py".to_string(),
            ".js".to_string(),
            ".ts".to_string(),
            ".java".to_string(),
            ".go".to_string(),
            ".cpp".to_string(),
            ".c".to_string(),
            ".json".to_string(),
            ".yaml".to_string(),
            ".toml".to_string(),
            ".md".to_string(),
        ];

        let input_schema = ToolSchema::object()
            .with_properties(HashMap::from([
                (
                    "category".to_string(),
                    JsonSchema::string_enum(vec![
                        "language".to_string(),
                        "framework".to_string(),
                        "command".to_string(),
                        "extension".to_string(),
                        "all".to_string(),
                    ])
                    .with_description("Category of completions to get"),
                ),
                (
                    "prefix".to_string(),
                    JsonSchema::string().with_description("Prefix to filter completions"),
                ),
            ]))
            .with_required(vec!["category".to_string()]);

        Self {
            input_schema,
            languages,
            frameworks,
            commands,
            file_extensions,
        }
    }

    fn get_completions(&self, category: &str, prefix: &str) -> Vec<String> {
        let prefix = prefix.to_lowercase();

        let source: &Vec<String> = match category {
            "language" => &self.languages,
            "framework" => &self.frameworks,
            "command" => &self.commands,
            "extension" => &self.file_extensions,
            "all" => {
                // Combine all categories
                let mut all = Vec::new();
                all.extend(self.languages.iter().cloned());
                all.extend(self.frameworks.iter().cloned());
                all.extend(self.commands.iter().cloned());
                all.extend(self.file_extensions.iter().cloned());
                return all
                    .into_iter()
                    .filter(|item| item.to_lowercase().starts_with(&prefix))
                    .take(20)
                    .collect();
            }
            _ => return vec![], // Invalid category
        };

        source
            .iter()
            .filter(|item| prefix.is_empty() || item.to_lowercase().starts_with(&prefix))
            .take(10)
            .cloned()
            .collect()
    }
}

// Implement fine-grained traits for ToolDefinition
impl HasBaseMetadata for IdeCompletionTool {
    fn name(&self) -> &str {
        "ide_completion"
    }
    fn title(&self) -> Option<&str> {
        Some("IDE Auto-Completion")
    }
}

impl HasDescription for IdeCompletionTool {
    fn description(&self) -> Option<&str> {
        Some(
            "Provides intelligent auto-completion suggestions for programming languages, frameworks, commands, and file extensions",
        )
    }
}

impl HasInputSchema for IdeCompletionTool {
    fn input_schema(&self) -> &ToolSchema {
        &self.input_schema
    }
}

impl HasOutputSchema for IdeCompletionTool {
    fn output_schema(&self) -> Option<&ToolSchema> {
        None
    }
}

impl HasAnnotations for IdeCompletionTool {
    fn annotations(&self) -> Option<&turul_mcp_protocol::tools::ToolAnnotations> {
        None
    }
}

impl HasToolMeta for IdeCompletionTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
        None
    }
}

#[async_trait]
impl McpTool for IdeCompletionTool {
    async fn call(
        &self,
        args: Value,
        _session: Option<SessionContext>,
    ) -> McpResult<CallToolResult> {
        let category = args
            .get("category")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("category"))?;

        let prefix = args.get("prefix").and_then(|v| v.as_str()).unwrap_or("");

        let completions = self.get_completions(category, prefix);

        let result = json!({
            "category": category,
            "prefix": prefix,
            "completions": completions,
            "count": completions.len()
        });

        Ok(CallToolResult::success(vec![
            ToolResult::text(format!(
                "Found {} completions for '{}' in category '{}'",
                completions.len(),
                prefix,
                category
            )),
            ToolResult::text(result.to_string()),
        ]))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting IDE Auto-Completion Server Example");

    let completion_tool = IdeCompletionTool::new();

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
