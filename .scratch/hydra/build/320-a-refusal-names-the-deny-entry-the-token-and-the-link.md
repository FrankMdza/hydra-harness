# 320 - A floor refusal names the longest matching deny entry, the token that resolved to it and the link it went through

Type: build
Status: resolved
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

## Outcome

Built, all in `src/policy/gate.py`:

- `protected_root`, the raw pass of the shell check and `protected_path_named_in` return the
  longest deny entry that matches, through one helper, `longest`.
- `resolved_pair` carries the path as the call wrote it as a third element. `link_crossed` walks a
  path one component at a time and returns the first symlink whose own location is outside the
  denied (or out-of-scope) set and whose target is inside it. A relative path is walked from the
  resolved cwd and its link printed relative, so `.claude/skills/unslop` rather than the absolute
  form; an absolute path crosses macOS's `/var` link, which the predicate rejects.
- The `hydra/protected-path` refusal of the shell check and of a file write, and the
  `hydra/out-of-scope` refusal of a file tool, append one sentence:
  `` `<token>` resolves to <path> through the link <link> to <target>. `` The link clause appears
  only when a link was crossed. The raw pass resolves nothing and appends
  `` `<word>` spells it out. ``, the whitespace-delimited word around the match. Newlines print
  as `\n` and a token past 200 characters is cut, so the reason stays one line for `codex`.
- The code before the colon is unchanged, so `reason_code` and `hydra doctor --walk` read the same.

Tests, `tests/gate.rs`: the two skill tests now link the skill (`.claude`, `.agents` and
`.cursor` roots) before asserting. The roots themselves stay allowed, and a `cat` or `head` of a
`SKILL.md` under a link is asserted refused, naming `skills-snapshots`, the token and the link.
Three new cases, one per fact: `a_refusal_names_the_longest_deny_entry_that_matched` (a gate whose
deny list and spellings carry its root first, as `hydra_home()` is first in production; shell
token pass, raw pass and `Write`), `a_refusal_quotes_the_token_as_the_command_wrote_it` (token
pass, raw pass, `Write` with a relative path, a newline in the token), and
`a_refusal_names_the_link_a_path_went_through` (`Write` through a skill link, an out-of-scope
`Read` through a link to elsewhere, no link clause when none was crossed).

Measured: `tests/gate.rs` 125 of 125 pass. Reverting `longest` to first-match fails exactly one
test, the longest-entry case. Workspace: 1406 passed, 4 skipped.

What the ticket assumed that turned out false:

- "The three facts as separate fields in the hook's JSON reply where the reply is JSON." Not
  done. No provider's tolerance of an extra key in its hook reply is measured, and harness spec
  6.6 records that `claude` allows a call whose reply shape it does not recognise. An unmeasured
  key on a deny could turn it into an allow, and `codex` reads the same `claude` shape. The facts
  are in the reason text in every dialect. Whether to add the fields, and the probe that would
  measure each provider first, is a decision for the session.
- The skill tests were not only passing for the wrong reason: once linked, the reads they asserted
  allowed are refused. The ticket kept the refusal, so the tests now assert it. Whether a shell read
  of the run's own pinned snapshot should be carved out (research 120, seat Opus, section 1) is
  still open and not this ticket's.

**Decision, 2026-10-15, the human on panel 130:** the three facts stay in the reason text and no
drift probe is spent. If a consumer of structured fields ever appears, they go on the `gate_refusal`
sidecar row of build 321, which Hydra writes and reads and no provider parses, not on the hook reply.
