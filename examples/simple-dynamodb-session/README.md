# Simple AWS DynamoDB Session Storage Example

This example demonstrates DynamoDB-backed session storage for MCP servers, showcasing AWS-native serverless capabilities perfect for Lambda functions, auto-scaling applications, and multi-region deployments.

## Features

- **Serverless-Native**: No connection pools, perfect for AWS Lambda
- **Automatic Scaling**: DynamoDB scales based on demand  
- **Built-in TTL**: Automatic session and event cleanup
- **Multi-Region Support**: Global Tables for worldwide deployment
- **AWS Integration**: IAM, CloudWatch, CloudTrail, VPC endpoints
- **Enterprise Features**: Encryption, backup, monitoring

## Prerequisites

### AWS Setup
1. **AWS Credentials**: Configure via CLI, IAM roles, or environment variables
2. **DynamoDB Table**: Auto-created or manually provisioned
3. **IAM Permissions**: Read/write access to DynamoDB

### Local Development (Optional)
```bash
# Run DynamoDB Local for development
docker run -p 8000:8000 amazon/dynamodb-local
```

## Quick Start

### Automated Setup (Recommended)

Use the built-in AWS DynamoDB management utilities:

```bash
# Set up DynamoDB tables and configuration
cargo run --bin dynamodb-setup

# Run the MCP server
cargo run --bin server

# When done, clean up (optional)
cargo run --bin dynamodb-teardown
```

### Manual Setup

#### 1. Configure AWS Credentials

```bash
# Option 1: AWS CLI
aws configure

# Option 2: Environment variables
export AWS_ACCESS_KEY_ID="your-key"
export AWS_SECRET_ACCESS_KEY="your-secret" 
export AWS_REGION="us-east-1"

# Option 3: IAM Role (recommended for EC2/Lambda)
# Attach DynamoDB permissions to your instance role
```

#### 2. Run the Server

```bash
cargo run --bin server
```

With custom configuration:
```bash
DYNAMODB_TABLE="my-sessions" AWS_REGION="us-west-2" cargo run --bin server
```

For local DynamoDB:
```bash
AWS_ENDPOINT_URL="http://localhost:8000" cargo run --bin server
```

## Usage

### Store Application State
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "store_app_state",
    "arguments": {
      "key": "user_profile",
      "data": {"name": "Alice", "role": "admin"}
    }
  }
}
```

### Simulate Lambda Function
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/call", 
  "params": {
    "name": "lambda_operation",
    "arguments": {
      "operation": "process_batch",
      "input": {"batch_id": 123, "items": 50}
    }
  }
}
```

### Multi-Region State
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "global_state",
    "arguments": {
      "region": "us-west-2",
      "data": {"cache_warmed": true}
    }
  }
}
```

## Tools

- **`store_app_state`** - Store application state with automatic TTL
- **`get_app_state`** - Retrieve application state from any region
- **`lambda_operation`** - Simulate serverless function with persistent state
- **`dynamodb_info`** - View DynamoDB capabilities and AWS integration
- **`global_state`** - Demonstrate multi-region Global Tables

## Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   MCP Server    │    │  SessionStorage │    │  AWS DynamoDB   │
│                 │    │      Trait      │    │                 │
│ ┌─────────────┐ │    │                 │    │ ┌─────────────┐ │
│ │SessionManager│◄────┤DynamoDbStorage  │────┤ │mcp-sessions │ │
│ │             │ │    │                 │    │ │   (table)   │ │
│ └─────────────┘ │    │                 │    │ └─────────────┘ │
│                 │    └─────────────────┘    │                 │
└─────────────────┘                           │ ┌─────────────┐ │
                                              │ │Global Tables│ │ 
                                              │ │Multi-region │ │
                                              │ └─────────────┘ │
                                              └─────────────────┘
```

## AWS Configuration

### DynamoDB Table Schema
```
Table: mcp-sessions
Partition Key: session_id (String)
TTL Attribute: ttl (Number) 
Global Tables: Enabled for multi-region
```

### IAM Policy Example
```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "dynamodb:GetItem",
        "dynamodb:PutItem", 
        "dynamodb:UpdateItem",
        "dynamodb:DeleteItem",
        "dynamodb:Query",
        "dynamodb:Scan"
      ],
      "Resource": "arn:aws:dynamodb:*:*:table/mcp-sessions*"
    }
  ]
}
```

## Deployment Scenarios

### AWS Lambda
```yaml
# serverless.yml
functions:
  mcpServer:
    handler: target/lambda/mcp-server/bootstrap
    runtime: provided.al2
    environment:
      DYNAMODB_TABLE: ${self:service}-sessions-${sls:stage}
    iamRoleStatements:
      - Effect: Allow
        Action: dynamodb:*
        Resource: !GetAtt SessionsTable.Arn
```

### ECS/Fargate
```yaml
# docker-compose.yml
version: '3'
services:
  mcp-server:
    build: .
    environment:
      - DYNAMODB_TABLE=mcp-sessions
      - AWS_REGION=us-east-1
    ports:
      - "8062:8062"
```

### Multi-Region Setup
1. **Enable Global Tables** in DynamoDB console
2. **Select regions** for replication
3. **Deploy servers** in multiple regions
4. **Configure load balancer** for geo-routing

