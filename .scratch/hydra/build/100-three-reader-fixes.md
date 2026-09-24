# 100 - A codex quota failure reads as rate limited, export refuses an output that aliases its sibling, and the codex MCP fragment is real TOML

Type: build
Status: resolved
Blocked by: 

## What

The third hiring panel, 2026-09-23. Implement `~/.claude/handoffs/2026-09-23-panel3/consolidated.md` "Before publishing" item 5, 10 and 11, findings H4, H41 and H43.
Read those findings in its Findings table first; each names the lines and what was measured.
Files: src/normalize/codex.rs, src/export.rs and src/provider/codex.rs (the MCP fragment only).

Documentation changes go in the Outcome as proposed sentences, not into `SECURITY.md`,
`IMPLEMENTATION.md` or `README.md`: several tickets touch those files at once and the session
applies them together, build ticket 102.

## Done when

Each with a test that fails before: record 01a0cfdb-e5c3's shape derives `rate_limited`; `--out` naming the preference sibling by another spelling is refused; quotes, backslashes and newlines round-trip through the MCP fragment. Every new test fails before the change, checked by reverting it. `cargo fmt --check` and
`cargo clippy --all-targets -- -D warnings` are clean and the suite passes outside a seatbelt.

## Outcome

Built by a Claude Code session working on Hydra's source, not through `hydra run`.

**H4, a `codex` quota failure.** `src/normalize/codex.rs` now reads `turn.failed` as the run's
terminator, the way it reads `turn.completed`, and keeps it as `Run::result`. A top-level `error`
frame goes to `noise`. `thread.started`, `turn.started`, `item.started` and `item.updated` are
dropped by name, and any other frame type raises one `Finding::Limit` per type, so a shape the
provider adds later shows up in the reading. `src/outcome.rs` gained two readings, both keyed on
`provider == "codex"` and `result.type == "turn.failed"`: `codex_limit_refusal` returns
`rate_limited` when `error.message` contains "You've hit your usage limit", and it runs beside
`provider_refused`, before the gated check. Any other `turn.failed` is `failed`. The ticket named
only three files. The mapping lives in `outcome.rs`, so that file changed too.

Measured on this machine's records with the built binary's `hydra show`. 01a0cfdb-e5c3 and
01a0d02d-f812 now read `rate_limited`, `derived`. So does 01a0cc9d-8f15, whose `turn.failed`
says "Error running remote compact task: You've hit your usage limit". 01a0ca7d-f703, stopped by
the provider's content filter, reads `failed`. No real `codex` record raises the new unknown-frame
limit; a tally over every `codex` record found only the types listed above plus `turn.completed`,
`item.completed` and `error`.

**What the ticket assumed that was false.** The finding said `turn.failed` appears nowhere in
`src/`, which held. It did not say that 01a0cc9d-8f15 had been deriving `completed` rather than
`failed`. That run finished its answer, then hit the limit in a compaction step, and closed on
`turn.failed` with exit 1. It now reads `rate_limited`, so rung one no longer learns from it. That
is the conservative direction, but it is a run whose task text is complete. If a human wants a
finished answer followed by a quota failure read as `completed`, that is a contract 8 decision.
`tests/outcome.rs::the_recorded_write_denials_are_the_only_gated_runs_on_this_machine` walks the
corpus and needed a `turn.failed` branch for it.

**H41, `--out` aliasing its sibling.** `src/export.rs::open_events_out` opens `--out` without
truncating, compares its device and inode with `preference.jsonl` beside it, and refuses when they
match. A file the call created only for the comparison is removed on refusal. The name check stays
as the first, cheaper refusal. The events file is truncated only after the comparison passes.

**H43, the `codex` MCP fragment.** `src/provider/codex.rs::mcp_config` builds a `toml::Table` and
serializes it with the `toml` crate already in `Cargo.toml`, and now returns `Result<String>`.
`src/mcp.rs` propagates the error. The server name is a key, so a dot or a quote in it no longer
splits the table header. The output differs in one visible way: `env` and `http_headers` are
written as `[mcp_servers.<name>.env]` sub-tables rather than inline tables. Both are the same TOML.

**Tests, each checked by reverting the fix it guards.**

- `tests/outcome.rs::a_codex_turn_that_hit_its_usage_limit_is_rate_limited_and_not_failed` builds
  01a0cfdb-e5c3's shape, 01a0cc9d-8f15's prefixed sentence and 01a0ca7d-f703's content-filter
  failure, and checks the unknown-frame limit fires once for a made-up type and never on the real
  shape. With `codex.rs` and `outcome.rs` reverted it failed on the first assertion: `left:
  Some((Failed, Derived))`, `right: Some((RateLimited, Derived))`.
- `tests/export.rs::out_naming_the_preference_file_by_another_spelling_is_refused` puts a file at
  `preference.jsonl` and exports to a symbolic link, a hard link and, on a case-insensitive
  volume, `Preference.jsonl`. Each must be refused with the sibling's bytes unchanged. Then, with
  no sibling, `Preference.jsonl` must be refused and leave nothing behind. On a case-sensitive
  volume the folded spelling is skipped and says so. With `export.rs` reverted it failed on the
  symbolic link: `unwrap_err()` on an `Ok`.
- `provider::codex::tests::quotes_backslashes_and_newlines_round_trip_through_the_mcp_fragment`
  puts quotes, backslashes, a newline, a single quote and a `]` in the command, the arguments,
  an env key and value, the URL, a header key and value and the server name, parses the fragment
  back and compares each string. With the old `format!` writer it failed: "TOML parse error at
  line 2, column 22, unclosed table".

`cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` are clean. `cargo test --lib`
passed 207, and `tests/outcome.rs` 44, `tests/normalize.rs` 83, `tests/export.rs` 22,
`tests/mcp.rs` 16 and `tests/show.rs` 3. The whole suite is in build ticket 104's Outcome, run
once over both tickets.

**Proposed sentences for build ticket 102.**

For `IMPLEMENTATION.md`, in the `codex` section beside the terminator paragraph:

> **A `codex` turn that fails ends on `turn.failed`, and since build ticket 100 that frame is the
> run's terminator.** A message containing "You've hit your usage limit" derives `rate_limited`,
> and any other `turn.failed` derives `failed`. This is a text match, like `cursor-agent`'s. Three
> records on this machine read `rate_limited` now: 01a0cfdb-e5c3, 01a0d02d-f812 and
> 01a0cc9d-8f15. The last one finished its answer before a compaction step hit the limit. A frame
> type the reader does not know raises a `Limit` finding instead of vanishing.

For `IMPLEMENTATION.md` or `SECURITY.md`, where `hydra export` writes its two files:

> `hydra export` refuses an `--out` that is the same file as `preference.jsonl` beside it,
> compared by device and inode before a byte is written. That covers `Preference.jsonl` on a
> case-insensitive volume and a symbolic or hard link by any name.

For `IMPLEMENTATION.md`, in the MCP section, on `codex`'s fragment:

> The `codex` fragment is serialized by the `toml` crate, so any string `config/mcp.md` holds,
> quotes, backslashes and newlines included, reaches `.codex/config.toml` as the same string.
