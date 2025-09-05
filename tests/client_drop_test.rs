//! Integration test to verify automatic DELETE request on MCP client drop
//! 
//! This script demonstrates that when an MCP client is dropped without
//! explicit disconnect(), it automatically sends a DELETE request to
//! clean up the session on the server.

use std::time::Duration;
use tokio::time::sleep;
use turul_mcp_client::{McpClient, ClientConfig};
use turul_mcp_client::transport::HttpTransport;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    let server_url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "http://127.0.0.1:52950/mcp".to_string());
    
    println!("🧪 Testing automatic DELETE on MCP client drop");
    println!("Server URL: {}", server_url);
    println!();
    
    // Test 1: Explicit disconnect (should send DELETE)
    println!("Test 1: Explicit disconnect");
    {
        let transport = Box::new(HttpTransport::new(&server_url)?);
        let config = ClientConfig::default();
        let client = McpClient::new(transport, config);
        
        client.connect().await?;
        let session_info = client.session_info().await;
        let session_id = session_info.session_id.unwrap_or_else(|| "no-session".to_string());
        println!("  Created session: {}", session_id);
        
        // Explicit disconnect - should send DELETE
        client.disconnect().await?;
        println!("  ✅ Explicit disconnect completed");
    }
    
    println!();
    
    // Test 2: Client drop (should auto-send DELETE)
    println!("Test 2: Client drop (automatic cleanup)");
    let session_id = {
        let transport = Box::new(HttpTransport::new(&server_url)?);
        let config = ClientConfig::default();
        let client = McpClient::new(transport, config);
        
        client.connect().await?;
        let session_info = client.session_info().await;
        let session_id = session_info.session_id.unwrap_or_else(|| "no-session".to_string());
        println!("  Created session: {}", session_id);
        
        // Client will be dropped here - should trigger automatic DELETE
        session_id
    };
    
    // Give the background task time to complete
    sleep(Duration::from_millis(100)).await;
    println!("  ✅ Client dropped - automatic DELETE should have been sent");
    println!("  Session ID that was cleaned up: {}", session_id);
    
    println!();
    println!("🎉 Test completed!");
    println!("Check the server logs to verify DELETE requests were received.");
    
    Ok(())
}