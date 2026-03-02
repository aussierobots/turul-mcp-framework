// turul-mcp-server v0.3
// Multi-step elicitation workflow using session state to track progress

use serde::{Deserialize, Serialize};
use serde_json::json;
use turul_mcp_builders::ElicitationBuilder;
use turul_mcp_derive::McpTool;
use turul_mcp_protocol::elicitation::{ElicitAction, StringFormat};
use turul_mcp_server::prelude::*;

#[derive(Serialize, Deserialize, schemars::JsonSchema)]
struct OnboardingResult {
    status: String,
    step: u32,
    message: String,
}

/// Multi-step onboarding tool that collects user info across 3 elicitation rounds.
/// Session state tracks which step the user is on.
#[derive(McpTool, Default)]
#[tool(
    name = "onboarding",
    description = "Multi-step user onboarding with progressive data collection",
    output = OnboardingResult
)]
struct OnboardingTool;

impl OnboardingTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<OnboardingResult> {
        let session = session.ok_or(McpError::tool_execution("Session required for onboarding"))?;

        // Get current step from session state (default: 1)
        let step: u32 = session.get_typed_state("onboarding_step").await.unwrap_or(1);

        match step {
            1 => {
                // Step 1: Collect basic identity
                let _request = ElicitationBuilder::form("Step 1 of 3: Basic Information")
                    .title("User Onboarding")
                    .string_field("full_name", "Full legal name")
                    .string_field_with_format("email", "Email address", StringFormat::Email)
                    .require_fields(vec!["full_name".into(), "email".into()])
                    .build();

                // In production: send request via ElicitationProvider, get response
                // Simulate acceptance for this example:
                let name = "Alice Smith";
                let email = "alice@example.com";

                // Store collected data in session state
                session
                    .set_typed_state("onboarding_name", name.to_string())
                    .await
                    .map_err(|e| McpError::tool_execution(e.to_string()))?;
                session
                    .set_typed_state("onboarding_email", email.to_string())
                    .await
                    .map_err(|e| McpError::tool_execution(e.to_string()))?;
                session
                    .set_typed_state("onboarding_step", 2u32)
                    .await
                    .map_err(|e| McpError::tool_execution(e.to_string()))?;

                Ok(OnboardingResult {
                    status: "in_progress".to_string(),
                    step: 1,
                    message: format!("Welcome, {}! Run onboarding again for step 2.", name),
                })
            }
            2 => {
                // Step 2: Collect role and preferences
                let _request = ElicitationBuilder::form("Step 2 of 3: Role & Preferences")
                    .title("User Onboarding")
                    .enum_field(
                        "role",
                        "Your role",
                        vec!["developer".into(), "designer".into(), "manager".into()],
                    )
                    .enum_field(
                        "team_size",
                        "Team size",
                        vec!["solo".into(), "small".into(), "medium".into(), "large".into()],
                    )
                    .boolean_field_with_default("notifications", "Enable email notifications", true)
                    .require_field("role")
                    .build();

                // Simulate response
                session
                    .set_typed_state("onboarding_role", "developer".to_string())
                    .await
                    .map_err(|e| McpError::tool_execution(e.to_string()))?;
                session
                    .set_typed_state("onboarding_step", 3u32)
                    .await
                    .map_err(|e| McpError::tool_execution(e.to_string()))?;

                Ok(OnboardingResult {
                    status: "in_progress".to_string(),
                    step: 2,
                    message: "Preferences saved. Run onboarding again for final step.".to_string(),
                })
            }
            3 => {
                // Step 3: Confirmation
                let name: String = session
                    .get_typed_state("onboarding_name")
                    .await
                    .unwrap_or_else(|| "Unknown".to_string());
                let email: String = session
                    .get_typed_state("onboarding_email")
                    .await
                    .unwrap_or_else(|| "unknown@example.com".to_string());
                let role: String = session
                    .get_typed_state("onboarding_role")
                    .await
                    .unwrap_or_else(|| "unknown".to_string());

                let _request = ElicitationBuilder::confirm(&format!(
                    "Confirm your details:\nName: {}\nEmail: {}\nRole: {}",
                    name, email, role
                ))
                .build();

                // Mark onboarding complete
                session
                    .set_typed_state("onboarding_step", 4u32)
                    .await
                    .map_err(|e| McpError::tool_execution(e.to_string()))?;

                Ok(OnboardingResult {
                    status: "completed".to_string(),
                    step: 3,
                    message: format!("Onboarding complete for {} ({})!", name, role),
                })
            }
            _ => Ok(OnboardingResult {
                status: "already_completed".to_string(),
                step,
                message: "Onboarding was already completed.".to_string(),
            }),
        }
    }
}
