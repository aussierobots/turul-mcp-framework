//! # Manual Tools Server Example
//!
//! This example demonstrates advanced manual implementation of MCP tools,
//! including session state management, progress notifications, and complex schemas.

use std::collections::HashMap;

use async_trait::async_trait;
use chrono::Utc;
use mcp_server::{McpServer, McpTool, SessionContext};
use mcp_protocol::{ToolSchema, ToolResult, schema::JsonSchema, McpError, McpResult};
use serde_json::{Value, json};
use uuid::Uuid;

/// File system tool that demonstrates complex schemas and state management
struct FileSystemTool;

#[async_trait]
impl McpTool for FileSystemTool {
    fn name(&self) -> &str {
        "file_operations"
    }

    fn description(&self) -> &str {
        "Simulated file system operations with session state tracking"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("operation".to_string(), JsonSchema::string_enum(vec![
                    "create".to_string(), "read".to_string(), "update".to_string(), 
                    "delete".to_string(), "list".to_string()
                ]).with_description("File operation to perform")),
                ("path".to_string(), JsonSchema::string()
                    .with_description("File path")),
                ("content".to_string(), JsonSchema::string()
                    .with_description("File content (for create/update operations)")),
            ]))
            .with_required(vec!["operation".to_string(), "path".to_string()])
    }

    async fn call(&self, args: Value, session: Option<SessionContext>) -> McpResult<Vec<ToolResult>> {
        let operation = args.get("operation")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("operation"))?;
        
        let path = args.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("path"))?;

        if let Some(ref session) = session {
            // Track operation in session state
            let operation_id = Uuid::new_v4().to_string();
            session.set_typed_state("last_operation_id", &operation_id)
                .map_err(|e| McpError::tool_execution(&format!("Failed to set session state: {}", e)))?;
            session.notify_log("info", format!("Starting {} operation on {}", operation, path));
            session.notify_progress(&operation_id, 10);
        }

        // Simulate processing time
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let result = match operation {
            "create" => {
                let content = args.get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                
                if let Some(ref session) = session {
                    session.notify_progress("file_create", 50);
                    // Store file list in session
                    let mut files: Vec<String> = session.get_typed_state("file_list")
                        .unwrap_or_else(|| vec![]);
                    files.push(path.to_string());
                    session.set_typed_state("file_list", &files)
                        .map_err(|e| McpError::tool_execution(&format!("Failed to update file list: {}", e)))?;
                }

                ToolResult::text(format!("Created file {} with {} bytes", path, content.len()))
            }
            "read" => {
                if let Some(ref session) = session {
                    session.notify_progress("file_read", 70);
                }
                ToolResult::text(format!("Reading file {}: [simulated content]", path))
            }
            "list" => {
                if let Some(ref session) = session {
                    let files: Vec<String> = session.get_typed_state("file_list")
                        .unwrap_or_else(|| vec!["example.txt".to_string(), "data.json".to_string()]);
                    
                    session.notify_progress("file_list", 90);
                    
                    ToolResult::text(json!({
                        "files": files,
                        "timestamp": Utc::now().to_rfc3339(),
                        "session_id": session.session_id
                    }).to_string())
                } else {
                    ToolResult::text(json!({
                        "files": ["example.txt", "data.json"],
                        "timestamp": Utc::now().to_rfc3339(),
                        "session_id": null
                    }).to_string())
                }
            }
            "update" => {
                let content = args.get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                ToolResult::text(format!("Updated file {} with {} bytes", path, content.len()))
            }
            "delete" => {
                if let Some(ref session) = session {
                    let mut files: Vec<String> = session.get_typed_state("file_list")
                        .unwrap_or_else(|| vec![]);
                    files.retain(|f| f != path);
                    session.set_typed_state("file_list", &files)
                        .map_err(|e| McpError::tool_execution(&format!("Failed to update file list: {}", e)))?;
                }
                ToolResult::text(format!("Deleted file {}", path))
            }
            _ => return Err(McpError::invalid_param_type("operation", "create|read|update|delete|list", operation))
        };

        if let Some(ref session) = session {
            let operation_id: String = session.get_typed_state("last_operation_id")
                .unwrap_or_else(|| "unknown".to_string());
            session.notify_progress_with_total(&operation_id, 100, 100);
            session.notify_log("info", format!("Completed {} operation", operation));
        }

        Ok(vec![result])
    }
}

/// Task management tool demonstrating complex business logic
struct TaskManagerTool;

#[async_trait]
impl McpTool for TaskManagerTool {
    fn name(&self) -> &str {
        "task_manager"
    }

