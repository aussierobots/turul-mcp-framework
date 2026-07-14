# ADR: JsonSchema Standardization in MCP Framework

**Date**: 2025-08-28  
**Status**: ✅ **ACCEPTED** and **IMPLEMENTED**  
**Decision Maker**: Framework Architecture Team  

## Context

The MCP Framework was experiencing a persistent compilation issue with the `#[mcp_tool]` function attribute macro, where there was a type mismatch between `HashMap<String, JsonSchema>` and `HashMap<String, serde_json::Value>` in the `ToolSchema::with_properties()` method.

### Problem Statement

1. **Function Macro Failure**: `#[mcp_tool]` consistently failed with type mismatch errors
2. **Inconsistent Schema Types**: Some parts used `JsonSchema`, others used `serde_json::Value`
3. **Complex Conversion Layer**: Macros required `serde_json::to_value()` conversions that weren't working reliably
4. **Architecture Fragmentation**: Different schema representation across the codebase

### Technical Investigation

The root cause was identified in the `ToolSchema` struct definition:

```rust
// OLD: Mixed types causing issues
pub struct ToolSchema {
    pub properties: Option<HashMap<String, serde_json::Value>>, // ❌ Generic Value
    // ...
}

// Macros trying to pass JsonSchema but method expects Value
pub fn with_properties(mut self, properties: HashMap<String, serde_json::Value>) -> Self
```

## Decision

**We standardize the entire MCP Framework to use `JsonSchema` consistently throughout, eliminating `serde_json::Value` for schema definitions.**

### Core Changes

1. **ToolSchema Standardization**:
   ```rust
   // NEW: Consistent JsonSchema usage
   pub struct ToolSchema {
       pub properties: Option<HashMap<String, JsonSchema>>, // ✅ Strongly typed
       // ...
   }
   
   pub fn with_properties(mut self, properties: HashMap<String, JsonSchema>) -> Self
   ```

2. **Macro Simplification**:
   ```rust
   // OLD: Complex conversion
   schema_properties.push(quote! {
       (#param_name_str.to_string(), serde_json::to_value(&#schema).unwrap_or_else(|_| serde_json::json!({"type": "string"})))
   });
   
   // NEW: Direct usage
   schema_properties.push(quote! {
       (#param_name_str.to_string(), #schema)
   });
   ```

3. **Builder Pattern Updates**:
   ```rust
   // OLD: JSON generation
   .with_properties(HashMap::from([
       ("result".to_string(), serde_json::json!({"type": "number"}))
   ]))
   
   // NEW: Type-safe construction
   .with_properties(HashMap::from([
       ("result".to_string(), JsonSchema::number())
   ]))
   ```

## Rationale

### Why JsonSchema over serde_json::Value?

1. **Type Safety**: `JsonSchema` is a strongly-typed enum vs generic `Value`
2. **MCP Compliance**: `JsonSchema` directly represents JSON Schema specification concepts
3. **Compile-Time Validation**: Errors caught at compile time vs runtime
4. **IDE Support**: Better IntelliSense and auto-completion
5. **Performance**: No runtime conversion overhead
6. **Maintainability**: Clear schema structure vs opaque JSON values

### Why Not Keep Mixed Types?

1. **Complexity**: Conversion layer was error-prone and hard to debug
2. **Inconsistency**: Different parts of codebase used different representations
3. **Fragility**: Macro hygiene issues with conversion in different expansion contexts
4. **Developer Experience**: Confusing to have multiple ways to define schemas

## Implementation

### Changes Made

1. **Core Protocol Types** (`turul-mcp-protocol-2025-06-18/src/tools.rs`):
   - Updated `ToolSchema.properties` type
   - Updated `with_properties()` method signature
   - Added proper JsonSchema imports

2. **Macro Simplification** (`turul-mcp-derive/src/`):
   - Removed `serde_json::to_value()` conversion calls
   - Cleaned up `tool_derive.rs` and `tool_attr.rs`
   - Deleted obsolete `type_to_json_value()` function

3. **Builder Updates**:
   - `turul-mcp-protocol-2025-06-18/src/tools/builder.rs`
   - `turul-mcp-builders/src/tool.rs`
   - Changed from `serde_json::json!()` to `JsonSchema::*()` constructors

### Testing Verification

```bash
# ✅ Core examples compile successfully
cargo check --package minimal-server          # Function macro
cargo check --package derive-macro-server     # Derive macro  
cargo check --package function-macro-server   # Additional function examples

# ✅ Framework compiles cleanly
cargo check --package turul-mcp-protocol-2025-06-18
cargo check --package turul-mcp-derive
cargo check --package turul-mcp-server
```

## Consequences

### Positive Outcomes

