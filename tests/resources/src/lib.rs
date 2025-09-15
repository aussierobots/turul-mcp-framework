//! Resources module demonstrating MCP resource integration
//!
//! This module shows how to properly implement MCP resources using the derive macro
//! with correct imports and error handling.

use turul_mcp_derive::McpResource;
use turul_mcp_server::prelude::*;

/// User profile resource
#[derive(McpResource, Clone, Serialize, Deserialize, Debug)]
#[resource(
    name = "user_profile",
    uri = "app://users/{user_id}",
    description = "User profile information and preferences"
)]
pub struct UserProfileResource {
    pub include_preferences: bool,
}
#[async_trait]
impl McpResource for UserProfileResource {
    async fn read(&self, params: Option<Value>) -> McpResult<Vec<ResourceContent>> {
        let user_id = params
            .as_ref()
            .and_then(|vars| vars.get("user_id"))
            .and_then(|id| id.as_str().map(String::from))
            .ok_or_else(|| McpError::InvalidParameters("Missing user_id parameter".to_string()))?;
        self.fetch_profile_data(user_id.as_str()).await
    }
}

impl UserProfileResource {
    pub fn new() -> Self {
        Self {
            include_preferences: false,
        }
    }

    pub fn with_preferences(mut self) -> Self {
        self.include_preferences = true;
        self
    }

    /// Business logic method to fetch user profile data
    pub async fn fetch_profile_data(&self, user_id: &str) -> McpResult<Vec<ResourceContent>> {
        let mut profile_data = json!({
            "user_id": user_id,
            "name": "John Doe",
            "email": "john.doe@example.com",
            "created_at": "2024-01-15T10:00:00Z",
            "status": "active"
        });

        if self.include_preferences {
            profile_data["preferences"] = json!({
                "theme": "dark",
                "language": "en",
                "notifications": {
                    "email": true,
                    "push": false,
                    "sms": false
                },
                "privacy": {
                    "profile_visible": true,
                    "allow_contact": false
                }
            });
        }

        Ok(vec![ResourceContent::text(
            &format!("app://users/{}", user_id),
            serde_json::to_string_pretty(&profile_data)
                .map_err(|e| McpError::tool_execution(&format!("Serialization error: {}", e)))?,
        )])
    }
}

/// Configuration resource
#[derive(McpResource, Clone, Serialize, Deserialize, Debug)]
#[resource(
    name = "app_config",
    uri = "app://config/{config_type}",
    description = "Application configuration settings"
)]
pub struct AppConfigResource {
    pub config_type: String, // "database", "api", "feature_flags"
    pub environment: String,
}

impl AppConfigResource {
    pub fn new(config_type: impl Into<String>, environment: impl Into<String>) -> Self {
        Self {
            config_type: config_type.into(),
            environment: environment.into(),
        }
    }

    /// Fetch configuration based on type and environment
    pub async fn fetch_config_data(&self) -> McpResult<Vec<ResourceContent>> {
        let config_data = match self.config_type.as_str() {
            "database" => json!({
                "type": "database",
                "environment": self.environment,
                "connection": {
                    "host": "db.example.com",
                    "port": 5432,
                    "database": format!("myapp_{}", self.environment),
                    "pool_size": 20,
                    "timeout": 30
                }
            }),
            "api" => json!({
                "type": "api",
                "environment": self.environment,
                "settings": {
                    "base_url": format!("https://api-{}.example.com", self.environment),
                    "timeout": 10000,
                    "rate_limit": {
                        "requests_per_minute": 1000,
                        "burst": 50
                    },
                    "cors": {
                        "enabled": true,
                        "origins": ["https://app.example.com"]
                    }
                }
            }),
            "feature_flags" => json!({
                "type": "feature_flags",
                "environment": self.environment,
                "flags": {
                    "new_dashboard": self.environment == "production",
                    "beta_features": self.environment != "production",
                    "advanced_analytics": true,
                    "maintenance_mode": false
                }
            }),
            _ => json!({
                "type": "unknown",
                "error": format!("Unknown config type: {}", self.config_type)
            }),
        };

        Ok(vec![ResourceContent::blob(
            &format!("app://config/{}", self.config_type),
            serde_json::to_string_pretty(&config_data).map_err(|e| {
                McpError::tool_execution(&format!("Config serialization error: {}", e))
            })?,
            "application/json".to_string(),
        )])
    }
}

