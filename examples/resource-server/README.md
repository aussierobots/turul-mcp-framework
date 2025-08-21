# Resource Server Example

A comprehensive demonstration of **MCP resource handling** using the `#[derive(McpResource)]` macro for creating resources with minimal boilerplate code. This example showcases various resource types and content handling patterns.

## Overview

This server demonstrates how to implement MCP resources using derive macros, showing different resource patterns including configuration files, system status, and user data. It provides a foundation for building resource-rich MCP servers.

## Features

### 📦 **Resource Types Demonstrated**
- **Configuration Resource** - JSON configuration with typed content
- **System Status Resource** - Dynamic system information
- **User Profile Resource** - Multi-content resource example
- **Derive macro powered** - Minimal boilerplate code

### 🔧 **Advanced Resource Features**
- **Multiple content types** - JSON, text, binary support
- **Content-Type handling** - Proper MIME type specification
- **Dynamic content generation** - Real-time data resources
- **URI-based addressing** - Structured resource identification

## Quick Start

### 1. Start the Server

```bash
cargo run --bin resource-server
```

The server will start on `http://127.0.0.1:8045/mcp`

### 2. Access Resources

#### Configuration Resource
```json
{
  "method": "resources/read",
  "params": {
    "uri": "file://config.json"
  }
}
```

#### System Status Resource
```json
{
  "method": "resources/read",
  "params": {
    "uri": "system://status"
  }
}
```

#### User Profile Resource
```json
{
  "method": "resources/read",
  "params": {
    "uri": "data://user-profile"
  }
}
```

#### List All Resources
```json
{
  "method": "resources/list"
}
```

## Resource Reference

### 📄 `file://config.json`

Application configuration resource with JSON content.

**Content Type:** `application/json`
**Features:**
- Static configuration data
- JSON content with pretty formatting
- Application settings and feature flags

**Sample Content:**
```json
{
  "app_name": "MCP Resource Server",
  "version": "1.0.0", 
  "debug": true,
  "features": ["resources", "derive_macros", "json_config"]
}
```

### 🖥️ `system://status`

Dynamic system status and health information.

**Content Type:** `application/json`
**Features:**
- Real-time system metrics
- Memory and CPU information
- Server uptime and health status
- Dynamic content generation

**Sample Content:**
```json
{
  "status": "healthy",
  "uptime_seconds": 1234,
  "memory_usage": "45.2 MB",
  "cpu_usage": "12.3%",
  "timestamp": "2025-01-01T12:00:00Z",
  "resources_served": 42
}
```

### 👤 `data://user-profile`

User profile data with multiple content sections.

**Content Type:** `application/json`
**Features:**
- Multi-content resource
- User information and preferences
- Profile data with metadata

**Sample Content:**
```json
{
  "user_id": "user123",
  "name": "John Doe",
  "email": "john@example.com",
  "preferences": {
    "theme": "dark",
    "language": "en",
    "notifications": true
  },
  "created_at": "2024-01-01T00:00:00Z",
  "last_login": "2025-01-01T12:00:00Z"
}
```

## Derive Macro Implementation

### Configuration Resource

```rust
#[derive(McpResource, Serialize, Deserialize)]
#[uri = "file://config.json"]
#[name = "Application Configuration"]
#[description = "Main application configuration file"]
struct ConfigResource {
    #[content]
    #[content_type = "application/json"]
    pub config_data: String,
}

impl ConfigResource {
    fn new() -> Self {
        let config = serde_json::json!({
            "app_name": "MCP Resource Server",
            "version": "1.0.0",
            "debug": true,
            "features": ["resources", "derive_macros", "json_config"]
        });
        
        Self {
            config_data: serde_json::to_string_pretty(&config).unwrap(),
        }
    }
}
```

### System Status Resource (Unit Struct)

```rust
#[derive(McpResource)]
#[uri = "system://status"]
#[name = "System Status"]
#[description = "Current system status and health information"]
struct SystemStatusResource;
```

### Multi-Content Resource

```rust
#[derive(McpResource, Serialize, Deserialize)]
#[uri = "data://user-profile"]
#[name = "User Profile"]
#[description = "User profile data with multiple content sections"]
struct UserProfileResource {
    #[content]
    #[content_type = "application/json"]
    pub profile_data: String,
    
    #[content]
    #[content_type = "text/plain"]
    pub bio: String,
}
```

## Manual vs. Derive Implementation

### Before (Manual Implementation)
```rust
// Manual resource implementation - 40+ lines
#[derive(Clone)]
struct ConfigResource {
    config_data: String,
}

#[async_trait]
impl McpResource for ConfigResource {
    fn uri(&self) -> &str {
        "file://config.json"
    }

    fn name(&self) -> &str {
        "Application Configuration"
    }

    fn description(&self) -> &str {
        "Main application configuration file"
    }

    async fn read(&self) -> McpResult<Vec<ResourceContent>> {
        Ok(vec![ResourceContent::blob(
            self.config_data.clone(),
            "application/json".to_string()
        )])
    }
}
```

