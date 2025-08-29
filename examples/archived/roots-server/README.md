# Roots Server Example

A comprehensive demonstration of **MCP roots functionality** for discovering root directories and implementing file system access control. This example shows how MCP servers define secure boundaries for file operations using root directory definitions.

## Overview

MCP roots define the top-level directories that an MCP server can access, providing crucial security boundaries and file system organization. This server demonstrates:

- **Root directory discovery** via `roots/list` endpoint
- **File system security boundaries** and access control
- **Permission-based access** (read-only vs read-write roots)
- **Simulated file operations** within root constraints

## Features

### 📁 **Root Directory Management**
- **5 configured root directories** with different access levels
- **Automatic root registration** via builder pattern
- **Security boundary enforcement** for all file operations
- **Dynamic root discovery** for MCP clients

### 🔐 **Security & Access Control**
- **Read-write roots**: `/workspace`, `/data`, `/tmp`
- **Read-only roots**: `/config`, `/logs` 
- **Path validation** prevents directory traversal attacks
- **Operation-level permissions** enforce security policies

### 🧪 **Demonstration Tools**
- **`list_roots`** - Show all available root directories
- **`inspect_root`** - Examine specific root properties
- **`simulate_file_operation`** - Test file operations with security
- **`demonstrate_root_security`** - Show security features

## Quick Start

### 1. Start the Server

```bash
cargo run -p roots-server
```

The server will start on `http://127.0.0.1:8050/mcp`

### 2. Discover Root Directories

```bash
curl -X POST http://127.0.0.1:8050/mcp \
  -H "Content-Type: application/json" \
  -d '{"method": "roots/list"}'
```

**Example Response:**
```json
{
  "result": {
    "roots": [
      {
        "uri": "file:///workspace",
        "name": "Project Workspace"
      },
      {
        "uri": "file:///data", 
        "name": "Data Storage"
      },
      {
        "uri": "file:///tmp",
        "name": "Temporary Files"
      },
      {
        "uri": "file:///config",
        "name": "Configuration Files"
      },
      {
        "uri": "file:///logs",
        "name": "Log Files"
      }
    ]
  }
}
```

## Root Directory Configuration

### 📂 **Project Workspace** - `file:///workspace`
- **Access**: Read/Write
- **Purpose**: Source code, documentation, build artifacts
- **Contents**: `src/`, `docs/`, `target/`, `Cargo.toml`, `README.md`
- **Security**: Sandboxed to project directory

### 💾 **Data Storage** - `file:///data`
- **Access**: Read/Write  
- **Purpose**: Application data, user files, databases
- **Contents**: `*.db`, `*.json`, `*.csv`, `user-uploads/`
- **Security**: Isolated data storage

### 🗂️ **Temporary Files** - `file:///tmp`
- **Access**: Read/Write (auto-cleanup)
- **Purpose**: Temporary files, cache, processing
- **Contents**: `temp-*`, `cache/`, `processing/`
- **Security**: Automatically cleaned up

### ⚙️ **Configuration Files** - `file:///config` 
- **Access**: Read-only
- **Purpose**: Application configuration, settings
- **Contents**: `config.json`, `settings.toml`, `env/`
- **Security**: Read-only to prevent accidental changes

### 📜 **Log Files** - `file:///logs`
- **Access**: Read-only
- **Purpose**: Application logs, audit trails
- **Contents**: `app.log`, `error.log`, `access.log`
- **Security**: Read-only, log rotation enabled

## Tool Demonstrations

### 1. List All Roots

```bash
curl -X POST http://127.0.0.1:8050/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "method": "tools/call",
    "params": {
      "name": "list_roots",
      "arguments": {}
    }
  }'
```

### 2. Inspect Specific Root

```bash
curl -X POST http://127.0.0.1:8050/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "method": "tools/call",
    "params": {
      "name": "inspect_root",
      "arguments": {
        "root_uri": "file:///workspace"
      }
    }
  }'
```

### 3. Simulate File Operations

```bash
# Test read operation
curl -X POST http://127.0.0.1:8050/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "method": "tools/call",
    "params": {
      "name": "simulate_file_operation",
      "arguments": {
        "operation": "read",
        "path": "file:///workspace/src/main.rs"
      }
    }
  }'

# Test write operation (will fail on read-only roots)
curl -X POST http://127.0.0.1:8050/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "method": "tools/call", 
    "params": {
      "name": "simulate_file_operation",
      "arguments": {
        "operation": "write",
        "path": "file:///config/secret.conf"
      }
    }
  }'
```

### 4. Security Demonstration

```bash
curl -X POST http://127.0.0.1:8050/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "method": "tools/call",
    "params": {
      "name": "demonstrate_root_security",
      "arguments": {}
    }
  }'
```

## Security Features

### 🛡️ **Path Validation**

```
✅ ALLOWED OPERATIONS:
- file:///workspace/src/main.rs (within workspace root)
- file:///data/user_uploads/doc.pdf (within data root) 
- file:///config/settings.json (read-only access)

❌ BLOCKED OPERATIONS:
- file:///etc/passwd (outside defined roots)
- file:///../../../system/file (directory traversal attack)
- Write to file:///config/* (read-only violation)
```

### 🔒 **Permission Enforcement**

