# Resource Macro Example

A demonstration of the **`resource!` declarative macro** for creating MCP resources with inline content generation closures. This example shows the most concise way to create simple MCP resources with dynamic content.

## Overview

This example demonstrates the `resource!` declarative macro, which provides the most concise syntax for creating MCP resources with inline content generation. It's perfect for simple resources that need dynamic content without the overhead of struct definitions.

## Features

### 🚀 **Declarative Resource Creation**
- **`resource!` macro** - Most concise resource definition syntax
- **Inline content generation** - Logic defined directly in the macro
- **Dynamic content** - Real-time data generation
- **Minimal boilerplate** - Fastest way to create simple resources

### 📦 **Resource Examples**
- **Configuration Resource** - JSON configuration with dynamic data
- **System Status Resource** - Real-time system information
- **Data Resource** - Dynamic data generation examples

## Quick Start

### 1. Start the Server

```bash
cargo run --bin resource-macro-example
```

The server will start on `http://127.0.0.1:8047/mcp`

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

#### Data Resource
```json
{
  "method": "resources/read",
  "params": {
    "uri": "data://sample"
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

Dynamic application configuration resource.

**Content Type:** `application/json`
**Features:**
- Dynamic configuration generation
- JSON content with pretty formatting
- Application metadata and feature flags

**Sample Content:**
```json
{
  "app_name": "Resource Macro Example",
  "version": "1.0.0",
  "debug": true,
  "features": ["resources", "declarative_macros"],
  "timestamp": "2025-01-01T12:00:00Z"
}
```

### 🖥️ `system://status`

Real-time system status information.

**Content Type:** `text/plain`
**Features:**
- Live system metrics
- Uptime tracking
- Memory and CPU information
- Dynamic content generation

**Sample Content:**
```
System Status: OK
Uptime: 1234 seconds
Memory: 512MB
CPU: 25%
Last Updated: 2025-01-01T12:00:00Z
```

### 📊 `data://sample`

Sample data resource with structured content.

**Content Type:** `application/json`
**Features:**
- Dynamic data generation
- Structured JSON content
- Real-time timestamps

**Sample Content:**
```json
{
  "data_type": "sample",
  "generated_at": "2025-01-01T12:00:00Z",
  "records": [
    {"id": 1, "value": "alpha"},
    {"id": 2, "value": "beta"},
    {"id": 3, "value": "gamma"}
  ],
  "metadata": {
    "count": 3,
    "source": "declarative_macro"
  }
}
```

## Declarative Macro Syntax

### Configuration Resource
```rust
let config_resource = resource! {
    uri: "file://config.json",
    name: "Application Configuration",
    description: "Main application configuration file",
    content: |_self| async move {
        let config = serde_json::json!({
            "app_name": "Resource Macro Example",
            "version": "1.0.0",
            "debug": true,
            "features": ["resources", "declarative_macros"],
            "timestamp": chrono::Utc::now().to_rfc3339()
        });
        Ok(vec![ResourceContent::blob(
            serde_json::to_string_pretty(&config).unwrap(),
            "application/json".to_string()
        )])
    }
};
```

### System Status Resource
```rust
let status_resource = resource! {
    uri: "system://status",
    name: "System Status",
    description: "Current system health and status information",
    content: |_self| async move {
        let uptime = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
            
        let status = format!(
            "System Status: OK\nUptime: {} seconds\nMemory: 512MB\nCPU: 25%\nLast Updated: {}",
            uptime % 3600,
            chrono::Utc::now().to_rfc3339()
        );
        Ok(vec![ResourceContent::text(status)])
    }
};
```

### Data Resource with Structured Content
```rust
let data_resource = resource! {
    uri: "data://sample",
    name: "Sample Data",
    description: "Dynamic sample data resource",
    content: |_self| async move {
        let data = serde_json::json!({
            "data_type": "sample",
            "generated_at": chrono::Utc::now().to_rfc3339(),
            "records": [
                {"id": 1, "value": "alpha"},
                {"id": 2, "value": "beta"},
                {"id": 3, "value": "gamma"}
            ],
            "metadata": {
                "count": 3,
                "source": "declarative_macro"
            }
        });
        Ok(vec![ResourceContent::blob(
            serde_json::to_string_pretty(&data).unwrap(),
            "application/json".to_string()
        )])
    }
};
```

## Macro Comparison

| Approach | Lines of Code | Complexity | Use Case |
|----------|---------------|------------|----------|
| Manual Implementation | ~40 lines | High | Complex resources, full control |
| Derive Macro | ~8 lines | Low | Structured resources, type safety |
| **Declarative Macro** | **~6 lines** | **Minimal** | **Simple resources, rapid prototyping** |

## Benefits of Declarative Resource Macros

### 🚀 **Ultra-Concise Syntax**
- **Shortest possible** resource definitions
- **Inline content generation** - no separate methods needed
- **No struct definitions** required
- **Self-contained** resource logic

### ⚡ **Rapid Development**
- **Fastest way** to create simple resources
- **Copy-paste friendly** for similar resources
- **Immediate functionality** - no boilerplate
- **Perfect for prototyping**

