//! # Interop Client Probe
//!
//! Points `turul-mcp-client` at a server this project did not write, and
//! reports what each leg of the journey did. It exists because the whole
//! client-as-driver row of the interop matrix was untested: every other check
//! in the repo has turul code on both ends of the wire, so a contract both
//! halves get wrong the same way looks identical to one they get right.
//!
//! Each leg prints a `LEG` line with one of three outcomes, and no leg aborts
//! the run — a peer that lacks prompts should show up as one reported leg, not
//! as a probe that stopped before reaching the rest.
//!
//! - `OK` — the leg was driven and the peer answered.
//! - `FAIL` — the leg was driven and the exchange broke.
//! - `SKIP` — the peer exposed nothing to drive it against, so the leg proves
//!   nothing either way. Distinct from `FAIL`, which would read as a defect in
//!   a peer that simply does not implement that surface.
//!
//! Exit status covers the modern-core legs only (`server/discover`,
//! `tools/list`, `tools/call`); the read surface is reported for the caller to
//! judge, since peers legitimately differ in what they expose. A skipped
//! `tools/call` still fails the run: the core claim is then unproven, and
//! reporting `CORE ok` off an unexercised core would be a false pass.
//!
//!   cargo run -p interop-client-probe -- `<url>`

use turul_mcp_client::completion::{CompleteArgument, CompletionReference, PromptReference};
use turul_mcp_client::schema::JsonSchema;
use turul_mcp_client::transport::HttpTransport;
use turul_mcp_client::{McpClient, McpVersion};

fn leg(name: &str, outcome: Result<String, String>) -> bool {
    match outcome {
        Ok(detail) => {
            println!("LEG {name} OK {detail}");
            true
        }
        Err(detail) => {
            println!("LEG {name} FAIL {detail}");
            false
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()),
        )
        .init();

    let url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "http://127.0.0.1:8700/mcp".to_string());
    println!("PROBE {url}");

    let transport = Box::new(HttpTransport::new(&url)?);
    let client = McpClient::new(transport, Default::default());

    // connect() is server/discover on the 2026 lane. A peer that answers the
    // 2025 handshake instead is a legitimate outcome to report, not a crash.
    let mut core_ok = leg(
        "server/discover",
        client
            .connect()
            .await
            .map(|()| String::new())
            .map_err(|e| e.to_string()),
    );
    let version = client.negotiated_version().await;
    println!("NEGOTIATED {version:?}");
    if version != Some(McpVersion::V2026_07_28) {
        println!("LEG version FAIL peer did not negotiate 2026-07-28");
        core_ok = false;
    }
    if let Some(discovered) = client.discovered_server().await {
        println!("SERVER {:?}", discovered.server_info);
    }

    let tools = match client.list_tools().await {
        Ok(tools) => {
            let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
            leg("tools/list", Ok(format!("{names:?}")));
            tools
        }
        Err(e) => {
            leg("tools/list", Err(e.to_string()));
            core_ok = false;
            Vec::new()
        }
    };

    // Arguments are derived from the advertised inputSchema rather than
    // hardcoded, so the probe works against any peer's tool set.
    if let Some(tool) = tools.first() {
        let mut args = serde_json::Map::new();
        if let Some(props) = tool.input_schema.properties.as_ref() {
            for (key, spec) in props {
                let value = match spec {
                    JsonSchema::Number { .. } | JsonSchema::Integer { .. } => serde_json::json!(5),
                    JsonSchema::Boolean { .. } => serde_json::json!(true),
                    _ => serde_json::json!("interop"),
                };
                args.insert(key.clone(), value);
            }
        }
        let name = tool.name.clone();
        core_ok &= leg(
            "tools/call",
            client
                .call_tool(&name, serde_json::Value::Object(args))
                .await
                .map(|r| {
                    serde_json::to_string(&r.content)
                        .unwrap_or_default()
                        .chars()
                        .take(120)
                        .collect()
                })
                .map_err(|e| e.to_string()),
        );
    } else {
        println!("LEG tools/call SKIP peer advertised no tools — core unproven, so CORE fails");
        core_ok = false;
    }

    let resources = client.list_resources().await;
    let first_uri = resources
        .as_ref()
        .ok()
        .and_then(|r| r.first().map(|r| r.uri.clone()));
    leg(
        "resources/list",
        resources
            .map(|r| format!("{} resource(s)", r.len()))
            .map_err(|e| e.to_string()),
    );
    if let Some(uri) = first_uri {
        leg(
            "resources/read",
            client
                .read_resource(&uri)
                .await
                .map(|c| format!("{} content block(s) from {uri}", c.len()))
                .map_err(|e| e.to_string()),
        );
    } else {
        println!("LEG resources/read SKIP peer exposed no resource to read");
    }
    leg(
        "resources/templates/list",
        client
            .list_resource_templates()
            .await
            .map(|t| format!("{} template(s)", t.len()))
            .map_err(|e| e.to_string()),
    );

    let prompts = client.list_prompts().await;
    let first_prompt = prompts
        .as_ref()
        .ok()
        .and_then(|p| p.first().map(|p| p.name.clone()));
    leg(
        "prompts/list",
        prompts
            .map(|p| format!("{} prompt(s)", p.len()))
            .map_err(|e| e.to_string()),
    );
    let completion_target = first_prompt.clone();
    if let Some(name) = first_prompt {
        leg(
            "prompts/get",
            client
                .get_prompt(&name, Some(serde_json::json!({ "name": "Ada" })))
                .await
                .map(|r| format!("{} message(s)", r.messages.len()))
                .map_err(|e| e.to_string()),
        );
    } else {
        println!("LEG prompts/get SKIP peer exposed no prompt to render");
    }

    // Completion is prompt-scoped, so it can only be driven when the peer
    // advertised a prompt above.
    if let Some(name) = completion_target {
        leg(
            "completion/complete",
            client
                .complete(
                    CompletionReference::Prompt(PromptReference::new(&name)),
                    CompleteArgument::new("name", "a"),
                    None,
                )
                .await
                .map(|r| format!("{} value(s)", r.completion.values.len()))
                .map_err(|e| e.to_string()),
        );
    } else {
        println!("LEG completion/complete SKIP peer exposed no prompt to complete against");
    }

    println!("CORE {}", if core_ok { "ok" } else { "failed" });
    if !core_ok {
        std::process::exit(1);
    }
    Ok(())
}
