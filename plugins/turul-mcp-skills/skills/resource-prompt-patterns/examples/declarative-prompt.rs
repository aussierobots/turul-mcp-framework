// turul-mcp-server v0.3
// Prompt Pattern 2: Declarative Macro prompt!{}
// Generates a full struct + all traits + McpPrompt impl from a closure.
// Good for inline one-off prompts.

use turul_mcp_server::prelude::*;

fn build_server() -> Result<McpServer, Box<dyn std::error::Error>> {
    let greet = prompt! {
        name: "greet",
        description: "Generate a personalized greeting",
        arguments: {
            name: String => "Person to greet", required
        },
        template: |args| async move {
            let name = args.as_ref()
                .and_then(|a| a.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("World");
            Ok(vec![
                PromptMessage::user_text(format!(
                    "Please write a warm, friendly greeting for {name}."
                )),
            ])
        }
    };

    let server = McpServer::builder()
        .name("greeting-server")
        .prompt(greet)   // .prompt() for declarative macro
        .build()?;
    Ok(server)
}
