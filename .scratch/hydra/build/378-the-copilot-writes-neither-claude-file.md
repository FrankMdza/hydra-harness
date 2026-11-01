# 378 - A gated interactive session writes neither `~/.claude.json` nor `history.jsonl`

Type: build
Status: claimed
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
