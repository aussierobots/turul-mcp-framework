# Lambda MCP Test Client

Comprehensive test client for validating `lambda-mcp-server` implementation, ensuring MCP 2025-06-18 Streamable HTTP protocol compliance, tool functionality, session management, and infrastructure integration.

## Features

### 🧪 **Comprehensive Test Suite**
- **Protocol Compliance**: Validates MCP 2025-06-18 specification adherence
- **Tool Functionality**: Tests all available tools with schema validation
- **Session Management**: Validates session lifecycle and state persistence  
- **Infrastructure Integration**: Tests AWS Lambda, DynamoDB, SQS integration
- **Performance Testing**: Load testing and performance benchmarks
- **Error Handling**: Comprehensive error condition testing

### 🔍 **Advanced Validation**
- **Schema Validation**: Validates tool responses against declared schemas
- **JSON-RPC 2.0 Compliance**: Ensures proper message formatting
- **Concurrent Session Testing**: Tests session isolation and concurrency
- **Real-time Monitoring**: Health monitoring and alerting capabilities

### 📊 **Rich Reporting**
- **Detailed Test Results**: Comprehensive pass/fail reporting with timing
- **Schema Validation Errors**: Detailed error reporting for schema mismatches  
- **Performance Metrics**: Response times, throughput, and resource usage
- **Interactive Mode**: Live server interaction and debugging

## Quick Start

### **Prerequisites**
- **Rust 1.70+** with 2024 edition support
- **Running lambda-mcp-server** (local or deployed)

### **Installation**
```bash
cd examples/lambda-mcp-client
cargo build --release
```

### **Basic Usage**

#### **Run Full Test Suite**
```bash
# Test against local server
cargo run -- test --url http://127.0.0.1:9000

# Test with detailed reporting
cargo run -- test --url http://127.0.0.1:9000 --detailed-report

# Run specific test suite
cargo run -- test --suite protocol --url http://127.0.0.1:9000
```

#### **Interactive Session**
```bash
# Connect interactively
cargo run -- connect --url http://127.0.0.1:9000

# With debug output
cargo run -- connect --url http://127.0.0.1:9000 --debug
```

#### **Schema Validation**
```bash
# Validate all tool schemas
cargo run -- validate-schemas --url http://127.0.0.1:9000

# Validate specific tool
cargo run -- validate-schemas --tool lambda_diagnostics --url http://127.0.0.1:9000
```

## Test Suites

### **Full Test Suite** (`--suite all`)
Comprehensive testing covering all aspects:

#### **Protocol Compliance Tests**
- ✅ MCP session initialization with capability negotiation
- ✅ JSON-RPC 2.0 message format validation
- ✅ Tools/resources/prompts list method testing
- ✅ Server capability validation
- ✅ Session header handling (`Mcp-Session-Id`)

#### **Tool Functionality Tests**
- ✅ `lambda_diagnostics` - Runtime metrics and Lambda context
- ✅ `session_info` - Session management and statistics
- ✅ `aws_real_time_monitor` - AWS resource monitoring
- ✅ `list_active_sessions` - Multi-session management
- ✅ `publish_test_event` - SQS event publishing
- ✅ Schema validation for all tool responses

#### **Session Management Tests**
- ✅ Complete session lifecycle (init → active → cleanup)
- ✅ Concurrent session isolation and state management
- ✅ Session timeout and cleanup behavior
- ✅ DynamoDB session persistence validation

#### **Infrastructure Integration Tests**
- ✅ DynamoDB session storage and retrieval
- ✅ SQS event processing and message handling
- ✅ Lambda execution context validation
- ✅ CloudWatch logging and metrics integration

#### **Performance & Reliability Tests**
- ✅ Response time benchmarks
- ✅ Concurrent request handling
- ✅ Memory usage monitoring
- ✅ Load testing under sustained traffic

#### **Error Handling Tests**
- ✅ Invalid method handling
- ✅ Malformed request processing
- ✅ Tool error conditions
- ✅ Network error recovery

### **Targeted Test Suites**

#### **Protocol Only** (`--suite protocol`)
Focus on MCP specification compliance:
```bash
cargo run -- test --suite protocol --url http://127.0.0.1:9000
```

#### **Tools Only** (`--suite tools`)  
Comprehensive tool testing and validation:
```bash
cargo run -- test --suite tools --url http://127.0.0.1:9000
```

#### **Session Management** (`--suite session`)
Session lifecycle and state management:
```bash
cargo run -- test --suite session --url http://127.0.0.1:9000
```

#### **Infrastructure** (`--suite infrastructure`)
AWS integration and infrastructure components:
```bash
cargo run -- test --suite infrastructure --url http://127.0.0.1:9000
```

## Interactive Mode

The interactive mode provides live server interaction for debugging and exploration:

```bash
$ cargo run -- connect --url http://127.0.0.1:9000 --debug

🔗 Interactive MCP Session
Connecting to: http://127.0.0.1:9000
Session ID: interactive-550e8400-e29b-41d4-a716-446655440000

Initializing session... ✅ Success
Listing tools... ✅ Success

Available Tools:
  lambda_diagnostics - Get Lambda execution metrics and diagnostics
  session_info - Get detailed session information
  aws_real_time_monitor - Monitor AWS resources in real-time
  list_active_sessions - List all active MCP sessions
  publish_test_event - Publish test events to SQS

Type 'help' for commands, 'quit' to exit

> help
Commands:
  help - Show this help
  tools - List available tools
  call <tool_name> [args] - Call a tool
  session - Show session info
  quit - Exit interactive session

> call lambda_diagnostics {"include_metrics": true}
{
  "content": [...],
  "isError": false
}

> session
{
  "session_id": "interactive-550e8400-e29b-41d4-a716-446655440000",
  "status": "active",
  "tool_calls": 2,
  ...
}

> quit
👋 Session ended
```

