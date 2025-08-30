use async_trait::async_trait;
use turul_mcp_derive::McpTool;
use turul_mcp_protocol::{CallToolResponse, ToolResult, ToolSchema};
use turul_mcp_server::{McpResult, McpServer, McpTool, SessionContext};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::info;

// DERIVE MACRO APPROACH: Simple echo tool
#[derive(McpTool, Clone)]
#[tool(name = "echo_derive", description = "Echo text using derive macro")]
#[output_type(String)]
struct EchoDeriveTool {
    #[param(description = "Text to echo")]
    text: String,
    #[param(description = "Number of times to repeat")]
    repeat: Option<u32>,
}

impl EchoDeriveTool {
    async fn execute(&self) -> McpResult<String> {
        let repeat_count = self.repeat.unwrap_or(1);
        Ok(self.text.repeat(repeat_count as usize))
    }
}

// MANUAL APPROACH: Same echo functionality but implemented manually
#[derive(Clone)]
struct EchoManualTool {
    text: String,
    repeat: Option<u32>,
}

impl EchoManualTool {
    fn new() -> Self {
        Self {
            text: String::new(),
            repeat: None,
        }
    }
}

#[async_trait]
impl McpTool for EchoManualTool {
    fn name(&self) -> &str {
        "echo_manual"
    }

    fn description(&self) -> &str {
        "Echo text using manual implementation"
    }

    fn input_schema(&self) -> ToolSchema {
        use turul_mcp_protocol::schema::JsonSchema;
        use std::collections::HashMap;
        
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("text".to_string(), JsonSchema::string().with_description("Text to echo")),
                ("repeat".to_string(), JsonSchema::integer().with_description("Number of times to repeat")),
            ]))
            .with_required(vec!["text".to_string()])
    }
    
    fn output_schema(&self) -> Option<ToolSchema> {
        use turul_mcp_protocol::schema::JsonSchema;
        use std::collections::HashMap;
        
        Some(ToolSchema::object()
            .with_properties(HashMap::from([
                ("value".to_string(), JsonSchema::string())
            ]))
            .with_required(vec!["value".to_string()]))
    }

    async fn call(&self, args: Value, _session: Option<SessionContext>) -> McpResult<Vec<ToolResult>> {
        // Log the struct fields to show they're part of the example (even if not used in execution)
        tracing::debug!("EchoManualTool called with struct fields: text='{}', repeat={:?}", 
                       self.text, self.repeat);
        
        let text = args.get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| turul_mcp_protocol::McpError::invalid_param_type("text", "string", "other"))?;
            
        let repeat = args.get("repeat")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);

        let repeat_count = repeat.unwrap_or(1);
        let result = text.repeat(repeat_count as usize);
        
        let json_text = serde_json::json!({"value": result}).to_string();
        Ok(vec![ToolResult::text(json_text)])
    }
    
    async fn execute(&self, args: Value, session: Option<SessionContext>) -> McpResult<CallToolResponse> {
        let content = self.call(args.clone(), session).await?;
        let response = CallToolResponse::success(content);
        
        if self.output_schema().is_some() {
            let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("");
            let repeat = args.get("repeat").and_then(|v| v.as_u64()).map(|v| v as u32);
            let repeat_count = repeat.unwrap_or(1);
            let result = text.repeat(repeat_count as usize);
            
            let structured_content = serde_json::json!({"value": result});
            Ok(response.with_structured_content(structured_content))
        } else {
            Ok(response)
        }
    }
}

// DERIVE MACRO APPROACH: Complex data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersonData {
    name: String,
    age: u32,
    email: String,
    is_active: bool,
}

impl std::fmt::Display for PersonData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PersonData {{ name: {}, age: {}, email: {}, is_active: {} }}", 
               self.name, self.age, self.email, self.is_active)
    }
}

#[derive(McpTool, Clone)]
#[tool(name = "person_creator_derive", description = "Create person data using derive macro")]
struct PersonCreatorDeriveTool {
    #[param(description = "Person's name")]
    name: String,
    #[param(description = "Person's age")]
    age: u32,
    #[param(description = "Person's email")]
    email: String,
    #[param(description = "Is person active")]
    is_active: Option<bool>,
}

