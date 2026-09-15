# Contributing

Hydra is small on purpose and most of its decisions are written down. Before proposing a
change, find the decision it touches: `docs/` is the design, `IMPLEMENTATION.md` is every
place the code departs from it and why, and `.scratch/hydra/map.md` is the history of how
each decision was made, one ticket per question. A change that contradicts a decision is
welcome, and it starts by reopening that ticket rather than by editing the code.

## What a good issue looks like

The most useful report is **provider drift**: a vendor CLI changed its stream, its hook
dialect, or its flags, and something Hydra relied on moved. There is an issue template for
it. Include the vendor version (`claude --version`, `cursor-agent --version`, `agy --version`),
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
at an empty directory to run the suite without reading your own records.

## What is not open for a pull request

The five protected configs of `docs/harness-spec.md` section 15.1: `policies/`, the
environment allowlist, the export allowlist, `config/preamble.md` and `config/mcp.md`. They
are the things a run must never be able to widen. A change to one of them is a decision, and
it goes through a map ticket first.

Prose follows the conventions in `CLAUDE.md`. Commit messages say what changed and why in
plain sentences.
