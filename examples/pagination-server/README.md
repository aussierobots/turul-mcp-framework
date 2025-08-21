# Pagination Server Example

A comprehensive example demonstrating cursor-based pagination in MCP servers. This server shows how to handle large datasets efficiently using pagination with proper MCP 2025-06-18 compliant `_meta` fields.

## Overview

This example implements a complete pagination system with:
- **Cursor-based pagination** for large datasets (2500 sample users)
- **Configurable page sizes** with proper validation and limits
- **Advanced search capabilities** with relevance scoring
- **Batch processing** with progress tracking
- **Filtering options** including active-only user filtering
- **MCP 2025-06-18 compliant** `_meta` fields for proper pagination metadata

## Features

### 🔍 **Three Comprehensive Tools**

1. **`list_users`** - List users with cursor-based pagination and filtering
2. **`search_users`** - Search users by name, email, or ID with pagination
3. **`batch_process`** - Process users in batches with progress tracking

### 📊 **Advanced Dataset Management**

- **2500 sample users** with realistic names, emails, and metadata
- **Configurable page sizes** (up to 100 per page for listing, 50 for search)
- **Thread-safe access** using Arc<Mutex<>> for concurrent operations
- **Memory efficient** cursor-based navigation

### 🎯 **MCP 2025-06-18 Compliance**

- **Proper `_meta` fields** with cursor, total, and has_more information
- **Cursor management** with string-based position tracking
- **Pagination metadata** including total counts and navigation hints

## Quick Start

### 1. Start the Server

```bash
cargo run --bin pagination-server
```

The server will start on `http://127.0.0.1:8044/mcp` and generate a dataset with 2500 sample users.

### 2. Test with MCP Client

You can interact with the server using any MCP client. Here are example tool calls:

#### List Users (Basic Pagination)
```json
{
  "name": "list_users",
  "arguments": {
    "limit": 10
  }
}
```

#### List Users with Cursor Navigation
```json
{
  "name": "list_users", 
  "arguments": {
    "cursor": "25",
    "limit": 25,
    "active_only": true
  }
}
```

#### Search Users with Pagination
```json
{
  "name": "search_users",
  "arguments": {
    "query": "alice",
    "limit": 10
  }
}
```

#### Batch Process Users
```json
{
  "name": "batch_process",
  "arguments": {
    "operation": "validate_emails",
    "batch_size": 50,
    "dry_run": true
  }
}
```

## Tool Reference

### 📋 `list_users`

Lists users with cursor-based pagination and optional filtering.

**Parameters:**
- `cursor` (optional): Pagination cursor for next page
- `limit` (optional): Number of users per page (1-100, default: 20)
- `filter` (optional): Filter users by name or email
- `active_only` (optional): Show only active users (default: false)

**Returns:**
- User list with pagination metadata
- Next cursor for navigation
- Total count and page information

### 🔍 `search_users`

Searches users by name, email, or ID with relevance scoring and pagination.

**Parameters:**
- `query` (required): Search query for name, email, or ID
- `cursor` (optional): Pagination cursor for next page
- `limit` (optional): Number of results per page (1-50, default: 10)

**Returns:**
- Search results with relevance scores
- Pagination metadata with match counts
- Next cursor for continued search

**Relevance Scoring:**
- **100 points**: Exact name match
- **80 points**: Name contains query
- **60 points**: Email contains query  
- **40 points**: Name starts with query (word boundary)

### ⚙️ `batch_process`

Processes users in batches with progress tracking and cursor-based resumption.

**Parameters:**
- `operation` (required): Operation to perform
  - `validate_emails` - Validate email format
  - `export_data` - Export user data
  - `send_notifications` - Send notifications to active users
  - `cleanup_inactive` - Mark inactive users for cleanup
- `batch_size` (optional): Users per batch (1-200, default: 50)
- `cursor` (optional): Resume processing from cursor position
- `dry_run` (optional): Preview operation without changes (default: false)

**Returns:**
- Batch processing results with detailed operation data
- Progress tracking with completion percentage
- Next cursor for batch resumption

## Data Structure

### User Model
```rust
struct User {
    id: u64,
    name: String,        // Realistic names from predefined list
    email: String,       // Generated emails with various domains
    created_at: DateTime<Utc>,  // Random creation dates
    is_active: bool,     // 80% of users are active
}
```

### Dataset Management
```rust
struct DatasetManager {
    users: Vec<User>,    // 2500 pre-generated users
    page_size: usize,    // Default page size (25)
}
```

## Pagination Implementation

### Cursor-Based Navigation

The server uses string-based cursors that encode the position in the dataset:

```rust
// Start position encoded as string
let cursor = "25";  // Start from user index 25

// Calculate page boundaries
let start_pos = cursor.parse::<usize>().unwrap_or(0);
let end_pos = std::cmp::min(start_pos + page_size, total);

// Generate next cursor
let next_cursor = if end_pos < total { 
    Some(end_pos.to_string()) 
} else { 
    None 
};
```

