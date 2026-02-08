# MCP Elicitation Server Example

This example demonstrates the Model Context Protocol (MCP) elicitation functionality for structured user input collection via JSON Schema. Elicitation enables servers to request structured data from users through interactive forms, confirmations, and input dialogs rendered by MCP clients.

## Overview

The MCP elicitation protocol allows servers to:
- Request structured user input using JSON Schema definitions
- Present interactive forms, confirmations, and validation dialogs
- Track progress through multi-step workflows
- Handle errors, cancellations, and timeouts gracefully
- Collect typed data with client-side validation

This server demonstrates 7 different elicitation patterns covering the most common use cases.

## Features Demonstrated

### 1. Simple Text Input (`simple_text_input`)
Basic text field collection with schema validation:
- Single text field with description
- Required field validation
- Title and context description

### 2. Number Input with Validation (`number_input_validation`)
Numeric input with constraints and defaults:
- Min/max value validation (age 13-120)
- Default value provision (25)
- Client-side validation before submission

### 3. Choice Selection (`choice_selection`)
Enum-based selection from predefined options:
- Multiple choice from 8 programming languages
- String enum schema generation
- Default selection (Rust)
- Various UI presentation options (dropdown, radio buttons, etc.)

### 4. Boolean Confirmation (`confirmation_dialog`)
Critical action confirmation dialogs:
- Yes/No confirmation for destructive operations
- Progress token tracking
- No default value (forces explicit choice)
- Modal dialog presentation

### 5. Complex Multi-Field Forms (`complex_form`)
Sophisticated forms with multiple field types:
- 6 different field types (string, email, number, enum, boolean, textarea)
- Required vs optional field distinction
- Mixed validation rules (age 18-100, country selection)
- Default values and advanced form patterns

### 6. Progress-Tracked Workflows (`progress_elicitation`)
Multi-step workflows with progress indication:
- Progress token for step correlation
- Workflow step indication (Step 1 of 3)
- Complex guided user interactions
- Cancellation handling throughout process

### 7. Error Handling Scenarios (`elicitation_error_handling`)
Comprehensive error and edge case handling:
- **Validation Errors**: Client-side schema validation
- **User Cancellation**: Graceful cancellation with notifications
- **Timeout Scenarios**: Time-limited input handling
- **Invalid Schema**: Schema validation before UI presentation

## Running the Server

```bash
# From the workspace root
cargo run -p elicitation-server

# Or with custom logging
RUST_LOG=debug cargo run -p elicitation-server
```

The server runs at `http://127.0.0.1:8053/mcp` and logs all available tools on startup.

## JSON Schema Patterns

### Text Input Schema
```json
{
  "type": "object",
  "properties": {
    "full_name": {
      "type": "string",
      "description": "Your full name (first and last name)"
    }
  },
  "required": ["full_name"]
}
```

### Number Input with Constraints
```json
{
  "type": "object", 
  "properties": {
    "age": {
      "type": "number",
      "description": "Your age in years",
      "minimum": 13,
      "maximum": 120
    }
  },
  "required": ["age"]
}
```

### Choice/Enum Selection
```json
{
  "type": "object",
  "properties": {
    "preferred_language": {
      "type": "string",
      "description": "Your preferred programming language",
      "enum": ["Rust", "Python", "JavaScript", "TypeScript", "Go", "Java", "C++", "Swift"]
    }
  },
  "required": ["preferred_language"]
}
```

### Boolean Confirmation
```json
{
  "type": "object",
  "properties": {
    "confirmed": {
      "type": "boolean",
      "description": "User confirmation"
    }
  },
  "required": ["confirmed"]
}
```

### Complex Multi-Field Form
```json
{
  "type": "object",
  "properties": {
    "full_name": {"type": "string", "description": "Your full name"},
    "email": {"type": "string", "description": "Your email address"},
    "age": {"type": "number", "minimum": 18, "maximum": 100},
    "country": {"type": "string", "enum": ["United States", "Canada", ...]},
    "newsletter": {"type": "boolean", "description": "Subscribe to newsletter"},
    "bio": {"type": "string", "description": "Brief bio (max 500 characters)"}
  },
  "required": ["full_name", "email", "age", "country"]
}
```

## Testing Examples

### Simple Text Input
```bash
curl -X POST http://127.0.0.1:8053/mcp \
  -H 'Content-Type: application/json' \
  -d '{
    "method": "tools/call",
    "params": {
      "name": "simple_text_input",
      "arguments": {}
    }
  }'
```

