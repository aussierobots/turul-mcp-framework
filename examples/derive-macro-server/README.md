# Derive Macro Server Example

This example demonstrates how to use the `#[derive(McpTool)]` macro for simplified tool creation with automatic schema generation from struct fields. Part of the **MCP Framework - Rust Implementation** with support for enhanced parameter constraints and defaults.

## 🚀 What This Example Shows

- **Derive Macro Usage**: Using `#[derive(McpTool)]` for automatic implementation
- **Schema Generation**: Automatic JSON schema creation from struct fields
- **Parameter Attributes**: Using `#[param]` attributes for parameter descriptions
- **Optional Parameters**: Handling optional fields with `Option<T>`
- **Error Handling**: Proper error handling in tool implementations
- **Complex Data Types**: Working with various parameter types (strings, numbers, booleans)
- **MCP 2025-06-18 Compliance**: Full specification compliance with proper `_meta` field handling

## 🛠️ Available Tools

### 1. Text Transform (`text_transform`)
Transform text in various ways:
- **uppercase**: Convert to uppercase
- **lowercase**: Convert to lowercase  
- **reverse**: Reverse the text
- **wordcount**: Count words in the text

**Parameters:**
- `text` (string): Input text to transform
- `transform` (string): Transformation type

### 2. Math Operations (`math_operations`)
Perform mathematical operations:
- **add**: Addition
- **subtract**: Subtraction
- **multiply**: Multiplication
- **divide**: Division (with zero-check)
- **power**: Exponentiation

**Parameters:**
- `a` (number): First number
- `b` (number): Second number
- `operation` (string): Operation type

### 3. Data Validation (`validate_data`)
Validate different types of data:
- **email**: Basic email validation
- **url**: URL format validation
- **phone**: Phone number validation
- **json**: JSON format validation
- **uuid**: UUID format validation

**Parameters:**
- `data` (string): Data to validate
- `validation_type` (string): Type of validation
- `format_output` (boolean, optional): Whether to format output as JSON

### 4. Counter (`counter`)
Simple counter tool (demonstrates limitations with session state in derive macros):

**Parameters:**
- `increment` (number): Amount to increment

### 5. Geometry (`geometry`)
Calculate geometric properties for different shapes:
- **circle**: Area and circumference
- **rectangle**: Area and perimeter
- **triangle**: Area calculation

**Parameters:**
- `shape` (string): Shape type
- `dimension1` (number): Primary dimension
- `dimension2` (number, optional): Secondary dimension
- `calculate_area` (boolean, optional): Whether to calculate area
- `calculate_perimeter` (boolean, optional): Whether to calculate perimeter

## 🏃 Running the Example

```bash
cargo run -p derive-macro-server
```

The server starts on `http://127.0.0.1:8765/mcp` by default.

You can specify a custom bind address:
```bash
cargo run -p derive-macro-server -- 0.0.0.0:9000
```

## 🧪 Testing the Tools

### Text Transform Example
```bash
curl -X POST http://127.0.0.1:8765/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "text_transform",
      "arguments": {
        "text": "Hello World",
        "transform": "uppercase"
      }
    },
    "id": "1"
  }'
```

### Math Operations Example
```bash
curl -X POST http://127.0.0.1:8765/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "math_operations",
      "arguments": {
        "a": 15.5,
        "b": 4.2,
        "operation": "multiply"
      }
    },
    "id": "2"
  }'
```

### Data Validation Example
```bash
curl -X POST http://127.0.0.1:8765/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "validate_data",
      "arguments": {
        "data": "user@example.com",
        "validation_type": "email",
        "format_output": true
      }
    },
    "id": "3"
  }'
```

### Geometry Example
```bash
curl -X POST http://127.0.0.1:8765/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "geometry",
      "arguments": {
        "shape": "rectangle",
        "dimension1": 10.0,
        "dimension2": 5.0,
        "calculate_area": true,
        "calculate_perimeter": true
      }
    },
    "id": "4"
  }'
```

## 🔧 Key Features Demonstrated

