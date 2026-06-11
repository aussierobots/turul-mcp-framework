# Simple PostgreSQL Storage Backend Example (2026-07-28 lane)

Demonstrates wiring a **durable PostgreSQL storage backend** into an MCP
server. On the 2026 stateless core there are no client-visible sessions —
the storage backs the transport's internal per-request contexts and event
streams. The demo tools drive the `SessionStorage` backend API directly
against one durable record per run: `storage_info` counts accumulate across
server restarts, which is the observable persistence proof.

Cross-request APPLICATION state belongs in your own store; the 2025-11-25
stateful session model lives on the opt-in lane (see `stateful-server`).

## Features

- **Session-scoped storage**: Each MCP session gets isolated key-value storage in PostgreSQL
- **Multi-instance sharing**: Multiple server instances can share the same PostgreSQL database
- **Automatic table creation**: Tables are created automatically when `verify_tables: true, create_tables: true`
- **ACID transactions**: PostgreSQL provides reliable data consistency

## Setup

### 1. Start PostgreSQL

Using Docker:
```bash
docker run -d --name postgres-session \
  -e POSTGRES_DB=mcp_sessions \
  -e POSTGRES_USER=mcp \
  -e POSTGRES_PASSWORD=mcp_pass \
  -p 5432:5432 \
  postgres:15
```

### 2. Create PostgreSQL Tables

**Option A: Using Setup Utility (Recommended)**
```bash
# Create PostgreSQL tables
DATABASE_URL="postgres://mcp:mcp_pass@localhost:5432/mcp_sessions" cargo run --bin postgres-setup

# Then run the server
DATABASE_URL="postgres://mcp:mcp_pass@localhost:5432/mcp_sessions" cargo run --bin simple-postgres-session
```

**Option B: Automatic Creation**
```bash
# Server will create tables automatically if they don't exist
cargo run --bin simple-postgres-session
```

The setup utility creates the required PostgreSQL tables with proper schema and indexes.

## Usage

The server runs at `http://127.0.0.1:8060/mcp` and provides these tools:

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
    "name": "storage_info",
    "arguments": {}
  }
}
```

## Available Tools

- **`store_value`** - Write to this run's durable demo record
- **`get_value`** - Read it back (within this run)
- **`storage_info`** - Backend stats; counts accumulate across restarts

## Session Storage Behavior

- **Session-scoped**: Data is isolated per session ID
- **Durable**: rows outlive the process — restart and watch `storage_info` counts grow
- **Multi-instance**: Multiple servers can share the same database
- **ACID compliance**: PostgreSQL ensures data consistency

## Configuration

The server uses this environment variable:

```bash
DATABASE_URL=postgres://mcp:mcp_pass@localhost:5432/mcp_sessions
```

## Multi-Instance Setup

To share sessions across multiple server instances:

1. **Start PostgreSQL** (shared database)
2. **Start multiple servers** with the same `DATABASE_URL`
3. **Sessions are shared** between all server instances

```bash
# Terminal 1
DATABASE_URL="postgres://mcp:pass@db.example.com:5432/shared_sessions" cargo run --bin simple-postgres-session

# Terminal 2 (different port, same database)
DATABASE_URL="postgres://mcp:pass@db.example.com:5432/shared_sessions" cargo run --bin simple-postgres-session -- --port 8061
```

## Example Session

1. **Create tables**: `DATABASE_URL="postgres://..." cargo run --bin postgres-setup`
2. **Start server**: `DATABASE_URL="postgres://..." cargo run --bin simple-postgres-session`
3. **Store data**: `store_value(key='user_id', value=123)`
4. **Restart server**: prior rows persist in the backend — `storage_info` shows the accumulated count
5. **Retrieve data**: `get_value(key='user_id')` returns `123`

Each session maintains its own isolated storage space in the PostgreSQL database.

## Cleanup

To delete all PostgreSQL tables and data (permanent deletion):

```bash
# WARNING: This will permanently delete ALL session data!
CONFIRM_DELETE=yes DATABASE_URL="postgres://..." cargo run --bin postgres-teardown
```

This drops both tables:
- `mcp_sessions` (main session table)
- `mcp_session_events` (events table)

## Available Commands

- **`cargo run --bin postgres-setup`** - Create PostgreSQL tables
- **`cargo run --bin simple-postgres-session`** - Run the MCP server
- **`cargo run --bin postgres-teardown`** - Drop PostgreSQL tables (requires `CONFIRM_DELETE=yes`)