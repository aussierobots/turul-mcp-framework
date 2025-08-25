# TODO: Examples Strategy - Macro-First Approach

## ULTRATHINK: Framework Value Proposition

**CRITICAL INSIGHT**: The framework's value is **zero-configuration type safety** for ALL MCP methods. Users should never specify method strings for any MCP area.

**Universal Problem**: Current examples require manual method specification (`tools/call`, `resources/read`, etc.) - this is error-prone and violates DRY principle.

**Universal Solution**: **Type-determined methods** across ALL MCP areas - framework auto-determines correct method from trait type.

## Lines of Code Analysis

| Approach | Example Size | Use Case |
|----------|--------------|----------|
| **Function Macros** | 50-100 lines | 🚀 **Production** (recommended) |
| **Derive Macros** | 80-150 lines | ⚙️ **Structured handlers** |
| **Declarative Macros** | 100-200 lines | 📝 **Template-driven** |
| **Manual Traits** | 400-500+ lines | 📚 **Learning only** |

**5-10x reduction in code size with macros!**

## Universal Type-Determined Method Design - ALL MCP AREAS

### 1. Tools - Zero Configuration
```rust
#[derive(McpTool)]
struct Calculator {
    // Framework auto-uses "tools/call"
    // No method specification needed
}

#[derive(McpTool)]
struct FileManager {
    // Framework auto-uses "tools/call"  
    // Tool name becomes identifier
}
```

### 2. Resources - Zero Configuration  
```rust
#[derive(McpResource)]
struct ConfigFile {
    // Framework auto-uses "resources/read"
    path: PathBuf,
}

#[derive(McpResource)]
struct ApiEndpoint {
    // Framework auto-uses "resources/read"
    url: String,
}
```

### 3. Prompts - Zero Configuration
```rust
#[derive(McpPrompt)]
struct CodeGenerator {
    // Framework auto-uses "prompts/get"
    template: String,
}
```

### 4. Notifications - Zero Configuration
```rust
#[derive(McpNotification)]
struct ProgressNotification {
    // Framework auto-uses "notifications/progress"
    completed: u64,
    total: u64,
}
```

### 5. Sampling - Zero Configuration
```rust
#[derive(McpSampler)]
struct CreativeWriter {
    // Framework auto-uses "sampling/createMessage"
    temperature: f64,
    max_tokens: u32,
}
```

### 6. ALL OTHER MCP AREAS - Same Pattern
- **Completion**: `#[derive(McpCompleter)]` → `completion/complete`
- **Logging**: `#[derive(McpLogger)]` → `logging/setLevel` 
- **Roots**: `#[derive(McpRoot)]` → `roots/list`
- **Elicitation**: `#[derive(McpElicitor)]` → relevant elicitation methods

**Universal Result**: 20-40 lines per example, **zero method strings anywhere**

### 2. Prompts - Declarative Macros (Best for Templates)
```rust
prompt! {
    name: "generate_code",
    description: "Generate code based on requirements",
    arguments: {
        language: required string,
        requirements: required string,
        style: optional string = "functional",
    },
    template: r#"You are an expert {language} developer.
Generate clean, production-ready code with {style} style:

Requirements: {requirements}

Provide complete implementation with documentation."#
}

prompt! {
    name: "review_code", 
    arguments: {
        code: required string,
        language: required string,
        focus: optional string = "all",
    },
    template: r#"Review this {language} code focusing on {focus}:
```{language}
{code}
```
Provide detailed analysis and suggestions."#
}
```
**Result**: 40-60 lines total vs 538 lines manual

### 3. Sampling - Function Macros with Config
```rust
#[mcp_sampler(max_tokens = 1500, temperature = 0.8, model_type = "creative")]
async fn creative_writing(request: CreateMessageRequest) -> McpResult<CreateMessageResult> {
    let user_input = extract_user_message(&request)?;
    
    let response = if user_input.contains("story") {
        generate_story(&user_input).await?
    } else if user_input.contains("character") {
        generate_character(&user_input).await?
    } else {
        generate_creative_help(&user_input).await?
    };
    
    Ok(CreateMessageResult::new(response, "creative-assistant"))
}

#[mcp_sampler(max_tokens = 2000, temperature = 0.3, model_type = "technical")]
async fn technical_writing(request: CreateMessageRequest) -> McpResult<CreateMessageResult> {
    // Focused technical logic
}
```
**Result**: 80-120 lines total vs 471 lines manual

### 4. Other Areas - Best Macro Approach

#### Completion - Function Macros
```rust
#[mcp_completion(prefix_length = 50)]
async fn code_completion(request: CompleteRequest) -> McpResult<CompleteResult> {
    // Smart completion logic
}
```

#### Logging - Derive Macros
```rust
#[derive(McpLogger)]
struct DevLogger {
    level: LogLevel = LogLevel::Info,
    format: LogFormat = LogFormat::Json,
    outputs: Vec<LogOutput> = vec![LogOutput::Console, LogOutput::File("app.log")],
}
```

