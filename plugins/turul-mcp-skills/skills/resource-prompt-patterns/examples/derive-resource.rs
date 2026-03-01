// turul-mcp-server v0.3
// Resource Pattern 2: Derive Macro #[derive(McpResource)]
// Derive generates metadata traits only — McpResource::read() is manual.
// Session access is available in the manual read() impl.

use turul_mcp_derive::McpResource;
use turul_mcp_server::prelude::*;

#[derive(McpResource, Clone)]
#[resource(
    name = "user_profile",
    uri = "file:///users/{user_id}.json",
    description = "User profile data with session-aware access control"
)]
struct UserProfileResource;

#[async_trait]
impl McpResource for UserProfileResource {
    async fn read(
        &self,
        params: Option<Value>,
        session: Option<&SessionContext>,
    ) -> McpResult<Vec<ResourceContent>> {
        // Extract template variable
        let user_id = params.as_ref()
            .and_then(|p| p.get("template_variables"))
            .and_then(|tv| tv.get("user_id"))
            .and_then(|v| v.as_str())
            .ok_or(McpError::invalid_params("Missing user_id"))?;

        // Session-aware access control
        if let Some(s) = session {
            let role: String = s.get_typed_state("role").await.unwrap_or_default();
            if role != "admin" && s.get_typed_state::<String>("user_id").await.as_deref() != Ok(user_id) {
                return Err(McpError::tool_execution("Access denied"));
            }
        }

        let profile_json = load_profile(user_id).await?;
        Ok(vec![ResourceContent::text(
            &format!("file:///users/{user_id}.json"),
            profile_json,
        )])
    }
}

// Registration: use .resource() for derive macros
fn build_server() -> Result<McpServer, Box<dyn std::error::Error>> {
    let server = McpServer::builder()
        .name("profile-server")
        .resource(UserProfileResource)
        .build()?;
    Ok(server)
}
