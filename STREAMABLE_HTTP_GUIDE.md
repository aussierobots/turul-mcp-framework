# MCP Streamable HTTP Transport Implementation Guide

**Status**: ✅ **COMPLETE IMPLEMENTATION** - Production-ready MCP 2025-06-18 Streamable HTTP Transport  
**Purpose**: Comprehensive guide for understanding and using the MCP Framework's Streamable HTTP implementation

## 🚀 **Quick Start**

### Basic Usage
```bash
# Start server
cargo run --example client-initialise-server -- --port 52935

# Test complete MCP Streamable HTTP compliance
export RUST_LOG=debug
cargo run --example client-initialise-report -- --url http://127.0.0.1:52935/mcp
# Output: "🎆 FULLY MCP COMPLIANT: Session management + Streamable HTTP working!"
```

### Zero-Configuration Server
```rust
use mcp_server::McpServer;
use mcp_derive::mcp_tool;

#[mcp_tool(name = "echo", description = "Echo a message")]
async fn echo_tool(
    #[param(description = "Message to echo")] message: String
) -> Result<String, String> {
    Ok(format!("Echo: {}", message))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = McpServer::builder()
        .tool_fn(echo_tool)  // Framework automatically maps to "tools/call"
        .build()?;
    
    server.listen_http("127.0.0.1:8000").await?;
    Ok(())
}
```

## 🏗️ **MCP Streamable HTTP Architecture**

### Protocol Overview
MCP Streamable HTTP Transport allows HTTP requests to return either:
1. **Standard JSON Responses** - Traditional request/response
2. **SSE Streams** - Real-time event streaming when `Accept: text/event-stream`

### Request Patterns

#### 1. POST Requests with JSON Response
```http
POST /mcp
Content-Type: application/json
Accept: application/json

{"jsonrpc":"2.0","method":"tools/call","params":{"name":"echo","arguments":{"message":"Hello"}}}
```
**Response**: Standard JSON-RPC response

#### 2. POST Requests with SSE Response  
```http
POST /mcp
Content-Type: application/json
Accept: text/event-stream
Mcp-Session-Id: 01234567-89ab-cdef-0123-456789abcdef

{"jsonrpc":"2.0","method":"tools/call","params":{"name":"echo","arguments":{"message":"Hello"}}}
```
**Response**: SSE stream containing tool result + real-time notifications

#### 3. GET Requests for Persistent Streams
```http
GET /mcp
Accept: text/event-stream
Mcp-Session-Id: 01234567-89ab-cdef-0123-456789abcdef
Last-Event-ID: 123
```
**Response**: Persistent SSE stream for server-initiated events with resumability

## 🔧 **Implementation Details**

### Session Management

#### Session Lifecycle
1. **Initialize Request**: Client sends `initialize` method
2. **Server Creates Session**: Server generates UUID v7 session ID  
3. **Session Header**: Server returns session ID via `Mcp-Session-Id` header
4. **Client Uses Session**: All subsequent requests include session ID header
5. **Session Context**: Tools receive SessionContext with notification capabilities

```rust
// SessionContext automatically provided to tools
#[mcp_tool]
async fn long_task(
    #[param] task_name: String,
    session: mcp_server::SessionContext  // Optional parameter
) -> Result<String, String> {
    // Send progress notification
    session.notify_progress("task-123", 50, Some(100), Some("Processing..."));
    
    // Send log message
    session.notify_log("info", "Task completed successfully");
    
    Ok("Task complete".to_string())
}
```

#### UUID v7 Sessions
- **Temporal Ordering**: Sessions are ordered by creation time
- **Database Friendly**: Better performance and indexing
- **Server Generated**: Never client-generated (MCP protocol compliance)

```rust
// Server automatically creates sessions
use uuid::Uuid;
let session_id = Uuid::now_v7(); // Time-ordered UUID
```

### Real-time Notifications

#### MCP Notification Types
```rust
// 1. Progress Notifications
session.notify_progress(
    "progress-token-123",  // progressToken
    75,                    // progress (0-100)
    Some(100),            // total (optional)
    Some("Almost done")    // message (optional)
);

// 2. Log/Message Notifications  
session.notify_log(
    "info",               // level (debug/info/warning/error)
    "Processing complete" // message
);

// 3. Resource List Changed (server-initiated)
// 4. Resource Updated (server-initiated)
// 5. Tools List Changed (server-initiated)
// 6. Request Cancelled (client-initiated)
```

#### Notification Format (JSON-RPC)
All notifications use proper MCP JSON-RPC format:
```json
{
  "jsonrpc": "2.0",
  "method": "notifications/progress",
  "params": {
    "progressToken": "task-123",
    "progress": 75,
    "total": 100,
    "message": "Processing..."
  }
}
```

### SSE Stream Implementation

#### Stream Types
1. **POST SSE Response**: Tool execution + notifications in single stream
2. **GET Persistent Stream**: Server-initiated events, resumable

