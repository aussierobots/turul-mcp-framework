//! # Manual Tools Server Example
//!
//! This example demonstrates advanced manual implementation of MCP tools,
//! including session state management, progress notifications, and complex schemas.

use std::collections::HashMap;

use async_trait::async_trait;
use chrono::Utc;
use serde_json::{Value, json};
use turul_mcp_protocol::tools::CallToolResult;
use turul_mcp_protocol::{McpError, McpResult, ToolResult, ToolSchema, schema::JsonSchema};
use turul_mcp_builders::prelude::*;  // HasBaseMetadata, HasDescription, etc.
use turul_mcp_server::{McpServer, McpTool, SessionContext};
use uuid::Uuid;

/// File system tool that demonstrates complex schemas and state management
struct FileSystemTool {
    input_schema: ToolSchema,
}

impl FileSystemTool {
    fn new() -> Self {
        let input_schema = ToolSchema::object()
            .with_properties(HashMap::from([
                (
                    "operation".to_string(),
                    JsonSchema::string_enum(vec![
                        "create".to_string(),
                        "read".to_string(),
                        "update".to_string(),
                        "delete".to_string(),
                        "list".to_string(),
                    ])
                    .with_description("File operation to perform"),
                ),
                (
                    "path".to_string(),
                    JsonSchema::string().with_description("File path"),
                ),
                (
                    "content".to_string(),
                    JsonSchema::string()
                        .with_description("File content (for create/update operations)"),
                ),
            ]))
            .with_required(vec!["operation".to_string(), "path".to_string()]);
        Self { input_schema }
    }
}

// Implement fine-grained traits
impl HasBaseMetadata for FileSystemTool {
    fn name(&self) -> &str {
        "file_operations"
    }
}

impl HasDescription for FileSystemTool {
    fn description(&self) -> Option<&str> {
        Some("Simulated file system operations with session state tracking")
    }
}

impl HasInputSchema for FileSystemTool {
    fn input_schema(&self) -> &ToolSchema {
        &self.input_schema
    }
}

impl HasOutputSchema for FileSystemTool {
    fn output_schema(&self) -> Option<&ToolSchema> {
        None
    }
}

impl HasAnnotations for FileSystemTool {
    fn annotations(&self) -> Option<&turul_mcp_protocol::tools::ToolAnnotations> {
        None
    }
}

impl HasToolMeta for FileSystemTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
        None
    }
}

#[async_trait]
impl McpTool for FileSystemTool {
    async fn call(
        &self,
        args: Value,
        session: Option<SessionContext>,
    ) -> McpResult<CallToolResult> {
        let operation = args
            .get("operation")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("operation"))?;

        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("path"))?;

        let session = session.unwrap_or_else(|| panic!("Session required"));

        // Get or create file system state
        let mut files: HashMap<String, String> = session
            .get_typed_state("virtual_files")
            .await
            .unwrap_or_default();

        let result = match operation {
            "create" => {
                let content = args
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::missing_param("content"))?;

                if files.contains_key(path) {
                    return Err(McpError::tool_execution(&format!(
                        "File '{}' already exists",
                        path
                    )));
                }

                files.insert(path.to_string(), content.to_string());
                session
                    .set_typed_state("virtual_files", &files)
                    .await
                    .unwrap();
                session.notify_progress(format!("create_{}", path), 1).await;

                json!({
                    "operation": "create",
                    "path": path,
                    "size": content.len(),
                    "created": Utc::now().to_rfc3339(),
                    "message": format!("Created file '{}'", path)
                })
            }
            "read" => {
                if let Some(content) = files.get(path) {
                    json!({
                        "operation": "read",
                        "path": path,
                        "content": content,
                        "size": content.len(),
                        "message": format!("Read file '{}'", path)
                    })
                } else {
                    return Err(McpError::tool_execution(&format!(
                        "File '{}' not found",
                        path
                    )));
                }
            }
            "update" => {
                let content = args
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::missing_param("content"))?;

                if !files.contains_key(path) {
                    return Err(McpError::tool_execution(&format!(
                        "File '{}' not found",
                        path
                    )));
                }

                files.insert(path.to_string(), content.to_string());
                session
                    .set_typed_state("virtual_files", &files)
                    .await
                    .unwrap();
                session.notify_progress(format!("update_{}", path), 1).await;

                json!({
                    "operation": "update",
                    "path": path,
                    "size": content.len(),
                    "updated": Utc::now().to_rfc3339(),
                    "message": format!("Updated file '{}'", path)
                })
            }
            "delete" => {
                if files.remove(path).is_some() {
                    session
                        .set_typed_state("virtual_files", &files)
                        .await
                        .unwrap();
                    session.notify_progress(format!("delete_{}", path), 1).await;

                    json!({
                        "operation": "delete",
                        "path": path,
                        "deleted": Utc::now().to_rfc3339(),
                        "message": format!("Deleted file '{}'", path)
                    })
                } else {
                    return Err(McpError::tool_execution(&format!(
                        "File '{}' not found",
                        path
                    )));
                }
            }
            "list" => {
                let file_list: Vec<Value> = files
                    .iter()
                    .map(|(file_path, content)| {
                        json!({
                            "path": file_path,
                            "size": content.len()
                        })
                    })
                    .collect();

                json!({
                    "operation": "list",
                    "files": file_list,
                    "total_files": files.len(),
                    "message": format!("Listed {} files", files.len())
                })
            }
            _ => {
                return Err(McpError::invalid_param_type(
                    "operation",
                    "create|read|update|delete|list",
                    operation,
                ));
            }
        };

        Ok(CallToolResult {
            content: vec![ToolResult::text(result.to_string())],
            is_error: None,
            structured_content: None,
            meta: None,
        })
    }
}

