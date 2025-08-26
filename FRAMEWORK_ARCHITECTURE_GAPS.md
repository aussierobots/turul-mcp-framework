# 🏗️ MCP Framework - Architecture Gaps & Implementation Plan

**Status**: Core framework AND testing architecture **FUNDAMENTALLY BROKEN**
**Priority**: **CRITICAL** - Testing masked architectural failures, SSE streaming broken, notifications violate MCP spec

## 🚨 **NEW CRITICAL DISCOVERY: NOTIFICATION FORMAT VIOLATION**

**Status**: ❌ **MCP SPEC VIOLATION** - Sending custom JSON instead of proper JSON-RPC notifications
**Impact**: SSE events contain custom JSON like `{"type":"progress",...}` instead of MCP-compliant `{"jsonrpc":"2.0","method":"notifications/progress","params":{...}}`
**Root Cause**: NotificationBroadcaster trait uses custom JSON format, not MCP JSON-RPC notification format
**Discovery**: Review of MCP TypeScript schema revealed notifications MUST be full JSON-RPC format

### **MCP TypeScript Schema Requirements**
```typescript
// ALL notifications MUST follow this structure:
export interface JSONRPCNotification extends Notification {
  jsonrpc: "2.0";
  method: string;
  params?: { ... };
}

// Example: Progress notification
export interface ProgressNotification extends Notification {
  method: "notifications/progress";
  params: {
    progressToken: ProgressToken;
    progress: number;
    total?: number;
    message?: string;
  };
}
```

### **Current Violation vs Required Format**
```json
// ❌ WRONG (Current implementation):
data: {"type":"progress","progressToken":"token","progress":50}

// ✅ CORRECT (MCP specification):
data: {"jsonrpc":"2.0","method":"notifications/progress","params":{"progressToken":"token","progress":50}}
```

## 🚨 **CRITICAL DISCOVERY: FUNDAMENTAL TESTING ARCHITECTURE FAILURE**

**Status**: ❌ **CRITICAL TESTING FLAW** - False confidence from component-only testing  
**Impact**: Complete notification flow failure masked by successful individual component tests  
**Root Cause**: Testing components instead of end-to-end integration flows  
**Discovery**: User question "I can't see server events from the client" revealed the testing never verified actual notification delivery

## 🧠 **ULTRATHINK ANALYSIS: SESSION ARCHITECTURE COMPARISON**

**Status**: ❌ **ARCHITECTURAL MISMATCH** - Wrong patterns implemented vs GPS project reference  
**Root Cause**: Session lifecycle not following MCP protocol - clients generating IDs instead of servers  
**Discovery**: GPS project shows proper server-provided session creation during initialize

### **GPS Project (Reference Implementation) ✅**

**Proper Session Lifecycle:**
1. **Initialize Request**: Client sends `initialize` (no session ID needed)
2. **Server Creates Session**: `session::new_session(version)` generates UUID  
3. **Session ID Returned**: `HandleResult::RpcResponseWithSession` carries session_id
4. **HTTP Header Set**: Response includes `Mcp-Session-Id: <uuid>` header
5. **Client Uses Session**: All subsequent requests include `Mcp-Session-Id` header
6. **Real SSE**: Sessions have broadcast channels for actual notification streaming

**Key Architecture Components:**
```rust
// During initialize processing:
let session_handle = session::new_session(chosen_version).await;
let hr = make_json_rpc_response_with_session(
    id.clone(),
    init_payload, 
    session_handle.session_id.clone(),
);
```

### **Streamable HTTP Compliance (Current) ❌**

**Broken Session Lifecycle:**
1. **Initialize Request**: Client sends `initialize` 
2. **No Session Creation**: Server doesn't create sessions ❌
3. **No Session ID Returned**: No session ID in response ❌  
4. **Client Violates Protocol**: Client generates session ID ❌
5. **Tools Unused**: SessionContext received but not used ❌
6. **Fake SSE**: Static responses instead of real streaming ❌

### **MCP 2025-06-18 Specification Compliance**

The MCP spec is transport-agnostic but for HTTP:
- **Session Management**: Transport-specific (HTTP uses `Mcp-Session-Id` headers)
- **Initialize Flow**: Establishes connection but doesn't specify session creation
- **Server Responsibility**: Sessions are server-managed resources

### **🚨 Root Cause: I Was Solving the Wrong Problem**

The real issues are:

1. **Missing Session Creation**: Server doesn't create sessions during initialize
2. **Missing Response Headers**: No `Mcp-Session-Id` header in HTTP responses
3. **Disconnected Architecture**: SSE endpoints return static responses, not real streams
4. **Tools Don't Notify**: SessionContext received but tools don't send notifications

The `session_context` parameter usage is fine - the problem is upstream in session creation and downstream in notification sending.

