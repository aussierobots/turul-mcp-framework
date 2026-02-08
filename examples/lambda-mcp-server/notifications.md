# Notifications Architecture

## Overview

This document describes the notification system architecture for the Lambda MCP Server, focusing on simplicity and MCP 2025-11-25 Streamable HTTP compliance.

## Core Principle: Results vs Notifications

### Tools Return Data, Not Success Messages

**❌ WRONG - Current Implementation:**
```rust
// Tool returns generic success message, actual data lost
let result = ToolResult::text("Session information retrieved successfully");
```

**✅ CORRECT - Target Implementation:**
```rust
// Tool returns the actual data as JSON
let result = ToolResult::json(session_info); // session_info contains the actual data
```

### Notifications Are For Progress, Not Results

**Notifications should be used for:**
- ✅ Progress updates during long-running operations
- ✅ System health status changes  
- ✅ Real-time events (SNS external events)
- ✅ Background processing updates

**Notifications should NOT be used for:**
- ❌ Final tool execution results
- ❌ Returning tool output data
- ❌ Success/failure confirmations

## Architecture Components

### 1. Tool Execution Flow

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Client        │    │   MCP Server    │    │  Tool Handler   │
│   Request       │───▶│   (JSON-RPC)    │───▶│                 │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                                                        │
                                                        ▼
                               ┌─────────────────────────────────────┐
                               │ Tool Processing                      │
                               │                                     │
                               │ 1. Validate parameters             │
                               │ 2. Execute business logic          │
                               │ 3. Optional: Send progress updates │
                               │    via broadcast_global_event()    │
                               │ 4. Return actual data as result    │
                               └─────────────────────────────────────┘
                                                        │
                                                        ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Client        │◀───│   MCP Server    │◀───│ ToolResult      │
│   Gets Data     │    │   (JSON-RPC)    │    │ (actual data)   │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

### 2. Notification Broadcasting

```
┌─────────────────────────────────────────────────────────────┐
│                    Lambda MCP Instance                       │
│                                                             │
│  ┌─────────────────┐    ┌─────────────────────────────────┐  │
│  │ Tool Execution  │───▶│ tokio::broadcast channel        │  │
│  │ Progress Events │    │ (global_events.rs)              │  │
│  └─────────────────┘    └─────────────────────────────────┘  │
│                                     │                       │
│  ┌─────────────────┐                │                       │
│  │ SNS Handler     │───────────────▶│                       │
│  │ (external       │                │                       │
│  │  events)        │                │                       │
│  └─────────────────┘                │                       │
│                                     │                       │
│                                     ▼                       │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │ SSE Connections (receive notifications)                 │  │
│  │ • GET /mcp with Accept: text/event-stream               │  │
│  │ • Real-time progress updates                           │  │
│  │ • System health alerts                                 │  │
│  └─────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Implementation Details

### Tool Return Pattern

**Session Info Tool Example:**
```rust
impl McpTool for SessionInfo {
    async fn call(&self, args: Value, session: Option<SessionContext>) -> McpResult<Vec<ToolResult>> {
        // 1. Get actual session data
        let session_data = session_manager.get_session(&session_id).await?;
        
        // 2. Optional: Send progress notification
        broadcast_global_event(GlobalEvent::tool_execution(
            "session_info",
            &session_id,
            ToolExecutionStatus::InProgress { progress: Some(50.0) },
            None,
        )).await;
        
        // 3. Build actual data response
        let session_info = json!({
            "session_id": session_id,
            "created_at": session_data.created_at,
            "last_activity": session_data.last_activity,
            "capabilities": session_data.client_capabilities,
            "statistics": {
                "duration_seconds": calculate_duration(),
                "ttl_remaining": calculate_ttl_remaining()
            }
        });
        
        // 4. Return actual data, not generic success message
        Ok(vec![ToolResult::json(session_info)])
    }
}
```

### Notification Event Types

```rust
// Progress notifications during tool execution
GlobalEvent::ToolExecution {
    tool_name: String,        // "session_info", "aws_monitor"
    session_id: String,       // Current session
    status: ToolExecutionStatus, // Started, InProgress, Completed, Failed
    result: Option<Value>,    // Progress data, not final result
    timestamp: DateTime<Utc>,
}

// System health notifications
GlobalEvent::SystemHealth {
    component: String,        // "database", "auth_service"
    status: String,          // "healthy", "warning", "error"
    details: Value,          // Health details
    timestamp: DateTime<Utc>,
}

// External SNS events
GlobalEvent::ExternalEvent {
    source: String,          // "aws.ec2", "cloudwatch"
    event_type: String,      // "instance_terminated"
    payload: Value,          // External event data
    timestamp: DateTime<Utc>,
}
```

### SSE Event Format (MCP Compliant)

```
Content-Type: text/event-stream

event: tool_execution
data: {"type":"tool_execution","tool_name":"session_info","session_id":"sess-123","status":"in_progress","progress":75.0,"timestamp":"2025-01-01T12:00:00Z"}
id: 01932b12-3456-7890-abcd-ef0123456789

event: system_health
data: {"type":"system_health","component":"database","status":"healthy","details":{"connections":45},"timestamp":"2025-01-01T12:00:01Z"}
id: 01932b12-3457-7890-abcd-ef0123456789
```

## Current Issues and Fixes

### Issue 1: Tools Return Generic Messages

**Problem:**
```rust
// session_tools.rs:119
let result = ToolResult::text("Session information retrieved successfully");
```

**Fix:**
```rust
let result = ToolResult::json(session_info_data);
```

### Issue 2: No Broadcast Receivers

**Problem:**
```
Failed to broadcast global event: SendError(SessionUpdate { session_id: "unknown", event_type: InfoRequested...
```

**Fix:**
- Initialize broadcast receivers when SSE connections are established
- Handle Lambda cold start scenario where no receivers exist initially

### Issue 3: Over-notification

**Problem:**
- Broadcasting events for every tool call result
- Mixing results with progress notifications

**Fix:**
- Only broadcast for actual progress updates
- Return data in tool results, not notifications

## Testing Strategy

### 1. Tool Data Return Tests
```bash
# Test that session_info returns actual session data
curl -X POST http://127.0.0.1:9000 \
  -H "Content-Type: application/json" \
  -H "Mcp-Session-Id: test-session" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"session_info","arguments":{}}}'

# Expected: JSON response with actual session data, not "success" message
```

### 2. Notification Broadcasting Tests
```bash
# Test SSE streaming receives progress notifications
curl -X GET http://127.0.0.1:9000 \
  -H "Accept: text/event-stream" \
  -H "Mcp-Session-Id: test-session"

# Expected: Server-sent events for progress updates only
```

### 3. Integration Tests
- Verify tools return data AND can send progress notifications
- Confirm SSE subscribers receive notifications
- Test external SNS event processing

## Memory/Context Preservation

To preserve context across sessions:

1. **notifications.md** (this file): Architecture reference
2. **Update CLAUDE.md**: Add notification principles
3. **Incremental TODO tracking**: Test each change before proceeding
4. **Reference GPS project**: For working SSE patterns

## Simplicity Principles

1. **One responsibility**: Tools return data, notifications for progress
2. **Minimal complexity**: Avoid over-engineering for this example
3. **MCP compliance**: Follow 2025-11-25 Streamable HTTP specification
4. **Test-driven**: Validate each change with lambda-mcp-client

## Next Steps

1. Fix `session_info` tool to return actual data
2. Fix broadcast channel initialization
3. Test with lambda-mcp-client
4. Repeat for other tools incrementally
5. Validate SSE streaming works with real data