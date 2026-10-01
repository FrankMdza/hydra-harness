# 194 - puppets in Rust: the commands that write without launching

Type: build
Status: open
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
