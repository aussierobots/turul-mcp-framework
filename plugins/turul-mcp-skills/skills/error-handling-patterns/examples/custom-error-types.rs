// turul-mcp-server v0.3
// Custom error types: From<MyError> for McpError

use turul_mcp_derive::mcp_tool;
use turul_mcp_protocol::McpError;
use turul_mcp_server::McpResult;

// Define a domain-specific error type
#[derive(Debug, thiserror::Error)]
enum DatabaseError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Query timeout after {seconds}s")]
    Timeout { seconds: u64 },
    #[error("Record not found: {id}")]
    NotFound { id: String },
}

// Implement From to allow using ? in tool handlers
impl From<DatabaseError> for McpError {
    fn from(err: DatabaseError) -> Self {
        match err {
            DatabaseError::ConnectionFailed(msg) => {
                McpError::tool_execution(&format!("Database connection failed: {msg}"))
            }
            DatabaseError::Timeout { seconds } => {
                McpError::tool_execution(&format!("Database query timed out after {seconds}s"))
            }
            DatabaseError::NotFound { id } => {
                McpError::tool_execution(&format!("Record not found: {id}"))
            }
        }
    }
}

// Now you can use ? directly with DatabaseError
#[mcp_tool(name = "get_user", description = "Get user by ID")]
async fn get_user(
    #[param(description = "User ID")] id: String,
) -> McpResult<String> {
    // ? converts DatabaseError → McpError via From impl
    let user = find_user(&id)?;
    Ok(user)
}

fn find_user(id: &str) -> Result<String, DatabaseError> {
    if id == "unknown" {
        Err(DatabaseError::NotFound { id: id.to_string() })
    } else {
        Ok(format!("User({})", id))
    }
}
