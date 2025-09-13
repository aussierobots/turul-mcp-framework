# turul-mcp-builders

[![Crates.io](https://img.shields.io/crates/v/turul-mcp-builders.svg)](https://crates.io/crates/turul-mcp-builders)
[![Documentation](https://docs.rs/turul-mcp-builders/badge.svg)](https://docs.rs/turul-mcp-builders)

Runtime construction library providing building blocks for MCP components. For server integration, use `turul_mcp_server::ToolBuilder` which wraps these builders.

## Overview

`turul-mcp-builders` provides **runtime flexibility** for building MCP components when you need dynamic, configuration-driven systems. Perfect for tools loaded from config files, user-defined workflows, or plugin architectures.

## Features

- ✅ **Complete MCP Coverage** - 9 builders for ALL MCP protocol areas
- ✅ **Runtime Flexibility** - Build components entirely at runtime
- ✅ **Type Safety** - Full parameter validation and schema generation
- ✅ **Configuration-Driven** - Perfect for config-file-based systems
- ✅ **Trait Integration** - All builders produce framework-compatible types
- ✅ **70+ Tests** - Comprehensive test coverage with zero warnings

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
turul-mcp-builders = "0.2.0"
turul-mcp-server = "0.2.0"
serde_json = "1.0"
```

### Basic Tool Builder

**Note**: For direct server integration, use `turul_mcp_server::ToolBuilder` instead, which wraps this crate's functionality.

```rust
// For server integration - use turul_mcp_server::ToolBuilder
use turul_mcp_server::{McpServer, ToolBuilder};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a calculator tool at runtime
    let calculator = ToolBuilder::new("calculator")
        .description("Add two numbers")
        .number_param("a", "First number")
        .number_param("b", "Second number")
        .number_output()
        .execute(|args| async move {
            let a = args.get("a").and_then(|v| v.as_f64())
                .ok_or("Missing parameter 'a'")?;
            let b = args.get("b").and_then(|v| v.as_f64())
                .ok_or("Missing parameter 'b'")?;
            Ok(json!({"result": a + b}))
        })
        .build()?;

    // Use in server
    let server = McpServerBuilder::new()
        .tool(calculator)
        .build()?;
        
    Ok(())
}
```

## Complete Builder Coverage

### 1. ToolBuilder - Dynamic Tool Construction

```rust
use turul_mcp_builders::ToolBuilder;

let tool = ToolBuilder::new("data_processor")
    .description("Process data with custom logic")
    .string_param("input", "Input data to process")
    .optional_string_param("format", "Output format (json, csv, xml)")
    .boolean_param("validate", "Validate input data")
    .execute(|args| async move {
        let input = args.get("input").and_then(|v| v.as_str())
            .ok_or("Missing parameter 'input'")?;
        let format = args.get("format").and_then(|v| v.as_str())
            .unwrap_or("json");
        let validate = args.get("validate").and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        // Custom processing logic
        let result = process_data(input, format, validate).await?;
        Ok(json!({"processed": result, "format": format}))
    })
    .build()?;
```

### 2. ResourceBuilder - Runtime Resource Creation

```rust
use turul_mcp_builders::ResourceBuilder;

let config_resource = ResourceBuilder::new("file:///app/config.json")
    .name("app_config")
    .description("Application configuration")
    .json_content(json!({
        "version": "1.0.0",
        "features": ["logging", "metrics", "auth"]
    }))
    .build()?;
```

### 3. PromptBuilder - Template-based Prompts

```rust
use turul_mcp_builders::PromptBuilder;

let greeting_prompt = PromptBuilder::new("personalized_greeting")
    .description("Generate personalized greetings")
    .string_argument("name", "Person to greet")
    .optional_string_argument("style", "Greeting style (formal, casual, friendly)")
    .user_message("Please create a {style} greeting for {name}")
    .assistant_message("I'll create a personalized greeting for you.")
    .build()?;
```

### 4. MessageBuilder - Sampling Messages

```rust
use turul_mcp_builders::MessageBuilder;
use serde_json::json;

let params = MessageBuilder::new()
    .system("You are a helpful geography assistant.")
    .user_text("What is the capital of France?")
    .assistant_text("The capital of France is Paris.")
    .temperature(0.7)
    .max_tokens(1000)
    .metadata(json!({
        "topic": "geography",
        "difficulty": "basic"
    }))
    .build_params();

// Or build a complete sampling request
let request = MessageBuilder::new()
    .user_text("What is the capital of France?")
    .temperature(0.7)
    .build_request();
```

### 5. CompletionBuilder - Autocompletion Context

```rust
use turul_mcp_builders::CompletionBuilder;

let completion = CompletionBuilder::for_prompt("code_completion")
    .argument("language", "rust")
    .argument("context", "struct definition")
    .build()?;
```

### 6. RootBuilder - Directory Permissions  

```rust
use turul_mcp_builders::RootBuilder;

let project_root = RootBuilder::new("file:///workspace/my-project")
    .name("project_files")
    .build()?;
```

### 7. ElicitationBuilder - User Input Forms

```rust
use turul_mcp_builders::ElicitationBuilder;
use turul_mcp_protocol::elicitation::StringFormat;

let user_form = ElicitationBuilder::new("Configure your application preferences")
    .title("User Preferences")
    .string_field("name", "Full Name")
    .string_field_with_format("email", "Email Address", StringFormat::Email)
    .enum_field("theme", "UI Theme", vec!["light".to_string(), "dark".to_string(), "auto".to_string()])
    .require_field("name")  // Make name required
    .require_field("email") // Make email required
    .build();
```

### 8. NotificationBuilder - MCP Notifications

```rust
use turul_mcp_builders::NotificationBuilder;

// Progress notification
let progress = NotificationBuilder::progress("task-123", 75)
    .total(100)
    .message("Processing files...")
    .build();

// Log notification  
let log = NotificationBuilder::log("info", json!({
    "message": "Operation completed",
    "duration_ms": 1250
}))
.logger("data_processor")
.build();
```

### 9. LoggingBuilder - Structured Logging

```rust
use turul_mcp_builders::LoggingBuilder;

let error_log = LoggingBuilder::error(json!({
    "error": "Database connection failed",
    "retry_count": 3,
    "connection_string": "postgresql://***",
    "error_code": "CONNECTION_TIMEOUT"
}))
.logger("database")
.meta_value("session_id", json!("sess-456"))
.meta_value("user_id", json!("user-789"))
.build();
```

## Configuration-Driven Systems

### Loading Tools from JSON Config

```rust
use turul_mcp_builders::ToolBuilder;
use serde_json::Value;

async fn load_tools_from_config(config_path: &str) -> Result<Vec<Box<dyn turul_mcp_server::McpTool>>, Box<dyn std::error::Error>> {
    let config: Value = serde_json::from_str(&std::fs::read_to_string(config_path)?)?;
    let mut tools = Vec::new();
    
    for tool_config in config["tools"].as_array()
        .ok_or("Missing 'tools' array in config")? {
        let name = tool_config["name"].as_str()
            .ok_or("Missing tool 'name' field")?;
        let description = tool_config["description"].as_str()
            .ok_or("Missing tool 'description' field")?;
        let mut builder = ToolBuilder::new(name).description(description);
            
        // Add parameters from config
        for param in tool_config["parameters"].as_array()
            .ok_or("Missing 'parameters' array")? {
            let name = param["name"].as_str()
                .ok_or("Missing parameter 'name' field")?;
            let description = param["description"].as_str()
                .ok_or("Missing parameter 'description' field")?;
            let param_type = param["type"].as_str()
                .ok_or("Missing parameter 'type' field")?;
            
            builder = match param_type {
                "string" => builder.string_param(name, description),
                "number" => builder.number_param(name, description),  
                "boolean" => builder.boolean_param(name, description),
                _ => return Err(format!("Unknown parameter type: {}", param_type).into()),
            };
        }
        
        // Add execution logic based on tool type
        let tool_type = tool_config["type"].as_str()
            .ok_or("Missing tool 'type' field")?;
        builder = match tool_type {
            "calculator" => builder.execute(calculator_logic()),
            "file_reader" => builder.execute(file_reader_logic()),
            "api_caller" => builder.execute(api_caller_logic()),
            _ => return Err(format!("Unknown tool type: {}", tool_type).into()),
        };
        
        tools.push(Box::new(builder.build()?) as Box<dyn turul_mcp_server::McpTool>);
    }
    
    Ok(tools)
}

// Example config.json:
{
  "tools": [
    {
      "name": "add_numbers",
      "description": "Add two numbers together",
      "type": "calculator", 
      "parameters": [
        {"name": "a", "type": "number", "description": "First number"},
        {"name": "b", "type": "number", "description": "Second number"}
      ]
    }
  ]
}
```

### Plugin Architecture

```rust
use turul_mcp_builders::ToolBuilder;

trait Plugin {
    fn create_tools(&self) -> Vec<Box<dyn turul_mcp_server::McpTool>>;
}

struct MathPlugin;

impl Plugin for MathPlugin {
    fn create_tools(&self) -> Result<Vec<Box<dyn turul_mcp_server::McpTool>>, Box<dyn std::error::Error>> {
        Ok(vec![
            Box::new(ToolBuilder::new("add")
                .description("Add two numbers")
                .number_param("a", "First number")
                .number_param("b", "Second number") 
                .execute(|args| async move {
                    let a = args["a"].as_f64()
                        .ok_or("Parameter 'a' must be a number")?;
                    let b = args["b"].as_f64()
                        .ok_or("Parameter 'b' must be a number")?;
                    Ok(json!({"result": a + b}))
                })
                .build()
                .map_err(|e| format!("Failed to build add tool: {}", e))?),
                
            Box::new(ToolBuilder::new("multiply")
                .description("Multiply two numbers")
                .number_param("x", "First number")
                .number_param("y", "Second number")
                .execute(|args| async move {
                    let x = args["x"].as_f64()
                        .ok_or("Parameter 'x' must be a number")?;
                    let y = args["y"].as_f64()
                        .ok_or("Parameter 'y' must be a number")?; 
                    Ok(json!({"result": x * y}))
                })
                .build()
                .map_err(|e| format!("Failed to build multiply tool: {}", e))?),
        ])
    }
}
```

## Advanced Usage

### Complex Parameter Validation

```rust
use turul_mcp_builders::ToolBuilder;
use serde_json::json;

let validated_tool = ToolBuilder::new("user_registration")
    .description("Register a new user with validation")
    .string_param("email", "User email address")
    .string_param("password", "User password") 
    .optional_number_param("age", "User age")
    .execute(|args| async move {
        let email = args.get("email").and_then(|v| v.as_str())
            .ok_or("Missing required parameter 'email'")?;
        let password = args.get("password").and_then(|v| v.as_str())
            .ok_or("Missing required parameter 'password'")?;
        let age = args.get("age").and_then(|v| v.as_f64());
        
        // Validation logic
        if !email.contains('@') {
            return Err("Invalid email format".into());
        }
        
        if password.len() < 8 {
            return Err("Password must be at least 8 characters".into());
        }
        
        if let Some(age) = age {
            if age < 13.0 || age > 120.0 {
                return Err("Age must be between 13 and 120".into());
            }
        }
        
        // Registration logic
        let user_id = register_user(email, password, age).await?;
        
        Ok(json!({
            "user_id": user_id,
            "email": email,
            "status": "registered"
        }))
    })
    .build()?;
```

### Error Handling Patterns

```rust
use turul_mcp_builders::ToolBuilder;

let robust_tool = ToolBuilder::new("file_processor")
    .description("Process files with comprehensive error handling")
    .string_param("file_path", "Path to file to process")
    .execute(|args| async move {
        let file_path = args.get("file_path")
            .and_then(|v| v.as_str())
            .ok_or("Missing file_path parameter")?;
            
        // File validation
        if !std::path::Path::new(file_path).exists() {
            return Err(format!("File not found: {}", file_path).into());
        }
        
        // Processing with error recovery
        match tokio::fs::read_to_string(file_path).await {
            Ok(content) => {
                let processed = process_file_content(&content).await?;
                Ok(json!({"status": "success", "result": processed}))
            }
            Err(e) => {
                Err(format!("Failed to read file {}: {}", file_path, e).into())
            }
        }
    })
    .build()?;
```

### Meta Information and Context

```rust
use turul_mcp_builders::*;
use std::collections::HashMap;

// Tool with metadata
let documented_tool = ToolBuilder::new("api_client")
    .description("Call external API with full documentation") 
    .string_param("endpoint", "API endpoint URL")
    .optional_string_param("method", "HTTP method (GET, POST, PUT, DELETE)")
    .meta_value("version", json!("1.2.0"))
    .meta_value("author", json!("api-team"))
    .meta_value("tags", json!(["external", "network", "api"]))
    .execute(|args| async move {
        // Implementation
        Ok(json!({"status": "called"}))
    })
    .build()?;
```

## Integration with Framework

### All Builders are Framework-Compatible

```rust
use turul_mcp_server::McpServer;
use turul_mcp_builders::*;

let server = McpServerBuilder::new()
    .tool(ToolBuilder::new("calc").build()?)           // ToolDefinition
    .resource(ResourceBuilder::new("uri").build()?)    // ResourceDefinition
    .prompt(PromptBuilder::new("template").build()?)   // PromptDefinition
    .build()?;
```

### Type Safety Guarantees

All builders produce types that implement the framework's definition traits:

- `ToolBuilder` → implements `ToolDefinition`
- `ResourceBuilder` → implements `ResourceDefinition` 
- `PromptBuilder` → implements `PromptDefinition`
- etc.

This ensures complete compatibility with the framework's trait system.

## Performance Considerations

### Runtime vs Compile-time

- **Schema Generation**: Runtime (flexible but has cost)
- **Parameter Parsing**: Runtime (same as other approaches)
- **Type Safety**: Compile-time (guaranteed via traits)

### Memory Usage

- Builders store configuration until `.build()` is called
- Built objects are optimized for runtime usage
- Consider caching built objects for repeated use

### When to Use Builders

Choose builders when you need:

✅ **Configuration-driven tools** - Loading from JSON/YAML  
✅ **Plugin architectures** - Dynamic tool registration
✅ **User-defined workflows** - End-user tool creation
✅ **A/B testing** - Runtime tool variations  
✅ **Complex business logic** - Multi-step tool construction

Avoid builders for:
❌ **Simple static tools** - Use function or derive macros instead
❌ **Performance-critical paths** - Compile-time approaches are faster
❌ **Type-heavy applications** - Compile-time validation better

## Testing

### Builder Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use turul_mcp_builders::ToolBuilder;
    
    #[tokio::test]
    async fn test_tool_builder() {
        let tool = ToolBuilder::new("test_tool")
            .description("Test tool")
            .string_param("input", "Test input")
            .execute(|args| async move {
                Ok(json!({"echo": args["input"]}))
            })
            .build()
            .expect("Tool should build successfully");
            
        // Test the built tool
        let args = json!({"input": "hello"});
        let result = tool.call(args, None).await
            .expect("Tool call should succeed");
        // Assert result format
    }
}
```

### Framework Integration Tests

```bash
# Run all builder tests
cargo test --package turul-mcp-builders

# Test specific builder
cargo test --package turul-mcp-builders tool_builder

# Integration tests with server
cargo test --package turul-mcp-builders --features integration
```

## Examples

See the [`examples/builders-showcase`](../../examples/builders-showcase) directory for a comprehensive demonstration of all 9 builders in action.

## License

Licensed under the MIT License. See [LICENSE](../../LICENSE) for details.