//! Implementation of #[derive(McpNotification)] macro

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Result};

use crate::utils::extract_string_attribute;

/// Auto-determine notification method from struct name (ZERO CONFIGURATION!)
/// Examples:
/// - `ProgressNotification` → `"notifications/progress"`
/// - `ResourcesListChangedNotification` → `"notifications/resources/list_changed"`
/// - `MessageNotification` → `"notifications/message"`
fn auto_determine_notification_method(struct_name: String) -> String {
    // Remove "Notification" suffix if present
    let base_name = if struct_name.ends_with("Notification") {
        &struct_name[..struct_name.len() - 12] // Remove "Notification"
    } else {
        &struct_name
    };
    
    // Convert CamelCase to snake_case and build method
    let snake_case = camel_to_snake_case(base_name);
    
    // Handle known patterns (MCP specification compliant)
    match snake_case.as_str() {
        "progress" => "notifications/progress".to_string(),
        "message" => "notifications/message".to_string(),
        "cancelled" => "notifications/cancelled".to_string(),
        "initialized" => "notifications/initialized".to_string(),
        "resource_updated" | "resources_updated" => "notifications/resources/updated".to_string(),
        "resources_list_changed" | "resources_changed" => "notifications/resources/list_changed".to_string(),
        "roots_list_changed" | "roots_changed" => "notifications/roots/list_changed".to_string(),
        "prompts_list_changed" | "prompts_changed" => "notifications/prompts/list_changed".to_string(),
        "tools_list_changed" | "tools_changed" => "notifications/tools/list_changed".to_string(),
        _ => {
            // For custom notifications, use notifications/{snake_case}
            format!("notifications/{}", snake_case)
        }
    }
}

/// Convert CamelCase to snake_case
fn camel_to_snake_case(input: &str) -> String {
    let mut result = String::new();
    for (i, ch) in input.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(ch.to_lowercase().next().unwrap());
    }
    result
}

