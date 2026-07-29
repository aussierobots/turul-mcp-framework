//! Smoke client for `lambda-mcp-server`
//!
//! **Deliberately targets the 2025-11-25 stateful lane**, because
//! `lambda-mcp-server` is pinned there: the flow below (`initialize` →
//! `notifications/initialized` → `Mcp-Session-Id` on every later request) was
//! removed by the 2026-07-28 stateless core. For the 2026 client pattern see
//! `streamable-http-client`; for the raw 2025 wire see
//! `streamable-http-client-2025-11-25`.
//!
//! Two subcommands:
//!
//! - `probe` — non-interactive: handshake, `tools/list`, and one `tools/call`
//!   per advertised tool. Exits non-zero if any step fails, so it is usable
//!   as a deploy smoke check.
//! - `connect` — the same handshake, then an interactive REPL for poking at
//!   individual tools.
//!
//! ```bash
//! # Locally, against `cargo lambda watch --package lambda-turul-mcp-server`:
//! cargo run -p lambda-turul-mcp-client -- probe
//!
//! # Against a deployed Function URL:
//! cargo run -p lambda-turul-mcp-client -- probe --url https://<id>.lambda-url.<region>.on.aws
//! ```

use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use colored::*;
use serde_json::{Value, json};
use uuid::Uuid;

mod client;

use client::{McpClient, McpClientConfig};

const DEFAULT_URL: &str = "http://127.0.0.1:9000/lambda-url/lambda-turul-mcp-server";

#[derive(Parser)]
#[command(name = "lambda-turul-mcp-client")]
#[command(about = "Smoke client for lambda-turul-mcp-server (MCP 2025-11-25 lane)")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Handshake, list tools, and call each one; non-zero exit on failure
    Probe(ConnectArgs),
    /// Handshake, then an interactive prompt
    Connect(ConnectArgs),
}

#[derive(Args)]
struct ConnectArgs {
    /// Server URL (`/mcp` is appended when absent)
    #[arg(long, default_value = DEFAULT_URL)]
    url: String,

    /// Session ID label for local logging
    #[arg(long)]
    session_id: Option<String>,

    /// Print full JSON for each response
    #[arg(long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "lambda_turul_mcp_client=info".into()),
        )
        .init();

    match Cli::parse().command {
        Commands::Probe(args) => run_probe(args).await,
        Commands::Connect(args) => run_interactive_session(args).await,
    }
}

async fn connect(args: &ConnectArgs) -> Result<McpClient> {
    let mut client = McpClient::new(McpClientConfig {
        base_url: args.url.clone(),
    })
    .await?;

    // The framework client runs the 2025 handshake (initialize +
    // notifications/initialized) inside connect() and keeps the session id
    // private, so this is the whole lifecycle from the caller's side.
    let info = client.connect().await?;
    println!("{} connected to {}", "✅".green(), info.endpoint);
    println!(
        "   negotiated: {}",
        info.negotiated_version
            .as_deref()
            .unwrap_or("unreported")
            .bright_green()
    );
    if args.debug {
        println!("{}", serde_json::to_string_pretty(&info)?);
    }

    Ok(client)
}

/// Non-interactive smoke check: every advertised tool must answer.
async fn run_probe(args: ConnectArgs) -> Result<()> {
    println!("{}", "🔎 lambda-mcp-server probe".bright_blue().bold());
    println!("Server: {}", args.url.bright_cyan());

    let client = connect(&args).await?;

    let tools = client.list_tools().await?;
    println!(
        "{} tools/list — {} tool(s)",
        "✅".green(),
        tools.tools.len()
    );

    // Each tool is called with empty arguments, so a tool with required
    // parameters answers with an error. That is a real round-trip, not a
    // deployment fault, so it is reported rather than gated on. The probe
    // fails only when the handshake or tools/list fails (both `?` above), or
    // when a server advertising tools answers none of them.
    let mut answered = 0usize;
    for tool in &tools.tools {
        match client.call_tool(&tool.name, Some(json!({}))).await {
            Ok(result) => {
                answered += 1;
                println!("{} tools/call {}", "✅".green(), tool.name.bright_cyan());
                if args.debug {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
            }
            Err(e) => println!(
                "{} tools/call {} — {e} (expected when the tool has required parameters)",
                "•".yellow(),
                tool.name.yellow()
            ),
        }
    }

    println!();
    if tools.tools.is_empty() || answered > 0 {
        println!(
            "{}",
            format!(
                "🎉 probe passed — handshake + tools/list OK, {answered}/{} tool(s) answered with empty arguments",
                tools.tools.len()
            )
            .bright_green()
            .bold()
        );
        Ok(())
    } else {
        println!(
            "{}",
            format!(
                "💥 probe failed — {} tool(s) advertised, none answered",
                tools.tools.len()
            )
            .bright_red()
            .bold()
        );
        std::process::exit(1);
    }
}

async fn run_interactive_session(args: ConnectArgs) -> Result<()> {
    println!("{}", "🔗 Interactive MCP Session".bright_blue().bold());
    println!("Connecting to: {}", args.url.bright_cyan());

    let label = args
        .session_id
        .clone()
        .unwrap_or_else(|| format!("interactive-{}", Uuid::new_v4()));
    println!("Label: {}", label.bright_green());

    let client = connect(&args).await?;

    let tools = client.list_tools().await?;
    println!("\n{}", "Available Tools:".bright_yellow().bold());
    for tool in &tools.tools {
        let desc = tool.description.as_deref().unwrap_or("No description");
        println!("  {} - {}", tool.name.bright_cyan(), desc);
    }

    println!(
        "\n{}",
        "Type 'help' for commands, 'quit' to exit".bright_yellow()
    );

    loop {
        use std::io::{self, Write};
        print!("\n> ");
        io::stdout().flush()?;

        let mut input = String::new();
        if io::stdin().read_line(&mut input)? == 0 {
            break; // EOF
        }
        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        match input {
            "quit" | "exit" => break,
            "help" => {
                println!("Commands:");
                println!("  help                    - Show this help");
                println!("  tools                   - List available tools");
                println!("  call <tool> [json_args] - Call a tool");
                println!("  endpoint                - Show the URL being posted to");
                println!("  quit                    - Exit");
            }
            "tools" => {
                let tools = client.list_tools().await?;
                println!("{}", serde_json::to_string_pretty(&tools)?);
            }
            "endpoint" => println!("{}", client.endpoint()),
            input if input.starts_with("call ") => {
                let parts: Vec<&str> = input.splitn(3, ' ').collect();
                if parts.len() < 2 {
                    println!("Usage: call <tool_name> [json_args]");
                    continue;
                }
                let arguments: Value = parts
                    .get(2)
                    .and_then(|raw| serde_json::from_str(raw).ok())
                    .unwrap_or_else(|| json!({}));
                match client.call_tool(parts[1], Some(arguments)).await {
                    Ok(result) => println!("{}", serde_json::to_string_pretty(&result)?),
                    Err(e) => println!("Error: {}", e.to_string().red()),
                }
            }
            _ => println!(
                "Unknown command: {}. Type 'help' for available commands.",
                input.red()
            ),
        }
    }

    println!("\n{}", "👋 Session ended".bright_blue());
    Ok(())
}