/// Log files resource
#[derive(McpResource, Clone, Serialize, Deserialize, Debug)]
#[resource(
    name = "log_files",
    uri = "app://logs/{log_type}",
    description = "Application log files and monitoring data"
)]
pub struct LogFilesResource {
    pub log_type: String, // "application", "access", "error"
    pub lines: Option<u32>,
}

impl LogFilesResource {
    pub fn new(log_type: impl Into<String>) -> Self {
        Self {
            log_type: log_type.into(),
            lines: None,
        }
    }

    pub fn with_lines(mut self, lines: u32) -> Self {
        self.lines = Some(lines);
        self
    }

    pub async fn fetch_log_data(&self) -> McpResult<Vec<ResourceContent>> {
        let lines_to_fetch = self.lines.unwrap_or(100);

        let sample_logs = match self.log_type.as_str() {
            "application" => vec![
                "2024-01-15T14:30:01Z INFO Starting application server on port 8080",
                "2024-01-15T14:30:02Z INFO Database connection established",
                "2024-01-15T14:30:05Z INFO User authentication service initialized",
                "2024-01-15T14:31:10Z INFO Processing user login request",
                "2024-01-15T14:31:11Z DEBUG Session created for user: john.doe@example.com",
            ],
            "access" => vec![
                "127.0.0.1 - - [15/Jan/2024:14:30:05 +0000] \"GET /api/users/profile HTTP/1.1\" 200 1234",
                "192.168.1.100 - - [15/Jan/2024:14:30:10 +0000] \"POST /api/auth/login HTTP/1.1\" 200 567",
                "10.0.0.50 - - [15/Jan/2024:14:30:15 +0000] \"GET /api/config/features HTTP/1.1\" 200 890",
                "127.0.0.1 - - [15/Jan/2024:14:30:20 +0000] \"PUT /api/users/preferences HTTP/1.1\" 200 445",
            ],
            "error" => vec![
                "2024-01-15T14:25:30Z ERROR Database connection timeout after 30s",
                "2024-01-15T14:26:45Z WARN Rate limit exceeded for IP: 192.168.1.200",
                "2024-01-15T14:28:12Z ERROR Failed to validate user token: expired",
                "2024-01-15T14:29:33Z ERROR API call failed: external service unavailable",
            ],
            _ => vec!["ERROR: Unknown log type requested"]
        };

        let log_content = sample_logs
            .into_iter()
            .take(lines_to_fetch as usize)
            .collect::<Vec<_>>()
            .join("\n");

        Ok(vec![ResourceContent::text(
            &format!("app://logs/{}", self.log_type),
            format!(
                "=== {} Logs (last {} lines) ===\n{}",
                self.log_type.to_uppercase(),
                lines_to_fetch,
                log_content
            ),
        )])
    }
}

/// File user resource demonstrating URI template "file:///user/{user_id}.json"
#[derive(McpResource, Clone, Serialize, Deserialize, Debug)]
#[resource(
    name = "file_user",
    uri = "file:///user/{user_id}.json",
    description = "User data stored as JSON files with URI template support"
)]
pub struct FileUserResource {
    pub user_id: String,
}

impl FileUserResource {
    pub fn new(user_id: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
        }
    }

    pub async fn fetch_user_data(&self) -> Result<Vec<ResourceContent>, McpError> {
        let user_data = json!({
            "id": self.user_id,
            "name": format!("User {}", self.user_id),
            "email": format!("user{}@example.com", self.user_id),
            "profile": {
                "created_at": "2024-01-01T00:00:00Z",
                "last_login": "2024-01-15T10:30:00Z",
                "preferences": {
                    "theme": "dark",
                    "language": "en",
                    "notifications": true
                }
            }
        });

        Ok(vec![ResourceContent::text(
            &format!("file:///user/{}.json", self.user_id),
            serde_json::to_string_pretty(&user_data).map_err(|e| {
                McpError::tool_execution(&format!("User data serialization error: {}", e))
            })?,
        )])
    }
}