pub fn derive_mcp_notification_impl(input: DeriveInput) -> Result<TokenStream> {
    let struct_name = &input.ident;

    // AUTO-DETERMINE method from struct name (ZERO CONFIGURATION!)
    let method = auto_determine_notification_method(struct_name.to_string());
    
    // Optional: Allow override with attribute (for edge cases)
    let method = extract_string_attribute(&input.attrs, "method")
        .unwrap_or(method);
    
    let priority = extract_string_attribute(&input.attrs, "priority")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);
    
    let requires_ack = extract_string_attribute(&input.attrs, "requires_ack")
        .map(|s| s.parse::<bool>().unwrap_or(false))
        .unwrap_or(false);

    let can_batch = extract_string_attribute(&input.attrs, "can_batch")
        .map(|s| s.parse::<bool>().unwrap_or(true))
        .unwrap_or(true);

    let expanded = quote! {
        #[automatically_derived]
        impl mcp_protocol::notifications::HasNotificationMetadata for #struct_name {
            fn method(&self) -> &str {
                #method
            }

            fn notification_type(&self) -> Option<&str> {
                // Extract type from method name (e.g., "notifications/resources/list_changed" -> "resources")
                let parts: Vec<&str> = #method.split('/').collect();
                if parts.len() >= 2 {
                    Some(parts[1])
                } else {
                    None
                }
            }

            fn requires_ack(&self) -> bool {
                #requires_ack
            }
        }

        #[automatically_derived]
        impl mcp_protocol::notifications::HasNotificationPayload for #struct_name {
            fn payload(&self) -> Option<&serde_json::Value> {
                use std::sync::LazyLock;
                static DEFAULT_PAYLOAD: LazyLock<serde_json::Value> = LazyLock::new(|| {
                    serde_json::json!({})
                });
                Some(&DEFAULT_PAYLOAD)
            }

            fn serialize_payload(&self) -> Result<String, String> {
                match self.payload() {
                    Some(data) => serde_json::to_string(data)
                        .map_err(|e| format!("Serialization error: {}", e)),
                    None => Ok("{}".to_string()),
                }
            }
        }

        #[automatically_derived]
        impl mcp_protocol::notifications::HasNotificationRules for #struct_name {
            fn priority(&self) -> u32 {
                #priority
            }

            fn can_batch(&self) -> bool {
                #can_batch
            }

            fn max_retries(&self) -> u32 {
                3 // Default retry count
            }

            fn should_deliver(&self) -> bool {
                true // Default: always deliver
            }
        }

        // NotificationDefinition is automatically implemented via blanket impl

        #[automatically_derived]
        #[async_trait::async_trait]
        impl mcp_server::McpNotification for #struct_name {
            async fn send(&self, payload: serde_json::Value) -> mcp_protocol::McpResult<mcp_server::notifications::DeliveryResult> {
                // Default implementation - this should be overridden by implementing send_impl
                match self.send_impl(payload).await {
                    Ok(result) => Ok(result),
                    Err(e) => Err(mcp_protocol::McpError::tool_execution(&e)),
                }
            }
        }

        impl #struct_name {
            /// Override this method to provide custom notification sending logic
            pub async fn send_impl(&self, payload: serde_json::Value) -> Result<mcp_server::notifications::DeliveryResult, String> {
                // Default: simulate successful delivery
                println!("Sending notification [{}]: {}", #method, payload);
                
                Ok(mcp_server::notifications::DeliveryResult {
                    status: mcp_server::notifications::DeliveryStatus::Sent,
                    attempts: 1,
                    error: None,
                    delivered_at: Some(
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs()
                    ),
                })
            }
        }
    };

    Ok(expanded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_zero_config_notification() {
        // ✅ ZERO CONFIGURATION - No method attribute needed!
        let input: DeriveInput = parse_quote! {
            #[derive(McpNotification)]
            struct ProgressNotification {
                message: String,
                severity: String,
            }
        };

        let result = derive_mcp_notification_impl(input);
        assert!(result.is_ok());
        
        // Framework auto-determines method as "notifications/progress"
    }
    
    #[test]
    fn test_resources_changed_notification() {
        // ✅ ZERO CONFIGURATION - Auto-determines "notifications/resources/list_changed"
        let input: DeriveInput = parse_quote! {
            #[derive(McpNotification)]
            struct ResourcesListChangedNotification;
        };

        let result = derive_mcp_notification_impl(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cancelled_notification() {
        let input: DeriveInput = parse_quote! {
            #[notification(priority = "10", requires_ack = "true")]
            struct CancelledNotification {
                request_id: String,
                reason: Option<String>,
            }
        };

        let result = derive_mcp_notification_impl(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_resource_updated_notification() {
        let input: DeriveInput = parse_quote! {
            #[notification(can_batch = "true")]
            struct ResourceUpdatedNotification {
                uri: String,
            }
        };

        let result = derive_mcp_notification_impl(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_initialized_notification() {
        let input: DeriveInput = parse_quote! {
            struct InitializedNotification {
                protocol_version: String,
            }
        };

        let result = derive_mcp_notification_impl(input);
        assert!(result.is_ok()); // Should succeed with auto-generated method "notifications/initialized"
    }

    #[test]
    fn test_minimal_notification() {
        let input: DeriveInput = parse_quote! {
            struct ProgressNotification {
                progress_token: String,
                progress: u64,
            }
        };

        let result = derive_mcp_notification_impl(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_zero_config_method_generation() {
        // Test zero-configuration method generation for different naming patterns
        let test_cases = vec![
            ("ProgressNotification", "notifications/progress"),
            ("ResourcesListChangedNotification", "notifications/resources/list_changed"),
            ("ToolsChangedNotification", "notifications/tools/list_changed"),
            ("InitializedNotification", "notifications/initialized"),
            ("CancelledNotification", "notifications/cancelled"),
        ];

        for (struct_name, expected_method) in test_cases {
            let method = auto_determine_notification_method(struct_name.to_string());
            assert_eq!(method, expected_method, "Failed for struct: {}", struct_name);
        }
    }

    #[test]
    fn test_notification_trait_implementations() {
        let input: DeriveInput = parse_quote! {
            struct TestNotification {
                data: String,
            }
        };

        let result = derive_mcp_notification_impl(input);
        assert!(result.is_ok());
        
        // Check that the generated code contains required trait implementations
        let code = result.unwrap().to_string();
        assert!(code.contains("HasNotificationMetadata"));
        assert!(code.contains("HasNotificationPayload"));
        assert!(code.contains("McpNotification"));
    }

    #[test]
    fn test_camel_to_snake_case_conversion() {
        let test_cases = vec![
            ("SimpleCase", "simple_case"),
            ("ComplexCamelCase", "complex_camel_case"),
            ("XMLParser", "x_m_l_parser"),
            ("IOBuffer", "i_o_buffer"),
            ("lowercase", "lowercase"),
        ];

        for (input, expected) in test_cases {
            let result = camel_to_snake_case(input);
            assert_eq!(result, expected, "Failed for input: {}", input);
        }
    }

    #[test]
    fn test_special_notification_types() {
        // Test standard MCP notification types get proper method names
        let test_cases = vec![
            ("ResourcesListChangedNotification", "notifications/resources/list_changed"),
            ("PromptsChangedNotification", "notifications/prompts/list_changed"),
            ("ToolsChangedNotification", "notifications/tools/list_changed"),
            ("RootsListChangedNotification", "notifications/roots/list_changed"),
        ];

        for (struct_name, expected_method) in test_cases {
            let method = auto_determine_notification_method(struct_name.to_string());
            assert_eq!(method, expected_method, "Failed for struct: {}", struct_name);
        }
    }
}