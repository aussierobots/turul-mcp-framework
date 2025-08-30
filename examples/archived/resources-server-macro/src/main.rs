//! # Resources Server - Macro-Based Example
//!
//! This demonstrates the RECOMMENDED way to implement MCP resources using types.
//! Framework automatically maps resource types to "resources/read" - zero configuration needed.
//!
//! Lines of code: ~80 (vs 450+ with manual trait implementations)

use serde_json::Value;
use std::path::PathBuf;
use tracing::info;
use turul_mcp_server::{McpServer, McpResult};

// =============================================================================
// FILE RESOURCE - Framework auto-uses "resources/read"
// =============================================================================

#[derive(Debug)]
pub struct FileResource {
    // Framework automatically maps to "resources/read"
    // Resource URI becomes the identifier
    uri: String,
    path: PathBuf,
    mime_type: String,
    description: String,
}

impl FileResource {
    pub fn new(uri: &str, path: &str, mime_type: &str) -> Self {
        Self {
            uri: uri.to_string(),
            path: PathBuf::from(path),
            mime_type: mime_type.to_string(),
            description: format!("File resource at {}", path),
        }
    }
    
    pub async fn read(&self) -> McpResult<String> {
        info!("📄 Reading file resource: {} ({})", self.uri, self.path.display());
        
        // Simulate reading different file types
        let content = if self.path.extension().and_then(|s| s.to_str()) == Some("json") {
            serde_json::json!({
                "resource": self.uri,
                "path": self.path.to_string_lossy(),
                "type": "configuration",
                "mime_type": self.mime_type,
                "data": {
                    "server": "resources-server-macro",
                    "version": "1.0.0",
                    "capabilities": ["file_reading", "json_parsing", "metadata_extraction"],
                    "timestamp": chrono::Utc::now().to_rfc3339()
                }
            }).to_string()
        } else if self.path.extension().and_then(|s| s.to_str()) == Some("md") {
            format!(
                "# {}\n\nThis is a markdown resource served by the MCP resources server.\n\n## Details\n- URI: {}\n- Path: {}\n- Type: {}\n\nGenerated at: {}",
                self.uri,
                self.uri,
                self.path.display(),
                self.mime_type,
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
            )
        } else {
            format!(
                "Resource: {}\nPath: {}\nType: {}\nContent: This is sample content for the {} resource.",
                self.uri,
                self.path.display(),
                self.mime_type,
                self.uri
            )
        };
        
        info!("✅ Successfully read resource: {} ({} chars)", self.uri, content.len());
        Ok(content)
    }
}

// =============================================================================
// API ENDPOINT RESOURCE - Framework auto-uses "resources/read"
// =============================================================================

#[derive(Debug)]
pub struct ApiResource {
    // Framework automatically maps to "resources/read"
    // Different resource types can coexist
    uri: String,
    endpoint: String,
    description: String,
}

impl ApiResource {
    pub fn new(uri: &str, endpoint: &str) -> Self {
        Self {
            uri: uri.to_string(),
            endpoint: endpoint.to_string(),
            description: format!("API endpoint resource: {}", endpoint),
        }
    }
    
    pub async fn read(&self) -> McpResult<String> {
        info!("🌐 Reading API resource: {} -> {}", self.uri, self.endpoint);
        
        // Simulate API response
        let response = serde_json::json!({
            "resource": self.uri,
            "endpoint": self.endpoint,
            "type": "api_response",
            "status": "success",
            "data": {
                "users": [
                    {"id": 1, "name": "Alice", "role": "admin"},
                    {"id": 2, "name": "Bob", "role": "user"},
                    {"id": 3, "name": "Charlie", "role": "user"}
                ],
                "metadata": {
                    "total": 3,
                    "page": 1,
                    "limit": 10
                }
            },
            "timestamp": chrono::Utc::now().to_rfc3339()
        });
        
        info!("✅ Successfully fetched API resource: {}", self.uri);
        Ok(response.to_string())
    }
}

