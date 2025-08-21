//! # Roots Server Example
//!
//! This example demonstrates MCP roots functionality for discovering root directories
//! and file system access patterns. Roots allow clients to understand the file system
//! structure that an MCP server can access.

use std::collections::HashMap;
use async_trait::async_trait;
use mcp_server::{McpServer, McpTool};
use mcp_protocol::{ToolSchema, ToolResult, schema::JsonSchema, roots::Root, McpError, McpResult};
use serde_json::Value;
use tracing::info;

/// Tool to list all available roots
struct ListRootsTool;

#[async_trait]
impl McpTool for ListRootsTool {
    fn name(&self) -> &str {
        "list_roots"
    }

    fn description(&self) -> &str {
        "List all available root directories that the server can access"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema::object()
    }

    async fn call(
        &self,
        _args: Value,
        _session: Option<mcp_server::SessionContext>,
    ) -> McpResult<Vec<ToolResult>> {
        let results = vec![
            ToolResult::text("Available Root Directories:\n\
                • file:///workspace - Project workspace directory\n\
                • file:///data - Data storage directory\n\
                • file:///tmp - Temporary files directory\n\
                • file:///config - Configuration files directory\n\
                • file:///logs - Log files directory".to_string()),
            ToolResult::text("Root Directory Usage:\n\
                - Roots define the top-level directories accessible to MCP tools\n\
                - Each root has a URI (typically file:// scheme) and optional name\n\
                - Clients can discover available roots via roots/list endpoint\n\
                - File operations are restricted to paths within these roots".to_string()),
        ];

        Ok(results)
    }
}

/// Tool to inspect a specific root directory
struct InspectRootTool;

#[async_trait]
impl McpTool for InspectRootTool {
    fn name(&self) -> &str {
        "inspect_root"
    }

    fn description(&self) -> &str {
        "Inspect a specific root directory and show its properties"
    }

    fn input_schema(&self) -> ToolSchema {
        let mut properties = HashMap::new();
        properties.insert("root_uri".to_string(), JsonSchema::string());
        
        ToolSchema::object()
            .with_properties(properties)
            .with_required(vec!["root_uri".to_string()])
    }

    async fn call(
        &self,
        args: Value,
        _session: Option<mcp_server::SessionContext>,
    ) -> McpResult<Vec<ToolResult>> {
        let root_uri = args.get("root_uri")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("root_uri"))?;

        // Simulate root directory inspection
        let root_info = match root_uri {
            "file:///workspace" => {
                "Root: file:///workspace\n\
                Name: Project Workspace\n\
                Type: Development workspace\n\
                Access: Read/Write\n\
                Purpose: Source code, documentation, build artifacts\n\
                Typical Contents: src/, docs/, target/, Cargo.toml, README.md\n\
                Security: Sandboxed to project directory"
            },
            "file:///data" => {
                "Root: file:///data\n\
                Name: Data Storage\n\
                Type: Data directory\n\
                Access: Read/Write\n\
                Purpose: Application data, user files, databases\n\
                Typical Contents: *.db, *.json, *.csv, user-uploads/\n\
                Security: Isolated data storage"
            },
            "file:///tmp" => {
                "Root: file:///tmp\n\
                Name: Temporary Files\n\
                Type: Temporary storage\n\
                Access: Read/Write (auto-cleanup)\n\
                Purpose: Temporary files, cache, processing\n\
                Typical Contents: temp-*, cache/, processing/\n\
                Security: Automatically cleaned up"
            },
            "file:///config" => {
                "Root: file:///config\n\
                Name: Configuration Files\n\
                Type: Configuration directory\n\
                Access: Read-only\n\
                Purpose: Application configuration, settings\n\
                Typical Contents: config.json, settings.toml, env/\n\
                Security: Read-only access to prevent accidental changes"
            },
            "file:///logs" => {
                "Root: file:///logs\n\
                Name: Log Files\n\
                Type: Logging directory\n\
                Access: Read-only\n\
                Purpose: Application logs, audit trails\n\
                Typical Contents: app.log, error.log, access.log\n\
                Security: Read-only, log rotation enabled"
            },
            _ => return Err(McpError::tool_execution(&format!("Unknown root URI: {}", root_uri))),
        };

