//! End-to-End Tests for MCP Elicitation Protocol
//!
//! Tests elicitation tools and structured data collection workflows.
//! Validates form generation, workflow management, and compliance features.

use mcp_elicitation_tests::test_utils::elicitation_capabilities;
use mcp_elicitation_tests::{debug, info, json, McpTestClient, TestFixtures, TestServerManager};

#[tokio::test]
async fn test_elicitation_onboarding_workflow() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_elicitation_server()
        .await
        .expect("Failed to start elicitation server");
    let mut client = McpTestClient::new(server.port());

    // Initialize with tool capabilities (elicitation uses tools)
    client
        .initialize_with_capabilities(elicitation_capabilities())
        .await
        .unwrap();

    // List available tools to find onboarding workflow tool
    let tools_result = client
        .make_request("tools/list", json!({}), 80)
        .await
        .expect("Failed to list tools");

    let tools = tools_result
        .get("result")
        .and_then(|r| r.as_object())
        .and_then(|obj| obj.get("tools"))
        .and_then(|tools| tools.as_array())
        .expect("Should have tools array");

    // Find onboarding workflow tool
    let onboarding_tool = tools
        .iter()
        .find(|tool| {
            tool.as_object()
                .and_then(|obj| obj.get("name"))
                .and_then(|name| name.as_str())
                .map(|name| name.contains("onboarding") || name.contains("workflow"))
                .unwrap_or(false)
        })
        .expect("Should have onboarding workflow tool");

    let tool_name = onboarding_tool
        .as_object()
        .and_then(|obj| obj.get("name"))
        .and_then(|name| name.as_str())
        .unwrap();

    info!("Found onboarding tool: {}", tool_name);

    // Start personal account onboarding workflow
    let workflow_result = client
        .call_tool(
            tool_name,
            json!({
                "workflow_type": "personal_account",
                "step_index": 0
            }),
        )
        .await
        .expect("Onboarding workflow should start");

    // Verify workflow started successfully
    assert!(
        workflow_result.contains_key("result"),
        "Should have workflow result"
    );

    if let Some(content) = TestFixtures::extract_tool_result_object(&workflow_result) {
        let content_str = content.to_string();
        assert!(
            content_str.contains("WORKFLOW STARTED") || content_str.contains("onboarding"),
            "Should indicate workflow started"
        );
        info!("✅ Personal onboarding workflow started successfully");
    }

    // Try business account workflow
    let business_workflow = client
        .call_tool(
            tool_name,
            json!({
                "workflow_type": "business_account",
                "step_index": 0
            }),
        )
        .await
        .expect("Business workflow should start");

    if let Some(content) = TestFixtures::extract_tool_result_object(&business_workflow) {
        let content_str = content.to_string();
        assert!(
            content_str.contains("WORKFLOW STARTED") || content_str.contains("onboarding"),
            "Should indicate business workflow started"
        );
        info!("✅ Business onboarding workflow started successfully");
    }
}

