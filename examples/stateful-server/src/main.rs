//! # Session-Aware Stateful Server Example
//!
//! This example demonstrates session-based state management in an MCP server.
//! It shows how to maintain persistent state across multiple requests within
//! the same session and handle session lifecycle events.

use std::collections::HashMap;

use async_trait::async_trait;
use mcp_server::{McpServer, McpTool, SessionContext};
use mcp_protocol::{ToolSchema, ToolResult, schema::JsonSchema, McpError, McpResult};
use serde_json::{Value, json};

/// Shopping cart tool that maintains state across requests
struct ShoppingCartTool;

#[async_trait]
impl McpTool for ShoppingCartTool {
    fn name(&self) -> &str {
        "shopping_cart"
    }

    fn description(&self) -> &str {
        "Manage a shopping cart with persistent state across requests"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("action".to_string(), JsonSchema::string_enum(vec![
                    "add".to_string(), "remove".to_string(), "list".to_string(), "clear".to_string()
                ]).with_description("Cart action to perform")),
                ("item".to_string(), JsonSchema::string()
                    .with_description("Item name (required for add/remove)")),
                ("quantity".to_string(), JsonSchema::integer()
                    .with_description("Item quantity (default: 1)")),
                ("price".to_string(), JsonSchema::number()
                    .with_description("Item price (required for add)")),
            ]))
            .with_required(vec!["action".to_string()])
    }

    async fn call(&self, args: Value, session: Option<SessionContext>) -> McpResult<Vec<ToolResult>> {
        let action = args.get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("action"))?;

        let session = session.ok_or_else(|| McpError::SessionError("This tool requires session context".to_string()))?;

        // Get or create cart state for this session
        let mut cart_items: HashMap<String, (i64, f64)> = session.get_typed_state("cart_items")
            .unwrap_or_default();
        
        let result = match action {
            "add" => {
                let item = args.get("item")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::missing_param("item"))?;
                let quantity = args.get("quantity")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(1);
                let price = args.get("price")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| McpError::missing_param("price"))?;

                if quantity <= 0 {
                    return Err(McpError::param_out_of_range("quantity", &quantity.to_string(), "must be positive"));
                }
                if price < 0.0 {
                    return Err(McpError::param_out_of_range("price", &price.to_string(), "cannot be negative"));
                }

                // Add or update item
                let (existing_qty, existing_price) = cart_items.get(item).cloned().unwrap_or((0, price));
                cart_items.insert(item.to_string(), (existing_qty + quantity, existing_price));

                session.set_typed_state("cart_items", &cart_items).unwrap();
                
                // Send progress notification
                session.notify_progress(format!("cart_item_{}", item), 1);

                json!({
                    "action": "add",
                    "item": item,
                    "quantity": quantity,
                    "price": price,
                    "total_quantity": cart_items.get(item).map(|(qty, _)| *qty).unwrap_or(0),
                    "message": format!("Added {} {} to cart at ${:.2} each", quantity, item, price)
                })
            }
            "remove" => {
                let item = args.get("item")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::missing_param("item"))?;
                let quantity = args.get("quantity")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(1);

                if let Some((existing_qty, price)) = cart_items.get(item).cloned() {
                    let new_qty = existing_qty - quantity;
                    if new_qty <= 0 {
                        cart_items.remove(item);
                        session.notify_progress(format!("cart_remove_{}", item), 1);
                        json!({
                            "action": "remove",
                            "item": item,
                            "quantity": existing_qty,
                            "removed_completely": true,
                            "message": format!("Removed all {} from cart", item)
                        })
                    } else {
                        cart_items.insert(item.to_string(), (new_qty, price));
                        session.notify_progress(format!("cart_update_{}", item), 1);
                        json!({
                            "action": "remove",
                            "item": item,
                            "quantity": quantity,
                            "remaining_quantity": new_qty,
                            "message": format!("Removed {} {}, {} remaining", quantity, item, new_qty)
                        })
                    }
                } else {
                    return Err(McpError::tool_execution(&format!("Item '{}' not found in cart", item)));
                }
            }
            "list" => {
                let items: Vec<Value> = cart_items.iter().map(|(name, (qty, price))| {
                    json!({
                        "name": name,
                        "quantity": qty,
                        "price": price,
                        "subtotal": *qty as f64 * price
                    })
                }).collect();

                let total: f64 = cart_items.values()
                    .map(|(qty, price)| *qty as f64 * price)
                    .sum();

                json!({
                    "action": "list",
                    "items": items,
                    "total_items": cart_items.len(),
                    "total_quantity": cart_items.values().map(|(qty, _)| qty).sum::<i64>(),
                    "total_price": total,
                    "cart_empty": cart_items.is_empty()
                })
            }
            "clear" => {
                let cleared_items = cart_items.len();
                cart_items.clear();
                session.set_typed_state("cart_items", &cart_items).unwrap();
                session.notify_progress("cart_clear", 1);

                json!({
                    "action": "clear",
                    "cleared_items": cleared_items,
                    "message": "Shopping cart cleared"
                })
            }
            _ => return Err(McpError::invalid_param_type("action", "add|remove|list|clear", action))
        };

        // Update session state
        session.set_typed_state("cart_items", &cart_items).unwrap();

        Ok(vec![ToolResult::text(result.to_string())])
    }
}

