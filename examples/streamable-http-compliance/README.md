# Streamable HTTP MCP 2025-06-18 Compliance Test Suite

This example provides comprehensive testing for **Streamable HTTP** compliance according to the MCP 2025-06-18 specification.

## 🎯 What This Tests

### Core Streamable HTTP Features
- ✅ **Server-Sent Events (SSE)** for real-time notifications
- ✅ **Progress notifications** with `progressToken` tracking
- ✅ **System notifications** with fan-out to all active sessions
- ✅ **Session-specific notifications** targeted to individual clients
- ✅ **Last-Event-ID header** support for SSE resumability
- ✅ **HTTP 202 Accepted** status for notifications per MCP 2025-06-18
- ✅ **Proper Content-Type headers** (`text/event-stream`)

### MCP 2025-06-18 Specification Compliance
- ✅ **Protocol version negotiation** (2025-06-18)
- ✅ **Session management** via `Mcp-Session-Id` headers
- ✅ **_meta field handling** in requests and responses
- ✅ **Tool execution** with progress tracking
- ✅ **Notification system** with proper method names
- ✅ **Error handling** and reconnection strategies

## 🚀 Quick Start Commands

### Option 1: Automated Testing (Recommended)

```bash
# Navigate to the MCP framework root directory
cd /path/to/mcp-framework

# Terminal 1: Start the test server
cargo run -p streamable-http-compliance --bin streamable-http-compliance

# Terminal 2: Run the automated compliance tests
cargo run -p streamable-http-compliance --bin client
```

### Option 2: Manual Step-by-Step Testing

```bash
# Start the server (Terminal 1)
cargo run -p streamable-http-compliance --bin streamable-http-compliance

# The server will display test commands to copy/paste
# Server runs on: http://127.0.0.1:8001/mcp
```

## 📋 Manual Testing Commands

When you start the server, it will display these exact commands to copy/paste:

### Initialize MCP Connection
```bash
curl -X POST http://127.0.0.1:8001/mcp \
  -H 'Content-Type: application/json' \
  -H 'Mcp-Session-Id: test-session-123' \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'
```

### Trigger Long Calculation (Progress Notifications)
```bash
curl -X POST http://127.0.0.1:8001/mcp \
  -H 'Content-Type: application/json' \
  -H 'Mcp-Session-Id: test-session-123' \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"long_calculation","arguments":{"number":5,"delay_ms":1000}}}'
```

### Check System Health (System Notifications)
```bash
curl -X POST http://127.0.0.1:8001/mcp \
  -H 'Content-Type: application/json' \
  -H 'Mcp-Session-Id: test-session-123' \
  -d '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"system_health","arguments":{"check_type":"cpu"}}}'
```

### Listen to SSE Stream (Separate Terminal)
```bash
curl -N -H 'Accept: text/event-stream' \
     -H 'Mcp-Session-Id: test-session-123' \
     -H 'Last-Event-ID: 0' \
     http://127.0.0.1:8001/mcp
```

## 📊 Expected Test Results

### SSE Events You Should See

1. **Progress Notifications** (`notifications/progress`)
   ```json
   {
     "jsonrpc": "2.0",
     "method": "notifications/progress", 
     "params": {
       "progressToken": "calc-12345678-1234-5678-9012-123456789012",
       "progress": 60,
       "total": 100,
       "message": "Calculating step 3/5"
     }
   }
   ```

2. **System Notifications** (`notifications/system`)
   ```json
   {
     "jsonrpc": "2.0",
     "method": "notifications/system",
     "params": {
       "notification_type": "metric_update",
       "component": "cpu_usage", 
       "status": "warning",
       "value": 92.1,
       "timestamp": 1640995200
     }
   }
   ```

3. **Session Notifications** (session-specific)
   ```json
   {
     "jsonrpc": "2.0",
     "method": "notifications/session",
     "params": {
       "session_id": "test-session-123",
       "action": "tool_completed",
       "details": "Long calculation finished"
     }
   }
   ```

### HTTP Response Verification

- ✅ **Notifications return `202 Accepted`** (not 200 OK)
- ✅ **SSE returns `200 OK`** with `Content-Type: text/event-stream`
- ✅ **Last-Event-ID header** is properly handled for resumption
- ✅ **CORS headers** are present for browser compatibility

## 🔧 Advanced Testing Scenarios

