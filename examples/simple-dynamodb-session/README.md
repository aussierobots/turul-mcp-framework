# Simple DynamoDB Session Storage Example

This example demonstrates DynamoDB-backed session storage for MCP servers. It shows how session state persists in AWS DynamoDB with automatic TTL cleanup.

## Features

- **Session-scoped storage**: Each MCP session gets isolated key-value storage in DynamoDB
- **Automatic table creation**: Tables are created automatically when `create_tables_if_missing: true`
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

- **`store_value`** - Store a value in this session's DynamoDB storage (session-scoped)
- **`get_value`** - Retrieve a value from this session's DynamoDB storage (session-scoped)
- **`session_info`** - Get information about the DynamoDB session

## Session Storage Behavior

- **Session-scoped**: Data is isolated per session ID
- **Persistent**: Data survives server restarts
- **TTL cleanup**: Sessions expire after 24 hours by default
- **Automatic scaling**: DynamoDB scales based on demand

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

## Example Session

1. **Create tables**: `MCP_SESSION_TABLE=my-sessions cargo run --bin dynamodb-setup`
2. **Start server**: `MCP_SESSION_TABLE=my-sessions cargo run --bin simple-dynamodb-session`
3. **Store data**: `store_value(key='user_id', value=123)`
4. **Restart server**: Server restarts, session persists in DynamoDB
5. **Retrieve data**: `get_value(key='user_id')` returns `123`

Each session maintains its own isolated storage space in the DynamoDB tables.

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