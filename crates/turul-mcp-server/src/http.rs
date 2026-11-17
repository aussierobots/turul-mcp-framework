//! HTTP transport support for MCP servers.
//!
//! Available when the `http` feature is enabled; re-exports everything from `turul_http_mcp_server`.

pub use turul_http_mcp_server::*;

/// HTTP-specific configuration and utilities.
pub mod config {
    use std::net::SocketAddr;

    /// HTTP server configuration.
    #[derive(Debug, Clone)]
    pub struct HttpConfig {
        /// Address to bind to
        pub bind_address: SocketAddr,
        /// MCP endpoint path
        pub mcp_path: String,
        /// Enable CORS
        pub enable_cors: bool,
        /// Enable SSE (if sse feature is available)
        pub enable_sse: bool,
        /// Maximum request body size
        pub max_body_size: usize,
    }

    impl Default for HttpConfig {
        fn default() -> Self {
            Self {
                bind_address: "127.0.0.1:8000".parse().unwrap(),
                mcp_path: "/mcp".to_string(),
                enable_cors: true,
                enable_sse: cfg!(feature = "sse"),
                max_body_size: 1024 * 1024, // 1MB
            }
        }
    }

    impl HttpConfig {
        /// Creates a new HTTP configuration with the default values.
        pub fn new() -> Self {
            Self::default()
        }

        /// Sets the bind address.
        pub fn bind_address(mut self, addr: SocketAddr) -> Self {
            self.bind_address = addr;
            self
        }

        /// Sets the MCP endpoint path.
        pub fn mcp_path(mut self, path: impl Into<String>) -> Self {
            self.mcp_path = path.into();
            self
        }

        /// Enables or disables CORS.
        pub fn cors(mut self, enable: bool) -> Self {
            self.enable_cors = enable;
            self
        }

        /// Enables or disables SSE.
        pub fn sse(mut self, enable: bool) -> Self {
            self.enable_sse = enable;
            self
        }

        /// Sets the maximum request body size.
        pub fn max_body_size(mut self, size: usize) -> Self {
            self.max_body_size = size;
            self
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::net::{IpAddr, Ipv4Addr};

        #[test]
        fn test_http_config_default() {
            let config = HttpConfig::default();
            assert_eq!(config.mcp_path, "/mcp");
            assert!(config.enable_cors);
            assert_eq!(config.max_body_size, 1024 * 1024);
        }

        #[test]
        fn test_http_config_builder() {
            let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 3000);
            let config = HttpConfig::new()
                .bind_address(addr)
                .mcp_path("/api/mcp")
                .cors(false)
                .max_body_size(2048);

            assert_eq!(config.bind_address, addr);
            assert_eq!(config.mcp_path, "/api/mcp");
            assert!(!config.enable_cors);
            assert_eq!(config.max_body_size, 2048);
        }
    }
}