// Note: McpResource trait is automatically implemented by the derive macro

/// User avatar resource demonstrating BlobResourceContent with base64 images
#[derive(McpResource, Clone, Serialize, Deserialize, Debug)]
#[resource(
    name = "user_avatar",
    uri = "file:///user/{user_id}/avatar.png",
    description = "User avatar images as base64-encoded blob resources"
)]
pub struct UserAvatarResource {
    pub user_id: String,
}

impl UserAvatarResource {
    pub fn new(user_id: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
        }
    }

    pub async fn fetch_avatar_data(&self) -> Result<Vec<ResourceContent>, McpError> {
        let base64_avatar = match self.user_id.as_str() {
            "123" => "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==",
            "456" => "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==",
            _ => "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChgGBfhzPMwAAAABJRU5ErkJggg=="
        };

        Ok(vec![ResourceContent::blob(
            &format!("file:///user/{}/avatar.png", self.user_id),
            base64_avatar.to_string(),
            "image/png",
        )])
    }
}

// Note: McpResource trait is automatically implemented by the derive macro

/// Binary data resource demonstrating various blob types
#[derive(McpResource, Clone, Serialize, Deserialize, Debug)]
#[resource(
    name = "binary_data",
    uri = "file:///data/{data_type}.{format}",
    description = "Binary data resources in various formats (PDF, ZIP, etc.)"
)]
pub struct BinaryDataResource {
    pub data_type: String,
    pub format: String,
}

impl BinaryDataResource {
    pub fn new(data_type: impl Into<String>, format: impl Into<String>) -> Self {
        Self {
            data_type: data_type.into(),
            format: format.into(),
        }
    }

    pub async fn fetch_binary_data(&self) -> Result<Vec<ResourceContent>, McpError> {
        let (base64_data, mime_type) = match self.format.as_str() {
            "pdf" => ("JVBERi0xLjQKJcfs9z8KMSAwIG9iago8PAovVHlwZSAvQ2F0YWxvZwovUGFnZXMgMiAwIFIKPj4KZW5kb2JqCjIgMCBvYmoKPDwKL1R5cGUgL1BhZ2VzCi9LaWRzIFszIDAgUl0KL0NvdW50IDEKPD4KZW5kb2JqCjMgMCBvYmoKPDwKL1R5cGUgL1BhZ2UKL1BhcmVudCAyIDAgUgovTWVkaWFCb3ggWzAgMCA2MTIgNzkyXQo+PgplbmRvYmoKeHJlZgowIDQKMDAwMDAwMDAwMCA2NTUzNSBmCjAwMDAwMDAwMDkgMDAwMDAgbgowMDAwMDAwMDc0IDAwMDAwIG4KMDAwMDAwMDEyMCAwMDAwMCBuCnRyYWlsZXIKPDwKL1NpemUgNAovUm9vdCAxIDAgUgo+PgpzdGFydHhyZWYKMTc5CiUlRU9G", "application/pdf"),
            "zip" => ("UEsDBAoAAAAAAIdF11QAAAAAAAAAAAAAAAAHAAAAdGVzdC50eHRQSwECFAAKAAAAAACHRddUAAAAAAAAAAAAAAAABwAAAAAAAAAAACAAAAAAAAAAdGVzdC50eHRQSwUGAAAAAAEAAQA1AAAAHwAAAAAA", "application/zip"),
            "png" => ("iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAYAAABytg0kAAAAEklEQVR42mNk+M9QwwAGIxFYAA6gAhHlqe7fAAAAAElFTkSuQmCC", "image/png"),
            _ => ("VGhpcyBpcyBhIHRlc3QgYmluYXJ5IGZpbGU=", "application/octet-stream")
        };

        Ok(vec![ResourceContent::blob(
            &format!("file:///data/{}.{}", self.data_type, self.format),
            base64_data.to_string(),
            mime_type,
        )])
    }
}

// Note: McpResource trait is automatically implemented by the derive macro

// Test modules
#[cfg(test)]
pub mod tests {
    pub mod mcp_resources_json_rpc_simplified;
    pub mod mcp_resources_sse_notifications;
}
