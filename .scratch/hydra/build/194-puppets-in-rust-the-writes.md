# 194 - puppets in Rust: the commands that write without launching

Type: build
Status: open
Blocked by: 193

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

## Done when

- fmt and clippy over the workspace; `cargo test -p hydra-puppets`; every command compared, with
  the list in the report.
