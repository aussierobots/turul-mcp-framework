//! # SQLite Pagination Server Example
//!
//! This example demonstrates comprehensive MCP pagination functionality using SQLite database
//! for realistic large dataset handling. It shows proper database pagination patterns,
//! connection management, and setup/teardown lifecycle.

use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use mcp_server::{McpServer, McpTool, SessionContext};
use mcp_protocol::{ToolSchema, ToolResult, McpError, McpResult};
use mcp_protocol::schema::JsonSchema;
use mcp_protocol::meta::{Meta, Cursor};
use serde_json::{json, Value};
use tracing::{info, error};
use chrono::{DateTime, Utc};
use rand::Rng;
use sqlx::{SqlitePool, Row, query_as};
use tempfile::TempDir;

/// Database pool wrapper for sharing across tools
#[derive(Clone)]
struct DatabaseManager {
    pool: SqlitePool,
    _temp_dir: Arc<TempDir>, // Keep temp directory alive
}

impl DatabaseManager {
    /// Create a new database with sample data
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        info!("Setting up SQLite database with sample data...");
        
        // Create temporary directory for database file
        let temp_dir = tempfile::tempdir()?;
        let db_path = temp_dir.path().join("pagination_demo.db");
        
        // Create database connection
        let database_url = format!("sqlite:{}", db_path.display());
        let pool = SqlitePool::connect(&database_url).await?;
        
        // Create tables
        sqlx::query(r#"
            CREATE TABLE users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                email TEXT NOT NULL UNIQUE,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                is_active BOOLEAN NOT NULL DEFAULT 1,
                department TEXT,
                last_login DATETIME,
                profile_data TEXT
            )
        "#).execute(&pool).await?;

        // Create indexes for better pagination performance
        sqlx::query(r#"
            CREATE INDEX idx_users_created_at ON users(created_at);
            CREATE INDEX idx_users_is_active ON users(is_active);
            CREATE INDEX idx_users_department ON users(department);
            CREATE INDEX idx_users_name ON users(name);
        "#).execute(&pool).await?;

        let manager = Self {
            pool,
            _temp_dir: Arc::new(temp_dir),
        };

        // Populate with sample data
        manager.populate_sample_data().await?;
        
        Ok(manager)
    }

