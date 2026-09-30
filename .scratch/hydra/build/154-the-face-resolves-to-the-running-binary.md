# 154 - The face resolves to the running binary, and anyone can install Hydra

Type: build
Status: claimed
Blocked by:

## What

`config/mcp.md` names the delegate face with an absolute path,
`command = "/Users/USER/hydra/target/release/hydra"`, because a bare `hydra` failed on
2026-09-07: the run's `PATH` is the human's and `hydra` was not on it (`config/mcp.md` near
line 56). That path is one person's, in a repository about to be public.

The human asked on 2026-09-30 to generalize it without breaking his install: a symlink for
him, a README that works for anyone, and an `AGENTS.md` so agents that read that file know how
to use Hydra.

`src/mcp.rs` already matches the face on the basename (`with_parent`), so `command = "hydra"`
is recognized as the face today; what fails is spawning it.

## Do

1. **Resolve at write time.** In `src/mcp.rs`, where a run's `mcp.json` is written, a server
   whose `command` basename is `hydra` and is not an absolute path gets `command` replaced by
   `std::env::current_exe()` of the running Hydra, canonicalized. An absolute path stays as
   written. The record's `mcp.injected` stays what the document declares, as today. Test in
   `tests/mcp.rs`: a fixture document with `command = "hydra"` produces an `mcp.json` whose
   command is the test binary's own path; one with an absolute path is unchanged.
2. **`hydra doctor`** gains one line: whether `hydra` on the human's `PATH` resolves to the
   running binary, to another one, or to nothing. It is a report, not a refusal.
3. **`config/mcp.md`**: the example block uses `command = "hydra"`, and the paragraph near line
   56 says that since this ticket Hydra writes its own path into the run's file, so the bare
   name works and the absolute path is no longer needed. This is protected config #5, which a
   run cannot write: leave that file alone, and put the exact new text of the block and the
   paragraph in your report for the human to paste.
4. **README, `## Install`.** Replace the `alias` line with a symlink on `PATH`:
   `mkdir -p ~/.local/bin && ln -sf ~/hydra/target/release/hydra ~/.local/bin/hydra`, and a line
   to check `~/.local/bin` is on `PATH`. Say why an alias is not enough: the face is spawned by
   the vendor's CLI, which sees `PATH` and not shell aliases. Add `hydra doctor` as the check.
5. **`AGENTS.md`** at the repository root, for an agent that reads it (codex reads `AGENTS.md`;
   `CLAUDE.md` stays for `claude`): what Hydra is in five lines, the three populations of
   `CLAUDE.md`'s last section in plain words, the commands an agent may run (`hydra run`,
   `hydra show`, `hydra status`, `hydra logs`, `hydra doctor`) with one example each, what it
   must never do (write `policies/`, `config/`, touch `runs/`, commit from inside a run), and
   where the design lives. Under 120 lines. Do not copy `CLAUDE.md`; point at it.

## Done when

- The two `tests/mcp.rs` tests failed before the change and pass after; `cargo test --test mcp`,
  `cargo test --lib`, fmt and clippy clean.
- `hydra doctor` prints the new line on this machine, and the report quotes it.
- `README.md` and `AGENTS.md` changed as above, `config/mcp.md` untouched, with its new text in
  the report.
