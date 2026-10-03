# 253 - The copilot takes a repository and keeps its hands off files

Type: build
Status: claimed
Blocked by:

## What

Map 100 D3 and D4, map 101 D5, all resolved 2026-10-03. On that day the copilot worked four hours
on the human's web repository without launching an agent, partly because nothing told it which
repository it serves except a sentence typed into the chat, and nothing stopped it from editing
files itself.

1. `integrations/copilot/bin/hydra-copilot` takes the repository as an optional second argument,
   after the model: `hydra-copilot [model] [repo]`. The repository defaults to Hydra's own, as
   today. It must be a git work tree (`git -C <repo> rev-parse --show-toplevel`); otherwise the
   script exits 1 with one line saying so. The resolved top level replaces `{{REPO}}`, and the
   existing check that the start directory is outside the repository applies to that one.
2. The source document is renamed `integrations/copilot/COPILOT.md` and the script renders it
   into the start directory as `CLAUDE.md`. Write the rendering so a second file name is one more
   line, since map 100 D3 says `AGENTS.md` and `GEMINI.md` follow; do not add them yet. The
   document says in one sentence that the repository it names is the one every `hydra run` and
   `hydra wait` starts from, and no other.
3. The copilot cannot use `claude`'s file-edit tools. `hydra cockpit --session claude` without
   `--policy` passes them as disallowed (`Edit`, `Write`, `NotebookEdit`; find the flag in
   `claude --help </dev/null`) through `src/provider/claude.rs`'s interactive argv, on the ungated
   arm only. The document says in one sentence that file changes go through `hydra run`.
4. `~/hydra-oficina/suprema.sh` is outside the repository; do not edit it. Say in the report what
   line it would need.

## Done when

- A test that runs the script with `HYDRA_BIN` pointed at a stub that prints its argv and cwd
  shows: no argument renders Hydra's path; a git repository argument renders its top level; a
  non-repository argument exits 1. A unit test on the interactive argv: ungated carries the
  disallowed tools, gated is unchanged. Each fails with its change reverted; say so. fmt, clippy
  over the workspace and `cargo test --lib` are clean.
- `COCKPIT.md` and `IMPLEMENTATION.md`: one sentence each.

## Outcome
