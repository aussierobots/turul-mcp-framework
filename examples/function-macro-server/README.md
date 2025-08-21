# Function Macro Server Example

This example demonstrates the `#[mcp_tool]` function attribute macro functionality (currently implemented using manual trait implementations as the macro is in development). Part of the **MCP Framework - Rust Implementation** with enhanced tool! macro supporting parameter constraints and defaults.

## 🚀 What This Example Shows

- **Function-Style Tools**: Each tool is conceptually a function with typed parameters
- **Parameter Types**: Different parameter types (numbers, strings, booleans)
- **Parameter Validation**: Input validation and error handling
- **Enum Parameters**: String enums for constrained choices
- **Future Macro Syntax**: Preview of upcoming function macro capabilities

## 🛠️ Available Tools

### 1. Add (`add`)
Add two numbers together:

**Parameters:**
- `a` (number): First number
- `b` (number): Second number

**Example:**
```json
{
  "a": 15.5,
  "b": 27.3
}
```

### 2. String Repeat (`string_repeat`)
Repeat a string multiple times:

**Parameters:**
- `text` (string): Text to repeat
- `count` (integer): Number of repetitions (0-1000)

**Example:**
```json
{
  "text": "Hello! ",
  "count": 3
}
```

### 3. Boolean Logic (`boolean_logic`)
Perform boolean operations:

**Parameters:**
- `a` (boolean): First boolean value
- `b` (boolean): Second boolean value
- `operation` (enum): Operation type ("and", "or", "xor")

**Example:**
```json
{
  "a": true,
  "b": false,
  "operation": "and"
}
```

## 🏃 Running the Example

```bash
cargo run -p function-macro-server
```

The server starts on `http://127.0.0.1:8003/mcp`.

## 🧪 Testing the Tools

### Add Numbers
```bash
curl -X POST http://127.0.0.1:8003/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "add",
      "arguments": {
        "a": 15.5,
        "b": 27.3
      }
    },
    "id": "1"
  }'
```

### String Repeat
```bash
curl -X POST http://127.0.0.1:8003/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "string_repeat",
      "arguments": {
        "text": "Echo! ",
        "count": 5
      }
    },
    "id": "2"
  }'
```

### Boolean Logic
```bash
curl -X POST http://127.0.0.1:8003/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "boolean_logic",
      "arguments": {
        "a": true,
        "b": false,
        "operation": "xor"
      }
    },
    "id": "3"
  }'
```

## 🔮 Future Function Macro Syntax

This example previews the upcoming `#[mcp_tool]` function macro syntax:

### Current Implementation (Manual)
```rust
struct AddTool;

#[async_trait]
impl McpTool for AddTool {
    fn name(&self) -> &str { "add" }
    fn description(&self) -> &str { "Add two numbers together" }
    
    fn input_schema(&self) -> ToolSchema {
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("a".to_string(), JsonSchema::number().with_description("First number")),
                ("b".to_string(), JsonSchema::number().with_description("Second number")),
            ]))
            .with_required(vec!["a".to_string(), "b".to_string()])
    }
    
    async fn call(&self, args: Value, _session: Option<SessionContext>) -> Result<Vec<ToolResult>, String> {
        let a = args.get("a").and_then(|v| v.as_f64()).ok_or("Missing parameter 'a'")?;
        let b = args.get("b").and_then(|v| v.as_f64()).ok_or("Missing parameter 'b'")?;
        
        let result = format!("{} + {} = {}", a, b, a + b);
        Ok(vec![ToolResult::text(result)])
    }
}
```

### Future Function Macro (Target)
```rust
#[mcp_tool(name = "add", description = "Add two numbers together")]
async fn add(
    #[param(description = "First number")] a: f64,
    #[param(description = "Second number")] b: f64,
) -> McpResult<String> {
    Ok(format!("{} + {} = {}", a, b, a + b))
}
```

### Advanced Function Macro Features (Planned)
```rust
#[mcp_tool(name = "string_repeat", description = "Repeat a string multiple times")]
async fn string_repeat(
    #[param(description = "Text to repeat")] text: String,
    #[param(description = "Number of repetitions", min = 0, max = 1000)] count: i32,
) -> McpResult<String> {
    Ok(text.repeat(count as usize))
}

#[mcp_tool(name = "boolean_logic", description = "Perform boolean operations")]
async fn boolean_logic(
    #[param(description = "First boolean value")] a: bool,
    #[param(description = "Second boolean value")] b: bool,
    #[param(description = "Operation", choices = ["and", "or", "xor"])] operation: String,
) -> McpResult<String> {
    let result = match operation.as_str() {
        "and" => a && b,
        "or" => a || b,
        "xor" => a ^ b,
        _ => return Err(format!("Unknown operation: {}", operation)),
    };
    Ok(format!("{} {} {} = {}", a, operation, b, result))
}
```

