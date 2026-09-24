# 102 - The documents after the third panel

Type: build
Status: resolved
Blocked by: 97, 98, 99, 100, 101, 103, 104, 105, 106

## What

`~/.claude/handoffs/2026-09-23-panel3/consolidated.md` "Before publishing" items 7, 8, 14, the documentation half of 15, and 16, plus the sentences
the other tickets of this batch proposed in their Outcomes, and the second panel's consolidated
report moved into `docs/reviews/` so its G ids resolve. Findings H7 to H13, H19, H30, H31, H35,
H42, H44, H45 and H48.

Build ticket 105 changed what 98 and 99 proposed: the admission profile moved to
`spawn/<pass-id>/admission.sb` and denies `.cargo` configuration file by file, and Hydra's git
now passes the excludes file and `safe.directory` back with `-c`. Build ticket 106 changed what
103 proposed: reap scrubs the run's store before it imports. Take the sentences from 105's and
106's Outcomes where they differ.

## Done when

Every sentence changed is checked against the code, and the counts are dated.

## Outcome

Built on 2026-09-24 by a Claude Code subagent on Opus 5.5 in its own worktree, not through
`hydra run`. No source, test, config or policy file changed. Each proposed sentence was checked
against master at `531423c` before it went in; where 98, 99 and 103 differ from 105 and 106, the
later ticket's text is what landed.

Per file:

- `SECURITY.md`. The first section no longer says no shell credential reaches the agent: it says
  the allowlist bounds the child and, since 97, the watcher, and that a run under `standard`
  still reads dotfiles and a same-uid `ps -E` still reads your terminals (H3). Admission is a
  bullet of its own with 98 and 105's copy, cargo home and `.cargo` denials, and what stays open
  (H2, H38). The git bullet is 99's with 105's excludes file, `safe.directory` and attributes
  pin (H5, H47). The object-store bullet is 103's with 106's scrub and cap, and names 108's window
  (H1). The watcher's day of exposure and the marker that reached no recorded child from 57 to 97
  are under known exposures. The `codex` fail-open bullet names 101's paid probe and its first
  pass. The export bullet says six secret classes and 100's `--out` refusal (H41, H45).
- `IMPLEMENTATION.md`. A `codex` row in the version table (H12). The exit-status paragraph says the
  harvested arm reads `exit_code` (H10). The marker paragraph and the `env_clear` inventory (97).
  The `codex` section: `turn.failed` (100, H4). The MCP section: the `toml` fragment (100, H43), and
  the drift paragraph's paid `codex` probe (101). The CI paragraph: the fixture corpus (101, H14).
  The export section: six classes and the `--out` refusal. The git section: 99, 105, 103 and 106,
  with the four providers' measured `git add` from 103's Outcome. Rung two: 98 as 105 amended it,
  104's suite inside a seatbelt, H19 closed by the copy, H35's probes skipping during admission,
  101's drivable path. Reap's `LineZero` and partial injection (101, 99, H40). A replay caveat
  after `GateMark` (H42). One rough edge from 104. A pointer to the second and third panels'
  reports.
- `README.md`. The environment bullet names what Hydra sets and the watcher; the outcome bullet
  names `codex`'s `turn.failed`; the gate bullet points at the `codex` fail-open (H30). 26 test
  files and the fixture walks (H9, H14). The authorship table through 107 from each Outcome and
  the git log, 106 tickets, 58 of them on an agent run (H7). The map at 75 of 76 and the build at
  107 of 108 (H8). The comparison table keeps its cells and gains a sourced line per project,
  re-read on 2026-09-24; a "no" is an absence and has no quote (H31).
- `CLAUDE.md`. Twenty-six test files with `fixture_corpus.rs`, `git_config.rs` and
  `launch_phases.rs` described, `drift.rs`'s two `codex` probes, `cleanup.rs`'s store tests, the
  four provider and normalizer files, the map at 75 of 76 and the build at 107 of 108.
- `CONTRIBUTING.md`. The fixture twins, and what a test must do to survive admission inside a
  seatbelt, including H16's process-wide variables.
- `tests/corpus/README.md`. Rows A6, A7, A8 and A13 name the watcher, the probes, the names Hydra
  sets, the git config test and the marker test.
- `docs/harness-spec.md`. A preface like the contract's on `codex`; the floor applies on every
  routed provider (H11); `codex` exits 0 on a gated run too; the git paragraph names 99 and the
  per-run store.
- `docs/run-record-contract.md`. `codex` in the `provider` enum (H44); `env_allowlist` names what
  Hydra sets and the pre-97 marker.
- `docs/rungs-spec.md`. Admission runs on a copy of the read tree; baseline-test provenance is
  built.
- `docs/reviews/`. The second and third panels' consolidated reports, each with a two-line note
  naming the commit it read and where the fixes are, `/Users/USER/hydra` replaced by "this
  repository" in one line (H13). Both indexed in `README.md`. The first panel's F30 and F31 rows
  say what fixed them, and a dated note under the table covers F30, F31, F32 and F33 (H48).
- `.scratch/hydra/build/`. The open residuals ticket was a second 107, beside the `hy` launcher,
  and `frontier.py` keys by number, so it vanished from the count and the frontier: 106 of 107
  with nothing takeable. It is 108 now, in its file, its title and `plan.md`, and the frontier
  prints 107 of 108 with 108 takeable.

**Sentences found false and corrected**, beyond the panel's list:

- `IMPLEMENTATION.md` said contract 5.5 did not list `may_edit`. It does, since 96.
- `IMPLEMENTATION.md`'s `codex` section said `codex` could not sit on the `evolve` panel until map
  63 closed. Map 63 closed on 2026-09-21 and build 61 put it there.
- `IMPLEMENTATION.md` said the fixture corpus was not built, and that a run could add objects and
  not alter one because the store is content-addressed. Map 74 measured the second false.
- `docs/rungs-spec.md` said baseline-test provenance was not built. Build 96 built it.
- 105's proposed sentence credited the 416 s admission to the review; it was build 98's
  measurement.

**False in `src/`, left alone because this ticket does not touch it:** the module doc of
`src/outcome.rs` says `codex` is unmeasured on a gated exit, where run 01a0c62b-d005 measured it;
the doc of `codex_limit_refusal` says all three quota records derived `failed` before, where 100's
Outcome says 01a0cc9d-8f15 derived `completed`; the doc on `Admission` in `src/rungs/evolve.rs`
names only the two `.cargo` files and not the name deny 105 added. `src/export.rs:4`'s "four
fixed classes" counts kinds of redaction, as `IMPLEMENTATION.md` does, not secret classes, so it
is not wrong.

The caller's brief put the map at 74 of 74. `frontier.py` prints 75 of 76 on 2026-09-24, with
map 76 a grilling for the human, and the documents say that.
