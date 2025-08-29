# Macro Calculator Server Example

A streamlined calculator implementation demonstrating the power of **MCP derive macros** for rapid tool development with minimal boilerplate code. This example shows how derive macros dramatically simplify MCP tool creation.

## Overview

This server implements the same mathematical operations as the basic calculator-server but uses **`#[derive(McpTool)]`** macros to eliminate boilerplate code. It demonstrates how modern Rust macros can make MCP development faster and more maintainable.

## Features

### ⚡ **Derive Macro Powered**
- **`#[derive(McpTool)]`** - Automatic trait implementation
- **Zero boilerplate** - No manual schema or parameter extraction
- **Compile-time validation** - Type-safe parameter handling
- **Automatic documentation** - Parameter descriptions from attributes

### 🧮 **Mathematical Operations**
- **Addition** - Add two floating-point numbers
- **Subtraction** - Subtract two floating-point numbers
- **Clean, readable code** with automatic error handling

## Quick Start

### 1. Start the Server

```bash
cargo run --bin macro-calculator
```

The server will start on `http://127.0.0.1:8765/mcp`

### 2. Test the Macro-Generated Tools

#### Addition
```json
{
  "name": "add",
  "arguments": {
    "a": 25.5,
    "b": 14.3
  }
}
```

#### Subtraction
```json
{
  "name": "subtract",
  "arguments": {
    "a": 100.7,
    "b": 23.2
  }
}
```

## Tool Reference

### ➕ `add`

Adds two numbers using derive macro implementation.

**Parameters:**
- `a` (required): First number (f64)
- `b` (required): Second number (f64)

**Response:**
```
"25.5 + 14.3 = 39.8"
```

### ➖ `subtract`

Subtracts the second number from the first using derive macro.

**Parameters:**
- `a` (required): First number (f64)  
- `b` (required): Second number (f64)

**Response:**
```
"100.7 - 23.2 = 77.5"
```

## Derive Macro Implementation

### Before (Manual Implementation)
```rust
// Manual trait implementation - 50+ lines of code
#[derive(Clone)]
struct AddTool;

#[async_trait]
impl McpTool for AddTool {
    fn name(&self) -> &str { "add" }
    
    fn description(&self) -> &str { "Add two numbers together" }
    
    fn input_schema(&self) -> ToolSchema {
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("a".to_string(), JsonSchema::number_with_description("First number")),
                ("b".to_string(), JsonSchema::number_with_description("Second number")),
            ]))
            .with_required(vec!["a".to_string(), "b".to_string()])
    }
    
    async fn call(&self, args: Value, _session: Option<SessionContext>) -> Result<Vec<ToolResult>, String> {
        let a = args.get("a").and_then(|v| v.as_f64()).ok_or("Missing parameter 'a'")?;
        let b = args.get("b").and_then(|v| v.as_f64()).ok_or("Missing parameter 'b'")?;
        let result = a + b;
        Ok(vec![ToolResult::text(format!("{} + {} = {}", a, b, result))])
    }
}
```

### After (Derive Macro Implementation)
```rust
// Derive macro implementation - 8 lines of code!
#[derive(McpTool, Clone)]
#[tool(name = "add", description = "Add two numbers together")]
struct AddTool {
    #[param(description = "First number")]
    a: f64,
    #[param(description = "Second number")]
    b: f64,
}

impl AddTool {
    async fn execute(&self) -> McpResult<String> {
        Ok(format!("{} + {} = {}", self.a, self.b, self.a + self.b))
    }
}
```

## Key Benefits

### 🚀 **Dramatic Code Reduction**
- **85% less code** compared to manual implementation
- **No boilerplate** - Focus on business logic only
- **Type safety** - Compile-time parameter validation
- **Automatic schema generation** from Rust types

### 🔧 **Developer Experience**
- **Faster development** - Less code to write and maintain
- **Fewer bugs** - Automatic parameter extraction and validation
- **Better readability** - Clear, declarative syntax
- **IDE support** - Full autocomplete and error checking

### ⚡ **Performance**
- **Zero runtime overhead** - All macro expansion happens at compile time
- **Optimized code generation** - Same performance as manual implementation
- **Type-safe parameters** - No runtime type checking needed

## Attribute Reference

