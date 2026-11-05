# 196 - puppets in Rust: the cutover

Type: build
Status: resolved
Blocked by: 195, 207

## What

Last of four tickets for map 90's `puppets` port. The cockpit runs `puppets.py` as a subprocess in
four places (`src/cockpit/app.rs`, `snapshot.rs`, `mod.rs`); CI runs the Python suite; the README
and `hy.zsh` name `puppets.py`.

## Do

0. Before anything is deleted, close the two findings of the last review that can compare the
   two (`cursor-agent` grok-4.7-high on build 195, 2026-10-01), each with a differential input or
   a unit test shown failing first:
   - `run_file_has_end` (`crates/puppets/src/launch.rs` near 485, through `read_text` in
     `write.rs` near 671; called by `wait`, `plans` and the judge panel). The Rust reads the whole
     run file and refuses it when any byte is not UTF-8. The Python reads it as text line by line
     and returns as soon as it has seen `run_end`, so a 12 KB record with `run_end` on line 0 and
     a `0xFF` later is ended for Python and an error for Rust. Read the file as bytes, line by
     line, and decide on the lines, so one bad byte in a provider's frame cannot stop `wait` or
     a panel that already sat a seat.
   - The Hunk pusher (`crates/puppets/src/panel.rs` near 1048 to 1064). It starts
     `current_exe()` again as `push-hunk-comments`; when that path is gone the spawn fails after
     the `review` event is in the log, the command exits 1, `--no-open` never prints `hunk show
     <branch>`, and a retry sits the panel again. When the pusher cannot start, say so on stderr
     and still print the command or exec Hunk; the review is recorded either way.
1. The session ran every differential test and the ignored real-log test on master at `e1d5b3b`
   on 2026-10-01, all green (62 unit, 5 in the binary, 18 differential, the real-log test), and
   one real launch, recorded below. Run them once more yourself before you delete anything.
2. The cockpit runs `hydra-puppets` (found beside the `hydra` binary, then on `PATH`) wherever it
   ran `puppets.py`; its stubs in `tests/cockpit.rs` follow.
3. Delete `integrations/puppets/puppets.py`, `sheet.py`, `models.py`, `jev_probe.py` only if
   ported (else say so and keep it) and the Python tests the Rust ones replace; keep the JSON data
   files where the crate reads them.
4. `pending --json` prints an `argv` that starts `python3 <repo>/integrations/puppets/puppets.py
   approve` (`crates/puppets/src/main.rs`, `fold::approve_argv`, kept by build 193 because
   `approve` was still Python). Point it at `hydra-puppets approve`, and drop
   `log::python_dir()`, which bakes the build checkout's path into the binary; the JSON data
   files need a home that does not.
5. CI: `cargo test --workspace`, and drop the puppets Python suite step. README, `hy.zsh`, the
   Herdr plugin and `IMPLEMENTATION.md` name `hydra-puppets`.

## Notes from 195

- Wait for the review of 195 and for build 207 before this ticket: once the Python is deleted
  nothing checks the Rust against it.
- `sheet.py`'s `build`, `budget` and `corpus_records`, and `models.py` beyond `load_aliases` and
  `canonical`, have no caller in the helper's commands and were not ported. Find who runs them
  (`grep -rn 'sheet.py\|sheet build\|models.py' --include='*' .` outside `.scratch/`), and
  either port them or keep those two files and say so.
- The differential tests need the Python. Decide per test: turn its inputs and the Python's
  recorded outputs into fixtures the Rust is checked against, or delete it. Do not leave a test
  that silently skips.
