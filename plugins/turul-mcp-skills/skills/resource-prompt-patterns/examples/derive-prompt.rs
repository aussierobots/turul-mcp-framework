// turul-mcp-server v0.3
// Prompt Pattern 1: Derive Macro #[derive(McpPrompt)]
// Derive generates metadata + argument traits only — McpPrompt::render() is manual.
// render() has no session parameter.

use turul_mcp_derive::McpPrompt;
use turul_mcp_server::prelude::*;

#[derive(McpPrompt)]
#[prompt(name = "code_review", description = "Review code for bugs and improvements")]
struct CodeReviewPrompt {
    #[argument(description = "Programming language of the code")]
    language: String,
    #[argument(description = "Source code to review")]
    code: String,
}

#[async_trait]
impl McpPrompt for CodeReviewPrompt {
    async fn render(
        &self,
        args: Option<HashMap<String, Value>>,
    ) -> McpResult<Vec<PromptMessage>> {
        let language = args.as_ref()
            .and_then(|a| a.get("language"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let code = args.as_ref()
            .and_then(|a| a.get("code"))
            .and_then(|v| v.as_str())
            .ok_or(McpError::invalid_params("Missing required argument: code"))?;

        Ok(vec![
            PromptMessage::user_text(format!(
                "Please review the following {language} code for bugs, \
                 security issues, and improvement opportunities:\n\n\
                 ```{language}\n{code}\n```"
            )),
            PromptMessage::assistant_text(
                "I'll review this code carefully. Let me analyze it for:".to_string()
            ),
        ])
    }
}

// Registration: .prompt() for derive macros
fn build_server() -> Result<McpServer, Box<dyn std::error::Error>> {
    let server = McpServer::builder()
        .name("review-server")
        .prompt(CodeReviewPrompt {
            language: String::new(),
            code: String::new(),
        })
        .build()?;
    Ok(server)
}
