# Real-time Notification Server Example

This example demonstrates SSE (Server-Sent Events) real-time notifications in an MCP server, showing how to broadcast updates to connected clients for progress tracking and live updates.

## 🚀 What This Example Shows

- **Server-Sent Events (SSE)**: Real-time push notifications to clients
- **Progress Tracking**: Live progress updates for long-running operations
- **Broadcast Notifications**: Send messages to all connected SSE clients
- **Connection Management**: Monitor and report SSE connection status
- **Real-time Workflows**: Build interactive applications with live feedback

## 🛠️ Available Tools

### 1. Simulate Progress (`simulate_progress`)
Simulate a long-running operation with real-time progress updates:

**Parameters:**
- `task_name` (string): Name of the task to simulate
- `duration_seconds` (number): Duration of the task in seconds (1-60, default: 10)
- `step_count` (integer): Number of progress steps (1-20, default: 5)

**Features:**
- Sends incremental progress updates via SSE
- Provides step-by-step completion status
- Unique task ID for tracking multiple operations

### 2. Send Notification (`send_notification`)
Broadcast notifications to all connected SSE clients:

**Parameters:**
- `message` (string): Message to broadcast
- `type` (enum): Notification type ("info", "warning", "error", "success")
- `data` (object, optional): Additional data to include

**Features:**
- Broadcasts to all active SSE connections
- Supports different notification types
- Includes timestamps and unique IDs

### 3. Connection Status (`connection_status`)
Get the current SSE connection status:

**Returns:**
- SSE enablement status
- Active connection count
- Connection endpoint information
- Usage instructions

## 🏃 Running the Example

```bash
cargo run -p notification-server
```

The server starts on `http://127.0.0.1:8005/mcp` with SSE enabled.

## 🧪 Testing Real-time Notifications

### 1. Connect to SSE Stream
In one terminal, connect to receive real-time updates:
```bash
curl -H "Accept: text/event-stream" http://127.0.0.1:8005/mcp
```

### 2. Initialize MCP Connection
In another terminal, initialize the MCP connection:
```bash
curl -X POST http://127.0.0.1:8005/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "initialize",
    "params": {
      "protocolVersion": "2025-11-25",
      "capabilities": {},
      "clientInfo": {"name": "test-client", "version": "1.0.0"}
    },
    "id": "1"
  }'
```

### 3. Start a Progress Task
Send a progress simulation request:
```bash
curl -X POST http://127.0.0.1:8005/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "simulate_progress",
      "arguments": {
        "task_name": "File Processing",
        "duration_seconds": 15,
        "step_count": 10
      }
    },
    "id": "2"
  }'
```

You should see real-time progress updates in the SSE terminal:
```
data: {"type":"progress","task_id":"uuid","task_name":"File Processing","progress":10,"step":1,"total_steps":10,"message":"Completed step 1 of 10"}

data: {"type":"progress","task_id":"uuid","task_name":"File Processing","progress":20,"step":2,"total_steps":10,"message":"Completed step 2 of 10"}

...
```

### 4. Send Custom Notifications
Broadcast custom notifications:
```bash
curl -X POST http://127.0.0.1:8005/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "send_notification",
      "arguments": {
        "message": "System maintenance will begin in 10 minutes",
        "type": "warning",
        "data": {
          "maintenance_window": "2024-01-15T02:00:00Z",
          "expected_duration": "30 minutes"
        }
      }
    },
    "id": "3"
  }'
```

### 5. Check Connection Status
Monitor active SSE connections:
```bash
curl -X POST http://127.0.0.1:8005/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "connection_status",
      "arguments": {}
    },
    "id": "4"
  }'
```

## 📡 SSE Event Types

### Progress Events
```json
{
  "type": "progress",
  "task_id": "uuid-string",
  "task_name": "Task Name",
  "progress": 45.5,
  "step": 5,
  "total_steps": 10,
  "message": "Completed step 5 of 10",
  "timestamp": "2024-01-15T10:30:00Z"
}
```

### Notification Events
```json
{
  "type": "notification",
  "notification_id": "uuid-string",
  "notification_type": "warning",
  "message": "System maintenance starting",
  "timestamp": "2024-01-15T02:00:00Z",
  "data": {
    "maintenance_window": "2024-01-15T02:00:00Z",
    "expected_duration": "30 minutes"
  }
}
```

### Status Updates
```json
{
  "type": "status",
  "status": "connected",
  "connection_id": "uuid-string",
  "client_count": 3,
  "timestamp": "2024-01-15T10:30:00Z"
}
```

## 🔧 SSE Implementation Details

### Server Configuration
```rust
let server = McpServer::builder()
    .name("notification-server")
    .version("1.0.0")
    .title("Real-time Notification Server")
    .tool(ProgressTool)
    .tool(NotificationTool)
    .tool(ConnectionStatusTool)
    .bind_address("127.0.0.1:8005".parse()?)
    .sse(true)  // Enable SSE support
    .build()?;
```

### Broadcasting Notifications
```rust
// In a production implementation with SSE access:
async fn call(&self, args: Value, sse_manager: Option<SSEManager>) -> Result<Vec<ToolResult>, String> {
    if let Some(sse_manager) = sse_manager {
        sse_manager.send_data(json!({
            "type": "progress",
            "task_id": task_id,
            "progress": progress,
            "message": "Step completed"
        })).await;
    }
    
    Ok(vec![ToolResult::text("Progress sent".to_string())])
}
```