#### Event Format
```
id: 42
event: data
data: {"jsonrpc":"2.0","method":"notifications/progress","params":{"progressToken":"task-123","progress":50}}

id: 43  
event: data
data: {"jsonrpc":"2.0","method":"notifications/message","params":{"level":"info","message":"Task completed"}}

```

#### Resumability with Last-Event-ID
```http
GET /mcp
Accept: text/event-stream
Mcp-Session-Id: session-uuid
Last-Event-ID: 42
```
Server replays events 43+ and continues with real-time events.

## 🔌 **Integration Examples**

### Client Implementation (Rust)
```rust
use reqwest;
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    
    // 1. Initialize and get session ID
    let init_response = client
        .post("http://127.0.0.1:8000/mcp")
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "initialize",
            "id": 1,
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "test-client", "version": "1.0"}
            }
        }))
        .send()
        .await?;
    
    let session_id = init_response
        .headers()
        .get("Mcp-Session-Id")
        .unwrap()
        .to_str()?;
    
    // 2. Call tool with SSE response
    let sse_response = client
        .post("http://127.0.0.1:8000/mcp")
        .header("Content-Type", "application/json")
        .header("Accept", "text/event-stream")
        .header("Mcp-Session-Id", session_id)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "id": 2,
            "params": {
                "name": "long_task",
                "arguments": {"task_name": "process_data"}
            }
        }))
        .send()
        .await?;
    
    // 3. Process SSE events
    let mut stream = sse_response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let data = chunk?;
        println!("SSE Event: {}", String::from_utf8_lossy(&data));
    }
    
    Ok(())
}
```

### JavaScript/TypeScript Client
```typescript
// 1. Initialize session
const initResponse = await fetch('http://127.0.0.1:8000/mcp', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    jsonrpc: '2.0',
    method: 'initialize',
    id: 1,
    params: {
      protocolVersion: '2025-06-18',
      capabilities: {},
      clientInfo: { name: 'web-client', version: '1.0' }
    }
  })
});

const sessionId = initResponse.headers.get('Mcp-Session-Id');

// 2. Create persistent SSE connection
const eventSource = new EventSource(`http://127.0.0.1:8000/mcp`, {
  headers: {
    'Accept': 'text/event-stream',
    'Mcp-Session-Id': sessionId
  }
});

eventSource.onmessage = (event) => {
  const notification = JSON.parse(event.data);
  console.log('Notification:', notification);
  
  if (notification.method === 'notifications/progress') {
    updateProgressBar(notification.params.progress);
  }
};

// 3. Call tools (separate HTTP requests)
const toolResponse = await fetch('http://127.0.0.1:8000/mcp', {
  method: 'POST',
  headers: { 
    'Content-Type': 'application/json',
    'Mcp-Session-Id': sessionId
  },
  body: JSON.stringify({
    jsonrpc: '2.0',
    method: 'tools/call',
    id: 2,
    params: { name: 'long_task', arguments: { task_name: 'process_data' } }
  })
});
```

## ⚙️ **Configuration & Customization**

### Custom SessionStorage Backend
```rust
use mcp_session_storage::SessionStorage;
use async_trait::async_trait;

struct RedisSessionStorage {
    client: redis::Client,
}

#[async_trait]
impl SessionStorage for RedisSessionStorage {
    type Error = redis::RedisError;
    
    async fn create_session(&self, capabilities: ServerCapabilities) -> Result<SessionInfo, Self::Error> {
        let session_id = uuid::Uuid::now_v7().to_string();
        let session_info = SessionInfo::new_with_id(session_id.clone());
        
        // Store in Redis
        let mut conn = self.client.get_async_connection().await?;
        conn.set(&session_id, serde_json::to_string(&session_info)?).await?;
        
        Ok(session_info)
    }
    
    // ... implement other SessionStorage methods
}

// Use custom backend
let storage = Arc::new(RedisSessionStorage { client: redis_client });
let server = McpServer::builder()
    .tool_fn(my_tool)
    .with_session_storage(storage)
    .build()?;
```

### Custom Notification Broadcasting
```rust
use mcp_http_server::NotificationBroadcaster;

struct NatsNotificationBroadcaster {
    nats_client: nats::Connection,
}

