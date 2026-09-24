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
Hydra saw; paste the relevant events rather than describing them.

## Before you open a pull request

```
cargo fmt --check
cargo clippy -- -D warnings
cargo build --release
cargo test --release --no-fail-fast -- --test-threads=1
```

The suite walks the run records on the machine it runs on, so run it single-threaded. A
test must build its own scratch layout with `Layout::at` and never call `Layout::discover()`,
which would touch the real `~/hydra`.

Several tests walk every `.jsonl` under `~/hydra/runs/` and check that each record still
meets the contract, normalizes, and derives an outcome. That path is `$HYDRA_ROOT/runs` when
the variable is set, and `~/hydra/runs` from the real home when it is not. Point the variable
at an empty directory to run the suite without reading your own records. Each of those walks has
a fixture twin that reads `tests/fixtures/records/` instead, so the assertions still run on an
empty root. A new record shape goes into a new fixture through `tests/fixture_corpus.rs`, never
as a copy of a real record.

Rung two's admission runs this suite inside a seatbelt, with a cleared environment and several
test threads. A test that launches the built binary with `HYDRA_ROOT`, or execs a setuid binary
such as `/bin/ps`, has to check `hydra::sandbox::inside_seatbelt()` and skip with a line on
stderr, or every rung-two pass fails on it. A test that sets a process-wide variable belongs in
an integration test file of its own with a lock around it, the way `tests/git_config.rs` does,
not in a unit test that shares a process with threads that read the environment.

## What is not open for a pull request

The five protected configs of `docs/harness-spec.md` section 15.1: `policies/`, the
environment allowlist, the export allowlist, `config/preamble.md` and `config/mcp.md`. They
are the things a run must never be able to widen. A change to one of them is a decision, and
it goes through a map ticket first.

Prose follows the conventions in `CLAUDE.md`. Hydra's gate binds the runs Hydra launches, not a
contributor editing Hydra's source; "Nothing here is enforced" in `CLAUDE.md` says why.
Commit messages say what changed and why in plain sentences.
