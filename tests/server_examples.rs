//! Tests for code examples from turul-mcp-server README.md
//!
//! These tests verify that all code examples in the turul-mcp-server README
//! compile correctly and use the correct API patterns.

use turul_mcp_derive::{McpTool, mcp_tool};
use turul_mcp_server::{McpResult, McpServer, SessionContext};

/// Test Level 1: Function Macros example from turul-mcp-server README
/// Verifies the basic function macro pattern compiles correctly
#[test]
fn test_level1_function_macros() {
    // This test just verifies compilation - the code itself is in the function definitions

    #[mcp_tool(name = "add", description = "Add two numbers")]
    async fn add(
        #[param(description = "First number")] a: f64,
        #[param(description = "Second number")] b: f64,
    ) -> McpResult<f64> {
        Ok(a + b)
    }

    // Verify server builder works with function macro
    let _server = McpServer::builder()
        .name("calculator-server")
        .version("1.0.0")
        .tool_fn(add) // Use original function name
        .bind_address("127.0.0.1:8080".parse().unwrap())
        .build()
        .expect("Server should build successfully");
}

/// Test Level 2: Derive Macros example from turul-mcp-server README
/// Verifies the struct-based derive pattern compiles correctly
#[test]
fn test_level2_derive_macros() {
    #[derive(McpTool, Clone)]
    #[tool(name = "calculator", description = "Add two numbers")]
    struct Calculator {
        #[param(description = "First number")]
        a: f64,
        #[param(description = "Second number")]
        b: f64,
    }

    impl Calculator {
        async fn execute(&self, _session: Option<SessionContext>) -> McpResult<f64> {
            Ok(self.a + self.b)
        }
    }

    // Verify server builder works with derive macro
    let _server = McpServer::builder()
        .name("calculator-server")
        .version("1.0.0")
        .tool(Calculator { a: 0.0, b: 0.0 })
        .bind_address("127.0.0.1:8080".parse().unwrap())
        .build()
        .expect("Server should build successfully");
}

/// Test Level 3: Builder Pattern example from turul-mcp-server README
/// Verifies the runtime builder pattern compiles correctly
#[test]
fn test_level3_builder_pattern() {
    use serde_json::json;
    use turul_mcp_server::ToolBuilder;

    let calculator = ToolBuilder::new("calculator")
        .description("Add two numbers")
        .number_param("a", "First number")
        .number_param("b", "Second number")
        .number_output() // Generates {"result": number} schema
        .execute(|args| async move {
            let a = args
                .get("a")
                .and_then(|v| v.as_f64())
                .ok_or("Missing parameter 'a'")?;
            let b = args
                .get("b")
                .and_then(|v| v.as_f64())
                .ok_or("Missing parameter 'b'")?;

            Ok(json!({"result": a + b}))
        })
        .build()
        .expect("Tool should build successfully");

    let _server = McpServer::builder()
        .name("calculator-server")
        .version("1.0.0")
        .tool(calculator)
        .bind_address("127.0.0.1:8080".parse().unwrap())
        .build()
        .expect("Server should build successfully");
}

/// Test Session Context example from turul-mcp-server README
/// Verifies session state management API is correct
#[test]
fn test_session_context() {
    #[derive(McpTool, Clone, Default)]
    #[tool(name = "stateful_counter", description = "Increment session counter")]
    struct StatefulCounter {
        // Derive macros require named fields, so we add a dummy field
        _marker: (),
    }

    impl StatefulCounter {
        async fn execute(&self, session: Option<SessionContext>) -> McpResult<i32> {
            if let Some(session) = session {
                // Get current counter or start at 0
                let current: i32 = session.get_typed_state("counter").await.unwrap_or(0);
                let new_count = current + 1;

                // Save updated counter
                session.set_typed_state("counter", new_count).await.unwrap();

                // Send progress notification
                session.notify_progress("counting", new_count as u64).await;

                Ok(new_count)
            } else {
                Ok(0) // No session available
            }
        }
    }

    // Verify the session context tool compiles
    let _counter = StatefulCounter::default();
}
