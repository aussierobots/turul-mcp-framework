# Enhanced Tool Macro Test Example

This example demonstrates advanced patterns using the `#[derive(McpTool)]` macro with enhanced parameter validation and optional parameters. Part of the **MCP Framework - Rust Implementation** with full MCP 2025-06-18 specification compliance.

## 🚀 What This Example Shows

- **Advanced Derive Macros**: Using `#[derive(McpTool)]` for complex tools
- **Parameter Validation**: Manual validation with descriptive error messages
- **Optional Parameters**: Using `Option<T>` types with default values
- **Complex Logic**: Advanced mathematical functions and validation
- **Error Handling**: Comprehensive error handling patterns
- **Type Safety**: Compile-time parameter type validation

## 🛠️ Available Tools

### 1. Enhanced Calculator (`enhanced_calculator`)
Advanced calculator with input validation and configurable precision:

**Parameters:**
- `a` (number): First number (0-1000)
- `b` (number): Second number (0-1000)  
- `operation` (string): Math operation (add, subtract, multiply, divide)
- `precision` (number, optional): Decimal precision for result (default: 2)

**Features:**
- Input range validation (0-1000)
- Division by zero protection
- Configurable decimal precision
- Descriptive error messages

### 2. Math Functions (`math_functions`)
Advanced mathematical functions with optional parameters:

**Parameters:**
- `function` (string): Function (sin, cos, tan, log, sqrt)
- `value` (number): Input value
- `degrees` (boolean, optional): Use degrees for trig functions (default: false)
- `precision` (number, optional): Decimal precision (default: 4)

**Features:**
- Trigonometric functions (sin, cos, tan)
- Natural logarithm with positive value validation
- Square root with non-negative validation
- Degree/radian conversion for trig functions
- Configurable precision

## 🏃 Running the Example

```bash
cargo run -p enhanced-tool-macro-test
```

The server starts on `http://127.0.0.1:8010/mcp` by default.

## 🧪 Testing the Tools

### Enhanced Calculator Example
```bash
curl -X POST http://127.0.0.1:8010/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "enhanced_calculator",
      "arguments": {
        "a": 25.5,
        "b": 4.2,
        "operation": "multiply",
        "precision": 3
      }
    },
    "id": "1"
  }'
```

### Math Functions Example - Trigonometry
```bash
curl -X POST http://127.0.0.1:8010/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "math_functions",
      "arguments": {
        "function": "sin",
        "value": 45,
        "degrees": true,
        "precision": 6
      }
    },
    "id": "2"
  }'
```

### Math Functions Example - Logarithm
```bash
curl -X POST http://127.0.0.1:8010/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "math_functions",
      "arguments": {
        "function": "log",
        "value": 2.718281828
      }
    },
    "id": "3"
  }'
```

### Error Handling Test - Invalid Range
```bash
curl -X POST http://127.0.0.1:8010/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "enhanced_calculator",
      "arguments": {
        "a": 1500,
        "b": 10,
        "operation": "add"
      }
    },
    "id": "4"
  }'
```

## 🔧 Key Features Demonstrated

### 1. Enhanced Derive Macro Pattern
```rust
#[derive(McpTool, Clone)]
#[tool(name = "enhanced_calculator", description = "Enhanced calculator with parameter validation")]
struct EnhancedCalculatorTool {
    #[param(description = "First number (0-1000)")]
    a: f64,
    #[param(description = "Second number (0-1000)")]
    b: f64,
    #[param(description = "Math operation (add, subtract, multiply, divide)")]
    operation: String,
    #[param(description = "Decimal precision for result")]
    precision: Option<i32>,
}
```

### 2. Manual Parameter Validation
```rust
impl EnhancedCalculatorTool {
    async fn execute(&self) -> McpResult<String> {
        // Validate input ranges
        if self.a < 0.0 || self.a > 1000.0 {
            return Err("First number must be between 0 and 1000".to_string());
        }
        if self.b < 0.0 || self.b > 1000.0 {
            return Err("Second number must be between 0 and 1000".to_string());
        }
        
        // Business logic with error handling
        let result = match self.operation.as_str() {
            "divide" => {
                if self.b == 0.0 {
                    return Err("Division by zero".to_string());
                }
                self.a / self.b
            },
            // ... other operations
        };
        
        let precision = self.precision.unwrap_or(2) as usize;
        Ok(format!("{:.prec$}", result, prec = precision))
    }
}
```

### 3. Optional Parameters with Defaults
```rust
// In the implementation
let degrees = self.degrees.unwrap_or(false);
let precision = self.precision.unwrap_or(4) as usize;

// Usage in calculations
let val = if degrees { self.value.to_radians() } else { self.value };
```

