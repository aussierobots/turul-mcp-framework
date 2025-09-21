//! Tests for code examples from turul-mcp-derive README.md
//!
//! These tests verify that all procedural macro examples in the turul-mcp-derive README
//! compile correctly and generate the expected trait implementations.

use turul_mcp_derive::{mcp_tool, McpTool};
use turul_mcp_server::{McpResult, SessionContext, McpServer};

/// Test basic function macro example from turul-mcp-derive README
#[test]
fn test_basic_function_macro() {
    #[mcp_tool(name = "calculator", description = "Add two numbers")]
    async fn calculator(
        #[param(description = "First number")] a: f64,
        #[param(description = "Second number")] b: f64,
    ) -> McpResult<f64> {
        Ok(a + b)
    }

    // Verify it can be used with server builder
    let _server = McpServer::builder()
        .name("test-server")
        .version("1.0.0")
        .tool_fn(calculator)
        .bind_address("127.0.0.1:8080".parse().unwrap())
        .build()
        .expect("Server should build successfully");
}

/// Test session context integration from turul-mcp-derive README
#[test]
fn test_session_context_function_macro() {
    #[mcp_tool(name = "counter", description = "Session-persistent counter")]
    async fn session_counter(
        session: Option<SessionContext>  // Automatically detected by macro
    ) -> McpResult<i32> {
        if let Some(session) = session {
            let count: i32 = session.get_typed_state("count").await.unwrap_or(0);
            let new_count = count + 1;
            session.set_typed_state("count", new_count).await.unwrap();
            Ok(new_count)
        } else {
            Ok(0) // No session available
        }
    }

    // Verify it compiles - can't easily test runtime behavior without complex setup
    let _server = McpServer::builder()
        .name("test-server")
        .version("1.0.0")  
        .tool_fn(session_counter)
        .bind_address("127.0.0.1:8080".parse().unwrap())
        .build()
        .expect("Server should build successfully");
}

/// Test custom output field from turul-mcp-derive README
#[test]
fn test_custom_output_field() {
    #[mcp_tool(
        name = "multiply", 
        description = "Multiply two numbers",
        output_field = "product"  // Custom output field name
    )]
    async fn multiply(
        #[param(description = "First number")] x: f64,
        #[param(description = "Second number")] y: f64,
    ) -> McpResult<f64> {
        Ok(x * y)  // Returns {"product": 15.0} instead of {"result": 15.0}
    }

    // Verify the custom output field configuration compiles
    let _server = McpServer::builder()
        .name("test-server")
        .version("1.0.0")
        .tool_fn(multiply)
        .bind_address("127.0.0.1:8080".parse().unwrap())
        .build()
        .expect("Server should build successfully");
}

/// Test progress notifications from turul-mcp-derive README
#[test]
fn test_progress_notifications_function_macro() {
    #[mcp_tool(name = "slow_task", description = "Task with progress updates")]
    async fn slow_task(
        #[param(description = "Number of steps")] steps: u32,
        session: Option<SessionContext>,
    ) -> McpResult<String> {
        for i in 1..=steps.min(3) { // Limit for testing
            if let Some(ref session) = session {
        session.notify_progress("slow-task", i as u64).await;
            }
            
            // Don't actually sleep in tests
        }
        
        Ok(format!("Completed {} steps", steps))
    }

    // Verify it compiles with progress notification calls
    let _server = McpServer::builder()
        .name("test-server")
        .version("1.0.0")
        .tool_fn(slow_task)
        .bind_address("127.0.0.1:8080".parse().unwrap())
        .build()
        .expect("Server should build successfully");
}

/// Test basic struct derive macro from turul-mcp-derive README
#[test]
fn test_basic_struct_derive() {
    #[derive(McpTool, Clone, Default)]
    #[tool(name = "calculator", description = "Advanced calculator")]
    struct Calculator {
        #[param(description = "First number")]
        a: f64,
        #[param(description = "Second number")]  
        b: f64,
        #[param(description = "Operation type")]
        operation: String,
    }

    impl Calculator {
        async fn execute(&self, _session: Option<SessionContext>) -> McpResult<f64> {
            match self.operation.as_str() {
                "add" => Ok(self.a + self.b),
                "multiply" => Ok(self.a * self.b),
                "subtract" => Ok(self.a - self.b),
                "divide" => {
                    if self.b == 0.0 {
                        Err("Division by zero".into())
                    } else {
                        Ok(self.a / self.b)
                    }
                }
                _ => Err("Unsupported operation".into())
            }
        }
    }

    // Verify struct-based tool works with server
    let _server = McpServer::builder()
        .name("calculator-server")
        .version("1.0.0")
        .tool(Calculator::default())
        .bind_address("127.0.0.1:8080".parse().unwrap())
        .build()
        .expect("Server should build successfully");
}

/// Test complex parameter types from turul-mcp-derive README
#[test]
fn test_complex_parameter_types() {
    #[derive(McpTool, Clone, Default)]
    #[tool(name = "complex_params", description = "Demonstrate all parameter types")]
    struct ComplexParamsTool {
        // Basic types
        #[param(description = "String parameter")]
        text: String,
        
        #[param(description = "Integer parameter")]
        _number: i32,
        
        #[param(description = "Float parameter")]
        _decimal: f64,
        
        #[param(description = "Boolean parameter")]
        _flag: bool,
        
        // Optional types
        #[param(description = "Optional string")]
        _optional_text: Option<String>,
        
        #[param(description = "Optional number")]
        _optional_number: Option<i32>,
        
        // Collections
        #[param(description = "List of strings")]
        _string_list: Vec<String>,
        
        #[param(description = "List of numbers")]
        _number_list: Vec<i32>,
        
        // Complex nested types
        #[param(description = "JSON object parameter")]
        _json_data: serde_json::Value,
    }

    impl ComplexParamsTool {
        async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
            Ok(format!("Processed complex parameters: {}", self.text))
        }
    }

    // Verify complex parameter types compile with derive macro
    let _server = McpServer::builder()
        .name("complex-server")
        .version("1.0.0")
        .tool(ComplexParamsTool::default())
        .bind_address("127.0.0.1:8080".parse().unwrap())
        .build()
        .expect("Server should build successfully");
}

/// Test parameter validation example from turul-mcp-derive README
#[test]
fn test_parameter_validation() {
    #[derive(McpTool, Clone, Default)]
    #[tool(name = "validated_tool", description = "Tool with parameter validation")]
    struct ValidatedTool {
        #[param(description = "Email address")]
        email: String,
        #[param(description = "Age in years (1-120)")]
        age: u8,
    }

    impl ValidatedTool {
        async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
            // Validation logic
            if !self.email.contains('@') {
                return Err("Invalid email format".into());
            }
            
            if self.age == 0 || self.age > 120 {
                return Err("Age must be between 1 and 120".into());
            }
            
            Ok(format!("Valid user: {} (age {})", self.email, self.age))
        }
    }

    // Verify validation tool compiles
    let _server = McpServer::builder()
        .name("validation-server")
        .version("1.0.0")
        .tool(ValidatedTool::default())
        .bind_address("127.0.0.1:8080".parse().unwrap())
        .build()
        .expect("Server should build successfully");
}
