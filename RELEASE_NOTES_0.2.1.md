# Release Notes - Version 0.2.1

**Release Date**: October 3, 2025
**Status**: 🔧 **Beta Release** - Critical bug fixes for MCP Inspector compatibility and Lambda notifications

## 🎯 Release Focus

This patch release addresses two critical issues discovered in production use:
1. **MCP Inspector Compatibility**: SSE event type incompatibility preventing notifications from appearing
2. **Lambda DynamoDB Notifications**: Race condition causing notifications to be missed on first invocation

## 🐛 Critical Fixes

### **MCP Inspector SSE Event Type Compatibility**
- **Issue**: MCP Inspector only processes SSE events with `event: message` (standard SSE spec)
- **Root Cause**: POST Streamable HTTP was using custom event types like `event: notifications/progress`
- **Impact**: Only 3 of 5 notifications appeared in MCP Inspector's notification panel
- **Fix**: All SSE events now use `event: message` except keepalives (which use SSE comment syntax)
- **Result**: All notifications now visible in MCP Inspector ✅

**Files Changed**:
- `crates/turul-http-mcp-server/src/stream_manager.rs` - POST SSE response formatting
- `crates/turul-http-mcp-server/src/sse.rs` - Legacy SSE module alignment
- `crates/turul-mcp-session-storage/src/traits.rs` - SseEvent::format() method

### **Lambda DynamoDB Notification Timing**
- **Issue**: Notifications worked on reconnect but not on initial Lambda invocation
- **Root Cause**: DynamoDB eventual consistency - writes completed but reads didn't see data yet
- **Impact**: Cold Lambda starts with DynamoDB would miss tool progress notifications
- **Fix**: Added `.consistent_read(true)` to DynamoDB query operations
- **Removed**: Unnecessary 50ms sleep that was a bandaid, not a proper fix
- **Result**: Notifications now work reliably on first Lambda invocation ✅

**Files Changed**:
- `crates/turul-mcp-session-storage/src/dynamodb.rs:1716` - get_recent_events()
- `crates/turul-mcp-session-storage/src/dynamodb.rs:1628` - get_events_after()
- `crates/turul-http-mcp-server/src/stream_manager.rs:715` - Removed 50ms sleep

**Trade-offs**:
- Consistent reads cost 2x DynamoDB read capacity units vs eventually consistent
- Critical for correct MCP behavior and worth the cost

### **Test Suite Alignment**
Updated all SSE-related tests to match new event format:
- `turul-mcp-session-storage/src/traits.rs` - Added keepalive test case
- `turul-http-mcp-server/src/sse.rs` - Updated legacy SSE tests
- `turul-http-mcp-server/src/tests/sse_tests.rs` - Fixed assertions
- `turul-http-mcp-server/src/tests/simple_tests.rs` - Separated keepalive tests

**Result**: All 440+ tests passing ✅

## 🔍 Technical Details

### SSE Event Type Standardization

**Before (0.2.0)**:
```
event: notifications/progress    ← Custom event type
data: {"jsonrpc":"2.0","method":"notifications/progress",...}

event: notifications/message     ← Custom event type  
data: {"jsonrpc":"2.0","method":"notifications/message",...}
```

**After (0.2.1)**:
```
event: message                   ← Standard SSE event type
data: {"jsonrpc":"2.0","method":"notifications/progress",...}

event: message                   ← Standard SSE event type
data: {"jsonrpc":"2.0","method":"notifications/message",...}
```

**Why This Matters**:
- JavaScript EventSource API only fires `onmessage` for `event: message` or omitted event lines
- Custom event types require explicit `addEventListener('custom-type', ...)` 
- MCP Inspector uses standard `onmessage` handler
- JSON-RPC method is in the data payload, event type is just transport wrapper

### DynamoDB Consistency Model

**The Race Condition**:
1. Tool executes and calls `session.notify_progress()`
2. Notifications stored to DynamoDB via `put_item().send().await` ✅
3. POST SSE response immediately queries `get_recent_events()`
4. Eventually consistent read might not see just-written data ❌
5. Cold Lambda + DynamoDB = higher latency = more likely to hit inconsistency window
6. Reconnect works because events are already replicated by then ✅

