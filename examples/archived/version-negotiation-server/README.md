# Version Negotiation Server Example

A comprehensive demonstration of **MCP protocol version negotiation** between client and server. This example shows how the MCP framework automatically negotiates the best compatible protocol version during the initialize handshake.

## Overview

The MCP framework supports multiple protocol versions and implements backward compatibility. This server demonstrates:

- **Automatic version negotiation** during client initialization
- **Session-aware version tracking** for tools and handlers
- **Capability adjustment** based on negotiated protocol version
- **Version-specific feature detection** and reporting

## Features

### 🔄 **Protocol Version Support**
- **2024-11-05** - Base MCP protocol
- **2025-03-26** - Added streamable HTTP/SSE support
- **2025-06-18** - Added _meta fields, progress tokens, cursors, and elicitation

### 🧪 **Demonstration Tools**
- **`version_info`** - Get negotiated version and session information
- **`test_version_negotiation`** - Test negotiation logic with different client versions

### ⚙️ **Negotiation Strategy**
1. **Accept client's requested version** if server supports it
2. **Use highest compatible version** between client and server
3. **Graceful fallback** with proper error handling

## Quick Start

### 1. Start the Server

```bash
cargo run -p version-negotiation-server
```

The server will start on `http://127.0.0.1:8049/mcp`

### 2. Test Version Negotiation

#### Initialize with Latest Version (2025-06-18)
```bash
curl -X POST http://127.0.0.1:8049/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "method": "initialize", 
    "params": {
      "protocol_version": "2025-06-18",
      "capabilities": {},
      "client_info": {
        "name": "test-client",
        "version": "1.0.0"
      }
    }
  }'
```

#### Initialize with Older Version (2024-11-05)
```bash
curl -X POST http://127.0.0.1:8049/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "method": "initialize", 
    "params": {
      "protocol_version": "2024-11-05",
      "capabilities": {},
      "client_info": {
        "name": "legacy-client",
        "version": "1.0.0"
      }
    }
  }'
```

### 3. Check Negotiated Version

```bash
curl -X POST http://127.0.0.1:8049/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "method": "tools/call",
    "params": {
      "name": "version_info",
      "arguments": {}
    }
  }'
```

## Version Negotiation Logic

### Server Support Matrix

| Client Requests | Server Response | Features Available |
|----------------|----------------|-------------------|
| `2025-06-18` | ✅ `2025-06-18` | All features (meta fields, progress, cursors, elicitation) |
| `2025-03-26` | ✅ `2025-03-26` | Streamable HTTP/SSE support |
| `2024-11-05` | ✅ `2024-11-05` | Base protocol only |
| `invalid` | ❌ Error | Version not supported |

### Implementation Details

```rust
fn negotiate_version(&self, client_version: &str) -> Result<McpVersion, String> {
    let requested_version = McpVersion::from_str(client_version)?;
    
    let supported_versions = vec![
        McpVersion::V2024_11_05,
        McpVersion::V2025_03_26, 
        McpVersion::V2025_06_18,
    ];
    
    // Strategy 1: Use client's version if supported
    if supported_versions.contains(&requested_version) {
        return Ok(requested_version);
    }
    
    // Strategy 2: Graceful fallback (implementation handles this)
    Err("Version not supported".to_string())
}
```

## Testing Tools

### Version Info Tool

Get detailed information about the negotiated version:

```json
{
  "method": "tools/call",
  "params": {
    "name": "version_info",
    "arguments": {}
  }
}
```

**Example Response:**
```
Protocol Version: 2025-06-18
Session ID: 550e8400-e29b-41d4-a716-446655440000

Supported Features: streamable-http, _meta-fields, progress-token, cursor, elicitation

Version Capabilities:
- Streamable HTTP: true
- Meta Fields: true  
- Progress & Cursor: true
- Elicitation: true
```

### Version Test Tool

Test negotiation logic with different client versions:

```json
{
  "method": "tools/call",
  "params": {
    "name": "test_version_negotiation",
    "arguments": {
      "client_version": "2024-11-05"
    }
  }
}
```

**Example Response:**
```
Version Negotiation Test
Client Requested: 2024-11-05
Server Response: ✅ Version 2024-11-05 accepted as requested
Server Supports: 2024-11-05, 2025-03-26, 2025-06-18
```

## Session Integration

The negotiated protocol version is automatically:

1. **Stored in session state** as `mcp_version`
2. **Available to all tools** through the session context
3. **Used for capability adjustment** in responses
4. **Logged for debugging** and monitoring

### Session State Access

```rust
async fn call(&self, _args: Value, session: Option<SessionContext>) -> Result<Vec<ToolResult>, String> {
    if let Some(ctx) = session {
        // Get negotiated version from session
        if let Some(version_info) = (ctx.get_state)("mcp_version").await {
            let version_str = version_info.as_str().unwrap_or("unknown");
            // Use version information...
        }
    }
    // ...
}
```

## Error Handling

### Unsupported Version

```bash
curl -X POST http://127.0.0.1:8049/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "method": "initialize",
    "params": {
      "protocol_version": "invalid-version",
      "capabilities": {},
      "client_info": {"name": "test", "version": "1.0"}
    }
  }'
```

**Response:**
```json
{
  "error": {
    "code": -32602,
    "message": "Version negotiation failed: Unsupported protocol version: invalid-version"
  }
}
```

## Real-world Applications

### 1. **Legacy Client Support**
Handle older MCP clients that only support basic protocol features.

### 2. **Feature Detection**
Dynamically enable/disable server features based on client capabilities.

### 3. **Graceful Upgrades**
Allow servers to add new protocol features without breaking existing clients.

### 4. **Client Compatibility Testing**
Test how different client implementations handle version negotiation.

## Architecture

```
┌─────────────────────┐    ┌──────────────────────┐    ┌─────────────────────┐
│   MCP Client        │────│  Initialize Handler  │────│  Session Manager    │
│                     │    │                      │    │                     │
│ - Requests Version  │    │ - Parses Client Ver  │    │ - Stores Negotiated │
│ - Sends Capabilities│    │ - Negotiates Version │    │ - Tracks Per-Session│
│ - Receives Response │    │ - Adjusts Capabilities│   │ - Provides Context  │
└─────────────────────┘    └──────────────────────┘    └─────────────────────┘
                                       │
                                       ▼
                           ┌──────────────────────┐
                           │  Protocol Version    │
                           │  Negotiation Logic   │
                           │                      │
                           │ - Version Parsing    │
                           │ - Compatibility Check│
                           │ - Feature Detection  │
                           └──────────────────────┘
```

## Protocol Evolution

### Version History

- **2024-11-05**: Initial MCP protocol specification
- **2025-03-26**: Added streamable HTTP transport and SSE notifications
- **2025-06-18**: Added _meta fields, progress tokens, cursors, and structured user elicitation

### Future Compatibility

The framework is designed to handle future protocol versions by:

1. **Adding new versions** to the `McpVersion` enum
2. **Updating capability detection** methods
3. **Implementing backward compatibility** rules
4. **Graceful feature degradation** for older clients

This version negotiation system ensures that MCP servers can evolve while maintaining compatibility with a diverse ecosystem of client implementations.