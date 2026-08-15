# Spec-Version Naming: ALWAYS the full date, NEVER a bare year

**Identify an MCP spec version by its full `YYYY-MM-DD` (or `YYYY_MM_DD`) date — never by year alone.** A bare year is ambiguous: 2025 shipped **two** specs (`2025-06-18` and `2025-11-25`). `v2026` / `client-2026-only` / `protocol-2025` are FORBIDDEN.

```rust
// CORRECT — full date, unambiguous
mod v2026_07_28;                         McpVersion::V2026_07_28
feature = "client-2026-07-28-only"       feature = "protocol-2025-11-25"
fn send_2026_07_28(...)                  // crates/turul-mcp-ext-tasks (no date = spec-neutral)

// WRONG — bare year, ambiguous
mod v2026;                               feature = "client-2026-only"
fn send_2026(...)                        "protocol-2025"
```

Applies to module names, function/identifier names, cargo features, type/enum variants, and prose. The only spec-version tokens that omit a date are deliberately spec-NEUTRAL names (e.g. a single `turul-mcp-ext-tasks` crate that spans specs — see ADR-028).
