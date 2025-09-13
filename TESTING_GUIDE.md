# MCP Framework Testing & Verification Guide

Complete guide for running and verifying MCP 2025-06-18 compliance in the turul-mcp-framework.

## Quick Start - Verify Everything Works

### 1. Build Framework
```bash
# Build entire framework
cargo build --workspace

# Verify no compilation errors
echo "✅ Framework builds successfully"
```

### 2. Run Compliance Tests
```bash
# Run full MCP compliance validation
cargo test -p turul-mcp-framework-integration-tests --test mcp_runtime_capability_validation -- --nocapture

# Expected output: all 4 tests pass
# ✅ test_tools_capability_truthfulness ... ok
# ✅ test_json_rpc_protocol_compliance ... ok  
# ✅ test_empty_server_capabilities ... ok
# ✅ integration::test_full_mcp_compliance_integration ... ok
```

### 3. Manual Server Verification
```bash
# Test a simple server (runs on random port)
cargo run --example minimal-server

# In another terminal, verify it responds
curl -X POST http://127.0.0.1:PORT/mcp \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}'

# Should return valid JSON-RPC response with server info
```

## Server Running Instructions

### Core Test Servers

#### 1. Resource Test Server
**Purpose**: Comprehensive resource testing with 17 different resource types

```bash
# Start server (check logs for actual port)
cargo run --package resource-test-server

# Or specify port
cargo run --package resource-test-server -- --port 8080

# Expected log output:
# INFO resource_test_server: 🚀 Starting MCP Resource Test Server on port 8080
# INFO turul_mcp_server::builder: 🔧 Auto-configured server capabilities:
# INFO turul_mcp_server::builder:    - Resources: true
```

**Resources Available**:
- `file:///tmp/test.txt` - File reading with error handling
- `memory://data` - Fast in-memory JSON data  
- `template://items/{id}` - URI template variables
- `session://info` - Session-aware resource
- `subscribe://updates` - Subscription support
- And 12 more specialized test resources

#### 2. Prompts Test Server  
**Purpose**: Comprehensive prompt testing with 11 different prompt types

```bash
# Start server
cargo run --package prompts-test-server -- --port 8081

# Expected log output:
# INFO prompts_test_server: 🚀 Starting MCP Prompts Test Server on port 8081
# INFO turul_mcp_server::builder: 🔧 Auto-configured server capabilities:
# INFO turul_mcp_server::builder:    - Prompts: true
```

**Prompts Available**:
- `simple_prompt` - Basic single-message prompt
- `string_args_prompt` - String argument validation
- `multi_message_prompt` - User/assistant conversation
- `session_aware_prompt` - Session-dependent prompts
- And 7 more specialized test prompts

#### 3. Comprehensive Server
**Purpose**: All MCP handlers enabled in one server

```bash
# Start comprehensive server
cargo run --package comprehensive-server -- --port 8082

# Expected log output shows ALL capabilities:
# INFO turul_mcp_server::builder: 🔧 Auto-configured server capabilities:
# INFO turul_mcp_server::builder:    - Tools: true
# INFO turul_mcp_server::builder:    - Resources: true  
# INFO turul_mcp_server::builder:    - Prompts: true
# INFO turul_mcp_server::builder:    - Roots: true
# INFO turul_mcp_server::builder:    - Elicitation: true
# INFO turul_mcp_server::builder:    - Completions: true
# INFO turul_mcp_server::builder:    - Logging: enabled
```

### Example Servers

```bash
# Simple servers (great for quick testing)
cargo run --example minimal-server                    # Port 8000
cargo run --example calculator-add-simple-server-derive  # Port 8001

# Feature-specific servers
cargo run --example resources-server -- --port 8041   # Resources only
cargo run --example prompts-server -- --port 8040     # Prompts only
cargo run --example notification-server               # SSE notifications

# Advanced servers  
cargo run --example stateful-server                   # Session management
cargo run --example performance-testing               # Load testing
```

## Manual MCP Compliance Verification

### Step 1: Initialize Connection

**Test any server** (replace PORT with actual port from logs):

