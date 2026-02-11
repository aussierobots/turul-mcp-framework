//! # Development Team Resource Server
//!
//! This example demonstrates a real-world MCP resources server that provides access to
//! development team resources including project documentation, database schemas,
//! configuration files, and system status. This simulates a central resource hub
//! that development teams use to access project artifacts and documentation.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::json;
use std::fs;
use std::path::Path;
use tracing::info;
use turul_mcp_builders::prelude::*; // HasResourceMetadata, HasResourceDescription, etc.
use turul_mcp_protocol::resources::ResourceContent;
use turul_mcp_server::McpResource;
use turul_mcp_server::{McpResult, McpServer, SessionContext};

/// Project documentation resource that loads from actual files
/// Real-world use case: Central documentation hub for development teams
struct ProjectDocumentationResource;

// Fine-grained trait implementations
impl HasResourceMetadata for ProjectDocumentationResource {
    fn name(&self) -> &str {
        "Project Documentation"
    }
}

impl HasResourceUri for ProjectDocumentationResource {
    fn uri(&self) -> &str {
        "file:///docs/project.md"
    }
}

impl HasResourceDescription for ProjectDocumentationResource {
    fn description(&self) -> Option<&str> {
        Some("Comprehensive project documentation including setup, architecture, and guidelines")
    }
}

impl HasResourceMimeType for ProjectDocumentationResource {
    fn mime_type(&self) -> Option<&str> {
        Some("text/markdown")
    }
}

impl HasResourceSize for ProjectDocumentationResource {}
impl HasResourceAnnotations for ProjectDocumentationResource {
    fn annotations(&self) -> Option<&turul_mcp_protocol::meta::Annotations> {
        None
    }
}
impl HasResourceMeta for ProjectDocumentationResource {}
impl HasIcons for ProjectDocumentationResource {}

// ResourceDefinition is automatically implemented via blanket impl
// Now implement the execution trait
#[async_trait]
impl McpResource for ProjectDocumentationResource {
    async fn read(
        &self,
        _params: Option<serde_json::Value>,
        _session: Option<&SessionContext>,
    ) -> McpResult<Vec<ResourceContent>> {
        let mut contents = Vec::new();

        // Load main project README
        let readme_path = Path::new("../../README.md");
        if readme_path.exists() {
            match fs::read_to_string(readme_path) {
                Ok(content) => {
                    contents.push(ResourceContent::text(
                        "file:///docs/project.md",
                        format!("# Main Project Documentation\n\n{}", content),
                    ));
                }
                Err(_) => {
                    contents.push(ResourceContent::text(
                        "file:///docs/project.md",
                        "# Project Documentation\n\nMain README file not accessible".to_string(),
                    ));
                }
            }
        }

        // Add project structure overview
        contents.push(ResourceContent::text(
            "file:///docs/project.md",
            "# MCP Framework Project Structure\n\n\
             ## Core Crates\n\
             - `mcp-server/` - Main MCP server framework\n\
             - `mcp-protocol/` - Protocol definitions and types\n\
             - `mcp-derive/` - Procedural macros for tool generation\n\
             - `http-mcp-server/` - HTTP transport layer\n\
             - `json-rpc-server/` - JSON-RPC implementation\n\n\
             ## Example Applications\n\
             - 25 comprehensive examples demonstrating different patterns\n\
             - Real-world use cases: IDE completion, CMS resources, notifications\n\
             - Performance testing and benchmarking tools\n\n\
             ## Development Workflow\n\
             1. Make changes to core framework\n\
             2. Update examples to demonstrate new features\n\
             3. Run comprehensive test suite\n\
             4. Update documentation and examples\n\
             5. Performance testing and validation"
                .to_string(),
        ));

        // Add architectural overview
        contents.push(ResourceContent::text("file:///docs/project.md",
            "# Architecture Overview\n\n\
             ## MCP Protocol Implementation\n\
             The framework implements the Model Context Protocol (MCP) 2025-11-25 specification:\n\n\
             - **Tools**: Server-side functions that clients can invoke\n\
             - **Resources**: Structured data and files that clients can access\n\
             - **Prompts**: AI prompt templates for language model interactions\n\
             - **Completion**: Auto-completion suggestions for user inputs\n\
             - **Notifications**: Real-time updates via Server-Sent Events\n\n\
             ## Session Management\n\
             - Stateful operations with automatic cleanup\n\
             - Type-safe state storage and retrieval\n\
             - Progress tracking and notification broadcasting\n\n\
             ## Development Patterns\n\
             1. **Manual Implementation**: Full control with trait implementation\n\
             2. **Derive Macros**: Automatic schema generation from structs\n\
             3. **Function Macros**: Natural function-based tool definitions\n\
             4. **Declarative Macros**: Ultra-concise tool creation".to_string()
        ));

        Ok(contents)
    }
}