### **🧪 TESTING ARCHITECTURE ANTI-PATTERN DISCOVERED**

**What Tests Were Actually Checking** ❌:
```
✅ HTTP POST requests work (tools execute)
✅ HTTP GET SSE connections establish  
✅ Static SSE responses return
❌ NO verification that notifications flow from tools to client
❌ NO end-to-end integration testing
❌ NO real streaming validation
```

**What Tests SHOULD Have Been Checking** ✅:
```
1. Start SSE connection (GET /mcp)
2. Call tool that sends notifications (POST /mcp)  
3. WAIT and verify SSE stream receives tool notifications
4. Verify notification content/timing/session routing
5. Test notification fan-out to multiple sessions
6. Test SSE resumability with Last-Event-ID
```

### **The False Confidence Problem**
- Tests passed ✅ → Framework appears working
- Components work individually ✅ → Integration assumed working  
- **Reality**: Components completely disconnected ❌
- **Result**: Notifications sent to void, SSE returns static responses

## 🚨 **ARCHITECTURAL FAILURE: SESSION-AWARE SSE STREAMING**

**Status**: ❌ **Phase 2 Complete, Phase 3 Missing** - Session context works, SSE streaming broken  
**Impact**: Tools know which session requested them, but notifications still lost  
**Root Cause**: SSE endpoints return static responses, not connected to NotificationBroadcaster  
**Discovery**: Proper end-to-end testing reveals the architectural disconnect

### **Current Broken Architecture Flow (Post Phase 2)**
```
❌ STILL BROKEN:
Client POST /mcp → SessionMcpHandler → JsonRpcDispatcher → Tool.execute(SessionContext) ✅
                                                              ↓
                                                    GLOBAL_BROADCASTER.send_notification() ✅
                                                              ↓
                                                    [notifications sent to void] ❌

Client GET /mcp → SessionMcpHandler → Static "data: stream_established" ❌
                                        ↑
                              NOT CONNECTED TO BROADCASTER!
```

### **Root Cause Update: SSE Endpoints Disconnected**
1. ✅ **JsonRpcDispatcher**: NOW calls tools with SessionContext  
2. ✅ **Tools**: NOW know which session requested them
3. ✅ **NotificationBroadcaster**: Receives notifications successfully
4. ❌ **SSE Endpoints**: STILL return static responses instead of real streams
5. ❌ **StreamManager**: Fully implemented but NEVER USED
6. ❌ **No connection**: Broadcaster creates notifications, SSE endpoint ignores them

### **Components Completely Disconnected**
- NotificationBroadcaster → Creates events → Go nowhere
- StreamManager → Exists but unused → No real streaming  
- SessionMcpHandler → Handles requests and SSE → No connection between them
- Tools → Execute successfully → Notifications lost in void

### **Required: Session-Aware Flow**
```
✅ CORRECT:
Client POST /mcp → SessionMcpHandler (extract session_id) → JsonRpcDispatcher (with context) → Tool.execute(session_context)
                                                                                                    ↓
                                                                                        session-specific notifications
                                                                                                    ↓
                                                                                        NotificationBroadcaster (per-session)

Client GET /mcp → SessionMcpHandler → StreamManager → Real SSE Stream (connected to session's notification channel)
                                                        ↓
                                               Client receives real-time tool notifications!
```

---

## 📋 **IMMEDIATE IMPLEMENTATION PLAN** 

### **🚨 PHASE -1: FIX NOTIFICATION FORMAT VIOLATION** (NEW CRITICAL)
**Status**: ❌ **BLOCKING EVERYTHING** - Notifications violate MCP spec
**Priority**: **MUST FIX FIRST** - Nothing will work with wrong notification format

#### **Fix NotificationBroadcaster Trait**
```rust
// Update trait to use proper MCP types
use mcp_json_rpc_server::JsonRpcNotification;
use mcp_protocol::notifications::ProgressNotification;

#[async_trait]
pub trait NotificationBroadcaster: Send + Sync {
    /// Send a proper JSON-RPC progress notification
    async fn send_progress_notification(
        &self,
        session_id: &str,
        notification: ProgressNotification, // Use MCP type!
    ) -> Result<(), BroadcastError>;
    
    /// Send any JSON-RPC notification
    async fn send_notification(
        &self,
        session_id: &str,
        notification: JsonRpcNotification, // Proper JSON-RPC!
    ) -> Result<(), BroadcastError>;
}
```

#### **Fix StreamManager to Send JSON-RPC**
```rust
// StreamManager must send proper JSON-RPC notifications
pub async fn broadcast_to_session(
    &self,
    session_id: &str,
    notification: JsonRpcNotification, // NOT custom JSON!
) -> Result<u64, StreamError> {
    // Serialize to proper JSON-RPC format
    let sse_data = format!("data: {}\n\n", 
        serde_json::to_string(&notification)?);
    // Send over SSE
}
```