## 🎯 Advantages of Function Macros

1. **Natural Syntax**: Functions feel more natural than structs for simple operations
2. **Type Safety**: Parameters are typed at the function signature level
3. **Less Boilerplate**: No need to define structs for simple tools
4. **Validation Attributes**: Parameter constraints defined in attributes
5. **Direct Implementation**: Function body contains the tool logic directly

## 🔧 Key Features Demonstrated

### 1. Parameter Types
- **Numbers**: `f64` for floating-point, `i64` for integers
- **Strings**: `String` for text input
- **Booleans**: `bool` for true/false values
- **Enums**: String enums for constrained choices

### 2. Parameter Validation
```rust
// Range validation
if count < 0 {
    return Err("Count must be non-negative".to_string());
}
if count > 1000 {
    return Err("Count too large (max 1000)".to_string());
}

// Enum validation
match operation {
    "and" | "or" | "xor" => { /* valid */ },
    _ => return Err(format!("Unknown operation: {}", operation)),
}
```

### 3. Schema Definition
Each tool defines its JSON schema with:
- Parameter names and types
- Parameter descriptions
- Required vs optional parameters
- Constraints and validation rules

## 🚨 Current Status

**Status**: Development Phase - Macro Implementation In Progress
- ✅ Manual implementation demonstrates target pattern  
- ✅ Enhanced `tool!` declarative macro with constraints available
- ⚠️ `#[mcp_tool]` function attribute macro in development
- 🔮 Advanced parameter validation coming soon

**What Works Now:**
- Manual trait implementations (this example)
- Derive macros for struct-based tools (`#[derive(McpTool)]`)
- Enhanced declarative `tool!` macro with parameter constraints
- Full MCP 2025-06-18 protocol support with real-time notifications

**What's Coming:**
- `#[mcp_tool]` function attribute macro completion
- Advanced parameter attribute support (`#[param]`)
- Automatic schema generation from function signatures
- Built-in validation constraints and type checking

## 📚 Related Examples

### Current Working Approaches
- **[derive-macro-server](../derive-macro-server/)**: Struct-based tools with derive macros  
- **[enhanced-tool-macro-test](../enhanced-tool-macro-test/)**: Enhanced `tool!` macro with constraints and defaults
- **[minimal-server](../minimal-server/)**: Simplest possible server

### Advanced Features
- **[stateful-server](../stateful-server/)**: Session management and state persistence
- **[comprehensive-server](../comprehensive-server/)**: All MCP features including real-time notifications
- **[notification-server](../notification-server/)**: Real-time updates and progress tracking

## 🛠️ Extending This Example

### Adding New Tools
```rust
// Current manual approach
struct NewTool;

#[async_trait]
impl McpTool for NewTool {
    fn name(&self) -> &str { "new_tool" }
    fn description(&self) -> &str { "Description here" }
    
    fn input_schema(&self) -> ToolSchema {
        // Define schema manually
    }
    
    async fn call(&self, args: Value, _session: Option<SessionContext>) -> Result<Vec<ToolResult>, String> {
        // Implementation here
    }
}

// Future macro approach (when available)
#[mcp_tool(name = "new_tool", description = "Description here")]
async fn new_tool(
    #[param(description = "Parameter description")] param: String,
) -> McpResult<String> {
    // Implementation here
    Ok("result".to_string())
}
```

### Complex Parameter Types
```rust
// Future support for complex types
#[mcp_tool(name = "complex_tool", description = "Tool with complex parameters")]
async fn complex_tool(
    #[param(description = "List of numbers")] numbers: Vec<f64>,
    #[param(description = "Configuration object")] config: serde_json::Value,
    #[param(description = "Optional parameter")] optional: Option<String>,
) -> McpResult<String> {
    // Implementation
}
```

## 🤔 When to Use Function Macros

**Best For:**
- Simple, stateless operations
- Mathematical functions
- Text processing tools
- Data validation utilities
- Quick prototyping

**Consider Alternatives For:**
- Complex stateful operations → Use struct-based tools
- Session management → Manual implementation
- Advanced MCP features → Comprehensive patterns
- Complex parameter relationships → Manual validation

## 🔄 Migration Path

When function macros become available:

1. **Keep Current Code**: Manual implementations will continue to work
2. **Gradual Migration**: Convert simple tools to function macros
3. **Mixed Approach**: Use both patterns in the same server
4. **Choose the Right Tool**: Function macros for simple operations, structs for complex ones

---

This example provides a preview of the streamlined function-based tool development coming to the MCP framework.