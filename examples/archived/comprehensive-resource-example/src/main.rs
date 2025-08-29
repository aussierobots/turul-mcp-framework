//! # Comprehensive Resource Example
//!
//! This example demonstrates all three approaches for creating MCP resources:
//! 1. Derive macros (#[derive(McpResource)])
//! 2. Declarative macros (resource!{})
//! 3. Manual trait implementation (impl McpResource)
//!
//! It also shows different resource types, parameter handling, and advanced features.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use mcp_derive::{McpResource, resource};
use mcp_server::{McpServer, McpResource};
use mcp_protocol::resources::ResourceContent;
use serde_json::{json, Value};
use std::collections::HashMap;

// =============================================================================
// APPROACH 1: DERIVE MACROS - #[derive(McpResource)]
// =============================================================================

/// Simple configuration file resource using derive macro
#[derive(McpResource, Serialize, Deserialize, Clone)]
#[uri = "file://config.json"]
#[name = "Application Configuration"]
#[description = "Main application configuration with JSON content"]
struct ConfigResource {
    #[content]
    #[content_type = "application/json"]
    pub config_data: String,
}

impl ConfigResource {
    fn new() -> Self {
        let config = json!({
            "app_name": "Comprehensive Resource Example",
            "version": "1.0.0",
            "debug": true,
            "database": {
                "host": "localhost",
                "port": 5432,
                "name": "example_db"
            },
            "features": ["resources", "derive_macros", "json_config"],
            "created_at": "2024-08-24T05:00:00Z"
        });
        
        Self {
            config_data: serde_json::to_string_pretty(&config).unwrap(),
        }
    }
}

/// Multi-content resource with different MIME types
#[derive(McpResource, Serialize, Deserialize, Clone)]
#[uri = "data://user-profile"]
#[name = "User Profile Data"]
#[description = "User profile with JSON data and plain text bio"]
struct UserProfileResource {
    #[content]
    #[content_type = "application/json"]
    pub profile_data: String,
    
    #[content]
    #[content_type = "text/plain"]
    pub bio: String,
    
    #[content]
    #[content_type = "text/csv"]
    pub activity_log: String,
    
    pub user_id: u64, // Not marked as content, won't be included
}

impl UserProfileResource {
    fn new(user_id: u64) -> Self {
        let profile = json!({
            "user_id": user_id,
            "username": format!("user_{}", user_id),
            "email": format!("user{}@example.com", user_id),
            "created_at": "2024-01-01T00:00:00Z",
            "preferences": {
                "theme": "dark",
                "language": "en",
                "notifications": true
            },
            "stats": {
                "login_count": 42,
                "last_login": "2024-08-24T04:00:00Z"
            }
        });
        
        let activity_log = vec![
            "timestamp,action,details",
            "2024-08-24T04:00:00Z,login,successful",
            "2024-08-24T04:05:00Z,view_profile,accessed",
            "2024-08-24T04:10:00Z,update_settings,theme_changed",
            "2024-08-24T04:15:00Z,logout,session_ended"
        ].join("\n");
        
        Self {
            profile_data: serde_json::to_string_pretty(&profile).unwrap(),
            bio: format!("This is user {} - a demo account created for testing the comprehensive MCP resource functionality. This user has been active since 2024.", user_id),
            activity_log,
            user_id,
        }
    }
}

/// Unit struct resource for system status
#[derive(McpResource)]
#[uri = "system://status"]
#[name = "System Status"]
#[description = "Real-time system health and performance metrics"]
struct SystemStatusResource;

// =============================================================================
// APPROACH 2: DECLARATIVE MACROS - resource!{}
// =============================================================================

// Note: These will be created in main() function since resource!{} creates instances

// =============================================================================
// APPROACH 3: MANUAL IMPLEMENTATION - impl McpResource
// =============================================================================

/// File system resource with parameter support
#[derive(Clone)]
struct FileSystemResource {
    base_path: String,
    allowed_extensions: Vec<String>,
}

impl FileSystemResource {
    fn new(base_path: String) -> Self {
        Self {
            base_path,
            allowed_extensions: vec!["txt".to_string(), "md".to_string(), "json".to_string(), "log".to_string()],
        }
    }
}

#[async_trait]
impl McpResource for FileSystemResource {
    fn uri(&self) -> &str {
        "file://filesystem"
    }

    fn name(&self) -> &str {
        "File System Access"
    }

    fn description(&self) -> &str {
        "Read files from the file system with path parameter support"
    }