/// API documentation resource loaded from external markdown file
/// Real-world use case: Team API documentation accessible via MCP
struct ApiDocumentationResource;

// Fine-grained trait implementations
impl HasResourceMetadata for ApiDocumentationResource {
    fn name(&self) -> &str {
        "API Documentation"
    }
}

impl HasResourceUri for ApiDocumentationResource {
    fn uri(&self) -> &str {
        "file:///docs/api.md"
    }
}

impl HasResourceDescription for ApiDocumentationResource {
    fn description(&self) -> Option<&str> {
        Some("Complete API documentation with authentication, endpoints, examples and SDKs")
    }
}

impl HasResourceMimeType for ApiDocumentationResource {
    fn mime_type(&self) -> Option<&str> {
        Some("text/markdown")
    }
}

impl HasResourceSize for ApiDocumentationResource {}
impl HasResourceAnnotations for ApiDocumentationResource {
    fn annotations(&self) -> Option<&turul_mcp_protocol::meta::Annotations> {
        None
    }
}
impl HasResourceMeta for ApiDocumentationResource {}
impl HasIcons for ApiDocumentationResource {}

#[async_trait]
impl McpResource for ApiDocumentationResource {
    async fn read(
        &self,
        _params: Option<serde_json::Value>,
        _session: Option<&SessionContext>,
    ) -> McpResult<Vec<ResourceContent>> {
        let api_docs_path = Path::new("data/api_docs.md");

        match fs::read_to_string(api_docs_path) {
            Ok(content) => Ok(vec![ResourceContent::text("file:///docs/api.md", content)]),
            Err(_) => {
                // Fallback content if file is not accessible
                Ok(vec![ResourceContent::text(
                    "file:///docs/api.md",
                    "# API Documentation\n\n\
                         Documentation file not found at data/api_docs.md\n\n\
                         This resource loads API documentation from external markdown files,\n\
                         demonstrating how production systems maintain documentation\n\
                         separate from code for easier updates and collaboration.\n\n\
                         ## Expected Structure\n\
                         - Authentication methods\n\
                         - Base URLs and versioning\n\
                         - Rate limiting policies\n\
                         - Endpoint documentation\n\
                         - Request/response examples\n\
                         - SDK and client library information\n\
                         - Error handling guidelines"
                        .to_string(),
                )])
            }
        }
    }
}

/// Configuration resource loaded from external JSON file
/// Real-world use case: Production configuration management with external files
struct ConfigurationResource;

// Fine-grained trait implementations
impl HasResourceMetadata for ConfigurationResource {
    fn name(&self) -> &str {
        "Application Configuration"
    }
}

impl HasResourceUri for ConfigurationResource {
    fn uri(&self) -> &str {
        "file:///config/app.json"
    }
}

impl HasResourceDescription for ConfigurationResource {
    fn description(&self) -> Option<&str> {
        Some("Production application configuration loaded from external JSON file")
    }
}

impl HasResourceMimeType for ConfigurationResource {
    fn mime_type(&self) -> Option<&str> {
        Some("application/json")
    }
}