### MCP 2025-06-18 Meta Fields

All paginated responses include proper `_meta` fields:

```rust
let meta = Meta::with_pagination(
    next_cursor.as_ref().map(|c| Cursor::new(c.clone())),
    Some(total as u64),
    end_pos < total  // has_more flag
);
```

### Response Format

Paginated responses include both text summaries and structured data:

```json
{
  "users": [...],
  "pagination": {
    "has_more": true,
    "next_cursor": "50", 
    "total": 2500,
    "current_page_size": 25
  }
}
```

## Performance Characteristics

### Memory Usage
- **Efficient dataset storage**: All users pre-generated at startup
- **Minimal per-request allocation**: Only page data cloned for thread safety
- **Cursor-based navigation**: No server-side state required

### Scalability
- **Thread-safe operations**: Multiple concurrent requests supported
- **Configurable limits**: Prevents resource exhaustion
- **Batch processing**: Handles large operations efficiently

### Response Times
- **Fast pagination**: O(1) cursor-based navigation
- **Efficient filtering**: In-memory string matching
- **Search optimization**: Relevance scoring with early termination

## Error Handling

The server includes comprehensive error handling:

- **Parameter validation**: Limit checking and required parameter enforcement
- **Cursor validation**: Graceful handling of invalid cursor values
- **Resource limits**: Protection against excessive batch sizes
- **Operation validation**: Proper error messages for invalid operations

## Thread Safety

All operations are thread-safe using:
- **Arc<Mutex<DatasetManager>>**: Shared dataset access
- **Data cloning**: Avoid holding locks across await points
- **Session isolation**: Independent cursors per client session

## Use Cases

### 1. **Large Dataset Navigation**
Perfect for applications that need to present large datasets to users with efficient navigation.

### 2. **Search with Pagination** 
Demonstrates how to implement search functionality that works seamlessly with pagination.

### 3. **Batch Operations**
Shows how to implement long-running batch operations with progress tracking and resumption.

### 4. **Real-world Data Patterns**
Realistic user data with proper email formats, names, and activity status.

## Configuration

### Server Configuration
```rust
let server = McpServer::builder()
    .name("pagination-server")
    .version("1.0.0") 
    .title("MCP Pagination Server")
    .instructions("Comprehensive MCP pagination functionality...")
    .bind_address("127.0.0.1:8044".parse()?)
    .build()?;
```

### Dataset Configuration
```rust
// Create dataset with 2500 users, 25 per page default
let dataset = Arc::new(Mutex::new(DatasetManager::new(2500, 25)));
```

## Integration Examples

### Client Implementation
```javascript
// Example MCP client usage
const client = new McpClient("http://127.0.0.1:8044/mcp");

// Paginate through all users
let cursor = null;
do {
    const response = await client.callTool("list_users", {
        cursor,
        limit: 50
    });
    
    const data = JSON.parse(response.content[1].resource);
    cursor = data.pagination.next_cursor;
    
    // Process users...
    console.log(`Processed ${data.users.length} users`);
} while (cursor);
```

### Search with Pagination
```javascript
async function searchUsers(query) {
    let cursor = null;
    let allResults = [];
    
    do {
        const response = await client.callTool("search_users", {
            query,
            cursor,
            limit: 20
        });
        
        const data = JSON.parse(response.content[1].resource);
        allResults = allResults.concat(data.results);
        cursor = data.pagination.next_cursor;
    } while (cursor);
    
    return allResults;
}
```

## Testing

### Basic Functionality Test
```bash
# Start the server
cargo run --bin pagination-server &

# Test with curl (requires MCP client setup)
curl -X POST http://127.0.0.1:8044/mcp \
  -H "Content-Type: application/json" \
  -d '{"method": "tools/call", "params": {"name": "list_users", "arguments": {"limit": 5}}}'
```

### Load Testing
```bash
# Test pagination performance
for i in {1..100}; do
    echo "Page $i"
    # Make paginated requests...
done
```

## Best Practices Demonstrated

1. **Efficient Pagination**: Cursor-based instead of offset-based for better performance
2. **Proper Limits**: Configurable limits prevent resource exhaustion  
3. **Thread Safety**: Safe concurrent access to shared data
4. **Error Handling**: Comprehensive validation and error messages
5. **MCP Compliance**: Proper use of `_meta` fields per specification
6. **Memory Management**: Efficient data handling without memory leaks
7. **Progress Tracking**: Real-time progress for long-running operations

## Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   MCP Client    │────│  Pagination      │────│  DatasetManager │
│                 │    │  Server          │    │                 │
│ - Navigation    │    │ - ListUsersTool  │    │ - 2500 Users    │
│ - Search        │    │ - SearchTool     │    │ - Thread Safety │
│ - Batch Ops     │    │ - BatchTool      │    │ - Filtering     │
└─────────────────┘    └──────────────────┘    └─────────────────┘
```

This example provides a complete foundation for implementing pagination in MCP servers, demonstrating best practices for handling large datasets efficiently while maintaining excellent user experience.