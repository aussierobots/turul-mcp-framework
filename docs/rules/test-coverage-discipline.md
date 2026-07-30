# Test Compliance

**Tests validate the MCP spec and intended contract — never change tests to preserve buggy behavior.**

- When code and tests disagree, verify against the MCP specification before changing either
- Never silently accept multiple wire formats in tests (e.g., `.strip_prefix("data: ")` to handle both SSE and JSON) — assert the expected Content-Type and body format explicitly
- Tests must assert wire-format compliance: Content-Type headers, HTTP status codes, JSON-RPC error codes, and response body shape

## Test Coverage Discipline (pre-publish gate)

**Every behavior-changing slice must satisfy all four before release:**

1. **ADR exists or is updated.** Tests validate the ADR contract, not what the code happens to do. If no ADR governs the changed behavior, write or update one in the same slice and reference it in the CHANGELOG entry.
2. **Production-path coverage.** Tests must exercise the entry point real consumers use (e.g., `Builder → server.handler() → handle_streaming()`), not just direct construction of the patched type. A fix verified only at the unit it touched does not cover the bug — v0.3.40 → v0.3.41 happened because tests bypassed the builder path.
3. **Wire-layer coverage for transport-protocol boundaries.** When the fix touches code that produces bytes consumed by another protocol layer (Lambda Runtime API, hyper, SSE wire format, JSON-RPC envelope, MCP streamable HTTP), the test MUST exercise the bytes that hit that next layer — not just the framework-internal types that produce them. v0.3.42 happened because tests asserted "BodyDataStream yields ≥1 item" while production failed at "Lambda Runtime API wire bytes after delimiter are non-empty." For transport-protocol tests, drain through a verbatim transliteration of the upstream serializer (e.g. `lambda_runtime-<version>/src/requests.rs`) and assert on the resulting byte sequence. No "faithful mock" loophole — use the upstream code unmodified (call it if pub, replicate verbatim with a source-line citation if not).
4. **Revert-and-fail check.** Temporarily revert the fix and run the new tests. They MUST fail. If they still pass, the test asserts code behavior rather than contract — rewrite it. Record the revert-and-fail result in the commit message.

A green test suite written alongside the fix is suspect by default. The revert-and-fail check is the only proof the regression net catches the bug. The wire-layer rule exists because a test calibrated to the framework's internal types will pass for any fix that satisfies those types, even when the fix doesn't satisfy the actual protocol contract consumed downstream.

## A Check That Cannot Fail Is Not a Check

Item 4 above makes a *test* prove it can fail. The same burden
applies to everything else that reports pass/fail — gates, guards, scripts, probes.
Each of these shipped green while checking nothing:

- A guard parsed `[[bin]]` blocks and was blind to `src/main.rs` autobins.
- A harness rebuilt into `CARGO_TARGET_DIR` and launched from a hardcoded
  `target/debug`, so it tested whichever stale binary was there.
- `check-protocol-purity.sh` was invoked by no gate, and its crate list omitted the
  crate this branch exists to build.
- A probe detected a spec violation, printed a warning, and exited 0.
- Script legs captured `$?` after a plain command under `set -e`, so the shell exited
  before the assignment and every failure branch was unreachable.
- A compliance row asserted "no framework path emits this code" without grepping for
  the literal.

Before claiming a check works, answer three questions with evidence:

1. **Does it run?** Follow the invocation to a gate — `grep` the script name in
   `ci-gates.sh` *and* one level of indirection (`verify_all_examples_unattended.sh`
   calls six scripts `ci-gates.sh` never names). Watch a counter move in the log.
2. **Can it fail?** Break the thing deliberately and watch it go red. If it stays
   green, it is calibrated to the mechanism you were thinking about, not the defect.
3. **Does the failure say why?** A check that fails without naming the cause costs
   the next reader the same diagnosis. Report the discriminating fact — which port,
   which token, exited-or-hung.

Applies equally to a shell exit code, a `#[test]`, a CI step and a compliance row.

## Briefing reviewer agents

**Any agent spawned to review code, audit compliance, or critique a design MUST first read the rules they'll be judging against. Their report is worth nothing if they don't know what "compliant" means in this repo.**

When spawning a reviewer agent (Explore, Plan, code-reviewer, devils-advocate, etc.), the prompt MUST tell them — explicitly, by path — to read:

1. `~/turul-mcp-framework/AGENTS.md` — repo policy (source of truth, wins on conflict)
2. `~/turul-mcp-framework/CLAUDE.md` — operator playbook, links to `docs/rules/`
3. Any ADR in `docs/adr/` that governs the area under review

Agents have filesystem access via the `Read` tool. They will NOT magically know about the Comments rule, the Branch Lock, the Protocol Crate Purity rule, the schema-line-numbers-rot caveat, or the `@see` anchor convention unless the prompt instructs them to read CLAUDE.md/AGENTS.md and apply those rules. Don't assume — instruct.

The reviewer's report should cite the rule it's invoking (e.g. "violates docs/rules/comments.md: schema line reference in production code") so the operator can verify the rule actually says what the agent claims.
