//! Implementation of #[derive(McpNotification)] macro

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Result};

use crate::utils::extract_string_attribute;

pub fn derive_mcp_notification_impl(input: DeriveInput) -> Result<TokenStream> {
    let struct_name = &input.ident;

    // Extract struct-level attributes
    let method = extract_string_attribute(&input.attrs, "method")
        .ok_or_else(|| syn::Error::new_spanned(&input, "McpNotification derive requires #[notification(method = \"...\")] attribute"))?;
    
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
                    Err(e) => Err(mcp_protocol::McpError::notification(&e)),
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
    fn test_simple_notification() {
        let input: DeriveInput = parse_quote! {
            #[notification(method = "notifications/custom/alert")]
            struct AlertNotification {
                message: String,
                severity: String,
            }
        };

        let result = derive_mcp_notification_impl(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_high_priority_notification() {
        let input: DeriveInput = parse_quote! {
            #[notification(method = "notifications/system/critical", priority = "10", requires_ack = "true")]
            struct CriticalNotification {
                error_code: u32,
                description: String,
            }
        };

        let result = derive_mcp_notification_impl(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_batch_notification() {
        let input: DeriveInput = parse_quote! {
            #[notification(method = "notifications/data/batch", can_batch = "true")]
            struct BatchNotification {
                items: Vec<String>,
            }
        };

        let result = derive_mcp_notification_impl(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_missing_method() {
        let input: DeriveInput = parse_quote! {
            struct GenericNotification {
                data: String,
            }
        };

        let result = derive_mcp_notification_impl(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_minimal_notification() {
        let input: DeriveInput = parse_quote! {
            #[notification(method = "notifications/test")]
            struct TestNotification;
        };

        let result = derive_mcp_notification_impl(input);
        assert!(result.is_ok());
    }
}