    /// Populate database with realistic sample data
    async fn populate_sample_data(&self) -> Result<(), sqlx::Error> {
        info!("Populating database with 10,000 sample users...");
        
        let first_names = [
            "Alice", "Bob", "Carol", "David", "Emma", "Frank", "Grace", "Henry", "Ivy", "Jack",
            "Kate", "Liam", "Maya", "Noah", "Olivia", "Paul", "Quinn", "Ruby", "Sam", "Tina",
            "Uma", "Victor", "Wendy", "Xavier", "Yuki", "Zoe", "Alex", "Blake", "Casey", "Drew"
        ];
        
        let last_names = [
            "Smith", "Johnson", "Williams", "Brown", "Jones", "Garcia", "Miller", "Davis", "Rodriguez", "Martinez",
            "Hernandez", "Lopez", "Gonzalez", "Wilson", "Anderson", "Thomas", "Taylor", "Moore", "Jackson", "Martin",
            "Lee", "Perez", "Thompson", "White", "Harris", "Sanchez", "Clark", "Ramirez", "Lewis", "Robinson"
        ];
        
        let departments = ["Engineering", "Marketing", "Sales", "HR", "Finance", "Operations", "Support", "Legal"];
        let domains = ["company.com", "corp.net", "business.org", "enterprise.co", "solutions.io"];
        
        // Insert in batches for better performance
        for batch_start in (0..10000).step_by(500) {
            let mut transaction = self.pool.begin().await?;
            
            for i in batch_start..std::cmp::min(batch_start + 500, 10000) {
                let first_name = first_names[i % first_names.len()];
                let last_name = last_names[(i / first_names.len()) % last_names.len()];
                let name = format!("{} {}", first_name, last_name);
                let email = format!("{}.{}{}@{}", 
                    first_name.to_lowercase(), 
                    last_name.to_lowercase(),
                    if i < 1000 { String::new() } else { format!("{}", i) }, // Add numbers for uniqueness
                    domains[i % domains.len()]
                );
                
                let is_active = rand::rng().random_bool(0.85); // 85% active
                let department = departments[i % departments.len()];
                
                // Random created_at within last 2 years
                let days_ago = rand::rng().random_range(1..730);
                let created_at = Utc::now() - chrono::Duration::days(days_ago);
                
                // Random last_login for active users
                let last_login = if is_active && rand::rng().random_bool(0.7) {
                    Some(Utc::now() - chrono::Duration::days(rand::rng().random_range(1..30)))
                } else {
                    None
                };
                
                let profile_data = json!({
                    "preferences": {
                        "theme": if rand::rng().random_bool(0.3) { "dark" } else { "light" },
                        "notifications": rand::rng().random_bool(0.8),
                        "language": if rand::rng().random_bool(0.9) { "en" } else { "es" }
                    },
                    "metadata": {
                        "employee_id": format!("EMP{:05}", i + 1),
                        "hire_date": created_at.format("%Y-%m-%d").to_string()
                    }
                }).to_string();
                
                sqlx::query(r#"
                    INSERT INTO users (name, email, created_at, is_active, department, last_login, profile_data)
                    VALUES (?, ?, ?, ?, ?, ?, ?)
                "#)
                .bind(&name)
                .bind(&email)
                .bind(&created_at)
                .bind(is_active)
                .bind(department)
                .bind(last_login)
                .bind(&profile_data)
                .execute(&mut *transaction)
                .await?;
            }
            
            transaction.commit().await?;
            
            if batch_start % 2500 == 0 {
                info!("Inserted {} users...", batch_start + 500);
            }
        }
        
        // Get final count
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?;
        
        info!("Database populated with {} users", count.0);
        Ok(())
    }

    /// Get paginated users with filtering
    async fn get_users_page(
        &self,
        cursor: Option<&str>,
        limit: i64,
        filter: Option<&str>,
        department: Option<&str>,
        active_only: bool,
    ) -> Result<(Vec<User>, Option<String>, i64), sqlx::Error> {
        // Build dynamic query without lifetime issues
        let mut where_conditions = Vec::new();
        
        if let Some(filter_text) = filter {
            let escaped_filter = filter_text.replace("'", "''"); // Basic SQL injection prevention
            where_conditions.push(format!("(name LIKE '%{}%' OR email LIKE '%{}%')", escaped_filter, escaped_filter));
        }
        
        if let Some(dept) = department {
            let escaped_dept = dept.replace("'", "''");
            where_conditions.push(format!("department = '{}'", escaped_dept));
        }
        
        if active_only {
            where_conditions.push("is_active = 1".to_string());
        }
        
        let where_clause = if where_conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_conditions.join(" AND "))
        };
        
        // Get total count
        let count_query = format!("SELECT COUNT(*) FROM users {}", where_clause);
        let total: (i64,) = sqlx::query_as(&count_query)
            .fetch_one(&self.pool)
            .await?;
        
        // Parse cursor for offset
        let offset = cursor
            .and_then(|c| c.parse::<i64>().ok())
            .unwrap_or(0);
        
        // Get page of users
        let users_query = format!(
            "SELECT id, name, email, created_at, is_active, department, last_login, profile_data 
             FROM users {} 
             ORDER BY created_at DESC, id DESC 
             LIMIT ? OFFSET ?",
            where_clause
        );
        
        let rows = sqlx::query(&users_query)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;
        
        let users: Vec<User> = rows.into_iter().map(|row| User {
            id: row.get(0),
            name: row.get(1),
            email: row.get(2),
            created_at: row.get(3),
            is_active: row.get(4),
            department: row.get(5),
            last_login: row.get(6),
            profile_data: row.get(7),
        }).collect();
        
        // Determine next cursor
        let next_cursor = if (offset + limit) < total.0 {
            Some((offset + limit).to_string())
        } else {
            None
        };
        