    fn mime_type(&self) -> Option<&str> {
        Some("text/plain")
    }

    fn annotations(&self) -> Option<Value> {
        Some(json!({
            "requires_params": true,
            "param_schema": {
                "path": {
                    "type": "string",
                    "description": "Relative file path to read",
                    "required": true
                }
            },
            "allowed_extensions": self.allowed_extensions,
            "base_path": self.base_path
        }))
    }

    async fn read(&self, params: Option<Value>) -> mcp_server::McpResult<Vec<ResourceContent>> {
        let file_path = params
            .as_ref()
            .and_then(|p| p.get("path"))
            .and_then(|p| p.as_str())
            .ok_or_else(|| mcp_protocol::McpError::missing_param("path"))?;

        // Validate file extension
        let extension = std::path::Path::new(file_path)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");
            
        if !self.allowed_extensions.contains(&extension.to_string()) {
            return Err(mcp_protocol::McpError::param_out_of_range(
                "path",
                file_path,
                &format!("File extension must be one of: {}", self.allowed_extensions.join(", "))
            ));
        }

        // Security check: prevent path traversal
        if file_path.contains("..") || file_path.starts_with('/') {
            return Err(mcp_protocol::McpError::param_out_of_range(
                "path",
                file_path,
                "Path must be relative and cannot contain '..'"
            ));
        }

        // Simulate file reading (in a real implementation, you'd read from actual filesystem)
        let full_path = format!("{}/{}", self.base_path, file_path);
        let content = match extension {
            "json" => {
                let mock_data = json!({
                    "file_path": full_path,
                    "file_type": "json",
                    "content": {
                        "message": "This is mock JSON content",
                        "timestamp": "2024-08-24T05:00:00Z"
                    }
                });
                ResourceContent::blob(serde_json::to_string_pretty(&mock_data).unwrap(), "application/json".to_string())
            },
            "txt" | "log" => {
                let mock_content = format!(
                    "Mock file content for: {}\n\nThis is simulated content for demonstration.\nFile type: {}\nRequested at: {}\n\nLine 1 of content\nLine 2 of content\nLine 3 of content",
                    full_path,
                    extension,
                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
                );
                ResourceContent::text(mock_content)
            },
            "md" => {
                let mock_markdown = format!(
                    "# Mock Markdown File: {}\n\nThis is **simulated** markdown content for demonstration purposes.\n\n## Features\n\n- Resource parameter support\n- Multiple content types\n- Security validation\n\n### Code Example\n\n```rust\nlet resource = FileSystemResource::new(\"./data\".to_string());\nlet content = resource.read(Some(json!({{\"path\": \"{}\"}}))).await?;\n```\n\n---\n\n*Generated at: {}*",
                    full_path,
                    file_path,
                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
                );
                ResourceContent::text(mock_markdown)
            },
            _ => ResourceContent::text(format!("Unsupported file type: {}", extension))
        };

        Ok(vec![content])
    }
}

/// Database resource with complex parameter handling
#[derive(Clone)]
struct DatabaseResource {
    connection_info: HashMap<String, String>,
}

impl DatabaseResource {
    fn new() -> Self {
        let mut connection_info = HashMap::new();
        connection_info.insert("host".to_string(), "localhost".to_string());
        connection_info.insert("port".to_string(), "5432".to_string());
        connection_info.insert("database".to_string(), "example_db".to_string());
        
        Self { connection_info }
    }
}

#[async_trait]
impl McpResource for DatabaseResource {
    fn uri(&self) -> &str {
        "db://localhost:5432/example_db"
    }

    fn name(&self) -> &str {
        "Database Query Interface"
    }

    fn description(&self) -> &str {
        "Execute SQL queries and return structured results"
    }

    fn mime_type(&self) -> Option<&str> {
        Some("application/json")
    }

    fn annotations(&self) -> Option<Value> {
        Some(json!({
            "requires_params": true,
            "param_schema": {
                "query": {
                    "type": "string",
                    "description": "SQL query to execute",
                    "required": true
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of rows to return",
                    "default": 100,
                    "minimum": 1,
                    "maximum": 1000
                }
            },
            "connection_info": self.connection_info,
            "supported_operations": ["SELECT", "SHOW", "DESCRIBE", "EXPLAIN"]
        }))
    }