    fn description(&self) -> &str {
        "Manage tasks with priorities, due dates, and session persistence"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("action".to_string(), JsonSchema::string_enum(vec![
                    "create".to_string(), "list".to_string(), "complete".to_string(), "update".to_string()
                ]).with_description("Task management action")),
                ("title".to_string(), JsonSchema::string()
                    .with_description("Task title")),
                ("priority".to_string(), JsonSchema::string_enum(vec![
                    "low".to_string(), "medium".to_string(), "high".to_string(), "urgent".to_string()
                ]).with_description("Task priority level")),
                ("due_date".to_string(), JsonSchema::string()
                    .with_description("Due date in ISO 8601 format")),
                ("task_id".to_string(), JsonSchema::string()
                    .with_description("Task ID for update/complete operations")),
            ]))
            .with_required(vec!["action".to_string()])
    }

    async fn call(&self, args: Value, session: Option<SessionContext>) -> McpResult<Vec<ToolResult>> {
        let action = args.get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("action"))?;

        if let Some(ref session) = session {
            session.notify_log("info", format!("Processing task {} action", action));
        }

        match action {
            "create" => {
                let title = args.get("title")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::missing_param("title"))?;
                
                let priority = args.get("priority")
                    .and_then(|v| v.as_str())
                    .unwrap_or("medium");

                let task_id = Uuid::new_v4().to_string();
                let task = json!({
                    "id": task_id,
                    "title": title,
                    "priority": priority,
                    "status": "pending",
                    "created": Utc::now().to_rfc3339(),
                    "due_date": args.get("due_date")
                });

                if let Some(ref session) = session {
                    let mut tasks: Vec<Value> = session.get_typed_state("tasks")
                        .unwrap_or_else(|| vec![]);
                    tasks.push(task.clone());
                    session.set_typed_state("tasks", &tasks)
                        .map_err(|e| McpError::tool_execution(&format!("Failed to save tasks: {}", e)))?;
                    session.notify_log("info", format!("Created task: {}", title));
                }

                Ok(vec![ToolResult::text(format!("Created task: {}", task))])
            }
            "list" => {
                if let Some(ref session) = session {
                    let tasks: Vec<Value> = session.get_typed_state("tasks")
                        .unwrap_or_else(|| vec![]);
                    
                    let summary = json!({
                        "total_tasks": tasks.len(),
                        "tasks": tasks,
                        "session_id": session.session_id
                    });

                    Ok(vec![ToolResult::text(summary.to_string())])
                } else {
                    Ok(vec![ToolResult::text("No session available - cannot persist tasks")])
                }
            }
            "complete" => {
                let task_id = args.get("task_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::missing_param("task_id"))?;

                if let Some(ref session) = session {
                    let mut tasks: Vec<Value> = session.get_typed_state("tasks")
                        .unwrap_or_else(|| vec![]);
                    
                    let mut found = false;
                    for task in &mut tasks {
                        if task["id"] == task_id {
                            task["status"] = json!("completed");
                            task["completed"] = json!(Utc::now().to_rfc3339());
                            found = true;
                            break;
                        }
                    }

                    if found {
                        session.set_typed_state("tasks", &tasks)
                            .map_err(|e| McpError::tool_execution(&format!("Failed to update tasks: {}", e)))?;
                        session.notify_log("info", format!("Completed task: {}", task_id));
                        Ok(vec![ToolResult::text(format!("Task {} marked as completed", task_id))])
                    } else {
                        Err(McpError::tool_execution(&format!("Task {} not found", task_id)))
                    }
                } else {
                    Err(McpError::tool_execution("No session available - cannot access tasks"))
                }
            }
            _ => Err(McpError::invalid_param_type("action", "create|list|complete|update", action))
        }
    }
}

/// Weather tool demonstrating external API simulation with caching
struct WeatherTool;

#[async_trait]
impl McpTool for WeatherTool {
    fn name(&self) -> &str {
        "weather"
    }

    fn description(&self) -> &str {
        "Get weather information with session-based caching"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("location".to_string(), JsonSchema::string()
                    .with_description("City name or coordinates")),
                ("units".to_string(), JsonSchema::string_enum(vec![
                    "celsius".to_string(), "fahrenheit".to_string()
                ]).with_description("Temperature units")),
            ]))
            .with_required(vec!["location".to_string()])
    }

    async fn call(&self, args: Value, session: Option<SessionContext>) -> McpResult<Vec<ToolResult>> {
        let location = args.get("location")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("location"))?;
        
        let units = args.get("units")
            .and_then(|v| v.as_str())
            .unwrap_or("celsius");

        if let Some(ref session) = session {
            session.notify_progress("weather_fetch", 20);
            
            // Check cache
            let cache_key = format!("weather_{}_{}", location, units);
            if let Some(cached) = session.get_typed_state::<Value>(&cache_key) {
                session.notify_log("info", "Returning cached weather data");
                return Ok(vec![ToolResult::text(cached.to_string())]);
            }
            
            session.notify_progress("weather_fetch", 60);
        }

        // Simulate API call
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let weather_data = json!({
            "location": location,
            "temperature": if units == "fahrenheit" { 72 } else { 22 },
            "units": units,
            "condition": "Partly cloudy",
            "humidity": 65,
            "wind_speed": 8,
            "timestamp": Utc::now().to_rfc3339(),
            "cached": false
        });

        if let Some(ref session) = session {
            // Cache the result
            let cache_key = format!("weather_{}_{}", location, units);
            session.set_typed_state(&cache_key, &weather_data)
                .map_err(|e| McpError::tool_execution(&format!("Failed to cache weather data: {}", e)))?;
            session.notify_progress_with_total("weather_fetch", 100, 100);
            session.notify_log("info", format!("Fetched and cached weather for {}", location));
        }

        Ok(vec![ToolResult::text(weather_data.to_string())])
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("Starting Manual Tools MCP Server");

    let server = McpServer::builder()
        .name("manual-tools-server")
        .version("1.0.0")
        .title("Manual Tools Example Server")
        .instructions("This server demonstrates advanced manual tool implementations with session state management, progress notifications, and complex business logic.")
        .tool(FileSystemTool)
        .tool(TaskManagerTool) 
        .tool(WeatherTool)
        .bind_address("127.0.0.1:8001".parse()?)
        .build()?;

    println!("Manual Tools server running at: http://127.0.0.1:8001/mcp");
    println!("Available tools:");
    println!("  - file_operations: Simulated file system with session state");
    println!("  - task_manager: Task management with persistence");
    println!("  - weather: Weather API with session caching");

    server.run().await?;
    Ok(())
}