        let results = vec![ToolResult::text(root_info.to_string())];
        Ok(results)
    }
}

/// Tool to simulate file operations within roots
struct FileOperationTool;

#[async_trait]
impl McpTool for FileOperationTool {
    fn name(&self) -> &str {
        "simulate_file_operation"
    }

    fn description(&self) -> &str {
        "Simulate file operations within root directories (read, write, list)"
    }

    fn input_schema(&self) -> ToolSchema {
        let mut properties = HashMap::new();
        properties.insert("operation".to_string(), JsonSchema::string());
        properties.insert("path".to_string(), JsonSchema::string());
        
        ToolSchema::object()
            .with_properties(properties)
            .with_required(vec!["operation".to_string(), "path".to_string()])
    }

    async fn call(
        &self,
        args: Value,
        _session: Option<mcp_server::SessionContext>,
    ) -> McpResult<Vec<ToolResult>> {
        let operation = args.get("operation")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("operation"))?;
            
        let path = args.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("path"))?;

        // Validate path is within allowed roots
        let allowed_roots = [
            "file:///workspace",
            "file:///data", 
            "file:///tmp",
            "file:///config",
            "file:///logs"
        ];

        let is_allowed = allowed_roots.iter().any(|root| path.starts_with(root));
        if !is_allowed {
            return Err(McpError::ResourceAccessDenied(format!("Path '{}' is outside allowed root directories", path)));
        }

        let result = match operation {
            "read" => {
                format!("✅ READ OPERATION\n\
                    Path: {}\n\
                    Status: Success\n\
                    Content: [Simulated file content would appear here]\n\
                    Size: 1,234 bytes\n\
                    Modified: 2024-12-18 10:30:00 UTC\n\
                    Permissions: -rw-r--r--", path)
            },
            "write" => {
                if path.starts_with("file:///config") || path.starts_with("file:///logs") {
                    return Err(McpError::ResourceAccessDenied("Write operation not allowed on read-only root".to_string()));
                }
                format!("✅ WRITE OPERATION\n\
                    Path: {}\n\
                    Status: Success\n\
                    Action: File created/updated\n\
                    Size: 567 bytes written\n\
                    Backup: Previous version backed up\n\
                    Permissions: -rw-r--r--", path)
            },
            "list" => {
                format!("✅ LIST OPERATION\n\
                    Directory: {}\n\
                    Status: Success\n\
                    Contents:\n\
                    • file1.txt (1.2 KB, 2024-12-18)\n\
                    • file2.json (3.4 KB, 2024-12-17)\n\
                    • subdirectory/ (directory, 2024-12-16)\n\
                    • .hidden_file (0.5 KB, 2024-12-15)\n\
                    Total: 4 items", path)
            },
            "delete" => {
                if path.starts_with("file:///config") || path.starts_with("file:///logs") {
                    return Err(McpError::ResourceAccessDenied("Delete operation not allowed on read-only root".to_string()));
                }
                format!("✅ DELETE OPERATION\n\
                    Path: {}\n\
                    Status: Success\n\
                    Action: File moved to trash\n\
                    Backup: Available for 30 days\n\
                    Recovery: Use 'restore' operation if needed", path)
            },
            _ => return Err(McpError::tool_execution(&format!("Unknown operation: {}", operation))),
        };

        let results = vec![ToolResult::text(result)];
        Ok(results)
    }
}

/// Tool to demonstrate root security and permissions
struct RootSecurityTool;

#[async_trait]
impl McpTool for RootSecurityTool {
    fn name(&self) -> &str {
        "demonstrate_root_security"
    }

    fn description(&self) -> &str {
        "Demonstrate how root directories provide security boundaries for file operations"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema::object()
    }

