# Simple Solutions First
**ALWAYS** prefer simple, minimal fixes over complex or over-engineered solutions:

```rust
// SIMPLE - Add parameter to existing signature
async fn read(&self, params: Option<Value>, session: Option<&SessionContext>) -> McpResult<Vec<ResourceContent>>

// COMPLEX - Create new traits, elaborate architectures
trait McpResourceLegacy { ... }  // Avoid backwards compatibility layers
trait McpResourceV2 { ... }      // Avoid versioned APIs
```

**Key Principles:**
- **Work within existing architecture** - don't rebuild what works
- **Major changes are too costly** - fix problems with minimal impact
- **One obvious way to do it** - avoid multiple patterns for the same thing
- **Green is not proof** - a passing suite says the checks ran, not that they
  could have failed. Establish that a check *can* fail on the bug before
  believing it passed for the right reason. See [test-coverage-discipline.md](test-coverage-discipline.md) item 4.

## Scope Discipline

- **Stay inside the approved plan and stated requirement** — do not broaden scope by changing adjacent contracts, tests, or semantics unless directly required
- **If a fix forces unrelated API behavior changes or test expectation changes, stop and reassess** — that's a signal you're modifying the wrong layer
- **If scope or architecture becomes ambiguous, stop and ask** — do not improvise
- **`replace_all` edits must be scoped precisely** — never use `replace_all` on patterns that appear in unrelated code paths

## Before Modifying Core Crates

- **Impact Analysis**: All examples, tests, user code affected?
- **Breaking changes documented** clearly
- **No panics** — `Result<T, McpError>` for all fallible operations
- **Zero warnings**: `cargo check` must be clean
- **Doctests**: Every ```rust block MUST compile — fix errors, don't convert to ```text
- **Extend existing** components, never create "enhanced" versions
- **Test with framework-native APIs**, not raw JSON manipulation

```rust
// Framework-native testing
let tool = CalculatorTool { a: 5.0, b: 3.0 };
let result = tool.call(json!({"a": 5.0, "b": 3.0}), None).await?;

// NOT raw JSON manipulation
let json_request = r#"{"method":"tools/call"}"#;
```
