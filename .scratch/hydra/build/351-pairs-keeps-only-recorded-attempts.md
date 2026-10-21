# 351 - `pairs` keeps only answers that `attempts.jsonl` holds, so nothing a panel wrote enters `chosen` or `rejected`

Type: build
Status: resolved
Size: haiku
Blocked by:
Opened: 2026-10-21, by the session, from map 133's question 3, which the human answered that day:
the training signal comes from verifiable rewards and from open-weight teachers whose licence
permits it, and the frontier models stay judges. Today `crates/apprentice/src/pairs.rs` lines
125 to 138 put the graded row's `better_answer`, text the panel wrote, into `chosen` on a `worse`
verdict and into `rejected` on `better`. A pair that holds a frontier model's words is training
text under the three vendors' clauses quoted in `research/133-the-plan-of-2026-10-15.md`
section 2, and nothing in the command tells the two apart.

## What

Both sides of a pair come from `attempts.jsonl`. `pairs` reads that file beside `bank.jsonl` and
keeps the set of task id and answer it holds, task ids compared the way the bank's are
(`py::same_key`). A graded row whose `local_answer` or `better_answer` equals no recorded answer
of that task is skipped and named, one line per row, in the shape the other skips use:
`pairs: skipped <id repr>, <field> is not a recorded attempt`. The panel's job becomes choosing
between two recorded attempts, and `better_answer` is the other attempt's text; the field keeps
its name so a graded file from before still parses. With no `attempts.jsonl` every row is skipped
the same way, so a deleted file closes the door rather than opening it. Nothing else about the
output changes: the pair's fields and their order, the redaction, the kept rows, the stamp and
the final line stay as the Python wrote them.

Out of scope: a sampler that makes a second attempt of one model at one prompt. `attempt` skips
a task already answered at the same `prompt_sha` and model (`attempt.rs` line 303), so today two
attempts of one task come from two models or two prompt versions. Which model wrote each side is
not written into the pair in this build.

## Contract

Input: `graded.jsonl` as `integrations/apprentice/README.md` describes it, `bank.jsonl`, and
`attempts.jsonl` whose lines carry `task_id` and `answer` (`attempt.rs` lines 473 to 476).
Output: `pairs.jsonl` holds only pairs whose `chosen` and `rejected` texts are, before redaction,
answers that `attempts.jsonl` records for that task id. Every other row is named on stdout and
counted in the final line's `skipped`. `records::read_jsonl` returns an empty list for a missing
file, which is the no-file case.

## Cases

The 104 recorded cases in `crates/apprentice/tests/fixtures/pairs.json` stay byte-equal, and
they plant no `attempts.jsonl`, so the fixture changes first. A one-off script, run with
`python3 -I` from outside the fixture directory and not committed, adds to each step's `planted`
an `attempts.jsonl` under that step's grader home, beside its `bank.jsonl`, with one line
`{"task_id": <id>, "answer": <text>}` per `local_answer` and `better_answer` of every graded row
the step plants, for rows whose two answers are strings and whose task id the planted bank holds
as an open task. A step that plants no bank, or that expects exit 1, needs nothing. Run the
differential test after the script and before any code: every case must still pass, since the
code does not read the file yet. A case that fails after the script is a case the script got
wrong, not a case to re-record.

Then three cases appended to the fixture by hand in the same shape, their expected `stdout`,
`exit` and `written` written by hand from the other cases' shape, which fail before the code and
pass after:

1. `a better_answer the attempts file does not hold is skipped and named`: a bank with one open
   task, `attempts.jsonl` with the local answer alone, one graded row with verdict `worse`.
   Stdout names the row with `better_answer`, then `pairs: 0 new, 0 in pairs.jsonl, 1 skipped`;
   `written` is the empty `pairs.jsonl`.
2. `an answer recorded for another task does not count`: `attempts.jsonl` holds both texts under
   `summary:1`, the graded row is for `summary:0`. Skipped, naming `local_answer`, which is
   checked first.
3. `no attempts file skips every row`: no `attempts.jsonl`, two graded rows that would each make
   a pair. Two skip lines, `2 skipped`, empty `pairs.jsonl`. Run twice: the second run gives the
   same.

The fixture's top-level object gains `"departs": "build 351 plants attempts.jsonl, which the
Python never read, and the three cases after the 104 recorded ones were written by hand"`. The
test reads only the keys it names, so the new key costs nothing.

## Do

1. Read `integrations/apprentice/README.md` lines 76 to 90, `pairs.rs` whole, `attempt.rs` lines
   300 to 320 for how the set of attempts already written is read, and the header of
   `crates/apprentice/tests/pairs.rs`.
2. The fixture, as above, script first and the three cases by hand. `cargo nextest run --release
   -p hydra-apprentice --test pairs` shows 104 green and 3 red.
3. The code: a set of task id and answer read from `attempts.jsonl` with `records::read_jsonl`,
   the check after the two answers are known to be strings and before the swap, the skip
   sentence, the count. Few lines.
4. The README paragraph on `pairs`; one paragraph in `IMPLEMENTATION.md` after the build 233
   sentence near line 3727; one sentence in the docstring of
   `crates/apprentice/tests/record_pairs.py` saying the fixture departs from the Python since
   this build.
5. `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
   `cargo nextest run --release --workspace --no-fail-fast`, `cargo test --release --workspace
   --doc`. Then `## Outcome` under this ticket with what was built, the fixture counts before and
   after, and anything above that turned out false.

## Outcome

Built. `pairs` reads `attempts.jsonl` beside `bank.jsonl` and keeps a pair only when both
answers are recorded attempts of that task. The check runs after both answers are known to be
strings and before the swap. It tests `local_answer` first and names the row with
`pairs: skipped <id>, <field> is not a recorded attempt`. Task ids compare with `py::same_key`,
as the bank's do. A missing `attempts.jsonl` skips every row. Nothing else in the output changed.

Fixture. Before: 104 recorded cases, none planting an attempts file. The planting script
added an `attempts.jsonl` beside the bank in 112 steps, 934 lines in all. The same 104 cases
pass with those files in place, and the Python ignores them. Then three hand-written cases were
appended, 107 in all, with `departs` at the top level. Before the code change the three new
cases failed (7 comparisons) and the 104 passed. After it, all 107 pass.
`cargo nextest run --release -p hydra-apprentice --test pairs` runs one test function, so the
104 and 3 split shows in the comparison list, not as separate test rows.

Checks: `cargo fmt --all --check` clean. `cargo clippy --workspace --all-targets -- -D warnings`
clean. `cargo nextest run --release --workspace --no-fail-fast`: 1460 passed, 4 skipped.
`cargo test --release --workspace --doc`: every crate reports ok.

Prose: the README paragraph on `pairs`, a paragraph in `IMPLEMENTATION.md` after the build 233
sentence, and one sentence in the docstring of `record_pairs.py`.

What the ticket got wrong or left out:

- A step that expects exit 1 does need planted attempts. Once `pairs` reads the file, a row the
  Python reached before its `ValueError` is skipped, so the NaN-id step exits 0 instead of 1 when
  its rows have no attempts. The script planted every step with a bank and graded rows, whatever
  its exit.
- Several steps reuse the bank or the graded file of an earlier step in the same case. The
  script carries that state across the steps of one case.
- `read_attempts` checks the answer as well as the task id for hashability on read. The ticket
  named only the id. This follows `Done::read` in `attempt.rs`.
