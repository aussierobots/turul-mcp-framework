# Calculator Server Example

A foundational example demonstrating basic MCP server implementation with mathematical operations. This server showcases **manual trait implementation** patterns and provides a solid foundation for understanding MCP tool development.

## Overview

This example implements a simple calculator as an MCP server using manual `McpTool` trait implementations. It demonstrates core MCP concepts including tool registration, parameter validation, session management, and progress notifications.

## Features

### 🧮 **Mathematical Operations**
- **Addition** - Add two numbers with progress notifications
- **Subtraction** - Subtract two numbers  
- **Multiplication** - Multiply two numbers
- **Division** - Divide numbers with zero-division protection

### 🔧 **MCP Fundamentals**
- **Manual Tool Implementation** - Direct `McpTool` trait implementation
- **Parameter Validation** - JSON schema definition and runtime validation
- **Session Management** - Optional session context for progress tracking
- **Progress Notifications** - Real-time operation progress updates
- **Error Handling** - Comprehensive input validation and error reporting

## Quick Start

### 1. Start the Server

```bash
cargo run --bin calculator-server
```

The server will start on `http://127.0.0.1:8764/mcp`

### 2. Test the Calculator

#### Addition with Progress Tracking
```json
{
  "name": "add",
  "arguments": {
    "a": 15.5,
    "b": 24.3
  }
}
```

#### Subtraction
```json
{
  "name": "subtract", 
  "arguments": {
    "a": 100,
    "b": 25
  }
}
```

#### Multiplication
```json
{
  "name": "multiply",
  "arguments": {
    "a": 7,
    "b": 8
  }
}
```

#### Division (with validation)
```json
{
  "name": "divide",
  "arguments": {
    "a": 84,
    "b": 12
  }
}
```

## Tool Reference

### ➕ `add`

Adds two numbers together with progress notifications.

**Parameters:**
- `a` (required): First number
- `b` (required): Second number

**Features:**
- Progress notifications (25%, 75%, 100%)
- Session logging for calculation steps
- Simulated processing delay

**Response:**
```
"15.5 + 24.3 = 39.8"
```

### ➖ `subtract`

Subtracts the second number from the first.

**Parameters:**
- `a` (required): Minuend (number to subtract from)
- `b` (required): Subtrahend (number to subtract)

**Response:**
```
"100 - 25 = 75"
```

### ✖️ `multiply`

Multiplies two numbers together.

**Parameters:**
- `a` (required): First number
- `b` (required): Second number

**Response:**
```
"7 × 8 = 56"
```

### ➗ `divide`

Divides the first number by the second with zero-division protection.

**Parameters:**
- `a` (required): Dividend (number to be divided)
- `b` (required): Divisor (number to divide by)

**Features:**
- Zero-division error handling
- Precise decimal calculations

**Response:**
```
"84 ÷ 12 = 7"
```

**Error Handling:**
```json
{
  "error": "Division by zero is not allowed"
}
```

## Implementation Patterns

### Manual Tool Implementation

```rust
#[derive(Clone)]
struct AddTool;

#[async_trait]
impl McpTool for AddTool {
    fn name(&self) -> &str {
        "add"
    }

    fn description(&self) -> &str {
        "Add two numbers together"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("a".to_string(), JsonSchema::number_with_description("First number")),
                ("b".to_string(), JsonSchema::number_with_description("Second number")),
            ]))
            .with_required(vec!["a".to_string(), "b".to_string()])
    }

    async fn call(&self, args: Value, session: Option<SessionContext>) -> Result<Vec<ToolResult>, String> {
        // Implementation...
    }
}
```

### Parameter Validation

```rust
let a = args
    .get("a")
    .and_then(|v| v.as_f64())
    .ok_or("Missing or invalid parameter 'a'")?;

let b = args
    .get("b")
    .and_then(|v| v.as_f64())
    .ok_or("Missing or invalid parameter 'b'")?;
```

### Progress Notifications

```rust
if let Some(ref session) = session {
    session.notify_progress("add-operation", 25);
    session.notify_log("info", "Starting addition calculation");
    
    // Simulate work
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    session.notify_progress("add-operation", 75);
    
    // Complete operation
    session.notify_progress_with_total("add-operation", 100, 100);
    session.notify_log("info", format!("Addition completed: {}", result));
}
```