### Progress Tracking Pattern
```rust
// Simulate long-running operation with progress updates
for step in 1..=step_count {
    tokio::time::sleep(step_duration).await;
    
    let progress = (step as f64 / step_count as f64) * 100.0;
    
    // Send progress update via SSE
    sse_manager.send_data(json!({
        "type": "progress",
        "task_id": task_id,
        "progress": progress,
        "step": step,
        "total_steps": step_count
    })).await;
}
```

## 🎯 Use Cases Demonstrated

### 1. Progress Tracking
- File upload/download progress
- Data processing operations
- Backup and restore operations
- Batch job monitoring

### 2. Real-time Notifications
- System alerts and warnings
- User activity notifications
- Status change announcements
- Chat message broadcasting

### 3. Live Dashboards
- System monitoring displays
- Real-time analytics
- Live data feeds
- Activity monitoring

### 4. Interactive Applications
- Multi-user collaboration
- Live chat systems
- Real-time gaming
- Live polling and voting

## 🌐 Client-Side Integration

### JavaScript SSE Client
```javascript
const eventSource = new EventSource('http://127.0.0.1:8005/mcp');

eventSource.onmessage = function(event) {
  const data = JSON.parse(event.data);
  
  switch(data.type) {
    case 'progress':
      updateProgressBar(data.task_id, data.progress);
      break;
    case 'notification':
      showNotification(data.message, data.notification_type);
      break;
    case 'status':
      updateConnectionStatus(data.status);
      break;
  }
};

eventSource.onerror = function(event) {
  console.error('SSE connection error:', event);
};
```

### React Integration
```jsx
import { useEffect, useState } from 'react';

function useSSENotifications(url) {
  const [notifications, setNotifications] = useState([]);
  const [progress, setProgress] = useState({});
  
  useEffect(() => {
    const eventSource = new EventSource(url);
    
    eventSource.onmessage = (event) => {
      const data = JSON.parse(event.data);
      
      if (data.type === 'progress') {
        setProgress(prev => ({
          ...prev,
          [data.task_id]: data
        }));
      } else if (data.type === 'notification') {
        setNotifications(prev => [...prev, data]);
      }
    };
    
    return () => eventSource.close();
  }, [url]);
  
  return { notifications, progress };
}
```

### Python SSE Client
```python
import requests
import json

def listen_to_notifications():
    response = requests.get(
        'http://127.0.0.1:8005/mcp',
        headers={'Accept': 'text/event-stream'},
        stream=True
    )
    
    for line in response.iter_lines():
        if line.startswith(b'data: '):
            data = json.loads(line[6:])
            handle_notification(data)

def handle_notification(data):
    if data['type'] == 'progress':
        print(f"Progress: {data['progress']:.1f}% - {data['message']}")
    elif data['type'] == 'notification':
        print(f"Notification: {data['message']}")
```

## 🚨 Production Considerations

### 1. Connection Management
- Handle connection drops and reconnection
- Implement connection pooling
- Add authentication for SSE connections
- Monitor connection health

### 2. Performance Optimization
- Limit message frequency to prevent flooding
- Implement message queuing for reliability
- Add message filtering and targeting
- Use connection-specific channels

### 3. Security
- Validate SSE connection authorization
- Implement rate limiting for notifications
- Add CSRF protection
- Secure message content

### 4. Reliability
- Implement message persistence
- Add retry mechanisms for failed deliveries
- Handle network interruptions gracefully
- Provide message acknowledgment

## 🔧 Advanced Features

### Message Filtering
```rust
// Filter messages by client preferences
if sse_manager.should_send_to_client(&client_id, &message_type) {
    sse_manager.send_to_client(&client_id, &message).await;
}
```

### Targeted Notifications
```rust
// Send to specific clients
sse_manager.send_to_clients(&client_ids, &message).await;

// Send to all clients in a group
sse_manager.send_to_group("admin", &message).await;
```

### Message Persistence
```rust
// Store messages for offline clients
message_store.save(&message).await;

// Replay missed messages on reconnection
let missed_messages = message_store.get_since(&client_id, &last_seen).await;
for message in missed_messages {
    sse_manager.send_to_client(&client_id, &message).await;
}
```

## 📚 Related Examples

### Foundation Examples
- **[minimal-server](../minimal-server/)**: Basic MCP server setup
- **[stateful-server](../stateful-server/)**: Session state management

### Advanced Examples
- **[comprehensive-server](../comprehensive-server/)**: All MCP features
- **[performance-testing](../performance-testing/)**: Load testing with notifications

## 🤝 Best Practices

1. **Message Design**: Keep messages small and focused
2. **Connection Handling**: Implement proper connection lifecycle management
3. **Error Recovery**: Handle network failures gracefully
4. **Rate Limiting**: Prevent message flooding
5. **Authentication**: Secure SSE endpoints appropriately
6. **Monitoring**: Track connection health and message delivery
7. **Testing**: Test with multiple concurrent connections

---

This example demonstrates how to build real-time, interactive MCP servers using Server-Sent Events for live updates and notifications.