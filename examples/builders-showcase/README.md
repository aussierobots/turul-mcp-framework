# Builders Showcase - Complete MCP Development Environment

This example demonstrates **all 9 MCP runtime builders** working together to create a sophisticated development environment management server. It showcases the full power of the `mcp-builders` crate for building complex MCP servers entirely at runtime.

## 🎯 What This Example Demonstrates

### Complete Builder Coverage

1. **🔧 ToolBuilder** - Development Tools
   - `create_project`: Project scaffolding and initialization
   - `build_project`: Build system with real-time progress notifications

2. **📄 ResourceBuilder** - Configuration & Templates
   - `project_config`: Dynamic JSON configuration resource
   - `component_template`: Code generation templates

3. **💬 PromptBuilder** - Interactive AI Assistance
   - `generate_code`: Code generation with templated prompts
   - `analyze_refactoring`: Code analysis and improvement suggestions

4. **🤖 MessageBuilder** - AI Message Configuration
   - Code review message settings (low temperature for consistency)
   - Architecture discussion settings (higher temperature for creativity)

5. **📝 CompletionBuilder** - Autocomplete Support
   - Command argument completion for prompts
   - Resource field completion for configurations

6. **📁 RootBuilder** - Directory Access Management
   - Source code root (read-write access)
   - Configuration root (read-only for safety)
   - Temporary workspace (full access)

7. **📋 ElicitationBuilder** - User Input Collection
   - Project setup wizard with validation
   - Development preferences configuration

8. **📡 NotificationBuilder** - Real-time Updates
   - Build progress notifications
   - Deployment status updates
   - System alerts with priority handling

9. **📊 LoggingBuilder** - Comprehensive Activity Tracking
   - Development activity logging
   - Performance metrics collection
   - Error tracking and reporting
   - Security audit logging

## 🚀 Running the Example

```bash
# From the repository root
cargo run --example builders-showcase
```

The server will start at `http://127.0.0.1:8080/mcp` and provide:

```
✅ Server created with all 9 builder types:
   🔧 Tools: Project management (create, build, test, deploy)
   📄 Resources: Configuration files and templates
   💬 Prompts: Code generation and refactoring
   🤖 Messages: AI-powered development assistance
   📝 Completion: Autocomplete for commands and paths
   📁 Roots: Source code and workspace directory access
   📋 Elicitation: Project setup and preference collection
   📡 Notifications: Real-time build and deployment updates
   📊 Logging: Development activity and performance tracking
```

## 🧪 Testing with MCP Inspector

Connect to the server using [MCP Inspector](https://github.com/modelcontextprotocol/inspector):

1. **Tools**: Try the `create_project` and `build_project` tools
2. **Resources**: Access `project_config` and `component_template` 
3. **Prompts**: Use `generate_code` and `analyze_refactoring`
4. **Notifications**: Watch real-time progress during builds
5. **Elicitation**: Fill out the project setup wizard

## 💡 Key Features Demonstrated

### Runtime Flexibility
- **No compile-time macros required** - everything built at runtime
- **Dynamic configuration** - tools and resources created from data
- **Hot-swappable components** - modify behavior without recompilation

### MCP Protocol Compliance
- **Full MCP 2025-06-18 specification** support
- **Proper schema validation** for all inputs and outputs  
- **Real-time notifications** with SSE streaming
- **Session management** with progress tracking

### Production-Ready Patterns
- **Comprehensive error handling** with structured responses
- **Progress notifications** for long-running operations
- **Structured logging** with metadata and context
- **Security-conscious** root access permissions
- **User input validation** with field constraints

### Advanced Builder Features

#### ToolBuilder Advanced Usage
```rust
let build_tool = ToolBuilder::new("build_project")
    .description("Build with real-time progress")
    .execute_with_session(|args, session| async move {
        // Access session context for notifications
        if let Some(ctx) = &session {
            ctx.notify_progress("build", 50, Some(100), Some("Compiling...")).await?;
        }
        // ... tool logic
    })
    .build()?;
```

#### ResourceBuilder Dynamic Content
```rust
let config = ResourceBuilder::new("file:///config.json")
    .json_content(json!({
        "dynamic_field": format!("Generated at {}", chrono::Utc::now())
    }))
    .build()?;
```

#### NotificationBuilder with Metadata
```rust
let notification = NotificationBuilder::progress("task-id", 75)
    .total(100)
    .message("Processing...")
    .meta_value("task_type", json!("compilation"))
    .meta_value("estimated_completion", json!("30s"))
    .build();
```

## 🔧 Customizing the Example

### Adding More Tools
```rust
let custom_tool = ToolBuilder::new("my_custom_tool")
    .description("Your custom development tool")
    .string_param("input", "Tool input")
    .execute(|args| async move {
        // Your tool logic here
        Ok(json!({"result": "success"}))
    })
    .build()?;

server = server.tool(custom_tool);
```

### Creating Dynamic Resources
```rust
let dynamic_resource = ResourceBuilder::new("file:///dynamic.json")
    .name("dynamic_config")
    .dynamic_content(Box::new(|| async move {
        // Generate content dynamically
        Ok(json!({
            "timestamp": chrono::Utc::now(),
            "system_info": get_system_info().await
        }).to_string())
    }))
    .build()?;
```

### Advanced Elicitation Forms
```rust
let advanced_form = ElicitationBuilder::new("Advanced Project Setup")
    .string_field_with_constraints("repo_url", "Git repository URL", 
        Some(r"^https://github\.com/.+"), None, Some(200))
    .enum_field("license", "License type", vec![
        "MIT".to_string(), "Apache-2.0".to_string(), "GPL-3.0".to_string()
    ])
    .integer_field_with_range("port", "Server port", Some(1024.0), Some(65535.0))
    .require_fields(vec!["repo_url".to_string(), "license".to_string()])
    .build();
```

## 🏗️ Architecture Benefits

### Single Source of Truth
All MCP protocol handling is unified through the builder pattern, ensuring consistency and reducing boilerplate.

### Type Safety
Runtime builders maintain compile-time type safety while providing runtime flexibility.

### Zero Configuration
No need to specify MCP method strings - the framework automatically determines all protocol methods from builder types.

### Extensibility  
Easy to add new capabilities by creating additional builders without modifying existing code.

### Testability
Each builder can be unit tested independently, and the entire server configuration is testable.

## 🎓 Learning Outcomes

After exploring this example, you'll understand how to:

1. **Build comprehensive MCP servers** using only runtime builders
2. **Integrate all MCP protocol areas** into a cohesive system
3. **Handle real-time notifications** and progress tracking
4. **Manage user input collection** with validation
5. **Implement structured logging** with metadata
6. **Configure directory access** with appropriate permissions
7. **Create dynamic content** that updates at runtime
8. **Design intuitive developer tools** with proper error handling

This example serves as a complete reference for building sophisticated MCP servers using the builders pattern - perfect for configuration-driven systems, plugin architectures, and dynamic tool environments!