### Number Input with Validation
```bash
curl -X POST http://127.0.0.1:8053/mcp \
  -H 'Content-Type: application/json' \
  -d '{
    "method": "tools/call", 
    "params": {
      "name": "number_input_validation",
      "arguments": {}
    }
  }'
```

### Error Handling Demo
```bash
curl -X POST http://127.0.0.1:8053/mcp \
  -H 'Content-Type: application/json' \
  -d '{
    "method": "tools/call",
    "params": {
      "name": "elicitation_error_handling", 
      "arguments": {
        "scenario": "validation_error"
      }
    }
  }'
```

## Elicitation Builder Utility

The `ElicitationBuilder` provides convenient methods for common patterns:

```rust
// Simple text input
let elicitation = ElicitationBuilder::text_input(
    "Enter your username",
    "username", 
    "Your unique username"
);

// Number input with constraints  
let elicitation = ElicitationBuilder::number_input(
    "Enter your age",
    "age",
    "Your age in years", 
    Some(13.0), // min
    Some(120.0) // max
);

// Choice selection
let choices = vec!["Rust".to_string(), "Python".to_string()];
let elicitation = ElicitationBuilder::choice_input(
    "Choose language",
    "language",
    "Preferred language",
    choices
);

// Boolean confirmation
let elicitation = ElicitationBuilder::confirm(
    "Are you sure you want to delete all files?"
);

// Complex multi-field form
let fields = vec![
    ("name".to_string(), JsonSchema::string()),
    ("age".to_string(), JsonSchema::number().with_minimum(18.0)),
];
let required = vec!["name".to_string()];
let elicitation = ElicitationBuilder::form(
    "Complete your profile", 
    fields,
    required
);
```

## MCP Client Integration

In a real MCP client implementation, elicitation requests would:

1. **Parse JSON Schema**: Extract field definitions and validation rules
2. **Generate UI**: Create forms, dialogs, and input controls dynamically
3. **Client-side Validation**: Validate user input against schema before submission
4. **Progress Tracking**: Show workflow progress and step indicators
5. **Error Handling**: Display validation errors and handle cancellations
6. **Response Submission**: Send structured data back to server

## Key Benefits

### ✨ Schema-Driven UI Generation
- Automatic form generation from JSON Schema
- Consistent validation and presentation rules
- Type safety and data integrity

### 🔒 Client-Side Input Validation  
- Immediate feedback for validation errors
- No server round-trips for basic validation
- Better user experience with real-time validation

### 🎨 Flexible UI Presentation Options
- Multiple ways to present same schema (dropdowns, radio buttons, etc.)
- Client can choose optimal UI patterns
- Responsive and accessible form generation

### 📊 Progress Tracking for Complex Flows
- Multi-step workflow coordination
- Progress tokens for step correlation
- Cancellation handling at any step

### 🛡️ Robust Error Handling
- Graceful handling of user cancellations
- Timeout protection for long operations  
- Clear error messages and recovery options

## Protocol Details

### Elicitation Request
```json
{
  "method": "elicitation/request",
  "params": {
    "schema": { /* JSON Schema */ },
    "prompt": "Please enter your information",
    "title": "User Information",
    "description": "Additional context...",
    "defaults": { "field": "default_value" },
    "required": false,
    "progressToken": "unique_token_123"
  }
}
```

### Elicitation Response  
```json
{
  "result": {
    "data": { /* User input matching schema */ },
    "completed": true,
    "message": "Thank you for your input"
  }
}
```

### Cancellation Notification
```json
{
  "method": "notifications/elicitation/cancelled",
  "params": {
    "reason": "User cancelled the operation",
    "progressToken": "unique_token_123"
  }
}
```

## Architecture Integration

This elicitation server integrates with the MCP framework using:

- **`ElicitationHandler`**: Core handler for `elicitation/request` endpoint
- **`.with_elicitation()`**: Builder method to enable elicitation capability  
- **`ElicitationBuilder`**: Utility for common elicitation patterns
- **Protocol Types**: Complete type definitions for requests/responses
- **Progress Tokens**: Workflow tracking and correlation
- **Meta Support**: MCP `_meta` field integration

The implementation follows MCP 2025-11-25 specification for maximum client compatibility.

## Next Steps

To extend this example:

1. **Add Custom Validation**: Implement custom JSON Schema validation patterns
2. **Multi-Step Workflows**: Create complex guided workflows with branching logic
3. **Conditional Fields**: Implement schema fields that appear/disappear based on other inputs
4. **File Upload**: Add file selection and upload elicitation patterns  
5. **Real-time Updates**: Integrate with SSE for real-time form updates
6. **Database Integration**: Connect elicitation results to persistent storage
7. **Authentication**: Add user authentication and session-aware elicitation