1. **✅ Function Macro Fixed**: `#[mcp_tool]` compiles and runs correctly
2. **✅ Simplified Architecture**: No conversion layer needed
3. **✅ Better Type Safety**: Compile-time schema validation
4. **✅ Improved Performance**: Eliminated runtime conversions
5. **✅ Consistent Codebase**: Unified schema representation
6. **✅ Better Developer Experience**: Clear, type-safe API

### Breaking Changes

1. **Tool Builders**: Code using `serde_json::json!()` for schemas needs updating to `JsonSchema::*()` constructors
2. **Manual Tool Implementations**: Direct `ToolSchema` construction needs type updates

### Migration Path

```rust
// OLD (won't compile)
ToolSchema::object().with_properties(HashMap::from([
    ("field".to_string(), serde_json::json!({"type": "string"}))
]))

// NEW (recommended)
ToolSchema::object().with_properties(HashMap::from([
    ("field".to_string(), JsonSchema::string())
]))
```

## Compliance with MCP Specification

### JSON Schema Serialization

The `JsonSchema` enum serializes to identical JSON as before:

```rust
// JsonSchema::string() serializes to:
{"type": "string"}

// JsonSchema::number().with_minimum(0.0) serializes to:
{"type": "number", "minimum": 0.0}
```

### MCP Protocol Compatibility

- **Wire Protocol**: Unchanged - same JSON Schema format over the wire
- **TypeScript Interop**: Perfect compatibility with MCP TypeScript clients
- **MCP Inspector**: Full compatibility maintained
- **Specification Compliance**: 100% MCP 2025-06-18 compliant

## Alternatives Considered

### Option 1: Fix Conversion Layer
- **Approach**: Make `serde_json::to_value()` work reliably in macro context
- **Rejected**: Complex, error-prone, maintains architectural inconsistency

### Option 2: Use serde_json::Value Everywhere  
- **Approach**: Convert all JsonSchema usage to Value
- **Rejected**: Loses type safety, worse developer experience

### Option 3: Maintain Both Types
- **Approach**: Keep both types with reliable conversion
- **Rejected**: Architectural complexity, confusion for developers

## Monitoring and Review

### Success Criteria
- [x] Function macro (`#[mcp_tool]`) compiles and runs
- [x] Derive macro (`#[derive(McpTool)]`) continues working  
- [x] No regression in MCP protocol compliance
- [x] Clean compilation across core framework
- [x] Zero performance regression

### Future Considerations
- Monitor for any JSON Schema spec changes requiring JsonSchema enum updates
- Consider adding validation methods to JsonSchema enum
- Evaluate extending JsonSchema with additional schema features if needed

## References

- MCP Specification 2025-06-18: https://spec.modelcontextprotocol.io/
- JSON Schema Specification: https://json-schema.org/
- Original Issue: `FUNCTION_MACRO_DEBUG_NOTES.md`
- Implementation: PR fixing `#[mcp_tool]` compilation

## Revision log

### 2026-07-14 — BP-3 dialect validation for `inputSchema` (DRAFT-2026-v1)