```bash
curl -X POST http://127.0.0.1:PORT/mcp \
  -H 'Content-Type: application/json' \
  -d '{
    "jsonrpc": "2.0",
    "method": "initialize", 
    "params": {
      "protocolVersion": "2025-06-18",
      "capabilities": {
        "tools": {"listChanged": true},
        "resources": {"subscribe": true, "listChanged": true},
        "prompts": {"listChanged": true}
      },
      "clientInfo": {
        "name": "manual-test-client",
        "version": "1.0.0"
      }
    },
    "id": "init-1"
  }' | jq
```

**✅ Expected Response Structure**:
```json
{
  "jsonrpc": "2.0",
  "id": "init-1", 
  "result": {
    "protocolVersion": "2025-06-18",
    "serverInfo": {
      "name": "server-name",
      "version": "0.2.0"
    },
    "capabilities": {
      "tools": {"listChanged": false},
      "resources": {"subscribe": false, "listChanged": false},
      "prompts": {"listChanged": false}
    }
  }
}
```

**🔍 Compliance Checks**:
- ✅ `protocolVersion` must be `"2025-06-18"`
- ✅ `capabilities.tools.listChanged` should be `false` (static framework)
- ✅ Response includes `serverInfo` with `name` and `version`
- ✅ No extra fields or non-spec extensions

### Step 2: Verify Capabilities Truthfulness

**Extract session ID** from Set-Cookie header or response:
```bash
# Save session ID (look for Set-Cookie or Mcp-Session-Id in headers)
SESSION_ID="your-uuid-v7-session-id-here"
```

**Test each capability** the server advertises:

#### If `tools: {}` capability exists:
```bash
curl -X POST http://127.0.0.1:PORT/mcp \
  -H 'Content-Type: application/json' \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/list",
    "params": {},
    "id": "tools-1"
  }' | jq
```

#### If `resources: {}` capability exists:
```bash
curl -X POST http://127.0.0.1:PORT/mcp \
  -H 'Content-Type: application/json' \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{
    "jsonrpc": "2.0", 
    "method": "resources/list",
    "params": {},
    "id": "resources-1"
  }' | jq
```

#### If `prompts: {}` capability exists:
```bash
curl -X POST http://127.0.0.1:PORT/mcp \
  -H 'Content-Type: application/json' \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{
    "jsonrpc": "2.0",
    "method": "prompts/list", 
    "params": {},
    "id": "prompts-1"
  }' | jq
```

**✅ All requests should return valid responses, NOT errors**

### Step 3: Test Core Operations

#### Test Tool Call (if tools available):
```bash
# First get available tools
TOOLS_RESPONSE=$(curl -s -X POST http://127.0.0.1:PORT/mcp \
  -H 'Content-Type: application/json' \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{"jsonrpc":"2.0","method":"tools/list","params":{},"id":"get-tools"}')

echo $TOOLS_RESPONSE | jq '.result.tools[0].name'

# Call the first tool (adjust arguments as needed)
curl -X POST http://127.0.0.1:PORT/mcp \
  -H 'Content-Type: application/json' \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "TOOL_NAME_FROM_ABOVE",
      "arguments": {}
    },
    "id": "call-1"
  }' | jq
```

#### Test Resource Read (if resources available):
```bash
# Get available resources
RESOURCES_RESPONSE=$(curl -s -X POST http://127.0.0.1:PORT/mcp \
  -H 'Content-Type: application/json' \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{"jsonrpc":"2.0","method":"resources/list","params":{},"id":"get-resources"}')

echo $RESOURCES_RESPONSE | jq '.result.resources[0].uri'

# Read the first resource  
curl -X POST http://127.0.0.1:PORT/mcp \
  -H 'Content-Type: application/json' \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{
    "jsonrpc": "2.0",
    "method": "resources/read",
    "params": {
      "uri": "RESOURCE_URI_FROM_ABOVE"  
    },
    "id": "read-1"
  }' | jq
```

### Step 4: Test SSE Event Stream

**Open SSE connection** (in separate terminal):
```bash
curl -N -H 'Accept: text/event-stream' \
  -H "Mcp-Session-Id: $SESSION_ID" \
  http://127.0.0.1:PORT/mcp
```

**Expected SSE output**:
```
event: heartbeat
data: ping

event: notification  
data: {"method":"test","params":{}}
```

