# turul-mcp-derive

[![Crates.io](https://img.shields.io/crates/v/turul-mcp-derive.svg)](https://crates.io/crates/turul-mcp-derive)
[![Documentation](https://docs.rs/turul-mcp-derive/badge.svg)](https://docs.rs/turul-mcp-derive)

Procedural macros for the turul-mcp-framework, providing zero-configuration tool creation with automatic schema generation.

## Overview

`turul-mcp-derive` provides two main macro approaches for creating MCP tools:

1. **Function Macros** (`#[mcp_tool]`) - Ultra-simple tool creation from functions
2. **Derive Macros** (`#[derive(McpTool)]`) - Struct-based tools with complex logic

Both approaches generate all required traits automatically and provide compile-time schema validation.

## Features

- ✅ **Zero Configuration** - No method strings, framework auto-determines everything
- ✅ **Compile-time Validation** - Schema errors caught at compile time
- ✅ **SessionContext Support** - Automatic session context passing
- ✅ **Type Safety** - Full Rust type system integration  
- ✅ **JSON Schema Generation** - Automatic OpenAPI-compatible schemas
- ✅ **Custom Output Fields** - Configurable output field names

## Function Macros - Level 1

### Basic Function Tool

```rust
use turul_mcp_derive::mcp_tool;
use turul_mcp_server::McpResult;

#[mcp_tool(name = "calculator", description = "Add two numbers")]
async fn calculator(
    #[param(description = "First number")] a: f64,
    #[param(description = "Second number")] b: f64,
) -> McpResult<f64> {
    Ok(a + b)
}

// Usage: Just pass the function to the server
let server = McpServer::builder()
    .tool_fn(calculator)  // Framework knows the function name!
    .build()?;
```

### SessionContext Integration

```rust
use turul_mcp_derive::mcp_tool;
use turul_mcp_server::{McpResult, SessionContext};

#[mcp_tool(name = "counter", description = "Session-persistent counter")]
async fn session_counter(
    session: Option<SessionContext>  // Automatically detected by macro
) -> McpResult<i32> {
    if let Some(session) = session {
        let count: i32 = session.get_typed_state("count").unwrap_or(0);
        let new_count = count + 1;
        session.set_typed_state("count", new_count)?;
        Ok(new_count)
    } else {
        Ok(0) // No session available
    }
}
```

### Custom Output Fields

```rust
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
```

### Progress Notifications

```rust
#[mcp_tool(name = "slow_task", description = "Task with progress updates")]
async fn slow_task(
    #[param(description = "Number of steps")] steps: u32,
    session: Option<SessionContext>,
) -> McpResult<String> {
    for i in 1..=steps {
        if let Some(ref session) = session {
            session.notify_progress("slow-task", i as u64);
        }
        
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    
    Ok(format!("Completed {} steps", steps))
}
```

## Derive Macros - Level 2

### Basic Struct Tool

```rust
use turul_mcp_derive::McpTool;
use turul_mcp_server::{McpResult, SessionContext};

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
```

### Complex Business Logic

```rust
#[derive(McpTool, Clone, Default)]
#[tool(name = "user_lookup", description = "Look up user information")]
struct UserLookupTool {
    #[param(description = "User ID to lookup")]
    user_id: String,
    #[param(description = "Include detailed profile")]
    include_details: Option<bool>,
}

impl UserLookupTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<serde_json::Value> {
        // Complex business logic with database queries
        let include_details = self.include_details.unwrap_or(false);
        
        if let Some(session) = session {
            session.notify_progress("lookup", 25);
        }
        
        // Simulate database lookup
        let user = self.lookup_user_in_database(&self.user_id).await?;
        
        if let Some(session) = session {
            session.notify_progress("lookup", 75);
        }
        
        let result = if include_details {
            self.get_detailed_profile(user).await?
        } else {
            self.get_basic_profile(user).await?
        };
        
        if let Some(session) = session {
            session.notify_progress("lookup", 100);
        }
        
        Ok(result)
    }
    
    async fn lookup_user_in_database(&self, user_id: &str) -> McpResult<User> {
        // Database implementation
        todo!()
    }
    
    async fn get_detailed_profile(&self, user: User) -> McpResult<serde_json::Value> {
        // Detailed profile logic
        todo!()
    }
    
    async fn get_basic_profile(&self, user: User) -> McpResult<serde_json::Value> {
        // Basic profile logic  
        todo!()
    }
}
```

## Advanced Parameter Types

### Supported Parameter Types

```rust
#[derive(McpTool, Clone, Default)]
#[tool(name = "complex_params", description = "Demonstrate all parameter types")]
struct ComplexParamsTool {
    // Basic types
    #[param(description = "String parameter")]
    text: String,
    
    #[param(description = "Integer parameter")]
    number: i32,
    
    #[param(description = "Float parameter")]
    decimal: f64,
    
    #[param(description = "Boolean parameter")]
    flag: bool,
    
    // Optional types
    #[param(description = "Optional string")]
    optional_text: Option<String>,
    
    #[param(description = "Optional number")]
    optional_number: Option<i32>,
    
    // Collections
    #[param(description = "List of strings")]
    string_list: Vec<String>,
    
    #[param(description = "List of numbers")]
    number_list: Vec<i32>,
    
    // Complex nested types
    #[param(description = "JSON object parameter")]
    json_data: serde_json::Value,
}
```

### Parameter Validation

```rust
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
```

## Schema Generation

### Automatic JSON Schema

The macros automatically generate JSON Schema compatible with MCP Inspector:

```rust
// This struct:
#[derive(McpTool, Clone, Default)]
#[tool(name = "example", description = "Example tool")]
struct ExampleTool {
    #[param(description = "Required string")]
    name: String,
    #[param(description = "Optional number")]
    count: Option<i32>,
}

// Generates this JSON Schema automatically:
{
  "type": "object",
  "properties": {
    "name": {
      "type": "string", 
      "description": "Required string"
    },
    "count": {
      "type": "integer",
      "description": "Optional number"
    }
  },
  "required": ["name"],
  "additionalProperties": false
}
```

### Custom Schema Attributes

```rust
#[derive(McpTool, Clone, Default)]
#[tool(name = "constrained", description = "Tool with schema constraints")]
struct ConstrainedTool {
    #[param(description = "Number between 1 and 100")]
    percentage: f64,  // Could add custom validation attributes in future
}
```

## Error Handling

### Compile-time Validation

The macros provide helpful compile-time error messages:

```rust
// This will fail to compile with helpful error:
#[mcp_tool(name = "bad_tool")]  // Missing description!
async fn bad_tool() -> McpResult<String> {
    Ok("test".to_string())
}
// Error: tool attribute must include description
```

### Runtime Error Integration

```rust
use turul_mcp_server::McpError;

#[mcp_tool(name = "error_example", description = "Demonstrate error handling")]
async fn error_example(
    #[param(description = "Value to validate")] value: i32
) -> McpResult<String> {
    if value < 0 {
        return Err(McpError::InvalidParams("Value must be non-negative".to_string()));
    }
    
    if value > 1000 {
        return Err("Value too large".into());  // String automatically converts to McpError
    }
    
    Ok(format!("Valid value: {}", value))
}
```

## Generated Code

### What the Macros Generate

For a function tool, the macro generates:

1. A wrapper struct implementing `ToolDefinition`
2. All required fine-grained traits (`HasBaseMetadata`, `HasDescription`, etc.)
3. `McpTool` trait implementation with parameter extraction
4. JSON Schema generation code
5. Error handling and response wrapping

### Integration with Framework

The generated code integrates seamlessly with the framework's trait system:

```rust
// Your code:
#[mcp_tool(name = "example", description = "Example")]
async fn example() -> McpResult<String> { Ok("test".to_string()) }

// Generated code (simplified):
struct ExampleTool;

impl HasBaseMetadata for ExampleTool {
    fn name(&self) -> &str { "example" }
}

impl HasDescription for ExampleTool {
    fn description(&self) -> Option<&str> { Some("Example") }
}

impl McpTool for ExampleTool {
    async fn call(&self, args: Value, session: Option<SessionContext>) -> McpResult<CallToolResponse> {
        // Generated parameter extraction and function call
        let result = example(/* extracted params */).await?;
        Ok(CallToolResponse::success(/* wrapped result */))
    }
}
```

## Testing Macros

### Unit Testing Tools

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use turul_mcp_server::SessionContext;
    
    #[tokio::test]
    async fn test_calculator_tool() {
        let tool = Calculator {
            a: 5.0,
            b: 3.0,
            operation: "add".to_string(),
        };
        
        let result = tool.execute(None).await.unwrap();
        assert_eq!(result, 8.0);
    }
}
```

### Integration Testing

```rust
#[tokio::test]
async fn test_tool_in_server() {
    let server = McpServer::builder()
        .tool_fn(calculator)
        .build()
        .unwrap();
        
    // Test server integration
    // (requires test infrastructure)
}
```

## Debugging

### Macro Expansion

To see what code the macros generate:

```bash
# View expanded macros
cargo expand --package your-package

# View specific function
cargo expand your_function_name
```

### Common Issues

1. **Missing SessionContext**: Function macros auto-detect `SessionContext` parameters by type
2. **Parameter Names**: Use exactly the same parameter names in function signature and schema
3. **Return Types**: Must return `McpResult<T>` where `T` implements `Serialize`

## Performance

### Compile-time vs Runtime

- **Schema Generation**: Compile-time (zero runtime cost)
- **Parameter Extraction**: Runtime (optimized JSON parsing)
- **Trait Dispatch**: Compile-time monomorphization

### Memory Usage

- Function tools: Zero-sized types when possible
- Struct tools: Only store necessary parameter data

## Compatibility

### Rust Version

Requires Rust 1.70+ for async fn in traits support.

### Framework Integration

Works with all turul-mcp-framework components:
- `turul-mcp-server` - Core server
- `turul-mcp-aws-lambda` - Lambda integration
- `turul-mcp-builders` - Runtime builders

## License

Licensed under the MIT License. See [LICENSE](../../LICENSE) for details.