### Error Handling

```rust
// Division by zero protection
if b == 0.0 {
    return Err("Division by zero is not allowed".to_string());
}

// Parameter validation
let a = args
    .get("a")
    .and_then(|v| v.as_f64())
    .ok_or("Missing or invalid parameter 'a'")?;
```

## Server Configuration

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting Calculator Server");

    let server = McpServer::builder()
        .name("calculator-server")
        .version("1.0.0")
        .title("Basic Calculator MCP Server")
        .instructions("Provides basic mathematical operations through MCP tools")
        .tool(AddTool)
        .tool(SubtractTool)
        .tool(MultiplyTool)
        .tool(DivideTool)
        .bind_address("127.0.0.1:8764".parse()?)
        .build()?;

    info!("Calculator server running at: http://127.0.0.1:8764/mcp");
    server.run().await?;
    Ok(())
}
```

## Session Management

The calculator demonstrates optional session management:

- **With Session**: Progress notifications and logging are enabled
- **Without Session**: Basic calculation functionality only

```rust
// Session-aware operation
if let Some(ref session) = session {
    session.notify_progress("operation", 50);
    session.notify_log("info", "Processing calculation");
}

// Core calculation (works with or without session)
let result = a + b;
```

## Error Scenarios

### Invalid Parameters
```json
{
  "name": "add",
  "arguments": {
    "a": "not-a-number",
    "b": 5
  }
}
```
**Response:** `"Missing or invalid parameter 'a'"`

### Missing Parameters  
```json
{
  "name": "multiply",
  "arguments": {
    "a": 7
  }
}
```
**Response:** `"Missing or invalid parameter 'b'"`

### Division by Zero
```json
{
  "name": "divide",
  "arguments": {
    "a": 10,
    "b": 0
  }
}
```
**Response:** `"Division by zero is not allowed"`

## Use Cases

### 1. **Learning MCP Basics**
Perfect introduction to MCP server development, tool implementation, and parameter handling.

### 2. **Manual Implementation Reference**
Demonstrates how to implement MCP tools without using derive macros or declarative macros.

### 3. **Session Management Examples**
Shows how to integrate optional session context for enhanced functionality.

### 4. **Validation Patterns**
Provides examples of parameter validation and error handling patterns.

### 5. **Progress Notification Demo**
Illustrates real-time progress updates for long-running operations.

## Testing

```bash
# Start the server
cargo run --bin calculator-server &

# Test with curl
curl -X POST http://127.0.0.1:8764/mcp \
  -H "Content-Type: application/json" \
  -d '{"method": "tools/call", "params": {"name": "add", "arguments": {"a": 10, "b": 5}}}'

# Test division by zero
curl -X POST http://127.0.0.1:8764/mcp \
  -H "Content-Type: application/json" \
  -d '{"method": "tools/call", "params": {"name": "divide", "arguments": {"a": 10, "b": 0}}}'
```

## Architecture

```
┌─────────────────────┐    ┌──────────────────────┐    ┌─────────────────────┐
│   MCP Client        │────│  Calculator Server   │────│  Session Manager    │
│                     │    │                      │    │                     │
│ - Tool Calls        │    │ - AddTool           │    │ - Progress Tracking │
│ - Parameter Input   │    │ - SubtractTool       │    │ - Log Management    │
│ - Progress Updates  │    │ - MultiplyTool       │    │ - Notification Hub  │
│ - Error Handling    │    │ - DivideTool         │    │                     │
└─────────────────────┘    └──────────────────────┘    └─────────────────────┘
```

## Key Learning Points

1. **Manual Tool Implementation**: Understanding the `McpTool` trait and its methods
2. **Schema Definition**: Creating JSON schemas for tool parameters
3. **Parameter Extraction**: Safely extracting and validating parameters from JSON
4. **Error Handling**: Proper error messages and validation
5. **Session Integration**: Optional session context for enhanced functionality
6. **Server Builder Pattern**: Using the builder pattern for server configuration

This example serves as the foundation for understanding MCP server development and provides a stepping stone to more advanced examples using derive macros and declarative macros.