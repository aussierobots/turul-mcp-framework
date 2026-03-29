# Storage Backend Matrix

Decision matrix for session and task storage backends in the Turul MCP Framework. All backends share the same builder API (`.with_session_storage()` / `.with_task_storage()`); only the config struct and feature flags differ.

All backends pass the same parity test suite ensuring session isolation, TTL enforcement, and state machine correctness. See: [CLAUDE.md — Session Management](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#session-management)

## Backend Selection

| Backend | Persistence | Multi-Instance | Setup | Best For |
|---|---|---|---|---|
| **InMemory** | None (process lifetime) | No | Zero config | Dev, tests, prototyping |
| **SQLite** | File-based | No (single process) | File path only | Single-instance prod, desktop apps |
| **PostgreSQL** | Server-based | Yes | Docker or managed DB | Multi-instance prod, horizontal scaling |
| **DynamoDB** | AWS-managed | Yes | AWS credentials + table | Serverless, Lambda, AWS-native deployments |

### Environment Guidance

| Environment | Session Storage | Task Storage |
|---|---|---|
| Local dev / tests | InMemory (default) | InMemory (default) |
| CI / integration tests | InMemory or SQLite | InMemory or SQLite |
| Single-instance prod | SQLite | SQLite |
| Multi-instance prod | PostgreSQL | PostgreSQL |
| AWS Lambda / serverless | DynamoDB | DynamoDB |

## Feature Flags

### Session Storage (`turul-mcp-session-storage`)

| Backend | Crate Feature | Extra Dependencies |
|---|---|---|
| InMemory | `in-memory` (default) | None |
| SQLite | `sqlite` | `sqlx` with `sqlite`, `runtime-tokio-rustls` |
| PostgreSQL | `postgres` | `sqlx` with `postgres`, `runtime-tokio-rustls` |
| DynamoDB | `dynamodb` | `aws-sdk-dynamodb`, `aws-config` |

### Task Storage (`turul-mcp-task-storage`)

| Backend | Crate Feature | Extra Dependencies |
|---|---|---|
| InMemory | `in-memory` (default) | `tokio` |
| SQLite | `sqlite` | `sqlx`, `tokio` |
| PostgreSQL | `postgres` | `sqlx`, `tokio` |
| DynamoDB | `dynamodb` | `aws-sdk-dynamodb`, `aws-config`, `tokio`, `base64` |

> **Note:** Since v0.3.27, the server crate's backend features forward to BOTH `turul-mcp-session-storage` AND `turul-mcp-task-storage`. You do NOT need to add them as separate dependencies with matching features — one feature on `turul-mcp-server` enables the backend everywhere. For `turul-mcp-server-state-storage` (dynamic tools), the backend feature is forwarded via weak dependency syntax when `dynamic-tools` is also enabled.

### Server Crate (`turul-mcp-server`)

Default features: `["http", "sse"]` — in-memory only, no backend deps compiled.

| Backend | `turul-mcp-server` features | Forwards to |
|---|---|---|
| InMemory | (default) | — |
| SQLite | `["sqlite"]` | session-storage + task-storage (+ server-state-storage if `dynamic-tools` active) |
| PostgreSQL | `["postgres"]` | session-storage + task-storage (+ server-state-storage if `dynamic-tools` active) |
| DynamoDB | `["dynamodb"]` | session-storage + task-storage (+ server-state-storage if `dynamic-tools` active) |
| Dynamic tools | `["dynamic-tools"]` | server-state-storage (in-memory) |
| Dynamic + DynamoDB | `["dynamodb", "dynamic-tools"]` | all three storage crates |

## Cargo.toml Patterns

### InMemory (default — no extra config)

```toml
[dependencies]
turul-mcp-server = "0.3"
# Default features: http + sse + in-memory storage. No backend deps compiled.
```

### SQLite

```toml
[dependencies]
turul-mcp-server = { version = "0.3", features = ["sqlite"] }
turul-mcp-session-storage = { version = "0.3", features = ["sqlite"] }
```

The `sqlite` feature on `turul-mcp-server` enables SQLite for both session and task storage automatically.

See: [examples/simple-sqlite-session](https://github.com/aussierobots/turul-mcp-framework/tree/main/examples/simple-sqlite-session)

### PostgreSQL

```toml
[dependencies]
turul-mcp-server = { version = "0.3", features = ["postgres"] }
turul-mcp-session-storage = { version = "0.3", features = ["postgres"] }
```

See: [examples/simple-postgres-session](https://github.com/aussierobots/turul-mcp-framework/tree/main/examples/simple-postgres-session)

### DynamoDB

```toml
[dependencies]
turul-mcp-server = { version = "0.3", features = ["dynamodb"] }
turul-mcp-session-storage = { version = "0.3", features = ["dynamodb"] }
```

See: [examples/simple-dynamodb-session](https://github.com/aussierobots/turul-mcp-framework/tree/main/examples/simple-dynamodb-session)

### DynamoDB + Dynamic Tools (Lambda production)

```toml
[dependencies]
turul-mcp-server = { version = "0.3", features = ["dynamodb", "dynamic-tools"] }
turul-mcp-session-storage = { version = "0.3", features = ["dynamodb"] }
turul-mcp-server-state-storage = { version = "0.3", features = ["dynamodb"] }
```

## Environment Variables

```bash
# SQLite
SQLITE_PATH=./sessions.db          # Session database file path (SqliteConfig default: mcp_sessions.db)
SQLITE_TASK_PATH=./tasks.db        # Task database file path

# PostgreSQL
DATABASE_URL=postgres://mcp:mcp_pass@localhost:5432/mcp_sessions

# DynamoDB
AWS_ACCESS_KEY_ID=your_access_key
AWS_SECRET_ACCESS_KEY=your_secret_key
AWS_REGION=us-east-1
MCP_SESSION_TABLE=mcp-sessions     # Convention used by scaffold-generated code (not read by DynamoDbSessionStorage natively)
MCP_TASK_TABLE=mcp-tasks           # Convention used by scaffold-generated code (not read by DynamoDbTaskStorage natively)
```

## Builder Patterns

All backends use the same builder method. The storage type is selected by the config struct you pass. These are quick-start examples; the authoritative API reference is the [crate documentation](https://docs.rs/turul-mcp-server).

### Session Storage

```rust
use turul_mcp_server::McpServer;

// InMemory (default -- nothing to add)
let server = McpServer::builder().name("my-server").build()?;

// SQLite
use turul_mcp_session_storage::{SqliteConfig, SqliteSessionStorage};
let config = SqliteConfig {
    database_path: PathBuf::from("./sessions.db"),
    verify_tables: true,
    create_tables: true,
    ..Default::default()
};
let storage = Arc::new(SqliteSessionStorage::with_config(config).await?);
let server = McpServer::builder()
    .name("my-server")
    .with_session_storage(storage)
    .build()?;

// PostgreSQL
use turul_mcp_session_storage::{PostgresConfig, PostgresSessionStorage};
let config = PostgresConfig {
    database_url: std::env::var("DATABASE_URL")?,
    verify_tables: true,
    create_tables: true,
    ..Default::default()
};
let storage = Arc::new(PostgresSessionStorage::with_config(config).await?);
let server = McpServer::builder()
    .name("my-server")
    .with_session_storage(storage)
    .build()?;

// DynamoDB
use turul_mcp_session_storage::{DynamoDbConfig, DynamoDbSessionStorage};
let config = DynamoDbConfig {
    table_name: "mcp-sessions".to_string(),
    verify_tables: true,
    create_tables: true,
    ..Default::default()
};
let storage = Arc::new(DynamoDbSessionStorage::with_config(config).await?);
let server = McpServer::builder()
    .name("my-server")
    .with_session_storage(storage)
    .build()?;
```

### Task Storage

```rust
use turul_mcp_task_storage::{SqliteTaskConfig, SqliteTaskStorage};

let config = SqliteTaskConfig {
    database_path: PathBuf::from("./tasks.db"),
    verify_tables: true,
    create_tables: true,
    ..Default::default()
};
let storage = Arc::new(SqliteTaskStorage::with_config(config).await?);
let server = McpServer::builder()
    .name("my-server")
    .with_task_storage(storage)
    .build()?;
```

Task storage follows the same pattern for PostgreSQL (`PostgresTaskConfig` / `PostgresTaskStorage`) and DynamoDB (`DynamoDbTaskConfig` / `DynamoDbTaskStorage`).

See: [CLAUDE.md — Task Storage & Executor Architecture](https://github.com/aussierobots/turul-mcp-framework/blob/main/CLAUDE.md#architecture)

## Config Struct Reference

> These tables reflect the actual Rust struct definitions and `Default` implementations. For the authoritative source, see the crate source code.

### SqliteConfig (Session)

| Field | Type | Default | Description |
|---|---|---|---|
| `database_path` | `PathBuf` | `mcp_sessions.db` | Database file path |
| `max_connections` | `u32` | `10` | Connection pool size |
| `connection_timeout_secs` | `u64` | `30` | Connection timeout |
| `session_timeout_minutes` | `u32` | `30` | Session TTL |
| `cleanup_interval_minutes` | `u32` | `5` | Background cleanup interval |
| `max_events_per_session` | `u32` | `1000` | Max stored events |
| `verify_tables` | `bool` | `false` | Verify tables at startup |
| `create_tables` | `bool` | `false` | Create tables if missing (requires `verify_tables`) |
| `create_database_if_missing` | `bool` | `true` | Auto-create DB file |

### PostgresConfig (Session)

| Field | Type | Default | Description |
|---|---|---|---|
| `database_url` | `String` | `postgres://localhost:5432/mcp_sessions` | Connection URL |
| `max_connections` | `u32` | `20` | Max pool connections |
| `min_connections` | `u32` | `2` | Min idle connections |
| `connection_timeout_secs` | `u64` | `30` | Connection timeout |
| `session_timeout_minutes` | `u32` | `30` | Session TTL |
| `cleanup_interval_minutes` | `u32` | `5` | Background cleanup interval |
| `max_events_per_session` | `u32` | `1000` | Max stored events |
| `enable_pooling_optimizations` | `bool` | `true` | Pool tuning |
| `statement_timeout_secs` | `u32` | `30` | Query timeout |
| `verify_tables` | `bool` | `false` | Verify tables at startup |
| `create_tables` | `bool` | `false` | Create tables if missing (requires `verify_tables`) |

### DynamoDbConfig (Session)

| Field | Type | Default | Description |
|---|---|---|---|
| `table_name` | `String` | `mcp-sessions` | DynamoDB table name |
| `region` | `String` | `$AWS_REGION` or `us-east-1` | AWS region |
| `session_ttl_minutes` | `u64` | `5` | Session TTL (**override for production — default is very short**) |
| `event_ttl_minutes` | `u64` | `5` | Event TTL (**override for production — default is very short**) |
| `max_events_per_session` | `u64` | `1000` | Max stored events |
| `enable_backup` | `bool` | `true` | Point-in-time recovery |
| `enable_encryption` | `bool` | `true` | Server-side encryption |
| `verify_tables` | `bool` | `false` | Verify tables at startup |
| `create_tables` | `bool` | `false` | Create tables if missing (requires `verify_tables`) |

> **Important:** DynamoDB TTL defaults are intentionally short (5 minutes) for framework testing. Production deployments should explicitly set `session_ttl_minutes` and `event_ttl_minutes` to appropriate values (e.g., 1440 for 24 hours).
