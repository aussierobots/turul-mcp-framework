// turul-mcp-server v0.3
// A tool that uses ElicitationBuilder to collect user input before processing

use serde::{Deserialize, Serialize};
use serde_json::json;
use turul_mcp_builders::ElicitationBuilder;
use turul_mcp_derive::McpTool;
use turul_mcp_protocol::elicitation::ElicitAction;
use turul_mcp_server::prelude::*;

#[derive(Serialize, Deserialize, schemars::JsonSchema)]
struct GreetingResult {
    message: String,
    personalized: bool,
}

#[derive(McpTool, Default)]
#[tool(
    name = "personalized_greeting",
    description = "Generate a personalized greeting by asking the user for their details",
    output = GreetingResult
)]
struct PersonalizedGreetingTool;

impl PersonalizedGreetingTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<GreetingResult> {
        // Build the elicitation request
        let request = ElicitationBuilder::new("Please tell us about yourself")
            .title("Greeting Setup")
            .string_field("name", "Your name")
            .enum_field(
                "language",
                "Preferred language",
                vec!["english".into(), "spanish".into(), "french".into()],
            )
            .boolean_field_with_default("formal", "Use formal greeting", false)
            .require_field("name")
            .build();

        // In production, the ElicitationProvider sends this to the client.
        // The client presents the form and returns the result.
        // For this example, simulate an accept response:
        let result = simulate_elicitation_response();

        match result.action {
            ElicitAction::Accept => {
                let content = result.content.unwrap_or_default();
                let name = content
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("World");
                let formal = content
                    .get("formal")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let lang = content
                    .get("language")
                    .and_then(|v| v.as_str())
                    .unwrap_or("english");

                let greeting = match (lang, formal) {
                    ("spanish", true) => format!("Estimado/a {}, es un placer.", name),
                    ("spanish", false) => format!("¡Hola, {}!", name),
                    ("french", true) => format!("Cher/Chère {}, enchanté(e).", name),
                    ("french", false) => format!("Salut, {} !", name),
                    (_, true) => format!("Dear {}, it is a pleasure.", name),
                    (_, false) => format!("Hey, {}!", name),
                };

                Ok(GreetingResult {
                    message: greeting,
                    personalized: true,
                })
            }
            ElicitAction::Decline | ElicitAction::Cancel => Ok(GreetingResult {
                message: "Hello, World!".to_string(),
                personalized: false,
            }),
        }
    }
}

fn simulate_elicitation_response() -> turul_mcp_protocol::elicitation::ElicitResult {
    use turul_mcp_builders::ElicitResultBuilder;
    ElicitResultBuilder::accept_fields(vec![
        ("name".into(), json!("Alice")),
        ("language".into(), json!("french")),
        ("formal".into(), json!(false)),
    ])
}

// Server setup
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = McpServer::builder()
        .name("greeting-server")
        .version("1.0.0")
        .with_elicitation() // Enable mock elicitation provider
        .tool(PersonalizedGreetingTool)
        .build()?;

    server.run().await?;
    Ok(())
}