/// Task manager tool with complex state management and notifications
struct TaskManagerTool {
    input_schema: ToolSchema,
}

impl TaskManagerTool {
    fn new() -> Self {
        let input_schema = ToolSchema::object()
            .with_properties(HashMap::from([
                (
                    "action".to_string(),
                    JsonSchema::string_enum(vec![
                        "create".to_string(),
                        "list".to_string(),
                        "complete".to_string(),
                        "delete".to_string(),
                        "update_status".to_string(),
                    ])
                    .with_description("Task action to perform"),
                ),
                (
                    "title".to_string(),
                    JsonSchema::string().with_description("Task title (required for create)"),
                ),
                (
                    "description".to_string(),
                    JsonSchema::string().with_description("Task description"),
                ),
                (
                    "task_id".to_string(),
                    JsonSchema::string()
                        .with_description("Task ID (required for complete/delete/update_status)"),
                ),
                (
                    "status".to_string(),
                    JsonSchema::string_enum(vec![
                        "todo".to_string(),
                        "in_progress".to_string(),
                        "completed".to_string(),
                    ])
                    .with_description("Task status (for update_status)"),
                ),
                (
                    "priority".to_string(),
                    JsonSchema::string_enum(vec![
                        "low".to_string(),
                        "medium".to_string(),
                        "high".to_string(),
                    ])
                    .with_description("Task priority"),
                ),
            ]))
            .with_required(vec!["action".to_string()]);
        Self { input_schema }
    }
}

impl HasBaseMetadata for TaskManagerTool {
    fn name(&self) -> &str {
        "task_manager"
    }
}

impl HasDescription for TaskManagerTool {
    fn description(&self) -> Option<&str> {
        Some("Manage tasks with status tracking and progress notifications")
    }
}

impl HasInputSchema for TaskManagerTool {
    fn input_schema(&self) -> &ToolSchema {
        &self.input_schema
    }
}

impl HasOutputSchema for TaskManagerTool {
    fn output_schema(&self) -> Option<&ToolSchema> {
        None
    }
}

impl HasAnnotations for TaskManagerTool {
    fn annotations(&self) -> Option<&turul_mcp_protocol::tools::ToolAnnotations> {
        None
    }
}

impl HasToolMeta for TaskManagerTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
        None
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct Task {
    id: String,
    title: String,
    description: Option<String>,
    status: String,
    priority: String,
    created: String,
    updated: String,
}

#[async_trait]
impl McpTool for TaskManagerTool {
    async fn call(
        &self,
        args: Value,
        session: Option<SessionContext>,
    ) -> McpResult<CallToolResult> {
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("action"))?;

        let session = session.unwrap_or_else(|| panic!("Session required"));

        // Get or create tasks state
        let mut tasks: HashMap<String, Task> =
            session.get_typed_state("tasks").await.unwrap_or_default();

