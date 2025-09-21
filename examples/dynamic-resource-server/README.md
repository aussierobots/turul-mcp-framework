# Dynamic Resource Server Example

A comprehensive demonstration of **dynamic MCP resources** with parameterized URIs for accessing specific entities. This example shows how real-world resource systems should work with identifiers instead of static paths.

## Overview

Unlike static resources with fixed URIs like `file://config.json`, this server demonstrates **dynamic resources** that accept identifiers to access specific entities:

- `file:///users/user-123` - Specific user profile
- `file:///products/prod-456` - Specific product information
- `file:///documents/doc-789` - Specific document content
- `file:///orders/order-2024-001` - Specific order details

## Features

### 🔗 **Dynamic URI Patterns**
- **Parameterized resources** with entity identifiers
- **Collection resources** for listing entities
- **Multiple content types** per resource
- **Proper error handling** for missing entities

### 📊 **Rich Sample Data**
- **50 users** with profiles, preferences, and metadata
- **100 products** with pricing, inventory, and images
- **30 documents** with content, metadata, and tags
- **75 orders** with items, status, and shipping info

### 🎯 **Real-world Patterns**
- **Entity relationships** (orders reference users and products)
- **Multiple representations** (JSON + human-readable text)
- **Metadata enrichment** (access counts, timestamps)
- **Business logic** (stock status, delivery estimates)

## Quick Start

### 1. Start the Server

```bash
cargo run -p dynamic-resource-server
```

The server will start on `http://127.0.0.1:8048/mcp`

### 2. Access Resources

#### Get Specific User Profile
```json
{
  "method": "resources/read",
  "params": {
    "uri": "users://user-001"
  }
}
```

#### Get Product Information
```json
{
  "method": "resources/read",
  "params": {
    "uri": "products://prod-0025"
  }
}
```

#### Get Document Content
```json
{
  "method": "resources/read",
  "params": {
    "uri": "documents://doc-005"
  }
}
```

#### Get Order Details
```json
{
  "method": "resources/read",
  "params": {
    "uri": "orders://order-2024-0010"
  }
}
```

#### List All Available Resources
```json
{
  "method": "resources/list"
}
```

## Resource Patterns

### 👤 User Resources

**Pattern:** `users://user-{id}`

**Example:** `users://user-001`

**Content:**
- Complete user profile with preferences
- Role and permission information  
- Activity status and metadata
- Profile image URLs

**Sample Response:**
```json
{
  "id": "user-001",
  "name": "User 1",
  "email": "user1@example.com",
  "role": "admin",
  "created_at": "2024-01-15T10:30:00Z",
  "is_active": true,
  "preferences": {
    "theme": "dark",
    "notifications": true,
    "language": "en"
  },
  "profile_image_url": "https://api.dicebear.com/7.x/avataaars/svg?seed=user1"
}
```

### 📦 Product Resources

**Pattern:** `products://prod-{id}`

**Example:** `products://prod-0025`

**Content:**
- Product details with pricing and inventory
- Category and tag information
- Image galleries and availability status
- Stock levels and restock estimates

**Sample Response:**
```json
{
  "id": "prod-0025",
  "name": "Product 25",
  "description": "High-quality product 25 with excellent features",
  "price": 149.99,
  "category": "Electronics",
  "in_stock": 45,
  "tags": ["tag5", "electronics"],
  "images": [
    "https://picsum.photos/400/300?random=25",
    "https://picsum.photos/400/300?random=1025"
  ],
  "availability": {
    "status": "in_stock",
    "quantity": 45,
    "estimated_restock": null
  }
}
```

### 📄 Document Resources

**Pattern:** `documents://doc-{id}`

**Example:** `documents://doc-005`

**Content:**
- Document content in markdown format
- Metadata with author and version info
- Tags and categorization
- File size and type information

**Sample Response:**
```json
{
  "id": "doc-005",
  "title": "Document 5: Important manual File",
  "doc_type": "manual",
  "author": "Author 6",
  "created_at": "2024-11-20T08:15:30Z",
  "updated_at": "2024-12-15T14:22:10Z",
  "tags": ["manual", "category-0"],
  "file_size": 524288,
  "mime_type": "text/markdown"
}
```

### 🛒 Order Resources

**Pattern:** `orders://order-{id}`

**Example:** `orders://order-2024-0010`

**Content:**
- Complete order information with items
- Customer and shipping details
- Payment method and status
- Calculated totals and delivery estimates

**Sample Response:**
```json
{
  "id": "order-2024-0010",
  "customer_id": "user-010",
  "status": "shipped",
  "items": [
    {
      "product_id": "prod-0010",
      "quantity": 2,
      "unit_price": 89.99,
      "subtotal": 179.98
    }
  ],
  "total_amount": 179.98,
  "shipping_address": {
    "street": "100 Main St",
    "city": "Sample City",
    "state": "SC",
    "zip": "10010"
  },
  "estimated_delivery": "2024-12-21T12:00:00Z"
}
```