        Ok((users, next_cursor, total.0))
    }

    /// Search users with relevance scoring
    async fn search_users(
        &self,
        query: &str,
        cursor: Option<&str>,
        limit: i64,
    ) -> Result<(Vec<User>, Option<String>, i64), sqlx::Error> {
        let offset = cursor.and_then(|c| c.parse::<i64>().ok()).unwrap_or(0);
        let search_pattern = format!("%{}%", query);
        
        // Get total count of matches
        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM users WHERE name LIKE ? OR email LIKE ? OR department LIKE ?"
        )
        .bind(&search_pattern)
        .bind(&search_pattern)
        .bind(&search_pattern)
        .fetch_one(&self.pool)
        .await?;
        
        // Get search results with relevance scoring
        let rows = sqlx::query(r#"
            SELECT id, name, email, created_at, is_active, department, last_login, profile_data,
                   CASE 
                       WHEN name LIKE ? THEN 100
                       WHEN email LIKE ? THEN 80
                       WHEN department LIKE ? THEN 60
                       ELSE 0
                   END as relevance_score
            FROM users 
            WHERE name LIKE ? OR email LIKE ? OR department LIKE ?
            ORDER BY relevance_score DESC, created_at DESC
            LIMIT ? OFFSET ?
        "#)
        .bind(&search_pattern)
        .bind(&search_pattern)
        .bind(&search_pattern)
        .bind(&search_pattern)
        .bind(&search_pattern)
        .bind(&search_pattern)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        
        let users: Vec<User> = rows.into_iter().map(|row| User {
            id: row.get(0),
            name: row.get(1),
            email: row.get(2),
            created_at: row.get(3),
            is_active: row.get(4),
            department: row.get(5),
            last_login: row.get(6),
            profile_data: row.get(7),
        }).collect();
        
        let next_cursor = if (offset + limit) < total.0 {
            Some((offset + limit).to_string())
        } else {
            None
        };
        
        Ok((users, next_cursor, total.0))
    }

    /// Update user activity status (for refresh operations)
    async fn refresh_user_activity(&self) -> Result<i64, sqlx::Error> {
        // Simulate activity updates - randomly activate some inactive users
        // and occasionally deactivate some active users
        let activated = sqlx::query(
            "UPDATE users SET is_active = 1, last_login = CURRENT_TIMESTAMP 
             WHERE is_active = 0 AND id IN (
                 SELECT id FROM users WHERE is_active = 0 ORDER BY RANDOM() LIMIT ?
             )"
        )
        .bind(50) // Activate up to 50 users
        .execute(&self.pool)
        .await?
        .rows_affected();
        
        let deactivated = sqlx::query(
            "UPDATE users SET is_active = 0 
             WHERE is_active = 1 AND last_login < datetime('now', '-60 days') AND id IN (
                 SELECT id FROM users WHERE is_active = 1 AND last_login < datetime('now', '-60 days') 
                 ORDER BY RANDOM() LIMIT ?
             )"
        )
        .bind(10) // Deactivate up to 10 old users
        .execute(&self.pool)
        .await?
        .rows_affected();
        
        Ok(activated as i64 + deactivated as i64)
    }
}

/// User data structure
#[derive(Debug, Clone)]
struct User {
    id: i64,
    name: String,
    email: String,
    created_at: DateTime<Utc>,
    is_active: bool,
    department: String,
    last_login: Option<DateTime<Utc>>,
    profile_data: String,
}

/// Tool for listing users with database pagination
struct ListUsersTool {
    db: DatabaseManager,
}

impl ListUsersTool {
    fn new(db: DatabaseManager) -> Self {
        Self { db }
    }
}

#[async_trait]
impl McpTool for ListUsersTool {
    fn name(&self) -> &str {
        "list_users"
    }