#[async_trait]
impl NotificationBroadcaster for NatsNotificationBroadcaster {
    async fn send_progress_notification(
        &self,
        session_id: &str,
        progress_token: &str,
        progress: u64,
        total: Option<u64>,
        message: Option<String>
    ) -> Result<(), BroadcastError> {
        let notification = JsonRpcNotification::new_with_object_params(
            "notifications/progress".to_string(),
            // ... build params
        );
        
        // Broadcast to NATS for distributed systems
        self.nats_client.publish(
            &format!("mcp.session.{}", session_id),
            serde_json::to_vec(&notification)?
        ).await?;
        
        Ok(())
    }
}
```

## 🧪 **Testing & Validation**

### Integration Testing
```rust
#[tokio::test]
async fn test_streamable_http_compliance() {
    // Start test server
    let server = McpServer::builder()
        .tool_fn(test_tool)
        .build()?;
    
    let server_handle = tokio::spawn(server.listen_http("127.0.0.1:0"));
    
    // Test initialization
    let client = reqwest::Client::new();
    let init_response = client.post("http://127.0.0.1:8000/mcp")
        .json(&init_request())
        .send().await?;
    
    assert_eq!(init_response.status(), 200);
    let session_id = init_response.headers()
        .get("Mcp-Session-Id").unwrap();
    
    // Test POST SSE response
    let sse_response = client.post("http://127.0.0.1:8000/mcp")
        .header("Accept", "text/event-stream")
        .header("Mcp-Session-Id", session_id)
        .json(&tool_call_request())
        .send().await?;
    
    assert_eq!(sse_response.status(), 200);
    assert_eq!(sse_response.headers().get("Content-Type").unwrap(), "text/event-stream");
    
    // Verify SSE events received
    let events = collect_sse_events(sse_response).await?;
    assert!(events.iter().any(|e| e.contains("notifications/progress")));
    assert!(events.iter().any(|e| e.contains("tool_result")));
}
```

### MCP Inspector Integration
```bash
# Install MCP Inspector
npm install -g @anthropic/mcp-inspector

# Test your server
mcp-inspector --url http://127.0.0.1:8000/mcp --protocol-version 2025-06-18
```

## 🚨 **Common Patterns & Best Practices**

### 1. Progressive Enhancement
```rust
#[mcp_tool]
async fn data_processor(
    #[param] dataset: String,
    session: Option<SessionContext>  // Always optional
) -> Result<String, String> {
    if let Some(ctx) = session {
        // Enhanced with real-time progress
        ctx.notify_progress("process-123", 0, Some(100), Some("Starting"));
        // ... processing with progress updates
        ctx.notify_progress("process-123", 100, Some(100), Some("Complete"));
    }
    // Works without session context too
    Ok("Processing complete".to_string())
}
```

### 2. Error Handling with Notifications
```rust
#[mcp_tool]
async fn risky_operation(
    #[param] input: String,
    session: Option<SessionContext>
) -> Result<String, String> {
    if let Some(ctx) = session {
        ctx.notify_log("info", "Starting risky operation");
    }
    
    match perform_operation(&input).await {
        Ok(result) => {
            if let Some(ctx) = session {
                ctx.notify_log("info", "Operation completed successfully");
            }
            Ok(result)
        }
        Err(e) => {
            if let Some(ctx) = session {
                ctx.notify_log("error", &format!("Operation failed: {}", e));
            }
            Err(e.to_string())
        }
    }
}
```

### 3. Long-Running Operations
```rust
#[mcp_tool]
async fn batch_processor(
    #[param] items: Vec<String>,
    session: Option<SessionContext>
) -> Result<String, String> {
    let total = items.len();
    let progress_token = format!("batch-{}", uuid::Uuid::new_v4());
    
    if let Some(ctx) = session {
        ctx.notify_progress(&progress_token, 0, Some(total as u64), Some("Starting batch"));
    }
    
    let mut results = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let result = process_item(item).await?;
        results.push(result);
        
        if let Some(ctx) = session {
            ctx.notify_progress(
                &progress_token,
                (i + 1) as u64,
                Some(total as u64),
                Some(&format!("Processed {} of {}", i + 1, total))
            );
        }
        
        // Small delay to show progress
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
    
    if let Some(ctx) = session {
        ctx.notify_log("info", &format!("Batch processing complete: {} items", total));
    }
    
    Ok(format!("Processed {} items successfully", total))
}
```

## 📊 **Performance Considerations**

### Connection Management
- **Session Cleanup**: Automatic cleanup after 30 minutes of inactivity
- **SSE Keep-Alive**: Built-in keep-alive prevents connection timeouts
- **Event Storage**: Configurable event retention for resumability

### Scaling Considerations
- **Single Instance**: InMemorySessionStorage for development/small deployments
- **Production**: SQLite/PostgreSQL SessionStorage for persistence
- **Distributed**: NATS/Redis backends for multi-instance deployments

### Monitoring
```rust
// Built-in metrics via SessionStorage
let session_count = storage.session_count().await?;
let event_count = storage.event_count().await?;
println!("Active sessions: {}, Total events: {}", session_count, event_count);
```

## 🔗 **Related Documentation**

- **MCP_SESSION_ARCHITECTURE.md** - Complete architecture reference
- **WORKING_MEMORY.md** - Quick reference for key patterns
- **BROKEN_EXAMPLES_STATUS.md** - Example fix patterns
- **CONSOLIDATED_ROADMAP.md** - Future enhancements roadmap

---

**BOTTOM LINE**: The MCP Framework provides a complete, production-ready implementation of MCP 2025-06-18 Streamable HTTP Transport with zero-configuration usage, real-time notifications, and pluggable architecture for any deployment scenario.