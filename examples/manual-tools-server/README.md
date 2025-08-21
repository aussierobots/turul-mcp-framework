# Manual Tools Server Example

This example demonstrates **advanced manual implementation** of MCP tools without using derive macros or declarative macros. It showcases complex schemas, session state management, progress notifications, and sophisticated tool implementations.

## Overview

This server provides a comprehensive demonstration of implementing MCP tools using the low-level `McpTool` trait directly. It's perfect for understanding the underlying mechanics of MCP tools and for implementing complex functionality that requires fine-grained control.

## Features

### 🛠️ **Advanced Tool Implementations**

1. **File System Operations** - Complex schema with operation enums and conditional parameters
2. **Mathematical Calculator** - Multi-operation tool with validation and error handling
3. **Session Counter** - Demonstrates session state management and persistence
4. **Progress Task** - Long-running task with real-time progress notifications

### 🔧 **Technical Demonstrations**

- **Manual Schema Definition**: Complex JSON schemas with enums, constraints, and validations
- **Session State Management**: Persistent state across tool calls within a session
- **Progress Notifications**: Real-time updates for long-running operations
- **Error Handling**: Comprehensive error management and user feedback
- **Parameter Validation**: Runtime validation and type checking

## Quick Start

### 1. Start the Server

```bash
cargo run --bin manual-tools-server
```

The server will start on `http://127.0.0.1:8040/mcp`

### 2. Test the Tools

#### File System Operations
```json
{
  "name": "file_operations",
  "arguments": {
    "operation": "create",
    "path": "/example/file.txt",
    "content": "Hello, World!"
  }
}
```

#### Calculator Operations
```json
{
  "name": "calculator",
  "arguments": {
    "operation": "add",
    "a": 15,
    "b": 25
  }
}
```

#### Session Counter
```json
{
  "name": "session_counter",
  "arguments": {
    "increment": 5
  }
}
```

#### Progress Task
```json
{
  "name": "progress_task",
  "arguments": {
    "steps": 10,
    "delay_ms": 500
  }
}
```

## Tool Reference

### 📁 `file_operations`

Simulates file system operations with session-based file tracking.

**Parameters:**
- `operation` (required): Operation type
  - `"create"` - Create a new file
  - `"read"` - Read file contents
  - `"update"` - Update file contents
  - `"delete"` - Delete a file
  - `"list"` - List all files in session
- `path` (required): File path
- `content` (optional): File content for create/update operations

**Features:**
- Session-based file storage simulation
- Comprehensive error handling for invalid operations
- Proper validation of file paths and content

### 🧮 `calculator`

Mathematical operations with comprehensive validation.

**Parameters:**
- `operation` (required): Math operation
  - `"add"` - Addition
  - `"subtract"` - Subtraction
  - `"multiply"` - Multiplication
  - `"divide"` - Division (with zero-division protection)
- `a` (required): First number
- `b` (required): Second number

**Features:**
- Type validation for numeric inputs
- Division by zero protection
- Detailed operation history
- Error handling for invalid operations

### 🔢 `session_counter`

Session-persistent counter with increment operations.

**Parameters:**
- `increment` (optional): Amount to increment (default: 1)

**Features:**
- Session state persistence
- Counter history tracking
- Automatic session initialization
- Increment validation and limits

### ⏳ `progress_task`

Long-running task with real-time progress notifications.

**Parameters:**
- `steps` (required): Number of processing steps (1-100)
- `delay_ms` (optional): Delay between steps in milliseconds (default: 1000)

**Features:**
- Real-time progress notifications via SSE
- Progress tracking with percentage completion
- Configurable processing speed
- Comprehensive status reporting

## Manual Implementation Patterns

### Schema Definition

```rust
fn input_schema(&self) -> ToolSchema {
    ToolSchema::object()
        .with_properties(HashMap::from([
            ("operation".to_string(), JsonSchema::string_enum(vec![
                "create".to_string(), "read".to_string(), "update".to_string(), 
                "delete".to_string(), "list".to_string()
            ]).with_description("File operation to perform")),
            ("path".to_string(), JsonSchema::string()
                .with_description("File path")),
            ("content".to_string(), JsonSchema::string()
                .with_description("File content (for create/update operations)")),
        ]))
        .with_required(vec!["operation".to_string(), "path".to_string()])
}
```

### Session State Management

```rust
async fn call(&self, args: Value, session: Option<SessionContext>) -> Result<Vec<ToolResult>, String> {
    if let Some(session) = session {
        // Get current counter value from session state
        let current: i64 = (session.get_state)("counter")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        
        // Update session state
        let new_value = current + increment;
        (session.set_state)("counter", json!(new_value));
        
        Ok(vec![ToolResult::text(format!("Counter: {} → {}", current, new_value))])
    } else {
        Err("Session required for counter operations".to_string())
    }
}
```

### Progress Notifications