// Note: No #[output_type] - this will use generic schema for PersonData
impl PersonCreatorDeriveTool {
    async fn execute(&self) -> McpResult<PersonData> {
        Ok(PersonData {
            name: self.name.clone(),
            age: self.age,
            email: self.email.clone(),
            is_active: self.is_active.unwrap_or(true),
        })
    }
}

// MANUAL APPROACH: Same person creator but manual implementation
#[derive(Clone)]
struct PersonCreatorManualTool;

#[async_trait]
impl McpTool for PersonCreatorManualTool {
    fn name(&self) -> &str {
        "person_creator_manual"
    }

    fn description(&self) -> &str {
        "Create person data using manual implementation"
    }

    fn input_schema(&self) -> ToolSchema {
        use turul_mcp_protocol::schema::JsonSchema;
        use std::collections::HashMap;
        
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("name".to_string(), JsonSchema::string().with_description("Person's name")),
                ("age".to_string(), JsonSchema::integer().with_description("Person's age")),
                ("email".to_string(), JsonSchema::string().with_description("Person's email")),
                ("is_active".to_string(), JsonSchema::boolean().with_description("Is person active")),
            ]))
            .with_required(vec!["name".to_string(), "age".to_string(), "email".to_string()])
    }
    
    fn output_schema(&self) -> Option<ToolSchema> {
        use turul_mcp_protocol::schema::JsonSchema;
        use std::collections::HashMap;
        
        Some(ToolSchema::object()
            .with_properties(HashMap::from([
                ("name".to_string(), JsonSchema::string()),
                ("age".to_string(), JsonSchema::integer()),
                ("email".to_string(), JsonSchema::string()),
                ("is_active".to_string(), JsonSchema::boolean()),
            ]))
            .with_required(vec!["name".to_string(), "age".to_string(), "email".to_string(), "is_active".to_string()]))
    }

    async fn call(&self, args: Value, _session: Option<SessionContext>) -> McpResult<Vec<ToolResult>> {
        let name = args.get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| turul_mcp_protocol::McpError::invalid_param_type("name", "string", "other"))?;
            
        let age = args.get("age")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| turul_mcp_protocol::McpError::invalid_param_type("age", "integer", "other"))? as u32;
            
        let email = args.get("email")
            .and_then(|v| v.as_str())
            .ok_or_else(|| turul_mcp_protocol::McpError::invalid_param_type("email", "string", "other"))?;
            
        let is_active = args.get("is_active")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let person = PersonData {
            name: name.to_string(),
            age,
            email: email.to_string(),
            is_active,
        };
        
        let json_text = serde_json::to_string(&person).map_err(|e| 
            turul_mcp_protocol::McpError::tool_execution(&e.to_string()))?;
        
        Ok(vec![ToolResult::text(json_text)])
    }
    
    async fn execute(&self, args: Value, session: Option<SessionContext>) -> McpResult<CallToolResponse> {
        let content = self.call(args.clone(), session).await?;
        let response = CallToolResponse::success(content);
        
        if self.output_schema().is_some() {
            let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let age = args.get("age").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            let email = args.get("email").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let is_active = args.get("is_active").and_then(|v| v.as_bool()).unwrap_or(true);
            
            let person = PersonData { name, age, email, is_active };
            let structured_content = serde_json::to_value(&person)
                .map_err(|e| turul_mcp_protocol::McpError::tool_execution(&e.to_string()))?;
            
            Ok(response.with_structured_content(structured_content))
        } else {
            Ok(response)
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting mixed approaches example server");
    let server = McpServer::builder()
        .name("mixed_approaches_example")
        .version("0.1.0")
        .title("Mixed Approaches Example Server")
        .instructions("Demonstrates both derive macro and manual trait implementation approaches")
        .tool(EchoDeriveTool {
            text: String::new(),
            repeat: None,
        })
        .tool(EchoManualTool::new())
        .tool(PersonCreatorDeriveTool {
            name: String::new(),
            age: 0,
            email: String::new(),
            is_active: None,
        })
        .tool(PersonCreatorManualTool)
        .bind_address("127.0.0.1:8648".parse()?)
        .build()?;

    server.run().await?;
    Ok(())
}