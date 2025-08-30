# Simple PostgreSQL Session Storage Example

This example demonstrates how to use PostgreSQL as the session storage backend for MCP servers, enabling persistent session state across server restarts and multi-instance deployments.

## Features

- **Persistent Session State**: User preferences survive server restarts
- **Multi-Instance Sharing**: Multiple server instances share the same session data
- **PostgreSQL Integration**: Production-ready relational database backend
- **SSE Notifications**: Real-time progress updates stored in PostgreSQL

## Quick Start

### Automated Setup (Recommended)

Use the built-in database management utilities:

```bash
# Set up PostgreSQL database and schema
cargo run --bin postgres-setup

# Run the MCP server
cargo run --bin server

# When done, clean up (optional)
cargo run --bin postgres-teardown
```

### Manual Setup

#### 1. Start PostgreSQL

Using Docker:
```bash
docker run -d --name postgres-session \
  -e POSTGRES_DB=mcp_sessions \
  -e POSTGRES_USER=mcp \
  -e POSTGRES_PASSWORD=mcp_pass \
  -p 5432:5432 \
  postgres:15
```

#### 2. Run the Server

```bash
cargo run --bin server
```

Or with custom database URL:
```bash
DATABASE_URL="postgres://user:pass@localhost:5432/mydb" cargo run --bin server
```

## Usage

### Store a Preference
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "store_preference",
    "arguments": {
      "key": "theme",
      "value": "dark"
    }
  }
}
```

### Retrieve a Preference
```json
{
  "jsonrpc": "2.0", 
  "id": 2,
  "method": "tools/call",
  "params": {
    "name": "get_preference",
    "arguments": {
      "key": "theme"
    }
  }
}
```

### Test Persistence

1. Store a preference: `store_preference(key="language", value="en")`
2. **Restart the server**
3. Retrieve the preference: `get_preference(key="language")`
4. ✅ Returns `"en"` - the data persisted in PostgreSQL!

## Multi-Instance Testing

1. Start first server instance: `cargo run --bin server` (port 8060)
2. Store preference in first instance
3. Start second server instance on different port:
   ```bash
   # Modify bind_address in main.rs to use port 8061
   cargo run --bin server
   ```
4. Retrieve preference from second instance - it works! 🎉

## Tools

- **`store_preference`** - Store user preference in PostgreSQL
- **`get_preference`** - Retrieve user preference from PostgreSQL
- **`list_preferences`** - List all stored preferences  
- **`session_info`** - View session storage backend information

## Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   MCP Server    │    │  SessionStorage │    │   PostgreSQL    │
│                 │    │      Trait      │    │    Database     │
│ ┌─────────────┐ │    │                 │    │                 │
│ │SessionManager│◄────┤PostgresStorage │────┤   Sessions      │
│ │             │ │    │                 │    │   Events        │
│ └─────────────┘ │    │                 │    │   State         │
│                 │    └─────────────────┘    └─────────────────┘
└─────────────────┘
```

## Configuration

The example uses these PostgreSQL settings:
- **Connection Pool**: 2-10 connections
- **Session Timeout**: 60 minutes
- **Cleanup Interval**: 10 minutes
- **Max Events**: 1000 per session

## Production Considerations

- Use connection pooling for better performance
- Set up regular maintenance for cleanup
- Consider read replicas for high-load scenarios
- Enable PostgreSQL logging for debugging
- Use SSL connections in production

## Database Management

### Setup Binary

The `postgres-setup` binary automates PostgreSQL setup:

```bash
# Uses default settings
cargo run --bin postgres-setup

# With custom environment variables
POSTGRES_HOST=myhost POSTGRES_PORT=5433 cargo run --bin postgres-setup
```

**What it does:**
- Starts PostgreSQL Docker container (if Docker available)
- Creates database schema with tables and indexes
- Configures session storage structure
- Provides connection verification

### Teardown Binary

The `postgres-teardown` binary provides flexible cleanup:

```bash
# Interactive mode - choose what to clean up
cargo run --bin postgres-teardown

# Command line options
cargo run --bin postgres-teardown -- --drop-tables
cargo run --bin postgres-teardown -- --drop-database  
cargo run --bin postgres-teardown -- --stop-container
cargo run --bin postgres-teardown -- --remove-container
cargo run --bin postgres-teardown -- --all
```

**Cleanup Options:**
1. **Clear session data** - Remove sessions but keep schema (safe for development)
2. **Drop tables** - Remove all MCP session tables and data
3. **Drop database** - Remove entire database
4. **Stop container** - Stop Docker container but keep it for restart
5. **Remove container** - Completely remove Docker container and data
6. **Full cleanup** - Drop tables and remove container

### Environment Variables

Both binaries support these environment variables:

```bash
export POSTGRES_HOST=localhost      # Default: localhost
export POSTGRES_PORT=5432          # Default: 5432
export POSTGRES_DB=mcp_sessions    # Default: mcp_sessions
export POSTGRES_USER=mcp           # Default: mcp
export POSTGRES_PASSWORD=mcp_pass  # Default: mcp_pass
```

## Troubleshooting

**Connection Failed**: Make sure PostgreSQL is running and accessible
**Table Errors**: The storage backend auto-creates required tables
**Permission Issues**: Ensure the database user has CREATE/INSERT/UPDATE/DELETE permissions