```rust
async fn call(&self, args: Value, session: Option<SessionContext>) -> Result<Vec<ToolResult>, String> {
    if let Some(session) = session {
        let nm = session.notification_manager;
        
        for step in 1..=steps {
            let progress = (step as f64 / steps as f64 * 100.0) as u8;
            
            // Send progress notification
            nm.send_progress_notification(&session.session_id, 
                "progress_task", 
                progress,
                &format!("Step {} of {}", step, steps)
            ).await;
            
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        }
    }
    
    Ok(vec![ToolResult::text("Task completed successfully")])
}
```

### Parameter Validation

```rust
async fn call(&self, args: Value, session: Option<SessionContext>) -> Result<Vec<ToolResult>, String> {
    // Required parameter extraction with validation
    let operation = args.get("operation")
        .and_then(|v| v.as_str())
        .ok_or("Missing or invalid operation parameter")?;
    
    // Numeric parameter validation
    let a = args.get("a")
        .and_then(|v| v.as_f64())
        .ok_or("Missing or invalid parameter 'a'")?;
    
    let b = args.get("b")
        .and_then(|v| v.as_f64())
        .ok_or("Missing or invalid parameter 'b'")?;
    
    // Operation validation
    match operation {
        "divide" if b == 0.0 => Err("Division by zero is not allowed".to_string()),
        "add" | "subtract" | "multiply" | "divide" => {
            // Process valid operation
            Ok(result)
        },
        _ => Err(format!("Unknown operation: {}", operation))
    }
}
```

## Implementation Benefits

### 🎯 **Complete Control**

Manual implementation provides full control over:
- Schema definition and validation
- Parameter processing and error handling
- Session state management
- Progress notification timing
- Response formatting

### 🔧 **Advanced Features**

Easily implement complex features:
- Custom validation logic
- Conditional parameter requirements
- Dynamic schema generation
- Complex state management
- Custom notification patterns

### 📚 **Learning Value**

Perfect for understanding:
- MCP protocol internals
- Tool trait implementation
- Session management mechanics
- Notification system architecture
- Schema design patterns

## Error Handling

The server demonstrates comprehensive error handling patterns:

```rust
// Parameter validation
let operation = args.get("operation")
    .and_then(|v| v.as_str())
    .ok_or("Missing operation parameter")?;

// Business logic validation
match operation {
    "divide" if b == 0.0 => Err("Division by zero is not allowed".to_string()),
    "create" if content.is_none() => Err("Content required for create operation".to_string()),
    _ => { /* Process valid operation */ }
}

// Session requirement validation
let session = session.ok_or("Session required for this operation")?;
```

## Session State Structure

Each tool maintains its own session state:

```json
{
  "counter": 42,
  "files": {
    "/example/file.txt": "Hello, World!",
    "/data/config.json": "{\"setting\": \"value\"}"
  },
  "calculation_history": [
    {"operation": "add", "a": 5, "b": 3, "result": 8},
    {"operation": "multiply", "a": 4, "b": 7, "result": 28}
  ]
}
```

## Use Cases

### 1. **Learning MCP Internals**
Perfect for developers who want to understand the underlying mechanics of MCP tools without macro abstractions.

### 2. **Complex Tool Requirements**
Ideal when you need fine-grained control over tool behavior that's difficult to express with macros.

### 3. **Dynamic Schema Generation**
When tool schemas need to be generated dynamically based on runtime conditions.

### 4. **Custom Validation Logic**
For implementing complex validation rules that go beyond simple type checking.

### 5. **Advanced Session Management**
When you need sophisticated session state management with custom persistence patterns.

## Architecture

```
┌─────────────────────┐    ┌──────────────────────┐    ┌─────────────────────┐
│   MCP Client        │────│  Manual Tools Server │────│  Session Manager    │
│                     │    │                      │    │                     │
│ - Tool Calls        │    │ - FileSystemTool     │    │ - State Persistence │
│ - Parameter Input   │    │ - CalculatorTool     │    │ - Session Cleanup   │
│ - Progress Updates  │    │ - SessionCounterTool │    │ - Notification Hub  │
│                     │    │ - ProgressTaskTool   │    │                     │
└─────────────────────┘    └──────────────────────┘    └─────────────────────┘
```

## Testing

```bash
# Start the server
cargo run --bin manual-tools-server &

# Test file operations
curl -X POST http://127.0.0.1:8040/mcp \
  -H "Content-Type: application/json" \
  -d '{"method": "tools/call", "params": {"name": "file_operations", "arguments": {"operation": "create", "path": "/test.txt", "content": "Hello"}}}'

# Test calculator
curl -X POST http://127.0.0.1:8040/mcp \
  -H "Content-Type: application/json" \
  -d '{"method": "tools/call", "params": {"name": "calculator", "arguments": {"operation": "add", "a": 10, "b": 5}}}'
```

This example serves as the foundation for understanding manual MCP tool implementation and provides patterns for building sophisticated, production-ready MCP servers.