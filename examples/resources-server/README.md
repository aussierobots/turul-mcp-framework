# Development Team Resource Server

A **real-world MCP resources server** that provides centralized access to development team resources including project documentation, API specifications, configuration files, database schemas, and system status monitoring. This example demonstrates how teams can use MCP to create a unified resource hub that loads data from external files.

## Real-World Use Case

This server simulates a **development team's resource portal** that provides:

- **📚 Project Documentation**: Centralized access to project docs and architecture
- **🔗 API Documentation**: Complete API specs loaded from external markdown files
- **⚙️ Configuration Management**: Production config files with security best practices
- **🗄️ Database Schema**: Version-controlled schema documentation and migration guides
- **📊 System Monitoring**: Real-time system health and performance metrics

### Why External Data Files?

Unlike hardcoded resource content, this server loads information from **external files** demonstrating:
- **Maintainability**: Update documentation without code changes
- **Collaboration**: Teams can maintain docs using standard tools (markdown, JSON, SQL)
- **Version Control**: Track changes to resources alongside code
- **Real-world pattern**: How production systems manage documentation and configuration

## Architecture

```
resources-server/
├── src/main.rs                 # Server implementation
├── data/                       # External resource files
│   ├── api_docs.md             # API documentation (markdown)
│   ├── app_config.json         # Application configuration
│   └── database_schema.sql     # Database schema with migrations
└── README.md
```

## Features

### 🎯 **Production Resource Patterns**
- **External file loading** with graceful fallbacks
- **Multiple content formats** (markdown, JSON, SQL)
- **Security-aware configuration** with environment variable patterns
- **Real-time status monitoring** with system metrics

### 📊 **Team Collaboration**
- **Centralized documentation** accessible via MCP protocol
- **Configuration transparency** for all team members
- **Schema documentation** for database changes
- **System visibility** for operations and debugging

### ⚡ **Production Ready**
- **Error handling** for missing or corrupted files
- **Structured content** with proper formatting
- **Performance monitoring** and health checks

## Running the Server

### Quick Start
```bash
# Ensure you're in the resources-server directory for data/ access
cd examples/resources-server

# Run the development resource server (default: 127.0.0.1:8041)
cargo run -p resources-server

# Expected output:
# INFO resources_server: 🚀 Starting Development Resources Server on port 8041
# INFO turul_mcp_server::builder: 🔧 Auto-configured server capabilities:
# INFO turul_mcp_server::builder:    - Resources: true
# INFO turul_mcp_server::server: ✅ Server started successfully on http://127.0.0.1:8041/mcp
```

### Verify Server is Working
```bash
# Initialize connection (in another terminal)
curl -X POST http://127.0.0.1:8041/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "initialize",
    "params": {
      "protocolVersion": "2025-11-25",
      "capabilities": {},
      "clientInfo": {"name": "test", "version": "1.0"}
    },
    "id": 1
  }' | jq

# List available resources
curl -X POST http://127.0.0.1:8041/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "resources/list",
    "params": {},
    "id": 2
  }' | jq

# Should show 5 available resources:
# - docs://project, docs://api, config://app, schema://database, status://system
```

## Available Resources

### 1. Project Documentation (`docs://project`)

**Purpose**: Comprehensive project documentation and architecture guides  
**Source**: Loads from actual project README and provides framework overview

**Example Request:**
```bash
curl -X POST http://127.0.0.1:8041/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "resources/read",
    "params": {"uri": "docs://project"},
    "id": 1
  }'
```

**Content Includes:**
- Main project README (loaded from `../../README.md`)
- MCP Framework project structure and architecture
- Development workflow and best practices
- Core crates and example applications overview

### 2. API Documentation (`docs://api`)

**Purpose**: Complete API documentation loaded from external markdown  
**Source**: `data/api_docs.md` (comprehensive API reference)

**Example Request:**
```bash
curl -X POST http://127.0.0.1:8041/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "resources/read",
    "params": {"uri": "docs://api"},
    "id": 1
  }'
```

**Content Includes:**
- Authentication methods and API key usage
- Complete endpoint documentation with examples
- Request/response schemas and error handling
- SDK examples for multiple languages (JavaScript, Python)
- Rate limiting and best practices

### 3. Application Configuration (`config://app`)

**Purpose**: Production configuration management with security best practices  
**Source**: `data/app_config.json` (comprehensive application settings)

**Example Request:**
```bash
curl -X POST http://127.0.0.1:8041/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "resources/read",
    "params": {"uri": "config://app"},
    "id": 1
  }'
```

**Content Includes:**
- Server, database, and Redis configuration
- Authentication and security settings
- Storage providers (local and S3) configuration
- Email, logging, and monitoring settings
- Environment variable security guide
- Feature flags and API configuration

### 4. Database Schema (`schema://database`)

**Purpose**: Database schema documentation and migration management  
**Source**: `data/database_schema.sql` (production-ready schema)

**Example Request:**
```bash
curl -X POST http://127.0.0.1:8041/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "resources/read",
    "params": {"uri": "schema://database"},
    "id": 1
  }'
```