        let result = match action {
            "create" => {
                let title = args
                    .get("title")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::missing_param("title"))?;

                let description = args
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let priority = args
                    .get("priority")
                    .and_then(|v| v.as_str())
                    .unwrap_or("medium");

                let task_id = Uuid::new_v4().to_string();
                let now = Utc::now().to_rfc3339();

                let task = Task {
                    id: task_id.clone(),
                    title: title.to_string(),
                    description,
                    status: "todo".to_string(),
                    priority: priority.to_string(),
                    created: now.clone(),
                    updated: now,
                };

                tasks.insert(task_id.clone(), task.clone());
                session.set_typed_state("tasks", &tasks).await.unwrap();
                session
                    .notify_progress(format!("task_created_{}", task_id), 1)
                    .await;

                json!({
                    "action": "create",
                    "task": task,
                    "message": format!("Created task '{}'", title)
                })
            }
            "list" => {
                let task_list: Vec<&Task> = tasks.values().collect();

                json!({
                    "action": "list",
                    "tasks": task_list,
                    "total_tasks": tasks.len(),
                    "message": format!("Listed {} tasks", tasks.len())
                })
            }
            "complete" => {
                let task_id = args
                    .get("task_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::missing_param("task_id"))?;

                if let Some(task) = tasks.get_mut(task_id) {
                    task.status = "completed".to_string();
                    task.updated = Utc::now().to_rfc3339();
                    let task_clone = task.clone();
                    session.set_typed_state("tasks", &tasks).await.unwrap();
                    session
                        .notify_progress(format!("task_completed_{}", task_id), 1)
                        .await;

                    json!({
                        "action": "complete",
                        "task": task_clone,
                        "message": format!("Completed task '{}'", task_clone.title)
                    })
                } else {
                    return Err(McpError::tool_execution(&format!(
                        "Task '{}' not found",
                        task_id
                    )));
                }
            }
            "delete" => {
                let task_id = args
                    .get("task_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::missing_param("task_id"))?;

                if let Some(task) = tasks.remove(task_id) {
                    session.set_typed_state("tasks", &tasks).await.unwrap();
                    session
                        .notify_progress(format!("task_deleted_{}", task_id), 1)
                        .await;

                    json!({
                        "action": "delete",
                        "deleted_task": task,
                        "message": format!("Deleted task '{}'", task.title)
                    })
                } else {
                    return Err(McpError::tool_execution(&format!(
                        "Task '{}' not found",
                        task_id
                    )));
                }
            }
            "update_status" => {
                let task_id = args
                    .get("task_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::missing_param("task_id"))?;
                let status = args
                    .get("status")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::missing_param("status"))?;

                if !["todo", "in_progress", "completed"].contains(&status) {
                    return Err(McpError::invalid_param_type(
                        "status",
                        "todo|in_progress|completed",
                        status,
                    ));
                }

                if let Some(task) = tasks.get_mut(task_id) {
                    task.status = status.to_string();
                    task.updated = Utc::now().to_rfc3339();
                    let task_clone = task.clone();
                    session.set_typed_state("tasks", &tasks).await.unwrap();
                    session
                        .notify_progress(format!("task_status_updated_{}", task_id), 1)
                        .await;

                    json!({
                        "action": "update_status",
                        "task": task_clone,
                        "message": format!("Updated task '{}' status to '{}'", task_clone.title, status)
                    })
                } else {
                    return Err(McpError::tool_execution(&format!(
                        "Task '{}' not found",
                        task_id
                    )));
                }
            }
            _ => {
                return Err(McpError::invalid_param_type(
                    "action",
                    "create|list|complete|delete|update_status",
                    action,
                ));
            }
        };

        Ok(CallToolResult {
            content: vec![ToolResult::text(result.to_string())],
            is_error: None,
            structured_content: None,
            meta: None,
        })
    }
}

/// Weather tool demonstrating simple API-style operations
struct WeatherTool {
    input_schema: ToolSchema,
}

impl WeatherTool {
    fn new() -> Self {
        let input_schema = ToolSchema::object()
            .with_properties(HashMap::from([
                (
                    "location".to_string(),
                    JsonSchema::string().with_description("Location to get weather for"),
                ),
                (
                    "units".to_string(),
                    JsonSchema::string_enum(vec!["celsius".to_string(), "fahrenheit".to_string()])
                        .with_description("Temperature units (default: celsius)"),
                ),
            ]))
            .with_required(vec!["location".to_string()]);
        Self { input_schema }
    }
}

