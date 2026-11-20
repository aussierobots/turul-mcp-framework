//! A 2026-07-28 server for the wasm32 client tests (`tests/wasm.rs`, ADR-040).
//!
//! It serves a blob resource with `_meta`, a resource that takes 2 s, and a
//! tool that reports progress. `scripts/ci-gates.sh wasm` starts it:
//!
//! ```text
//! cargo run -p turul-mcp-client --example wasm_e2e_fixture -- --port 52940
//! ```

use std::collections::HashMap;

use base64::Engine as _;
use turul_mcp_derive::McpTool;
use turul_mcp_protocol::resources::ResourceContent;
use turul_mcp_server::prelude::*;

/// URI of the blob resource; its bytes are `0..=255`.
const BLOB_URI: &str = "file:///fixture/blob.bin";
/// SHA-256 of `0..=255`, sent as the blob's `_meta.sha256`.
const BLOB_SHA256: &str = "40aff2e9d2d8922e47afd4648e6967497158785fbd1da870e7110266bf944880";
/// URI of the resource that answers after 2 s.
const SLOW_URI: &str = "file:///fixture/slow.txt";

struct BlobResource;

impl HasResourceMetadata for BlobResource {
    fn name(&self) -> &str {
        "blob"
    }
}
impl HasResourceDescription for BlobResource {}
impl HasResourceUri for BlobResource {
    fn uri(&self) -> &str {
        BLOB_URI
    }
}
impl HasResourceMimeType for BlobResource {}
impl HasResourceSize for BlobResource {}
impl HasResourceAnnotations for BlobResource {}
impl HasResourceMeta for BlobResource {}
impl HasIcons for BlobResource {}

#[async_trait::async_trait]
impl McpResource for BlobResource {
    async fn read(
        &self,
        _params: Option<serde_json::Value>,
        _session: Option<&SessionContext>,
    ) -> McpResult<Vec<ResourceContent>> {
        let bytes: Vec<u8> = (0..=255).collect();
        let blob = base64::engine::general_purpose::STANDARD.encode(bytes);
        let mut content = ResourceContent::blob(BLOB_URI, blob, "application/octet-stream");
        if let ResourceContent::Blob(b) = &mut content {
            b.meta = Some(HashMap::from([(
                "sha256".to_string(),
                serde_json::Value::String(BLOB_SHA256.to_string()),
            )]));
        }
        Ok(vec![content])
    }
}

struct SlowResource;

impl HasResourceMetadata for SlowResource {
    fn name(&self) -> &str {
        "slow"
    }
}
impl HasResourceDescription for SlowResource {}
impl HasResourceUri for SlowResource {
    fn uri(&self) -> &str {
        SLOW_URI
    }
}
impl HasResourceMimeType for SlowResource {}
impl HasResourceSize for SlowResource {}
impl HasResourceAnnotations for SlowResource {}
impl HasResourceMeta for SlowResource {}
impl HasIcons for SlowResource {}

#[async_trait::async_trait]
impl McpResource for SlowResource {
    async fn read(
        &self,
        _params: Option<serde_json::Value>,
        _session: Option<&SessionContext>,
    ) -> McpResult<Vec<ResourceContent>> {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        Ok(vec![ResourceContent::text(SLOW_URI, "late")])
    }
}

/// Reports one progress notification on the request stream, then completes.
#[derive(McpTool, Clone, Default)]
#[tool(name = "progress_worker", description = "Works with progress", output = String)]
struct ProgressWorkerTool {}

impl ProgressWorkerTool {
    async fn execute(&self, session: Option<SessionContext>) -> McpResult<String> {
        if let Some(session) = session {
            session.notify_request_progress(0.5, Some(1.0)).await;
        }
        Ok("worked".to_string())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let mut port: u16 = 52940;
    while let Some(arg) = args.next() {
        if arg == "--port" {
            port = args.next().ok_or("--port needs a value")?.parse()?;
        }
    }

    let server = McpServer::builder()
        .name("wasm-e2e-fixture")
        .version(env!("CARGO_PKG_VERSION"))
        .resource(BlobResource)
        .resource(SlowResource)
        .tool(ProgressWorkerTool::default())
        .bind_address(format!("127.0.0.1:{port}").parse()?)
        .build()?;
    server.run().await?;
    Ok(())
}