### `#[tool(...)]` Attributes
- `name = "..."` - Tool name for MCP registration
- `description = "..."` - Human-readable tool description

### `#[param(...)]` Attributes  
- `description = "..."` - Parameter description for documentation
- Additional constraints can be added (validation, ranges, etc.)

## Generated Code Features

The derive macro automatically generates:

1. **`McpTool` trait implementation**
2. **JSON schema generation** from Rust types
3. **Parameter extraction** with type safety
4. **Error handling** for missing/invalid parameters
5. **Documentation integration** from attributes

## Type Support

The derive macro supports all standard Rust types:

```rust
#[derive(McpTool, Clone)]
#[tool(name = "example", description = "Type examples")]
struct ExampleTool {
    #[param(description = "String parameter")]
    text: String,
    
    #[param(description = "Integer parameter")]
    count: i32,
    
    #[param(description = "Float parameter")]  
    value: f64,
    
    #[param(description = "Boolean parameter")]
    enabled: bool,
    
    #[param(description = "Optional parameter")]
    optional_field: Option<String>,
}
```

## Server Configuration

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().init();

    let server = McpServer::builder()
        .name("macro-calculator")
        .version("1.0.0")
        .title("Macro-Powered Calculator")
        .instructions("Calculator using derive macros for rapid development")
        .tool(AddTool { a: 0.0, b: 0.0 })
        .tool(SubtractTool { a: 0.0, b: 0.0 })
        .bind_address("127.0.0.1:8765".parse()?)
        .build()?;

    server.run().await?;
    Ok(())
}
```

## Error Handling

The derive macro provides automatic error handling:

### Invalid Parameter Type
```json
{
  "name": "add",
  "arguments": {
    "a": "not-a-number",
    "b": 5.0
  }
}
```
**Response:** `"Invalid parameter type for 'a'"`

### Missing Required Parameter
```json
{
  "name": "add", 
  "arguments": {
    "a": 10.0
  }
}
```
**Response:** `"Missing parameter 'b'"`

## Comparison with Other Examples

| Feature | Manual Implementation | Derive Macro | Declarative Macro |
|---------|---------------------|---------------|------------------|
| Lines of Code | ~50 per tool | ~8 per tool | ~12 per tool |
| Type Safety | Manual validation | Automatic | Automatic |
| Schema Generation | Manual | Automatic | Automatic |
| Parameter Extraction | Manual | Automatic | Automatic |
| Error Handling | Manual | Automatic | Automatic |
| Learning Curve | Complex | Simple | Moderate |

## Testing

```bash
# Start the server
cargo run --bin macro-calculator &

# Test addition
curl -X POST http://127.0.0.1:8765/mcp \
  -H "Content-Type: application/json" \
  -d '{"method": "tools/call", "params": {"name": "add", "arguments": {"a": 10.5, "b": 5.3}}}'

# Test subtraction
curl -X POST http://127.0.0.1:8765/mcp \
  -H "Content-Type: application/json" \
  -d '{"method": "tools/call", "params": {"name": "subtract", "arguments": {"a": 20.0, "b": 7.5}}}'
```

## Use Cases

### 1. **Rapid Prototyping**
Perfect for quickly building MCP tools without boilerplate code.

### 2. **Production Development**
Ideal for production servers where code maintainability is crucial.

### 3. **Team Development**  
Reduces complexity for team members less familiar with MCP internals.

### 4. **Type-Safe APIs**
Ensures compile-time validation of tool parameters and schemas.

### 5. **Documentation Generation**
Automatic documentation from code attributes.

## Architecture

```
┌─────────────────────┐    ┌──────────────────────┐    ┌─────────────────────┐
│   MCP Client        │────│  Macro Calculator    │────│  Derive Macros      │
│                     │    │                      │    │                     │
│ - Tool Calls        │    │ - AddTool           │    │ - Code Generation   │
│ - Type Validation   │    │ - SubtractTool       │    │ - Schema Creation   │
│ - Error Handling    │    │ - Auto-generated     │    │ - Parameter Extract │
│                     │    │   implementations    │    │ - Error Handling    │
└─────────────────────┘    └──────────────────────┘    └─────────────────────┘
```

This example demonstrates how derive macros can dramatically simplify MCP tool development while maintaining full type safety and performance, making it the preferred approach for most MCP server implementations.