impl HasBaseMetadata for WeatherTool {
    fn name(&self) -> &str {
        "weather"
    }
}

impl HasDescription for WeatherTool {
    fn description(&self) -> Option<&str> {
        Some("Get weather information with session-based caching")
    }
}

impl HasInputSchema for WeatherTool {
    fn input_schema(&self) -> &ToolSchema {
        &self.input_schema
    }
}

impl HasOutputSchema for WeatherTool {
    fn output_schema(&self) -> Option<&ToolSchema> {
        None
    }
}

impl HasAnnotations for WeatherTool {
    fn annotations(&self) -> Option<&turul_mcp_protocol::tools::ToolAnnotations> {
        None
    }
}

impl HasToolMeta for WeatherTool {
    fn tool_meta(&self) -> Option<&HashMap<String, Value>> {
        None
    }
}

#[async_trait]
impl McpTool for WeatherTool {
    async fn call(
        &self,
        args: Value,
        session: Option<SessionContext>,
    ) -> McpResult<CallToolResult> {
        let location = args
            .get("location")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("location"))?;

        let units = args
            .get("units")
            .and_then(|v| v.as_str())
            .unwrap_or("celsius");

        // Simulate weather data
        let temp_celsius = match location.to_lowercase().as_str() {
            "london" => 15,
            "new york" => 22,
            "tokyo" => 25,
            "sydney" => 20,
            "paris" => 18,
            _ => 20, // default
        };

        let (temp, temp_unit) = match units {
            "fahrenheit" => (temp_celsius * 9 / 5 + 32, "°F"),
            _ => (temp_celsius, "°C"),
        };

        let weather_data = json!({
            "location": location,
            "temperature": temp,
            "units": temp_unit,
            "condition": "Partly cloudy",
            "humidity": 65,
            "wind_speed": 12,
            "timestamp": Utc::now().to_rfc3339(),
            "cached": false
        });

        // Cache in session if available
        if let Some(session) = session {
            let cache_key = format!("weather_{}", location.to_lowercase().replace(" ", "_"));
            session
                .set_typed_state(&cache_key, &weather_data)
                .await
                .unwrap();
            session
                .notify_progress(format!("weather_fetched_{}", location), 1)
                .await;
        }

        Ok(CallToolResult {
            content: vec![ToolResult::text(weather_data.to_string())],
            is_error: None,
            structured_content: None,
            meta: None,
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("🔧 Starting Manual Tools MCP Server");
    println!("🎯 Demonstrating advanced manual tool implementations");

    let server = McpServer::builder()
        .name("manual-tools-server")
        .version("1.0.0")
        .title("Manual Tools Implementation Example")
        .instructions("This server demonstrates advanced manual implementation of MCP tools with fine-grained trait composition, session state management, progress notifications, and complex schemas.")
        .tool(FileSystemTool::new())
        .tool(TaskManagerTool::new())
        .tool(WeatherTool::new())
        .bind_address("127.0.0.1:8007".parse()?)
        .sse(true)
        .build()?;

    println!("🚀 Manual tools server running at: http://127.0.0.1:8007/mcp");
    println!("\n📋 Available tools:");
    println!("  📁 file_operations: Simulated file system with CRUD operations");
    println!("  ✅ task_manager: Task management with status tracking and priorities");
    println!("  🌤️  weather: Weather information with session caching");
    println!("\n✨ Advanced features demonstrated:");
    println!("  🔧 Manual fine-grained trait implementations");
    println!("  💾 Session-based state persistence");
    println!("  📊 Progress notifications and status updates");
    println!("  📝 Complex JSON schemas with enums and validation");
    println!("  🧩 Trait composition patterns");

    println!("\n🎯 Example usage:");
    println!(
        "  1. Create file: file_operations(operation='create', path='/hello.txt', content='Hello World')"
    );
    println!("  2. Create task: task_manager(action='create', title='Learn MCP', priority='high')");
    println!("  3. Get weather: weather(location='London', units='celsius')");

    server.run().await?;
    Ok(())
}
