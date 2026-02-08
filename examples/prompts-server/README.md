# Prompts Server Example

This example demonstrates the MCP prompts functionality with dynamic prompt generation. Prompts allow servers to provide pre-defined templates that clients can use for AI interactions.

## Overview

The prompts server provides five different prompt types for common development scenarios:

1. **Code Generation** - Generate code for specific functions with language and style preferences
2. **Documentation** - Create comprehensive documentation for code, APIs, or technical concepts  
3. **Code Review** - Perform thorough code reviews with constructive feedback
4. **Debugging** - Step-by-step troubleshooting guidance for problems
5. **Architecture Design** - Design software architecture and system components

## Features

- **Dynamic Prompt Generation**: Prompts are generated based on user-provided arguments
- **Flexible Parameters**: Each prompt accepts different parameters for customization
- **Comprehensive Coverage**: Covers major development workflow scenarios
- **Production Ready**: Includes proper error handling and validation

## Running the Server

### Quick Start
```bash
# Run the prompts server (default: 127.0.0.1:8040)
cargo run -p prompts-server

# Expected output:
# INFO prompts_server: 🚀 Starting MCP Prompts Server on port 8040
# INFO turul_mcp_server::builder: 🔧 Auto-configured server capabilities:
# INFO turul_mcp_server::builder:    - Prompts: true
# INFO turul_mcp_server::server: ✅ Server started successfully on http://127.0.0.1:8040/mcp
```

### Verify Server is Working
```bash
# Initialize connection (in another terminal)
curl -X POST http://127.0.0.1:8040/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "initialize",
    "params": {
      "protocolVersion": "2025-06-18",
      "capabilities": {},
      "clientInfo": {"name": "test", "version": "1.0"}
    },
    "id": 1
  }' | jq

# Check available prompts  
curl -X POST http://127.0.0.1:8040/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "prompts/list",
    "params": {},
    "id": 2
  }' | jq

# Should show 5 available prompts:
# - code-generation, documentation, code-review, debugging, architecture-design
```

## Available Prompts

### 1. Code Generation (`code-generation`)

Generate code for a specific function with optional language and style preferences.

**Parameters:**
- `function_name` (optional): Name of the function to generate (default: "my_function")
- `language` (optional): Programming language (default: "rust")  
- `description` (optional): What the function should do (default: "a function that does something useful")
- `style` (optional): Coding style preference (default: "clean")

**Example:**
```bash
curl -X POST http://127.0.0.1:8040/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "prompts/get",
    "params": {
      "name": "code-generation",
      "arguments": {
        "function_name": "calculate_fibonacci",
        "language": "python",
        "description": "calculate the nth Fibonacci number efficiently",
        "style": "functional"
      }
    },
    "id": 1
  }'
```

### 2. Documentation (`documentation`)

Generate comprehensive documentation for code, APIs, or technical concepts.

**Parameters:**
- `subject` (required): What to document
- `type` (optional): Documentation type (default: "api")
- `audience` (optional): Target audience (default: "developers")
- `format` (optional): Output format (default: "markdown")

**Example:**
```bash
curl -X POST http://127.0.0.1:8040/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "prompts/get",
    "params": {
      "name": "documentation",
      "arguments": {
        "subject": "REST API for user authentication",
        "type": "api",
        "audience": "frontend developers",
        "format": "markdown"
      }
    },
    "id": 1
  }'
```

### 3. Code Review (`code-review`)

Perform comprehensive code review with suggestions for improvements.

**Parameters:**
- `code` (required): Code to review
- `language` (optional): Programming language (default: "auto-detect")
- `focus` (optional): Review focus area (default: "general")

**Example:**
```bash
curl -X POST http://127.0.0.1:8040/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "prompts/get",
    "params": {
      "name": "code-review",
      "arguments": {
        "code": "def process_data(data):\n    return [x*2 for x in data if x > 0]",
        "language": "python",
        "focus": "performance"
      }
    },
    "id": 1
  }'
```

### 4. Debugging (`debugging`)

Help debug issues with step-by-step troubleshooting guidance.

**Parameters:**
- `problem` (required): Description of the problem
- `context` (optional): Context information (default: "general software development")
- `error_message` (optional): Specific error message if available

**Example:**
```bash
curl -X POST http://127.0.0.1:8040/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "prompts/get",
    "params": {
      "name": "debugging",
      "arguments": {
        "problem": "Application crashes randomly after 5 minutes of usage",
        "context": "Rust web service using Tokio",
        "error_message": "thread '\''main'\'' panicked at '\''called `Result::unwrap()` on an `Err` value: Connection timed out'\''"
      }
    },
    "id": 1
  }'
```

### 5. Architecture Design (`architecture-design`)

Design software architecture and system components with best practices.

**Parameters:**
- `system_type` (required): Type of system to design
- `requirements` (optional): System requirements (default: "general system requirements")
- `scale` (optional): Expected scale (default: "medium")
- `constraints` (optional): Technical constraints (default: "standard constraints")

**Example:**
```bash
curl -X POST http://127.0.0.1:8040/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "prompts/get",
    "params": {
      "name": "architecture-design",
      "arguments": {
        "system_type": "real-time chat application",
        "requirements": "support 10k concurrent users, message history, file sharing",
        "scale": "large",
        "constraints": "must use Kubernetes, PostgreSQL, Redis"
      }
    },
    "id": 1
  }'
```

## Implementation Details

### Prompt Registration

The server uses the enhanced MCP framework builder to register custom prompts:

```rust
let server = McpServer::builder()
    .name("prompts-server")
    .prompt(CodeGenerationPrompt)
    .prompt(DocumentationPrompt)
    .prompt(CodeReviewPrompt)
    .prompt(DebuggingPrompt)
    .prompt(ArchitecturePrompt)
    .with_prompts()
    .build()?;
```

### Custom Prompt Implementation

Each prompt implements the `McpPrompt` trait:

```rust
#[async_trait]
impl McpPrompt for CodeGenerationPrompt {
    fn name(&self) -> &str {
        "code-generation"
    }

    fn description(&self) -> &str {
        "Generate code for a specific function with optional language and style preferences"
    }

    async fn generate(&self, args: HashMap<String, Value>) -> McpResult<Vec<PromptMessage>> {
        // Extract parameters with defaults
        let function_name = args.get("function_name")
            .and_then(|v| v.as_str())
            .unwrap_or("my_function");
        
        // Generate dynamic prompt content
        let prompt_content = format!("...", /* template variables */);
        
        Ok(vec![PromptMessage::text(prompt_content)])
    }
}
```

## Protocol Compliance

This example follows the MCP 2025-11-25 specification for prompts:

- **prompts/list**: Returns available prompts with descriptions
- **prompts/get**: Generates prompt content based on provided arguments
- **Dynamic Generation**: Prompts are generated at request time with user parameters
- **Proper Error Handling**: Missing required parameters return appropriate errors

## Use Cases

- **AI-Assisted Development**: Provide structured prompts for code generation tasks
- **Documentation Automation**: Generate documentation templates for different audiences
- **Code Review Automation**: Create consistent code review prompts
- **Troubleshooting Guides**: Systematic debugging assistance
- **Architecture Planning**: Structured approach to system design

## Integration

This prompts server can be integrated with:

- **AI/LLM Clients**: Use prompts with ChatGPT, Claude, or other AI models
- **IDEs and Editors**: Integrate prompts into development tools
- **CI/CD Pipelines**: Automated code review and documentation generation
- **Development Workflows**: Standardize development processes across teams