//! # Origin Policy Server (DNS-rebinding protection + CORS)
//!
//! Origin validation is ON by default: a request whose `Origin` header is
//! present but neither loopback nor same-host is rejected with HTTP 403
//! before auth or dispatch. This stops DNS-rebinding — a malicious page on
//! `https://evil.example` rebinding its hostname to 127.0.0.1 so the
//! victim's browser posts to your local MCP server with the browser's
//! ambient network position.
//!
//! A browser app served from a *different* origin therefore needs a
//! conscious allowlist decision — that is this example:
//!
//! ```bash
//! # Default policy: same-origin or loopback only
//! cargo run -p origin-policy-server
//!
//! # Allow a browser app origin (repeatable flag)
//! cargo run -p origin-policy-server -- --allow-origin https://app.example.com
//!
//! # Behind an API gateway that enforces origin upstream
//! cargo run -p origin-policy-server -- --disable-origin-check
//! ```
//!
//! Origin validation and CORS are different layers that must agree:
//! - **Origin policy** is server-side rejection (403) — the security boundary.
//! - **CORS headers** are browser-side consent — without them the browser
//!   blocks the response from a cross-origin page even when the server
//!   would have answered.

use clap::Parser;
use turul_http_mcp_server::OriginPolicy;
use turul_mcp_derive::McpTool;
use turul_mcp_server::prelude::*;

#[derive(Parser)]
#[command(about = "MCP server demonstrating OriginPolicy (ADR DNS-rebinding protection)")]
struct Args {
    #[arg(long, default_value = "8643")]
    port: u16,

    /// Add an origin to the allowlist (repeatable). Implies AllowList policy.
    /// The literal value "null" admits `Origin: null` (sandboxed iframes).
    #[arg(long = "allow-origin")]
    allow_origins: Vec<String>,

    /// Disable origin validation entirely (origin enforced upstream).
    #[arg(long)]
    disable_origin_check: bool,
}

#[derive(McpTool, Clone, Default)]
#[tool(
    name = "whereami",
    description = "Echo which origin policy admitted this request"
)]
struct WhereAmITool {}

impl WhereAmITool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
        Ok("request admitted by the active origin policy".to_string())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let args = Args::parse();

    let (policy, label) = if args.disable_origin_check {
        (
            OriginPolicy::Disabled,
            "Disabled (origin enforced upstream)".to_string(),
        )
    } else if !args.allow_origins.is_empty() {
        let label = format!("AllowList({:?}) + same-origin/loopback", args.allow_origins);
        (OriginPolicy::AllowList(args.allow_origins.clone()), label)
    } else {
        (
            OriginPolicy::SameOriginOrLoopback,
            "SameOriginOrLoopback (default)".to_string(),
        )
    };

    let server = McpServer::builder()
        .name("origin-policy-server")
        .version("0.4.0")
        .title("Origin Policy Example")
        .instructions("Demonstrates Origin-header validation (DNS-rebinding protection).")
        .tool(WhereAmITool::default())
        .origin_policy(policy)
        .bind_address(format!("127.0.0.1:{}", args.port).parse()?)
        .build()?;

    tracing::info!("Origin policy: {label}");
    tracing::info!(
        "Server running at http://127.0.0.1:{}/mcp — probe the policy:",
        args.port
    );
    tracing::info!("  # no Origin header (curl, native clients) → 200");
    tracing::info!(
        "  curl -s -o /dev/null -w '%{{http_code}}\\n' -X POST http://127.0.0.1:{}/mcp -H 'Content-Type: application/json' -H 'MCP-Protocol-Version: 2026-07-28' -H 'Mcp-Method: server/discover' -d '{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"server/discover\",\"params\":{{\"_meta\":{{\"io.modelcontextprotocol/protocolVersion\":\"2026-07-28\",\"io.modelcontextprotocol/clientInfo\":{{\"name\":\"curl\",\"version\":\"1.0\"}},\"io.modelcontextprotocol/clientCapabilities\":{{}}}}}}}}'",
        args.port
    );
    tracing::info!("  # cross-origin browser page → 403 unless allowlisted");
    tracing::info!("  (same curl plus: -H 'Origin: https://evil.example')");

    server.run().await?;
    Ok(())
}