impl HasResourceSize for ConfigurationResource {}
impl HasResourceAnnotations for ConfigurationResource {
    fn annotations(&self) -> Option<&turul_mcp_protocol::meta::Annotations> {
        None
    }
}
impl HasResourceMeta for ConfigurationResource {}
impl HasIcons for ConfigurationResource {}

#[async_trait]
impl McpResource for ConfigurationResource {
    async fn read(
        &self,
        _params: Option<serde_json::Value>,
        _session: Option<&SessionContext>,
    ) -> McpResult<Vec<ResourceContent>> {
        let config_path = Path::new("data/app_config.json");

        let mut contents = Vec::new();

        match fs::read_to_string(config_path) {
            Ok(config_content) => {
                // Parse and pretty-print the JSON for better readability
                match serde_json::from_str::<serde_json::Value>(&config_content) {
                    Ok(config_json) => {
                        contents.push(ResourceContent::text(
                            "file:///config/app.json",
                            serde_json::to_string_pretty(&config_json).unwrap(),
                        ));
                    }
                    Err(_) => {
                        contents.push(ResourceContent::text(
                            "file:///config/app.json",
                            config_content,
                        ));
                    }
                }
            }
            Err(_) => {
                contents.push(ResourceContent::text(
                    "file:///config/app.json",
                    "# Application Configuration\n\n\
                     Configuration file not found at data/app_config.json\n\n\
                     This resource demonstrates production configuration management\n\
                     where configuration is externalized from code for:\n\n\
                     - **Environment-specific settings**\n\
                     - **Security**: Sensitive values via environment variables\n\
                     - **Maintainability**: Configuration updates without deployments\n\
                     - **Compliance**: Audit trails for configuration changes\n\n\
                     ## Expected Configuration Sections\n\
                     - Server settings (host, port, workers)\n\
                     - Database configuration\n\
                     - Redis/cache settings\n\
                     - Authentication and security\n\
                     - Feature flags\n\
                     - Monitoring and observability\n\
                     - External service integrations"
                        .to_string(),
                ));
            }
        }

        // Add environment variables documentation
        contents.push(ResourceContent::text(
            "file:///config/app.json",
            "# Environment Variables Guide\n\n\
             ## Security Best Practices\n\
             Never commit sensitive values to configuration files. Use environment variables:\n\n\
             ```bash\n\
             # Required secrets\n\
             export JWT_SECRET=\"your-256-bit-secret\"\n\
             export DB_PASSWORD=\"your-database-password\"\n\
             export REDIS_PASSWORD=\"your-redis-password\"\n\
             export S3_ACCESS_KEY=\"your-s3-access-key\"\n\
             export S3_SECRET_KEY=\"your-s3-secret-key\"\n\n\
             # Optional overrides\n\
             export APP_PORT=\"8080\"\n\
             export LOG_LEVEL=\"info\"\n\
             export WORKERS=\"4\"\n\
             ```\n\n\
             ## Configuration Management\n\
             1. **Development**: Use .env files (never commit)\n\
             2. **Staging**: Environment-specific configs\n\
             3. **Production**: Secure secret management systems\n\
             4. **Docker**: Environment variables in compose files\n\
             5. **Kubernetes**: ConfigMaps and Secrets"
                .to_string(),
        ));

        Ok(contents)
    }
}

/// Database schema resource loaded from external SQL file
/// Real-world use case: Database schema documentation and migrations
struct DatabaseSchemaResource;

// Fine-grained trait implementations
impl HasResourceMetadata for DatabaseSchemaResource {
    fn name(&self) -> &str {
        "Database Schema"
    }
}

impl HasResourceUri for DatabaseSchemaResource {
    fn uri(&self) -> &str {
        "file:///schema/database.json"
    }
}

impl HasResourceDescription for DatabaseSchemaResource {
    fn description(&self) -> Option<&str> {
        Some("Production database schema with tables, indexes, and relationships")
    }
}

impl HasResourceMimeType for DatabaseSchemaResource {
    fn mime_type(&self) -> Option<&str> {
        Some("application/sql")
    }
}