### After (Derive Macro)
```rust
// Derive macro implementation - 8 lines!
#[derive(McpResource, Serialize, Deserialize)]
#[uri = "file://config.json"]
#[name = "Application Configuration"]
#[description = "Main application configuration file"]
struct ConfigResource {
    #[content]
    #[content_type = "application/json"]
    pub config_data: String,
}
```

## Attribute Reference

### Resource-Level Attributes
- `#[uri = "..."]` - Resource URI for identification
- `#[name = "..."]` - Human-readable resource name
- `#[description = "..."]` - Resource description

### Field-Level Attributes  
- `#[content]` - Marks field as resource content
- `#[content_type = "..."]` - MIME type for content

## Supported Content Types

The derive macro supports various content types:

```rust
#[derive(McpResource)]
#[uri = "example://multi-content"]
#[name = "Multi-Content Example"]
#[description = "Example showing different content types"]
struct MultiContentResource {
    #[content]
    #[content_type = "application/json"]
    json_data: String,
    
    #[content]
    #[content_type = "text/plain"]
    text_data: String,
    
    #[content]
    #[content_type = "text/html"]
    html_data: String,
    
    #[content]
    #[content_type = "application/octet-stream"]
    binary_data: Vec<u8>,
}
```

## Dynamic Content Generation

For dynamic resources, implement custom logic:

```rust
impl SystemStatusResource {
    async fn generate_status(&self) -> String {
        let status = serde_json::json!({
            "status": "healthy",
            "uptime_seconds": self.get_uptime(),
            "memory_usage": self.get_memory_usage(),
            "cpu_usage": self.get_cpu_usage(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "resources_served": self.get_resource_count()
        });
        
        serde_json::to_string_pretty(&status).unwrap()
    }
}
```

## Server Configuration

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let server = McpServer::builder()
        .name("resource-server")
        .version("1.0.0")
        .title("MCP Resource Server Example")
        .instructions("Demonstrates MCP resource handling with derive macros")
        .resource(ConfigResource::new())
        .resource(SystemStatusResource)
        .resource(UserProfileResource::new())
        .bind_address("127.0.0.1:8045".parse()?)
        .build()?;

    server.run().await?;
    Ok(())
}
```

## Resource URI Schemes

The example demonstrates various URI schemes:

| Scheme | Example | Use Case |
|--------|---------|----------|
| `file://` | `file://config.json` | File-based resources |
| `system://` | `system://status` | System information |
| `data://` | `data://user-profile` | Application data |
| `api://` | `api://external-service` | External API proxies |
| `memory://` | `memory://cache` | In-memory data |

## Testing

```bash
# Start the server
cargo run --bin resource-server &

# List all resources
curl -X POST http://127.0.0.1:8045/mcp \
  -H "Content-Type: application/json" \
  -d '{"method": "resources/list"}'

# Read configuration resource
curl -X POST http://127.0.0.1:8045/mcp \
  -H "Content-Type: application/json" \
  -d '{"method": "resources/read", "params": {"uri": "file://config.json"}}'

# Get system status
curl -X POST http://127.0.0.1:8045/mcp \
  -H "Content-Type: application/json" \
  -d '{"method": "resources/read", "params": {"uri": "system://status"}}'
```

## Error Handling

The derive macro provides automatic error handling:

### Resource Not Found
```json
{
  "method": "resources/read",
  "params": {
    "uri": "file://nonexistent.json"
  }
}
```
**Response:** `"Resource not found: file://nonexistent.json"`

### Invalid URI Format
```json
{
  "method": "resources/read", 
  "params": {
    "uri": "invalid-uri"
  }
}
```
**Response:** `"Invalid resource URI format"`

## Use Cases

### 1. **Configuration Management**
Serve application configuration files through MCP resources.

### 2. **System Monitoring**
Provide real-time system status and health information.

### 3. **Data Access**
Expose application data through structured resource interfaces.

### 4. **API Proxying**
Proxy external APIs through MCP resource endpoints.

### 5. **Content Management**
Serve various content types (JSON, text, binary) through unified interface.

## Architecture

```
┌─────────────────────┐    ┌──────────────────────┐    ┌─────────────────────┐
│   MCP Client        │────│  Resource Server     │────│  Resource Providers │
│                     │    │                      │    │                     │
│ - Resource Lists    │    │ - ConfigResource     │    │ - Configuration     │
│ - Resource Reads    │    │ - SystemResource     │    │ - System Status     │
│ - URI Navigation    │    │ - UserResource       │    │ - User Data         │
│ - Content Handling  │    │ - Derive Macros      │    │ - Dynamic Content   │
└─────────────────────┘    └──────────────────────┘    └─────────────────────┘
```

This example demonstrates how derive macros can dramatically simplify MCP resource implementation while providing full functionality for serving various types of content through the MCP protocol.