# 229 - herdr: the shell asks `hydra-herdr` for a JSON field

Type: build
Status: resolved
Blocked by: 225

## What

Map ticket 95, question B: `hy.zsh`, `bin/hydra-in-pane` and `bin/hydra-tui-in-pane` stay shell.
They call `python3` to read one field of a JSON answer (`hy.zsh` near 59, 83, 97 and 204 to 208;
the launchers near 23 to 28). Those calls become `hydra-herdr`.

## Do

1. One small subcommand of `hydra-herdr` that reads JSON on stdin and prints the field a path
   names, with the Python snippets' behaviour on a missing field, a null, a non-string and bad
   JSON (record each from the snippets as they are).
2. Replace the snippets. Nothing else in the three scripts changes: quoting, exit codes, the
   `tee` of stderr through a process substitution in `hydra-tui-in-pane` stay as they are.
3. `test_hy.py`'s cases pass with the stubs it has. `smoke.sh` needs a live Herdr and is the
   session's to run.

## Done when

- Each test failed before the change, or the report says why it could not; fmt and clippy clean over
  the workspace; the tests of every crate the ticket touches pass; no dependency added without a
  reason in the report.

## Outcome

Dropped 2026-10-03 by map ticket 104 (D1, B): Herdr is frozen and the cockpit is the one interface, so the plugin is not ported to Rust. The run graph moves into the cockpit as build 271.
