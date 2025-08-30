use reqwest;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list"
    });
    
    println!("Sending request: {}", request);
    
    let response = client
        .post("http://127.0.0.1:9000/lambda-url/lambda-turul-mcp-server")
        .header("Content-Type", "application/json")
        .header("mcp-session-id", "test-session")
        .json(&request)
        .send()
        .await?;
    
    println!("Status: {}", response.status());
    println!("Headers: {:?}", response.headers());
    
    let text = response.text().await?;
    println!("Response: {}", text);
    
    Ok(())
}
