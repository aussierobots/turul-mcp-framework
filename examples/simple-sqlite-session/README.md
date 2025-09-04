# Simple SQLite Session Storage Example

This example demonstrates SQLite-backed session storage for MCP servers. Perfect for single-instance deployments that need persistent session state without the complexity of a database server.

## Features

- **Session-scoped storage**: Each MCP session gets isolated key-value storage in SQLite
- **File-based persistence**: Session data stored in local SQLite database file
- **Zero configuration**: No database server setup required
- **Automatic creation**: Database and tables created automatically
- **Lightweight**: Minimal resource usage, perfect for development and desktop apps

## Setup

### 1. Create SQLite Database

**Option A: Using Setup Utility (Recommended)**
```bash
# Create SQLite database and tables
SQLITE_PATH="./my-sessions.db" cargo run --bin sqlite-setup

# Then run the server
SQLITE_PATH="./my-sessions.db" cargo run --bin simple-sqlite-session
```

**Option B: Automatic Creation**
```bash
# Server will create database automatically if it doesn't exist
cargo run --bin simple-sqlite-session
```

The setup utility creates the SQLite database file with all required tables and schema.

## Usage

The server runs at `http://127.0.0.1:8061/mcp` and provides these tools:

### Store Value in Session
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "store_value",
    "arguments": {
      "key": "theme",
      "value": "dark"
    }
  }
}
```

### Get Value from Session
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/call",
  "params": {
    "name": "get_value",
    "arguments": {
      "key": "theme"
    }
  }
}
```

### Session Information
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "session_info",
    "arguments": {}
  }
}
```

## Available Tools

- **`store_value`** - Store a value in this session's SQLite storage (session-scoped)
- **`get_value`** - Retrieve a value from this session's SQLite storage (session-scoped)
- **`session_info`** - Get information about the SQLite session

## Session Storage Behavior

- **Session-scoped**: Data is isolated per session ID
- **Persistent**: Data survives server restarts
- **File-based**: Data stored in local SQLite database file
- **Single-process**: SQLite is designed for single-process access

## Configuration

The server uses this environment variable:

```bash
SQLITE_PATH=./sessions.db    # SQLite database file path
```

## Use Cases

Perfect for:
- **Development environments**: Local development with persistent state
- **Desktop applications**: Client-side applications needing local storage
- **Single-instance deployments**: Simple deployments without database servers
- **Local persistence**: Any scenario requiring lightweight, local data storage

## Example Session

1. **Create database**: `SQLITE_PATH="./my-sessions.db" cargo run --bin sqlite-setup`
2. **Start server**: `SQLITE_PATH="./my-sessions.db" cargo run --bin simple-sqlite-session`
3. **Store data**: `store_value(key='user_id', value=123)`
4. **Restart server**: Server restarts, session persists in SQLite file
5. **Retrieve data**: `get_value(key='user_id')` returns `123`

Each session maintains its own isolated storage space in the SQLite database file.

## Cleanup

To delete the SQLite database file (permanent deletion):

```bash
# WARNING: This will permanently delete ALL session data!
CONFIRM_DELETE=yes SQLITE_PATH="./my-sessions.db" cargo run --bin sqlite-teardown
```

## Database File

- **Location**: `./sessions.db` (or custom via `SQLITE_PATH`)
- **Format**: Standard SQLite database file
- **Backup**: Simply copy the `.db` file
- **Portability**: Database file is cross-platform

## Available Commands

- **`cargo run --bin sqlite-setup`** - Create SQLite database and tables
- **`cargo run --bin simple-sqlite-session`** - Run the MCP server
- **`cargo run --bin sqlite-teardown`** - Delete SQLite database file (requires `CONFIRM_DELETE=yes`)