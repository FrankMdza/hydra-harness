# Build 359 - `src/env.rs` tests `Provider::Codex` and `OPENAI_API_KEY`

Status: claimed
Size: haiku

## Goal

In `src/env.rs`, `ChildEnv::build` handles `Provider::Claude`, `Provider::CursorAgent`, and `Provider::Agy | Provider::Codex`, while the unit tests in `src/env.rs` cover `Provider::Claude`, `Provider::CursorAgent`, and `Provider::Agy`, leaving `Provider::Codex` and `OPENAI_API_KEY` untested.

## What to change

1. **`src/env.rs`:**
   - In `drops_the_secrets_this_shell_exports`, add `"OPENAI_API_KEY"` to the list of secret environment variable names verified absent from the built child environment.
   - Add a new unit test `codex_gets_the_base_four_and_the_proxies_and_nothing_else` right after `agy_gets_the_base_four_and_the_proxies_and_nothing_else` that calls `ChildEnv::build(Provider::Codex).unwrap()`, checks that every variable in `built.vars()` is in `BASE` or `PROXY`, and asserts that `"OPENAI_API_KEY"`, `"CODEX_HOME"`, `"GEMINI_API_KEY"`, `"ANTHROPIC_API_KEY"`, `"NODE_EXTRA_CA_CERTS"`, `"SHELL"`, and `"TMPDIR"` are not in `built.vars`.

## Done when

- `cargo fmt --all --check` passes.
- `cargo clippy --release --workspace --all-targets -- -D warnings` passes.
- `cargo test --release --lib env` passes.
