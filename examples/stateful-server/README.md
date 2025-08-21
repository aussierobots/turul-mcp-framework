# Session-Aware Stateful Server Example

This example demonstrates session-based state management in an MCP server, showing how to maintain persistent state across multiple requests within the same session and handle session lifecycle events.

## 🚀 What This Example Shows

- **Session State Management**: Persistent state across multiple requests
- **State Persistence**: Automatic state cleanup when sessions expire
- **Real-time Notifications**: Progress notifications for state changes
- **Type-Safe State**: Using typed state storage and retrieval
- **Session Context**: Leveraging session information in tools
- **Complex State Operations**: Managing shopping carts and user preferences

## 🛠️ Available Tools

### 1. Shopping Cart (`shopping_cart`)
Manage a persistent shopping cart with state across requests:

**Actions:**
- **`add`**: Add items to cart with quantity and price
- **`remove`**: Remove specific quantities from cart
- **`list`**: Display all cart contents with totals
- **`clear`**: Empty the entire cart

**Parameters:**
- `action` (string): Action to perform
- `item` (string): Item name (required for add/remove)
- `quantity` (integer): Item quantity (default: 1)
- `price` (number): Item price (required for add)

### 2. User Preferences (`user_preferences`)
Manage user preferences with session persistence:

**Actions:**
- **`set`**: Set a preference key-value pair
- **`get`**: Retrieve a specific preference
- **`list`**: List all preferences
- **`reset`**: Clear all preferences

**Parameters:**
- `action` (string): Action to perform
- `key` (string): Preference key (required for set/get)
- `value` (object): Preference value (required for set)

### 3. Session Info (`session_info`)
Get information about the current session:

**Returns:**
- Session ID
- Initialization status
- Session availability

## 🏃 Running the Example

```bash
cargo run -p stateful-server
```

The server starts on `http://127.0.0.1:8006/mcp` with SSE (Server-Sent Events) enabled for real-time notifications.

## 🧪 Testing Session State

### 1. Initialize a Session
```bash
curl -X POST http://127.0.0.1:8006/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "initialize",
    "params": {
      "protocolVersion": "2025-06-18",
      "capabilities": {},
      "clientInfo": {"name": "test-client", "version": "1.0.0"}
    },
    "id": "1"
  }'
```

### 2. Shopping Cart Operations

#### Add Items to Cart
```bash
curl -X POST http://127.0.0.1:8006/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "shopping_cart",
      "arguments": {
        "action": "add",
        "item": "apple",
        "quantity": 5,
        "price": 1.50
      }
    },
    "id": "2"
  }'
```

#### Add More Items
```bash
curl -X POST http://127.0.0.1:8006/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "shopping_cart",
      "arguments": {
        "action": "add",
        "item": "banana",
        "quantity": 3,
        "price": 0.75
      }
    },
    "id": "3"
  }'
```

#### List Cart Contents
```bash
curl -X POST http://127.0.0.1:8006/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "shopping_cart",
      "arguments": {
        "action": "list"
      }
    },
    "id": "4"
  }'
```

#### Remove Items
```bash
curl -X POST http://127.0.0.1:8006/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "shopping_cart",
      "arguments": {
        "action": "remove",
        "item": "apple",
        "quantity": 2
      }
    },
    "id": "5"
  }'
```

### 3. User Preferences Operations

#### Set Preferences
```bash
curl -X POST http://127.0.0.1:8006/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "user_preferences",
      "arguments": {
        "action": "set",
        "key": "theme",
        "value": {"mode": "dark", "accent": "blue"}
      }
    },
    "id": "6"
  }'
```

#### Get Specific Preference
```bash
curl -X POST http://127.0.0.1:8006/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "user_preferences",
      "arguments": {
        "action": "get",
        "key": "theme"
      }
    },
    "id": "7"
  }'
```

#### List All Preferences
```bash
curl -X POST http://127.0.0.1:8006/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "user_preferences",
      "arguments": {
        "action": "list"
      }
    },
    "id": "8"
  }'
```

### 4. Session Information
```bash
curl -X POST http://127.0.0.1:8006/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "session_info",
      "arguments": {}
    },
    "id": "9"
  }'
```

## 🔧 Key Session Management Features

### 1. Session Context Usage
```rust
async fn call(&self, args: Value, session: Option<SessionContext>) -> Result<Vec<ToolResult>, String> {
    let session = session.ok_or("This tool requires session context")?;
    
    // Get typed state
    let mut cart_items: HashMap<String, (i64, f64)> = session.get_typed_state("cart_items")
        .unwrap_or_default();
    
    // Modify state
    cart_items.insert("new_item".to_string(), (1, 9.99));
    
    // Save state back to session
    session.set_typed_state("cart_items", &cart_items).unwrap();
    
    // Send progress notifications
    session.notify_progress("cart_update", 1);
    
    Ok(vec![ToolResult::text("Updated".to_string())])
}
```

### 2. State Persistence Patterns

