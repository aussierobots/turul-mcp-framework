# MCP 2025-06-18 Specification Compliant Server

A comprehensive example demonstrating **full compliance** with the MCP 2025-06-18 specification, including proper `_meta` field handling, progress tokens, structured JSON-RPC responses, and advanced protocol features.

## Overview

This server showcases all the advanced features introduced in the MCP 2025-06-18 specification, providing a reference implementation for building production-ready MCP servers that fully comply with the latest protocol standards.

## Features

### 🎯 **MCP 2025-06-18 Compliance**
- **Structured `_meta` fields** with progress tokens and pagination
- **Progress tracking** with real-time updates and status notifications
- **Proper JSON-RPC 2.0** response formatting with metadata
- **Enhanced error handling** with detailed error metadata
- **Protocol version negotiation** support

### 🔧 **Advanced Protocol Features**
- **Progress tokens** for long-running operation tracking
- **Cursor-based pagination** for large dataset handling
- **Comprehensive metadata** in all responses
- **Session-aware progress** notifications
- **Specification-compliant** response structures

## Quick Start

### 1. Start the Server

```bash
cargo run --bin spec-compliant-server
```

The server will start on `http://127.0.0.1:8043/mcp`

### 2. Test Specification Compliance

#### Data Processing with Progress Tracking
```json
{
  "method": "tools/call",
  "params": {
    "name": "process_data",
    "arguments": {
      "data": "Sample dataset",
      "steps": 5
    }
  }
}
```

#### Metadata Demonstration
```json
{
  "method": "tools/call", 
  "params": {
    "name": "metadata_demo",
    "arguments": {
      "include_progress": true,
      "include_pagination": true
    }
  }
}
```

#### Resource with Pagination
```json
{
  "method": "resources/read",
  "params": {
    "uri": "spec://large-dataset",
    "cursor": "page-2"
  }
}
```

## Tool Reference

### 🔄 `process_data`

Demonstrates progress tracking with proper `_meta` field usage.

**Parameters:**
- `data` (required): Data to process
- `steps` (required): Number of processing steps

**Features:**
- Real-time progress notifications
- Step-by-step processing simulation
- Proper metadata in responses
- Session-aware progress tracking

**MCP 2025-06-18 Response:**
```json
{
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Successfully processed 'Sample dataset' in 5 steps"
      }
    ],
    "_meta": {
      "progressToken": "task-12345",
      "progress": 1.0,
      "currentStep": 5,
      "totalSteps": 5,
      "estimatedRemainingSeconds": 0
    }
  }
}
```

### 📊 `metadata_demo`

Showcases various `_meta` field types and structures.

**Parameters:**
- `include_progress` (optional): Include progress metadata
- `include_pagination` (optional): Include pagination metadata
- `include_custom` (optional): Include custom metadata fields

**Features:**
- Comprehensive metadata examples
- Different `_meta` field configurations
- Progress token generation
- Custom metadata demonstration

**Sample Response:**
```json
{
  "result": {
    "content": [
      {
        "type": "text", 
        "text": "Metadata demonstration complete"
      }
    ],
    "_meta": {
      "progressToken": "demo-67890",
      "cursor": "demo-cursor-123",
      "total": 100,
      "hasMore": false,
      "progress": 1.0,
      "customField": "Specification compliant metadata"
    }
  }
}
```

## Resource Examples

### 📁 `spec://large-dataset`

Demonstrates paginated resource access with proper metadata.

**Features:**
- Cursor-based pagination
- Large dataset simulation
- Proper `_meta` fields in resource responses
- Navigation metadata

**Paginated Response:**
```json
{
  "content": [
    {
      "type": "text",
      "text": "Dataset page content..."
    }
  ],
  "_meta": {
    "cursor": "page-3",
    "total": 1000,
    "hasMore": true,
    "currentPageSize": 50
  }
}
```

### 🔧 `spec://system-status`

System status resource with real-time metadata.

**Features:**
- Dynamic status information
- Timestamp metadata
- System health indicators
- Performance metrics

## Specification Compliance Features

### Progress Token Management

```rust
use turul_mcp_protocol::{Meta, ProgressToken};

// Create progress token for long-running operation
let progress_token = ProgressToken::new("operation-12345");

// Add to response metadata
let meta = Meta::new()
    .set_progress_token(progress_token)
    .set_progress(0.5, Some(5), Some(10))
    .set_estimated_remaining(30.0);
```

### Structured Metadata