    fn description(&self) -> &str {
        "List users with SQLite-based cursor pagination, filtering, and department selection"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("cursor".to_string(), JsonSchema::String {
                    description: Some("Pagination cursor (offset) for next page".to_string()),
                    pattern: None,
                    min_length: None,
                    max_length: None,
                    enum_values: None,
                }),
                ("limit".to_string(), JsonSchema::Integer {
                    description: Some("Number of users per page (1-100)".to_string()),
                    minimum: Some(1),
                    maximum: Some(100),
                }),
                ("filter".to_string(), JsonSchema::String {
                    description: Some("Filter users by name or email".to_string()),
                    pattern: None,
                    min_length: None,
                    max_length: None,
                    enum_values: None,
                }),
                ("department".to_string(), JsonSchema::String {
                    description: Some("Filter by department".to_string()),
                    pattern: None,
                    min_length: None,
                    max_length: None,
                    enum_values: Some(vec![
                        "Engineering".to_string(),
                        "Marketing".to_string(),
                        "Sales".to_string(),
                        "HR".to_string(),
                        "Finance".to_string(),
                        "Operations".to_string(),
                        "Support".to_string(),
                        "Legal".to_string(),
                    ]),
                }),
                ("active_only".to_string(), JsonSchema::Boolean {
                    description: Some("Show only active users".to_string()),
                }),
            ]))
    }

    async fn call(&self, args: Value, _session: Option<SessionContext>) -> McpResult<Vec<ToolResult>> {
        let cursor = args.get("cursor").and_then(|v| v.as_str());
        let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(25);
        let filter = args.get("filter").and_then(|v| v.as_str());
        let department = args.get("department").and_then(|v| v.as_str());
        let active_only = args.get("active_only").and_then(|v| v.as_bool()).unwrap_or(false);

        if limit > 100 {
            return Err(McpError::param_out_of_range("limit", &limit.to_string(), "1-100"));
        }

        match self.db.get_users_page(cursor, limit, filter, department, active_only).await {
            Ok((users, next_cursor, total)) => {
                let users_data: Vec<_> = users.iter().map(|user| {
                    json!({
                        "id": user.id,
                        "name": user.name,
                        "email": user.email,
                        "created_at": user.created_at.to_rfc3339(),
                        "is_active": user.is_active,
                        "department": user.department,
                        "last_login": user.last_login.map(|dt| dt.to_rfc3339()),
                        "profile": serde_json::from_str::<Value>(&user.profile_data).unwrap_or(json!({}))
                    })
                }).collect();

                let meta = Meta::with_pagination(
                    next_cursor.as_ref().map(|c| Cursor::new(c.clone())),
                    Some(total as u64),
                    next_cursor.is_some()
                );

                let pagination_info = json!({
                    "users": users_data,
                    "pagination": {
                        "has_more": meta.has_more,
                        "next_cursor": next_cursor,
                        "total": total,
                        "current_page_size": users.len(),
                        "query_info": {
                            "filter": filter,
                            "department": department,
                            "active_only": active_only
                        }
                    }
                });

                let result_text = format!(
                    "Found {} users (showing {} of {} total)\n\nFilters applied:\n- Filter: {}\n- Department: {}\n- Active only: {}\n\nPagination:\n- Has more: {}\n- Next cursor: {}\n- Total users: {}",
                    users.len(),
                    users.len(),
                    total,
                    filter.unwrap_or("None"),
                    department.unwrap_or("All"),
                    active_only,
                    meta.has_more.unwrap_or(false),
                    next_cursor.unwrap_or_else(|| "None".to_string()),
                    total
                );

                Ok(vec![
                    ToolResult::text(result_text),
                    ToolResult::resource(pagination_info),
                ])
            }
            Err(e) => {
                error!("Database error in list_users: {}", e);
                Err(McpError::tool_execution(&format!("Database error: {}", e)))
            }
        }
    }
}

/// Tool for searching users with database queries
struct SearchUsersTool {
    db: DatabaseManager,
}

impl SearchUsersTool {
    fn new(db: DatabaseManager) -> Self {
        Self { db }
    }
}

#[async_trait]
impl McpTool for SearchUsersTool {
    fn name(&self) -> &str {
        "search_users"
    }