## Performance Testing

### **Basic Benchmark**
```bash
cargo run -- benchmark --url http://127.0.0.1:9000 --requests 100 --concurrency 10
```

### **Load Testing** 
```bash
cargo run -- benchmark \
  --url http://127.0.0.1:9000 \
  --requests 1000 \
  --concurrency 50 \
  --rate-limit 100
```

## Health Monitoring

Continuous health monitoring with configurable alerts:

```bash
# Basic monitoring
cargo run -- monitor --url http://127.0.0.1:9000 --interval 30

# With alert configuration
cargo run -- monitor \
  --url http://127.0.0.1:9000 \
  --interval 10 \
  --alert-config alerts.yaml
```

## Configuration

### **Test Client Configuration**
```toml
# config.toml
[client]
timeout_seconds = 30
user_agent = "lambda-mcp-client/0.1.0"
max_retries = 3

[validation]
strict_mode = true
schema_validation = true

[performance]
default_concurrency = 10
benchmark_duration = 60
```

### **Alert Configuration Example**
```yaml
# alerts.yaml
thresholds:
  response_time_ms: 1000
  error_rate_percent: 5
  success_rate_percent: 95

notifications:
  - type: console
    level: warning
  - type: file
    path: alerts.log
    level: error
```

## Expected Output

### **Successful Test Run**
```
🧪 Lambda MCP Server Test Suite
Server URL: http://127.0.0.1:9000
Test Suite: all
Concurrency: 1

✨ [00:00:15] [████████████████████████████████████████] 25/25 ✅ All Tools Execution

📊 Test Results
============================================================
Total tests: 25
Passed: 25
Failed: 0
Duration: 15.23s

✅ Passed Tests:
  Protocol Initialize (0.45s)
  Protocol Tools List (0.23s)
  Lambda Diagnostics Tool (1.34s)
  Session Info Tool (0.67s)
  All Tools Execution (2.45s)
  Session Lifecycle (1.12s)
  Concurrent Sessions (2.34s)
  DynamoDB Session Storage (0.89s)
  SQS Event Processing (3.21s)
  Basic Performance (1.87s)
  ...

============================================================
🎉 All tests passed!
```

### **Test Failures**
```
❌ Failed Tests:
  Lambda Diagnostics Tool - Schema validation failed: execution_time_ms expected number, got object
    Details: {
      "validation_errors": [
        "execution_time_ms: expected number, found object at path $.result.content[0].runtime_metrics.execution_time_ms"
      ]
    }
```

## Integration with CI/CD

### **GitHub Actions Example**
```yaml
name: MCP Server Testing
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Setup Infrastructure
        run: |
          cd examples/lambda-mcp-server
          ./scripts/setup-infrastructure.sh
      - name: Start MCP Server
        run: |
          cargo lambda watch &
          sleep 10
      - name: Run Tests
        run: |
          cd examples/lambda-mcp-client
          cargo run -- test --url http://127.0.0.1:9000 --detailed-report
```

## Development

### **Adding Custom Tests**
```rust
use crate::test_suite::{TestCase, TestCaseBuilder};

let custom_test = TestCaseBuilder::new("Custom Test", "custom_type")
    .description("Test custom functionality")
    .duration_secs(10)
    .parameters(json!({"param": "value"}))
    .priority(1)
    .build();
```

### **Custom Validation**
```rust
use crate::schema_validator::SchemaValidator;

let mut validator = SchemaValidator::new(true);
validator.add_tool_schema("my_tool".to_string(), my_schema)?;
validator.validate_tool_response("my_tool", &response)?;
```

## Troubleshooting

### **Common Issues**

#### **Connection Refused**
```bash
Error: Connection refused
```
- **Solution**: Ensure `lambda-mcp-server` is running on the specified URL
- **Check**: `cargo lambda watch` in the server directory

#### **Schema Validation Failures**
```bash
Error: Tool response validation failed for lambda_diagnostics
```
- **Solution**: Tool implementation doesn't match declared schema
- **Fix**: Update tool implementation or schema definition

#### **Timeout Errors**
```bash
Error: Request timeout after 30s
```
- **Solution**: Increase timeout or check server performance
- **Command**: `--timeout 60` flag for longer operations

#### **Session Management Issues**
```bash
Error: Session not found
```  
- **Solution**: Check DynamoDB table setup and permissions
- **Verify**: Session table creation in infrastructure setup

## Contributing

1. **Add Tests**: Extend test suites in `src/test_suite.rs`
2. **Improve Validation**: Enhance schema validation in `src/schema_validator.rs`
3. **Performance Testing**: Add benchmarks in test runner
4. **Documentation**: Update examples and troubleshooting guides

---

## Summary

This comprehensive test client ensures `lambda-mcp-server` meets all MCP specification requirements while validating tool functionality, session management, and infrastructure integration. Use it for:

- ✅ **Development Testing**: Validate changes during development
- ✅ **CI/CD Integration**: Automated testing in deployment pipelines  
- ✅ **Performance Monitoring**: Continuous performance benchmarking
- ✅ **Debugging**: Interactive server exploration and troubleshooting

**Built with ❤️ for comprehensive MCP server validation!**