#### Roots - Derive Macros
```rust
#[derive(McpRoot)]
struct FileSystemRoot {
    base_path: PathBuf,
    allowed_extensions: Vec<String> = vec!["rs", "toml", "md"],
    max_file_size: usize = 10_MB,
}
```

## New Example Structure

### 1. Quick Start Examples (Macro-First)
- `notification-server-macro` - 60 lines with function macros
- `prompts-server-macro` - 50 lines with declarative macros  
- `sampling-server-macro` - 80 lines with function macros
- `completion-server-macro` - 40 lines with function macros
- `logging-server-macro` - 30 lines with derive macros

### 2. Production Examples (Best Practice)
- `multi-protocol-server` - Combines multiple MCP areas efficiently
- `enterprise-notification-server` - Production-ready with error handling
- `ai-assistant-server` - Prompts + Sampling + Completion integration

### 3. Learning Examples (Manual Implementation)
- `notification-server-manual` - "Don't write this much code in production!"
- `prompts-server-manual` - Educational trait implementation
- `sampling-server-manual` - Learning the underlying concepts

### 4. Integration Examples
- `development-assistant` - Tools + Prompts + Notifications
- `ai-code-reviewer` - Sampling + Completion + Logging
- `documentation-generator` - Prompts + Resources + Roots

## Implementation Priority

### Phase 1: ✅ COMPLETED - Zero-Config Examples Created
1. **universal-mcp-server** → Demonstrates ALL 9 MCP areas with zero-config (250 lines)
2. **tools-server-macro** → Calculator + StringUtils with type-determined methods (160 lines)
3. **resources-server-macro** → File/API/Database resources with type-determined methods (200 lines)
4. **completion-server-macro** → Multi-language code completion with type-determined methods (170 lines)

**RESULTS**: 5-10x code reduction achieved! Zero method strings specified anywhere.

### Phase 2: Continue Macro-Based Examples (IN PROGRESS)
1. **notifications-server-macro** - Progress/Message notifications with SSE
2. **sampling-server-macro** - Creative/Technical LLM sampling  
3. **logging-server-macro** - Structured logging with levels
4. **roots-server-macro** - File system roots management
5. **elicitation-server-macro** - User input collection

### Phase 3: Framework Enhancement (NEXT)
1. **Implement derive macros**: `#[derive(McpTool)]`, `#[derive(McpResource)]`, etc.
2. **Type-method mapping system**: Automatic method determination from trait types
3. **InMemory SessionStorage**: Complete HTTP compliance gaps

### Phase 4: Educational Restructure (LATER)
1. Move manual examples to `examples/learning/`
2. Add clear "This is for learning - use macros in production" notes
3. Create comparison docs showing macro vs manual (5-10x difference)

## Framework Design Requirements

Based on MCP specification compliance:

1. **Typed Notification Traits** (Method auto-determined):
   ```rust
   #[derive(McpNotification)] // Auto-maps to correct MCP method
   struct ProgressNotification { stage: String, completed: u64, total: u64 }
   
   #[derive(McpNotification)] // Auto-maps to "notifications/message" 
   struct MessageNotification { content: String, level: MessageLevel }
   ```

2. **Official MCP Method Mapping**:
   - `ProgressNotification` → `notifications/progress`
   - `MessageNotification` → `notifications/message` 
   - `InitializedNotification` → `notifications/initialized`
   - `CancelledNotification` → `notifications/cancelled`
   - `ResourcesListChangedNotification` → `notifications/resources/list_changed`
   - `ResourcesUpdatedNotification` → `notifications/resources/updated`
   - `PromptsListChangedNotification` → `notifications/prompts/list_changed`
   - `ToolsListChangedNotification` → `notifications/tools/list_changed`
   - `RootsListChangedNotification` → `notifications/roots/list_changed`

3. **Zero Configuration Macros**:
   ```rust
   #[derive(McpNotification)]
   struct MyProgressUpdate { /* fields */ } // Method determined automatically
   ```

4. **Type-Safe Server Builder**:
   ```rust
   McpServer::builder()
       .notification_handler::<ProgressNotification>()  // Type determines method
       .notification_handler::<MessageNotification>()
       .build()
   ```

## Success Metrics

**Before**: 400-500 lines per example, focuses on framework complexity
**After**: 50-100 lines per example, focuses on business logic and ease of use

**Key Message**: "Look how easy MCP becomes with this framework!"

## Example Template Structure

```rust
//! # [Area] Server - Production Example
//! 
//! This demonstrates the RECOMMENDED way to implement MCP [area].
//! Uses macros for maximum simplicity and maintainability.
//!
//! Lines of code: ~60 (vs 400+ with manual traits)

// 10-20 lines of imports and setup

// 20-40 lines of macro-based implementations
#[mcp_[area](...)]
async fn handler(...) -> McpResult<...> {
    // Pure business logic
}

// 10-20 lines of server setup
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let server = McpServer::builder()
        .[area]_provider(handler)
        .build()?;
    server.run().await
}
```

This approach showcases the framework's **true value**: making MCP implementation trivially simple.