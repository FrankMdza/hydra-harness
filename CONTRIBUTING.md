# Contributing

Hydra is small on purpose and most of its decisions are written down. Before proposing a
change, find the decision it touches: `docs/` is the design, `IMPLEMENTATION.md` is every
place the code departs from it and why, and `.scratch/hydra/map.md` is the history of how
each decision was made, one ticket per question. A change that contradicts a decision is
welcome, and it starts by reopening that ticket rather than by editing the code.

## What a good issue looks like

The most useful report is **provider drift**: a vendor CLI changed its stream, its hook
dialect, or its flags, and something Hydra relied on moved. There is an issue template for
it. Include the vendor version (`claude --version`, `cursor-agent --version`, `agy --version`,
`codex --version`, or the four lines `hydra doctor` prints),
the run id, and the lines of the record that changed. `tests/drift.rs` is where a confirmed
drift becomes a test.

For anything else, the record is the bug report. `hydra show <run-id> --events` prints what
Hydra saw; paste the relevant events rather than describing them. That command prints raw event
content without running the twelve-class secret redactor that `hydra export` applies, so inspect
the output before pasting it into a public issue.

## Before you open a pull request

```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo build --release --workspace
cargo nextest run --release --workspace
cargo test --release --workspace --doc
```

The test suite runs in parallel since build ticket 296. It reads the checked-in synthetic
fixtures under `tests/fixtures/records/`. `hydra doctor --walk` reads the live records under
`~/hydra/runs/` instead, since build ticket 126. A test must build its own scratch layout with
`Layout::at` and never call `Layout::discover()`, which would touch the real `~/hydra`.

`hydra doctor --walk` walks every `.jsonl` under `~/hydra/runs/`, or under `$HYDRA_ROOT/runs`
when the variable is set. The integration tests `record_contract.rs`, `normalize.rs` and
`outcome.rs` walk `tests/fixtures/records/` instead. Point `HYDRA_ROOT` at an empty directory to run the walk without
reading your own records. A new record shape goes into a new fixture through
`tests/fixture_corpus.rs`, never as a copy of a real record.

Rung two's admission runs this suite inside a seatbelt, with a cleared environment and several
test threads. A test that launches the built binary with `HYDRA_ROOT`, or execs a setuid binary
such as `/bin/ps`, has to check `hydra::sandbox::inside_seatbelt()` and skip with a line on
stderr, or every rung-two pass fails on it. No test mutates the process environment since
build ticket 296; pass environment variables on the child `Command` instead of calling
`std::env::set_var`.

## What is not open for a pull request

The six protected configs of `docs/harness-spec.md` section 15.1: `policies/`, the
environment allowlist, the export allowlist, `config/preamble.md`, `config/mcp.md` and
`config/panels.json`, since build ticket 138. They
are the things a run must never be able to widen. A change to one of them is a decision, and
it goes through a map ticket first.

Prose follows the conventions in `CLAUDE.md`. Hydra's gate binds the runs Hydra launches, not a
contributor editing Hydra's source; "Nothing here is enforced" in `CLAUDE.md` says why.
Commit messages say what changed and why in plain sentences.
