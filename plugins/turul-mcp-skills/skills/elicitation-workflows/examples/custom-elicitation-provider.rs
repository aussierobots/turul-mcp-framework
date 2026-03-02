// turul-mcp-server v0.3
// Implementing a custom ElicitationProvider for real UI integration

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use turul_mcp_protocol::elicitation::{
    ElicitAction, ElicitCreateRequest, ElicitResult, PrimitiveSchemaDefinition,
};
use turul_mcp_protocol::McpError;
use turul_mcp_server::handlers::ElicitationProvider;
use turul_mcp_server::prelude::*;

/// Example: CLI-based elicitation provider
/// Prompts the user on the terminal and reads their input.
struct CliElicitationProvider;

#[async_trait]
impl ElicitationProvider for CliElicitationProvider {
    async fn elicit(&self, request: &ElicitCreateRequest) -> Result<ElicitResult, McpError> {
        println!("\n--- Elicitation Request ---");
        println!("Message: {}", request.params.message);
        println!("Fields:");

        let mut content = HashMap::new();

        for (field_name, field_schema) in &request.params.requested_schema.properties {
            let description = match field_schema {
                PrimitiveSchemaDefinition::String(s) => {
                    s.description.as_deref().unwrap_or("(string)")
                }
                PrimitiveSchemaDefinition::Number(n) => {
                    n.description.as_deref().unwrap_or("(number)")
                }
                PrimitiveSchemaDefinition::Boolean(b) => {
                    b.description.as_deref().unwrap_or("(boolean)")
                }
                PrimitiveSchemaDefinition::Enum(e) => {
                    e.description.as_deref().unwrap_or("(choice)")
                }
            };

            let is_required = request
                .params
                .requested_schema
                .required
                .as_ref()
                .map(|r| r.contains(field_name))
                .unwrap_or(false);

            let required_marker = if is_required { " *" } else { "" };
            println!("  {}{}: {}", field_name, required_marker, description);

            // In a real CLI provider, you would read from stdin here.
            // This example simulates user input.
            let value = simulate_cli_input(field_name, field_schema);
            content.insert(field_name.clone(), value);
        }

        Ok(ElicitResult {
            action: ElicitAction::Accept,
            content: Some(content),
            validation_results: None,
        })
    }
}

fn simulate_cli_input(
    field_name: &str,
    schema: &PrimitiveSchemaDefinition,
) -> serde_json::Value {
    match schema {
        PrimitiveSchemaDefinition::String(_) => {
            serde_json::json!(format!("input_{}", field_name))
        }
        PrimitiveSchemaDefinition::Number(_) => serde_json::json!(42.0),
        PrimitiveSchemaDefinition::Boolean(_) => serde_json::json!(true),
        PrimitiveSchemaDefinition::Enum(e) => {
            // Pick the first enum value
            serde_json::json!(e.enum_values.first().unwrap_or(&"unknown".to_string()))
        }
    }
}

/// Example: Web form elicitation provider
/// Sends the schema to a web frontend and waits for the response.
struct WebFormProvider {
    callback_url: String,
}

#[async_trait]
impl ElicitationProvider for WebFormProvider {
    async fn elicit(&self, request: &ElicitCreateRequest) -> Result<ElicitResult, McpError> {
        // 1. POST the schema to your web frontend
        let client = reqwest::Client::new();
        let response = client
            .post(&self.callback_url)
            .json(&serde_json::json!({
                "message": request.params.message,
                "schema": request.params.requested_schema,
            }))
            .send()
            .await
            .map_err(|e| McpError::tool_execution(format!("Failed to send form: {}", e)))?;

        // 2. Parse the response
        if !response.status().is_success() {
            return Ok(ElicitResult {
                action: ElicitAction::Cancel,
                content: None,
                validation_results: None,
            });
        }

        let content: HashMap<String, serde_json::Value> = response
            .json()
            .await
            .map_err(|e| McpError::tool_execution(format!("Invalid response: {}", e)))?;

        Ok(ElicitResult {
            action: ElicitAction::Accept,
            content: Some(content),
            validation_results: None,
        })
    }
}

// Server setup with custom provider
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Option 1: CLI provider
    let server = McpServer::builder()
        .name("cli-elicitation-server")
        .version("1.0.0")
        .with_elicitation_provider(CliElicitationProvider)
        .build()?;

    // Option 2: Web form provider
    // let server = McpServer::builder()
    //     .name("web-elicitation-server")
    //     .version("1.0.0")
    //     .with_elicitation_provider(WebFormProvider {
    //         callback_url: "http://localhost:3000/elicit".to_string(),
    //     })
    //     .build()?;

    server.run().await?;
    Ok(())
}
