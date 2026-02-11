//! Large Message Handling Tests
//!
//! Tests protocol robustness with large payloads, complex data structures,
//! and boundary conditions for production-grade MCP implementations

use mcp_e2e_shared::{McpTestClient, TestFixtures, TestServerManager};
use serde_json::json;
use serial_test::serial;
use tracing::{debug, info, warn};

#[tokio::test]
#[serial]
async fn test_large_json_parameter_handling() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_tools_server()
        .await
        .expect("Failed to start tools server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::tools_capabilities())
        .await
        .unwrap();

    // Create a large JSON object (approximately 1MB)
    let mut large_config = serde_json::Map::new();

    // Add many nested objects and arrays
    for i in 0..1000 {
        let section_name = format!("config_section_{}", i);
        let mut section = serde_json::Map::new();

        // Add various data types
        section.insert("id".to_string(), json!(i));
        section.insert(
            "name".to_string(),
            json!(format!("Configuration section {}", i)),
        );
        section.insert("enabled".to_string(), json!(i % 2 == 0));
        section.insert("priority".to_string(), json!(i as f64 / 10.0));

        // Add nested arrays
        let mut items = Vec::new();
        for j in 0..10 {
            items.push(json!({
                "item_id": j,
                "value": format!("item_{}_{}", i, j),
                "metadata": {
                    "created": "2025-09-12T19:00:00Z",
                    "tags": ["test", "large", "data"],
                    "nested": {
                        "level": 3,
                        "content": format!("Deep nested content for item {} in section {}", j, i)
                    }
                }
            }));
        }
        section.insert("items".to_string(), json!(items));

        large_config.insert(section_name, json!(section));
    }

    let large_payload = json!(large_config);
    let payload_size = serde_json::to_string(&large_payload).unwrap().len();
    info!(
        "🔍 Testing large payload: {} bytes ({:.1} MB)",
        payload_size,
        payload_size as f64 / 1024.0 / 1024.0
    );

    // Test with data_transformer tool that can handle large JSON objects
    let start_time = std::time::Instant::now();

    let result = client
        .call_tool(
            "data_transformer",
            json!({
                "operation": "count_keys",
                "data": large_payload
            }),
        )
        .await;

    let duration = start_time.elapsed();
    info!("⏱️ Large message processing took: {:?}", duration);

    match result {
        Ok(response) => {
            if let Some(result_obj) = TestFixtures::extract_tool_result_object(&response) {
                let key_count = result_obj.get("key_count").unwrap().as_i64().unwrap();
                info!(
                    "✅ Large payload processed successfully: {} keys found",
                    key_count
                );
                assert!(
                    key_count > 1000,
                    "Should have processed many keys from large payload"
                );

                // Performance check: should complete within reasonable time
                assert!(
                    duration.as_secs() < 30,
                    "Large payload processing took too long: {:?}",
                    duration
                );
            } else {
                warn!("⚠️  Tool handled large payload but returned non-standard result format");
            }
        }
        Err(e) => {
            // Large payloads might be rejected by the server - that's also valid
            info!("ℹ️  Large payload rejected (acceptable): {:?}", e);
        }
    }
}

#[tokio::test]
#[serial]
async fn test_very_long_string_parameters() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_tools_server()
        .await
        .expect("Failed to start tools server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::tools_capabilities())
        .await
        .unwrap();

    // Create a very long string (1MB of text)
    let long_text = "A".repeat(1024 * 1024); // 1MB of A's
    info!(
        "🔍 Testing very long string: {} characters",
        long_text.len()
    );

    let start_time = std::time::Instant::now();

    let result = client
        .call_tool(
            "string_processor",
            json!({
                "operation": "length",
                "text": long_text
            }),
        )
        .await;

    let duration = start_time.elapsed();
    info!("⏱️ Long string processing took: {:?}", duration);

    match result {
        Ok(response) => {
            if let Some(result_obj) = TestFixtures::extract_tool_result_object(&response) {
                let text_length = result_obj.get("length").unwrap().as_i64().unwrap();
                info!(
                    "✅ Long string processed successfully: {} characters",
                    text_length
                );
                assert_eq!(text_length, 1024 * 1024, "Length should match input string");

                // Performance check
                assert!(
                    duration.as_secs() < 10,
                    "Long string processing took too long: {:?}",
                    duration
                );
            } else {
                warn!("⚠️  Tool handled long string but returned unexpected format");
            }
        }
        Err(e) => {
            info!(
                "ℹ️  Long string rejected by server (acceptable for memory protection): {:?}",
                e
            );
        }
    }
}

