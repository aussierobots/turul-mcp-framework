//! # Manual Tools Server Example
//!
//! This example demonstrates MCP tools with session state management,
//! progress notifications, and complex schemas using `#[derive(McpTool)]` macros.

use std::collections::HashMap;

use chrono::Utc;
use serde::Deserialize;
use serde_json::{Value, json};
use turul_mcp_derive::McpTool;
use turul_mcp_server::prelude::*;
use uuid::Uuid;

/// File system tool that demonstrates complex schemas and state management
#[derive(McpTool, Clone, Default, Deserialize)]
#[tool(
    name = "file_operations",
    description = "Simulated file system operations with session state tracking"
)]
pub struct FileSystemTool {
    #[param(description = "File operation to perform (create, read, update, delete, list)")]
    pub operation: String,
    #[param(description = "File path")]
    pub path: String,
    #[param(description = "File content (for create/update operations)", optional)]
    pub content: Option<String>,
}

impl FileSystemTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<Value> {
        let session = session.ok_or_else(|| McpError::tool_execution("Session required"))?;

        // Get or create file system state
        let mut files: HashMap<String, String> = session
            .get_typed_state("virtual_files")
            .await
            .unwrap_or_default();

        let result = match self.operation.as_str() {
            "create" => {
                let content = self
                    .content
                    .as_deref()
                    .ok_or_else(|| McpError::missing_param("content"))?;

                if files.contains_key(&self.path) {
                    return Err(McpError::tool_execution(&format!(
                        "File '{}' already exists",
                        self.path
                    )));
                }

                files.insert(self.path.clone(), content.to_string());
                session
                    .set_typed_state("virtual_files", &files)
                    .await
                    .unwrap();
                session.notify_progress(format!("create_{}", self.path), 1).await;

                json!({
                    "operation": "create",
                    "path": self.path,
                    "size": content.len(),
                    "created": Utc::now().to_rfc3339(),
                    "message": format!("Created file '{}'", self.path)
                })
            }
            "read" => {
                if let Some(content) = files.get(&self.path) {
                    json!({
                        "operation": "read",
                        "path": self.path,
                        "content": content,
                        "size": content.len(),
                        "message": format!("Read file '{}'", self.path)
                    })
                } else {
                    return Err(McpError::tool_execution(&format!(
                        "File '{}' not found",
                        self.path
                    )));
                }
            }
            "update" => {
                let content = self
                    .content
                    .as_deref()
                    .ok_or_else(|| McpError::missing_param("content"))?;

                if !files.contains_key(&self.path) {
                    return Err(McpError::tool_execution(&format!(
                        "File '{}' not found",
                        self.path
                    )));
                }

                files.insert(self.path.clone(), content.to_string());
                session
                    .set_typed_state("virtual_files", &files)
                    .await
                    .unwrap();
                session.notify_progress(format!("update_{}", self.path), 1).await;

                json!({
                    "operation": "update",
                    "path": self.path,
                    "size": content.len(),
                    "updated": Utc::now().to_rfc3339(),
                    "message": format!("Updated file '{}'", self.path)
                })
            }
            "delete" => {
                if files.remove(&self.path).is_some() {
                    session
                        .set_typed_state("virtual_files", &files)
                        .await
                        .unwrap();
                    session.notify_progress(format!("delete_{}", self.path), 1).await;

                    json!({
                        "operation": "delete",
                        "path": self.path,
                        "deleted": Utc::now().to_rfc3339(),
                        "message": format!("Deleted file '{}'", self.path)
                    })
                } else {
                    return Err(McpError::tool_execution(&format!(
                        "File '{}' not found",
                        self.path
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
                    &self.operation,
                ));
            }
        };

        Ok(result)
    }
}

/// Task manager tool with complex state management and notifications
#[derive(McpTool, Clone, Default, Deserialize)]
#[tool(
    name = "task_manager",
    description = "Manage tasks with status tracking and progress notifications"
)]
pub struct TaskManagerTool {
    #[param(description = "Task action to perform (create, list, complete, delete, update_status)")]
    pub action: String,
    #[param(description = "Task title (required for create)", optional)]
    pub title: Option<String>,
    #[param(description = "Task description", optional)]
    pub description: Option<String>,
    #[param(description = "Task ID (required for complete/delete/update_status)", optional)]
    pub task_id: Option<String>,
    #[param(description = "Task status: todo, in_progress, completed (for update_status)", optional)]
    pub status: Option<String>,
    #[param(description = "Task priority: low, medium, high", optional)]
    pub priority: Option<String>,
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

impl TaskManagerTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<Value> {
        let session = session.ok_or_else(|| McpError::tool_execution("Session required"))?;