**The Fix**:
```rust
// Before
let query_result = self.client.query()
    .table_name(&event_table)
    .key_condition_expression("session_id = :session_id")
    .send().await?;

// After  
let query_result = self.client.query()
    .table_name(&event_table)
    .key_condition_expression("session_id = :session_id")
    .consistent_read(true)  // ← Guarantees read-your-writes
    .send().await?;
```

## 📦 Version Updates

All turul-* crates updated from 0.2.0 → **0.2.1**:
- `turul-mcp-server`
- `turul-mcp-client`
- `turul-http-mcp-server`
- `turul-mcp-protocol`
- `turul-mcp-protocol-2025-06-18`
- `turul-mcp-derive`
- `turul-mcp-builders`
- `turul-mcp-json-rpc-server`
- `turul-mcp-session-storage`
- `turul-mcp-aws-lambda`

## 🧪 Verification

### Test Commands
```bash
# Test SSE formatting
cargo test --package turul-mcp-session-storage test_sse_event_formatting

# Test legacy SSE module
cargo test --package turul-http-mcp-server sse

# Full workspace check
cargo check --workspace
```

### Expected Results
```
test result: ok. 440+ passed; 0 failed
Finished `dev` profile in X.XXs
```

## 🚀 Upgrade Guide

### For Existing Users

**No Breaking Changes** - This is a patch release with bug fixes only.

**Recommended Actions**:
1. Update `Cargo.toml` dependencies to `0.2.1`
2. Run `cargo update` to fetch new versions
3. Test Lambda deployments to verify notification behavior
4. Test with MCP Inspector to verify all notifications appear

**DynamoDB Users**:
- Consistent reads are now enabled automatically
- Expect 2x read capacity unit consumption for event queries
- This is necessary for correct behavior and worth the cost

**No Code Changes Required** - All fixes are internal to the framework.

## 📊 Impact Assessment

### Who Should Upgrade?

**High Priority**:
- ✅ Users testing with MCP Inspector (notification visibility)
- ✅ Lambda deployments using DynamoDB (notification reliability)
- ✅ Production systems expecting consistent notification behavior

**Medium Priority**:
- ✅ Non-Lambda deployments (benefits from standardization)
- ✅ Development environments (better debugging experience)

**Low Priority**:
- Deployments without SSE notifications enabled
- Systems not using progress notifications

## 🔗 Related Issues

### Codex Analysis
Special thanks to Codex for identifying:
- High: `test_sse_event_formatting` assertions (traits.rs:499)
- High: Streamable-HTTP test assumptions (sse_tests.rs:90, 157)
- Recommendation: Update all tests to match new wire format

### User Report
Original issue: "Lambda notifications don't work on first connect but work after reconnect"

**Resolution Timeline**:
1. Identified SSE event type incompatibility with MCP Inspector
2. Discovered DynamoDB eventual consistency race condition
3. Fixed both issues with proper solutions (not workarounds)
4. Updated complete test suite for consistency
5. Verified all functionality works correctly

## 🎉 What's Working Now

✅ **MCP Inspector**: All 5 notifications visible (progress + messages)  
✅ **Lambda Cold Starts**: Notifications work on first invocation  
✅ **DynamoDB**: Consistent reads guarantee notification visibility  
✅ **SSE Standards**: Compliant with standard EventSource API  
✅ **Test Suite**: 440+ tests passing with updated expectations  

## 📈 Performance Notes

### DynamoDB Read Capacity
- **Eventually Consistent**: 1 RCU per 8KB read
- **Strongly Consistent**: 1 RCU per 4KB read  
- **Impact**: 2x RCU consumption for event queries
- **Justification**: Correctness over cost optimization

### Lambda Performance
- No additional cold start overhead
- Removed unnecessary 50ms sleep (faster responses!)
- DynamoDB latency unchanged (consistent reads ~10-20ms)

## 🙏 Acknowledgments

- **Codex**: For thorough analysis identifying test inconsistencies
- **Early Adopters**: For reporting Lambda notification timing issues
- **MCP Community**: For standardized SSE event type requirements

---

**Ready to upgrade?** Simply update to version 0.2.1 in your `Cargo.toml`:

```toml
[dependencies]
turul-mcp-server = "0.2.1"
```

**Need help?** Check the main README or open an issue on GitHub.

## 🔜 Next Release (0.2.2)

Planned improvements:
- Additional storage backend optimizations
- Enhanced Lambda cold start performance
- More comprehensive integration tests
- Documentation updates for new patterns

---

*This is a recommended upgrade for all production deployments using SSE notifications.*