### **PHASE 0: END-TO-END INTEGRATION TESTING** 🧪
**Status**: ❌ **CRITICAL** - Must implement proper testing to validate all fixes
**Priority**: **IMMEDIATE** - No more false confidence from component-only testing

#### **0.1: Create Real End-to-End Test**
**File**: `/examples/streamable-http-compliance/src/integration_test.rs`

**Test Architecture Requirements**:
```rust
#[tokio::test]
async fn test_end_to_end_notification_flow() {
    // 1. Start server with real broadcaster
    let server = start_test_server().await;
    
    // 2. Create client and establish SSE connection
    let mut client = StreamableHttpClient::new(server_url);
    let sse_stream = client.start_sse_stream().await; // REAL STREAMING
    
    // 3. Call tool that sends notifications  
    let _response = client.call_tool("long_calculation", json!({"number": 3})).await;
    
    // 4. CRITICAL: Wait and verify SSE stream receives notifications
    let notifications = sse_stream.collect_events_for(Duration::from_secs(5)).await;
    
    // 5. Validate notification content and timing
    assert!(notifications.len() >= 3); // initial + steps + completion
    assert!(notifications[0].data.contains("Starting factorial"));
    assert!(notifications.last().unwrap().data.contains("complete"));
    
    // 6. Test session isolation - different sessions don't receive each other's events
    // 7. Test SSE resumability with Last-Event-ID
}
```

#### **0.2: Test Must Validate JSON-RPC Format**
```rust
// Test MUST verify proper JSON-RPC notification format
assert!(sse_event.data.contains("\"jsonrpc\":\"2.0\""));
assert!(sse_event.data.contains("\"method\":\"notifications/progress\""));
assert!(sse_event.data.contains("\"params\":{"));
assert!(!sse_event.data.contains("\"type\":\"progress\"")); // No custom format!
```

## 🚨 **UPDATED IMPLEMENTATION ORDER (CRITICAL)**

### **🛑 MANDATORY IMPLEMENTATION ORDER**
1. **Phase -1**: Fix notification format to use proper JSON-RPC (BLOCKS EVERYTHING)
2. **Phase 0**: Create end-to-end integration tests (VALIDATES FIXES)
3. **Phase 1**: Complete bridge architecture (CONNECTS SYSTEMS)
4. **Phase 2**: Validate with MCP Inspector (CONFIRMS COMPLIANCE)

### **🚨 KEY MANDATORY REQUIREMENTS (PRESERVED)**
- **Session IDs**: MUST be server-provided, never client-generated
- **Zero-Config**: Users NEVER specify method strings - framework auto-determines
- **JSON-RPC Format**: ALL notifications MUST be proper JSON-RPC with `jsonrpc: "2.0"`
- **MCP Compliance**: Use ONLY official methods from 2025-06-18 spec
- **Extend Existing**: Never create duplicate/enhanced versions of components
- **Zero Warnings**: Each phase must complete with `cargo check` showing 0 warnings

### **🎯 SUCCESS CRITERIA UPDATED**
- ✅ Notifications sent as proper JSON-RPC over SSE
- ✅ MCP Inspector can parse and display notifications correctly
- ✅ End-to-end test validates complete tool → SSE → client flow
- ✅ Session isolation works (notifications only reach intended sessions)
- ✅ All notification methods follow MCP specification exactly

#### **0.3: Client-Side SSE Stream Processing**
**File**: `/examples/streamable-http-compliance/src/client.rs`

**Add Real Streaming Support**:
```rust
impl StreamableHttpClient {
    /// Start real SSE stream that processes events continuously
    pub async fn start_sse_stream(&mut self) -> SseEventStream {
        let response = self.client
            .get(&self.base_url)
            .header("Accept", "text/event-stream")
            .header("Mcp-Session-Id", &self.session_id)
            .send()
            .await?;
            
        // Process streaming response, NOT static response
        SseEventStream::new(response.bytes_stream())
    }
}

pub struct SseEventStream {
    // Actual stream processing, not just connection test
    pub async fn collect_events_for(&mut self, duration: Duration) -> Vec<SseEvent>;
}
```

**Phase 0 Todo List**:
- [ ] Create end-to-end integration test file
- [ ] Implement SseEventStream with real streaming processing
- [ ] Add comprehensive server-side notification logging
- [ ] Test notification flow: tool → broadcaster → SSE → client
- [ ] Test session isolation (multiple clients, different sessions)
- [ ] Test SSE resumability with Last-Event-ID
- [ ] Validate that test FAILS before Phase 3 implementation
- [ ] Document expected test failure (proves test works correctly)

---

