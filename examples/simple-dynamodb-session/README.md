# Simple DynamoDB Storage Backend Example (2026-07-28 lane)

Demonstrates wiring a **durable DynamoDB storage backend** into an MCP
server. On the 2026 stateless core there are no client-visible sessions —
the storage backs the transport's internal per-request contexts and event
streams. The demo tools drive the `SessionStorage` backend API directly
against one durable record per run: `storage_info` counts accumulate across
server restarts, which is the observable persistence proof.

Cross-request APPLICATION state belongs in your own store; the 2025-11-25
stateful session model lives on the opt-in lane (see `stateful-server`).

## Features

- **Backend API surface**: the demo drives `set_session_state`/`get_session_state` directly against a per-run record
- **Automatic table creation**: Tables are created automatically when `verify_tables: true, create_tables: true`
- **TTL cleanup**: Sessions and events automatically expire based on configured TTL
- **AWS native**: Uses AWS SDK with proper IAM integration

## Setup

### 1. Configure AWS Credentials

```bash
# Option 1: AWS CLI
aws configure

# Option 2: Environment variables
export AWS_ACCESS_KEY_ID=your_access_key
export AWS_SECRET_ACCESS_KEY=your_secret_key
export AWS_REGION=us-east-1

# Option 3: IAM Role (recommended for EC2/Lambda)
# Attach DynamoDB permissions to your instance role
```

### 2. Create DynamoDB Tables

**Option A: Using Setup Utility (Recommended)**
```bash
# Create both DynamoDB tables (session + events)
MCP_SESSION_TABLE=my-sessions AWS_REGION=us-east-1 cargo run --bin dynamodb-setup

# Then run the server
MCP_SESSION_TABLE=my-sessions cargo run --bin simple-dynamodb-session
```

**Option B: Automatic Creation**
```bash
# Server will create tables automatically if they don't exist
cargo run --bin simple-dynamodb-session
```

The setup utility creates both required tables:
- **Main session table**: `{MCP_SESSION_TABLE}` (e.g., `my-sessions`)
- **Events table**: `{MCP_SESSION_TABLE}-events` (e.g., `my-sessions-events`)

## Usage

The server runs at `http://127.0.0.1:8062/mcp` and provides these tools:

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

- **`store_value`** - Write to this run's durable demo record
- **`get_value`** - Read it back (within this run)
- **`storage_info`** - Backend stats; counts accumulate across restarts

## Storage Behavior

- **Durable**: rows outlive the process — restart and watch `storage_info` counts grow
- **TTL cleanup**: Records expire after 24 hours by default
- **Automatic scaling**: DynamoDB scales based on demand
- **Per-run record**: each server start creates a fresh demo record, so
  `store_value`/`get_value` round-trip within one run only. What survives a
  restart is the *rows*, not the demo record id — see the walkthrough below.

## Configuration

The server uses these environment variables:

**Application Configuration:**
```bash
MCP_SESSION_TABLE=mcp-sessions   # DynamoDB table name
AWS_REGION=us-east-1             # AWS region
```

**AWS Authentication (choose one):**
```bash
# Option 1: Access Keys
export AWS_ACCESS_KEY_ID=your_access_key
export AWS_SECRET_ACCESS_KEY=your_secret_key

# Option 2: AWS Profile
export AWS_PROFILE=your_profile

# Option 3: IAM Role (automatic on EC2/Lambda)
# No environment variables needed
```

## IAM Permissions

Your AWS credentials need these DynamoDB permissions:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "dynamodb:CreateTable",
        "dynamodb:DescribeTable",
        "dynamodb:GetItem",
        "dynamodb:PutItem", 
        "dynamodb:UpdateItem",
        "dynamodb:DeleteItem"
      ],
      "Resource": "arn:aws:dynamodb:*:*:table/mcp-sessions*"
    }
  ]
}
```

## Durability walkthrough

1. **Create tables**: `MCP_SESSION_TABLE=my-sessions cargo run --bin dynamodb-setup`
2. **Start server**: `MCP_SESSION_TABLE=my-sessions cargo run --bin simple-dynamodb-session`
3. **Baseline**: `storage_info()` → note `stored_records`
4. **Round-trip**: `store_value(key='user_id', value=123)` then
   `get_value(key='user_id')` → `123`
5. **Restart server** against the same `MCP_SESSION_TABLE`
6. **Proof**: `storage_info()` → `stored_records` has **grown**; the prior
   run's rows are still in DynamoDB.
   `get_value(key='user_id')` → `null`, because this run created a *new*
   demo record. Durability is in the accumulated row count, not in the demo
   record id, which the process does not carry across restarts.

## Cleanup

To delete all DynamoDB tables and data (permanent deletion):

```bash
# WARNING: This will permanently delete ALL session data!
CONFIRM_DELETE=yes MCP_SESSION_TABLE=my-sessions cargo run --bin dynamodb-teardown
```

This removes both tables:
- `{MCP_SESSION_TABLE}` (main session table)
- `{MCP_SESSION_TABLE}-events` (events table)

## Available Commands

- **`cargo run --bin dynamodb-setup`** - Create both DynamoDB tables
- **`cargo run --bin simple-dynamodb-session`** - Run the MCP server
- **`cargo run --bin dynamodb-teardown`** - Delete both DynamoDB tables (requires `CONFIRM_DELETE=yes`)