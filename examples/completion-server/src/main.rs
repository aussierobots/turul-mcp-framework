//! # IDE Auto-Completion Server Example
//!
//! This example demonstrates a real-world MCP completion server that provides intelligent
//! auto-completion suggestions for developers working in IDEs and code editors.
//! The server loads completion data from external JSON files and provides context-aware
//! suggestions for programming languages, frameworks, commands, and development tools.

use async_trait::async_trait;
use mcp_protocol::{
    McpError,
    completion::{CompleteRequest, CompletionResponse, CompletionSuggestion},
};
use mcp_server::{McpHandler, McpResult, McpServer};
use serde::{Deserialize, Serialize};
use serde_json::{Value, from_str};
use std::fs;
use std::path::Path;
use tracing::info;

#[derive(Debug, Deserialize, Serialize)]
struct Language {
    name: String,
    label: String,
    description: String,
    category: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Framework {
    name: String,
    label: String,
    description: String,
    category: String,
    language: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct DevelopmentCommand {
    name: String,
    label: String,
    description: String,
    category: String,
    common_tools: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct LanguageData {
    programming_languages: Vec<Language>,
}

#[derive(Debug, Deserialize)]
struct FrameworkData {
    web_frameworks: Vec<Framework>,
}

#[derive(Debug, Deserialize)]
struct CommandData {
    development_commands: Vec<DevelopmentCommand>,
}

/// IDE Auto-Completion Handler that loads data from external files
/// Real-world use case: Provides intelligent auto-completion for developers in IDEs and editors
pub struct IdeCompletionHandler {
    languages: Vec<Language>,
    frameworks: Vec<Framework>,
    commands: Vec<DevelopmentCommand>,
    file_extensions: Vec<String>,
}

impl IdeCompletionHandler {
    /// Load completion data from external JSON files
    /// Real-world pattern: Configuration and data externalization for maintainability
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let data_dir = Path::new("data");

        // Load programming languages
        let languages = Self::load_languages(data_dir)?;

        // Load web frameworks
        let frameworks = Self::load_frameworks(data_dir)?;

        // Load development commands
        let commands = Self::load_commands(data_dir)?;

        // Common file extensions (could also be externalized)
        let file_extensions = vec![
            ".rs".to_string(),
            ".py".to_string(),
            ".js".to_string(),
            ".ts".to_string(),
            ".java".to_string(),
            ".go".to_string(),
            ".cpp".to_string(),
            ".c".to_string(),
            ".kt".to_string(),
            ".swift".to_string(),
            ".php".to_string(),
            ".rb".to_string(),
            ".json".to_string(),
            ".yaml".to_string(),
            ".toml".to_string(),
            ".md".to_string(),
            ".txt".to_string(),
            ".html".to_string(),
            ".css".to_string(),
            ".scss".to_string(),
            ".sql".to_string(),
        ];

        info!(
            "Loaded {} languages, {} frameworks, {} commands",
            languages.len(),
            frameworks.len(),
            commands.len()
        );

        Ok(Self {
            languages,
            frameworks,
            commands,
            file_extensions,
        })
    }

    fn load_languages(data_dir: &Path) -> Result<Vec<Language>, Box<dyn std::error::Error>> {
        let file_path = data_dir.join("languages.json");
        let content = fs::read_to_string(file_path)?;
        let data: LanguageData = from_str(&content)?;
        Ok(data.programming_languages)
    }

    fn load_frameworks(data_dir: &Path) -> Result<Vec<Framework>, Box<dyn std::error::Error>> {
        let file_path = data_dir.join("frameworks.json");
        let content = fs::read_to_string(file_path)?;
        let data: FrameworkData = from_str(&content)?;
        Ok(data.web_frameworks)
    }

    fn load_commands(
        data_dir: &Path,
    ) -> Result<Vec<DevelopmentCommand>, Box<dyn std::error::Error>> {
        let file_path = data_dir.join("development_commands.json");
        let content = fs::read_to_string(file_path)?;
        let data: CommandData = from_str(&content)?;
        Ok(data.development_commands)
    }

    fn get_language_completions(&self, prefix: &str) -> Vec<CompletionSuggestion> {
        self.languages
            .iter()
            .filter(|lang| lang.name.starts_with(prefix))
            .map(|lang| CompletionSuggestion {
                value: lang.name.clone(),
                label: Some(lang.label.clone()),
                description: Some(format!("{} ({})", lang.description, lang.category)),
                annotations: None,
            })
            .collect()
    }

    fn get_extension_completions(&self, prefix: &str) -> Vec<CompletionSuggestion> {
        self.file_extensions
            .iter()
            .filter(|ext| ext.starts_with(prefix))
            .map(|ext| CompletionSuggestion {
                value: ext.clone(),
                label: Some(format!("{} file", ext)),
                description: Some(
                    match ext.as_str() {
                        ".rs" => "Rust source file",
                        ".py" => "Python script file",
                        ".js" => "JavaScript file",
                        ".ts" => "TypeScript file",
                        ".json" => "JSON data file",
                        ".yaml" => "YAML configuration file",
                        ".md" => "Markdown documentation",
                        _ => "File extension",
                    }
                    .to_string(),
                ),
                annotations: None,
            })
            .collect()
    }

    fn get_command_completions(&self, prefix: &str) -> Vec<CompletionSuggestion> {
        self.commands
            .iter()
            .filter(|cmd| cmd.name.starts_with(prefix))
            .map(|cmd| CompletionSuggestion {
                value: cmd.name.clone(),
                label: Some(cmd.label.clone()),
                description: Some(format!(
                    "{} - Tools: {}",
                    cmd.description,
                    cmd.common_tools.join(", ")
                )),
                annotations: None,
            })
            .collect()
    }

    fn get_framework_completions(&self, prefix: &str) -> Vec<CompletionSuggestion> {
        self.frameworks
            .iter()
            .filter(|fw| fw.name.starts_with(prefix))
            .map(|fw| CompletionSuggestion {
                value: fw.name.clone(),
                label: Some(fw.label.clone()),
                description: Some(format!(
                    "{} ({} - {})",
                    fw.description, fw.language, fw.category
                )),
                annotations: None,
            })
            .collect()
    }

    fn get_smart_completions(
        &self,
        argument_name: &str,
        current_value: &str,
    ) -> Vec<CompletionSuggestion> {
        let prefix = current_value.to_lowercase();

        // Context-aware completion based on argument name
        match argument_name.to_lowercase().as_str() {
            "language" | "lang" | "programming_language" => self.get_language_completions(&prefix),
            "extension" | "ext" | "file_extension" => self.get_extension_completions(&prefix),
            "command" | "cmd" | "action" => self.get_command_completions(&prefix),
            "framework" | "library" | "lib" => self.get_framework_completions(&prefix),
            "filename" | "file" | "path" => {
                // Suggest common file patterns
                let mut suggestions = Vec::new();
                if prefix.is_empty() || "main".starts_with(&prefix) {
                    suggestions.push(CompletionSuggestion {
                        value: "main.rs".to_string(),
                        label: Some("main.rs".to_string()),
                        description: Some("Main Rust source file".to_string()),
                        annotations: None,
                    });
                }
                if prefix.is_empty() || "readme".starts_with(&prefix) {
                    suggestions.push(CompletionSuggestion {
                        value: "README.md".to_string(),
                        label: Some("README.md".to_string()),
                        description: Some("Project documentation file".to_string()),
                        annotations: None,
                    });
                }
                if prefix.is_empty() || "cargo".starts_with(&prefix) {
                    suggestions.push(CompletionSuggestion {
                        value: "Cargo.toml".to_string(),
                        label: Some("Cargo.toml".to_string()),
                        description: Some("Rust package configuration".to_string()),
                        annotations: None,
                    });
                }
                suggestions
            }
            "version" => {
                // Suggest semantic version patterns
                vec![
                    CompletionSuggestion {
                        value: "1.0.0".to_string(),
                        label: Some("1.0.0".to_string()),
                        description: Some("Major release version".to_string()),
                        annotations: None,
                    },
                    CompletionSuggestion {
                        value: "0.1.0".to_string(),
                        label: Some("0.1.0".to_string()),
                        description: Some("Initial development version".to_string()),
                        annotations: None,
                    },
                    CompletionSuggestion {
                        value: "2.0.0-beta".to_string(),
                        label: Some("2.0.0-beta".to_string()),
                        description: Some("Beta pre-release version".to_string()),
                        annotations: None,
                    },
                ]
            }
            "environment" | "env" => {
                vec![
                    CompletionSuggestion {
                        value: "development".to_string(),
                        label: Some("Development".to_string()),
                        description: Some("Development environment".to_string()),
                        annotations: None,
                    },
                    CompletionSuggestion {
                        value: "staging".to_string(),
                        label: Some("Staging".to_string()),
                        description: Some("Staging environment".to_string()),
                        annotations: None,
                    },
                    CompletionSuggestion {
                        value: "production".to_string(),
                        label: Some("Production".to_string()),
                        description: Some("Production environment".to_string()),
                        annotations: None,
                    },
                ]
            }
            _ => {
                // Fallback: combine all relevant completions
                let mut all_suggestions = Vec::new();
                all_suggestions.extend(self.get_language_completions(&prefix).into_iter().take(3));
                all_suggestions.extend(self.get_command_completions(&prefix).into_iter().take(3));
                all_suggestions.extend(self.get_framework_completions(&prefix).into_iter().take(2));
                all_suggestions
            }
        }
    }
}

#[async_trait]
impl McpHandler for IdeCompletionHandler {
    async fn handle(&self, params: Option<Value>) -> McpResult<Value> {
        if let Some(params) = params {
            let request: CompleteRequest = serde_json::from_value(params).map_err(|e| {
                McpError::invalid_param_type("params", "CompleteRequest", &e.to_string())
            })?;

            let suggestions =
                self.get_smart_completions(&request.params.argument.name, &request.params.argument.value);

            // Limit to 10 suggestions for better UX
            let limited_suggestions: Vec<_> = suggestions.into_iter().take(10).collect();

            let response = CompletionResponse::new(limited_suggestions);
            serde_json::to_value(response).map_err(|e| {
                McpError::tool_execution(&format!("Failed to serialize completion response: {}", e))
            })
        } else {
            // Return example completions when no params provided
            let suggestions = vec![
                CompletionSuggestion {
                    value: "rust".to_string(),
                    label: Some("Rust programming language".to_string()),
                    description: Some(
                        "Systems programming language focused on safety and performance"
                            .to_string(),
                    ),
                    annotations: None,
                },
                CompletionSuggestion {
                    value: "python".to_string(),
                    label: Some("Python programming language".to_string()),
                    description: Some(
                        "High-level programming language with elegant syntax".to_string(),
                    ),
                    annotations: None,
                },
                CompletionSuggestion {
                    value: "javascript".to_string(),
                    label: Some("JavaScript programming language".to_string()),
                    description: Some(
                        "Dynamic programming language for web development".to_string(),
                    ),
                    annotations: None,
                },
            ];

            let response = CompletionResponse::new(suggestions);
            serde_json::to_value(response).map_err(|e| {
                McpError::tool_execution(&format!("Failed to serialize completion response: {}", e))
            })
        }
    }

    fn supported_methods(&self) -> Vec<String> {
        vec!["completion/complete".to_string()]
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting IDE Auto-Completion Server Example");

    let completion_handler = IdeCompletionHandler::new()
        .map_err(|e| format!("Failed to initialize completion handler: {}", e))?;

    let server = McpServer::builder()
        .name("ide-completion-server")
        .version("1.0.0")
        .title("IDE Auto-Completion Server")
        .instructions("Real-world IDE completion server that provides intelligent auto-completion suggestions for developers. Loads data from external JSON files for programming languages, frameworks, and development commands.")
        .handler(completion_handler)
        .bind_address("127.0.0.1:8042".parse()?)
        .build()?;

    info!("IDE completion server running at: http://127.0.0.1:8042/mcp");
    info!("Real-world completion features for developers:");
    info!(
        "  - Programming language suggestions with categories (language, lang, programming_language)"
    );
    info!("  - File extension completions with descriptions (extension, ext, file_extension)");
    info!("  - Development command completions with tool examples (command, cmd, action)");
    info!("  - Framework suggestions with language and category info (framework, library, lib)");
    info!("  - File path suggestions for common project files (filename, file, path)");
    info!("  - Version pattern suggestions for semantic versioning (version)");
    info!("  - Environment completions for deployment contexts (environment, env)");
    info!("Data loaded from external JSON files in data/ directory");

    server.run().await?;
    Ok(())
}
