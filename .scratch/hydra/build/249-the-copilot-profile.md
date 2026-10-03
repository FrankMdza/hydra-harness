# 249 - The copilot's profile: Sonnet, its own short document, outside the repository's instructions

Type: build
Status: claimed
Blocked by: 246, 248

## What

Map ticket 97, D5 and D6 (resolved 2026-10-03). The ungated copilot (`hydra cockpit --session
claude` with no policy) started in `~/hydra` reads about 50k tokens of instructions meant for
building Hydra, and ran on Opus. It should run on Sonnet by default and start from a short
document of its own, in a directory that does not carry the repository's `CLAUDE.md`.

## Do

1. `integrations/copilot/`: a `CLAUDE.md` for the copilot alone, under 150 lines: what Hydra is in
   five lines; how to launch an agent (`cd <repo> && hydra run --provider ... --policy ...
   --timeout 0`), wait for several with `hydra wait` (build 248), read one with `hydra show`; the
   policies in one line each; never apply, commit or delete without the person's word; at most
   two live agents on an 8 GB machine; the help from `COCKPIT.md` (build 246) by pointer.
2. `integrations/copilot/bin/hydra-copilot [model]`: runs `hydra cockpit --session claude --model
   ${1:-sonnet}` with the chat's working directory in `integrations/copilot/`, and the repository
   the agents work on passed in the document or the environment. Check what the chat's
   environment keeps (`src/env.rs`, `src/interactive.rs` `run_ungated`): it clears the
   environment, so a variable may not arrive; say what you used.
3. If `hydra cockpit --session` has no way to set the chat's directory other than its own cwd,
   say so and use the cwd; do not add a flag unless it is needed.
4. `README.md`: one paragraph on the copilot and its cost; `IMPLEMENTATION.md`: one sentence.

## Done when

- A test reads the document and asserts its length and that it names `hydra wait` and
  `COCKPIT.md`; a test of the launcher's argv without launching. fmt, clippy and `cargo test`
  pass. The Outcome says nothing about cost that was not measured: measuring it is the human's
  next session.
