# Prompts Test Server

An E2E **fixture**, not a tutorial. It registers 11 prompts chosen to cover the
`prompts/*` surface and its edge cases, and exists so `tests/prompts`
(`cargo test -p mcp-prompts-tests`, 88 tests) has one server to point at.

If you are learning how to write a prompt, read `examples/prompts-server`
(hand-written `McpPrompt` with template substitution) instead.

## Spec lane

Pinned to **2025-11-25** in `Cargo.toml` (`protocol-2025-11-25` on every
framework dependency), independent of the workspace 2026-07-28 default: the
suite drives it through the stateful `initialize` →
`notifications/initialized` → `Mcp-Session-Id` handshake.

## What each prompt is for

| Prompt | Exercises |
|---|---|
| `simple_prompt` | No arguments, fixed messages |
| `string_args_prompt` | Required and optional string arguments |
| `number_args_prompt` | Numeric argument arriving as a string, range 1–100 |
| `boolean_args_prompt` | Boolean argument arriving as a string |
| `template_prompt` | `{placeholder}` substitution into the message body |
| `multi_message_prompt` | A `user`/`assistant`/`user` message sequence |
| `session_aware_prompt` | Reading session context while rendering |
| `validation_prompt` | Email format and age-range validation with typed errors |
| `dynamic_prompt` | Output shape varying by `mode` (creative/analytical/supportive) |
| `empty_messages_prompt` | An empty `messages` array |
| `validation_failure_prompt` | Always fails validation, for error-path tests |

`GetPromptParams.arguments` is `map<string,string>` at the wire boundary, which
is why the numeric and boolean prompts parse their arguments out of strings
and return `-32602`-mapped `McpError`s on bad input.

## Run

```bash
# --port 0 (the default) takes an ephemeral port; the choice is logged
cargo run -p prompts-test-server -- --port 8021
```

The E2E harness starts it itself via `TestServerManager::start_prompts_server()`
(`tests/shared/src/e2e_utils.rs`), so running it by hand is only for poking at
it with curl.

## Duplicate copy

`tests/prompts/bin/main.rs` is a byte-identical copy of `src/main.rs`, built as
the `prompts-test-server-e2e` binary. Nothing spawns that binary — the harness
runs `cargo run -p prompts-test-server`, i.e. this package. Edit this file; the
copy is dead weight awaiting removal.