    async fn read(&self, params: Option<Value>) -> mcp_server::McpResult<Vec<ResourceContent>> {
        let query = params
            .as_ref()
            .and_then(|p| p.get("query"))
            .and_then(|q| q.as_str())
            .ok_or_else(|| mcp_protocol::McpError::missing_param("query"))?;

        let limit = params
            .as_ref()
            .and_then(|p| p.get("limit"))
            .and_then(|l| l.as_u64())
            .unwrap_or(100);

        if limit < 1 || limit > 1000 {
            return Err(mcp_protocol::McpError::param_out_of_range(
                "limit",
                &limit.to_string(),
                "Limit must be between 1 and 1000"
            ));
        }

        // Validate query type (security check)
        let query_upper = query.to_uppercase();
        let query_trimmed = query_upper.trim();
        let allowed_prefixes = ["SELECT", "SHOW", "DESCRIBE", "EXPLAIN"];
        if !allowed_prefixes.iter().any(|prefix| query_trimmed.starts_with(prefix)) {
            return Err(mcp_protocol::McpError::param_out_of_range(
                "query",
                query,
                &format!("Query must start with one of: {}", allowed_prefixes.join(", "))
            ));
        }

        // Simulate database query execution
        let mock_results = json!({
            "query": query,
            "limit": limit,
            "execution_time_ms": 45,
            "rows_returned": std::cmp::min(limit, 3),
            "results": [
                {
                    "id": 1,
                    "name": "Example Row 1",
                    "created_at": "2024-08-24T04:00:00Z",
                    "status": "active"
                },
                {
                    "id": 2,
                    "name": "Example Row 2", 
                    "created_at": "2024-08-24T04:30:00Z",
                    "status": "pending"
                },
                {
                    "id": 3,
                    "name": "Example Row 3",
                    "created_at": "2024-08-24T05:00:00Z", 
                    "status": "completed"
                }
            ],
            "metadata": {
                "connection": self.connection_info,
                "query_type": query_upper.split_whitespace().next().unwrap_or("UNKNOWN"),
                "timestamp": chrono::Utc::now().to_rfc3339()
            }
        });

        Ok(vec![ResourceContent::blob(
            serde_json::to_string_pretty(&mock_results).unwrap(),
            "application/json".to_string()
        )])
    }
}

