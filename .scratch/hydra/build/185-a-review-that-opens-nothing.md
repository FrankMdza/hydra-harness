# 185 - A review that opens nothing

Type: build
Status: open
Blocked by: 181, 183

## What

Found by build 183. `puppets.py review` ends by `execvp` of `hunk show <branch>` (the end of
`cmd_review` in `integrations/puppets/puppets.py`). The cockpit starts it detached with stdout and
stderr in a log file and no terminal, so Hunk will likely fail or hang: the cockpit row then stays
on `review running`, or shows a verdict while the exit says failed.

## Do

Tests first, each shown failing before the change.

1. `puppets.py review <goal> <task> --no-open`: runs the panel, records the verdict and pushes the
   findings as it does now, then prints the `hunk show ...` command it would have run and exits 0,
   without exec. Without the flag nothing changes. Test in `integrations/puppets/tests/` with
   stubs for the judge launch and Hunk the suite already uses (find them; do not launch a real
   judge).
2. The cockpit (`src/cockpit/app.rs`, the review spawn from build 183) passes `--no-open`, and the
   run's detail shows the printed `hunk show ...` line from the log once the review ended, so the
   human can open it. Test in `tests/cockpit.rs` with the stub helper asserting the argv.
3. `integrations/puppets/README.md` and `IMPLEMENTATION.md`.

## Done when

- Each test failed before the change; the puppets suite; fmt, clippy, `cargo test --lib`, `--test
  cockpit` pass.