#[tokio::test]
async fn test_elicitation_compliance_forms() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_elicitation_server()
        .await
        .expect("Failed to start elicitation server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(elicitation_capabilities())
        .await
        .unwrap();

    // List tools to find compliance form tool
    let tools_result = client
        .make_request("tools/list", json!({}), 81)
        .await
        .expect("Failed to list tools");

    let tools = tools_result
        .get("result")
        .and_then(|r| r.as_object())
        .and_then(|obj| obj.get("tools"))
        .and_then(|tools| tools.as_array())
        .expect("Should have tools array");

    // Find compliance tool
    let compliance_tool = tools.iter().find(|tool| {
        tool.as_object()
            .and_then(|obj| obj.get("name"))
            .and_then(|name| name.as_str())
            .map(|name| name.contains("compliance"))
            .unwrap_or(false)
    });

    if let Some(tool) = compliance_tool {
        let tool_name = tool
            .as_object()
            .and_then(|obj| obj.get("name"))
            .and_then(|name| name.as_str())
            .unwrap();

        info!("Found compliance tool: {}", tool_name);

        // Test GDPR data request form
        let gdpr_result = client
            .call_tool(
                tool_name,
                json!({
                    "form_type": "gdpr_data_request"
                }),
            )
            .await
            .expect("GDPR form should work");

        if let Some(content) = TestFixtures::extract_tool_result_object(&gdpr_result) {
            let content_str = content.to_string();
            assert!(
                content_str.contains("GDPR") || content_str.contains("data request"),
                "Should indicate GDPR form"
            );
            info!("✅ GDPR compliance form generated successfully");
        }

        // Test CCPA opt-out form
        let ccpa_result = client
            .call_tool(
                tool_name,
                json!({
                    "form_type": "ccpa_opt_out"
                }),
            )
            .await
            .expect("CCPA form should work");

        if let Some(content) = TestFixtures::extract_tool_result_object(&ccpa_result) {
            let content_str = content.to_string();
            assert!(
                content_str.contains("CCPA") || content_str.contains("opt out"),
                "Should indicate CCPA form"
            );
            info!("✅ CCPA compliance form generated successfully");
        }
    } else {
        info!("ℹ️  No compliance tool found (may not be implemented)");
    }
}

#[tokio::test]
async fn test_elicitation_preference_collection() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_elicitation_server()
        .await
        .expect("Failed to start elicitation server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(elicitation_capabilities())
        .await
        .unwrap();

    // List tools to find preference collection tool
    let tools_result = client
        .make_request("tools/list", json!({}), 82)
        .await
        .expect("Failed to list tools");

    let tools = tools_result
        .get("result")
        .and_then(|r| r.as_object())
        .and_then(|obj| obj.get("tools"))
        .and_then(|tools| tools.as_array())
        .expect("Should have tools array");

    // Find preference tool
    let preference_tool = tools.iter().find(|tool| {
        tool.as_object()
            .and_then(|obj| obj.get("name"))
            .and_then(|name| name.as_str())
            .map(|name| name.contains("preference"))
            .unwrap_or(false)
    });

    if let Some(tool) = preference_tool {
        let tool_name = tool
            .as_object()
            .and_then(|obj| obj.get("name"))
            .and_then(|name| name.as_str())
            .unwrap();

        info!("Found preference tool: {}", tool_name);

        // Test notification preferences
        let notification_result = client
            .call_tool(
                tool_name,
                json!({
                    "preference_type": "notification_preferences"
                }),
            )
            .await
            .expect("Notification preferences should work");

        if let Some(content) = TestFixtures::extract_tool_result_object(&notification_result) {
            let content_str = content.to_string();
            assert!(
                content_str.contains("NOTIFICATION") || content_str.contains("preference"),
                "Should indicate notification preferences"
            );
            info!("✅ Notification preferences collection working");
        }

        // Test accessibility preferences
        let accessibility_result = client
            .call_tool(
                tool_name,
                json!({
                    "preference_type": "accessibility_preferences"
                }),
            )
            .await
            .expect("Accessibility preferences should work");

        if let Some(content) = TestFixtures::extract_tool_result_object(&accessibility_result) {
            let content_str = content.to_string();
            assert!(
                content_str.contains("ACCESSIBILITY") || content_str.contains("preference"),
                "Should indicate accessibility preferences"
            );
            info!("✅ Accessibility preferences collection working");
        }
    } else {
        info!("ℹ️  No preference collection tool found (may not be implemented)");
    }
}

