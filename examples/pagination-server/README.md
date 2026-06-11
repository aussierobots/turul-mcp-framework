# Pagination Server Example

A comprehensive example demonstrating **application-level** cursor pagination
inside tool results: a SQLite-backed dataset navigated with opaque cursor
strings that the client passes back on each call.

> **Not the protocol's list pagination.** MCP's own `cursor`/`nextCursor`
> contract applies to list operations (`tools/list`, `resources/list`, ...)
> and the framework handles it for you. This example shows the complementary
> *application* pattern: paginating large data through ordinary `tools/call`
> results, with the cursor carried in the tool's JSON payload.

## Overview

This example implements a complete pagination system with:
- **Cursor-based pagination** for large datasets (10,000 sample users)
- **Configurable page sizes** with proper validation and limits
- **Advanced search capabilities** with relevance scoring
- **Dataset refresh operations** with summary statistics
- **Filtering options** including active-only user filtering

## Features

### 🔍 **Three Comprehensive Tools**

1. **`list_users`** - List users with cursor-based pagination and filtering
2. **`search_users`** - Search users by name, email, or ID with pagination
3. **`refresh_data`** - Refresh user activity status / report dataset statistics

### 📊 **Advanced Dataset Management**

- **10,000 sample users** with realistic names, emails, and metadata
- **Configurable page sizes** (up to 100 per page for listing, 50 for search)
- **Thread-safe access** using Arc<Mutex<>> for concurrent operations
- **Memory efficient** cursor-based navigation

### 🎯 **Stateless cursors**

- **Opaque cursor strings** returned in each tool result's `pagination` block
- **Client-supplied position**: the server keeps no per-client cursor state —
  pass `cursor` back on the next call to continue
- **Pagination metadata** (`has_more`, `next_cursor`, `total`) inside the
  tool's JSON result

## Quick Start

### 1. Start the Server

```bash
cargo run -p pagination-server
```

The server picks a free local port, prints its `http://127.0.0.1:<port>/mcp`
URL on startup, and seeds a SQLite dataset with 10,000 sample users.

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

#### Refresh Data
```json
{
  "name": "refresh_data",
  "arguments": {
    "operation": "update_activity"
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

### ⚙️ `refresh_data`

Mutates and reports on the dataset in place.

**Parameters:**
- `operation` (optional): `update_activity` (toggle user activity status) or
  `full_stats` (report dataset statistics)

**Returns:**
- Operation summary with affected-row counts or dataset statistics

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
struct DatabaseManager {
    pool: SqlitePool,    // SQLite-backed dataset (10,000 seeded users)
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

### Response Format

Paginated responses carry the cursor inside the tool's JSON result, alongside
the data and a text summary:

```json
{
  "users": [...],
  "pagination": {
    "has_more": true,
    "next_cursor": "50",
    "total": 10000,
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
- **Connection pooling**: SQLite pool shared across concurrent requests

### Response Times
- **Fast pagination**: O(1) cursor-based navigation
- **Efficient filtering**: SQL WHERE-clause filtering in the database
- **Search optimization**: Relevance scoring with early termination

## Error Handling

The server includes comprehensive error handling:

- **Parameter validation**: Limit checking and required parameter enforcement
- **Cursor validation**: Graceful handling of invalid cursor values
- **Operation validation**: Proper error messages for invalid operations

## Thread Safety

All operations are thread-safe using:
- **Global `OnceLock<DatabaseManager>`**: one shared SQLite pool for all tools
- **Pool-managed connections**: no locks held across await points
- **No server-side cursor state**: cursors are client-supplied strings, so concurrent clients never interfere

## Use Cases

### 1. **Large Dataset Navigation**
Perfect for applications that need to present large datasets to users with efficient navigation.

### 2. **Search with Pagination** 
Demonstrates how to implement search functionality that works seamlessly with pagination.

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
    .bind_address(format!("127.0.0.1:{}", port).parse()?)  // free local port
    .build()?;
```

### Dataset Configuration
```rust
// Seed the SQLite dataset with 10,000 users at startup
let db = Arc::new(DatabaseManager::new().await?);
```

## Integration Examples

### Client Implementation
```javascript
// Example MCP client usage
const client = new McpClient("http://127.0.0.1:<port>/mcp"); // port printed at startup

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
# Start the server (it prints its URL, e.g. http://127.0.0.1:55123/mcp)
cargo run -p pagination-server &

# Test with curl (substitute the printed port; 2026-07-28 stateless)
curl -X POST http://127.0.0.1:<port>/mcp \
  -H 'Content-Type: application/json' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: tools/call' \
  -H 'Mcp-Name: list_users' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"curl","version":"1.0"},"io.modelcontextprotocol/clientCapabilities":{}},"name":"list_users","arguments":{"limit":5}}}'
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
5. **Honest framing**: app-level cursors in tool results, distinct from protocol list pagination
6. **Memory Management**: Efficient data handling without memory leaks
7. **Progress Tracking**: Real-time progress for long-running operations

## Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   MCP Client    │────│  Pagination      │────│ DatabaseManager │
│                 │    │  Server          │    │                 │
│ - Navigation    │    │ - ListUsersTool  │    │ - 10,000 Users  │
│ - Search        │    │ - SearchTool     │    │ - Thread Safety │
│ - Refresh Ops   │    │ - RefreshTool    │    │ - Filtering     │
└─────────────────┘    └──────────────────┘    └─────────────────┘
```

This example provides a complete foundation for implementing pagination in MCP servers, demonstrating best practices for handling large datasets efficiently while maintaining excellent user experience.