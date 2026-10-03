# 253 - The copilot takes a repository and keeps its hands off files

Type: build
Status: resolved
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

Resolved 2026-10-03 by run `01a103a7-f596` (`cursor-agent` composer-2.5, `standard`, 1.35M
tokens). The session added one change to the agent's diff and verified one test by breaking it.

Built: `hydra-copilot [model] [repo]`. A second argument must be a git work tree, and its top
level is rendered into `{{REPO}}`; otherwise the script exits 1 with one line. The document is
`integrations/copilot/COPILOT.md`, rendered to `CLAUDE.md` in the start directory from a list of
source and destination pairs, one line per future file name. The ungated interactive argv carries
`--disallowed-tools Edit,Write,NotebookEdit`; the gated one does not (`claude --help` lists the
flag). Tests: `tests/copilot_repo_and_tools.rs` (the script against a stub `hydra`, and the argv)
and two argv tests in `claude.rs`. The session broke the script's git check and the script test
failed; restored, it passes.

What the session changed: `tests/copilot.rs` (build 249) still read `integrations/copilot/CLAUDE.md`
and would have failed; it now reads `COPILOT.md`. The agent named this and could not edit it.

Left: `~/hydra-oficina/suprema.sh` passes `"$@"` through, so `suprema.sh sonnet ~/repo` works
with no edit. `COPILOT.md` still names the gated cockpit mode, which build 254 removes; the session
fixes that sentence when 254 lands. The copilot keeps its shell, so it can still write a file;
map 101 accepted that.