impl HasResourceSize for DatabaseSchemaResource {}
impl HasResourceAnnotations for DatabaseSchemaResource {
    fn annotations(&self) -> Option<&turul_mcp_protocol::meta::Annotations> {
        None
    }
}
impl HasResourceMeta for DatabaseSchemaResource {}
impl HasIcons for DatabaseSchemaResource {}

#[async_trait]
impl McpResource for DatabaseSchemaResource {
    async fn read(
        &self,
        _params: Option<serde_json::Value>,
        _session: Option<&SessionContext>,
    ) -> McpResult<Vec<ResourceContent>> {
        let schema_path = Path::new("data/database_schema.sql");

        let mut contents = Vec::new();

        match fs::read_to_string(schema_path) {
            Ok(schema_content) => {
                contents.push(ResourceContent::text(
                    "file:///schema/database.json",
                    schema_content,
                ));
            }
            Err(_) => {
                contents.push(ResourceContent::text(
                    "file:///schema/database.json",
                    "-- Database Schema\n\
                     -- Schema file not found at data/database_schema.sql\n\n\
                     /*\n\
                      * This resource demonstrates how development teams manage\n\
                      * database schemas using external SQL files for:\n\
                      *\n\
                      * - Version control: Track schema changes over time\n\
                      * - Migrations: Structured database updates\n\
                      * - Documentation: Living schema documentation\n\
                      * - Team collaboration: Shared schema understanding\n\
                      * - Deployment: Automated schema deployment\n\
                      */\n\n\
                     -- Expected schema structure:\n\
                     -- ✓ User management tables\n\
                     -- ✓ Content management (posts, comments)\n\
                     -- ✓ Media and file storage\n\
                     -- ✓ Authentication and sessions\n\
                     -- ✓ Audit logs and tracking\n\
                     -- ✓ Proper indexes and constraints\n\
                     -- ✓ Views for common queries"
                        .to_string(),
                ));
            }
        }

        // Add database documentation
        contents.push(ResourceContent::text(
            "file:///schema/database.json",
            "# Database Architecture Guide\n\n\
             ## Schema Management Best Practices\n\n\
             ### 1. Migration Strategy\n\
             - **Version Control**: All schema changes in SQL files\n\
             - **Forward-only**: Never edit existing migrations\n\
             - **Rollback Plans**: Document rollback procedures\n\
             - **Testing**: Test migrations on staging first\n\n\
             ### 2. Performance Considerations\n\
             - **Indexes**: Strategic indexing for query patterns\n\
             - **Constraints**: Enforce data integrity at database level\n\
             - **Partitioning**: Consider for large tables\n\
             - **Monitoring**: Track query performance metrics\n\n\
             ### 3. Security Features\n\
             - **Row-level Security**: Implemented where needed\n\
             - **Audit Trails**: Complete change tracking\n\
             - **Access Control**: Minimal privilege principles\n\
             - **Encryption**: Sensitive data protection\n\n\
             ### 4. Backup and Recovery\n\
             - **Automated Backups**: Daily production backups\n\
             - **Point-in-time Recovery**: Transaction log backups\n\
             - **Disaster Recovery**: Cross-region replication\n\
             - **Testing**: Regular restore testing"
                .to_string(),
        ));

        Ok(contents)
    }
}

/// System status resource providing real-time information
struct SystemStatusResource;

// Fine-grained trait implementations
impl HasResourceMetadata for SystemStatusResource {
    fn name(&self) -> &str {
        "System Status"
    }
}

impl HasResourceUri for SystemStatusResource {
    fn uri(&self) -> &str {
        "file:///status/system.json"
    }
}

impl HasResourceDescription for SystemStatusResource {
    fn description(&self) -> Option<&str> {
        Some("Real-time system status and health metrics")
    }
}

impl HasResourceMimeType for SystemStatusResource {
    fn mime_type(&self) -> Option<&str> {
        Some("application/json")
    }
}