#[tokio::test]
async fn test_elicitation_customer_surveys() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_elicitation_server()
        .await
        .expect("Failed to start elicitation server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(elicitation_capabilities())
        .await
        .unwrap();

    // List tools to find survey tool
    let tools_result = client
        .make_request("tools/list", json!({}), 83)
        .await
        .expect("Failed to list tools");

    let tools = tools_result
        .get("result")
        .and_then(|r| r.as_object())
        .and_then(|obj| obj.get("tools"))
        .and_then(|tools| tools.as_array())
        .expect("Should have tools array");

    // Find survey tool
    let survey_tool = tools.iter().find(|tool| {
        tool.as_object()
            .and_then(|obj| obj.get("name"))
            .and_then(|name| name.as_str())
            .map(|name| name.contains("survey") || name.contains("satisfaction"))
            .unwrap_or(false)
    });

    if let Some(tool) = survey_tool {
        let tool_name = tool
            .as_object()
            .and_then(|obj| obj.get("name"))
            .and_then(|name| name.as_str())
            .unwrap();

        info!("Found survey tool: {}", tool_name);

        // Test customer satisfaction survey
        let survey_result = client
            .call_tool(
                tool_name,
                json!({
                    "survey_type": "customer_satisfaction",
                    "customer_segment": "new_customer"
                }),
            )
            .await
            .expect("Customer survey should work");

        if let Some(content) = TestFixtures::extract_tool_result_object(&survey_result) {
            let content_str = content.to_string();
            assert!(
                content_str.contains("SURVEY") || content_str.contains("satisfaction"),
                "Should indicate customer survey"
            );
            info!("✅ Customer satisfaction survey working");
        }

        // Test different customer segments
        let segments = vec!["existing_customer", "premium_customer", "at_risk_customer"];
        for segment in segments {
            let segment_result = client
                .call_tool(
                    tool_name,
                    json!({
                        "survey_type": "customer_satisfaction",
                        "customer_segment": segment
                    }),
                )
                .await;

            match segment_result {
                Ok(response) => {
                    if let Some(content) = TestFixtures::extract_tool_result_object(&response) {
                        let content_str = content.to_string();
                        assert!(
                            content_str.contains(segment) || content_str.contains("SURVEY"),
                            "Should handle segment: {}",
                            segment
                        );
                        info!("✅ Survey for {} segment working", segment);
                    }
                }
                Err(e) => {
                    info!("ℹ️  Survey for {} segment failed: {:?}", segment, e);
                }
            }
        }
    } else {
        info!("ℹ️  No survey tool found (may not be implemented)");
    }
}

#[tokio::test]
async fn test_elicitation_data_validation() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_elicitation_server()
        .await
        .expect("Failed to start elicitation server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(elicitation_capabilities())
        .await
        .unwrap();

    // List tools to find validation tool
    let tools_result = client
        .make_request("tools/list", json!({}), 84)
        .await
        .expect("Failed to list tools");

    let tools = tools_result
        .get("result")
        .and_then(|r| r.as_object())
        .and_then(|obj| obj.get("tools"))
        .and_then(|tools| tools.as_array())
        .expect("Should have tools array");

    // Find validation tool
    let validation_tool = tools.iter().find(|tool| {
        tool.as_object()
            .and_then(|obj| obj.get("name"))
            .and_then(|name| name.as_str())
            .map(|name| name.contains("validation"))
            .unwrap_or(false)
    });

    if let Some(tool) = validation_tool {
        let tool_name = tool
            .as_object()
            .and_then(|obj| obj.get("name"))
            .and_then(|name| name.as_str())
            .unwrap();

        info!("Found validation tool: {}", tool_name);

        // Test different validation categories
        let validation_categories = vec![
            "field_validation",
            "business_rules",
            "security_policies",
            "compliance_checks",
        ];

        for category in validation_categories {
            let validation_result = client
                .call_tool(
                    tool_name,
                    json!({
                        "validation_category": category
                    }),
                )
                .await;

            match validation_result {
                Ok(response) => {
                    if let Some(content) = TestFixtures::extract_tool_result_object(&response) {
                        let content_str = content.to_string();
                        assert!(
                            content_str.contains("VALIDATION") || content_str.contains(category),
                            "Should handle validation category: {}",
                            category
                        );
                        info!("✅ {} validation working", category);
                    }
                }
                Err(e) => {
                    info!("ℹ️  {} validation failed: {:?}", category, e);
                }
            }
        }
    } else {
        info!("ℹ️  No validation tool found (may not be implemented)");
    }
}

