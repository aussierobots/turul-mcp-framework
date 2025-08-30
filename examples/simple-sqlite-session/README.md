# Simple SQLite Session Storage Example

This example demonstrates SQLite-backed session storage for MCP servers, providing file-based persistence perfect for single-instance deployments and development environments.

## Features

- **File-Based Persistence**: Session data stored in local SQLite database
- **Zero Configuration**: No database server setup required
- **ACID Transactions**: Reliable data consistency
- **Automatic Schema Creation**: Database and tables created automatically
- **Lightweight**: Minimal resource usage and dependencies

## Quick Start

### Automated Setup (Recommended)

Use the built-in database management utilities:

```bash
# Set up SQLite database and schema
cargo run --bin sqlite-setup

# Run the MCP server
cargo run --bin server

# When done, clean up (optional)
cargo run --bin sqlite-teardown
```

### Manual Setup

#### 1. Run the Server

```bash
cargo run --bin server
```

With custom database location:
```bash
SQLITE_PATH="./my-sessions.db" cargo run --bin server
```

The server will automatically:
- Create the SQLite database file if it doesn't exist
- Set up the required table schema
- Start accepting requests on http://127.0.0.1:8061/mcp

## Usage

### Save a Setting
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "save_setting",
    "arguments": {
      "name": "ui_mode",
      "value": "dark"
    }
  }
}
```

### Load a Setting
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/call",
  "params": {
    "name": "load_setting",
    "arguments": {
      "name": "ui_mode"
    }
  }
}
```

### Test Persistence

1. **Save data**: `save_setting(name="auto_save", value=true)`
2. **Increment counter**: `increment_counter(counter_name="app_launches")`
3. **Stop the server** (Ctrl+C)
4. **Restart the server**: `cargo run --bin server`
5. **Load data**: `load_setting(name="auto_save")` ✅ Returns `true`
6. **Increment again**: `increment_counter(counter_name="app_launches")` ✅ Continues from previous count!

## Tools

- **`save_setting`** - Save user setting to SQLite database
- **`load_setting`** - Load user setting from SQLite database
- **`increment_counter`** - Increment persistent counter (demonstrates stateful operations)
- **`storage_stats`** - View SQLite storage backend information
- **`backup_data`** - Create backup metadata (copy sessions.db file for full backup)

## Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   MCP Server    │    │  SessionStorage │    │  SQLite Database│
│                 │    │      Trait      │    │  (sessions.db)  │
│ ┌─────────────┐ │    │                 │    │                 │
│ │SessionManager│◄────┤ SqliteStorage   │────┤   sessions      │
│ │             │ │    │                 │    │   events        │
│ └─────────────┘ │    │                 │    │   state         │
│                 │    └─────────────────┘    └─────────────────┘
└─────────────────┘
```

## Configuration

Default SQLite settings:
- **Database File**: `./sessions.db`
- **Connection Pool**: 5 connections
- **Session Timeout**: 30 minutes
- **Cleanup Interval**: 5 minutes
- **WAL Mode**: Enabled (better concurrency)
- **Foreign Keys**: Enabled

## Database Management

### Setup Binary

The `sqlite-setup` binary automates SQLite database creation:

```bash
# Uses default settings (./mcp_sessions.db)
cargo run --bin sqlite-setup

# With custom environment variables
SQLITE_DB_PATH="./custom.db" SQLITE_DATA_DIR="./backups" cargo run --bin sqlite-setup
```

**What it does:**
- Creates SQLite database file with proper schema
- Sets up tables and indexes for session storage
- Configures SQLite optimizations (WAL mode, caching)
- Runs database verification tests
- Provides interactive options for existing databases

### Teardown Binary

The `sqlite-teardown` binary provides flexible database cleanup:

```bash
# Interactive mode - choose what to clean up
cargo run --bin sqlite-teardown

# Command line options
cargo run --bin sqlite-teardown -- --clear-data     # Clear data, keep schema
cargo run --bin sqlite-teardown -- --drop-tables    # Remove all tables
cargo run --bin sqlite-teardown -- --backup         # Create timestamped backup
cargo run --bin sqlite-teardown -- --vacuum         # Optimize database file
cargo run --bin sqlite-teardown -- --delete         # Delete database file
cargo run --bin sqlite-teardown -- --all            # Backup + full cleanup
```

**Cleanup Options:**
1. **Clear session data** - Remove all sessions/events but keep schema (safe for development)
2. **Drop tables** - Remove all MCP session tables and data
3. **Backup database** - Create timestamped backup in data directory
4. **Vacuum database** - Reclaim space and optimize file size
5. **Delete database** - Completely remove database file and temporary files
6. **Full cleanup** - Create backup then delete everything

### Environment Variables

Both binaries support these environment variables:

```bash
export SQLITE_DB_PATH="./mcp_sessions.db"  # Database file path
export SQLITE_DATA_DIR="./data"            # Directory for backups
```

### Advanced Backup and Restore

#### Automated Backup
```bash
# Create backup with the management utility
cargo run --bin sqlite-teardown -- --backup
```

#### Manual Backup
```bash
# Stop the server first
cp mcp_sessions.db backup-$(date +%Y%m%d).db
```

#### Restore from Backup
```bash
# Stop the server first  
cp backup-20240830.db mcp_sessions.db
# Restart the server
```

## File Locations

- **Database File**: `./sessions.db` (configurable with `SQLITE_PATH`)
- **WAL File**: `./sessions.db-wal` (Write-Ahead Log)
- **Shared Memory**: `./sessions.db-shm`

## Use Cases

**Perfect for**:
- Single-instance MCP server deployments
- Development and testing environments
- Desktop applications requiring persistence
- Local data storage without database server overhead

**Not recommended for**:
- Multi-instance deployments (no shared access)
- High-concurrency scenarios (SQLite limitations)
- Distributed systems (use PostgreSQL instead)

## Troubleshooting

**Database Locked**: Ensure only one server instance is running
**Permission Denied**: Check write permissions for database directory
**Corruption Issues**: SQLite auto-recovery usually handles this
**Performance**: Consider WAL mode (enabled by default) for better write performance

## Development

The example includes comprehensive logging:
```bash
RUST_LOG=debug cargo run --bin server
```

This shows all SQLite operations and session management activities.