- The first real launch: run `hydra-puppets` once against the real `hydra` on a throwaway goal
  (the session does this, with the human's quota in mind) before the Python goes.

## The first real launch, 2026-10-01

The session drove one goal through `hydra-puppets` against the real `hydra`, in a scratch
`HYDRA_PUPPETS_HOME`: `new`, `choose <path>`, `approve --plan`, `tasks`, `set`, a `launch` refused
for the missing table approval (exit 1), `approve --table`, `launch` (run `01a0f666`,
cursor-agent composer-2.5, 22,342 tokens, `completed`), `wait`, which read the real `hydra show`
and appended `run_end`, `board`, `pending --json`, and `verdict rejected`. `show` from the Rust
and from `puppets.py` on the resulting log were byte-identical before and after `wait`. Not
exercised against the real thing: `plan`, `plans`, `choose --judge`, `review`, a Herdr pane, Hunk.

## First half, 2026-10-01

Run `01a0f66b` (claude opus 5.5, `standard`, 5.5M tokens) did item 0, the rewiring and the
documents, and stopped at a green tree with the Python present. The session applied it to master.

Done: `run_file_has_end` reads the run record as bytes, a line at a time, and returns at the
first `run_end`; a pusher that cannot start is reported and `review` still prints the Hunk
command or becomes Hunk. The cockpit runs the `hydra-puppets` beside the running binary, else the
one on `PATH`; `integrations/herdr/bin/hydra-board` does the same. `pending --json` prints an
`argv` that starts with the answering `hydra-puppets`. `log::python_dir()` is gone:
`log::data_dir()` finds `prompts/` and the JSON files at run time from `HYDRA_PUPPETS_DATA`, else
`integrations/puppets` under `HYDRA_HOME`, else the first directory above the binary that holds
`integrations/puppets/prompts`. The damaged-log hint says `hydra-puppets repair <goal>`, in the
Python too, so the differential tests still compare stderr byte for byte. CI builds and tests
the workspace, and the session pinned its Python to 3.14, which the differential tests follow.

Measured on master: fmt and clippy clean over the workspace; `cargo test -p hydra-puppets` 65
unit, 5 in the binary, 18 differential, and the ignored real-log test; `cargo test --test
cockpit` 110; `cargo test --lib` 224; the puppets Python suite and the Herdr suite pass.

Not done, and why: no Python was deleted. `integrations/grader/labels.py` imports `puppets.py`
for `fold`, `records.py` imports `models.py`, and `baseline.py` imports `jev_probe.py`; the
ticket assumed only the cockpit, CI, the README and `hy.zsh` depended on the Python. `sheet.py`'s
`build`, `budget` and `show` are run by hand from the README. `docs/reviews/` names `puppets.py`
in five files, so the grep condition below cannot hold without editing `docs/`. How the grader
gets a goal's fold once `puppets.py` goes is map ticket 93. The second half of this ticket waits
on it.

## Second half, after map 93

Read `.scratch/hydra/issues/93-where-the-grader-reads-a-goal-once-puppets-py-is-gone.md`. In this
order, the tree green after each step:

1. `integrations/grader/labels.py` gets a goal's state from `hydra-puppets show <goal>`. The
   binary is an absolute path: `HYDRA_PUPPETS_BIN`, else `hydra-puppets` beside `HYDRA_BIN` when
   that is absolute, else `target/release/hydra-puppets` under the checkout the grader file is
   in. Never `PATH`. A missing binary, a non-zero exit (a damaged log is one), or JSON without the
   fields it reads raises, and the grader run that hit it writes no labels at all: find where
   labels are written and make that hold, with a test. `labels.py`'s own reader of raw events
   for `judge_run_ids` follows the helper's rule too: it runs only after `show` succeeded.
2. **The measurement that gates the deletion.** A test that computes `labels_for_goal` both
   ways, through `puppets.fold` and through the shell-out, over every goal log the differential
   tests harvest plus logs with human verdicts, reviews and plans, and requires them equal. Run
   it, and put the count of goals and of labels compared in the report. The session runs the same
   over the real `~/hydra/puppets` before it applies your diff. If any label differs, stop
   there, leave the Python in place, and say which.
3. `jev_probe.py` and its tests move to `integrations/grader/` (`git mv`); `baseline.py` loads it
   from there. `models.py` and `sheet.py` stay where they are, with `sheet.py`'s tests.
4. Record the Python's output: for every differential input, the stdout, stderr, exit code and
   resulting files the Python gave, written once by a script that is committed
   (`crates/puppets/tests/record.py`) into `crates/puppets/tests/fixtures/`. The differential
   tests then compare the Rust with those files and no longer start `python3`. Keep the
   comparator's normalizations. Say how large the fixtures are; if they pass 5 MB, keep the
   harvested inputs and drop the least informative, and say which.
5. Delete `puppets.py` and the Python tests that test it. Keep the JSON data files and
   `prompts/` where `log::data_dir()` finds them. CI drops the puppets Python suite step, keeps
   `sheet.py`'s tests if there are any, and no longer needs its Python pinned for the
   differential tests; say whether anything else still needs 3.14.
6. `README.md`, `integrations/puppets/README.md`, `integrations/grader/README.md` and
   `IMPLEMENTATION.md` say what is left in Python and why.

`docs/reviews/` names `puppets.py` in five files and is history; leave `docs/` alone. The grep
condition below reads: outside `.scratch/` and `docs/`, `puppets.py` is named only where a
sentence says it was deleted.

## Done when

- fmt and clippy over the workspace; `cargo test --workspace`; `grep -rn puppets.py` finds only
  history in `.scratch/`.

## Outcome

Resolved 2026-10-01, in two runs of claude opus 5.5 under `standard`: `01a0f66b` (5.5M tokens,
the first half above) and `01a0f685` (18.7M tokens, the second half after map 93). The session
ran the gate on the real goal log before applying the second diff, and applied it as written
with one addition.

Built in the second half: `integrations/grader/labels.py` reads a goal through `hydra-puppets
show -- <goal>`, checks the shape it reads, and raises `LabelsError` on a missing binary, a
non-zero exit or another shape; the binary is `HYDRA_PUPPETS_BIN`, else beside an absolute
`HYDRA_BIN`, else `target/release/hydra-puppets` of the checkout, never `PATH`; `collect` calls
it before any write, so a raise leaves the dataset, the cursor and the scores untouched.
`jev_probe.py` and its test moved to `integrations/grader/`. `crates/puppets/tests/record.py`
ran the Python once on every differential input and wrote `crates/puppets/tests/fixtures/` (446
files, 3.3 MB); the differential tests now compare the Rust with those files and start no
`python3`. `puppets.py`, its seven test files and `harvest.py` are deleted. `sheet.py` and
`models.py` stay, with `sheet.py`'s suite. CI no longer pins Python and keeps the `sheet.py` and
grader suites.

The gate of map 93, measured before the deletion: 405 goal logs of the differential tests
labelled both ways, 366 equal with 396 labels, 39 refused by `show`, none of which had a label
under the Python fold; no label differed. The session ran the same over the real
`~/hydra/puppets` (one goal log, 9 labels, equal) and over the goal of the first real launch (1
label, equal). `show` took 5 ms per goal from the release build, 387 ms on the first call.

Measured on master after applying: fmt and clippy clean over the workspace; `cargo test
--workspace`, 40 test binaries, none failed (`hydra-puppets` 65 unit, 5 in the binary, 18
differential over 1,312 read commands, 173 write, 87 launch, 9 locales); the grader suite 133
run with one error, `test_launchd`, which cannot import `plistlib` on this machine's Python; the
`sheet.py` suite 10; the Herdr suite 42. The session changed one word of `board` and 138 of
1,312 fixture comparisons failed, then restored it.

The session's addition: `.claude/skills/puppets/SKILL.md` still told the session to run
`python3 integrations/puppets/puppets.py`. It names `hydra-puppets` now.

What the ticket assumed that is false: the grep condition. `puppets.py` is still named in the
fixtures (recorded usage lines and tracebacks), in the comparator's sample strings, and in
sentences that say it was deleted. `hy.zsh` never named it.

Left open, each said in `IMPLEMENTATION.md`:

- The fixtures cannot be recorded again from this checkout. A case added to `differential.rs`
  needs a checkout from before this ticket to record it.
- The fixtures were recorded and replayed on arm64 macOS with CPython 3.14.7 only. CI has not
  run them.
- A damaged goal log now fails a whole grader pass, where the old reader passed over a bad last
  line. Map 93 asked for it.
- The grader's tests need a built `hydra-puppets` under `target/`.
- `show` from the launchd job was not run.
- Against the real router only `new`, `choose <path>`, `approve`, `tasks`, `set`, `launch`,
  `wait`, `board`, `pending`, `show` and `verdict` have run; `plan`, `plans`, `choose --judge`,
  `review`, a Herdr pane and Hunk have only met stubs.