#[tokio::test]
async fn test_elicitation_tool_schemas() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_elicitation_server()
        .await
        .expect("Failed to start elicitation server");
    let mut client = McpTestClient::new(server.port());

    client
        .initialize_with_capabilities(elicitation_capabilities())
        .await
        .unwrap();

    // List all tools and verify their schemas
    let tools_result = client
        .make_request("tools/list", json!({}), 85)
        .await
        .expect("Failed to list tools");

    let tools = tools_result
        .get("result")
        .and_then(|r| r.as_object())
        .and_then(|obj| obj.get("tools"))
        .and_then(|tools| tools.as_array())
        .expect("Should have tools array");

    info!("Found {} elicitation tools", tools.len());

    // Verify each tool has proper schema
    for (i, tool) in tools.iter().enumerate() {
        let tool_obj = tool.as_object().expect("Tool should be object");

        // Check required fields
        assert!(tool_obj.contains_key("name"), "Tool {} missing name", i);
        assert!(
            tool_obj.contains_key("description"),
            "Tool {} missing description",
            i
        );
        assert!(
            tool_obj.contains_key("inputSchema"),
            "Tool {} missing inputSchema",
            i
        );

        let name = tool_obj.get("name").unwrap().as_str().unwrap();
        let description = tool_obj.get("description").unwrap().as_str().unwrap();
        let input_schema = tool_obj.get("inputSchema").unwrap().as_object().unwrap();

        // Verify schema structure
        assert!(
            input_schema.contains_key("type"),
            "Tool {} schema missing type",
            name
        );
        assert_eq!(
            input_schema.get("type").unwrap().as_str().unwrap(),
            "object",
            "Tool {} schema should be object type",
            name
        );

        // Should have properties if it's an object schema
        if input_schema.contains_key("properties") {
            let properties = input_schema.get("properties").unwrap().as_object().unwrap();
            assert!(
                !properties.is_empty(),
                "Tool {} should have properties if specified",
                name
            );
        }

        info!("✅ Tool '{}' has valid schema: {}", name, description);
    }

    // Verify we have some elicitation-related tools
    let elicitation_tool_names: Vec<&str> = tools
        .iter()
        .filter_map(|tool| {
            tool.as_object()
                .and_then(|obj| obj.get("name"))
                .and_then(|name| name.as_str())
        })
        .filter(|name| {
            name.contains("onboarding")
                || name.contains("compliance")
                || name.contains("preference")
                || name.contains("survey")
                || name.contains("validation")
        })
        .collect();

    assert!(
        !elicitation_tool_names.is_empty(),
        "Should have at least one elicitation-related tool"
    );

    info!(
        "✅ Found {} elicitation tools: {:?}",
        elicitation_tool_names.len(),
        elicitation_tool_names
    );
}

#[tokio::test]
async fn test_elicitation_server_capabilities() {
    let _ = tracing_subscriber::fmt::try_init();

    let server = TestServerManager::start_elicitation_server()
        .await
        .expect("Failed to start elicitation server");
    let mut client = McpTestClient::new(server.port());

    // Initialize and check capabilities
    let init_result = client
        .initialize_with_capabilities(elicitation_capabilities())
        .await
        .unwrap();

    debug!("Initialize result: {:?}", init_result);

    // Check server capabilities
    if let Some(server_capabilities) = init_result.get("capabilities") {
        // Should have tools capability
        assert!(
            server_capabilities.get("tools").is_some(),
            "Elicitation server should advertise tools capability"
        );

        if let Some(tools_cap) = server_capabilities.get("tools") {
            info!("✅ Server advertises tools capabilities: {:?}", tools_cap);
        }

        // Check for elicitation capability if present
        if let Some(elicitation_cap) = server_capabilities.get("elicitation") {
            info!(
                "✅ Server advertises elicitation capabilities: {:?}",
                elicitation_cap
            );
        } else {
            info!("ℹ️  Server implements elicitation via tools (no separate capability)");
        }
    }

    // Verify tools are actually available
    let tools_result = client
        .make_request("tools/list", json!({}), 86)
        .await
        .expect("Tools should be available");

    assert!(
        tools_result.contains_key("result"),
        "Should have tools result"
    );
    info!("✅ Elicitation server tools are accessible");
}