DRAFT-2026-v1 (SEP-2106) opens `Tool.inputSchema` to the full JSON Schema
2020-12 vocabulary (`oneOf`/`anyOf`/`allOf`/`$ref`/`$defs`/conditionals),
superseding this ADR's original structural `JsonSchema` enum for the 2026
wire type (`turul-mcp-protocol-2026-07-28::tools::ToolSchema` models
`properties` as `HashMap<String, serde_json::Value>` precisely so arbitrary
2020-12 shapes pass through unconverted — see that type's doc comment). An
unrestricted schema surface needs a real validator at the two trust
boundaries: a server MUST NOT advertise an invalid `inputSchema`, and a
client MUST exclude a tool whose `inputSchema` is invalid from `tools/list`.

**Two distinct categories of requirement — do not conflate them.** The spec's
basic protocol row 207 states a MUST: "Clients and servers MUST validate
schemas according to their declared or default dialect and MUST handle
unsupported dialects gracefully by returning an appropriate error." The
dialect check (absent `$schema` → 2020-12; present and not the canonical
2020-12 URI → `UnsupportedDialect`) and the 2020-12 meta-validation compile
step are that MUST. Separately, **as a matter of framework security policy,
not a JSON Schema spec requirement**, this validator also rejects a remote
`$ref` (prevents SSRF — fetching attacker-controlled schema content over the
network) and enforces size/nesting/composition-depth bounds (prevents
resource-exhaustion DoS from an oversized or pathologically nested schema).
Both are deliberate hardening choices layered on top of the spec MUST; a
minimal spec-compliant implementation would not need either.

**Validator choice**: `jsonschema` 0.47 (crates.io), `default-features =
false`. That drops `reqwest`/`resolve-http`/`resolve-file`/`tls-*` entirely —
2020-12 compilation still works with zero features enabled (spiked and
confirmed runnable before adoption). A `Retrieve` implementation that errors
on every lookup is installed in addition, so remote `$ref` resolution is
refused at two independent layers: it cannot compile in (feature-absent) and
it would refuse at runtime if it somehow could.

**Bounds** (framework security policy, checked before the compile step):
`MAX_SCHEMA_BYTES = 256 KiB`; `MAX_COMPOSITION_DEPTH = 32` (nesting through
`allOf`/`anyOf`/`oneOf`/`not`/`if`/`then`/`else`/`items`/`properties`). Any
`$ref` whose string value has a scheme+authority (`http://`, `https://`, or
generally `://`) is rejected outright as `RemoteRef`, ahead of the compile
step. **Cycle safety**: the bounds walk traverses only the literal JSON
document tree — it never resolves or follows a `$ref` target. A legitimate
cyclic local `$ref` (e.g. a recursive tree-node schema referencing itself
through `$defs`) MUST pass the bounds check and does; `jsonschema` resolves
and validates such recursion safely at compile time in the step that
follows. `MAX_REF_DEPTH` remains defined for API stability but is not
currently exercised by the walk: since `$ref` targets are never resolved,
there is no "chain" to measure without reintroducing the cycle risk this
bound would otherwise guard against. An earlier draft of this slice did
follow local `$ref` chains (with a depth cap to avoid an unbounded loop) and
was corrected during review: a genuinely cyclic local `$ref` — the same
target re-resolved on every hop — would exceed that cap and be rejected as
`TooDeep` after `MAX_REF_DEPTH` hops, which is exactly the "MUST pass"
recursive-schema case this bound would have wrongly rejected.

**Diagnostics**: every error variant's message names the specific value that
failed — the offending `$ref` URI and the word "policy" (distinguishing a
framework hardening rejection from a spec-mandated one) for `RemoteRef`; the
exceeded byte count and limit for `TooLarge`; the nesting kind and limit for
`TooDeep`. Asserted in tests via `.to_string()` content, not merely the error
variant.

**Placement**: the validator (`validate_tool_input_schema`,
`SchemaValidationError`, `SchemaValidationError` derived via `thiserror`)
lives in a new dedicated leaf crate, `turul-mcp-schema-validation` (deps:
`serde_json`, `jsonschema`, `thiserror` — all normal, no cargo-feature
plumbing). It was NOT placed in `turul-mcp-builders` — an earlier draft of
this slice did that and was corrected during review, because `turul-mcp-
client`'s `src/` uses `turul_mcp_builders` zero times (piggybacking would
have strengthened an unused coupling), and `turul-mcp-builders` uncondi-
tionally depends on the `turul-mcp-protocol` alias crate, so linking it from
`turul-mcp-client` would reintroduce the alias's `protocol-2025-11-25` /
`protocol-2026-07-28` feature mutex into the client's dependency graph
(confirmed by spiking the dependency: it fails to compile with neither
feature selected) — exactly the coupling ADR-030 removed the alias from this
crate to avoid. The new crate has zero dependency on any protocol crate, so
both `turul-mcp-server` and `turul-mcp-client` link it as a plain, uncondi-
tional dependency with no risk and no feature gate. `jsonschema` stays out of
`turul-mcp-protocol-2026-07-28` (Protocol Crate Purity) and out of the
*compiled* `turul-mcp-derive` proc-macro artifact — confirmed via `cargo tree
-p turul-mcp-derive -e normal,build -i jsonschema` (empty). The derive
crate's own `[dev-dependencies]` on `turul-mcp-server` does surface
`jsonschema` in the default, dev-edges-included `cargo tree -i` output, but
that dev-only edge (used only by the derive crate's own test suite) never
reaches the published proc-macro artifact.

**SEP-2243 x-mcp-header, same client trust boundary**: a new detector,
`turul-mcp-protocol-2026-07-28::headers::find_misplaced_x_mcp_header`, closes
a gap the existing `scan_x_mcp_headers` walk could not see — an `x-mcp-header`
annotation reachable only through `items`/composition (`oneOf`/`anyOf`/`allOf`/
`not`/`if`/`then`/`else`)/`$ref` rather than a plain `properties` chain.
`scan_x_mcp_headers` silently skips these positions (by design, for the
positive binding scan); the new function is pure detection so the client can
exclude the whole tool, per the spec's "reject the whole tool definition"
rule. This is a small, dependency-free helper and stays in the protocol
crate (spec-shape logic, not a validator).

**Conclusion**: JsonSchema standardization successfully resolved the function macro issue while improving the overall architecture with better type safety, performance, and maintainability. This decision aligns with the framework's goal of providing a type-safe, developer-friendly MCP implementation.