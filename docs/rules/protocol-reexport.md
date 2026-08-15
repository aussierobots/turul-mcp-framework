# Protocol Re-export Rule (MANDATORY)

**NEVER reference versioned protocol crates directly.** Always use the `turul-mcp-protocol` re-export crate.

```rust
// CORRECT
use turul_mcp_protocol::*;
use turul_mcp_protocol::elicitation::ElicitResult;

// WRONG - NEVER reference versioned crates directly
use turul_mcp_protocol_2026_07_28::*;   // FORBIDDEN
use turul_mcp_protocol_2025_11_25::*;   // FORBIDDEN
use turul_mcp_protocol_2025_06_18::*;   // FORBIDDEN
```

**Only exceptions**:
1. `crates/turul-mcp-protocol/` (the re-export crate itself).
2. Each versioned protocol crate within its own source (`turul-mcp-protocol-2026-07-28`, `-2025-11-25`, `-2025-06-18`).
3. **`crates/turul-mcp-client/`** — the bilingual client links **both** versioned protocol crates directly (`turul-mcp-protocol-2025-11-25` and `turul-mcp-protocol-2026-07-28`, gated by the `client-bilingual` / `client-2025-11-25-only` / `client-2026-07-28-only` features) so a single client can negotiate and speak either wire spec per connection. It does **not** route through the `turul-mcp-protocol` alias. This is the one consumer-side exception; it is documented in ADR-030 and ADR-001's revision log.

**Import Hierarchy** (prefer top):
- `turul_mcp_server::prelude::*` — re-exports everything (protocol + builders + server types)
- `turul_mcp_builders::prelude::*` — framework traits + runtime builders
- `turul_mcp_protocol::*` — MCP spec types only (Tool, Resource, Prompt, McpError)