| Root Directory | Read | Write | Delete | Notes |
|---------------|------|-------|---------|--------|
| `/workspace` | ✅ | ✅ | ✅ | Full access |
| `/data` | ✅ | ✅ | ✅ | Full access |
| `/tmp` | ✅ | ✅ | ✅ | Auto-cleanup |
| `/config` | ✅ | ❌ | ❌ | Read-only |
| `/logs` | ✅ | ❌ | ❌ | Read-only |

### 🚨 **Attack Prevention**

1. **Directory Traversal**: Blocks `../` patterns and paths outside roots
2. **Unauthorized Access**: Prevents access to system files
3. **Permission Violations**: Enforces read-only restrictions
4. **Sandbox Isolation**: Operations confined to defined boundaries

## Implementation Details

### Root Registration

```rust
let server = McpServer::builder()
    .name("roots-server")
    .version("1.0.0")
    // Individual root registration
    .root(Root::new("file:///workspace").with_name("Project Workspace"))
    .root(Root::new("file:///data").with_name("Data Storage"))
    .root(Root::new("file:///tmp").with_name("Temporary Files"))
    .root(Root::new("file:///config").with_name("Configuration Files"))
    .root(Root::new("file:///logs").with_name("Log Files"))
    .build()?;
```

### Security Validation

```rust
async fn simulate_file_operation(&self, args: Value) -> Result<Vec<ToolResult>, String> {
    let operation = args.get("operation").and_then(|v| v.as_str())?;
    let path = args.get("path").and_then(|v| v.as_str())?;

    // Validate path is within allowed roots
    let allowed_roots = [
        "file:///workspace", "file:///data", "file:///tmp",
        "file:///config", "file:///logs"
    ];
    
    let is_allowed = allowed_roots.iter().any(|root| path.starts_with(root));
    if !is_allowed {
        return Err(format!("Path '{}' is outside allowed root directories", path));
    }

    // Check permission for operation
    if operation == "write" && (path.starts_with("file:///config") || path.starts_with("file:///logs")) {
        return Err("Write operation not allowed on read-only root".to_string());
    }
    
    // Process operation...
}
```

## Real-world Applications

### 1. **Development Environments**
- Define project boundaries for code editors and IDEs
- Restrict file operations to workspace directories
- Separate configuration from working files

### 2. **Content Management Systems**
- Isolate user content from system files
- Implement tiered access (public, private, admin)
- Enforce upload restrictions by directory

### 3. **Build & CI Systems**
- Restrict build processes to designated directories
- Separate source, build, and artifact directories
- Prevent accidental system file access

### 4. **Data Processing Pipelines**
- Define input, processing, and output directories
- Enforce data governance boundaries
- Implement retention policies per directory

### 5. **Security-Critical Applications**
- Implement principle of least privilege
- Audit file access patterns
- Prevent privilege escalation attacks

## Architecture

```
┌─────────────────────┐    ┌──────────────────────┐    ┌─────────────────────┐
│   MCP Client        │────│  Roots Handler       │────│  Root Definitions   │
│                     │    │                      │    │                     │
│ - Discover Roots    │    │ - Root Registration  │    │ - /workspace (RW)   │
│ - Request Files     │    │ - Path Validation    │    │ - /data (RW)        │
│ - File Operations   │    │ - Permission Check   │    │ - /tmp (RW)         │
│                     │    │ - Security Enforce   │    │ - /config (RO)      │
│                     │    │                      │    │ - /logs (RO)        │
└─────────────────────┘    └──────────────────────┘    └─────────────────────┘
                                       │
                                       ▼
                           ┌──────────────────────┐
                           │  File System         │
                           │  Security Layer      │
                           │                      │
                           │ - Sandboxing         │
                           │ - Access Control     │
                           │ - Audit Logging      │
                           └──────────────────────┘
```

## Best Practices

### 🔐 **Security**
1. **Minimal Roots**: Only define necessary root directories
2. **Principle of Least Privilege**: Use read-only where possible
3. **Path Validation**: Always validate paths against allowed roots
4. **Regular Audits**: Monitor and log file access patterns

### 📁 **Organization**
1. **Logical Separation**: Group related files in appropriate roots
2. **Clear Naming**: Use descriptive root names and URIs
3. **Consistent Permissions**: Apply consistent access patterns
4. **Documentation**: Document root purposes and restrictions

### 🚀 **Performance**
1. **Efficient Validation**: Cache root patterns for fast path checking
2. **Lazy Loading**: Load root contents on-demand
3. **Batch Operations**: Process multiple files efficiently
4. **Resource Cleanup**: Implement automatic cleanup for temp directories

## Testing

```bash
# Start the server
cargo run -p roots-server &

# Test root discovery
curl -X POST http://127.0.0.1:8050/mcp \
  -H "Content-Type: application/json" \
  -d '{"method": "roots/list"}'

# Test security boundaries
curl -X POST http://127.0.0.1:8050/mcp \
  -H "Content-Type: application/json" \
  -d '{"method": "tools/call", "params": {"name": "simulate_file_operation", "arguments": {"operation": "read", "path": "file:///etc/passwd"}}}'

# Should return error: "Path outside allowed root directories"
```

This roots server example demonstrates how MCP provides secure, organized file system access through well-defined root directories, enabling safe file operations while maintaining strict security boundaries.