# Comments

**Comments describe what the code IS and what's non-obvious to a human reading it.** Not session history, not internal phase tags, not decision-record citations, not line numbers that will rot. Keep them clean and minimal — a comment earns its place only by explaining something the code itself cannot.

**Forbidden:**
- **Internal development phases** — `Phase 3.4`, `Slice 1`, `Batch N`, `Group A`, `Migration step 2`. These mean nothing once the session ends.
- **Internal requirement / gap-register / audit identifiers** — `BP-3`, `GAP-CF-9`, `VER-1`, `TX/GAP-7`, `CF/GAP-x`, or any tracking ID from `docs/plans/2026-07-28-spec-compliance.md` (the compliance matrix / gap register). Same class as phase tags: they are how *we* track a fix, not what the code IS. In source, state the spec requirement itself (the MUST) or cite the external `SEP-####` / schema `@see` anchor — never the internal gap ID. (Docs — the matrix, ADRs, CHANGELOG — may cite these IDs freely.)
- **Internal decision-record (ADR) references** — `per ADR-025`, `see ADR-029`, `cuts the shim per ADR-025`, `ADR-030 §Decision`. ADRs record *why we decided*, which is process history that belongs in the ADR, the CHANGELOG, or the commit — not in source. State the code's actual constraint instead (e.g. `the frozen 2025-* crates keep turul-rpc 0.1`, not `per ADR-025`). This applies to `.rs` **and** `Cargo.toml`/manifest comments. (Project docs — CHANGELOG.md, ADRs, COMPLIANCE.md, plan docs — may cite ADRs freely; source comments must not.) **External MCP spec anchors are different and remain allowed** — a `SEP-####` or `@see` reference names the *wire contract the code implements* (what it IS), see the Allowed list below.
- **Upstream schema line numbers** — `Schema line 2627`, `lines 943–949`. Line numbers shift every time we re-pin the schema (`refresh --write`); the comment quietly becomes wrong without anyone noticing.
- **Tombstones / dev log narratives** — `was removed in v0.3.42`, `formerly known as X`, `pending Phase 5`. Git history is the log; code comments are not.
- **Comparative claims you haven't verified** — `unlike every other Result type`, `the only place we do X`. Either grep and enumerate (`X, Y, Z all share this shape`) or don't claim it.
- **Self-references to `CLAUDE.md` / `AGENTS.md` in code comments** — `see CLAUDE.md §Comments`, `per CLAUDE.md §Notification Wire Format`. The repo playbook governs *how* code is written, not what individual files cite. Code comments should describe the code; they don't need a citation to the rule that says *"comments should describe the code."* Project docs (CHANGELOG.md, ADRs, COMPLIANCE.md, plan docs) may cite CLAUDE.md / AGENTS.md by name when explaining process decisions — `.rs` source files must not.
- **Speculation about author intent** — `intentional or oversight`, `presumably because the spec authors meant`. We don't know intent.

**Allowed — and preferred when the WHY is non-obvious:**
- Hidden invariants that can't be expressed in the type — `caller must hold the mutex when invoking this`
- Constraints not visible from one site — `keep in sync with the kebab-case in foo.rs::REGISTRY`
- Workarounds for specific bugs with context — `reqwest #1234: trailing newline corrupts the cookie jar`
- Verifiable schema anchors by NAME, not line — `Wire shape of \`elicitation/create\`'s URL-mode params — see \`ElicitRequestURLParams\` in the 2026-07-28 schema.` Names survive re-pins; line numbers don't.
- **Mirror the schema's `@see` anchors** when the upstream type carries one. The MCP schema uses TypeDoc `@see` block tags pointing to the spec docs (e.g. `@see [General fields: _meta](/specification/2026-07-28/basic/index#meta)`). When our Rust binding documents that type, carry the same anchor through as a doc link — anchors are URL fragments tied to section IDs, not line numbers, so they survive re-pins. Example: `/// See [General fields: _meta](https://modelcontextprotocol.io/specification/2026-07-28/basic/index#meta).` Anchors that are missing from the upstream schema (an `@see` we couldn't find) ARE useful information — flag them, don't make them up.

**Default: write no comment.** If removing the comment wouldn't confuse a future reader, don't write it. Well-named identifiers describe WHAT; comments earn their place by explaining WHY.

## Slice Completion Gate

**Before declaring a slice "complete" or writing a summary claim ("X is now clean", "no violations remain", "all instances fixed"), run a pre-declared verification grep across the FULL scope of the rule — not just the instances the prior reviewer named.**

The recurring failure mode this gate exists to stop: reviewer surfaces N instances → operator fixes N → operator claims "rule satisfied across crate" → next reviewer finds M more instances the first didn't search for → repeat. Each "fix" was correct; each "claim" was premature.

**Mandatory before claiming a comment-rule slice done** (applied to the whole crate, not just `src/`):

```bash
# All counts MUST be 0. If non-zero, surface each hit with explicit
# disposition (keep as historical/migration note, or rewrite) BEFORE
# claiming the slice is done.
grep -rEc 'Schema line|schema line|Schema lines|schema lines|lines [0-9]'  <crate>/
grep -rEc 'Phase [0-9]\.\?|Slice [0-9]|Group [A-G] —|Group [A-G]:'         <crate>/
grep -rEnc 'BP-[0-9]|GAP-[A-Z]|VER-[0-9]|TX/GAP|CF/GAP'                    <crate>/src/ <crate>/tests/  # gap-register IDs — none in source
grep -rEc 'CLAUDE\.md|AGENTS\.md'                                          <crate>/src/ <crate>/tests/
grep -rEc 'removed:|was removed|no longer:|formerly known|deleted with'    <crate>/
grep -rEn '\b2025-11-25\b'                                                 <crate>/  # then disposition each hit
grep -rEc 'initialization handshake|notifications/initialized'             <crate>/src/

# Identifiers, not just prose: the SCREAMING-CASE patterns above never match a
# gap ID that has been snake_cased into a fn name, a server name, or a string.
grep -rEc '\bbp[0-9]|\bgap_[a-z]|\bcf[0-9]|\bpat_g[0-9]|\br[0-9]_[a-z]'    <crate>/src/ <crate>/tests/

# FILENAMES, not just contents. Every grep above reads file bodies, so a
# tracking ID in a path — `tests/verify_bp3_build_2026.rs`, a `[[test]]` target
# name, `scripts/verify_phase4.sh` — is structurally invisible to them.
# Fix by renaming to what the file verifies, keeping the prefix
# (`verify_phase4.sh` → `verify_storage_backends.sh`), then update every place
# that names the old target: scripts/ci-gates.sh, Cargo.toml `[[test]]`
# entries, and any docs/plans row citing the test by filename.
find <crate>/ -type f -printf '%f\n' \
  | grep -Eic 'bp[0-9]|gap|cf[0-9]|ver[0-9]|(^|_)r[0-9](_|\.)|phase[0-9]|slice[0-9]|batch[0-9]'
```

For ambiguous hits — historical migration notes that explain a current shape vs. stale current-spec claims — list each one in the slice summary with a per-instance disposition. Never silently let them pass.

The verification runs **before** the "done" claim, not after a reviewer finds the gap. The gate is the same regardless of which reviewer or agent wrote the fixes.
