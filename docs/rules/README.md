# Rules Index

One standing rule per file. `CLAUDE.md` links here instead of inlining these —
keep the rule text in exactly one place.

Precedence, per `CLAUDE.md` §Source of Truth: the MCP spec + vendored schema,
then `docs/adr/`, then the code, outrank every file in this directory. A rule
here is prose *about* the system; where it contradicts the spec, an ADR, or
working code, the rule is what's wrong — fix the wording, not the system.

| Rule | Governs |
|---|---|
| [protocol-crate-purity.md](protocol-crate-purity.md) | What may live in `turul-mcp-protocol*` crates; frozen 2025-* crates |
| [protocol-reexport.md](protocol-reexport.md) | Always import via `turul-mcp-protocol`, never a versioned crate directly |
| [spec-version-naming.md](spec-version-naming.md) | Full `YYYY-MM-DD` spec dates, never a bare year |
| [zero-configuration-design.md](zero-configuration-design.md) | No method strings — framework derives them from types |
| [crate-versioning.md](crate-versioning.md) | Per-crate `version =`, what's stale vs. legitimately still 0.3, workspace deps |
| [comments.md](comments.md) | What a source comment may say; forbidden tags/citations; slice completion gate |
| [test-coverage-discipline.md](test-coverage-discipline.md) | Pre-publish test gate; what makes a check meaningless; reviewer-agent briefing |
| [notification-architecture.md](notification-architecture.md) | SessionManager as sole event bus; wire-complete notification envelopes; handler error rules |
| [wire-format-compliance.md](wire-format-compliance.md) | Streamable HTTP headers/status codes, camelCase JSON, structuredContent, 2025-11-25 opt-in lane |
| [scope-discipline.md](scope-discipline.md) | Minimal fixes only; stay inside the approved plan; core-crate change checklist |