#[tokio::test]
#[serial]
async fn test_deep_nested_json_structures() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_tools_server()
        .await
        .expect("Failed to start tools server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::tools_capabilities())
        .await
        .unwrap();

    // Create deeply nested JSON structure (100 levels deep)
    fn create_deep_nested(depth: i32) -> serde_json::Value {
        if depth <= 0 {
            json!({ "value": "deepest_level", "depth": 0 })
        } else {
            json!({
                "level": depth,
                "message": format!("Level {} of deep nesting", depth),
                "nested": create_deep_nested(depth - 1),
                "metadata": {
                    "created_at": "2025-09-12T19:00:00Z",
                    "level_info": format!("This is nesting level {}", depth)
                }
            })
        }
    }

    let deep_structure = create_deep_nested(100);
    info!("🔍 Testing deeply nested structure: 100 levels deep");

    let start_time = std::time::Instant::now();

    let result = client
        .call_tool(
            "data_transformer",
            json!({
                "operation": "max_depth",
                "data": deep_structure
            }),
        )
        .await;

    let duration = start_time.elapsed();
    info!("⏱️ Deep nesting processing took: {:?}", duration);

    match result {
        Ok(response) => {
            if let Some(result_obj) = TestFixtures::extract_tool_result_object(&response) {
                let max_depth = result_obj.get("max_depth").unwrap().as_i64().unwrap();
                info!(
                    "✅ Deep structure processed successfully: {} levels deep",
                    max_depth
                );
                assert!(max_depth >= 50, "Should detect significant nesting depth");

                // Performance check
                assert!(
                    duration.as_secs() < 5,
                    "Deep structure processing took too long: {:?}",
                    duration
                );
            } else {
                warn!("⚠️  Tool handled deep structure but returned unexpected format");
            }
        }
        Err(e) => {
            info!(
                "ℹ️  Deep structure rejected (acceptable for stack protection): {:?}",
                e
            );
        }
    }
}

#[tokio::test]
#[serial]
async fn test_large_array_parameters() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_tools_server()
        .await
        .expect("Failed to start tools server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::tools_capabilities())
        .await
        .unwrap();

    // Create a large array with complex objects
    let mut large_array = Vec::new();
    for i in 0..10000 {
        large_array.push(json!({
            "id": i,
            "name": format!("Item {}", i),
            "category": match i % 5 {
                0 => "electronics",
                1 => "books",
                2 => "clothing",
                3 => "home",
                _ => "other"
            },
            "price": (i as f64) * 1.99,
            "tags": vec![
                format!("tag_{}", i % 10),
                format!("category_{}", i % 5),
                "bulk_test".to_string()
            ],
            "properties": {
                "weight": i % 1000,
                "color": if i % 2 == 0 { "blue" } else { "red" },
                "available": i % 3 != 0
            }
        }));
    }

    let array_payload = json!(large_array);
    let payload_size = serde_json::to_string(&array_payload).unwrap().len();
    info!(
        "🔍 Testing large array: {} items, {} bytes",
        large_array.len(),
        payload_size
    );

    let start_time = std::time::Instant::now();

    let result = client
        .call_tool(
            "data_transformer",
            json!({
                "operation": "count_items",
                "data": array_payload
            }),
        )
        .await;

    let duration = start_time.elapsed();
    info!("⏱️ Large array processing took: {:?}", duration);

    match result {
        Ok(response) => {
            if let Some(result_obj) = TestFixtures::extract_tool_result_object(&response) {
                let item_count = result_obj.get("item_count").unwrap().as_i64().unwrap();
                info!(
                    "✅ Large array processed successfully: {} items",
                    item_count
                );
                assert_eq!(item_count, 10000, "Should count all array items");

                // Performance check
                assert!(
                    duration.as_secs() < 15,
                    "Large array processing took too long: {:?}",
                    duration
                );
            } else {
                warn!("⚠️  Tool handled large array but returned unexpected format");
            }
        }
        Err(e) => {
            info!(
                "ℹ️  Large array rejected (acceptable for memory protection): {:?}",
                e
            );
        }
    }
}

