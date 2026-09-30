# 171 - The cockpit shows policies and opens the editor

Type: build
Status: resolved
Blocked by: 170

## What

The panel of 2026-09-30 (unanimous, option B) settled how the human edits policies from the
cockpit: he does not, the cockpit hands him his own editor. `policies/` is protected config #1 and
only the human edits it; decision 86 kept a writing editor out of the cockpit. Every run records
`policy_sha`, the sha of the policy set it launched under (`src/policy/mod.rs:540`), and a live run
keeps its gate when the file changes. Conditions the seats set: the cockpit shows the current
sha against each live run's sha, so nobody thinks a live run picked up an edit; a half-written
policy file must not become the next launch's gate.

## Do

Read `src/cockpit/` (`app.rs`, `mod.rs`, `host.rs`), `src/policy/mod.rs` (how the set loads and
its sha), and `policies/README.md` before code.

1. A `p` key opens a policy pane: the five policy names, the current policy set sha, and for each
   live run its `policy_sha` with a mark where it differs from the current one. Read-only.
2. In that pane, `e` on a policy suspends the cockpit (leave the alternate screen, disable raw mode
   and mouse capture, as the panic hook already restores the terminal), runs `$EDITOR` (or `vi`)
   on that file, waits, restores the terminal, reloads the policy set, and shows the new sha. The
   cockpit never writes the file itself.
3. If the reloaded set fails to load or compile, the pane says so in red with the loader's error.
   Say in your report whether a launch after a broken save is refused today by the loader, and
   where; do not change the loader.
4. With `--session`, the chat child keeps running while the editor is open; say in the report
   whether that holds.
5. `IMPLEMENTATION.md`, the cockpit passage.

Tests first in `tests/cockpit.rs`, drawing into a `Buffer` and with a stub `EDITOR` script that
appends a comment to the file: the pane lists the five policies and marks a run whose sha differs;
`e` runs the stub and the pane shows the new sha; a stub that writes an unloadable file shows the
error. Use a scratch `Layout` with its own `policies/`, never the real one.

## Done when

- Each test failed before the change; fmt, clippy, `cargo test --lib` and `cargo test --test
  cockpit` pass.

## Outcome

Resolved 2026-09-30. Built by `claude` opus on run 01a0f2e2-de3a under `standard`. The session
applied it as written. `p` opens a read-only policy pane: the five files, the current set's sha
(or the loader's error in red), and each live run's policy and `policy_sha` from line 0, marked
`current`, `differs: it keeps the gate it launched with`, `no sha recorded` or `cannot compare`.
`e` suspends the screen, runs `$EDITOR` (or `vi`) on the file, restores the screen and reloads the
set; the cockpit never writes the file, and inside a seatbelt `e` refuses (the run added that).
Three tests; fmt and clippy clean, `--test cockpit` 28, `--lib` 220 on master.

From reading the code: a broken save is refused at the next launch by `PolicySet::discover()` in
`run::preflight_inputs`, before a run id exists, so it leaves no record. With `--session` the
chat child keeps running while the editor is open; its output appears after. The ticket's
"appends a comment" stub was wrong for JSON; the loadable stub appends whitespace. The snapshot's
reader thread keeps the set it loaded at start, noted in `IMPLEMENTATION.md`. Never run in a real
terminal.