## 📋 **CORE ARCHITECTURE IMPLEMENTATION PLAN**

### **PHASE 1: Fix Compilation Issues** ⚡
**Status**: ❌ BLOCKING - Must fix first

**Problem**: Response borrowing after move in error handling
**Files**: `/examples/streamable-http-compliance/src/client.rs`

**Exact Fixes Needed** (4 locations):
```rust
// Lines 95, 143, 187, 229 - Current broken code:
let error_body = response.text().await.unwrap_or_else(...);
error!("❌ Failed with status: {}", response.status()); // ← response moved!

// Fix:
let status = response.status();
let error_body = response.text().await.unwrap_or_else(...);
error!("❌ Failed with status: {}", status);
```

**Phase 1 Todo List**:
- [ ] Fix response borrowing in client.rs line 95 (initialize error handling)
- [ ] Fix response borrowing in client.rs line 143 (notification error handling)
- [ ] Fix response borrowing in client.rs line 187 (tool call error handling)
- [ ] Fix response borrowing in client.rs line 229 (SSE connection error handling)
- [ ] Verify `cargo check` passes with no compilation errors
- [ ] Ensure no new warnings introduced

---

### **PHASE 2: Session Context Propagation** 🔄
**Status**: ❌ CRITICAL - Core architectural fix

#### **2.1: Extend JsonRpcHandler Trait**
**File**: `/crates/json-rpc-server/src/async.rs`

**Current Problem**:
```rust
// Line 142: NO SESSION CONTEXT!
match handler.handle(&request.method, request.params).await {
```

**Solution**:
```rust
// Add session context to trait
#[async_trait]
pub trait JsonRpcHandler: Send + Sync {
    async fn handle(&self, method: &str, params: Option<RequestParams>, session_context: Option<SessionContext>) -> JsonRpcResult<Value>;
}

// New session context struct
#[derive(Debug, Clone)]
pub struct SessionContext {
    pub session_id: String,
    pub broadcaster: Option<Arc<dyn NotificationBroadcaster>>,
    pub timestamp: u64,
}
```

#### **2.2: Update JsonRpcDispatcher**
**File**: `/crates/json-rpc-server/src/async.rs:136-171`

**Add Method**:
```rust
impl JsonRpcDispatcher {
    pub async fn handle_request_with_context(&self, request: JsonRpcRequest, session_context: SessionContext) -> JsonRpcResponse {
        let handler = self.handlers.get(&request.method).or(self.default_handler.as_ref());
        
        match handler {
            Some(handler) => {
                match handler.handle(&request.method, request.params, Some(session_context)).await {
                    // ... existing error handling
                }
            }
            // ... existing None handling
        }
    }
}
```

#### **2.3: Connect SessionMcpHandler**
**File**: `/crates/http-mcp-server/src/session_handler.rs:215`

**Current**:
```rust
let response = self.dispatcher.handle_request(request).await;
```

**Fix**:
```rust
let session_context = SessionContext {
    session_id: session_id.unwrap_or("unknown".to_string()),
    broadcaster: Some(self.notification_broadcaster.clone()),
    timestamp: chrono::Utc::now().timestamp_millis() as u64,
};
let response = self.dispatcher.handle_request_with_context(request, session_context).await;
```

**Phase 2 Todo List**:
- [ ] Add SessionContext struct to json-rpc-server crate
- [ ] Extend JsonRpcHandler trait with session context parameter
- [ ] Update JsonRpcHandler::handle signature in trait definition
- [ ] Add JsonRpcDispatcher::handle_request_with_context method
- [ ] Update all existing handler implementations to accept session context parameter
- [ ] Add notification broadcaster imports to session_handler.rs
- [ ] Update SessionMcpHandler to create and pass session context from headers
- [ ] Verify `cargo check` passes with no compilation errors
- [ ] Ensure no warnings about unused session context parameters

---

### **PHASE 3: Real SSE Streaming Infrastructure** 📡  
**Status**: ❌ CRITICAL - SSE is fake

#### **3.1: Add Components to SessionMcpHandler**
**File**: `/crates/http-mcp-server/src/session_handler.rs:77-82`

**Current**:
```rust
pub struct SessionMcpHandler<S: SessionStorage = InMemorySessionStorage> {
    pub(crate) config: ServerConfig,
    pub(crate) dispatcher: Arc<JsonRpcDispatcher>,
    pub(crate) session_storage: Arc<S>,
    pub(crate) stream_config: StreamConfig,
}
```