    fn description(&self) -> &str {
        "Search users by name, email, or department with SQLite full-text capabilities"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("query".to_string(), JsonSchema::String {
                    description: Some("Search query for name, email, or department".to_string()),
                    pattern: None,
                    min_length: Some(1),
                    max_length: None,
                    enum_values: None,
                }),
                ("cursor".to_string(), JsonSchema::String {
                    description: Some("Pagination cursor for next page".to_string()),
                    pattern: None,
                    min_length: None,
                    max_length: None,
                    enum_values: None,
                }),
                ("limit".to_string(), JsonSchema::Integer {
                    description: Some("Number of results per page (1-50)".to_string()),
                    minimum: Some(1),
                    maximum: Some(50),
                }),
            ]))
            .with_required(vec!["query".to_string()])
    }

    async fn call(&self, args: Value, _session: Option<SessionContext>) -> McpResult<Vec<ToolResult>> {
        let query = args.get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("query"))?;
            
        let cursor = args.get("cursor").and_then(|v| v.as_str());
        let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(20);

        if limit > 50 {
            return Err(McpError::param_out_of_range("limit", &limit.to_string(), "1-50"));
        }

        match self.db.search_users(query, cursor, limit).await {
            Ok((users, next_cursor, total)) => {
                let search_results: Vec<_> = users.iter().map(|user| {
                    json!({
                        "id": user.id,
                        "name": user.name,
                        "email": user.email,
                        "created_at": user.created_at.to_rfc3339(),
                        "is_active": user.is_active,
                        "department": user.department,
                        "last_login": user.last_login.map(|dt| dt.to_rfc3339()),
                        "relevance_score": calculate_relevance_score(&user.name, &user.email, &user.department, query)
                    })
                }).collect();

                let meta = Meta::with_pagination(
                    next_cursor.as_ref().map(|c| Cursor::new(c.clone())),
                    Some(total as u64),
                    next_cursor.is_some()
                );

                let response_data = json!({
                    "query": query,
                    "results": search_results,
                    "pagination": {
                        "has_more": meta.has_more,
                        "next_cursor": next_cursor,
                        "total_matches": total,
                        "current_page_size": users.len()
                    }
                });

                let result_text = format!(
                    "Search Results for '{}': {} matches (showing {} of {})\n\nTop results:\n{}\n\nPagination:\n- Has more: {}\n- Next cursor: {}",
                    query,
                    total,
                    users.len(),
                    total,
                    serde_json::to_string_pretty(&search_results.iter().take(3).collect::<Vec<_>>()).unwrap_or_else(|_| "Error formatting results".to_string()),
                    meta.has_more.unwrap_or(false),
                    next_cursor.unwrap_or_else(|| "None".to_string())
                );

                Ok(vec![
                    ToolResult::text(result_text),
                    ToolResult::resource(response_data),
                ])
            }
            Err(e) => {
                error!("Database error in search_users: {}", e);
                Err(McpError::tool_execution(&format!("Database error: {}", e)))
            }
        }
    }
}

/// Tool for refreshing database data (demonstrates dynamic operations)
struct RefreshDataTool {
    db: DatabaseManager,
}

impl RefreshDataTool {
    fn new(db: DatabaseManager) -> Self {
        Self { db }
    }
}

#[async_trait]
impl McpTool for RefreshDataTool {
    fn name(&self) -> &str {
        "refresh_data"
    }

    fn description(&self) -> &str {
        "Refresh database by updating user activity status and generating new data"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("operation".to_string(), JsonSchema::String {
                    description: Some("Type of refresh operation".to_string()),
                    pattern: None,
                    min_length: None,
                    max_length: None,
                    enum_values: Some(vec![
                        "update_activity".to_string(),
                        "full_stats".to_string(),
                    ]),
                }),
            ]))
    }

    async fn call(&self, args: Value, _session: Option<SessionContext>) -> McpResult<Vec<ToolResult>> {
        let operation = args.get("operation")
            .and_then(|v| v.as_str())
            .unwrap_or("update_activity");

        match operation {
            "update_activity" => {
                match self.db.refresh_user_activity().await {
                    Ok(updated_count) => {
                        let result = json!({
                            "operation": "update_activity",
                            "updated_users": updated_count,
                            "timestamp": Utc::now().to_rfc3339(),
                            "message": format!("Successfully updated activity status for {} users", updated_count)
                        });

                        Ok(vec![
                            ToolResult::text(format!("Data refresh completed: {} users updated", updated_count)),
                            ToolResult::resource(result),
                        ])
                    }
                    Err(e) => {
                        error!("Failed to refresh user activity: {}", e);
                        Err(McpError::tool_execution(&format!("Database error during refresh: {}", e)))
                    }
                }
            }
            "full_stats" => {
                // Get comprehensive database statistics
                match self.get_database_stats().await {
                    Ok(stats) => {
                        let result_text = format!(
                            "Database Statistics:\n- Total users: {}\n- Active users: {}\n- Inactive users: {}\n- Departments: {}\n- Recent activity: {} users in last 30 days",
                            stats["total_users"], stats["active_users"], stats["inactive_users"], 
                            stats["departments"], stats["recent_activity"]
                        );

                        Ok(vec![
                            ToolResult::text(result_text),
                            ToolResult::resource(stats),
                        ])
                    }
                    Err(e) => {
                        error!("Failed to get database stats: {}", e);
                        Err(McpError::tool_execution(&format!("Database error: {}", e)))
                    }
                }
            }
            _ => Err(McpError::invalid_param_type("operation", "update_activity|full_stats", operation))
        }
    }
}

