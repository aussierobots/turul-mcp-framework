//! Macros resolve through `turul_mcp_server` alone: nothing here imports
//! `turul_mcp_protocol`, `turul_mcp_builders`, `serde_json` or `async_trait`.

use turul_mcp_server::JsonRpcHandler;
use turul_mcp_server::prelude::*;

#[mcp_tool(name = "add", description = "Add two numbers")]
async fn add(a: f64, b: f64) -> McpResult<f64> {
    Ok(a + b)
}

#[derive(McpTool, Default)]
#[tool(name = "greet", description = "Greet")]
struct Greet {
    #[param(description = "Name")]
    name: String,
}

impl Greet {
    async fn execute(&self, _s: Option<SessionContext>) -> McpResult<String> {
        Ok(format!("hi {}", self.name))
    }
}

#[tokio::test]
async fn prelude_alone_registers_macro_tools() {
    McpServer::builder()
        .name("prelude-only")
        .tool_fn(add)
        .tool(Greet::default())
        .build()
        .unwrap();

    let mut tools: std::collections::HashMap<String, Arc<dyn McpTool>> = Default::default();
    tools.insert("add".into(), Arc::new(add()));
    tools.insert("greet".into(), Arc::new(Greet::default()));
    let listed = turul_mcp_server::ListToolsHandler::new(tools, false)
        .handle("tools/list", None, None)
        .await
        .unwrap();
    let mut names: Vec<&str> = listed["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    names.sort();
    assert_eq!(names, ["add", "greet"]);
}