**Fix**:
```rust
use crate::stream_manager::StreamManager;
use crate::notification_broadcaster::{NotificationBroadcaster, ChannelNotificationBroadcaster};

pub struct SessionMcpHandler<S: SessionStorage = InMemorySessionStorage> {
    pub(crate) config: ServerConfig,
    pub(crate) dispatcher: Arc<JsonRpcDispatcher>,
    pub(crate) session_storage: Arc<S>,
    pub(crate) stream_config: StreamConfig,
    pub(crate) stream_manager: Arc<StreamManager<S>>,        // ← ADD
    pub(crate) notification_broadcaster: Arc<dyn NotificationBroadcaster>, // ← ADD
}
```

#### **3.2: Replace Static SSE with Real Streaming**
**File**: `/crates/http-mcp-server/src/session_handler.rs:306-320`

**Current (BROKEN)**:
```rust
let sse_data = format!("data: {{\"type\":\"stream_established\",...}}}\n\n");
Ok(Response::builder()
    .body(Full::new(Bytes::from(sse_data))) // ← STATIC!
    .unwrap())
```

**Fix**:
```rust
// Extract Last-Event-ID from headers
let last_event_id = req.headers()
    .get("Last-Event-ID")
    .and_then(|v| v.to_str().ok())
    .and_then(|s| s.parse::<u64>().ok());

// Create real SSE stream
let sse_response = self.stream_manager.handle_sse_connection(
    session_id,
    "main".to_string(),
    last_event_id
).await?;

Ok(sse_response) // Real streaming response!
```

#### **3.3: Update Constructor and Clone**
**File**: `/crates/http-mcp-server/src/session_handler.rs:113-116`

**Add Constructor**:
```rust
impl<S: SessionStorage + 'static> SessionMcpHandler<S> {
    pub fn with_streaming(
        config: ServerConfig,
        dispatcher: Arc<JsonRpcDispatcher>,
        session_storage: Arc<S>,
        stream_config: StreamConfig,
    ) -> Self {
        let stream_manager = Arc::new(StreamManager::with_config(
            session_storage.clone(),
            stream_config.clone()
        ));
        
        let broadcaster = Arc::new(ChannelNotificationBroadcaster::with_buffer_size(
            stream_config.channel_buffer_size
        ));

        Self { 
            config, 
            dispatcher, 
            session_storage, 
            stream_config, 
            stream_manager,
            notification_broadcaster: broadcaster,
        }
    }
}
```

**Phase 3 Todo List**:
- [ ] Add StreamManager and NotificationBroadcaster imports to session_handler.rs
- [ ] Add stream_manager and notification_broadcaster fields to SessionMcpHandler struct
- [ ] Create with_streaming constructor that instantiates streaming components
- [ ] Update Clone implementation to include new fields
- [ ] Replace static SSE response in handle_sse_request with StreamManager.handle_sse_connection
- [ ] Extract Last-Event-ID header and pass to StreamManager
- [ ] Update existing with_storage constructor to use with_streaming internally
- [ ] Verify `cargo check` passes with no compilation errors
- [ ] Ensure no warnings about unused stream_manager or notification_broadcaster fields

---

### **PHASE 4: Session-Aware Tool Integration** 🔧
**Status**: ❌ CRITICAL - Tools don't know their session

#### **4.1: Update Tool Framework**
**File**: `/crates/mcp-server/src/lib.rs` (McpTool trait)

**Add Session Context Support**:
```rust
use crate::SessionContext; // Import from json-rpc-server

#[async_trait]
pub trait McpTool: Send + Sync {
    async fn execute(&self) -> McpResult<CallToolResult>; // Keep existing
    
    // Add session-aware execution
    async fn execute_with_context(&self, session_context: SessionContext) -> McpResult<CallToolResult> {
        // Default implementation calls existing execute()
        self.execute().await
    }
}
```

#### **4.2: Fix Tool Implementations**
**File**: `/examples/streamable-http-compliance/src/main.rs`

**Current (BROKEN)**:
```rust
let session_id = "default-session"; // ← HARDCODED!
```

**Fix**:
```rust
impl LongCalculationTool {
    async fn execute_with_context(&self, session_context: SessionContext) -> McpResult<CallToolResult> {
        let session_id = &session_context.session_id; // ← REAL SESSION!
        
        if let Some(broadcaster) = session_context.broadcaster {
            broadcaster.send_progress_notification(
                session_id, // ← GOES TO CORRECT SESSION!
                &progress_token,
                progress,
                total,
                message
            ).await?;
        }
        
        // ... rest of tool logic
    }
}
```

#### **4.3: Connect to MCP Framework**
**File**: `/crates/mcp-server/src/handlers/tools.rs` (or equivalent)

**Update Tool Handler**:
```rust
impl JsonRpcHandler for ToolHandler {
    async fn handle(&self, method: &str, params: Option<RequestParams>, session_context: Option<SessionContext>) -> JsonRpcResult<Value> {
        if let Some(context) = session_context {
            // Call tool with session context
            tool.execute_with_context(context).await
        } else {
            // Fallback to session-less execution
            tool.execute().await
        }
    }
}
```