### Last-Event-ID Resumability Test

```bash
# 1. Start SSE connection and note some event IDs
curl -N -H "Accept: text/event-stream" \
     -H "Mcp-Session-Id: resume-test" \
     http://127.0.0.1:8001/mcp

# 2. Disconnect and reconnect with Last-Event-ID
curl -N -H "Accept: text/event-stream" \
     -H "Mcp-Session-Id: resume-test" \
     -H "Last-Event-ID: 42" \
     http://127.0.0.1:8001/mcp

# Should only receive events after ID 42
```

### Multi-Session Fan-out Test

```bash
# Terminal A: Session 1
curl -N -H "Accept: text/event-stream" \
     -H "Mcp-Session-Id: session-1" \
     http://127.0.0.1:8001/mcp

# Terminal B: Session 2  
curl -N -H "Accept: text/event-stream" \
     -H "Mcp-Session-Id: session-2" \
     http://127.0.0.1:8001/mcp

# Terminal C: Trigger system notification
curl -X POST http://127.0.0.1:8001/mcp \
  -H "Content-Type: application/json" \
  -H "Mcp-Session-Id: session-1" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"system_health","arguments":{"check_type":"memory"}}}'

# Both sessions should receive the system notification
```

## 🧪 Running Tests

```bash
# Build everything first
cargo build -p streamable-http-compliance

# Run unit tests (if available)
cargo test -p streamable-http-compliance

# Run integration tests with server + client
# Terminal 1:
cargo run -p streamable-http-compliance --bin streamable-http-compliance

# Terminal 2:
cargo run -p streamable-http-compliance --bin client
```

## 📋 Compliance Checklist

After running the test suite, verify these items:

- [ ] **SSE Connection**: `Content-Type: text/event-stream` ✅
- [ ] **Protocol Version**: Server responds with `2025-06-18` ✅  
- [ ] **Notification Status**: Returns `202 Accepted` ✅
- [ ] **Progress Tracking**: Events contain valid `progressToken` ✅
- [ ] **System Fan-out**: All sessions receive system notifications ✅
- [ ] **Session Isolation**: Session-specific notifications target correctly ✅
- [ ] **Event Resumability**: `Last-Event-ID` header works ✅
- [ ] **Error Handling**: Graceful disconnection and reconnection ✅
- [ ] **Meta Fields**: `_meta` data preserved in events ✅
- [ ] **JSON-RPC Format**: All notifications use proper JSON-RPC 2.0 ✅

## 🎉 Success Criteria

✅ **FULLY COMPLIANT** when all tests pass:
- SSE stream establishes successfully
- Progress notifications received during long operations
- System notifications fan out to all sessions  
- HTTP status codes match MCP 2025-06-18 specification
- Event resumability works with Last-Event-ID
- All JSON-RPC notifications are properly formatted

This comprehensive test suite validates that the framework's Streamable HTTP implementation meets and exceeds the MCP 2025-06-18 specification requirements.

## 🛠️ **Troubleshooting**

### Port Already in Use
```bash
# If port 8001 is busy, kill existing processes:
pkill -f "streamable-http-compliance"

# Or find and kill specific process:
lsof -ti:8001 | xargs kill
```

### Build Issues
```bash
# Clean rebuild if needed:
cargo clean
cargo build -p streamable-http-compliance

# Check all dependencies:
cargo check -p streamable-http-compliance
```

### Client Timeout
- Ensure server is running first
- Wait 2-3 seconds after starting server before running client
- Check server logs for any errors

---

## 📋 **Quick Command Reference**

### Essential Commands (Copy & Run)

```bash
# Navigate to MCP framework directory
cd /path/to/mcp-framework

# Terminal 1: Start test server
cargo run -p streamable-http-compliance --bin streamable-http-compliance

# Terminal 2: Run automated tests
cargo run -p streamable-http-compliance --bin client
```

**That's it!** The automated client will run all compliance tests and report results.

### Expected Output
```
✅ MCP connection initialized for session: compliance-test-session
✅ Protocol version 2025-06-18 confirmed
✅ Notifications return 202 Accepted (MCP 2025-06-18 compliant)
✅ Tool 'long_calculation' called successfully
✅ Tool 'system_health' called successfully
✅ SSE connection established (Content-Type: text/event-stream)
✅ All compliance tests passed!
```

---