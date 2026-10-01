# 194 - puppets in Rust: the commands that write without launching

Type: build
Status: resolved
Blocked by: 197, 199

## What

Second of four tickets for map 90's `puppets` port. After 193 the crate reads and folds a goal log
and answers the read commands byte for byte like `puppets.py`.

## Do

Port, with the same arguments, output, exit codes and log lines as the Python: `new`, `set`,
`approve` (all `--expect-*` checks under the lock, exit 2 and 3 as today), `verdict`, `choose`
(recording a verdict, not launching the judges: if `choose` launches runs, stop at the recording
and say where the launch is), `tasks`, `assign` (with `sheet.py` and `models.py` ported:
`aliases.json`, `vendors.json`, `priors.json` read as they are), `reconcile` if it launches
nothing. Differential tests as in 193: each command on the same starting log in two temp homes,
one per implementation, then the resulting logs compared line by line (normalize only timestamps
and ids, and say which). The Python stays.

## Notes from 193

- `log::locked_update` exists and holds one `flock` across the read and the append. It decodes
  line by line, where `_locked_update` reads in text mode: with invalid UTF-8 anywhere in the file
  the Python raises `UnicodeDecodeError`, and the Rust can return `UnterminatedLog` or skip the
  line instead. Add that input to the differential test and make the two agree.
- Map ticket 91 and build 197 changed what a bad line does before this ticket: `read_log` and
  `_locked_update` both raise `DamagedLog`, in the Python and in the crate. Every writer you port
  refuses a damaged log with that message and exit 1, and the differential test covers it.
- `pending`'s `argv` still names `python3 integrations/puppets/puppets.py approve`. Leave it;
  196 changes it.
- The differential test normalizes three things (`pending`'s `argv[0]`, the torn file's
  timestamp, a traceback by exception class). For the writers, add only timestamps and generated
  ids, and list each in the report.
- Build 199 closed the ten parity gaps codex found in 193, the lone surrogate among them, and
  tightened the differential test's three normalizations. Keep to its comparator.

## Done when

- fmt and clippy over the workspace; `cargo test -p hydra-puppets`; every command compared, with
  the list in the report.

## Outcome

Resolved 2026-10-01. Run `01a0f616` (claude opus 5.5, `standard`, 7.6M tokens, about 20 minutes).
The session read `approve`, `verdict`, `new` and `set` in `write.rs` line by line against the
Python and took the rest on its tests; applied to master as written.

Built: `crates/puppets/src/write.rs` with `new`, `set`, `approve`, `verdict`, `choose`, `tasks`,
`assign` and `reconcile`; `sheet.rs` with `rank`, `beta_lower` and `ranked_assignment`;
`models.rs` with `load_aliases` and `canonical`. `approve` folds once to refuse early, then
repeats every `--expect-*` check on the lines read under the lock and records that fold's
revision and run, as `cmd_approve` does; with the recheck removed its two tests fail. `choose
--judge` stops after its three refusals: the launch is `run_judge_panel` in `cmd_choose`
(`puppets.py` near 1933), which is build 195's.

Measured on master: fmt and clippy clean over the workspace; `cargo test -p hydra-puppets` 43
unit tests and 10 differential tests; the writer test runs 150 commands, each on the same
starting log in two temp homes, and compares stdout, stderr, the exit code and the resulting log
line by line; the ignored real-log test passed; `cargo test --lib` 224 passed; the puppets Python
suite 204 passed. Nothing under `integrations/puppets/` changed.

Normalized for the writers, on top of build 199's comparator: the `ts` of each appended line,
only when the log still begins with its input; and the goal id `new` generates, only when it is a
version 4 UUID alone on stdout or as the log's file name.

What the ticket assumed that is false: `assign` reads only `priors.json`, not `aliases.json` or
`vendors.json`; `vendors.json` is read only by the `choose --judge` refusals. The note about
invalid UTF-8 was out of date since build 197.

Left open, for build 195 or a review: `sheet.py` is half ported (`build`, `budget` and
`corpus_records` call `hydra show` and no command here uses them); `beta_lower` sums logs where
Python calls `math.lgamma`, checked on six pairs to 1e-12, so a bound within rounding of 0.5, 0.6
or 0.7 could rank differently; usage errors exit 2 in both with clap's text in place of
argparse's; `Path.stem` follows Python 3.14; `verdict` folds outside the lock and then appends,
as the Python does.

