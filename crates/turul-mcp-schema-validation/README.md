# turul-mcp-schema-validation

JSON Schema 2020-12 dialect validation for MCP `Tool.inputSchema`.

MCP 2026-07-28 (SEP-2106) opened `Tool.inputSchema` to the full JSON Schema
2020-12 vocabulary — `oneOf` / `anyOf` / `allOf` / `$ref` / `$defs` and
conditionals — and states the requirement this crate satisfies:

> Clients and servers MUST validate schemas according to their declared or
> default dialect and MUST handle unsupported dialects gracefully by returning
> an appropriate error.

`validate_tool_input_schema` is that MUST: a dialect check plus a 2020-12
meta-validation compile step.

## Two checks that are policy, not spec

Be clear about which line each check sits on, because only the first is
portable to other implementations:

| Check | Source | Why |
|---|---|---|
| Dialect + 2020-12 meta-validation | **MCP spec MUST** (SEP-2106) | Interop correctness |
| Remote `$ref` rejected | **Framework security policy** | Prevents SSRF — a schema that fetches attacker-controlled content over the network |
| Size / nesting / composition-depth bounds | **Framework security policy** | Prevents resource-exhaustion DoS from an oversized or pathologically nested schema |

A schema this crate rejects for either policy reason may still be valid JSON
Schema 2020-12 and may be accepted by another MCP implementation. That is
deliberate; it is not a spec disagreement.

## Limits

```rust
MAX_SCHEMA_BYTES       = 256 * 1024   // 256 KiB
MAX_REF_DEPTH          = 32
MAX_COMPOSITION_DEPTH  = 32
```

## Usage

```rust
use turul_mcp_schema_validation::validate_tool_input_schema;

validate_tool_input_schema(&schema)?;
```

The framework calls this for you on the tool-registration path; depend on the
crate directly only if you are validating schemas outside a `turul-mcp-server`.

## Testing

```bash
cargo test -p turul-mcp-schema-validation
```
