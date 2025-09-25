# HTTP 400 Investigation Report

**Status**: Active Investigation
**Date**: 2025-09-25
**Issue**: 6 HTTP MCP tests failing with HTTP 400 Bad Request errors

## Background

After implementing MCP 2025-06-18 Streamable HTTP transport and fixing the binary cache issue, streaming functionality now works correctly. However, 6 tests continue to fail with HTTP 400 responses, indicating request parsing or validation issues.

## Failing Tests

Based on previous test runs, the following tests consistently fail with HTTP 400 errors:

1. **Basic Protocol Tests**
   - Request parsing failures
   - Header validation issues
   - Content-Type mismatches

2. **Session Management Tests**
   - Session creation endpoint failures
   - Session ID format validation
   - Session header processing

3. **Tool Call Tests**
   - Tool parameter validation
   - Request body structure issues
   - Method routing problems

## Test Patterns

The failures appear to be concentrated in specific areas:

### Pattern 1: Header Validation
```
Status: 400 Bad Request
Cause: Missing or invalid MCP headers
```

### Pattern 2: Request Body Structure
```
Status: 400 Bad Request
Cause: JSON-RPC request structure validation
```

### Pattern 3: Session Endpoint Issues
```
Status: 400 Bad Request
Cause: Session creation/management endpoints
```

## Investigation Strategy

### Phase 1: Capture Request/Response Pairs ⬅️ CURRENT
Need to run failing tests with detailed logging to capture:
- Complete HTTP request (method, headers, body)
- Complete HTTP response (status, headers, body)
- Server-side error logs with stack traces

### Phase 2: Root Cause Analysis
Once we have actual failure data:
- Categorize failures by type (header, body, routing, validation)
- Identify if issues are in test harness or actual server bugs
- Check against MCP 2025-06-18 specification compliance

### Phase 3: Fix Implementation
Based on root cause analysis:
- Fix legitimate server bugs
- Update test harness for correct request format
- Validate against MCP specification requirements

## Technical Context

### Current HTTP Server Architecture
- **Protocol Routing**: Working correctly (verified)
- **Streaming Implementation**: Working correctly (verified with curl)
- **Session Storage**: Working correctly for basic cases
- **JSON-RPC Dispatcher**: Unknown status - may be source of 400 errors

### Recent Changes
- Implemented StreamableHttpHandler with proper chunked streaming
- Fixed session persistence in storage
- Added correct MCP headers (`MCP-Protocol-Version`, `Mcp-Session-Id`, etc.)
- Fixed binary cache issue causing stale test behavior

### Known Working
- Manual curl tests with streaming requests work perfectly
- Transfer-Encoding: chunked is properly set
- JSON-RPC frames stream correctly (Progress, PartialResult, FinalResult)

### Unknown Status
- **Request Validation**: May be rejecting valid MCP requests
- **JSON-RPC Parsing**: May have strict validation causing 400s
- **Header Processing**: May be case-sensitive or format-strict
- **Content-Type Handling**: May expect specific MIME types

## Next Steps

1. **Immediate**: Run one failing test with maximum debug logging
2. **Capture**: Complete request/response pair with server logs
3. **Analyze**: Determine if test input is valid per MCP specification
4. **Categorize**: Is this a server bug or test harness issue?
5. **Fix**: Implement appropriate solution

## Expected Outcome

After investigation, we expect to find one of:

**A) Server Bug**: HTTP server rejecting valid MCP 2025-06-18 requests
- Fix: Update request validation logic
- Timeline: Quick fix once identified

**B) Test Harness Bug**: Tests sending invalid request format
- Fix: Update test to send proper MCP-compliant requests
- Timeline: Update test cases

**C) Specification Compliance**: Server correctly rejecting invalid requests
- Fix: Update tests to match MCP 2025-06-18 specification
- Timeline: Research specification and update tests

---

**Note**: This investigation is critical for production readiness. HTTP 400 errors indicate the server is rejecting potentially valid MCP client requests, which would break real-world usage.