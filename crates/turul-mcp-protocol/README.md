# turul-mcp-protocol

[![Crates.io](https://img.shields.io/crates/v/turul-mcp-protocol.svg)](https://crates.io/crates/turul-mcp-protocol)
[![Documentation](https://docs.rs/turul-mcp-protocol/badge.svg)](https://docs.rs/turul-mcp-protocol)

Model Context Protocol (MCP) specification implementation - Current version alias for future-proofing.

## Overview

`turul-mcp-protocol` is a version alias crate that re-exports the current stable version of the Model Context Protocol implementation. This provides future-proofing and consistency across the turul-mcp-framework ecosystem.

**Currently aliases:** `turul-mcp-protocol-2025-06-18`

## Features

- ✅ **Future-Proof API** - Always points to the latest stable MCP specification
- ✅ **Consistent Imports** - Use `turul_mcp_protocol::` throughout your codebase
- ✅ **Seamless Upgrades** - Protocol version changes only require updating this dependency
- ✅ **Complete Re-export** - All types, traits, and constants from the current version
- ✅ **Zero Runtime Cost** - Pure compile-time aliasing with no performance impact

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
turul-mcp-protocol = "0.2.0"
```

### Import Pattern

**✅ ALWAYS use the alias:**
```rust
use turul_mcp_protocol::{
    Tool, CallToolRequest, CallToolResult,
    Resource, ReadResourceRequest, ReadResourceResult,
    InitializeRequest, InitializeResult
};
```

**❌ NEVER import the versioned crate directly:**
```rust
// DON'T DO THIS
use turul_mcp_protocol_2025_06_18::{Tool, CallToolRequest};
```

## Protocol Version Abstraction

### Current Version Information

```rust
use turul_mcp_protocol::{CURRENT_VERSION, MCP_VERSION, McpVersion};

fn check_protocol_version() {
    println!("Current MCP version: {}", CURRENT_VERSION);  // "2025-06-18"
    println!("Protocol constant: {}", MCP_VERSION);        // "2025-06-18"
    
    let version = McpVersion::from_str(CURRENT_VERSION).unwrap();
    assert_eq!(version, McpVersion::V2025_06_18);
}
```

### Feature Compatibility

```rust
use turul_mcp_protocol::McpVersion;

fn check_feature_support(version: McpVersion) -> bool {
    match version {
        McpVersion::V2024_11_05 => {
            // Basic MCP without Streamable HTTP
            false
        }
        McpVersion::V2025_03_26 => {
            // Streamable HTTP support
            true
        }
        McpVersion::V2025_06_18 => {
            // Full feature set with _meta, cursor, progressToken
            true
        }
    }
}
```

## Complete API Re-export

All types and functionality from the current MCP specification are available:

### Core Protocol Types

```rust
use turul_mcp_protocol::{
    // Version and capabilities
    McpVersion, ClientCapabilities, ServerCapabilities,
    Implementation,
    
    // Initialization
    InitializeRequest, InitializeResult,
    
    // Tools
    Tool, ToolSchema, CallToolRequest, CallToolResult,
    ListToolsRequest, ListToolsResult,
    
    // Resources  
    Resource, ReadResourceRequest, ReadResourceResult,
    ListResourcesRequest, ListResourcesResult,
    
    // Prompts
    Prompt, GetPromptRequest, GetPromptResult,
    ListPromptsRequest, ListPromptsResult,
    
    // Sampling
    CreateMessageRequest, CreateMessageResult,
    
    // Notifications
    ProgressNotification, LoggingMessageNotification,
    ResourceUpdatedNotification, ResourceListChangedNotification,
};
```

### Error Types

```rust
use turul_mcp_protocol::McpError;

fn handle_mcp_error(error: McpError) {
    match error {
        McpError::ToolNotFound(name) => println!("Tool not found: {}", name),
        McpError::ResourceNotFound(uri) => println!("Resource not found: {}", uri),
        McpError::InvalidParameters(msg) => println!("Invalid parameters: {}", msg),
        McpError::ToolExecutionError(msg) => println!("Tool execution failed: {}", msg),
        _ => println!("MCP error: {}", error),
    }
}
```

### Trait System

```rust
use turul_mcp_protocol::{
    // MCP Spec Types
    Tool, Resource, Prompt,
    ToolSchema, ResourceContent, PromptMessage,
    McpError, McpResult,

    // Request/Response traits (spec-level)
    HasMethod, HasParams,
    HasData, HasMeta,
};

// For framework traits like HasBaseMetadata, ToolDefinition, etc:
// use turul_mcp_builders::prelude::*;

// Example using spec types
struct MyConfig {
    name: String,
    description: String,
}

impl MyConfig {
    fn to_tool(&self) -> Tool {
        Tool::new(&self.name)
            .with_description(&self.description)
    }
}
```

## Framework Integration

### Server Integration

The protocol types integrate seamlessly with `turul-mcp-server` to build a complete server. The following is a complete, runnable example.

**Dependencies:**

```toml
[dependencies]
turul-mcp-protocol = "0.2.0"
turul-mcp-server = "0.2.0"
turul-mcp-derive = "0.2.0"
tokio = { version = "1.0", features = ["full"] }
```

**Example:**

```rust
use turul_mcp_server::prelude::*;
use turul_mcp_derive::mcp_tool;

#[mcp_tool(name = "my_tool", description = "An example tool")]
async fn my_tool(#[param(description = "A message to echo")] message: String) -> McpResult<String> {
    Ok(format!("You said: {}", message))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = McpServer::builder()
        .name("My MCP Server")
        .version("1.0.0")
        .tool_fn(my_tool)
        .bind_address("127.0.0.1:8080".parse()?)
        .build()?;

    println!("Server listening on http://127.0.0.1:8080");
    server.run().await?;
    Ok(())
}
```

### Client Integration

```rust
use turul_mcp_protocol::{InitializeRequest, CallToolRequest};
use turul_mcp_client::{McpClient, McpClientBuilder, transport::HttpTransport};

let transport = HttpTransport::new("http://localhost:8080/mcp")?;
let client = McpClientBuilder::new()
    .with_transport(Box::new(transport))
    .build();

// Protocol types work directly with client methods
let tools = client.list_tools().await?;
let result = client.call_tool("my_tool", args).await?;
```

## Version Migration Guide

### Future Protocol Updates

When a new MCP protocol version is released, upgrading is seamless:

```rust
// Your code stays the same
use turul_mcp_protocol::{Tool, CallToolRequest, CallToolResult};

// Only the dependency version changes in Cargo.toml:
// [dependencies]
// turul-mcp-protocol = "0.2"  # Now points to MCP 2026-XX-XX
```

### Handling Version-Specific Features

```rust
use turul_mcp_protocol::{McpVersion, CURRENT_VERSION};

fn use_version_specific_feature() {
    let current_version = McpVersion::from_str(CURRENT_VERSION).unwrap();
    
    match current_version {
        McpVersion::V2025_06_18 => {
            // Use current features like _meta support
            println!("Using MCP 2025-06-18 features");
        }
        // Future versions would be handled here
        _ => {
            println!("Using features from version: {}", CURRENT_VERSION);
        }
    }
}
```

## Testing

The alias crate includes comprehensive tests to ensure re-exports work correctly:

```bash
# Test that all re-exports are functional
cargo test --package turul-mcp-protocol

# Test version consistency
cargo test --package turul-mcp-protocol test_current_version

# Test protocol parsing
cargo test --package turul-mcp-protocol test_version_parsing
```

### Version Testing

```rust
#[cfg(test)]
mod tests {
    use turul_mcp_protocol::*;

    #[test]
    fn test_protocol_compatibility() {
        // Test that basic protocol types work
        let implementation = Implementation::new("test", "1.0.0");
        assert_eq!(implementation.name(), "test");
        
        let capabilities = ClientCapabilities::default();
        assert!(capabilities.roots.is_none());
        
        // Test version constants
        assert_eq!(CURRENT_VERSION, "2025-06-18");
        assert_eq!(MCP_VERSION, "2025-06-18");
    }
    
    #[test]
    fn test_all_major_types_available() {
        // Ensure all major protocol types are accessible
        let _tool = Tool::new("test", ToolSchema::object());
        let _resource = Resource::new("file:///test", "Test resource");
        let _prompt = Prompt::new("test");
        
        // If this compiles, re-exports are working
        assert!(true);
    }
}
```

## Architecture Decision Record

### Why Use a Version Alias?

**Decision**: Create `turul-mcp-protocol` as an alias to the current MCP specification version.

**Rationale**:
1. **Future-Proofing**: Applications can upgrade MCP versions by changing one dependency
2. **Consistent Imports**: All framework code uses `turul_mcp_protocol::` imports  
3. **Clear Separation**: Framework code vs specific protocol version implementation
4. **Gradual Migration**: New protocol versions can be adopted incrementally

**Implementation**:
- `turul-mcp-protocol` re-exports the current stable version
- All framework crates use the alias for imports
- Versioned crates contain actual protocol implementations
- Migration path is clear and documented

## Feature Flags

```toml
[dependencies]
turul-mcp-protocol = { version = "0.2", features = ["server"] }
```

Available features:
- `default` - Core protocol types and traits
- `server` - Server-specific functionality and traits
- `client` - Client-specific functionality and helpers

These features are passed through to the underlying protocol implementation.

## Contributing

When contributing to the framework:

1. **Always use the alias**: Import from `turul_mcp_protocol`, not the versioned crate
2. **Update documentation**: When the alias points to a new version, update examples
3. **Test compatibility**: Ensure changes work with the aliased version
4. **Version consistency**: Keep the alias pointing to the latest stable version

## License

Licensed under the MIT License. See [LICENSE](../../LICENSE) for details.