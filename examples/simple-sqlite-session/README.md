# Simple SQLite Storage Backend Example (2026-07-28 lane)

Demonstrates wiring a **durable SQLite storage backend** into an MCP server.
On the 2026 stateless core there are no client-visible sessions — the storage
backs the transport's internal per-request contexts and event streams. The
demo tools drive the `SessionStorage` backend API directly against one
durable record per run, so the persistence teaching is observable and true:
`storage_info` counts accumulate across server restarts.

Cross-request APPLICATION state belongs in your own store; the 2025-11-25
stateful session model lives on the opt-in lane (see `stateful-server`).

## Features

- **Backend API surface**: the demo drives `set_session_state`/`get_session_state` directly against a per-run record
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

### Store a value
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

### Read it back
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

### Backend statistics
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "storage_info",
    "arguments": {}
  }
}
```

## Available Tools

- **`store_value`** - Write to this run's durable SQLite demo record
- **`get_value`** - Read it back (within this run)
- **`storage_info`** - Backend stats; counts accumulate across restarts (the durability proof)

## Storage Behavior

- **Durable**: rows outlive the process — restart and watch `storage_info` counts grow
- **File-based**: Data stored in local SQLite database file
- **Single-process**: SQLite is designed for single-process access
- **Per-run record**: each server start creates a fresh demo record, so
  `store_value`/`get_value` round-trip within one run only. What survives a
  restart is the *rows*, not the demo record id — see the walkthrough below.

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

## Durability walkthrough (verified)

1. **Create database**: `SQLITE_PATH="./my-sessions.db" cargo run --bin sqlite-setup`
2. **Start server**: `SQLITE_PATH="./my-sessions.db" cargo run --bin simple-sqlite-session`
3. **Baseline**: `storage_info()` → note `stored_records`
4. **Round-trip**: `store_value(key='user_id', value=123)` then
   `get_value(key='user_id')` → `123`
5. **Restart server** (Ctrl+C, re-run with the same `SQLITE_PATH`)
6. **Proof**: `storage_info()` → `stored_records` has **grown**; the prior
   run's rows are still in the file.
   `get_value(key='user_id')` → `null`, because this run created a *new*
   demo record. Durability is in the accumulated row count, not in the demo
   record id, which the process does not carry across restarts.

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