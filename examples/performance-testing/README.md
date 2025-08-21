# Performance Testing Suite

A comprehensive performance testing suite for the MCP (Model Context Protocol) framework. This suite provides tools for load testing, stress testing, benchmarking, and memory analysis to ensure your MCP servers can handle production workloads.

## Overview

The performance testing suite includes:

- **Load Test Server**: High-performance MCP server optimized for testing
- **Performance Client**: Configurable load testing client with various patterns
- **Stress Test**: Comprehensive stress testing with resource exhaustion scenarios
- **Memory Benchmark**: Memory usage analysis and leak detection
- **Criterion Benchmarks**: Micro-benchmarks for detailed performance analysis

## Quick Start

### 1. Start the Load Test Server

```bash
cargo run --bin load_test_server
```

The server will start on `http://127.0.0.1:8080/mcp` with the following tools:
- `fast_compute`: Minimal overhead computation
- `cpu_intensive`: CPU stress testing
- `memory_allocate`: Memory allocation testing
- `async_io`: Async I/O simulation
- `session_counter`: Session-aware operations
- `perf_stats`: Performance statistics
- `error_simulation`: Error condition testing

### 2. Run Performance Tests

#### Throughput Test
```bash
cargo run --bin performance_client -- --duration-seconds 60 throughput --requests-per-second 1000
```

#### Stress Test
```bash
cargo run --bin performance_client -- --duration-seconds 120 stress --computation-size 500
```

#### Memory Test
```bash
cargo run --bin performance_client -- --duration-seconds 60 memory --mb-per-request 10
```

#### Burst Test
```bash
cargo run --bin performance_client -- --duration-seconds 60 burst --burst-size 100 --burst-interval-seconds 5
```

### 3. Run Comprehensive Stress Tests

#### Memory Exhaustion
```bash
cargo run --bin stress_test memory --max-concurrent 100 --mb-per-request 50
```

#### CPU Exhaustion
```bash
cargo run --bin stress_test cpu --max-concurrent 200 --computation-size 1000
```

#### Connection Flooding
```bash
cargo run --bin stress_test flood --connections-per-second 1000 --max-connections 5000
```

#### Chaos Testing
```bash
cargo run --bin stress_test chaos --max-concurrent 100
```

### 4. Memory Analysis

#### Baseline Memory Usage
```bash
cargo run --bin memory_benchmark baseline --request-count 1000
```

#### Memory Leak Detection
```bash
cargo run --bin memory_benchmark leak-detection --iterations 10000
```

#### Memory Growth Analysis
```bash
cargo run --bin memory_benchmark growth-analysis --measurement-interval-seconds 30
```

### 5. Run Micro-benchmarks

#### Tool Execution Benchmarks
```bash
cargo bench tool_execution
```

#### Session Management Benchmarks
```bash
cargo bench session_management
```

#### Notification Broadcasting Benchmarks
```bash
cargo bench notification_broadcasting
```

## Test Scenarios

### Load Testing Patterns

1. **Throughput Test**: Measures maximum requests per second
2. **Latency Test**: Measures response time under various loads
3. **Concurrent User Test**: Simulates multiple concurrent users
4. **Burst Test**: Tests handling of sudden traffic spikes

### Stress Testing Scenarios

1. **Memory Exhaustion**: Tests server behavior under memory pressure
2. **CPU Exhaustion**: Tests server behavior under CPU pressure
3. **Connection Flooding**: Tests connection handling limits
4. **Error Recovery**: Tests error handling and recovery mechanisms
5. **Chaos Testing**: Mixed stressors for comprehensive testing

### Memory Testing

1. **Baseline Measurement**: Establishes normal memory usage patterns
2. **Leak Detection**: Identifies potential memory leaks
3. **Growth Analysis**: Monitors memory usage over time
4. **Allocation Patterns**: Analyzes memory allocation behavior

## Performance Metrics

### Key Metrics Tracked

- **Requests per Second (RPS)**: Server throughput
- **Response Time**: Average, min, max response times
- **Error Rate**: Percentage of failed requests
- **Memory Usage**: Current, peak, and growth patterns
- **CPU Utilization**: Server CPU usage under load
- **Connection Handling**: Concurrent connection limits

### Success Criteria

- **Throughput**: > 1000 RPS for simple operations
- **Response Time**: < 100ms p99 for fast operations
- **Error Rate**: < 1% under normal load
- **Memory Stability**: < 10% growth over extended periods
- **Recovery Time**: < 30 seconds after stress events

## Configuration Options

### Load Test Server Configuration

The server can be configured via environment variables or command-line arguments:

```bash
# Custom bind address
cargo run --bin load_test_server -- --bind-address 0.0.0.0:9000

# Enable debug logging
RUST_LOG=debug cargo run --bin load_test_server
```

### Performance Client Configuration

```bash
# Custom server URL
cargo run --bin performance_client --server-url http://localhost:9000/mcp throughput

# Increased concurrency
cargo run --bin performance_client --concurrency 50 throughput

# Extended test duration
cargo run --bin performance_client -- --duration-seconds 300 throughput
```

## Interpreting Results

### Throughput Test Results

```
=== Performance Test Results ===
Requests sent: 60000
Requests completed: 59985 (99.9%)
Requests failed: 15 (0.1%)
Average response time: 45 ms
Min response time: 12 ms
Max response time: 234 ms
Requests per second: 999.75
```

**Analysis**:
- ✅ **Excellent**: > 95% completion rate, < 100ms average response time
- ⚠️ **Good**: > 90% completion rate, < 200ms average response time
- ❌ **Poor**: < 90% completion rate, > 500ms average response time

### Memory Test Results

```
=== Memory Snapshot: Final ===
Current usage: 45.67 MB
Peak usage: 52.34 MB
Total allocated: 1.23 GB
Allocation count: 1,234,567
RSS (system): 78.90 MB

=== Baseline Analysis ===
Memory per request: 1,024 bytes
Memory efficiency: 1024.00 requests per MB
```

**Analysis**:
- ✅ **Excellent**: < 1KB per request, stable memory usage
- ⚠️ **Good**: < 10KB per request, minimal growth
- ❌ **Poor**: > 100KB per request, continuous growth

### Stress Test Results

```
=== Comprehensive Stress Test Results ===
Request Statistics:
  Requests sent: 50000
  Requests completed: 48750 (97.5%)
  Requests successful: 47125 (94.3%)
  Requests failed: 1625 (3.3%)

Overall Reliability Score: 94.25%
✅ GOOD - Server handled stress adequately
```

**Reliability Scores**:
- ✅ **Excellent**: ≥ 99% reliability
- ✅ **Good**: ≥ 95% reliability
- ⚠️ **Moderate**: ≥ 90% reliability
- ❌ **Poor**: < 90% reliability

## Benchmarking Best Practices

### 1. Environment Setup

- Use dedicated test machines
- Minimize background processes
- Use consistent network conditions
- Run tests multiple times for consistency

### 2. Test Design

- Start with baseline measurements
- Gradually increase load to find limits
- Test realistic usage patterns
- Include error scenarios

### 3. Monitoring

- Monitor both client and server metrics
- Track system resources (CPU, memory, network)
- Use profiling tools for detailed analysis
- Set up alerts for threshold violations

### 4. Analysis

- Compare results across different configurations
- Look for performance regressions
- Identify bottlenecks and optimization opportunities
- Document findings and recommendations

## Integration with CI/CD

### GitHub Actions Example

```yaml
name: Performance Tests

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

jobs:
  performance:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          
      - name: Start Test Server
        run: |
          cargo run --bin load_test_server &
          sleep 10
          
      - name: Run Performance Tests
        run: |
          cargo run --bin performance_client -- --duration-seconds 30 throughput
          cargo run --bin memory_benchmark baseline --request-count 100
          
      - name: Run Benchmarks
        run: |
          cargo bench -- --output-format json > benchmark_results.json
          
      - name: Upload Results
        uses: actions/upload-artifact@v3
        with:
          name: performance-results
          path: benchmark_results.json
```

## Troubleshooting

### Common Issues

1. **Connection Refused**: Ensure the load test server is running
2. **High Error Rates**: Check server logs for error details
3. **Low Throughput**: Verify network configuration and server capacity
4. **Memory Leaks**: Use memory profiling tools for detailed analysis

### Performance Tuning

1. **Server Optimization**:
   - Adjust thread pool sizes
   - Tune garbage collection settings
   - Optimize database connections
   - Enable connection pooling

2. **Client Optimization**:
   - Increase concurrency limits
   - Use connection keep-alive
   - Implement request batching
   - Add retry mechanisms

### Debugging Tools

- **Memory Profiling**: `cargo run --bin memory_benchmark`
- **CPU Profiling**: Use `perf` or `flamegraph`
- **Network Analysis**: Use `tcpdump` or `wireshark`
- **System Monitoring**: Use `htop`, `iostat`, `netstat`

## Contributing

When adding new performance tests:

1. Add comprehensive documentation
2. Include baseline expectations
3. Implement proper error handling
4. Add configuration options
5. Include analysis and interpretation guidance

## License

This performance testing suite is part of the MCP framework and follows the same licensing terms.