## Monitoring and Observability

### CloudWatch Metrics
- DynamoDB read/write capacity utilization
- Throttling events
- Error rates
- Latency metrics

### CloudTrail Logging
- API call audit trail
- Data access patterns
- Security monitoring

### Custom Metrics
```rust
// Example: Custom CloudWatch metrics
let client = CloudWatchClient::new(&region);
client.put_metric_data()
  .namespace("MCP/Sessions")
  .metric_data(MetricDatum::builder()
    .metric_name("SessionCreated")
    .value(1.0)
    .build())
  .send()
  .await?;
```

## Cost Optimization

### On-Demand vs Provisioned
- **On-Demand**: Pay per request (recommended for variable workloads)  
- **Provisioned**: Fixed capacity (cheaper for predictable workloads)

### TTL Configuration
```rust
let config = DynamoDbConfig {
    session_ttl_hours: 24,    // Adjust based on needs
    event_ttl_hours: 6,       // Events expire faster
    max_events_per_session: 1000, // Limit storage per session
    ..Default::default()
};
```

## Troubleshooting

**Access Denied**: Check IAM permissions and policies
**Table Not Found**: Verify table name and region configuration
**Throttling**: Enable auto-scaling or increase provisioned capacity
**Hot Partitions**: Review partition key design and access patterns
**High Costs**: Monitor usage in CloudWatch and adjust TTL settings

## Database Management

### Setup Binary

The `dynamodb-setup` binary automates DynamoDB table creation:

```bash
# Uses default settings (tables: mcp-sessions, mcp-session-events)
cargo run --bin dynamodb-setup

# With custom environment variables
AWS_REGION="us-west-2" \
DYNAMODB_SESSIONS_TABLE="my-sessions" \
DYNAMODB_EVENTS_TABLE="my-events" \
ENVIRONMENT="production" \
cargo run --bin dynamodb-setup
```

**What it does:**
- Creates DynamoDB tables with optimal schema design
- Configures Global Secondary Indexes (GSI) for efficient queries
- Sets up DynamoDB Streams for real-time change capture
- Enables Time-to-Live (TTL) for automatic cleanup
- Applies proper tags for AWS resource management
- Uses Pay-per-Request billing (serverless scaling)

**Table Schema Created:**
- **Sessions Table**: `session_id` (partition key) + GSIs for `last_activity` and `created_at`
- **Events Table**: `session_id` (partition key) + `event_id` (sort key) + GSI for `timestamp`

### Teardown Binary

The `dynamodb-teardown` binary provides flexible cleanup options:

```bash
# Interactive mode - choose what to clean up
cargo run --bin dynamodb-teardown

# Command line options
cargo run --bin dynamodb-teardown -- --clear-data       # Clear table data only
cargo run --bin dynamodb-teardown -- --backup           # Create on-demand backups
cargo run --bin dynamodb-teardown -- --disable-streams  # Disable streams (save costs)
cargo run --bin dynamodb-teardown -- --delete           # Delete tables completely
cargo run --bin dynamodb-teardown -- --all              # Backup + delete
```

**Cleanup Options:**
1. **Clear table data** - Scan and delete all items (keeps table structure)
2. **Create backups** - On-demand backups with timestamp naming
3. **Disable streams** - Turn off DynamoDB Streams to reduce costs
4. **Delete tables** - Complete table deletion (⚠️ irreversible)
5. **Full cleanup** - Backup tables then delete everything

**⚠️ Important**: Table deletion is permanent! Always create backups for production data.

### Environment Variables

Both binaries support these environment variables:

```bash
export AWS_REGION="us-east-1"                          # AWS region
export DYNAMODB_SESSIONS_TABLE="mcp-sessions"          # Sessions table name
export DYNAMODB_EVENTS_TABLE="mcp-session-events"      # Events table name  
export ENVIRONMENT="development"                       # Environment tag
```

### Cost Management

**Monitoring Usage:**
```bash
# Check current table status and costs
aws dynamodb describe-table --table-name mcp-sessions

# Monitor consumed capacity
aws cloudwatch get-metric-statistics \
  --namespace AWS/DynamoDB \
  --metric-name ConsumedReadCapacityUnits \
  --dimensions Name=TableName,Value=mcp-sessions
```

**Backup Management:**
```bash
# List all backups
aws dynamodb list-backups --table-name mcp-sessions

# Restore from backup (if needed)
aws dynamodb restore-table-from-backup \
  --target-table-name mcp-sessions-restored \
  --backup-arn arn:aws:dynamodb:us-east-1:123456789012:backup/01234567890123456789012345678901
```

## Local Development

Use DynamoDB Local for development:
```bash
# Start DynamoDB Local
docker run -p 8000:8000 amazon/dynamodb-local -jar DynamoDBLocal.jar -sharedDb

# Run with local endpoint
AWS_ENDPOINT_URL="http://localhost:8000" cargo run --bin server

# Create table manually if needed
aws dynamodb create-table \
  --endpoint-url http://localhost:8000 \
  --table-name mcp-sessions \
  --attribute-definitions AttributeName=session_id,AttributeType=S \
  --key-schema AttributeName=session_id,KeyType=HASH \
  --provisioned-throughput ReadCapacityUnits=5,WriteCapacityUnits=5
```