**Phase 4 Todo List**:
- [ ] Add SessionContext import to mcp-server crate
- [ ] Extend McpTool trait with execute_with_context method
- [ ] Update LongCalculationTool to implement execute_with_context
- [ ] Update SystemHealthTool to implement execute_with_context
- [ ] Remove all hardcoded "default-session" references
- [ ] Update MCP framework tool handlers to pass session context to tools
- [ ] Ensure tools send notifications via session_context.broadcaster
- [ ] Verify `cargo check` passes with no compilation errors
- [ ] Ensure no warnings about unused session context in tools

---

### **PHASE 5: StreamManager Integration** 🌊
**Status**: ❌ UNUSED - StreamManager exists but never used

#### **5.1: Update Server Builder**
**File**: `/crates/http-mcp-server/src/server.rs:195-200`

**Current**:
```rust
let handler = SessionMcpHandler::with_storage(
    self.config.clone(),
    Arc::clone(&self.dispatcher),
    Arc::clone(&self.session_storage),
    self.stream_config.clone(),
);
```

**Fix**:
```rust
let handler = SessionMcpHandler::with_streaming(  // ← Use streaming constructor
    self.config.clone(),
    Arc::clone(&self.dispatcher),
    Arc::clone(&self.session_storage),
    self.stream_config.clone(),
);
```

#### **5.2: Connect Notification Broadcasting**
**File**: `/examples/streamable-http-compliance/src/main.rs:290-295`

**Current**:
```rust
// Initialize global broadcaster
GLOBAL_BROADCASTER.set(broadcaster.clone() as Arc<dyn NotificationBroadcaster>)
    .map_err(|_| anyhow::anyhow!("Failed to initialize global broadcaster"))?;
```

**Fix**: Remove global broadcaster - session handlers will have their own
```rust
// Remove GLOBAL_BROADCASTER - each SessionMcpHandler has its own broadcaster
// Tools get broadcaster via session_context instead of global static
```

**Phase 5 Todo List**:
- [ ] Update HttpMcpServer to use SessionMcpHandler::with_streaming constructor
- [ ] Remove GLOBAL_BROADCASTER static and related code from main.rs
- [ ] Remove get_broadcaster() function and references
- [ ] Update session cleanup to integrate with StreamManager.cleanup_broadcasters
- [ ] Ensure StreamManager is instantiated and used for all SSE connections
- [ ] Verify `cargo check` passes with no compilation errors
- [ ] Ensure no warnings about unused StreamManager methods or global broadcaster

---

### **PHASE 6: End-to-End Integration Testing** 🧪
**Status**: ❌ UNTESTED - Need to verify complete flow

#### **6.1: Update Client for Full Testing**
**File**: `/examples/streamable-http-compliance/src/client.rs`

**Add Real SSE Stream Reading**:
```rust
pub async fn listen_to_sse_stream(&mut self, duration: Duration) -> Result<Vec<SseEvent>> {
    // Actually read SSE events instead of just testing connection
    let mut request = self.client
        .get(&self.base_url)
        .header("Accept", "text/event-stream")
        .header("Mcp-Session-Id", &self.session_id);
        
    if let Some(ref last_event_id) = self.last_event_id {
        request = request.header("Last-Event-ID", last_event_id);
    }
    
    let response = request.send().await?;
    // Parse actual SSE events from stream
    // Return collected events for verification
}
```

#### **6.2: Integration Test Cases**
**Test Scenarios**:
1. **Tool Notification Routing**: Call tool → verify notifications reach correct session
2. **Session Isolation**: Multiple sessions → verify notifications don't leak
3. **SSE Resumability**: Disconnect/reconnect with Last-Event-ID → verify replay
4. **Progress Tracking**: Long calculation → verify all progress updates received
5. **System Notifications**: Health check → verify fan-out to all active sessions

**Phase 6 Todo List**:
- [ ] Add listen_to_sse_stream method to client for reading actual events
- [ ] Implement SSE event parsing in client
- [ ] Add integration test case for tool notification routing
- [ ] Add integration test case for session isolation verification
- [ ] Add integration test case for Last-Event-ID resumability
- [ ] Add integration test case for progress notification streaming
- [ ] Add integration test case for system notification fan-out
- [ ] Verify MCP 2025-06-18 compliance (202 Accepted, proper headers, event IDs)
- [ ] Ensure `cargo check` and `cargo test` pass with no warnings
- [ ] Verify complete end-to-end integration test passes

---

## **🚨 CRITICAL GAPS IDENTIFIED (ADDITIONAL)**

### **Gap 1: SessionStorage Trait Architecture** ⚡ **HIGHEST PRIORITY**
**Current**: Hard-coded in-memory state via closures
**Missing**: Pluggable backend trait abstraction

