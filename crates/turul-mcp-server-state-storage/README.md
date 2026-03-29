# turul-mcp-server-state-storage

Server-global state storage for MCP entity registries. Provides pluggable persistence for entity activation state (tools, resources, prompts) across different backends, enabling cross-instance coordination for dynamic tool changes.

Part of the [Turul MCP Framework](https://github.com/aussierobots/turul-mcp-framework).

## Backends

| Backend | Feature | Use Case |
|---|---|---|
| **InMemory** | `in-memory` (default) | Single-process, dev/testing |
| **SQLite** | `sqlite` | Local durable, single-instance |
| **PostgreSQL** | `postgres` | Multi-instance, horizontal scaling |
| **DynamoDB** | `dynamodb` | Serverless, AWS Lambda |

## Usage

```toml
[dependencies]
# In-memory only (default)
turul-mcp-server-state-storage = "0.3"

# With DynamoDB
turul-mcp-server-state-storage = { version = "0.3", features = ["dynamodb"] }
```

Typically used via `turul-mcp-server` — the `dynamic-tools` feature pulls in this crate, and backend features (`sqlite`, `postgres`, `dynamodb`) forward automatically via weak dependency syntax:

```toml
turul-mcp-server = { version = "0.3", features = ["dynamodb", "dynamic-tools"] }
```

## Trait

```rust
#[async_trait]
pub trait ServerStateStorage: Send + Sync {
    async fn get_entity_state(&self, entity_type: &str, entity_id: &str) -> Result<Option<EntityState>>;
    async fn set_entity_state(&self, entity_type: &str, entity_id: &str, state: EntityState) -> Result<()>;
    async fn get_active_entities(&self, entity_type: &str) -> Result<Vec<String>>;
    async fn get_all_entities(&self, entity_type: &str) -> Result<Vec<EntityState>>;
    async fn get_fingerprint(&self, entity_type: &str) -> Result<Option<String>>;
    async fn set_fingerprint(&self, entity_type: &str, fingerprint: &str) -> Result<()>;
    async fn get_snapshot(&self, entity_type: &str) -> Result<Option<RegistrySnapshot>>;
    async fn backend_name(&self) -> &str;
}
```

## License

MIT OR Apache-2.0