    async fn call(
        &self,
        _args: Value,
        _session: Option<mcp_server::SessionContext>,
    ) -> McpResult<Vec<ToolResult>> {
        let results = vec![
            ToolResult::text("🔐 ROOT DIRECTORY SECURITY\n\
                \n\
                Roots provide security boundaries for MCP servers:\n\
                \n\
                1. PATH RESTRICTION\n\
                   • File operations limited to defined root directories\n\
                   • Prevents access to system files outside scope\n\
                   • Blocks directory traversal attacks (../ patterns)\n\
                \n\
                2. PERMISSION CONTROL\n\
                   • Each root can have different access levels\n\
                   • Read-only roots: /config, /logs\n\
                   • Read-write roots: /workspace, /data, /tmp\n\
                \n\
                3. SANDBOXING\n\
                   • MCP server operates in isolated environment\n\
                   • Cannot access files outside defined roots\n\
                   • Prevents unauthorized system access".to_string()),
            
            ToolResult::text("🛡️ SECURITY EXAMPLES\n\
                \n\
                ALLOWED OPERATIONS:\n\
                ✅ file:///workspace/src/main.rs (read/write)\n\
                ✅ file:///data/user_uploads/doc.pdf (read/write)\n\
                ✅ file:///config/settings.json (read-only)\n\
                \n\
                BLOCKED OPERATIONS:\n\
                ❌ file:///etc/passwd (outside roots)\n\
                ❌ file:///../../../system/file (traversal attack)\n\
                ❌ file:///config/secret.key (read-only violation)\n\
                \n\
                AUTOMATIC PROTECTIONS:\n\
                🔒 Path validation before operation\n\
                🔒 Permission checking per root\n\
                🔒 Sandboxed execution environment".to_string()),
        ];

        Ok(results)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("📁 Starting MCP Roots Server Example");

    // Create root directory definitions
    let workspace_root = Root::new("file:///workspace")
        .with_name("Project Workspace");
    let data_root = Root::new("file:///data")
        .with_name("Data Storage");
    let tmp_root = Root::new("file:///tmp")
        .with_name("Temporary Files");
    let config_root = Root::new("file:///config")
        .with_name("Configuration Files");
    let logs_root = Root::new("file:///logs")
        .with_name("Log Files");

    let server = McpServer::builder()
        .name("roots-server")
        .version("1.0.0")
        .title("MCP Roots Server Example")
        .instructions("This server demonstrates MCP roots functionality for file system access control and directory discovery. Roots define the boundaries of file operations and provide security isolation.")
        .tool(ListRootsTool)
        .tool(InspectRootTool)
        .tool(FileOperationTool)
        .tool(RootSecurityTool)
        // Add root directories the server can access
        .root(workspace_root)
        .root(data_root)
        .root(tmp_root)
        .root(config_root)
        .root(logs_root)
        .bind_address("127.0.0.1:8050".parse()?)
        .build()?;

    info!("🚀 Roots server running at: http://127.0.0.1:8050/mcp");
    info!("");
    info!("📋 Features demonstrated:");
    info!("  📁 Root directory discovery via roots/list endpoint");
    info!("  🔐 File system access control and security boundaries");
    info!("  🛡️ Permission-based access (read-only vs read-write)");
    info!("  📂 Simulated file operations within root constraints");
    info!("");
    info!("🔧 Available tools:");
    info!("  📋 list_roots - Show all available root directories");
    info!("  🔍 inspect_root - Examine specific root directory properties");
    info!("  📝 simulate_file_operation - Test file operations with security");
    info!("  🛡️ demonstrate_root_security - Show security features");
    info!("");
    info!("📁 Configured root directories:");
    info!("  • file:///workspace - Project Workspace (RW)");
    info!("  • file:///data - Data Storage (RW)");
    info!("  • file:///tmp - Temporary Files (RW, auto-cleanup)");
    info!("  • file:///config - Configuration Files (RO)");
    info!("  • file:///logs - Log Files (RO)");
    info!("");
    info!("🧪 Test roots discovery:");
    info!("  curl -X POST http://127.0.0.1:8050/mcp \\");
    info!("    -H 'Content-Type: application/json' \\");
    info!("    -d '{{\"method\": \"roots/list\"}}'");

    server.run().await?;
    Ok(())
}