        // Get or create tasks state
        let mut tasks: HashMap<String, Task> =
            session.get_typed_state("tasks").await.unwrap_or_default();

        let result = match self.action.as_str() {
            "create" => {
                let title = self
                    .title
                    .as_deref()
                    .ok_or_else(|| McpError::missing_param("title"))?;

                let description = self.description.clone();

                let priority = self.priority.as_deref().unwrap_or("medium");

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
                let task_id = self
                    .task_id
                    .as_deref()
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
                let task_id = self
                    .task_id
                    .as_deref()
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
                let task_id = self
                    .task_id
                    .as_deref()
                    .ok_or_else(|| McpError::missing_param("task_id"))?;
                let status = self
                    .status
                    .as_deref()
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
                    &self.action,
                ));
            }
        };

        Ok(result)
    }
}

/// Weather tool demonstrating simple API-style operations
#[derive(McpTool, Clone, Default, Deserialize)]
#[tool(
    name = "weather",
    description = "Get weather information with session-based caching"
)]
pub struct WeatherTool {
    #[param(description = "Location to get weather for")]
    pub location: String,
    #[param(description = "Temperature units: celsius or fahrenheit (default: celsius)", optional)]
    pub units: Option<String>,
}

impl WeatherTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<Value> {
        let units = self.units.as_deref().unwrap_or("celsius");

        // Simulate weather data
        let temp_celsius = match self.location.to_lowercase().as_str() {
            "london" => 15,
            "new york" => 22,
            "tokyo" => 25,
            "sydney" => 20,
            "paris" => 18,
            _ => 20, // default
        };

        let (temp, temp_unit) = match units {
            "fahrenheit" => (temp_celsius * 9 / 5 + 32, "F"),
            _ => (temp_celsius, "C"),
        };

        let weather_data = json!({
            "location": self.location,
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
            let cache_key = format!("weather_{}", self.location.to_lowercase().replace(' ', "_"));
            session
                .set_typed_state(&cache_key, &weather_data)
                .await
                .unwrap();
            session
                .notify_progress(format!("weather_fetched_{}", self.location), 1)
                .await;
        }

        Ok(weather_data)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("Starting Manual Tools MCP Server");
    println!("Demonstrating derive macro tools with session state and progress notifications");

    let server = McpServer::builder()
        .name("manual-tools-server")
        .version("1.0.0")
        .title("Manual Tools Implementation Example")
        .instructions("This server demonstrates MCP tools with session state management, progress notifications, and complex schemas using derive macros.")
        .tool(FileSystemTool::default())
        .tool(TaskManagerTool::default())
        .tool(WeatherTool::default())
        .bind_address("127.0.0.1:8007".parse()?)
        .sse(true)
        .build()?;

    println!("Manual tools server running at: http://127.0.0.1:8007/mcp");
    println!("\nAvailable tools:");
    println!("  file_operations: Simulated file system with CRUD operations");
    println!("  task_manager: Task management with status tracking and priorities");
    println!("  weather: Weather information with session caching");
    println!("\nFeatures demonstrated:");
    println!("  - Session-based state persistence");
    println!("  - Progress notifications and status updates");
    println!("  - Parameter validation with McpError types");

    println!("\nExample usage:");
    println!(
        "  1. Create file: file_operations(operation='create', path='/hello.txt', content='Hello World')"
    );
    println!("  2. Create task: task_manager(action='create', title='Learn MCP', priority='high')");
    println!("  3. Get weather: weather(location='London', units='celsius')");

    server.run().await?;
    Ok(())
}
