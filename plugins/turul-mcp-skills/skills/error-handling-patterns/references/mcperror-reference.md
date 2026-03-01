# McpError Reference

Complete reference for all 22 `McpError` variants, constructors, error codes, and `From` implementations.

## Import

```rust
use turul_mcp_protocol::{McpError, McpResult};
// Or via the server prelude:
use turul_mcp_server::prelude::*;  // includes McpError, McpResult
```

## Type Alias

```rust
pub type McpResult<T> = Result<T, McpError>;
```

## Parameter Errors (code: -32602)

### MissingParameter

```rust
McpError::MissingParameter(String)
McpError::missing_param("query")
```
Wire: `"Missing required parameter: query"`

### InvalidParameterType

```rust
McpError::InvalidParameterType { param: String, expected: String, actual: String }
McpError::invalid_param_type("count", "integer", "string")
```
Wire: `"Invalid parameter type for 'count': expected integer, got string"`

### ParameterOutOfRange

```rust
McpError::ParameterOutOfRange { param: String, value: String, constraint: String }
McpError::param_out_of_range("limit", "150", "must be 1-100")
```
Wire: `"Parameter 'limit' value 150 is out of range: must be 1-100"`

### InvalidParameters

```rust
McpError::InvalidParameters(String)
```
Wire: `"Invalid parameters: {msg}"`
No convenience constructor — use `McpError::InvalidParameters("msg".into())` directly. For specific parameter issues, prefer `missing_param`, `invalid_param_type`, or `param_out_of_range`.

### InvalidRequest

```rust
McpError::InvalidRequest { message: String }
```
Wire: `"Invalid request: {message}"`
Rarely used in handlers — this is for malformed JSON-RPC requests.

## Not Found Errors

### ToolNotFound (code: -32001)

```rust
McpError::ToolNotFound(String)
```
Wire: `"Tool not found: {name}"`
Used by the framework dispatcher. Rarely needed in handler code.

### ResourceNotFound (code: -32002)

```rust
McpError::ResourceNotFound(String)
```
Wire: `"Resource not found: {uri}"`

### PromptNotFound (code: -32003)

```rust
McpError::PromptNotFound(String)
```
Wire: `"Prompt not found: {name}"`

## Execution Errors

### ToolExecutionError (code: -32010)

```rust
McpError::ToolExecutionError(String)
McpError::tool_execution("Database query failed")
```
Wire: `"Tool execution failed: Database query failed"`
**Also produced by `From<String>` and `From<&str>` conversions.**

### ResourceAccessDenied (code: -32011)

```rust
McpError::ResourceAccessDenied(String)
```
Wire: `"Resource access denied: {uri}"`

### ResourceExecutionError (code: -32012)

```rust
McpError::ResourceExecutionError(String)
McpError::resource_execution("File not readable")
```
Wire: `"Resource execution failed: File not readable"`

### PromptExecutionError (code: -32013)

```rust
McpError::PromptExecutionError(String)
McpError::prompt_execution("Template rendering failed")
```
Wire: `"Prompt execution failed: Template rendering failed"`

## Validation Errors

### ValidationError (code: -32020)

```rust
McpError::ValidationError(String)
McpError::validation("Email format invalid")
```
Wire: `"Validation error: Email format invalid"`

### InvalidCapability (code: -32021)

```rust
McpError::InvalidCapability(String)
```
Wire: `"Invalid capability: {cap}"`

### VersionMismatch (code: -32022)

```rust
McpError::VersionMismatch { expected: String, actual: String }
```
Wire: `"Protocol version mismatch: expected {expected}, got {actual}"`

## Configuration/Session Errors

### ConfigurationError (code: -32030)

```rust
McpError::ConfigurationError(String)
McpError::configuration("Missing DATABASE_URL environment variable")
```
Wire: `"Configuration error: Missing DATABASE_URL environment variable"`

### SessionError (code: -32031)

```rust
McpError::SessionError(String)
```
Wire: `"Session error: {msg}"`

## Transport/Protocol Errors

### TransportError (code: -32040)

```rust
McpError::TransportError(String)
McpError::transport("Connection reset")
```
Wire: `"Transport error: Connection reset"`

### JsonRpcProtocolError (code: -32041)

```rust
McpError::JsonRpcProtocolError(String)
McpError::json_rpc_protocol("Invalid JSON-RPC version")
```
Wire: `"JSON-RPC protocol error: Invalid JSON-RPC version"`

## Internal Errors (code: -32603)

### IoError

```rust
McpError::IoError(std::io::Error)
```
Wire: `"IO error: {err}"`
**Has `From<std::io::Error>` impl — works with `?` operator.**

### SerializationError

```rust
McpError::SerializationError(serde_json::Error)
```
Wire: `"Serialization error: {err}"`
**Has `From<serde_json::Error>` impl — works with `?` operator.**

## Pass-Through Error

### JsonRpcError (code: custom)

```rust
McpError::JsonRpcError { code: i64, message: String, data: Option<Value> }
McpError::json_rpc_error(-32099, "Custom protocol error", None)
```
Wire: Preserves original code/message/data verbatim.
Used by `tasks/result` to reproduce the original error as required by the MCP spec.

## From Implementations

| Source Type | Target McpError | Auto `?` |
|---|---|---|
| `String` | `ToolExecutionError` | Yes |
| `&str` | `ToolExecutionError` | Yes |
| `std::io::Error` | `IoError` | Yes |
| `serde_json::Error` | `SerializationError` | Yes |

**Warning:** `From<String>` and `From<&str>` always produce `ToolExecutionError`. In resource or prompt handlers, use `resource_execution()` or `prompt_execution()` explicitly.

## Constructor Summary

| Constructor | Variant |
|---|---|
| `missing_param(param)` | `MissingParameter` |
| `invalid_param_type(param, expected, actual)` | `InvalidParameterType` |
| `param_out_of_range(param, value, constraint)` | `ParameterOutOfRange` |
| `tool_execution(msg)` | `ToolExecutionError` |
| `resource_execution(msg)` | `ResourceExecutionError` |
| `prompt_execution(msg)` | `PromptExecutionError` |
| `validation(msg)` | `ValidationError` |
| `configuration(msg)` | `ConfigurationError` |
| `transport(msg)` | `TransportError` |
| `json_rpc_protocol(msg)` | `JsonRpcProtocolError` |
| `json_rpc_error(code, msg, data)` | `JsonRpcError` |