**Content Includes:**
- Complete database schema with tables, indexes, and constraints
- User management, content, and media tables
- Audit logging and session management
- Views for common queries and default data
- Schema management best practices
- Migration strategy and backup procedures

### 5. System Status (`status://system`)

**Purpose**: Real-time system monitoring and health metrics  
**Source**: Generated dynamically with current timestamps

**Example Request:**
```bash
curl -X POST http://127.0.0.1:8041/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "resources/read",
    "params": {"uri": "status://system"},
    "id": 1
  }'
```

**Content Includes:**
- Server status, version, and uptime information
- Database connection metrics and performance
- Redis cache statistics and memory usage
- System resource utilization (CPU, memory, disk)
- Application metrics (requests/minute, error rate)
- Comprehensive health report with status indicators

## External Data Files

### API Documentation (`data/api_docs.md`)
```markdown
# API Documentation

## Authentication
All API requests require a valid API key...

## Endpoints
### GET /users
Retrieve a list of users...
```

### Configuration (`data/app_config.json`)
```json
{
  "application": {
    "name": "Content Management System",
    "version": "2.1.4",
    "environment": "production"
  },
  "server": {
    "host": "0.0.0.0",
    "port": 8080,
    "workers": 4
  },
  "security": {
    "jwt": {
      "secret": "${JWT_SECRET}",
      "expiration": 3600
    }
  }
}
```

### Database Schema (`data/database_schema.sql`)
```sql
-- Production Database Schema
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    username VARCHAR(50) UNIQUE NOT NULL,
    email VARCHAR(100) UNIQUE NOT NULL,
    -- ... comprehensive schema
);

-- Indexes and constraints
CREATE INDEX idx_users_email ON users(email);
```

## Implementation Highlights

### External File Loading
```rust
impl ApiDocumentationResource {
    async fn read(&self) -> McpResult<Vec<ResourceContent>> {
        let api_docs_path = Path::new("data/api_docs.md");
        
        match fs::read_to_string(api_docs_path) {
            Ok(content) => Ok(vec![ResourceContent::text(content)]),
            Err(_) => {
                // Graceful fallback with explanation
                Ok(vec![ResourceContent::text(fallback_content)])
            }
        }
    }
}
```

### Configuration with Security
```rust
// Production configuration management
match fs::read_to_string("data/app_config.json") {
    Ok(config_content) => {
        // Parse and validate JSON configuration
        let config_json: serde_json::Value = serde_json::from_str(&config_content)?;
        // Pretty-print for readability
        ResourceContent::text(serde_json::to_string_pretty(&config_json)?)
    }
}
```

### Dynamic Status Generation
```rust
// Real-time system status
let now: DateTime<Utc> = Utc::now();
let status = json!({
    "timestamp": now.to_rfc3339(),
    "server": {"status": "healthy", "version": "1.2.3"},
    "metrics": {"requests_per_minute": 1247, "error_rate": 0.02}
});
```

## Real-World Applications

### Development Teams
- **Onboarding**: New team members access all project resources in one place
- **Documentation**: Living documentation that stays up-to-date with code
- **Configuration**: Transparent configuration management for all environments
- **Debugging**: Quick access to schema and system status for troubleshooting

### DevOps and Operations
- **Monitoring**: System health and performance metrics via MCP
- **Configuration Management**: Centralized access to all application settings
- **Schema Management**: Database documentation and migration tracking
- **Incident Response**: Quick access to system status and configuration

### Integration Scenarios
- **CI/CD Pipelines**: Access configuration and documentation during builds
- **IDE Extensions**: Project resources accessible directly in development environments
- **API Clients**: Documentation and configuration for API integration
- **Monitoring Tools**: System metrics and health data via standardized protocol

## Security Considerations

### Configuration Security
- **Environment Variables**: Sensitive values never stored in config files
- **Access Control**: Resource access can be extended with authentication
- **Audit Trails**: Changes to external files tracked via version control
- **Separation of Concerns**: Public documentation vs sensitive configuration

### Best Practices Demonstrated
```bash
# Secure environment variable patterns
export JWT_SECRET="your-256-bit-secret"
export DB_PASSWORD="your-database-password"

# Configuration management
- Development: .env files (never committed)
- Staging: Environment-specific configs
- Production: Secure secret management systems
```

## Extension Opportunities

### Enhanced Data Sources
- **Git Integration**: Load documentation from repository branches
- **Database Connectivity**: Live schema introspection and table statistics
- **Monitoring APIs**: Real metrics from Prometheus, Grafana, or DataDog
- **Cloud Services**: Configuration from AWS Parameter Store or Kubernetes ConfigMaps

### Advanced Features
- **Authentication**: User-specific resource access with permissions
- **Caching**: Performance optimization for expensive resource operations
- **Versioning**: Historical versions of configuration and documentation
- **Real-time Updates**: SSE notifications when external files change

### Team Workflow Integration
- **Pull Request Reviews**: Configuration and schema change validation
- **Documentation Generation**: Auto-generated API docs from code annotations
- **Environment Promotion**: Configuration comparison across environments
- **Compliance Reporting**: Audit trails for configuration and schema changes

This resources server demonstrates how external data files enable maintainable, collaborative resource management that scales with team growth and evolving documentation needs.