/// User preferences tool that persists settings across sessions
struct UserPreferencesTool;

#[async_trait]
impl McpTool for UserPreferencesTool {
    fn name(&self) -> &str {
        "user_preferences"
    }

    fn description(&self) -> &str {
        "Manage user preferences with session persistence"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema::object()
            .with_properties(HashMap::from([
                ("action".to_string(), JsonSchema::string_enum(vec![
                    "set".to_string(), "get".to_string(), "list".to_string(), "reset".to_string()
                ]).with_description("Preference action to perform")),
                ("key".to_string(), JsonSchema::string()
                    .with_description("Preference key (required for set/get)")),
                ("value".to_string(), JsonSchema::object()
                    .with_description("Preference value (required for set)")),
            ]))
            .with_required(vec!["action".to_string()])
    }

    async fn call(&self, args: Value, session: Option<SessionContext>) -> McpResult<Vec<ToolResult>> {
        let action = args.get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::missing_param("action"))?;

        let session = session.ok_or_else(|| McpError::SessionError("This tool requires session context".to_string()))?;

        // Get or create preferences state
        let mut preferences: HashMap<String, Value> = session.get_typed_state("user_preferences")
            .unwrap_or_default();

        let result = match action {
            "set" => {
                let key = args.get("key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::missing_param("key"))?;
                let value = args.get("value")
                    .ok_or_else(|| McpError::missing_param("value"))?;

                preferences.insert(key.to_string(), value.clone());
                session.set_typed_state("user_preferences", &preferences).unwrap();

                json!({
                    "action": "set",
                    "key": key,
                    "value": value,
                    "message": format!("Set preference '{}' to {}", key, value)
                })
            }
            "get" => {
                let key = args.get("key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::missing_param("key"))?;

                if let Some(value) = preferences.get(key) {
                    json!({
                        "action": "get",
                        "key": key,
                        "value": value,
                        "found": true
                    })
                } else {
                    json!({
                        "action": "get",
                        "key": key,
                        "value": null,
                        "found": false,
                        "message": format!("Preference '{}' not found", key)
                    })
                }
            }
            "list" => {
                json!({
                    "action": "list",
                    "preferences": preferences,
                    "count": preferences.len()
                })
            }
            "reset" => {
                let cleared_count = preferences.len();
                preferences.clear();
                session.set_typed_state("user_preferences", &preferences).unwrap();

                json!({
                    "action": "reset",
                    "cleared_count": cleared_count,
                    "message": "All preferences cleared"
                })
            }
            _ => return Err(McpError::invalid_param_type("action", "add|remove|list|clear", action))
        };

        Ok(vec![ToolResult::text(result.to_string())])
    }
}

/// Session information tool
struct SessionInfoTool;

#[async_trait]
impl McpTool for SessionInfoTool {
    fn name(&self) -> &str {
        "session_info"
    }

    fn description(&self) -> &str {
        "Get information about the current session"
    }

    fn input_schema(&self) -> ToolSchema {
        ToolSchema::object()
    }

    async fn call(&self, _args: Value, session: Option<SessionContext>) -> McpResult<Vec<ToolResult>> {
        if let Some(session) = session {
            let session_id = &session.session_id;
            let is_initialized = (session.is_initialized)();
            
            let info = json!({
                "session_id": session_id,
                "has_session": true,
                "is_initialized": is_initialized,
                "note": "Full state introspection not available in current API"
            });

            Ok(vec![ToolResult::text(info.to_string())])
        } else {
            Ok(vec![ToolResult::text(json!({
                "has_session": false,
                "message": "No session context available"
            }).to_string())])
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("Starting Session-Aware Stateful MCP Server");

    let server = McpServer::builder()
        .name("stateful-server")
        .version("1.0.0")
        .title("Stateful Server Example")
        .instructions("This server demonstrates session-based state management. State persists across requests within the same session and is automatically cleaned up when sessions expire.")
        .tool(ShoppingCartTool)
        .tool(UserPreferencesTool)
        .tool(SessionInfoTool)
        .bind_address("127.0.0.1:8006".parse()?)
        .sse(true)
        .build()?;

    println!("Stateful server running at: http://127.0.0.1:8006/mcp");
    println!("\\nAvailable tools:");
    println!("  - shopping_cart: Manage a persistent shopping cart (add, remove, list, clear)");
    println!("  - user_preferences: Manage user preferences (set, get, list, reset)");
    println!("  - session_info: Get current session information");
    println!("\\nExample usage:");
    println!("  1. Add items to cart: shopping_cart(action='add', item='apple', quantity=3, price=1.50)");
    println!("  2. List cart contents: shopping_cart(action='list')");
    println!("  3. Set preference: user_preferences(action='set', key='theme', value='dark')");
    println!("  4. Get session info: session_info()");
    println!("\\nNote: State persists within each session and is cleaned up automatically.");

    server.run().await?;
    Ok(())
}