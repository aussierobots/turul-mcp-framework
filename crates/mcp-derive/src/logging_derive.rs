//! Implementation of #[derive(McpLogger)] macro

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Result};

use crate::utils::extract_string_attribute;

pub fn derive_mcp_logger_impl(input: DeriveInput) -> Result<TokenStream> {
    let struct_name = &input.ident;

    // Extract struct-level attributes
    let logger_name = extract_string_attribute(&input.attrs, "name")
        .unwrap_or_else(|| "default".to_string());
    
    let level = extract_string_attribute(&input.attrs, "level")
        .unwrap_or_else(|| "info".to_string());

    // Parse the logging level
    let logging_level = match level.as_str() {
        "debug" => quote! { mcp_protocol::logging::LoggingLevel::Debug },
        "info" => quote! { mcp_protocol::logging::LoggingLevel::Info },
        "notice" => quote! { mcp_protocol::logging::LoggingLevel::Notice },
        "warning" => quote! { mcp_protocol::logging::LoggingLevel::Warning },
        "error" => quote! { mcp_protocol::logging::LoggingLevel::Error },
        "critical" => quote! { mcp_protocol::logging::LoggingLevel::Critical },
        "alert" => quote! { mcp_protocol::logging::LoggingLevel::Alert },
        "emergency" => quote! { mcp_protocol::logging::LoggingLevel::Emergency },
        _ => quote! { mcp_protocol::logging::LoggingLevel::Info },
    };

    let expanded = quote! {
        #[automatically_derived]
        impl mcp_protocol::logging::HasLoggingMetadata for #struct_name {
            fn method(&self) -> &str {
                "notifications/message"
            }

            fn logger_name(&self) -> Option<&str> {
                Some(#logger_name)
            }
        }

        #[automatically_derived]
        impl mcp_protocol::logging::HasLogLevel for #struct_name {
            fn level(&self) -> mcp_protocol::logging::LoggingLevel {
                #logging_level
            }

            fn should_log(&self, message_level: mcp_protocol::logging::LoggingLevel) -> bool {
                message_level.should_log(self.level())
            }
        }

        #[automatically_derived]
        impl mcp_protocol::logging::HasLogFormat for #struct_name {
            fn data(&self) -> &serde_json::Value {
                use std::sync::LazyLock;
                static DEFAULT_DATA: LazyLock<serde_json::Value> = LazyLock::new(|| {
                    serde_json::json!({"message": "Default log message"})
                });
                &DEFAULT_DATA
            }

            fn format_message(&self) -> String {
                match self.data() {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Object(obj) => {
                        if let Some(msg) = obj.get("message") {
                            msg.as_str().unwrap_or("Invalid message").to_string()
                        } else {
                            serde_json::to_string(obj).unwrap_or_else(|_| "Invalid log data".to_string())
                        }
                    }
                    other => serde_json::to_string(other).unwrap_or_else(|_| "Invalid log data".to_string()),
                }
            }
        }

        #[automatically_derived]
        impl mcp_protocol::logging::HasLogTransport for #struct_name {
            fn should_deliver(&self, level: mcp_protocol::logging::LoggingLevel) -> bool {
                self.should_log(level)
            }

            fn batch_size(&self) -> Option<usize> {
                Some(100) // Default batch size
            }
        }

        // LoggerDefinition is automatically implemented via blanket impl

        #[automatically_derived]
        #[async_trait::async_trait]
        impl mcp_server::McpLogger for #struct_name {
            async fn log(&self, level: mcp_protocol::logging::LoggingLevel, data: serde_json::Value) -> mcp_protocol::McpResult<()> {
                // Default implementation - this should be overridden by implementing log_impl
                match self.log_impl(level, data).await {
                    Ok(_) => Ok(()),
                    Err(e) => Err(mcp_protocol::McpError::logging(&e)),
                }
            }

            async fn set_level(&self, request: mcp_protocol::logging::SetLevelRequest) -> mcp_protocol::McpResult<()> {
                // Default implementation - this should be overridden by implementing set_level_impl
                match self.set_level_impl(request).await {
                    Ok(_) => Ok(()),
                    Err(e) => Err(mcp_protocol::McpError::logging(&e)),
                }
            }
        }

        impl #struct_name {
            /// Override this method to provide custom logging logic
            pub async fn log_impl(&self, level: mcp_protocol::logging::LoggingLevel, data: serde_json::Value) -> Result<(), String> {
                // Default: print to stdout
                let formatted = match &data {
                    serde_json::Value::String(s) => s.clone(),
                    other => serde_json::to_string(other).unwrap_or_else(|_| "Invalid data".to_string()),
                };
                
                let level_str = format!("{:?}", level).to_uppercase();
                println!("[{}] [{}] {}", level_str, #logger_name, formatted);
                Ok(())
            }

            /// Override this method to provide custom level setting logic
            pub async fn set_level_impl(&self, _request: mcp_protocol::logging::SetLevelRequest) -> Result<(), String> {
                // Default: no-op (level is fixed at compile time for derived loggers)
                Ok(())
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
    fn test_simple_logger() {
        let input: DeriveInput = parse_quote! {
            #[logger(name = "app_logger", level = "info")]
            struct ApplicationLogger {
                format: String,
                output_file: Option<String>,
            }
        };

        let result = derive_mcp_logger_impl(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_debug_logger() {
        let input: DeriveInput = parse_quote! {
            #[logger(name = "debug_logger", level = "debug")]
            struct DebugLogger;
        };

        let result = derive_mcp_logger_impl(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_minimal_logger() {
        let input: DeriveInput = parse_quote! {
            struct SimpleLogger;
        };

        let result = derive_mcp_logger_impl(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_error_logger() {
        let input: DeriveInput = parse_quote! {
            #[logger(name = "error_logger", level = "error")]
            struct ErrorLogger {
                file_path: String,
            }
        };

        let result = derive_mcp_logger_impl(input);
        assert!(result.is_ok());
    }
}