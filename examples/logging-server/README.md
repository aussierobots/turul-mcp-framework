# Logging Server Example

This example demonstrates the MCP logging functionality with dynamic log level management. Logging allows servers to receive log level configuration from clients and provides centralized log management for distributed MCP systems.

## Overview

The logging server provides comprehensive log management capabilities including:

1. **Dynamic Log Level Control** - Set global and per-logger log levels via MCP
2. **Log Generation** - Generate sample log entries for testing and demonstration
3. **Log Viewing** - View recent logs with filtering by level and logger
4. **Configuration Management** - View current logging configuration and settings

## Features

- **Multi-Level Logging**: Supports 8 log levels (debug, info, notice, warning, error, critical, alert, emergency)
- **Per-Logger Configuration**: Set different log levels for different loggers
- **Log History**: Maintains configurable history of log entries (default: 1000 entries)
- **Advanced Filtering**: Filter logs by level, logger name, and count
- **Real-time Configuration**: Dynamic log level changes without server restart
- **Rich Metadata**: Logs include timestamps, thread info, modules, and custom data

## Running the Server

```bash
# Run the logging server (default: 127.0.0.1:8043)
cargo run -p logging-server

# Set log level via MCP
curl -X POST http://127.0.0.1:8043/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "logging/setLevel",
    "params": {
      "level": "debug"
    },
    "id": 1
  }'
```

## Available Tools

### 1. Generate Logs (`generate_logs`)

Generate sample log entries for testing log level filtering and management.

**Parameters:**
- `count` (integer, optional): Number of log entries to generate (1-50, default: 5)
- `logger` (string, optional): Logger name for the entries (default: "test-logger")

**Example:**
```bash
curl -X POST http://127.0.0.1:8043/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "generate_logs",
      "arguments": {
        "count": 10,
        "logger": "my-service"
      }
    },
    "id": 1
  }'
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": [
    {
      "type": "text",
      "text": "Generated 10 log entries for logger 'my-service':\n[DEBUG] my-service: Application startup completed\n[INFO] my-service: Processing user request\n[NOTICE] my-service: Database connection established\n..."
    }
  ],
  "id": 1
}
```

### 2. View Logs (`view_logs`)

View recent log entries with optional filtering by level and logger.

**Parameters:**
- `count` (integer, optional): Number of recent entries to show (1-100, default: 20)
- `level` (string, optional): Minimum log level to show (debug|info|notice|warning|error|critical|alert|emergency)
- `logger` (string, optional): Filter by specific logger name

**Example:**
```bash
curl -X POST http://127.0.0.1:8043/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "view_logs",
      "arguments": {
        "count": 5,
        "level": "warning",
        "logger": "my-service"
      }
    },
    "id": 1
  }'
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": [
    {
      "type": "text",
      "text": "Recent Log Entries (showing 3):\n==================================================\n[2024-08-17T10:30:45Z] [ERROR] my-service: Failed to connect to external API\n  Thread: worker-2\n  Module: example_module\n\n[2024-08-17T10:30:44Z] [WARNING] my-service: Rate limit exceeded for IP: 192.168.1.100\n  Thread: worker-1\n  Module: example_module\n\n[2024-08-17T10:30:43Z] [CRITICAL] my-service: Memory usage threshold reached\n  Thread: worker-3\n  Module: example_module"
    }
  ],
  "id": 1
}
```

### 3. Log Configuration (`log_config`)

Get current logging configuration including global and per-logger levels.

**Parameters:** None

**Example:**
```bash
curl -X POST http://127.0.0.1:8043/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "log_config"
    },
    "id": 1
  }'
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": [
    {
      "type": "text",
      "text": "Current Logging Configuration:\n{\n  \"global_level\": \"info\",\n  \"logger_levels\": {\n    \"my-service\": \"debug\",\n    \"database\": \"warning\"\n  },\n  \"total_log_entries\": 45,\n  \"max_history\": 1000,\n  \"level_priorities\": {\n    \"debug\": 0,\n    \"info\": 1,\n    \"notice\": 2,\n    \"warning\": 3,\n    \"error\": 4,\n    \"critical\": 5,\n    \"alert\": 6,\n    \"emergency\": 7\n  }\n}"
    }
  ],
  "id": 1
}
```

## MCP Logging Protocol

### Set Log Level (`logging/setLevel`)

Sets the global log level for the server.

**Parameters:**
- `level` (required): Log level to set (debug|info|notice|warning|error|critical|alert|emergency)

**Example:**
```bash
curl -X POST http://127.0.0.1:8043/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "logging/setLevel",
    "params": {
      "level": "debug"
    },
    "id": 1
  }'
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": null,
  "id": 1
}
```

## Log Levels

The server supports 8 standard log levels with increasing severity:

| Level     | Priority | Usage                                    |
|-----------|----------|------------------------------------------|
| debug     | 0        | Detailed debugging information           |
| info      | 1        | General informational messages           |
| notice    | 2        | Normal but significant conditions        |
| warning   | 3        | Warning conditions                       |
| error     | 4        | Error conditions                         |
| critical  | 5        | Critical conditions                      |
| alert     | 6        | Action must be taken immediately         |
| emergency | 7        | System is unusable                       |

