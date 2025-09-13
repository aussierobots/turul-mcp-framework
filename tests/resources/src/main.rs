//! MCP Resources Example
//!
//! This example demonstrates MCP Resources specification compliance
//! using derive macros and proper resource implementation patterns.

use mcp_resources_tests::{
    UserProfileResource, AppConfigResource, LogFilesResource,
    FileUserResource, UserAvatarResource, BinaryDataResource
};
use turul_mcp_server::prelude::*;
use tracing::info;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    info!("🔧 MCP Resources Specification Example");
    
    // Create resource instances demonstrating different patterns
    let user_resource = UserProfileResource::new("demo_user").with_preferences();
    let config_resource = AppConfigResource::new("database", "production");
    let log_resource = LogFilesResource::new("application").with_lines(10);
    
    info!("📊 Testing Resource Definition Traits:");
    info!("  - User Resource: {} ({})", user_resource.name(), user_resource.uri());
    info!("  - Config Resource: {} ({})", config_resource.name(), config_resource.uri());
    info!("  - Log Resource: {} ({})", log_resource.name(), log_resource.uri());
    
    // Test resource content generation
    info!("📄 Testing Resource Content Generation:");
    
    let user_content = user_resource.read(None).await?;
    info!("✅ User resource generated {} content items", user_content.len());
    
    let config_content = config_resource.read(None).await?;
    info!("✅ Config resource generated {} content items", config_content.len());
    
    let log_content = log_resource.read(None).await?;
    info!("✅ Log resource generated {} content items", log_content.len());
    
    // Test business logic methods
    info!("🔍 Testing Business Logic Methods:");
    
    let profile_data = user_resource.fetch_profile_data().await?;
    info!("✅ User profile business logic: {} content items", profile_data.len());
    
    let config_data = config_resource.fetch_config_data().await?;
    info!("✅ Config business logic: {} content items", config_data.len());
    
    let log_data = log_resource.fetch_log_data().await?;
    info!("✅ Log business logic: {} content items", log_data.len());
    
    // Test new URI template resources
    info!("🚀 Testing New URI Template Resources:");
    
    // Test FileUserResource with URI template "file:///user/{user_id}.json"
    let file_user_123 = FileUserResource::new("123");
    let file_user_456 = FileUserResource::new("456");
    
    let user_123_data = file_user_123.fetch_user_data().await?;
    let user_456_data = file_user_456.fetch_user_data().await?;
    
    info!("✅ FileUserResource(123): {} TextResourceContent items", user_123_data.len());
    info!("✅ FileUserResource(456): {} TextResourceContent items", user_456_data.len());
    
    // Test UserAvatarResource with BlobResourceContent
    let avatar_123 = UserAvatarResource::new("123");
    let avatar_456 = UserAvatarResource::new("456");
    
    let avatar_123_data = avatar_123.fetch_avatar_data().await?;
    let avatar_456_data = avatar_456.fetch_avatar_data().await?;
    
    info!("✅ UserAvatarResource(123): {} BlobResourceContent items", avatar_123_data.len());
    info!("✅ UserAvatarResource(456): {} BlobResourceContent items", avatar_456_data.len());
    
    // Test BinaryDataResource with various formats
    let pdf_resource = BinaryDataResource::new("document", "pdf");
    let zip_resource = BinaryDataResource::new("archive", "zip");
    
    let pdf_data = pdf_resource.fetch_binary_data().await?;
    let zip_data = zip_resource.fetch_binary_data().await?;
    
    info!("✅ BinaryDataResource(pdf): {} items", pdf_data.len());
    info!("✅ BinaryDataResource(zip): {} items", zip_data.len());
    
    info!("📊 Demonstrating TextResourceContent vs BlobResourceContent:");
    
    // Show TextResourceContent example
    match &user_123_data[0] {
        ResourceContent::Text(text_content) => {
            info!("📄 TextResourceContent example:");
            info!("   URI: {}", text_content.uri);
            info!("   MIME: {:?}", text_content.mime_type);
            info!("   Text length: {} characters", text_content.text.len());
        }
        ResourceContent::Blob(_) => info!("❌ Expected Text content"),
    }
    
    // Show BlobResourceContent example
    match &avatar_123_data[0] {
        ResourceContent::Blob(blob_content) => {
            info!("📁 BlobResourceContent example:");
            info!("   URI: {}", blob_content.uri);
            info!("   MIME: {:?}", blob_content.mime_type);
            info!("   Base64 length: {} characters", blob_content.blob.len());
        }
        ResourceContent::Text(_) => info!("❌ Expected Blob content"),
    }
    
    // Build MCP server with all resources
    let _server = McpServer::builder()
        .name("MCP Resources Test Server")
        .version("1.0.0")
        .resource(user_resource)
        .resource(config_resource)
        .resource(log_resource)
        .resource(file_user_123)
        .resource(avatar_123)
        .resource(pdf_resource)
        .build()?;
    
    info!("🎉 MCP Resources Example completed successfully!");
    info!("✨ All resource patterns working: Derive Macros ✅ | Business Logic ✅ | URI Templates ✅ | Text/Blob Content ✅");
    
    Ok(())
}