### 4. Comprehensive Error Handling
```rust
match self.function.as_str() {
    "log" => {
        if self.value <= 0.0 {
            return Err("Logarithm requires positive value".to_string());
        }
        self.value.ln()
    },
    "sqrt" => {
        if self.value < 0.0 {
            return Err("Square root requires non-negative value".to_string());
        }
        self.value.sqrt()
    },
    _ => return Err(format!("Unknown function: {}", self.function)),
}
```

## ⚡ Advantages of This Pattern

1. **Type Safety**: Compile-time parameter validation
2. **Clear Structure**: Well-defined struct with documented parameters
3. **Flexible Validation**: Custom validation logic in implementation
4. **Optional Parameters**: Natural `Option<T>` handling with defaults
5. **Comprehensive Errors**: Detailed error messages for different failure modes
6. **Maintainable**: Easy to extend with new parameters or validation

## 🚨 Future Enhancements

This example demonstrates current capabilities using derive macros. Future framework versions may include:

- **Declarative `tool!` Macro**: Simplified syntax with inline parameter constraints
- **Built-in Validation**: Automatic range validation without manual checks
- **Constraint Attributes**: `#[param(min = 0.0, max = 1000.0)]` syntax
- **Default Values**: `#[param(default = 2)]` attribute support

## 📚 Related Examples

### Current Working Approaches
- **[derive-macro-server](../derive-macro-server/)**: Basic derive macro patterns
- **[function-macro-server](../function-macro-server/)**: Function-style tool definitions
- **[minimal-server](../minimal-server/)**: Simplest possible server

### Advanced Features
- **[stateful-server](../stateful-server/)**: Session management and state persistence
- **[comprehensive-server](../comprehensive-server/)**: All MCP features including real-time notifications
- **[notification-server](../notification-server/)**: Real-time updates and progress tracking

## 🛠️ Extending This Example

### Adding Parameter Validation
```rust
#[derive(McpTool, Clone)]
#[tool(name = "validated_tool", description = "Tool with complex validation")]
struct ValidatedTool {
    #[param(description = "Email address")]
    email: String,
    #[param(description = "Age (18-120)")]
    age: i32,
    #[param(description = "Score (0.0-100.0)")]
    score: f64,
}

impl ValidatedTool {
    async fn execute(&self) -> McpResult<String> {
        // Email validation
        if !self.email.contains('@') {
            return Err("Invalid email format".to_string());
        }
        
        // Age validation
        if self.age < 18 || self.age > 120 {
            return Err("Age must be between 18 and 120".to_string());
        }
        
        // Score validation
        if self.score < 0.0 || self.score > 100.0 {
            return Err("Score must be between 0.0 and 100.0".to_string());
        }
        
        Ok(format!("Valid: email={}, age={}, score={}", 
                   self.email, self.age, self.score))
    }
}
```

### Adding Complex Optional Parameters
```rust
#[derive(McpTool, Clone)]
#[tool(name = "advanced_tool", description = "Tool with complex optional parameters")]
struct AdvancedTool {
    #[param(description = "Input data")]
    data: String,
    #[param(description = "Processing options")]
    options: Option<serde_json::Value>,
    #[param(description = "Output format")]
    format: Option<String>,
    #[param(description = "Verbose output")]
    verbose: Option<bool>,
}

impl AdvancedTool {
    async fn execute(&self) -> McpResult<String> {
        let format = self.format.as_deref().unwrap_or("json");
        let verbose = self.verbose.unwrap_or(false);
        
        // Use options if provided
        if let Some(options) = &self.options {
            // Process with options
        }
        
        // Format output based on format parameter
        match format {
            "json" => Ok(serde_json::json!({
                "data": self.data,
                "verbose": verbose
            }).to_string()),
            "text" => Ok(format!("Data: {}", self.data)),
            _ => Err(format!("Unknown format: {}", format)),
        }
    }
}
```

## 🤝 Best Practices

1. **Validation Early**: Validate all parameters at the start of execute()
2. **Clear Error Messages**: Provide specific, actionable error messages
3. **Sensible Defaults**: Choose reasonable defaults for optional parameters
4. **Type Appropriateness**: Use appropriate types (f64 for math, i32 for counts)
5. **Documentation**: Write clear parameter descriptions
6. **Consistent Naming**: Use consistent parameter naming conventions
7. **Error Handling**: Handle all possible error conditions gracefully

## 🔄 Migration Path

When enhanced macro features become available:

1. **Keep Current Structure**: Struct-based tools will remain supported
2. **Add Constraints**: Migrate to attribute-based constraints
3. **Simplify Validation**: Remove manual validation where possible
4. **Enhanced Defaults**: Use attribute-based default values

---

This example demonstrates advanced derive macro patterns and serves as a foundation for building sophisticated MCP tools with comprehensive validation and error handling.