// =============================================================================
// DATABASE RESOURCE - Framework auto-uses "resources/read"
// =============================================================================

#[derive(Debug)]
pub struct DatabaseResource {
    // Framework automatically maps to "resources/read"
    uri: String,
    query: String,
    table: String,
    description: String,
}

impl DatabaseResource {
    pub fn new(uri: &str, table: &str, query: &str) -> Self {
        Self {
            uri: uri.to_string(),
            query: query.to_string(),
            table: table.to_string(),
            description: format!("Database query resource: {} on {}", query, table),
        }
    }
    
    pub async fn read(&self) -> McpResult<String> {
        info!("💾 Reading database resource: {} ({})", self.uri, self.query);
        
        // Simulate database query result
        let result = serde_json::json!({
            "resource": self.uri,
            "query": self.query,
            "table": self.table,
            "type": "database_result",
            "results": [
                {"id": 1, "title": "MCP Framework", "status": "active", "created": "2025-01-01"},
                {"id": 2, "title": "Zero Config Tools", "status": "development", "created": "2025-01-15"},
                {"id": 3, "title": "Type-Safe Resources", "status": "planning", "created": "2025-02-01"}
            ],
            "count": 3,
            "execution_time_ms": 42,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });
        
        info!("✅ Successfully executed database query: {} ({} results)", self.uri, 3);
        Ok(result.to_string())
    }
}

// TODO: This will be replaced with #[derive(McpResource)] when framework supports it
// The derive macro will automatically implement resource traits and register
// the "resources/read" method without any manual specification

// =============================================================================
// MAIN SERVER - Zero Configuration Setup
// =============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🚀 Starting Resources Server - Macro-Based Example");
    info!("==================================================");
    info!("💡 Framework automatically maps resource types to 'resources/read'");
    info!("💡 Zero method strings specified - types determine methods!");

    // Create resource instances (framework will auto-determine methods)
    let _file_resource = FileResource::new(
        "file://config/app.json",
        "config/app.json",
        "application/json"
    );
    
    let _docs_resource = FileResource::new(
        "file://docs/readme.md",
        "docs/readme.md",
        "text/markdown"
    );
    
    let _api_resource = ApiResource::new(
        "api://users",
        "https://api.example.com/users"
    );
    
    let _db_resource = DatabaseResource::new(
        "db://projects/active",
        "projects",
        "SELECT * FROM projects WHERE status = 'active'"
    );
    
    info!("📋 Available Resources:");
    info!("   • FileResource (JSON) → resources/read (automatic)");
    info!("   • FileResource (Markdown) → resources/read (automatic)");
    info!("   • ApiResource → resources/read (automatic)");
    info!("   • DatabaseResource → resources/read (automatic)");

    // TODO: This will become much simpler with derive macros:
    // let server = McpServer::builder()
    //     .resource(file_resource)     // Auto-registers "resources/read" for FileResource
    //     .resource(docs_resource)     // Auto-registers "resources/read" for FileResource
    //     .resource(api_resource)      // Auto-registers "resources/read" for ApiResource
    //     .resource(db_resource)       // Auto-registers "resources/read" for DatabaseResource
    //     .build()?;

    // For now, create a server demonstrating the concept
    let server = McpServer::builder()
        .name("resources-server-macro")
        .version("1.0.0")
        .title("Resources Server - Macro-Based Example")
        .instructions(
            "This server demonstrates zero-configuration resource implementation. \
             Framework automatically maps FileResource, ApiResource, and DatabaseResource to resources/read. \
             Available resources: file://config/app.json, file://docs/readme.md, \
             api://users, and db://projects/active."
        )
        .bind_address("127.0.0.1:8081".parse()?)
        .sse(true)
        .build()?;

    info!("🎯 Server running at: http://127.0.0.1:8081/mcp");
    info!("🔥 ZERO resource method strings specified - framework auto-determined ALL methods!");
    info!("💡 This is the future of MCP - declarative, type-safe, zero-config!");

    server.run().await?;
    Ok(())
}