// =============================================================================
// MAIN FUNCTION - DEMONSTRATION
// =============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("🚀 Comprehensive MCP Resource Example");
    println!("=====================================");
    println!("Demonstrating all three approaches for creating MCP resources:\n");

    // =============================================================================
    // APPROACH 1: DERIVE MACRO RESOURCES
    // =============================================================================
    
    println!("📦 APPROACH 1: DERIVE MACRO RESOURCES (#[derive(McpResource)])");
    println!("----------------------------------------------------------------");

    let config_resource = ConfigResource::new();
    let user_profile = UserProfileResource::new(12345);
    let system_status = SystemStatusResource;

    println!("✨ Created derive macro resources:");
    println!("   1. ConfigResource - JSON configuration");
    println!("   2. UserProfileResource - Multi-content with JSON/text/CSV");
    println!("   3. SystemStatusResource - Unit struct for system status");

    // Test derive macro resources
    println!("\n🧪 Testing derive macro resources:");
    
    println!("\n1. Config Resource:");
    println!("   URI: {}", config_resource.uri());
    println!("   Name: {}", config_resource.name());
    println!("   Description: {}", config_resource.description());
    match config_resource.read(None).await {
        Ok(content) => {
            println!("   Content items: {}", content.len());
            for (i, item) in content.iter().enumerate() {
                match item {
                    ResourceContent::Text { text } => println!("   Item {}: Text ({} chars)", i + 1, text.len()),
                    ResourceContent::Blob { data, mime_type } => println!("   Item {}: {} ({} chars)", i + 1, mime_type, data.len()),
                }
            }
        }
        Err(e) => println!("   ❌ Error: {}", e),
    }

    println!("\n2. User Profile Resource (Multi-content):");
    println!("   URI: {}", user_profile.uri());
    println!("   Name: {}", user_profile.name());
    match user_profile.read(None).await {
        Ok(content) => {
            println!("   Content items: {}", content.len());
            for (i, item) in content.iter().enumerate() {
                match item {
                    ResourceContent::Text { text } => println!("   Item {}: Text ({} chars)", i + 1, text.len()),
                    ResourceContent::Blob { data, mime_type } => println!("   Item {}: {} ({} chars)", i + 1, mime_type, data.len()),
                }
            }
        }
        Err(e) => println!("   ❌ Error: {}", e),
    }

    // =============================================================================
    // APPROACH 2: DECLARATIVE MACRO RESOURCES
    // =============================================================================

    println!("\n\n🔧 APPROACH 2: DECLARATIVE MACRO RESOURCES (resource!{{}})");
    println!("-----------------------------------------------------------");

    // Real-time log resource
    let log_resource = resource! {
        uri: "file://application.log",
        name: "Application Log",
        description: "Live application log with recent entries and statistics",
        content: |_self| async move {
            let log_entries = vec![
                "2024-08-24 05:00:00 INFO  [main] Application started",
                "2024-08-24 05:00:01 INFO  [config] Configuration loaded successfully",
                "2024-08-24 05:00:02 INFO  [resources] Initializing MCP resources",
                "2024-08-24 05:00:03 INFO  [resources] - ConfigResource: ready",
                "2024-08-24 05:00:04 INFO  [resources] - UserProfileResource: ready", 
                "2024-08-24 05:00:05 INFO  [resources] - SystemStatusResource: ready",
                "2024-08-24 05:00:06 INFO  [server] MCP server listening on 127.0.0.1:8020",
                "2024-08-24 05:00:07 DEBUG [resources] Resource macro examples initialized",
                "2024-08-24 05:00:08 INFO  [main] All systems operational",
                "2024-08-24 05:00:09 DEBUG [resources] Log resource accessed",
            ].join("\n");
            
            Ok(vec![ResourceContent::text(format!(
                "APPLICATION LOG\n===============\n\n{}\n\n--- LOG STATISTICS ---\nTotal entries: 10\nLog level breakdown:\n  INFO:  8 entries\n  DEBUG: 2 entries\n  ERROR: 0 entries\n\nLast updated: {}",
                log_entries,
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
            ))])
        }
    };

    // API endpoints resource with dynamic content
    let api_endpoints = resource! {
        uri: "api://endpoints",
        name: "API Endpoints Documentation",
        description: "Dynamic API endpoint documentation with usage examples",
        content: |_self| async move {
            let endpoints_data = json!({
                "api_version": "v1",
                "base_url": "http://127.0.0.1:8020/mcp",
                "endpoints": [
                    {
                        "path": "/tools/list",
                        "method": "POST",
                        "description": "List available MCP tools",
                        "example": {
                            "request": {"jsonrpc": "2.0", "id": 1, "method": "tools/list"},
                            "response": {"jsonrpc": "2.0", "id": 1, "result": {"tools": []}}
                        }
                    },
                    {
                        "path": "/resources/list", 
                        "method": "POST",
                        "description": "List available MCP resources",
                        "example": {
                            "request": {"jsonrpc": "2.0", "id": 2, "method": "resources/list"},
                            "response": {"jsonrpc": "2.0", "id": 2, "result": {"resources": []}}
                        }
                    },
                    {
                        "path": "/resources/read",
                        "method": "POST", 
                        "description": "Read content from a specific resource",
                        "example": {
                            "request": {
                                "jsonrpc": "2.0", 
                                "id": 3, 
                                "method": "resources/read",
                                "params": {"uri": "file://config.json"}
                            }
                        }
                    }
                ],
                "generated_at": chrono::Utc::now().to_rfc3339()
            });
            
            Ok(vec![ResourceContent::blob(
                serde_json::to_string_pretty(&endpoints_data).unwrap(),
                "application/json".to_string()
            )])
        }
    };

    println!("✨ Created declarative macro resources:");
    println!("   1. log_resource - Real-time application log");
    println!("   2. api_endpoints - Dynamic API documentation");

    // Test declarative macro resources
    println!("\n🧪 Testing declarative macro resources:");
    
    println!("\n1. Log Resource:");
    println!("   URI: {}", log_resource.uri());
    println!("   Name: {}", log_resource.name());
    match log_resource.read(None).await {
        Ok(content) => {
            println!("   Content items: {}", content.len());
            if let Some(ResourceContent::Text { text }) = content.first() {
                let preview = if text.len() > 100 { 
                    format!("{}...", &text[..100]) 
                } else { 
                    text.clone() 
                };
                println!("   Preview: {}", preview.replace('\n', " "));
            }
        }
        Err(e) => println!("   ❌ Error: {}", e),
    }

    println!("\n2. API Endpoints Resource:");
    println!("   URI: {}", api_endpoints.uri());
    println!("   Name: {}", api_endpoints.name());
    match api_endpoints.read(None).await {
        Ok(content) => {
            println!("   Content items: {}", content.len());
            if let Some(ResourceContent::Blob { data, mime_type }) = content.first() {
                println!("   Type: {} ({} chars)", mime_type, data.len());
            }
        }
        Err(e) => println!("   ❌ Error: {}", e),
    }

    // =============================================================================
    // APPROACH 3: MANUAL IMPLEMENTATION RESOURCES
    // =============================================================================

    println!("\n\n⚙️  APPROACH 3: MANUAL IMPLEMENTATION RESOURCES (impl McpResource)");
    println!("------------------------------------------------------------------");

    let filesystem_resource = FileSystemResource::new("./data".to_string());
    let database_resource = DatabaseResource::new();

    println!("✨ Created manual implementation resources:");
    println!("   1. FileSystemResource - Parameter-based file access");
    println!("   2. DatabaseResource - SQL query interface");

    // Test manual implementation resources
    println!("\n🧪 Testing manual implementation resources:");

    println!("\n1. File System Resource (with parameters):");
    println!("   URI: {}", filesystem_resource.uri());
    println!("   Name: {}", filesystem_resource.name());
    println!("   Annotations: {}", filesystem_resource.annotations().map_or("None".to_string(), |a| a.to_string()));
    
    // Test with different file types
    let test_files = vec!["config.json", "readme.txt", "changelog.md", "app.log"];
    for file in test_files {
        let params = Some(json!({"path": file}));
        match filesystem_resource.read(params).await {
            Ok(content) => {
                println!("   ✅ {}: {} item(s)", file, content.len());
            }
            Err(e) => println!("   ❌ {}: Error - {}", file, e),
        }
    }

    println!("\n2. Database Resource (with SQL parameters):");
    println!("   URI: {}", database_resource.uri());
    println!("   Name: {}", database_resource.name());
    
    let test_queries = vec![
        ("SELECT * FROM users LIMIT 5", Some(json!({"query": "SELECT * FROM users", "limit": 5}))),
        ("SHOW TABLES", Some(json!({"query": "SHOW TABLES"}))),
        ("Invalid query", Some(json!({"query": "DROP TABLE users"}))), // This should fail
    ];
    
    for (desc, params) in test_queries {
        match database_resource.read(params).await {
            Ok(content) => {
                println!("   ✅ {}: {} item(s)", desc, content.len());
            }
            Err(e) => println!("   ❌ {}: Error - {}", desc, e),
        }
    }

    // =============================================================================
    // CREATE AND RUN SERVER
    // =============================================================================

    println!("\n\n🌟 CREATING MCP SERVER WITH ALL RESOURCES");
    println!("==========================================");

    let server = McpServer::builder()
        .name("comprehensive-resource-example")
        .version("1.0.0")
        .title("Comprehensive Resource Example Server")
        .instructions("This server demonstrates all three approaches for creating MCP resources with various content types and parameter handling.")
        
        // Add derive macro resources
        .resource(config_resource)
        .resource(user_profile) 
        .resource(system_status)
        
        // Add declarative macro resources
        .resource(log_resource)
        .resource(api_endpoints)
        
        // Add manual implementation resources
        .resource(filesystem_resource)
        .resource(database_resource)
        
        .with_resources()
        .bind_address("127.0.0.1:8020".parse()?)
        .build()?;

    println!("\n🚀 Server starting at: http://127.0.0.1:8020/mcp");
    println!("\n📋 Available Resources:");
    println!("   DERIVE MACROS:");
    println!("   • file://config.json - Application Configuration (JSON)");
    println!("   • data://user-profile - User Profile Data (JSON + Text + CSV)");  
    println!("   • system://status - System Status (Auto-generated)");
    println!("\n   DECLARATIVE MACROS:");
    println!("   • file://application.log - Real-time Application Log");
    println!("   • api://endpoints - API Endpoints Documentation (JSON)");
    println!("\n   MANUAL IMPLEMENTATION:");
    println!("   • file://filesystem - File System Access (requires path parameter)");
    println!("   • db://localhost:5432/example_db - Database Query Interface (requires query parameter)");

    println!("\n📖 Usage Examples:");
    println!("   curl -X POST http://127.0.0.1:8020/mcp -d '{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"resources/list\"}}'");
    println!("   curl -X POST http://127.0.0.1:8020/mcp -d '{{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"resources/read\",\"params\":{{\"uri\":\"file://config.json\"}}}}'");
    println!("   curl -X POST http://127.0.0.1:8020/mcp -d '{{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"resources/read\",\"params\":{{\"uri\":\"file://filesystem\",\"path\":\"config.json\"}}}}'");

    server.run().await?;
    Ok(())
}