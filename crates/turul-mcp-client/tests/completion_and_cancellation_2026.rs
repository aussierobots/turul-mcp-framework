//! `completion/complete` and `notifications/cancelled` through the public
//! client API, against a REAL in-process 2026-07-28 server.
//!
//! Both were unreachable from `McpClient` before: there was no `complete` op at
//! all, and `notifications/cancelled` had only a private envelope builder.
#![cfg(feature = "client-bilingual")]

use turul_mcp_client::config::ClientConfig;
use turul_mcp_client::transport::http::HttpTransport;
use turul_mcp_client::{McpClient, McpVersion};
use turul_mcp_derive::McpTool;
use turul_mcp_server::prelude::*;

#[derive(McpTool, Clone, Default)]
#[tool(name = "echo", description = "Echo back the message", output = String)]
struct EchoTool {
    #[param(description = "Message to echo back")]
    message: String,
}

impl EchoTool {
    async fn execute(&self, _session: Option<SessionContext>) -> McpResult<String> {
        Ok(format!("Echo: {}", self.message))
    }
}

/// Completes the `name` argument of the `greet` prompt.
struct GreetNameCompleter;

impl HasCompletionMetadata for GreetNameCompleter {
    fn method(&self) -> &str {
        "completion/complete"
    }
    fn reference(&self) -> &turul_mcp_protocol::completion::CompletionReference {
        use std::sync::OnceLock;
        use turul_mcp_protocol::completion::{CompletionReference, PromptReference};
        static REFERENCE: OnceLock<CompletionReference> = OnceLock::new();
        REFERENCE.get_or_init(|| CompletionReference::Prompt(PromptReference::new("greet")))
    }
}

impl HasCompletionContext for GreetNameCompleter {
    fn argument(&self) -> &turul_mcp_protocol::completion::CompleteArgument {
        use std::sync::OnceLock;
        use turul_mcp_protocol::completion::CompleteArgument;
        static ARGUMENT: OnceLock<CompleteArgument> = OnceLock::new();
        ARGUMENT.get_or_init(|| CompleteArgument::new("name", ""))
    }
}

impl HasCompletionHandling for GreetNameCompleter {}

#[async_trait::async_trait]
impl turul_mcp_server::McpCompletion for GreetNameCompleter {
    async fn complete(
        &self,
        request: turul_mcp_protocol::completion::CompleteRequest,
    ) -> McpResult<turul_mcp_protocol::completion::CompleteResult> {
        use turul_mcp_protocol::completion::{CompleteResult, CompletionResult};
        let prefix = request.params.argument.value.to_lowercase();
        let values: Vec<String> = ["alpha", "beta", "gamma"]
            .iter()
            .filter(|v| v.starts_with(&prefix))
            .map(|v| v.to_string())
            .collect();
        Ok(CompleteResult::new(CompletionResult::new(values)))
    }
}

async fn start_2026_server() -> String {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();

    let server = McpServer::builder()
        .name("completion-cancel-2026")
        .version("0.4.0")
        .tool(EchoTool::default())
        .completion_provider(GreetNameCompleter)
        .bind_address(format!("127.0.0.1:{port}").parse().unwrap())
        .build()
        .expect("build 2026 server");

    tokio::spawn(async move {
        server.run().await.ok();
    });

    let url = format!("http://127.0.0.1:{port}/mcp");
    let probe = reqwest::Client::new();
    for _ in 0..50 {
        if probe.get(&url).send().await.is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    url
}

async fn connect(url: &str) -> McpClient {
    let transport = Box::new(HttpTransport::new(url).unwrap());
    let client = McpClient::new(transport, ClientConfig::default());
    client.connect().await.expect("negotiation must succeed");
    assert_eq!(
        client.negotiated_version().await,
        Some(McpVersion::V2026_07_28)
    );
    client
}

#[tokio::test]
async fn complete_reaches_the_server_and_returns_its_suggestions() {
    use turul_mcp_protocol_2025_11_25::completion::{
        CompleteArgument, CompletionReference, PromptReference,
    };

    let url = start_2026_server().await;
    let client = connect(&url).await;

    let result = client
        .complete(
            CompletionReference::Prompt(PromptReference::new("greet")),
            CompleteArgument::new("name", "b"),
            None,
        )
        .await
        .expect("completion/complete must be reachable from the client");

    assert_eq!(
        result.completion.values,
        vec!["beta".to_string()],
        "the server's provider must drive the result"
    );
}

#[tokio::test]
async fn complete_passes_the_context_arguments_through() {
    use turul_mcp_protocol_2025_11_25::completion::{
        CompleteArgument, CompletionContext, CompletionReference, PromptReference,
    };

    let url = start_2026_server().await;
    let client = connect(&url).await;

    let context = CompletionContext {
        arguments: Some(std::collections::HashMap::from([(
            "locale".to_string(),
            "en".to_string(),
        )])),
    };
    let result = client
        .complete(
            CompletionReference::Prompt(PromptReference::new("greet")),
            CompleteArgument::new("name", ""),
            Some(context),
        )
        .await
        .expect("a request carrying context must still be accepted");

    assert_eq!(result.completion.values.len(), 3);
}

#[tokio::test]
async fn cancel_request_is_accepted_by_the_server() {
    let url = start_2026_server().await;
    let client = connect(&url).await;

    client
        .cancel_request("req_0", Some("user cancelled"))
        .await
        .expect("notifications/cancelled must be accepted on the wire");
}