```rust
// NEEDS TO BE BUILT - Core trait for all session backends
#[async_trait]
pub trait SessionStorage: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;
    
    // Session lifecycle
    async fn create_session(&self, session_id: String, capabilities: ServerCapabilities) -> Result<SessionInfo, Self::Error>;
    async fn get_session(&self, session_id: &str) -> Result<Option<SessionInfo>, Self::Error>;
    async fn update_session(&self, session_id: &str, info: SessionInfo) -> Result<(), Self::Error>;
    async fn delete_session(&self, session_id: &str) -> Result<bool, Self::Error>;
    
    // Stream management per session
    async fn create_stream(&self, session_id: &str, stream_id: String) -> Result<StreamInfo, Self::Error>;
    async fn get_stream(&self, session_id: &str, stream_id: &str) -> Result<Option<StreamInfo>, Self::Error>;
    async fn update_stream(&self, session_id: &str, stream_id: &str, info: StreamInfo) -> Result<(), Self::Error>;
    async fn delete_stream(&self, session_id: &str, stream_id: &str) -> Result<bool, Self::Error>;
    
    // Event persistence for SSE resumability
    async fn store_event(&self, session_id: &str, stream_id: &str, event_id: u64, event: SseEvent) -> Result<(), Self::Error>;
    async fn get_events_after(&self, session_id: &str, stream_id: &str, after_event_id: u64) -> Result<Vec<(u64, SseEvent)>, Self::Error>;
    
    // Cleanup and maintenance
    async fn expire_sessions(&self, older_than: std::time::SystemTime) -> Result<Vec<String>, Self::Error>;
    async fn list_sessions(&self) -> Result<Vec<String>, Self::Error>;
}
```

### **Gap 2: Multiple Backend Implementations** ⚡ **CRITICAL**
**Current**: Only basic in-memory (via closures)
**Missing**: Production-ready backends for different deployment scenarios

#### **Phase 1: InMemorySessionStorage** (1-2 days)
- Enhanced version of current implementation
- Proper session metadata tracking
- Stream management per session
- Event storage for resumability
- Cleanup and expiration

#### **Phase 2: SqliteSessionStorage** (2-3 days)
- Local file-based persistence
- ACID transactions for session operations
- Efficient event querying with indexes
- Migration support

#### **Phase 3: PostgresqlSessionStorage** (3-4 days)  
- Production database backend
- Connection pooling
- Optimized queries for high throughput
- Distributed session support

#### **Phase 4: NatsSessionStorage** (4-5 days)
- JetStream for event persistence
- Key-Value store for session metadata
- Distributed coordination
- Real-time event streaming

#### **Phase 5: AwsSessionStorage** (5-7 days)
- DynamoDB for session/stream metadata
- SNS for cross-instance notifications
- S3 for large event payloads
- Lambda-optimized design

### **Gap 3: SSE Resumability & Event Management** ⚡ **CRITICAL**
**Current**: Simple broadcast with no persistence or ordering
**Missing**: MCP 2025-06-18 compliant SSE with resumability

```rust
// NEEDS TO BE BUILT - Enhanced SSE events with proper metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseEvent {
    pub id: u64,                    // MISSING: Monotonic event ID
    pub timestamp: u64,             // MISSING: Event timestamp  
    pub stream_id: String,          // MISSING: Per-session stream ID
    pub event_type: String,         // MISSING: Event type classification
    pub data: Value,                // EXISTS: Event payload
    pub retry: Option<u32>,         // MISSING: Retry timeout
}

// NEEDS TO BE BUILT - Stream management
pub struct StreamManager {
    session_storage: Arc<dyn SessionStorage>,
    event_counter: AtomicU64,       // MISSING: Global event ID counter
}

impl StreamManager {
    // MISSING: Last-Event-ID support  
    pub async fn handle_sse_request(&self, session_id: &str, last_event_id: Option<u64>) -> Result<impl Stream<Item = SseEvent>, Error>;
    
    // MISSING: Per-session event broadcasting
    pub async fn broadcast_to_session(&self, session_id: &str, event: SseEvent) -> Result<(), Error>;
    
    // MISSING: Event persistence
    pub async fn store_and_broadcast(&self, session_id: &str, stream_id: &str, event: SseEvent) -> Result<(), Error>;
}
```

### **Gap 4: HTTP MCP 2025-06-18 Compliance** 🔧 **IMPORTANT**
**Current**: Basic HTTP with generic responses
**Missing**: Specification-compliant status codes and headers

#### **Missing HTTP Features:**
- **202 Accepted** responses for notifications (not 200/204)
- **Content-Type switching** between `application/json` and `text/event-stream`
- **Session ID assignment** in `InitializeResult` response via `Mcp-Session-Id` header
- **DELETE /mcp/{session-id}** endpoint for explicit session termination
- **Last-Event-ID** header processing for SSE resumption