### Level Filtering

When a log level is set, only messages at that level or higher will be processed. For example:
- Setting level to "warning" will show: warning, error, critical, alert, emergency
- Setting level to "debug" will show all levels

## Implementation Details

### Centralized Logging State

The server maintains a centralized logging state that tracks:

```rust
pub struct LoggingState {
    current_level: LogLevel,           // Global log level
    loggers: HashMap<String, LogLevel>, // Per-logger levels
    log_history: Vec<LogEntry>,        // Recent log entries
    max_history: usize,                // Maximum entries to keep
}
```

### Enhanced Logging Handler

The custom logging handler processes MCP logging requests:

```rust
#[async_trait]
impl McpHandler for EnhancedLoggingHandler {
    async fn handle(&self, params: Option<Value>) -> McpResult<Value> {
        if let Some(params) = params {
            let request: SetLevelRequest = serde_json::from_value(params)?;
            
            // Update global level
            let mut state = self.state.lock().unwrap();
            state.set_global_level(request.level.clone());
            
            // Log the change
            let log_entry = LogEntry::new(
                LogLevel::Info,
                format!("Log level changed to {:?}", request.level)
            ).with_logger("mcp-server".to_string());
            
            state.add_log_entry(log_entry);
            
            Ok(Value::Null)
        } else {
            Err("Missing parameters for logging/setLevel".to_string())
        }
    }
}
```

### Log Entry Structure

Each log entry includes rich metadata:

```json
{
  "level": "info",
  "message": "Application startup completed",
  "logger": "my-service",
  "data": {
    "timestamp": "2024-08-17T10:30:45Z",
    "sequence": 1,
    "thread": "worker-1",
    "module": "example_module",
    "generated": true
  }
}
```

### Thread-Safe State Management

The logging state uses `Arc<Mutex<LoggingState>>` for thread-safe access:

```rust
let logging_state = Arc::new(Mutex::new(LoggingState::new()));

// Safe access from multiple tools
let state = self.state.lock().unwrap();
let recent_logs = state.get_recent_logs(20);
```

## Protocol Compliance

This example follows the MCP 2025-06-18 specification for logging:

- **logging/setLevel**: Accepts `SetLevelRequest` with proper log level validation
- **Error Handling**: Returns appropriate errors for invalid requests
- **Response Format**: Returns `null` for successful setLevel operations
- **Thread Safety**: Concurrent access to logging state is properly synchronized

## Advanced Features

### Per-Logger Configuration

Set different log levels for different components:

```rust
// Set database logger to warning level
state.set_logger_level("database".to_string(), LogLevel::Warning);

// Set API logger to debug level  
state.set_logger_level("api".to_string(), LogLevel::Debug);
```

### Log History Management

Automatically manages log history with configurable limits:

```rust
pub fn add_log_entry(&mut self, entry: LogEntry) {
    self.log_history.push(entry);
    if self.log_history.len() > self.max_history {
        self.log_history.remove(0); // Remove oldest entry
    }
}
```

### Advanced Filtering

Filter logs by multiple criteria:

```rust
pub fn filter_logs(&self, level: Option<&LogLevel>, logger: Option<&str>) -> Vec<&LogEntry> {
    self.log_history
        .iter()
        .filter(|entry| {
            // Filter by minimum level
            if let Some(filter_level) = level {
                if !self.should_log(&entry.level, filter_level) {
                    return false;
                }
            }
            // Filter by logger name
            if let Some(filter_logger) = logger {
                if let Some(entry_logger) = &entry.logger {
                    if entry_logger != filter_logger {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            true
        })
        .collect()
}
```

## Use Cases

### Development and Debugging
- **Real-time Debugging**: Change log levels without restarting services
- **Component Isolation**: Set different log levels for different modules
- **Issue Investigation**: Filter logs by severity and component

### Operations and Monitoring
- **Centralized Log Management**: Collect logs from multiple MCP servers
- **Alert Integration**: Monitor for critical/emergency level logs
- **Performance Tuning**: Adjust log verbosity based on system load

### Distributed Systems
- **Service Coordination**: Synchronize log levels across microservices
- **Debugging Distributed Issues**: Trace issues across service boundaries
- **Compliance**: Maintain audit trails with proper log levels

## Extension Opportunities

This logging server can be extended to include:

- **Log Forwarding**: Send logs to external systems (Elasticsearch, Splunk)
- **Real-time Streaming**: WebSocket or SSE for live log streaming
- **Log Parsing**: Structure extraction from unstructured log messages
- **Alerting**: Trigger alerts based on log patterns and thresholds
- **Authentication**: Per-user or per-client log level configuration
- **Persistence**: Store logs to disk or database
- **Metrics**: Generate metrics from log patterns and frequencies
- **Log Rotation**: Automatic log file rotation and archival
- **Search**: Full-text search across log history
- **Dashboard**: Web UI for log viewing and management