impl HasResourceSize for SystemStatusResource {}
impl HasResourceAnnotations for SystemStatusResource {
    fn annotations(&self) -> Option<&turul_mcp_protocol::meta::Annotations> {
        None
    }
}
impl HasResourceMeta for SystemStatusResource {}
impl HasIcons for SystemStatusResource {}

#[async_trait]
impl McpResource for SystemStatusResource {
    async fn read(
        &self,
        _params: Option<serde_json::Value>,
        _session: Option<&SessionContext>,
    ) -> McpResult<Vec<ResourceContent>> {
        let now: DateTime<Utc> = Utc::now();

        let status = json!({
            "timestamp": now.to_rfc3339(),
            "uptime": "2d 14h 32m 18s",
            "server": {
                "status": "healthy",
                "version": "1.2.3",
                "environment": "production",
                "last_restart": "2024-01-15T08:30:00Z"
            },
            "database": {
                "status": "connected",
                "connections": {
                    "active": 15,
                    "idle": 5,
                    "max": 20
                },
                "response_time_ms": 12.5,
                "last_backup": "2024-01-17T02:00:00Z"
            },
            "redis": {
                "status": "connected",
                "memory_usage": "245MB",
                "keys": 12847,
                "response_time_ms": 2.1
            },
            "resources": {
                "cpu_usage": 45.2,
                "memory_usage": 68.7,
                "disk_usage": 23.4,
                "network_io": {
                    "rx_mbps": 125.3,
                    "tx_mbps": 89.7
                }
            },
            "metrics": {
                "requests_per_minute": 1247,
                "error_rate": 0.02,
                "average_response_time_ms": 85.6,
                "active_sessions": 342
            }
        });

        Ok(vec![
            ResourceContent::text(
                "file:///status/system.json",
                serde_json::to_string_pretty(&status).unwrap(),
            ),
            ResourceContent::text(
                "file:///status/system.json",
                "# System Health Report\n\n\
                 ## Overall Status: ✅ HEALTHY\n\n\
                 ### Services\n\
                 - **Web Server**: ✅ Running (version 1.2.3)\n\
                 - **Database**: ✅ Connected (PostgreSQL 15.2)\n\
                 - **Cache**: ✅ Connected (Redis 7.0)\n\
                 - **Background Jobs**: ✅ Processing\n\n\
                 ### Performance\n\
                 - **Response Time**: 85.6ms average\n\
                 - **Error Rate**: 0.02% (well below 1% threshold)\n\
                 - **Throughput**: 1,247 requests/minute\n\n\
                 ### Resources\n\
                 - **CPU**: 45.2% (normal)\n\
                 - **Memory**: 68.7% (normal)\n\
                 - **Disk**: 23.4% (plenty of space)\n\n\
                 ### Alerts\n\
                 No active alerts or warnings."
                    .to_string(),
            ),
        ])
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting Development Team Resource Server");

    let server = McpServer::builder()
        .name("development-resource-server")
        .version("1.0.0")
        .title("Development Team Resource Server")
        .instructions("Real-world MCP resources server for development teams. Provides access to project documentation, API specs, configuration files, database schemas, and system status. Loads data from external files demonstrating production resource management patterns.")
        .resource(ProjectDocumentationResource)
        .resource(ApiDocumentationResource)
        .resource(ConfigurationResource)
        .resource(DatabaseSchemaResource)
        .resource(SystemStatusResource)
        .with_resources()
        .bind_address("127.0.0.1:8041".parse()?)
        .build()?;

    info!("Development resource server running at: http://127.0.0.1:8041/mcp");
    info!("Real-world team resources available:");
    info!("  - docs://project: Comprehensive project documentation and architecture");
    info!("  - docs://api: Complete API documentation loaded from external markdown");
    info!("  - config://app: Production configuration management with security best practices");
    info!("  - schema://database: Database schema and migration management");
    info!("  - status://system: Real-time system monitoring and health metrics");
    info!("External data files: data/api_docs.md, data/app_config.json, data/database_schema.sql");

    server.run().await?;
    Ok(())
}
