# 249 - The copilot's profile: Sonnet, its own short document, outside the repository's instructions

Type: build
Status: resolved
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

## Outcome

Resolved 2026-10-03 by run `01a101b2-a68e` (`cursor-agent` composer-2.5, `standard`, 1.39M
tokens), applied by the session with one change to how the chat starts.

Built: `integrations/copilot/CLAUDE.md`, the copilot's own document (under 90 lines: what Hydra
is, launching with `hydra run --timeout 0`, waiting with `hydra wait`, reading with `hydra show`,
the policies in a line each, never applying or reaping without the person's word, two live agents
at most, `COCKPIT.md` by pointer), and `integrations/copilot/bin/hydra-copilot [model]`, which
starts `hydra cockpit --session claude --model ${1:-sonnet}` with no policy. Tests: two in
`tests/copilot.rs`.

Changed by the session: the run started the chat in `integrations/copilot/`, on the reading that
Claude Code would load that directory's `CLAUDE.md` instead of the repository's. Claude Code reads
the `CLAUDE.md` of its directory and of every directory above it, so the repository's build
instructions would still load, which is what map 97's D6 set out to stop. The launcher now
renders the document with the repository's absolute path into `${HYDRA_COPILOT_HOME:-
~/.hydra-copilot}`, refuses a directory inside the repository, and starts there; the document
names `{{REPO}}` where it had `cd ../..`. The document also said `--timeout 0` means no timeout;
it means return at once and leave the agent running, and now says so. Tried by hand with
`HYDRA_BIN=echo`: the rendered file carries the path five times and the argv is `cockpit
--session claude --model sonnet`; a directory under the repository is refused with exit 1.

Not measured: what a copilot costs now. The human's own global instructions still load in any
directory; the repository's and its memory index do not. Measuring the cost is the human's next
session.