#### Type-Safe State Storage
```rust
// Store complex data structures
let cart_items: HashMap<String, (i64, f64)> = HashMap::new();
session.set_typed_state("cart_items", &cart_items).unwrap();

// Retrieve with default fallback
let cart_items: HashMap<String, (i64, f64)> = session.get_typed_state("cart_items")
    .unwrap_or_default();
```

#### Progress Notifications
```rust
// Notify about cart operations
session.notify_progress(format!("cart_item_{}", item), 1);
session.notify_progress("cart_clear", 1);
session.notify_progress(format!("cart_remove_{}", item), 1);
```

### 3. Session Lifecycle

#### Automatic Cleanup
- Sessions expire after inactivity (default: 30 minutes)
- State is automatically cleaned up when sessions expire
- New sessions start with empty state

#### Session Validation
```rust
// Tools can require session context
let session = session.ok_or("This tool requires session context")?;

// Check session status
let is_initialized = (session.is_initialized)();
let session_id = &session.session_id;
```

## 💡 State Management Patterns

### 1. Shopping Cart Pattern
Demonstrates complex state with multiple operations:
- Adding items with validation
- Removing specific quantities
- Calculating totals and subtotals
- Clearing all contents

### 2. User Preferences Pattern
Shows key-value state management:
- Setting arbitrary preferences
- Retrieving specific values
- Listing all preferences
- Resetting to defaults

### 3. Session Information Pattern
Provides introspection capabilities:
- Session ID access
- Initialization status
- State availability checking

## 🔄 Real-time Notifications

The server sends SSE notifications for:
- **Cart Operations**: Item additions, removals, clears
- **Preference Changes**: Setting new preferences
- **Progress Updates**: Real-time operation feedback

### Listening to SSE Events
```bash
curl -N http://127.0.0.1:8006/sse
```

## 📊 Example State Evolution

### Initial State
```json
{
  "cart_items": {},
  "user_preferences": {}
}
```

### After Adding Items
```json
{
  "cart_items": {
    "apple": [5, 1.50],
    "banana": [3, 0.75]
  },
  "user_preferences": {}
}
```

### After Setting Preferences
```json
{
  "cart_items": {
    "apple": [5, 1.50],
    "banana": [3, 0.75]
  },
  "user_preferences": {
    "theme": {"mode": "dark", "accent": "blue"},
    "language": "en",
    "notifications": true
  }
}
```

## 🎯 Use Cases Demonstrated

### 1. E-commerce Applications
- Shopping cart management
- User preference storage
- Session-based workflows

### 2. Personalization Systems
- User settings persistence
- Customization options
- Session-specific configurations

### 3. Multi-step Workflows
- State preservation across steps
- Progress tracking
- Rollback capabilities

### 4. Real-time Applications
- Live state updates
- Progress notifications
- Event-driven interactions

## 🚨 Production Considerations

### 1. State Persistence
In production, consider:
- Database-backed session storage
- Redis or other external state stores
- State replication for high availability
- Backup and recovery strategies

### 2. Session Management
- Configurable session timeouts
- Active session monitoring
- Resource usage limits
- Security considerations

### 3. Scalability
- Distributed session storage
- Load balancing strategies
- State sharding approaches
- Cache optimization

## 🔧 Advanced Features

### Custom State Serialization
```rust
// Custom state types
#[derive(Serialize, Deserialize)]
struct CustomState {
    data: Vec<String>,
    metadata: HashMap<String, Value>,
}

// Store custom types
let state = CustomState { /* ... */ };
session.set_typed_state("custom", &state).unwrap();

// Retrieve custom types
let state: CustomState = session.get_typed_state("custom")
    .unwrap_or_default();
```

### State Validation
```rust
// Validate state before operations
if cart_items.len() > MAX_CART_ITEMS {
    return Err("Cart is full".to_string());
}

if quantity <= 0 {
    return Err("Quantity must be positive".to_string());
}
```

### Transaction-like Operations
```rust
// Save state only if operation succeeds
let mut cart_backup = cart_items.clone();
match perform_operation(&mut cart_items) {
    Ok(_) => session.set_typed_state("cart_items", &cart_items).unwrap(),
    Err(e) => {
        cart_items = cart_backup; // Rollback
        return Err(e);
    }
}
```

## 📚 Related Examples

### Simpler Examples
- **[minimal-server](../minimal-server/)**: Basic server without state
- **[derive-macro-server](../derive-macro-server/)**: Macro-based tools

### Advanced Examples
- **[notification-server](../notification-server/)**: Real-time notifications focus
- **[comprehensive-server](../comprehensive-server/)**: All MCP features

### Performance Testing
- **[performance-testing](../performance-testing/)**: Load testing with session management

## 🤝 Best Practices

1. **State Design**: Keep state structures simple and serializable
2. **Error Handling**: Always validate state operations
3. **Resource Management**: Clean up resources in state operations
4. **Notifications**: Use progress notifications for user feedback
5. **Session Validation**: Always check for session availability
6. **State Size**: Monitor and limit state size per session
7. **Backup Strategy**: Implement state backup for critical operations

---

This example demonstrates the power of session-based state management in MCP servers, enabling rich, interactive applications that maintain context across multiple requests.