#[tokio::test]
#[serial]
async fn test_unicode_and_special_characters() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_tools_server()
        .await
        .expect("Failed to start tools server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::tools_capabilities())
        .await
        .unwrap();

    // Create text with various Unicode characters and special cases
    let complex_text = format!(
        "{}{}{}{}{}{}{}",
        "Basic ASCII: Hello World! ",
        "Accented: café, naïve, résumé ",
        "Symbols: ★☆♠♣♥♦◆◇ ",
        "Math: ∀∃∂∇∞∑∏∫ ",
        "Emoji: 🚀🔧⚡🎯📊✅❌ ",
        "CJK: 你好世界 こんにちは 안녕하세요 ",
        "Complex: \u{1F468}\u{200D}\u{1F4BB} \u{1F91D}\u{1F3FB} \u{0048}\u{0065}\u{006C}\u{006C}\u{006F}"
    );

    info!(
        "🔍 Testing Unicode text: {} characters",
        complex_text.chars().count()
    );

    let result = client
        .call_tool(
            "string_processor",
            json!({
                "operation": "analyze",
                "text": complex_text
            }),
        )
        .await;

    match result {
        Ok(response) => {
            if let Some(result_obj) = TestFixtures::extract_tool_result_object(&response) {
                info!("✅ Unicode text processed successfully");
                debug!("Unicode analysis result: {:?}", result_obj);

                // Verify the text was preserved correctly
                if let Some(processed_text) = result_obj.get("text") {
                    let processed_str = processed_text.as_str().unwrap();
                    assert!(
                        processed_str.contains("Hello World"),
                        "Basic ASCII should be preserved"
                    );
                    assert!(
                        processed_str.contains("café"),
                        "Accented characters should be preserved"
                    );
                    assert!(processed_str.contains("🚀"), "Emoji should be preserved");
                    assert!(
                        processed_str.contains("你好"),
                        "CJK characters should be preserved"
                    );
                }
            } else {
                warn!("⚠️  Tool handled Unicode but returned unexpected format");
            }
        }
        Err(e) => {
            warn!("⚠️  Unicode handling failed: {:?}", e);
        }
    }
}

#[tokio::test]
#[serial]
async fn test_message_size_limits_and_graceful_degradation() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_tools_server()
        .await
        .expect("Failed to start tools server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(TestFixtures::tools_capabilities())
        .await
        .unwrap();

    // Test progressively larger messages to find limits
    let test_sizes = vec![
        (1024, "1KB"),
        (10 * 1024, "10KB"),
        (100 * 1024, "100KB"),
        (1024 * 1024, "1MB"),
        (5 * 1024 * 1024, "5MB"), // This might be rejected
    ];

    let mut max_successful_size = 0;
    let mut failures = 0;

    for (size, label) in test_sizes {
        let large_data = "x".repeat(size);
        info!("🔍 Testing message size: {}", label);

        let result = client
            .call_tool(
                "string_processor",
                json!({
                    "operation": "length",
                    "text": large_data
                }),
            )
            .await;

        match result {
            Ok(response) => {
                if let Some(result_obj) = TestFixtures::extract_tool_result_object(&response) {
                    let length = result_obj.get("length").unwrap().as_i64().unwrap();
                    assert_eq!(length as usize, size);
                    max_successful_size = size;
                    info!("✅ {} message handled successfully", label);
                } else {
                    info!("ℹ️  {} message processed with non-standard response", label);
                    max_successful_size = size;
                }
            }
            Err(e) => {
                failures += 1;
                info!("ℹ️  {} message rejected (size limit): {:?}", label, e);

                // After a rejection, try to make sure the server is still responsive
                let recovery_test = client
                    .call_tool(
                        "calculator",
                        json!({
                            "operation": "add",
                            "a": 1.0,
                            "b": 1.0
                        }),
                    )
                    .await;

                match recovery_test {
                    Ok(_) => info!("✅ Server recovered from size limit rejection"),
                    Err(recovery_error) => {
                        warn!(
                            "⚠️  Server may have issues after size rejection: {:?}",
                            recovery_error
                        );
                        break; // Stop testing if server is unresponsive
                    }
                }
            }
        }
    }

    info!("📊 Message size testing complete:");
    info!(
        "   Max successful size: {} bytes ({:.1} MB)",
        max_successful_size,
        max_successful_size as f64 / 1024.0 / 1024.0
    );
    info!("   Failures (size limits): {}", failures);

    // Should handle at least 100KB successfully
    assert!(
        max_successful_size >= 100 * 1024,
        "Server should handle at least 100KB messages, max was: {}",
        max_successful_size
    );
}