```rust
// Comprehensive metadata example
let meta = Meta::new()
    .set_progress_token("task-abc123")
    .set_cursor("page-5")
    .set_pagination(Some("next-page".into()), Some(500), true)
    .set_progress(0.75, Some(75), Some(100))
    .set_estimated_remaining(45.0)
    .add_extra("processingMode", "batch")
    .add_extra("quality", "high");
```

### JSON-RPC 2.0 Compliance

```rust
use turul_mcp_protocol::{JsonRpcResponse, ResultWithMeta};

// Create spec-compliant response
let response = JsonRpcResponse::success(
    ResultWithMeta::new(tool_results)
        .with_meta(meta),
    request_id
);
```

### Error Handling with Metadata

```rust
// Enhanced error responses
let error_meta = Meta::new()
    .add_extra("errorCode", "VALIDATION_FAILED")
    .add_extra("retryAfter", 30)
    .add_extra("supportContact", "support@example.com");

let error_response = JsonRpcResponse::error(
    JsonRpcError::new(-32602, "Invalid params")
        .with_data(error_meta),
    request_id
);
```

## Protocol Features Demonstrated

### 1. **Progress Tracking**
- Real-time progress updates
- Step-by-step operation tracking
- Estimated completion times
- Progress token management

### 2. **Pagination Support**
- Cursor-based navigation
- Total count information
- Has-more indicators
- Page size management

### 3. **Metadata Enhancement**
- Custom metadata fields
- Structured information
- Protocol compliance
- Client hint support

### 4. **Session Management**
- Session-aware operations
- Progress persistence
- Context preservation
- State management

## Server Configuration

```rust
use turul_mcp_protocol::PROTOCOL_VERSION;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = McpServer::builder()
        .name("spec-compliant-server")
        .version("1.0.0")
        .title("MCP 2025-06-18 Specification Compliant Server")
        .instructions("Demonstrates full compliance with MCP 2025-06-18 specification")
        .protocol_version(PROTOCOL_VERSION)
        .tool(ProcessDataTool {
            data: String::new(),
            steps: 0,
        })
        .tool(MetadataDemoTool {
            include_progress: false,
            include_pagination: false,
            include_custom: false,
        })
        .resource(spec_large_dataset_resource())
        .resource(spec_system_status_resource())
        .bind_address("127.0.0.1:8043".parse()?)
        .build()?;

    server.run().await?;
    Ok(())
}
```

## Testing Specification Compliance

### Progress Tracking Test
```bash
curl -X POST http://127.0.0.1:8043/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "process_data",
      "arguments": {"data": "test", "steps": 3}
    },
    "id": 1
  }'
```

### Metadata Validation Test
```bash
curl -X POST http://127.0.0.1:8043/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call", 
    "params": {
      "name": "metadata_demo",
      "arguments": {
        "include_progress": true,
        "include_pagination": true,
        "include_custom": true
      }
    },
    "id": 2
  }'
```

### Resource Pagination Test
```bash
curl -X POST http://127.0.0.1:8043/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "resources/read",
    "params": {
      "uri": "spec://large-dataset",
      "cursor": "page-1"
    },
    "id": 3
  }'
```

## Compliance Validation

### Required Response Structure
All responses include proper `_meta` fields:

```json
{
  "jsonrpc": "2.0",
  "result": {
    "content": [...],
    "_meta": {
      "progressToken": "...",
      "cursor": "...",
      "total": 123,
      "hasMore": true,
      "progress": 0.75,
      "currentStep": 75,
      "totalSteps": 100,
      "estimatedRemainingSeconds": 30.0
    }
  },
  "id": 1
}
```

### Progress Token Format
- Unique identifiers for tracking operations
- Persistent across multiple requests
- Used for operation resumption
- Compliance with token format requirements

### Metadata Standards
- Structured `_meta` field placement
- Consistent naming conventions
- Type safety and validation
- Optional field handling

## Use Cases

### 1. **Specification Reference**
Perfect reference implementation for MCP 2025-06-18 compliance requirements.

### 2. **Production Server Development**
Foundation for building production-ready MCP servers with full specification support.

### 3. **Protocol Testing**
Comprehensive test server for validating MCP client implementations.

### 4. **Feature Demonstration**
Showcases all advanced features available in the latest MCP specification.

### 5. **Migration Guide**
Example of upgrading existing MCP servers to 2025-06-18 specification compliance.

This server serves as the authoritative reference for implementing MCP 2025-06-18 specification-compliant servers and demonstrates best practices for modern MCP development.