### 1. Derive Macro Syntax
```rust
#[derive(McpTool, Clone)]
#[tool(name = "tool_name", description = "Tool description")]
struct MyTool {
    #[param(description = "Parameter description")]
    param1: String,
    
    #[param(description = "Optional parameter", optional)]
    param2: Option<i32>,
}
```

### 2. Automatic Schema Generation
The macro automatically generates JSON schema based on field types:
- `String` → JSON string schema
- `f64`/`i32` → JSON number/integer schema
- `bool` → JSON boolean schema
- `Option<T>` → Optional parameter with type T

### 3. Parameter Validation
The macro handles:
- Required vs optional parameters
- Type validation and conversion
- Descriptive error messages
- Parameter documentation

### 4. Implementation Pattern
```rust
impl MyTool {
    async fn execute(&self) -> McpResult<String> {
        // Tool logic here
        Ok("result".to_string())
    }
}
```

## ⚡ Advantages of Derive Macros

1. **Less Boilerplate**: Automatic trait implementation
2. **Type Safety**: Compile-time type checking
3. **Auto Schema**: JSON schema generated from types
4. **Documentation**: Parameter descriptions in the code
5. **Maintainability**: Changes to struct automatically update schema

## 🚨 Current Limitations

1. **Session Context**: Derive macros don't currently support session context (see counter tool)
2. **Complex Schemas**: Limited support for complex nested schemas
3. **Custom Validation**: No built-in support for custom parameter validation
4. **Advanced Features**: Some advanced MCP features require manual implementation

## 🔄 Comparison with Manual Implementation

| Feature | Derive Macro | Manual Implementation |
|---------|--------------|----------------------|
| Code Length | Short | Longer |
| Type Safety | High | High |
| Schema Generation | Automatic | Manual |
| Session Support | Limited | Full |
| Customization | Limited | Full |
| Learning Curve | Easy | Moderate |

## 📚 Next Steps

After understanding derive macros, explore:

1. **[function-macro-server](../function-macro-server/)**: Function-style tool definitions using `#[mcp_tool]`
2. **[enhanced-tool-macro-test](../enhanced-tool-macro-test/)**: Enhanced `tool!` macro with constraints and defaults
3. **[stateful-server](../stateful-server/)**: Session management and state
4. **[comprehensive-server](../comprehensive-server/)**: All MCP features including real-time notifications

## 🛠️ Extending This Example

### Adding New Tools
```rust
#[derive(McpTool, Clone)]
#[tool(name = "new_tool", description = "My new tool")]
struct NewTool {
    #[param(description = "Input parameter")]
    input: String,
}

impl NewTool {
    async fn execute(&self) -> McpResult<String> {
        // Your logic here
        Ok(format!("Processed: {}", self.input))
    }
}

// Add to server builder
.tool(NewTool { input: String::new() })
```

### Custom Parameter Types
```rust
#[derive(McpTool, Clone)]
#[tool(name = "complex_tool", description = "Tool with complex types")]
struct ComplexTool {
    #[param(description = "List of values")]
    values: Vec<f64>,
    
    #[param(description = "Configuration object")]
    config: serde_json::Value,
}
```

### Error Handling Patterns
```rust
impl MyTool {
    async fn execute(&self) -> McpResult<String> {
        // Validate input
        if self.input.is_empty() {
            return Err("Input cannot be empty".to_string());
        }
        
        // Perform operation
        match self.operation.as_str() {
            "valid_op" => Ok("Success".to_string()),
            _ => Err(format!("Unknown operation: {}", self.operation)),
        }
    }
}
```

## 🤝 Best Practices

1. **Clear Descriptions**: Write descriptive tool and parameter descriptions
2. **Input Validation**: Always validate inputs in the execute method
3. **Error Messages**: Provide helpful error messages
4. **Type Selection**: Choose appropriate types for parameters
5. **Optional Parameters**: Use `Option<T>` for optional parameters
6. **Documentation**: Document complex tools and their behavior

---

This example demonstrates the power and simplicity of the derive macro approach while highlighting when you might need more advanced manual implementations.