### **Gap 5: Zero-Config Derive Macros** 🔧 **IMPORTANT**  
**Current**: Derive macros require method string attributes
**Missing**: True zero-configuration with automatic method determination

```rust
// CURRENT (BAD) - User specifies method strings
#[derive(McpNotification)]
#[notification(method = "notifications/progress")]  // ❌ METHOD STRINGS!
struct ProgressNotification;

// TARGET (GOOD) - Framework auto-determines methods
#[derive(McpNotification)]  
struct ProgressNotification;  // Framework → notifications/progress (from type name)

#[derive(McpTool)]
struct Calculator;  // Framework → tools/call (from trait implementation)

#[derive(McpResource)]
struct FileResource;  // Framework → resources/read (from trait implementation)
```

## 🛠️ **Implementation Priority Order**

### **Week 1: Core Session Architecture**
1. **SessionStorage Trait** (1 day)
2. **InMemorySessionStorage** (2 days)  
3. **Enhanced SessionManager** (2 days)

### **Week 2: SSE Resumability**  
4. **Enhanced SseEvent with IDs** (1 day)
5. **StreamManager with persistence** (2 days)
6. **Last-Event-ID support** (2 days)

### **Week 3: HTTP Compliance**
7. **202 Accepted responses** (1 day)
8. **Session ID headers** (1 day) 
9. **DELETE endpoint** (1 day)
10. **Content-Type switching** (2 days)

### **Week 4: Database Backends**
11. **SqliteSessionStorage** (3 days)
12. **Testing and integration** (2 days)

### **Week 5+: Advanced Backends**
13. **PostgresqlSessionStorage** (Week 5)
14. **NatsSessionStorage** (Week 6)  
15. **AwsSessionStorage** (Week 7)

## 🎯 **Success Criteria**

### **Tier 1: Foundation (Weeks 1-2)**
- ✅ Pluggable SessionStorage trait working
- ✅ InMemorySessionStorage with full feature set
- ✅ SSE resumability with Last-Event-ID support
- ✅ Per-session stream management

### **Tier 2: Production Ready (Weeks 3-4)**  
- ✅ HTTP MCP 2025-06-18 compliance
- ✅ SQLite backend working
- ✅ Proper session lifecycle management
- ✅ Zero-config derive macros

### **Tier 3: Distributed (Weeks 5+)**
- ✅ PostgreSQL production backend
- ✅ NATS distributed backend  
- ✅ AWS Lambda-optimized backend
- ✅ Performance benchmarks and optimization

---

## 🎯 **IMMEDIATE FIX PLAN: DON'T REINVENT - COPY GPS PATTERN**

### **✅ What's Already Working**
- SessionContext propagation from JsonRpcHandler ✅
- Client handles optional session IDs ✅  
- Dual connection model (POST/GET) ✅
- NotificationBroadcaster infrastructure exists ✅

### **🚨 Critical Issues to Fix**

#### **1. Server Must Create Sessions During Initialize**
- Copy GPS project's `new_session()` pattern
- Create session UUID during initialize request processing
- Return session ID via HTTP headers (not body)

#### **2. HTTP Layer Must Set Session Headers**  
- Implement `ResponseWithSession` pattern from GPS project
- Set `Mcp-Session-Id` header in HTTP responses
- Client will automatically use header for subsequent requests

#### **3. Connect SSE to Real Streaming**
- Replace static SSE responses with actual NotificationBroadcaster connections
- Connect session broadcast channels to SSE endpoints
- Remove hardcoded SSE response data

#### **4. Tools Must Use SessionContext for Notifications**
- Fix "unused variable" warnings by actually using session_context
- Tools should send notifications via broadcaster when they have session context
- Remove hardcoded session IDs from tools (like "unknown-session")

### **🔧 Implementation Approach**
1. **Don't reinvent** - copy proven patterns from GPS project
2. **Fix session_context usage** - use the existing parameter properly  
3. **Connect the pipes** - link NotificationBroadcaster to SSE endpoints
4. **Follow MCP protocol** - server provides session IDs, not clients

### **🛑 MANDATORY STOP GATE**
**CRITICAL**: Implementation must stop after fixing initialize session creation
- User confirmation required before proceeding to SSE streaming fixes
- Must verify session IDs are server-provided via client-initialise-report
- Must pass initialize session tests before touching downstream components
- This ensures root architectural issue (missing session creation) is fixed first

**BOTTOM LINE**: Framework needs **SOLID FOUNDATION** before examples. Current architecture is prototype-level, missing production session management, SSE resumability, and HTTP compliance required by MCP 2025-06-18 specification.