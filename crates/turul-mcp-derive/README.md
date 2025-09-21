# turul-mcp-derive

[![Crates.io](https://img.shields.io/crates/v/turul-mcp-derive.svg)](https://crates.io/crates/turul-mcp-derive)
[![Documentation](https://docs.rs/turul-mcp-derive/badge.svg)](https://docs.rs/turul-mcp-derive)

Procedural macros for the turul-mcp-framework, providing zero-configuration creation of MCP tools, resources, and prompts with automatic schema generation.

## Overview

`turul-mcp-derive` provides macro approaches for creating MCP components:

### Tools
1. **Function Macros** (`#[mcp_tool]`) - Ultra-simple tool creation from functions
2. **Derive Macros** (`#[derive(McpTool)]`) - Struct-based tools with complex logic

### Resources
3. **Function Macros** (`#[mcp_resource]`) - Create resources from async functions with URI templates

### Prompts
4. **Derive Macros** (`#[derive(McpPrompt)]`) - Structured prompt definitions with arguments

All approaches generate required traits automatically and provide compile-time validation.

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
    .name("calculator-server")
    .version("1.0.0")
    .tool_fn(calculator)  // Framework knows the function name!
    .bind_address("127.0.0.1:8080".parse()?)
    .build()?;

server.run().await
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
        let count: i32 = session.get_typed_state("count").await.unwrap_or(0);
        let new_count = count + 1;
        session.set_typed_state("count", new_count).await
            .map_err(|e| format!("Failed to save state: {}", e))?;
        Ok(new_count)
    } else {
        Ok(0) // No session available
    }
}
```

### Custom Output Fields

**Note**: `output_field` only affects structured output generation - the field name used when the return value is wrapped in a JSON object for structured responses.

```rust
#[mcp_tool(
    name = "multiply", 
    description = "Multiply two numbers",
    output_field = "product"  // Custom output field name (also supports: field = "product")
)]
async fn multiply(
    #[param(description = "First number")] x: f64,
    #[param(description = "Second number")] y: f64,
) -> McpResult<f64> {
    Ok(x * y)  // Returns {"product": 15.0} instead of {"result": 15.0} in structured output
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
            session.notify_progress("slow-task", i as u64).await;
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
#[tool(name = "calculator", description = "Advanced calculator", output_field = "result")]
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
            session.notify_progress("lookup", 25).await;
        }
        
        // Simulate database lookup
        let user = self.lookup_user_in_database(&self.user_id).await?;
        
        if let Some(session) = session {
            session.notify_progress("lookup", 75).await;
        }
        
        let result = if include_details {
            self.get_detailed_profile(user).await?
        } else {
            self.get_basic_profile(user).await?
        };
        
        if let Some(session) = session {
            session.notify_progress("lookup", 100).await;
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
        return Err(McpError::InvalidParameters("Value must be non-negative".to_string()));
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
    async fn call(&self, args: Value, session: Option<SessionContext>) -> McpResult<CallToolResult> {
        // Generated parameter extraction and function call
        let result = example(/* extracted params */).await?;
        Ok(CallToolResult::success(/* wrapped result */))
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
        
        let result = tool.execute(None).await
            .expect("Tool execution should succeed");
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
        .expect("Server should build successfully");
        
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

## Prompts - Level 3

### Basic Prompt Derive

```rust
use turul_mcp_derive::McpPrompt;
use turul_mcp_server::{McpResult, McpPrompt};
use async_trait::async_trait;

#[derive(McpPrompt, Clone, Serialize, Deserialize, Debug)]
#[prompt(name = "code_review", description = "Review code for quality and security")]
struct CodeReviewPrompt {
    #[argument(description = "Programming language")]
    language: String,

    #[argument(description = "Code to review")]
    code: String,

    #[argument(description = "Review focus (optional)")]
    focus: Option<String>,
}

// Users must implement McpPrompt manually for custom render logic
#[async_trait]
impl McpPrompt for CodeReviewPrompt {
    async fn render(&self, _args: Option<HashMap<String, Value>>) -> McpResult<Vec<PromptMessage>> {
        let focus = self.focus.as_deref().unwrap_or("comprehensive");

        let prompt = format!(
            "You are an expert {} developer. Provide {} code review for:\n\n{}",
            self.language, focus, self.code
        );

        Ok(vec![PromptMessage::user_text(&prompt)])
    }
}

// Server usage
let server = McpServer::builder()
    .name("prompt-server")
    .version("1.0.0")
    .prompt(CodeReviewPrompt::default())
    .bind_address("127.0.0.1:8080".parse()?)
    .build()?;

server.run().await
```

### Default vs Custom Render

The `#[derive(McpPrompt)]` macro generates metadata traits only. For custom behavior:

**Option 1: Use Default Render** (for simple prompts)
```rust
#[derive(McpPrompt)]
#[prompt(name = "simple", description = "Simple prompt")]
struct SimplePrompt;

// No manual implementation needed - uses trait default
// Returns: "Prompt: simple - Simple prompt"
```

**Option 2: Custom Render** (for complex prompts)
```rust
#[derive(McpPrompt)]
#[prompt(name = "database_query", description = "Query database and format results")]
struct DatabasePrompt {
    #[argument(description = "SQL query")]
    query: String,
}

#[async_trait]
impl McpPrompt for DatabasePrompt {
    async fn render(&self, args: Option<HashMap<String, Value>>) -> McpResult<Vec<PromptMessage>> {
        // Custom database logic
        let results = db.execute(&self.query).await?;
        let formatted = format_results(results);
        Ok(vec![PromptMessage::user_text(&formatted)])
    }
}
```

### Prompt Patterns

**Template Substitution**
```rust
#[async_trait]
impl McpPrompt for TemplatePrompt {
    async fn render(&self, args: Option<HashMap<String, Value>>) -> McpResult<Vec<PromptMessage>> {
        let args = args.unwrap_or_default();
        let user_name = args.get("user_name").and_then(|v| v.as_str()).unwrap_or("User");

        let message = format!("Hello {}! {}", user_name, self.template);
        Ok(vec![PromptMessage::user_text(&message)])
    }
}
```

**Multi-Modal Content**
```rust
#[async_trait]
impl McpPrompt for MultiModalPrompt {
    async fn render(&self, _args: Option<HashMap<String, Value>>) -> McpResult<Vec<PromptMessage>> {
        Ok(vec![
            PromptMessage::user_text("Analyze this image:"),
            PromptMessage {
                role: Role::User,
                content: ContentBlock::Image {
                    data: self.image_data.clone(),
                    mime_type: "image/png".to_string(),
                },
            },
        ])
    }
}
```

## Resources - Level 3

### Resource Function Macro

The `#[mcp_resource]` function attribute macro allows creating MCP resources from regular async functions with automatic URI template detection and parameter extraction.

```rust
use turul_mcp_derive::mcp_resource;
use turul_mcp_server::{McpResult, McpServer};
use turul_mcp_protocol::resources::ResourceContent;

// Static resource - no template variables
#[mcp_resource(
    uri = "file:///config.json",
    name = "config",
    description = "Application configuration file"
)]
async fn get_config() -> McpResult<Vec<ResourceContent>> {
    let config = serde_json::json!({
        "app_name": "My App",
        "version": "1.0.0",
        "debug": true
    });

    Ok(vec![ResourceContent::blob(
        "file:///config.json",
        serde_json::to_string_pretty(&config).unwrap(),
        "application/json".to_string()
    )])
}

// Template resource with automatic parameter extraction
#[mcp_resource(
    uri = "file:///users/{user_id}.json",
    name = "user_profile",
    description = "User profile data for a specific user ID"
)]
async fn get_user_profile(user_id: String) -> McpResult<Vec<ResourceContent>> {
    let profile = serde_json::json!({
        "user_id": user_id,
        "username": format!("user_{}", user_id),
        "email": format!("user_{}@example.com", user_id)
    });

    Ok(vec![ResourceContent::blob(
        format!("file:///users/{}.json", user_id),
        serde_json::to_string_pretty(&profile).unwrap(),
        "application/json".to_string()
    )])
}

// Resource with additional parameters
#[mcp_resource(
    uri = "file:///logs/{log_type}.log",
    name = "log_entries",
    description = "Log entries filtered by type and level"
)]
async fn get_log_entries(log_type: String, params: serde_json::Value) -> McpResult<Vec<ResourceContent>> {
    let level = params.get("level")
        .and_then(|v| v.as_str())
        .unwrap_or("info");

    let entries = format!("2024-01-01 10:00:00 {} [{}] Sample log entry", level.to_uppercase(), log_type);

    Ok(vec![ResourceContent::text(
        format!("file:///logs/{}.log", log_type),
        entries
    )])
}
```

### Server Integration with resource_fn

The `#[mcp_resource]` macro generates both the resource implementation and a constructor function for easy registration:

```rust
let server = McpServer::builder()
    .name("resource-server")
    .version("1.0.0")
    .resource_fn(get_config)       // Static resource
    .resource_fn(get_user_profile) // Template: file:///users/{user_id}.json
    .resource_fn(get_log_entries)  // Template with params
    .bind_address("127.0.0.1:8080".parse()?)
    .build()?;

server.run().await
```

### Template Variable Extraction

The framework automatically extracts template variables from the URI pattern and maps them to function parameters:

```rust
// URI: file:///data/{category}/{item_id}.json
#[mcp_resource(
    uri = "file:///data/{category}/{item_id}.json",
    name = "data_item",
    description = "Data item by category and ID"
)]
async fn get_data_item(category: String, item_id: String) -> McpResult<Vec<ResourceContent>> {
    // category and item_id are automatically extracted from template_variables
    let data = serde_json::json!({
        "category": category,
        "item_id": item_id,
        "content": format!("Data for {} item {}", category, item_id)
    });

    Ok(vec![ResourceContent::blob(
        format!("file:///data/{}/{}.json", category, item_id),
        serde_json::to_string_pretty(&data).unwrap(),
        "application/json".to_string()
    )])
}
```

### Resource Macro Features

- ✅ **Automatic McpResource Implementation** - Generates all required traits
- ✅ **URI Template Detection** - Auto-detects `{variable}` patterns
- ✅ **Parameter Extraction** - Maps template variables to function parameters
- ✅ **Static Resource Support** - Handles resources without templates
- ✅ **Custom MIME Types** - Optional `mime_type` attribute
- ✅ **Additional Parameters** - Supports `params: serde_json::Value` for extra data

### Resource Attributes

```rust
#[mcp_resource(
    uri = "required://absolute/path",           // Required: Absolute URI
    name = "resource_name",                     // Optional: Defaults to function name
    description = "Resource description",       // Optional: Defaults to generated text
    mime_type = "application/json"              // Optional: MIME type hint
)]
```

### Alternative: Constructor Function Pattern

For complex resources or when the macro doesn't fit your needs, use the constructor function pattern:

```rust
use turul_mcp_server::{McpServer, McpResource};
use turul_mcp_protocol::resources::*;

// Define resource struct
struct ConfigResource;

impl HasResourceMetadata for ConfigResource {
    fn name(&self) -> &str { "config" }
}

impl HasResourceUri for ConfigResource {
    fn uri(&self) -> &str { "file:///config.json" }
}

// ... implement other required traits

#[async_trait]
impl McpResource for ConfigResource {
    async fn read(&self, _params: Option<serde_json::Value>) -> McpResult<Vec<ResourceContent>> {
        // Custom implementation
        Ok(vec![])
    }
}

// Constructor function
fn create_config_resource() -> ConfigResource {
    ConfigResource
}

// Register with .resource_fn()
let server = McpServer::builder()
    .name("config-server")
    .version("1.0.0")
    .resource_fn(create_config_resource)  // Uses constructor function
    .bind_address("127.0.0.1:8080".parse()?)
    .build()?;

server.run().await
```

### Comparison: Macro vs Manual Implementation

| Feature | `#[mcp_resource]` Macro | Manual Implementation |
|---------|------------------------|----------------------|
| **Setup Time** | Minimal - just attributes | More setup required |
| **Template Variables** | Automatic extraction | Manual parameter handling |
| **Type Safety** | Compile-time validation | Manual validation needed |
| **Flexibility** | Good for common patterns | Full control over behavior |
| **Generated Code** | Automatic trait impls | Manual trait implementations |
| **Registration** | `.resource_fn(function_name)` | `.resource_fn(constructor)` |

Choose the macro for rapid development and standard patterns. Use manual implementation for complex business logic or when you need full control over resource behavior.

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