### 🎯 **Dynamic Content**
- **Real-time data generation**
- **Async content creation**
- **Multiple content types** support
- **Flexible content logic**

## Advanced Features

### Multiple Content Types
```rust
let multi_content_resource = resource! {
    uri: "data://multi-format",
    name: "Multi-Format Data",
    description: "Resource providing data in multiple formats",
    content: |_self| async move {
        let data = get_sample_data().await;
        Ok(vec![
            ResourceContent::blob(
                serde_json::to_string_pretty(&data).unwrap(),
                "application/json".to_string()
            ),
            ResourceContent::text(
                format!("CSV: {}", data_to_csv(&data))
            ),
            ResourceContent::blob(
                data_to_xml(&data),
                "application/xml".to_string()
            )
        ])
    }
};
```

### Conditional Content
```rust
let conditional_resource = resource! {
    uri: "api://conditional-data",
    name: "Conditional Data",
    description: "Resource that provides different content based on conditions",
    content: |_self| async move {
        let current_hour = chrono::Utc::now().hour();
        
        let content = if current_hour < 12 {
            "Morning data set"
        } else if current_hour < 18 {
            "Afternoon data set"
        } else {
            "Evening data set"
        };
        
        Ok(vec![ResourceContent::text(format!(
            "Time-based content: {} (hour: {})",
            content, current_hour
        ))])
    }
};
```

### External API Integration
```rust
let api_resource = resource! {
    uri: "api://external-service",
    name: "External API Data",
    description: "Proxy for external API data",
    content: |_self| async move {
        // Fetch data from external API
        let response = reqwest::get("https://api.example.com/data").await?;
        let data = response.text().await?;
        
        Ok(vec![ResourceContent::blob(
            data,
            "application/json".to_string()
        )])
    }
};
```

## Server Configuration

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Create resources using declarative macros
    let config_resource = resource! { /* definition */ };
    let status_resource = resource! { /* definition */ };
    let data_resource = resource! { /* definition */ };

    let server = McpServer::builder()
        .name("resource-macro-example")
        .version("1.0.0")
        .title("Declarative Resource Macro Example")
        .instructions("Demonstrates the most concise way to create MCP resources")
        .resource(config_resource)
        .resource(status_resource)
        .resource(data_resource)
        .bind_address("127.0.0.1:8047".parse()?)
        .build()?;

    server.run().await?;
    Ok(())
}
```

## Content Types Supported

The declarative macro supports all MCP content types:

```rust
// Text content
Ok(vec![ResourceContent::text("Plain text content")])

// JSON content  
Ok(vec![ResourceContent::blob(
    json_data,
    "application/json".to_string()
)])

// Binary content
Ok(vec![ResourceContent::blob(
    binary_data,
    "application/octet-stream".to_string()
)])

// Multiple content types
Ok(vec![
    ResourceContent::text("Text version"),
    ResourceContent::blob(json_data, "application/json".to_string()),
    ResourceContent::blob(csv_data, "text/csv".to_string())
])
```

## Error Handling

```rust
let error_handling_resource = resource! {
    uri: "data://with-validation",
    name: "Validated Data",
    description: "Resource with proper error handling",
    content: |_self| async move {
        match fetch_data().await {
            Ok(data) => Ok(vec![ResourceContent::text(data)]),
            Err(e) => Err(format!("Failed to fetch data: {}", e))
        }
    }
};
```

## Testing

```bash
# Start the server
cargo run --bin resource-macro-example &

# List all resources
curl -X POST http://127.0.0.1:8047/mcp \
  -H "Content-Type: application/json" \
  -d '{"method": "resources/list"}'

# Read configuration
curl -X POST http://127.0.0.1:8047/mcp \
  -H "Content-Type: application/json" \
  -d '{"method": "resources/read", "params": {"uri": "file://config.json"}}'

# Get system status
curl -X POST http://127.0.0.1:8047/mcp \
  -H "Content-Type: application/json" \
  -d '{"method": "resources/read", "params": {"uri": "system://status"}}'
```

## Use Cases

### 1. **Rapid Prototyping**
Perfect for quickly creating resources without ceremony.

### 2. **Configuration Endpoints**
Ideal for serving dynamic configuration data.

### 3. **Status Pages**
Great for system status and health check endpoints.

### 4. **API Proxying**
Quick way to proxy external APIs through MCP resources.

### 5. **Data Transformation**
Perfect for simple data transformation and formatting resources.

## Architecture

```
┌─────────────────────┐    ┌──────────────────────┐    ┌─────────────────────┐
│   MCP Client        │────│  Resource Macro      │────│  Content Generators │
│                     │    │  Example             │    │                     │
│ - Resource Lists    │    │ - config_resource    │    │ - JSON Generation   │
│ - Resource Reads    │    │ - status_resource    │    │ - System Status     │
│ - URI Navigation    │    │ - data_resource      │    │ - Dynamic Data      │
│ - Content Handling  │    │ - Declarative Macros │    │ - External APIs     │
└─────────────────────┘    └──────────────────────┘    └─────────────────────┘
```

This example demonstrates how declarative macros provide the most concise way to create MCP resources while maintaining full functionality for serving dynamic content through the MCP protocol.