impl RefreshDataTool {
    async fn get_database_stats(&self) -> Result<Value, sqlx::Error> {
        let total: (i64,) = query_as("SELECT COUNT(*) FROM users").fetch_one(&self.db.pool).await?;
        let active: (i64,) = query_as("SELECT COUNT(*) FROM users WHERE is_active = 1").fetch_one(&self.db.pool).await?;
        let recent: (i64,) = query_as("SELECT COUNT(*) FROM users WHERE last_login > datetime('now', '-30 days')").fetch_one(&self.db.pool).await?;
        let departments: (i64,) = query_as("SELECT COUNT(DISTINCT department) FROM users").fetch_one(&self.db.pool).await?;

        Ok(json!({
            "total_users": total.0,
            "active_users": active.0,
            "inactive_users": total.0 - active.0,
            "recent_activity": recent.0,
            "departments": departments.0,
            "last_updated": Utc::now().to_rfc3339()
        }))
    }
}

/// Helper function to calculate search relevance score
fn calculate_relevance_score(name: &str, email: &str, department: &str, query: &str) -> f64 {
    let query_lower = query.to_lowercase();
    let name_lower = name.to_lowercase();
    let email_lower = email.to_lowercase();
    let dept_lower = department.to_lowercase();
    
    let mut score = 0.0;
    
    // Exact matches get highest scores
    if name_lower == query_lower {
        score += 100.0;
    } else if name_lower.contains(&query_lower) {
        score += 80.0;
    }
    
    if email_lower.contains(&query_lower) {
        score += 60.0;
    }
    
    if dept_lower.contains(&query_lower) {
        score += 40.0;
    }
    
    // Word boundary matches get bonus
    for word in name_lower.split_whitespace() {
        if word.starts_with(&query_lower) {
            score += 30.0;
        }
    }
    
    score
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting SQLite Pagination Server Example");

    // Setup database with lifecycle management
    let db = match DatabaseManager::new().await {
        Ok(db) => {
            info!("✅ Database setup completed successfully");
            db
        }
        Err(e) => {
            error!("❌ Failed to setup database: {}", e);
            return Err(e);
        }
    };

    let server = McpServer::builder()
        .name("pagination-server")
        .version("2.0.0")
        .title("SQLite MCP Pagination Server")
        .instructions("This server demonstrates comprehensive MCP pagination functionality using SQLite database with realistic large dataset handling, connection management, and dynamic operations.")
        .tool(ListUsersTool::new(db.clone()))
        .tool(SearchUsersTool::new(db.clone()))
        .tool(RefreshDataTool::new(db.clone()))
        .bind_address("127.0.0.1:8044".parse()?)
        .build()?;
    
    info!("🚀 SQLite Pagination server running at: http://127.0.0.1:8044/mcp");
    info!("📊 Database contains 10,000 sample users across 8 departments");
    info!("");
    info!("Available tools:");
    info!("  📋 list_users: SQLite-based pagination with filtering and department selection");
    info!("  🔍 search_users: Full-text search with relevance scoring and pagination");
    info!("  🔄 refresh_data: Dynamic database operations and statistics");
    info!("");
    info!("Key improvements:");
    info!("  ✅ SQLite database with proper indexing");
    info!("  ✅ Real database pagination (LIMIT/OFFSET)");
    info!("  ✅ Dynamic data refresh capabilities");
    info!("  ✅ Connection pool management");
    info!("  ✅ Setup/teardown lifecycle");
    info!("  ✅ Realistic large dataset (10,000 users)");
    info!("  ✅ Performance optimized queries");

    // Run server with graceful shutdown
    match server.run().await {
        Ok(_) => {
            info!("Server shutdown completed");
            Ok(())
        }
        Err(e) => {
            error!("Server error: {}", e);
            Err(Box::new(e))
        }
    }
}