## Collection Resources

### List All Users
**URI:** `users://`

Returns a summary list of all available users with basic information.

### List All Products  
**URI:** `products://`

Returns a catalog of all products with pricing and availability.

## Implementation Patterns

### Dynamic URI Parsing
```rust
fn parse_uri(&self, uri: &str) -> Option<(String, String)> {
    // Parse "users://user-123" into ("users", "user-123")
    let parts: Vec<&str> = uri.split("://").collect();
    if parts.len() != 2 {
        return None;
    }
    Some((parts[0].to_string(), parts[1].to_string()))
}
```

### Resource Resolution
```rust
async fn read_resource(&self, uri: &str) -> Result<Vec<ResourceContent>, String> {
    let (resource_type, resource_id) = self.parse_uri(uri)?;
    
    match resource_type.as_str() {
        "users" => self.read_user(&resource_id).await,
        "products" => self.read_product(&resource_id).await,
        "documents" => self.read_document(&resource_id).await,
        "orders" => self.read_order(&resource_id).await,
        _ => Err(format!("Unknown resource type: {}", resource_type)),
    }
}
```

### Multiple Content Types
```rust
// Return both JSON data and human-readable text
Ok(vec![
    ResourceContent::blob(
        serde_json::to_string_pretty(&user_data).unwrap(),
        "application/json".to_string(),
    ),
    ResourceContent::text(format!(
        "User Profile: {}\nEmail: {}\nRole: {}\nStatus: {}",
        user.name, user.email, user.role, 
        if user.is_active { "Active" } else { "Inactive" }
    )),
])
```

## Error Handling

### Resource Not Found
```json
{
  "method": "resources/read",
  "params": {
    "uri": "users://nonexistent-user"
  }
}
```
**Response:** `"User not found: nonexistent-user"`

### Invalid URI Format
```json
{
  "method": "resources/read",
  "params": {
    "uri": "invalid-format"
  }
}
```
**Response:** `"Invalid URI format: invalid-format"`

## Real-world Applications

### 1. **User Management Systems**
Access user profiles, permissions, and activity data.

### 2. **E-commerce Platforms**
Retrieve product catalogs, inventory, and order information.

### 3. **Document Management**
Access documents, metadata, and version history.

### 4. **Order Processing**
Track orders, shipping status, and customer information.

### 5. **API Documentation**
Dynamic access to API endpoints, schemas, and examples.

## Comparison: Static vs Dynamic

### Static Resources (Limited)
```
file://config.json          - Only one config file
data://user-profile         - Only one user profile
system://status             - Only system status
```

### Dynamic Resources (Scalable)
```
users://user-123            - Any user by ID
products://prod-456          - Any product by ID
documents://doc-789          - Any document by ID
orders://order-2024-001      - Any order by ID
```

## Testing

```bash
# Start the server
cargo run -p dynamic-resource-server &

# List available resources
curl -X POST http://127.0.0.1:8048/mcp \
  -H "Content-Type: application/json" \
  -d '{"method": "resources/list"}'

# Get specific user
curl -X POST http://127.0.0.1:8048/mcp \
  -H "Content-Type: application/json" \
  -d '{"method": "resources/read", "params": {"uri": "users://user-001"}}'

# Get product information
curl -X POST http://127.0.0.1:8048/mcp \
  -H "Content-Type: application/json" \
  -d '{"method": "resources/read", "params": {"uri": "products://prod-0025"}}'

# Get order details
curl -X POST http://127.0.0.1:8048/mcp \
  -H "Content-Type: application/json" \
  -d '{"method": "resources/read", "params": {"uri": "orders://order-2024-0010"}}'
```

## Architecture

```
┌─────────────────────┐    ┌──────────────────────┐    ┌─────────────────────┐
│   MCP Client        │────│  Dynamic Resource    │────│  Data Stores        │
│                     │    │  Handler             │    │                     │
│ - Resource Lists    │    │ - URI Parsing        │    │ - Users Map         │
│ - Resource Reads    │    │ - Entity Resolution  │    │ - Products Map      │
│ - Dynamic URIs      │    │ - Content Generation │    │ - Documents Map     │
│ - Error Handling    │    │ - Multiple Formats   │    │ - Orders Map        │
└─────────────────────┘    └──────────────────────┘    └─────────────────────┘
```

This example demonstrates how MCP resources should work in real applications - with dynamic URIs that can access any entity by identifier, just like REST APIs or database queries. Much more practical than static file paths!