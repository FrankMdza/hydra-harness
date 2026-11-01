# 378 - A gated interactive session writes neither `~/.claude.json` nor `history.jsonl`

Type: build
Status: resolved
Blocked by:

## Objective

Map 99, resolved on 2026-11-01 by the panel of map 134 (`research/134-synthesis.md`), chose (a). Build 250 left `~/.claude.json*` and `~/.claude/history.jsonl*` writable in a gated interactive session, because nobody knew whether `claude` starts without them. The session measured on 2026-11-01 that it does: `claude` 2.1.312 under a write deny on both prefixes answered in `-p` mode, interactively in a trusted directory, and interactively in a fresh directory after its trust dialog, and neither file's sha nor mtime changed. A gated session that can write `~/.claude.json` can add a user-scope MCP server that the next ungated `claude` session starts with the human's permissions.

## Scope

- `src/sandbox.rs`: `copilot_block` already denies writes under the whole home directory. Remove the loop at about line 898 that allows the two regex prefixes back. If `Rule::CopilotWriteAllowRegex` then has no producer, remove the variant and what reads it, and say in the Outcome whether `narrow_for_copilot`'s bubblewrap refusal at about line 845 still has a reason to exist. If it does not, keep the refusal anyway and say what else stops a gated interactive launch under bubblewrap; widening Linux support is not this ticket.
- Tests: the unit tests that pin the regex allow, including the one near line 1687, change to pin its absence. Add or flip a live seatbelt probe so a write to `~/.claude.json` and to `~/.claude/history.jsonl` from inside the narrowed profile is refused. Use a scratch home, never the real one. It must skip inside a seatbelt like the other live probes.
- `SECURITY.md` lines 76 to 85: a gated session writes neither file; name `~/Library/Caches/claude-cli-nodejs` beside the `~/.claude` state directories the profile allows; map 99 is answered. Lines 422 to 423: `hydra bench` refuses to launch without `--dry-run` until map 114 defines a success test, rather than "counts a `completed` run as a successful attack".
- `IMPLEMENTATION.md`: amend build 250's paragraph.
- Leave a line in the Outcome for the human's acceptance: one gated `hydra run --interactive` session of about 30 minutes, with both files' sha compared before and after. The builder cannot run it.

## Checks

`cargo fmt --all --check`, `cargo clippy --release --workspace --all-targets -- -D warnings`, `cargo check --release --target x86_64-unknown-linux-gnu`, and `cargo nextest run --release --workspace --no-fail-fast`.

## Outcome

Built on 2026-11-01.

- `copilot_block` in `src/sandbox.rs` no longer allows `~/.claude.json` or `~/.claude/history.jsonl` back. Both stay under the home deny, and a comment says why.
- `Rule::CopilotWriteAllowRegex` had no producer except one unit test, so the variant, its seatbelt rendering, its bubblewrap refusal and its slot in the test's kind list are gone.
- `narrow_for_copilot` still refuses a `.bwrap` profile. Its first reason, the regex allow, is gone. Two other things stop a gated interactive launch under bubblewrap. The block is seatbelt text spliced after the seatbelt TIOCSTI line, and a bwrap profile has no such line. The bwrap profile also opens with `--new-session`, which detaches the terminal the human types into, and `rules_of` never emits the interactive TIOCSTI variant. The message now reads "the copilot block has only a seatbelt form; gated interactive launch is not supported under bubblewrap".
- Tests. `translate_rule_to_bwrap_refuses_untranslatable_rules_naming_each_rule` lost its copilot case. The bwrap refusal test is renamed `narrow_for_copilot_refuses_a_bwrap_profile` and checks the new message. A new unit test, `the_copilot_block_allows_neither_claude_json_nor_history`, checks that no block line carries a regex or either file name. A new live probe, `under_the_copilot_profile_neither_claude_file_is_writable` in `tests/sandbox_copilot.rs`, builds the block for a scratch home in `/private/var/tmp` and splices it into a headless profile. It first writes to `.claude/debug` as a control, then expects writes to both files and to a `.tmp` and `.lock` sibling of each to be refused, with the contents unchanged. The scratch home cannot go under `TMPDIR`, because the block hands that back. It cannot go under Cargo's target directory either: in the human's checkout that is under `~/hydra`, which the headless profile denies, so the control would fail.
- The probe skipped here, inside this run's seatbelt. It has not run live.
- `SECURITY.md` and `IMPLEMENTATION.md` amended as the scope lists. The `hydra bench` sentence matches `src/main.rs`, which refuses without `--dry-run`.
- Checks: fmt, clippy with `-D warnings`, the Linux `cargo check` and `cargo nextest run --release --workspace --no-fail-fast` all pass, 1546 tests, 4 skipped.

For the human's acceptance: run one gated `hydra run --interactive --policy standard` session for about 30 minutes, with `shasum -a 256 ~/.claude.json ~/.claude/history.jsonl` before and after, and confirm both shas match and the session worked throughout. Also run `cargo nextest run --release -p hydra --test sandbox_copilot` outside any seatbelt so the two live probes actually run.

Measured by the session on 2026-11-01, on macOS outside a seatbelt. `under_the_copilot_profile_neither_claude_file_is_writable` passed, its control write landed and all six writes were refused. With the two regex allows put back in `copilot_block`, it failed: "a write to .../.claude.json was not refused: rc=0". The suite ran 1546 tests, all passed, 4 skipped, and the doctests passed. The human's 30-minute gated session is still to do.