### Step 5: Verify Resource Templates (if supported)

```bash
curl -X POST http://127.0.0.1:PORT/mcp \
  -H 'Content-Type: application/json' \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{
    "jsonrpc": "2.0",
    "method": "resources/templates/list",
    "params": {},
    "id": "templates-1"
  }' | jq
```

## Comprehensive Testing Scripts

### Quick Compliance Check Script
```bash
#!/bin/bash
# save as: quick_compliance_check.sh

PORT=${1:-8080}
echo "🧪 Testing MCP server on port $PORT"

# Test initialize
echo "1. Testing initialize..."
INIT_RESPONSE=$(curl -s -X POST http://127.0.0.1:$PORT/mcp \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}')

echo $INIT_RESPONSE | jq '.result.protocolVersion'

if [[ $(echo $INIT_RESPONSE | jq -r '.result.protocolVersion') == "2025-06-18" ]]; then
    echo "✅ Protocol version correct"
else
    echo "❌ Protocol version incorrect"
    exit 1
fi

# Extract session if available
SESSION_ID=$(echo $INIT_RESPONSE | jq -r '.result.sessionId // empty')
if [[ -n "$SESSION_ID" ]]; then
    echo "✅ Session ID: $SESSION_ID"
fi

# Test ping
echo "2. Testing ping..." 
PING_RESPONSE=$(curl -s -X POST http://127.0.0.1:$PORT/mcp \
  -H 'Content-Type: application/json' \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{"jsonrpc":"2.0","method":"ping","params":{},"id":2}')

if [[ $(echo $PING_RESPONSE | jq -r '.result') == "{}" ]]; then
    echo "✅ Ping successful"
else
    echo "❌ Ping failed"
fi

echo "🎉 Basic compliance verified"
```

**Usage**:
```bash
chmod +x quick_compliance_check.sh

# Test any server
cargo run --example minimal-server &
./quick_compliance_check.sh 8000
```

## Troubleshooting Common Issues

### Server Won't Start
```bash
# Check if port is busy
lsof -i :8080

# Kill existing processes
pkill -f "server-name"

# Check compilation errors
cargo check --package server-name
```

### Connection Refused
```bash
# Verify server is running
ps aux | grep server-name

# Check server logs
RUST_LOG=info cargo run --package server-name

# Test with telnet
telnet 127.0.0.1 8080
```

### Invalid JSON Responses
```bash
# Check response format
curl -v http://127.0.0.1:8080/mcp \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"initialize","params":{},"id":1}' | jq .

# Enable debug logging
RUST_LOG=debug cargo run --package server-name
```

### Session Issues
```bash
# Verify session ID format (should be UUID v7)
echo $SESSION_ID | grep -E '^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$'

# Check session headers
curl -I http://127.0.0.1:8080/mcp
```

## Performance Testing

### Load Testing with Multiple Clients
```bash
# Run performance test server
cargo run --example performance-testing &

# Run concurrent tests  
for i in {1..10}; do
  (./quick_compliance_check.sh 8000 &)
done
wait

echo "✅ Concurrent client test complete"
```

### Memory and Resource Usage
```bash
# Monitor server resource usage
cargo run --example comprehensive-server &
SERVER_PID=$!

# Watch memory usage
watch -n 1 "ps -p $SERVER_PID -o pid,ppid,cmd,%mem,%cpu"

# Send test requests
for i in {1..100}; do
  curl -s -X POST http://127.0.0.1:8002/mcp \
    -H 'Content-Type: application/json' \
    -d '{"jsonrpc":"2.0","method":"ping","params":{},"id":'$i'}' > /dev/null
done

kill $SERVER_PID
```

## Continuous Integration Testing

### GitHub Actions / CI Pipeline
```bash
# Full test suite for CI
cargo test --workspace --verbose
cargo test -p turul-mcp-framework-integration-tests --test mcp_runtime_capability_validation
cargo build --workspace --release

# Verify examples compile
for example in examples/*/; do
  cargo check --manifest-path "$example/Cargo.toml"
done
```

This guide provides comprehensive instructions for running and manually verifying MCP 2025-06-18 compliance across the entire framework.