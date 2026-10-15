# 320 - A floor refusal names the longest matching deny entry, the token that resolved to it and the link it went through

Type: build
Status: open
Blocked by:
Opened: 2026-10-15, by the session, from map 120's answer, question 2. Today the refusal names
the first deny entry that matched, which on this machine is always `/users/user/hydra` because
`hydra_home()` is first in `denied_write_paths` and the checkout is the runtime root. A provider
reads "refused for naming ~/hydra" and cannot tell whether it named a sibling run, a record, or
a skill file whose `realpath` lands in `~/hydra/skills-snapshots/`. Run `01a1332c` spent 146k
tokens on that.

## What

The refusal message of the shell check and of the path scopes carries three facts:

- the longest deny entry that matched, not the first (`~/hydra/runs` rather than `~/hydra`);
- the token of the command that resolved to it, as the provider wrote it;
- when the token reached the entry through a symlink, the link it went through (`.claude/skills/x`
  resolves to `~/hydra/skills-snapshots/<sha>/x`).

Never a suggestion that may be false ("use a relative path" was false for `01a1332c`). The
message shape is part of what `tests/gate.rs` asserts, so change the assertions with it and
keep the refusal's machine-readable field (whatever `hydra show` reads to label a refusal)
unchanged in name.

Opus found that the two skill tests at `tests/gate.rs` lines 2543 and 2601 pass for the wrong
reason: they never link the skill, so the `realpath` into the snapshot is never taken. Make them
link the skill before asserting, and assert the new message names the link.

## Do

1. Read `src/policy/mod.rs` around line 964 (the own-path carve-out) and `gate.py`'s
   `protected_root`, and `.scratch/hydra/research/120-seat-opus.md` section on the refusal.
2. The message, in the gate source; the three facts as separate fields in the hook's JSON reply
   where the reply is JSON, and in the one-line reason where it is text (codex's stderr line).
3. `tests/gate.rs`: the two skill tests fixed; one new case per fact.

## Done means

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo
nextest run --release --workspace`, `cargo test --release --workspace --doc` green. Amend
`IMPLEMENTATION.md` where the refusal text is described.

Rules inside the run: relative paths only, never look outside your own worktree, write files with
your file tool and not with shell heredocs, and the test suite is green inside a run since build
302 without any `env -u`.
